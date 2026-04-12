# Changelog

All notable changes to NPCterm39 will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [v1.3.0] - 2026-04-08

### Added

- Built-in web debug viewer with live terminal rendering, cursor tracking, and full color support via WebSocket
- Activity log sidebar in the viewer showing every MCP tool call with timestamps, parameters, and color-coded categories
- 3 new MCP tools: `viewer_start`, `viewer_stop`, `viewer_open` for on-demand viewer control
- `viewer_start` auto-probes ports 8039-8049 if the default port is busy
- `viewer_open` starts the viewer and opens the system browser in one call (macOS, Linux, Windows)
- Graceful shutdown support for the viewer server via oneshot channel
- Viewer dirty-row tracking independent from agent reads and tick events
- "For Agent Builders" section in README highlighting token-efficient patterns
- "Debug Viewer" section in README
- Demo script at `docs/demo-script.md` for recording showcase videos

### Changed

- Viewer is now on-demand (started via MCP tools) instead of auto-starting at launch
- Default viewer port changed from 3939 to 8039
- Viewer binds to `127.0.0.1` (localhost only) instead of `0.0.0.0`
- Removed `--port` and `--no-viewer` CLI flags (replaced by MCP tool control)
- All HTML UI elements now use monospace font consistently
- Tool count updated from 14 to 17 across README and architecture docs

## [v1.2.0] - 2026-04-07

### Changed

- Migrated MCP server layer from hand-rolled JSON-RPC implementation to [TurboMCP](https://github.com/Epistates/turbomcp) 3.0 SDK
- Server is now async, powered by tokio (`current_thread` runtime)
- Tool definitions use `#[tool]` attribute macros with auto-generated JSON schemas from Rust function signatures
- MCP protocol version support expanded: `2024-11-05`, `2025-06-18`, and `2025-11-25` (multi-version negotiation)

### Removed

- Custom JSON-RPC types (`JsonRpcRequest`, `JsonRpcResponse`, `ToolDef`, `ToolCallResult`) — replaced by TurboMCP's protocol layer
- Manual tool definitions and dispatch logic (624 lines replaced by ~280 lines of annotated methods)

### Notes

- No changes to terminal emulation, PTY handling, input, screen reading, or event system
- External behavior (tool names, descriptions, parameters, responses) is unchanged
- Binary size increased slightly due to tokio/turbomcp dependencies

## [v1.1.0] - 2026-04-06

### Added

- Incremental screen reads: `terminal_read_screen` now supports `mode: "changes"` to return only new output since the last read, with configurable `max_lines` (1-200, default 50)
- Wider terminal sizes: 160x40 and 200x50 in addition to 80x24 and 120x40
- `has_new_content` field in `terminal_status` response indicating unread output
- Independent read-dirty tracking in grid (separate from tick/event dirty state)
- OpenClaw plugin support with `.mcp.json` bundle configuration
- OpenClaw install instructions in README
- Expanded install section: `cargo install`, pre-built binaries, and build from source
- Claude Desktop / Claude Code setup instructions in README

## [v1.0.0] - 2026-04-05

### Added

- Full ANSI/VT100 terminal emulation with PTY spawning via `portable-pty`
- MCP server exposing 15 tools over JSON-RPC stdio transport
- Terminal lifecycle management: `terminal_create`, `terminal_destroy`, `terminal_list`
- Input tools: `terminal_send_key`, `terminal_send_keys`, `terminal_mouse`
- Screen reading tools: `terminal_read_screen`, `terminal_show_screen`, `terminal_read_rows`, `terminal_read_region`
- AI-friendly coordinate overlay for screen navigation (`terminal_show_screen`)
- Terminal status and process state detection (`terminal_status`)
- Event system with ring buffer: CommandFinished, WaitingForInput, Bell, ProcessStateChanged, ScreenChanged (`terminal_poll_events`)
- Text selection support (`terminal_select`)
- Scroll support (`terminal_scroll`)
- Multiple concurrent terminals with 2-character base-36 IDs
- Fixed terminal sizes: 80x24 and 120x40
- Dirty row tracking for incremental screen reads
- Synchronized output support (CSI 2026 h/l)
- Background tick thread (10ms) for PTY drain and state detection
- Process state heuristics for macOS: Running, Idle, WaitingForInput, Exited
- Release profile with LTO, stripped symbols, and single codegen unit

### Notes

- Ported from term39 
