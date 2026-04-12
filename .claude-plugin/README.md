# NPCterm - Claude Bundle Plugin

This directory makes NPCterm installable as an OpenClaw **Claude bundle**.

## Install

```bash
openclaw plugins install git@github.com:alejandroqh/npcterm.git
```

## What it provides

OpenClaw detects this as a Claude bundle and maps the MCP server config from `.mcp.json`.
The npcterm binary runs as a stdio MCP subprocess, exposing all tools:

- `terminal_create` - spawn a new terminal (80x24, 120x40, 160x40, or 200x50)
- `terminal_destroy` - destroy a terminal and kill its PTY process
- `terminal_list` - list all active terminals
- `terminal_send_key` - send a single keystroke
- `terminal_send_keys` - send a batch of text and special keys
- `terminal_mouse` - send mouse events (click, scroll, drag)
- `terminal_read_screen` - read screen with coordinate overlay (full or incremental)
- `terminal_show_screen` - read screen as plain text
- `terminal_read_rows` - read specific rows from the screen
- `terminal_read_region` - read a rectangular region
- `terminal_status` - get terminal status, process state, and has_new_content flag
- `terminal_poll_events` - poll the event queue
- `terminal_select` - select text on screen
- `terminal_scroll` - scroll the terminal viewport
- `viewer_start` - start the web debug viewer
- `viewer_stop` - stop the web debug viewer
- `viewer_open` - open the debug viewer in the system browser

## Requirements

The `npcterm` binary must be in PATH, or update `.mcp.json` to point to the binary location.

Install via Cargo:

```bash
cargo install npcterm
```

Or download a pre-built binary from the `dist/` directory.
