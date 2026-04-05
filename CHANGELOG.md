# Changelog

All notable changes to NPCterm39 will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [1.0.0] - 2026-04-05

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

- Ported from term39 (Tauri-based terminal UI), stripped of all UI code
- macOS only for process state detection
- No test suite yet
