use serde_json::{Value, json};

use crate::input::keys::Key;
use crate::input::mouse::MouseAction;
use crate::manager::instance::TerminalInstance;
use crate::manager::registry::TerminalRegistry;

use super::types::{ToolCallResult, ToolDef};

fn get_id<'a>(args: &'a Value) -> Result<&'a str, ToolCallResult> {
    args.get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolCallResult::error("Missing 'id' parameter".into()))
}

fn get_instance<'a>(
    registry: &'a mut TerminalRegistry,
    args: &Value,
) -> Result<&'a mut TerminalInstance, ToolCallResult> {
    let id = get_id(args)?;
    registry
        .get_mut(id)
        .ok_or_else(|| ToolCallResult::error(format!("Terminal '{}' not found", id)))
}

/// Return all tool definitions
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "terminal_create".into(),
            description: "Create a new terminal instance. Returns {id, cols, rows}. The id is required for all subsequent terminal operations. Available sizes: 80x24 (default), 120x40, 160x40, 200x50.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "size": { "type": "string", "enum": ["80x24", "120x40", "160x40", "200x50"], "default": "80x24" },
                    "shell": { "type": "string", "description": "Shell path (optional)" }
                }
            }),
        },
        ToolDef {
            name: "terminal_destroy".into(),
            description: "Destroy a terminal instance and kill its PTY process. Returns {success: bool}.".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "id": { "type": "string" } },
                "required": ["id"]
            }),
        },
        ToolDef {
            name: "terminal_list".into(),
            description: "List all terminal instances with id, size, state, and running command.".into(),
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        ToolDef {
            name: "terminal_send_key".into(),
            description: "Send a single keystroke. Supports: a-z, Enter, Tab, Escape, Backspace, Delete, arrows, Home, End, PageUp, PageDown, F1-F12, Ctrl+key, Alt+key, space. For multiple keystrokes or text input, use terminal_send_keys instead.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "key": { "type": "string", "description": "Key name (e.g. 'Enter', 'Ctrl+c', 'a')" }
                },
                "required": ["id", "key"]
            }),
        },
        ToolDef {
            name: "terminal_send_keys".into(),
            description: "Send a batch of text and special keys in one call (preferred over terminal_send_key for multi-step input). Each element is either {\"text\":\"string\"} for raw text or {\"key\":\"Enter\"} for special keys (Enter, Tab, Escape, Ctrl+c, etc). Example: [{\"text\":\"echo hello\"},{\"key\":\"Enter\"}]".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "input": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "text": { "type": "string", "description": "Raw text to type" },
                                "key": { "type": "string", "description": "Special key name (Enter, Tab, Ctrl+c, etc)" }
                            }
                        }
                    }
                },
                "required": ["id", "input"]
            }),
        },
        ToolDef {
            name: "terminal_mouse".into(),
            description: "Perform a mouse action. col and row required for left_click, right_click, double_click, move, set_position. get_position returns current cursor coords.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "action": { "type": "string", "enum": ["left_click", "right_click", "double_click", "move", "get_position", "set_position"] },
                    "col": { "type": "integer" },
                    "row": { "type": "integer" }
                },
                "required": ["id", "action"]
            }),
        },
        ToolDef {
            name: "terminal_read_screen".into(),
            description: "Read terminal screen with coordinate overlay. Use mode 'changes' to see only new output since your last read (default 50 lines, max 200). Default mode 'full' returns the complete screen.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "mode": { "type": "string", "enum": ["full", "changes"], "default": "full" },
                    "max_lines": { "type": "integer", "default": 50, "description": "Max lines to return in 'changes' mode (1-200)" }
                },
                "required": ["id"]
            }),
        },
        ToolDef {
            name: "terminal_show_screen".into(),
            description: "Return terminal screen as plain text without coordinates. Lighter output, best for reading content or passing to other tools.".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "id": { "type": "string" } },
                "required": ["id"]
            }),
        },
        ToolDef {
            name: "terminal_read_rows".into(),
            description: "Read specific rows from terminal screen with coordinate overlay (column headers + row numbers).".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "start_row": { "type": "integer" },
                    "end_row": { "type": "integer" }
                },
                "required": ["id", "start_row", "end_row"]
            }),
        },
        ToolDef {
            name: "terminal_read_region".into(),
            description: "Read a rectangular region from terminal screen with row numbers.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "col1": { "type": "integer" }, "row1": { "type": "integer" },
                    "col2": { "type": "integer" }, "row2": { "type": "integer" }
                },
                "required": ["id", "col1", "row1", "col2", "row2"]
            }),
        },
        ToolDef {
            name: "terminal_status".into(),
            description: "Get terminal status: process state, running command, cursor position, dirty rows, last N screen lines, pending event count, and scrollback depth. Token-efficient alternative to reading the full screen.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "last_n_lines": { "type": "integer", "default": 5 }
                },
                "required": ["id"]
            }),
        },
        ToolDef {
            name: "terminal_poll_events".into(),
            description: "Drain pending terminal events (destructive: events are removed after reading). Event types: CommandFinished, WaitingForInput, Bell, ProcessStateChanged, ScreenChanged.".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "id": { "type": "string" } },
                "required": ["id"]
            }),
        },
        ToolDef {
            name: "terminal_select".into(),
            description: "Select text by coordinate range (like click-drag) and return it. Unlike read_region, this is a logical text selection that can span across lines.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "start_col": { "type": "integer" }, "start_row": { "type": "integer" },
                    "end_col": { "type": "integer" }, "end_row": { "type": "integer" }
                },
                "required": ["id", "start_col", "start_row", "end_col", "end_row"]
            }),
        },
        ToolDef {
            name: "terminal_scroll".into(),
            description: "Navigate scrollback history: page_up, page_down, or search (requires 'text' param). Returns {scroll_offset} (0 = bottom). Use terminal_read_screen after scrolling to see the result.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "action": { "type": "string", "enum": ["page_up", "page_down", "search"] },
                    "text": { "type": "string" }
                },
                "required": ["id", "action"]
            }),
        },
    ]
}

/// Handle a tool call
pub fn handle_tool_call(
    registry: &mut TerminalRegistry,
    tool_name: &str,
    args: &Value,
) -> ToolCallResult {
    match tool_name {
        "terminal_create" => {
            let size = args.get("size").and_then(|v| v.as_str()).unwrap_or("80x24");
            let (cols, rows) = match size {
                "120x40" => (120, 40),
                "160x40" => (160, 40),
                "200x50" => (200, 50),
                _ => (80, 24),
            };
            let shell = args.get("shell").and_then(|v| v.as_str());
            match registry.create(cols, rows, shell) {
                Ok(id) => ToolCallResult::text(json!({ "id": id, "cols": cols, "rows": rows }).to_string()),
                Err(e) => ToolCallResult::error(e),
            }
        }

        "terminal_destroy" => {
            let id = match get_id(args) {
                Ok(id) => id,
                Err(e) => return e,
            };
            ToolCallResult::text(json!({ "success": registry.destroy(id) }).to_string())
        }

        "terminal_list" => {
            ToolCallResult::text(json!({ "terminals": registry.list() }).to_string())
        }

        "terminal_send_key" => {
            let key_str = match args.get("key").and_then(|v| v.as_str()) {
                Some(k) => k,
                None => return ToolCallResult::error("Missing 'key' parameter".into()),
            };
            let key = match Key::from_str(key_str) {
                Ok(k) => k,
                Err(e) => return ToolCallResult::error(e),
            };
            let instance = match get_instance(registry, args) {
                Ok(i) => i,
                Err(e) => return e,
            };
            match instance.send_key(key) {
                Ok(_) => ToolCallResult::text(json!({ "success": true }).to_string()),
                Err(e) => ToolCallResult::error(format!("Failed to send key: {}", e)),
            }
        }

        "terminal_send_keys" => {
            let instance = match get_instance(registry, args) {
                Ok(i) => i,
                Err(e) => return e,
            };
            let input_arr = match args.get("input").and_then(|v| v.as_array()) {
                Some(k) => k,
                None => return ToolCallResult::error("Missing 'input' parameter".into()),
            };
            for item in input_arr {
                if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                    if let Err(e) = instance.write_raw(text.as_bytes()) {
                        return ToolCallResult::error(format!("Failed to send text: {}", e));
                    }
                } else if let Some(key_str) = item.get("key").and_then(|v| v.as_str()) {
                    let key = match Key::from_str(key_str) {
                        Ok(k) => k,
                        Err(e) => return ToolCallResult::error(e),
                    };
                    if let Err(e) = instance.send_key_no_flush(key) {
                        return ToolCallResult::error(format!("Failed to send key: {}", e));
                    }
                } else {
                    return ToolCallResult::error("Each input element must have 'text' or 'key'".into());
                }
            }
            let _ = instance.flush_input();
            ToolCallResult::text(json!({ "success": true }).to_string())
        }

        "terminal_mouse" => {
            let action_str = match args.get("action").and_then(|v| v.as_str()) {
                Some(a) => a,
                None => return ToolCallResult::error("Missing 'action' parameter".into()),
            };
            let col = args.get("col").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
            let row = args.get("row").and_then(|v| v.as_u64()).unwrap_or(0) as u16;

            let action = match action_str {
                "left_click" => MouseAction::LeftClick { col, row },
                "right_click" => MouseAction::RightClick { col, row },
                "double_click" => MouseAction::DoubleClick { col, row },
                "move" => MouseAction::MoveTo { col, row },
                "get_position" => MouseAction::GetPosition,
                "set_position" => MouseAction::SetPosition { col, row },
                _ => return ToolCallResult::error(format!("Unknown action: {}", action_str)),
            };

            let instance = match get_instance(registry, args) {
                Ok(i) => i,
                Err(e) => return e,
            };
            let result = instance.send_mouse(action);
            ToolCallResult::text(serde_json::to_string(&result).unwrap())
        }

        "terminal_read_screen" => {
            let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("full");
            let max_lines = args
                .get("max_lines")
                .and_then(|v| v.as_u64())
                .map(|v| (v as usize).clamp(1, 200))
                .unwrap_or(50);
            let instance = match get_instance(registry, args) {
                Ok(i) => i,
                Err(e) => return e,
            };
            match mode {
                "changes" => ToolCallResult::text(instance.read_changes(max_lines)),
                _ => ToolCallResult::text(instance.read_screen()),
            }
        }

        "terminal_show_screen" => {
            let instance = match get_instance(registry, args) {
                Ok(i) => i,
                Err(e) => return e,
            };
            ToolCallResult::text(instance.show_screen())
        }

        "terminal_read_rows" => {
            let start = args.get("start_row").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let end = args.get("end_row").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let instance = match get_instance(registry, args) {
                Ok(i) => i,
                Err(e) => return e,
            };
            ToolCallResult::text(instance.read_rows(start, end))
        }

        "terminal_read_region" => {
            let col1 = args.get("col1").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let row1 = args.get("row1").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let col2 = args.get("col2").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let row2 = args.get("row2").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let instance = match get_instance(registry, args) {
                Ok(i) => i,
                Err(e) => return e,
            };
            ToolCallResult::text(instance.read_region(col1, row1, col2, row2))
        }

        "terminal_status" => {
            let last_n = args.get("last_n_lines").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
            let instance = match get_instance(registry, args) {
                Ok(i) => i,
                Err(e) => return e,
            };
            ToolCallResult::text(serde_json::to_string(&instance.get_status(last_n)).unwrap())
        }

        "terminal_poll_events" => {
            let instance = match get_instance(registry, args) {
                Ok(i) => i,
                Err(e) => return e,
            };
            let events = instance.poll_events();
            ToolCallResult::text(json!({ "events": events }).to_string())
        }

        "terminal_select" => {
            let sc = args.get("start_col").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let sr = args.get("start_row").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let ec = args.get("end_col").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let er = args.get("end_row").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let instance = match get_instance(registry, args) {
                Ok(i) => i,
                Err(e) => return e,
            };
            let text = instance.select_range(sc, sr, ec, er);
            ToolCallResult::text(json!({ "selected_text": text }).to_string())
        }

        "terminal_scroll" => {
            let action = match args.get("action").and_then(|v| v.as_str()) {
                Some(a) => a,
                None => return ToolCallResult::error("Missing 'action' parameter".into()),
            };
            let search_text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let instance = match get_instance(registry, args) {
                Ok(i) => i,
                Err(e) => return e,
            };
            match action {
                "page_up" => {
                    let offset = instance.scroll_page_up();
                    ToolCallResult::text(json!({ "scroll_offset": offset }).to_string())
                }
                "page_down" => {
                    let offset = instance.scroll_page_down();
                    ToolCallResult::text(json!({ "scroll_offset": offset }).to_string())
                }
                "search" => {
                    if search_text.is_empty() {
                        return ToolCallResult::error("Missing 'text' for search".into());
                    }
                    let (offset, found) = instance.scroll_to_text(search_text);
                    ToolCallResult::text(json!({ "scroll_offset": offset, "found": found }).to_string())
                }
                _ => ToolCallResult::error(format!("Unknown scroll action: {}", action)),
            }
        }

        _ => ToolCallResult::error(format!("Unknown tool: {}", tool_name)),
    }
}
