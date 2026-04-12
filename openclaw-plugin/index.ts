// OpenClaw native plugin for npcterm
// Spawns `npcterm` and communicates via MCP JSON-RPC over stdio.

import { definePluginEntry } from "openclaw/plugin-sdk/plugin-entry";
import { Type } from "@sinclair/typebox";
import { spawn, type ChildProcess } from "node:child_process";
import { createInterface, type Interface as ReadlineInterface } from "node:readline";

const CALL_TIMEOUT_MS = 120_000;

// ---------------------------------------------------------------------------
// MCP JSON-RPC client - talks to `npcterm` over stdio
// ---------------------------------------------------------------------------

interface PendingEntry {
  resolve: (v: unknown) => void;
  reject: (e: Error) => void;
  timer: ReturnType<typeof setTimeout>;
}

interface McpClient {
  proc: ChildProcess;
  rl: ReadlineInterface;
  nextId: number;
  pending: Map<number, PendingEntry>;
  initialized: Promise<void>;
}

function destroyClient(client: McpClient) {
  client.rl.close();
  for (const [, entry] of client.pending) {
    clearTimeout(entry.timer);
    entry.reject(new Error("npcterm process exited"));
  }
  client.pending.clear();
  if (client.proc.exitCode === null) {
    client.proc.kill();
  }
}

function createMcpClient(binaryPath: string): McpClient {
  const proc = spawn(binaryPath, [], {
    stdio: ["pipe", "pipe", "ignore"],
    env: { ...process.env },
  });

  const rl = createInterface({ input: proc.stdout! });
  const client: McpClient = {
    proc,
    rl,
    nextId: 0,
    pending: new Map(),
    initialized: Promise.resolve(),
  };

  rl.on("line", (line) => {
    try {
      const msg = JSON.parse(line);
      if (msg.id !== undefined) {
        const entry = client.pending.get(msg.id);
        if (entry) {
          clearTimeout(entry.timer);
          client.pending.delete(msg.id);
          if (msg.error) {
            entry.reject(new Error(msg.error.message || JSON.stringify(msg.error)));
          } else {
            entry.resolve(msg.result);
          }
        }
      }
    } catch {
      // ignore non-JSON lines
    }
  });

  proc.on("exit", () => destroyClient(client));

  // MCP initialize handshake
  client.initialized = (async () => {
    await rpcCall(client, "initialize", {
      protocolVersion: "2024-11-05",
      capabilities: {},
      clientInfo: { name: "openclaw-npcterm", version: "1.0.0" },
    });
    proc.stdin!.write(JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" }) + "\n");
  })();

  return client;
}

function rpcCall(client: McpClient, method: string, params?: unknown): Promise<unknown> {
  return new Promise((resolve, reject) => {
    client.nextId++;
    const id = client.nextId;
    const timer = setTimeout(() => {
      client.pending.delete(id);
      reject(new Error(`npcterm RPC timeout: ${method}`));
    }, CALL_TIMEOUT_MS);
    client.pending.set(id, { resolve, reject, timer });
    client.proc.stdin!.write(JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n");
  });
}

interface ToolResult {
  content?: Array<{ type: string; text?: string }>;
  isError?: boolean;
}

async function callTool(client: McpClient, toolName: string, args?: Record<string, unknown>): Promise<string> {
  await client.initialized;
  const result = await rpcCall(client, "tools/call", { name: toolName, arguments: args ?? {} }) as ToolResult;

  if (result.isError) {
    const text = result.content?.map((c) => c.text ?? "").join("\n") || "Unknown error";
    throw new Error(text);
  }

  return result.content?.map((c) => c.text ?? "").join("\n") || "";
}

// ---------------------------------------------------------------------------
// Plugin entry
// ---------------------------------------------------------------------------

export default definePluginEntry({
  id: "npcterm",
  name: "npcterm",
  description: "Headless, in-memory terminal emulator for AI agents",

  register(api) {
    const config = api.pluginConfig as {
      binaryPath?: string;
    };
    const binaryPath = config.binaryPath || "npcterm";

    let client: McpClient | null = null;
    let clientCreating = false;

    function getClient(): McpClient {
      if (!client || client.proc.exitCode !== null) {
        if (clientCreating) return client!;
        clientCreating = true;
        client = createMcpClient(binaryPath);
        clientCreating = false;
      }
      return client;
    }

    function proxyTool(
      name: string,
      description: string,
      parameters: ReturnType<typeof Type.Object>,
    ) {
      api.registerTool({
        name,
        description,
        parameters,
        async execute(_id, params) {
          const text = await callTool(getClient(), name, params as Record<string, unknown>);
          return { content: [{ type: "text" as const, text }] };
        },
      });
    }

    // -- terminal_create ------------------------------------------------------
    proxyTool("terminal_create",
      "Create a new terminal instance (80x24, 120x40, 160x40, or 200x50)",
      Type.Object({
        size: Type.Optional(Type.String({ description: "Terminal size: 80x24 (default), 120x40, 160x40, 200x50" })),
        shell: Type.Optional(Type.String({ description: "Shell path (e.g. /bin/zsh)" })),
      }),
    );

    // -- terminal_destroy -----------------------------------------------------
    proxyTool("terminal_destroy",
      "Destroy a terminal and kill its PTY process",
      Type.Object({
        id: Type.String({ description: "Terminal ID" }),
      }),
    );

    // -- terminal_list --------------------------------------------------------
    proxyTool("terminal_list",
      "List all active terminals with id, size, state, and running command",
      Type.Object({}),
    );

    // -- terminal_send_key ----------------------------------------------------
    proxyTool("terminal_send_key",
      "Send a single keystroke (Enter, Tab, Ctrl+c, etc.)",
      Type.Object({
        id: Type.String({ description: "Terminal ID" }),
        key: Type.String({ description: "Key name (e.g. 'Enter', 'Ctrl+c', 'a')" }),
      }),
    );

    // -- terminal_send_keys ---------------------------------------------------
    proxyTool("terminal_send_keys",
      "Send a batch of text and special keys in one call",
      Type.Object({
        id: Type.String({ description: "Terminal ID" }),
        input: Type.Array(Type.Object({
          text: Type.Optional(Type.String({ description: "Raw text to type" })),
          key: Type.Optional(Type.String({ description: "Special key name (Enter, Tab, etc.)" })),
        }), { description: "Array of {text} or {key} items" }),
      }),
    );

    // -- terminal_mouse -------------------------------------------------------
    proxyTool("terminal_mouse",
      "Send mouse events (left_click, right_click, double_click, move, get_position, set_position)",
      Type.Object({
        id: Type.String({ description: "Terminal ID" }),
        action: Type.String({ description: "Mouse action: left_click, right_click, double_click, move, get_position, set_position" }),
        col: Type.Optional(Type.Number({ description: "Column" })),
        row: Type.Optional(Type.Number({ description: "Row" })),
      }),
    );

    // -- terminal_read_screen -------------------------------------------------
    proxyTool("terminal_read_screen",
      "Read terminal screen with coordinate overlay. Use mode 'changes' for incremental reads",
      Type.Object({
        id: Type.String({ description: "Terminal ID" }),
        mode: Type.Optional(Type.String({ description: "Read mode: 'full' or 'changes'" })),
        max_lines: Type.Optional(Type.Number({ description: "Max lines in 'changes' mode (1-200, default 50)" })),
      }),
    );

    // -- terminal_show_screen -------------------------------------------------
    proxyTool("terminal_show_screen",
      "Read terminal screen as plain text without coordinates",
      Type.Object({
        id: Type.String({ description: "Terminal ID" }),
      }),
    );

    // -- terminal_read_rows ---------------------------------------------------
    proxyTool("terminal_read_rows",
      "Read specific rows from terminal screen with coordinate overlay",
      Type.Object({
        id: Type.String({ description: "Terminal ID" }),
        start_row: Type.Number({ description: "Start row" }),
        end_row: Type.Number({ description: "End row" }),
      }),
    );

    // -- terminal_read_region -------------------------------------------------
    proxyTool("terminal_read_region",
      "Read a rectangular region from terminal screen",
      Type.Object({
        id: Type.String({ description: "Terminal ID" }),
        col1: Type.Number({ description: "Start column" }),
        row1: Type.Number({ description: "Start row" }),
        col2: Type.Number({ description: "End column" }),
        row2: Type.Number({ description: "End row" }),
      }),
    );

    // -- terminal_status ------------------------------------------------------
    proxyTool("terminal_status",
      "Get terminal status: process state, cursor, dirty rows, pending events",
      Type.Object({
        id: Type.String({ description: "Terminal ID" }),
        last_n_lines: Type.Optional(Type.Number({ description: "Number of trailing screen lines to include" })),
      }),
    );

    // -- terminal_poll_events -------------------------------------------------
    proxyTool("terminal_poll_events",
      "Drain pending terminal events (CommandFinished, WaitingForInput, Bell, etc.)",
      Type.Object({
        id: Type.String({ description: "Terminal ID" }),
      }),
    );

    // -- terminal_select ------------------------------------------------------
    proxyTool("terminal_select",
      "Select text by coordinate range and return it",
      Type.Object({
        id: Type.String({ description: "Terminal ID" }),
        start_col: Type.Number({ description: "Start column" }),
        start_row: Type.Number({ description: "Start row" }),
        end_col: Type.Number({ description: "End column" }),
        end_row: Type.Number({ description: "End row" }),
      }),
    );

    // -- terminal_scroll ------------------------------------------------------
    proxyTool("terminal_scroll",
      "Scroll terminal: page_up, page_down, or search (requires 'text' param)",
      Type.Object({
        id: Type.String({ description: "Terminal ID" }),
        action: Type.String({ description: "Scroll action: page_up, page_down, or search" }),
        text: Type.Optional(Type.String({ description: "Search text (required for 'search' action)" })),
      }),
    );

    // -- viewer_start ---------------------------------------------------------
    proxyTool("viewer_start",
      "Start the web debug viewer (default port 8039)",
      Type.Object({
        port: Type.Optional(Type.Number({ description: "Port to bind (default 8039)" })),
      }),
    );

    // -- viewer_stop ----------------------------------------------------------
    proxyTool("viewer_stop",
      "Stop the web debug viewer",
      Type.Object({}),
    );

    // -- viewer_open ----------------------------------------------------------
    proxyTool("viewer_open",
      "Open the debug viewer in the system browser (starts it if needed)",
      Type.Object({
        port: Type.Optional(Type.Number({ description: "Port to bind if starting viewer (default 8039)" })),
      }),
    );

    // -- cleanup on gateway shutdown -----------------------------------------
    api.on("shutdown", () => {
      if (client) destroyClient(client);
    });

    api.logger.info("npcterm plugin registered");
  },
});
