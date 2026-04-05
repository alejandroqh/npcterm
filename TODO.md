# NpcTerm39 — Implementation TODO

Headless, in-memory terminal emulator for AI agents, exposed via MCP. Written in Rust.
Reuses terminal emulation code from term39 (`../q39/term39/`).

---

## Phase 1: Project Scaffolding

- [x] **1.1** Create `Cargo.toml` — deps: `portable-pty`, `vte`, `unicode-width`, `serde`, `serde_json`, `tokio`, `uuid`, `chrono`
- [x] **1.2** Create module skeleton (all `mod.rs`, `main.rs`, `lib.rs`)
- [x] **1.3** Verify `cargo check` compiles

## Phase 2: Core Terminal Emulation

Port from term39 `src/term_emu/`, stripped of UI.

- [x] **2.1** Port cell types (`cell.rs`) — `Color`, `NamedColor`, `CellAttributes`, `TerminalCell`, `Cursor`, `CursorShape`, `CharacterSet`. Add `Serialize`. Add `wide: bool` to cell. Source: `term_grid.rs:1-127`
- [x] **2.2** Port `TerminalGrid` (`grid.rs`) — rows, scrollback, cursor, attrs, DEC modes. Add `dirty_rows`, `dirty_flag`, `bell_pending`. Source: `term_grid.rs:130+`
- [x] **2.3** Port `AnsiHandler` (`ansi_handler.rs`) — VTE Perform impl, all CSI/SGR/cursor/scroll. Set `bell_pending` on `\x07`. Source: `ansi_handler.rs`
- [x] **2.4** Port `Selection` (`selection.rs`) — range-based selection, `get_selected_text()`. Source: `selection.rs`

## Phase 3: PTY Management

- [x] **3.1** Port `TerminalEmulator` (`emulator.rs`) — PTY spawn via `portable-pty`, reader thread, `write_input()`, `process_output()`. Fixed sizes only (80x24 or 120x40). Source: `terminal_emulator.rs`
- [x] **3.2** Process state detection — `ProcessState` enum (Running/Idle/WaitingForInput/Exited), `is_alive()`, `get_foreground_process_name()`, `last_output_time` tracking

## Phase 4: Input Handling

- [x] **4.1** Key mapping (`keys.rs`) — `Key` enum (Char, Enter, Tab, Esc, Backspace, Delete, arrows, F1-F12, Ctrl+x, Alt+x). `to_escape_sequence()`, `from_str()` for MCP parsing
- [x] **4.2** Mouse handling (`mouse.rs`) — `MouseAction` enum (LeftClick, RightClick, DoubleClick, MoveTo, GetPosition, SetPosition). `MouseState` tracking. SGR mouse sequences

## Phase 5: Screen Reading

- [x] **5.1** Screen reader (`reader.rs`) — full read with coordinate overlay (tens+units column headers, 2-digit row nums). Partial read by rows. Region read
- [x] **5.2** Cell formatter (`formatter.rs`) — JSON format (char, fg, bg, attrs, wide). Text format (plain chars + overlay). Compact format (omit defaults)

## Phase 6: Status & Events

- [x] **6.1** Status query (`query.rs`) — `TerminalStatus`: state, running_command, last_n_lines, cursor_pos, mouse_pos, dirty, changed_rows, pending_events, size
- [x] **6.2** Event system (`events.rs`) — `TerminalEvent` (CommandFinished, WaitingForInput, Bell, ProcessStateChanged, ScreenChanged). EventQueue per instance

## Phase 7: Text Selection & Scrollback

- [x] **7.1** Command-based selection — `select_range(start, end)` returns text. Handles wide chars
- [x] **7.2** Page-based scrollback — `scroll_page_up()`, `scroll_page_down()`, `scroll_to_text()` (search + jump)

## Phase 8: Terminal Instance Manager

- [x] **8.1** `TerminalInstance` (`instance.rs`) — self-contained: emulator + mouse_state + selection + scroll_offset + event_queue. All methods: `send_key()`, `send_mouse()`, `read_screen()`, `get_status()`, `poll_events()`, `tick()`
- [x] **8.2** `TerminalRegistry` (`registry.rs`) — `HashMap<String, TerminalInstance>`. `create()`, `get()`, `destroy()`, `list()`, `tick_all()`. No globals

## Phase 9: MCP Server

- [x] **9.1** MCP server setup (`server.rs`) — stdio JSON-RPC transport, background tick task
- [x] **9.2** Register MCP tools (`tools.rs`):
  - [x] `terminal_create` — `{ size?, shell? }` → `{ id, cols, rows }`
  - [x] `terminal_destroy` — `{ id }` → `{ success }`
  - [x] `terminal_list` — → `{ terminals[] }`
  - [x] `terminal_send_key` — `{ id, key }` → `{ success }`
  - [x] `terminal_send_keys` — `{ id, keys[] }` → `{ success, count }`
  - [x] `terminal_mouse` — `{ id, action, col?, row? }` → `{ mouse_col, mouse_row, selected_text? }`
  - [x] `terminal_read_screen` — `{ id, format?, overlay? }` → screen content
  - [x] `terminal_read_rows` — `{ id, start_row, end_row }` → partial content
  - [x] `terminal_read_region` — `{ id, col1, row1, col2, row2 }` → region content
  - [x] `terminal_status` — `{ id, last_n_lines? }` → lightweight status
  - [x] `terminal_poll_events` — `{ id }` → `{ events[] }`
  - [x] `terminal_select` — `{ id, start_col, start_row, end_col, end_row }` → `{ selected_text }`
  - [x] `terminal_scroll` — `{ id, action, text? }` → `{ scroll_offset, found? }`
- [x] **9.3** Request/response serde types (`types.rs`)

## Phase 10: Testing

- [ ] **10.1** Unit tests — grid, ANSI, keys, mouse, dirty tracking
- [ ] **10.2** Integration tests — spawn terminal, send commands, read screen, verify output
- [ ] **10.3** MCP protocol tests — JSON-RPC roundtrip, lifecycle, error handling

---

## Module Structure

```
src/
  main.rs              lib.rs
  terminal/   cell.rs  grid.rs  ansi_handler.rs  emulator.rs  selection.rs
  input/      keys.rs  mouse.rs
  screen/     reader.rs  formatter.rs
  status/     query.rs  events.rs
  manager/    instance.rs  registry.rs
  mcp/        server.rs  tools.rs  types.rs
```

## Dependency Order

```
Phase 1 → 2 → 3 → (4, 5, 6, 7 parallel) → 8 → 9 → 10
```

## Reference Files (term39)

| Port from | Source |
|---|---|
| Cell types, Grid | `src/term_emu/term_grid.rs` |
| ANSI handler | `src/term_emu/ansi_handler.rs` |
| PTY / Emulator | `src/term_emu/terminal_emulator.rs` |
| Selection | `src/term_emu/selection.rs` |
| Function key seqs | `src/input/keyboard_handlers.rs:27-43` |
