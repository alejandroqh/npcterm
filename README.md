# NPCterm39

A headless, in-memory terminal emulator for AI agents, exposed via [MCP](https://modelcontextprotocol.io/) (Model Context Protocol). Written in Rust.

NPCterm39 gives AI agents **full terminal access** -- the ability to spawn shells, run arbitrary commands, read screen output, send keystrokes, and interact with TUI applications. This is one of the most powerful capabilities you can grant an AI agent: it is effectively equivalent to giving it access to a computer.

> **Use with precautions.** A terminal is an unrestricted execution environment. Any command the agent can type, the system will run. This includes installing software, modifying files, accessing the network, and anything else a shell user can do. Deploy NPCterm39 in sandboxed or controlled environments, and always apply the principle of least privilege. Do not expose it to untrusted agents without appropriate safeguards.

## Features

- **Full ANSI/VT100 terminal emulation** with PTY spawning via `portable-pty`
- **15 MCP tools** for complete terminal control over JSON-RPC stdio
- **Incremental screen reads** with dirty-row tracking for efficient output consumption
- **Process state detection** -- knows when a command is running, idle, waiting for input, or exited
- **Event system** -- ring buffer of terminal events (CommandFinished, WaitingForInput, Bell, etc.)
- **AI-friendly coordinate overlay** for precise screen navigation
- **Mouse, selection, and scroll support** for interacting with TUI applications
- **Multiple concurrent terminals** with short 2-character IDs

## Requirements

- Rust 2024 edition (1.85+)
- macOS (process state detection uses `ps` heuristics tuned for macOS)

## Build

```bash
cargo build --release
```

The optimized binary is built with LTO and stripped symbols.

## Usage

NPCterm39 is an MCP server. It communicates over stdin/stdout using JSON-RPC. To use it, configure it as an MCP server in your AI agent's MCP configuration.

### MCP Configuration Example

```json
{
  "mcpServers": {
    "npcterm39": {
      "command": "/path/to/npcterm39"
    }
  }
}
```

### Available Tools

| Tool | Description |
|------|-------------|
| `terminal_create` | Spawn a new terminal (80x24 or 120x40) |
| `terminal_destroy` | Destroy a terminal and its PTY |
| `terminal_list` | List all active terminals |
| `terminal_send_key` | Send a single keystroke |
| `terminal_send_keys` | Send a sequence of keystrokes |
| `terminal_mouse` | Send mouse events (click, scroll, drag) |
| `terminal_read_screen` | Read the full screen buffer |
| `terminal_show_screen` | Read screen with coordinate overlay headers |
| `terminal_read_rows` | Read specific rows from the screen |
| `terminal_read_region` | Read a rectangular region of the screen |
| `terminal_status` | Get terminal status and process state |
| `terminal_poll_events` | Poll the event queue |
| `terminal_select` | Select text on screen |
| `terminal_scroll` | Scroll the terminal viewport |

### Example: Yes, your AI can quit Vim

```jsonc
// 1. Create a terminal
// -> terminal_create {}
// <- {"id": "a0", "cols": 80, "rows": 24}

// 2. Open vim
// -> terminal_send_keys {"id": "a0", "input": [{"text": "vim"}, {"key": "Enter"}]}
// <- {"success": true}

// 3. Read the screen to confirm vim is open
// -> terminal_show_screen {"id": "a0"}
// <- ~                              VIM - Vi IMproved
// <- ~                               version 9.2.250
// <- ~                           by Bram Moolenaar et al.
// <- ~                type  :q<Enter>               to exit
// <- ...

// 4. Quit vim (the hard part, apparently)
// -> terminal_send_keys {"id": "a0", "input": [{"text": ":q"}, {"key": "Enter"}]}
// <- {"success": true}

// Back at the shell. First try.
```

NPCterm39 gives AI agents full TUI interaction -- opening, navigating, and closing interactive programs like `vim`, `htop`, `less`, or any curses-based application.

## Architecture

```
MCP Server (stdio JSON-RPC)
       |
  Tool Handlers (15 tools)
       |
  TerminalRegistry (concurrent terminal management)
       |
  TerminalInstance (emulator + mouse + selection + events)
       |
  TerminalEmulator (PTY spawn, I/O threads, grid)
    |-- TerminalGrid (screen buffer, scrollback, dirty tracking)
    |   '-- AnsiHandler (VTE parser)
    |-- TerminalCell (character + style attributes)
    '-- PTY (portable-pty)
```

Each terminal spawns a background PTY reader thread. A global tick thread (10ms interval) drains PTY output through the VTE parser, detects process state changes, and emits events.

## Security Considerations

NPCterm39 provides **unrestricted shell access** to whatever agent connects to it. Before deploying:

- **Sandbox the environment.** Run inside containers, VMs, or other isolation boundaries.
- **Limit the agent's permissions.** Use restricted user accounts, filesystem permissions, and network policies.
- **Monitor activity.** Log terminal events and review agent behavior.
- **Do not run as root.** The PTY inherits the permissions of the NPCterm39 process.
- **Treat this as you would SSH access.** If you wouldn't give the agent an SSH session to the machine, don't give it NPCterm39 either.

## License

MIT

## Author

Alejandro Quintanar
