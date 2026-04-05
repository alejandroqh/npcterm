use serde_json::{Value, json};

use crate::input::keys::Key;
use crate::input::mouse::MouseAction;
use crate::manager::registry::TerminalRegistry;
use crate::screen::formatter::cell_to_info;

use super::types::{ToolCallResult, ToolDef};

/// Return all tool definitions
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "terminal_create".into(),
            description: "Create a new terminal instance".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "size": { "type": "string", "enum": ["80x24", "120x40"], "default": "80x24", "description": "Terminal size" },
                    "shell": { "type": "string", "description": "Shell path (optional, uses system default)" }
                }
            }),
        },
        ToolDef {
            name: "terminal_destroy".into(),
            description: "Destroy a terminal instance".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Terminal instance ID" }
                },
                "required": ["id"]
            }),
        },
        ToolDef {
            name: "terminal_list".into(),
            description: "List all terminal instances".into(),
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        ToolDef {
            name: "terminal_send_key".into(),
            description: "Send a keystroke to terminal. Keys: a-z, Enter, Tab, Escape, Backspace, Delete, Up, Down, Left, Right, Home, End, PageUp, PageDown, F1-F12, Ctrl+c, Alt+x, space".into(),
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
            description: "Send multiple keystrokes to terminal (batch)".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "keys": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["id", "keys"]
            }),
        },
        ToolDef {
            name: "terminal_mouse".into(),
            description: "Perform a mouse action on the terminal".into(),
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
            description: "Read full terminal screen with coordinate overlay".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "format": { "type": "string", "enum": ["text", "json"], "default": "text" }
                },
                "required": ["id"]
            }),
        },
        ToolDef {
            name: "terminal_read_rows".into(),
            description: "Read specific rows from terminal screen".into(),
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
            description: "Read a rectangular region from terminal screen".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "col1": { "type": "integer" },
                    "row1": { "type": "integer" },
                    "col2": { "type": "integer" },
                    "row2": { "type": "integer" }
                },
                "required": ["id", "col1", "row1", "col2", "row2"]
            }),
        },
        ToolDef {
            name: "terminal_status".into(),
            description: "Get lightweight terminal status (token-optimized, no full screen)".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "last_n_lines": { "type": "integer", "default": 5, "description": "Number of last lines to include" }
                },
                "required": ["id"]
            }),
        },
        ToolDef {
            name: "terminal_poll_events".into(),
            description: "Get and clear pending terminal events (bell, command finished, screen changed, etc.)".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" }
                },
                "required": ["id"]
            }),
        },
        ToolDef {
            name: "terminal_select".into(),
            description: "Select text by coordinate range and return it".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "start_col": { "type": "integer" },
                    "start_row": { "type": "integer" },
                    "end_col": { "type": "integer" },
                    "end_row": { "type": "integer" }
                },
                "required": ["id", "start_col", "start_row", "end_col", "end_row"]
            }),
        },
        ToolDef {
            name: "terminal_scroll".into(),
            description: "Page-based scrollback: page_up, page_down, or search for text".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "action": { "type": "string", "enum": ["page_up", "page_down", "search"] },
                    "text": { "type": "string", "description": "Search text (required for 'search' action)" }
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
                _ => (80, 24),
            };
            let shell = args.get("shell").and_then(|v| v.as_str());

            match registry.create(cols, rows, shell) {
                Ok(id) => ToolCallResult::text(
                    serde_json::to_string(&json!({ "id": id, "cols": cols, "rows": rows }))
                        .unwrap(),
                ),
                Err(e) => ToolCallResult::error(e),
            }
        }

        "terminal_destroy" => {
            let id = match args.get("id").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => return ToolCallResult::error("Missing 'id' parameter".into()),
            };
            let success = registry.destroy(id);
            ToolCallResult::text(json!({ "success": success }).to_string())
        }

        "terminal_list" => {
            let terminals = registry.list();
            ToolCallResult::text(json!({ "terminals": terminals }).to_string())
        }

        "terminal_send_key" => {
            let id = match args.get("id").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => return ToolCallResult::error("Missing 'id' parameter".into()),
            };
            let key_str = match args.get("key").and_then(|v| v.as_str()) {
                Some(k) => k,
                None => return ToolCallResult::error("Missing 'key' parameter".into()),
            };

            let key = match Key::from_str(key_str) {
                Ok(k) => k,
                Err(e) => return ToolCallResult::error(e),
            };

            let instance = match registry.get_mut(id) {
                Some(i) => i,
                None => return ToolCallResult::error(format!("Terminal '{}' not found", id)),
            };

            match instance.send_key(key) {
                Ok(_) => ToolCallResult::text(json!({ "success": true }).to_string()),
                Err(e) => ToolCallResult::error(format!("Failed to send key: {}", e)),
            }
        }

        "terminal_send_keys" => {
            let id = match args.get("id").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => return ToolCallResult::error("Missing 'id' parameter".into()),
            };
            let keys_arr = match args.get("keys").and_then(|v| v.as_array()) {
                Some(k) => k,
                None => return ToolCallResult::error("Missing 'keys' parameter".into()),
            };

            let instance = match registry.get_mut(id) {
                Some(i) => i,
                None => return ToolCallResult::error(format!("Terminal '{}' not found", id)),
            };

            let mut count = 0;
            for key_val in keys_arr {
                let key_str = match key_val.as_str() {
                    Some(s) => s,
                    None => continue,
                };
                let key = match Key::from_str(key_str) {
                    Ok(k) => k,
                    Err(e) => return ToolCallResult::error(e),
                };
                if let Err(e) = instance.send_key(key) {
                    return ToolCallResult::error(format!("Failed at key {}: {}", count, e));
                }
                count += 1;
            }

            ToolCallResult::text(json!({ "success": true, "count": count }).to_string())
        }

        "terminal_mouse" => {
            let id = match args.get("id").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => return ToolCallResult::error("Missing 'id' parameter".into()),
            };
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

            let instance = match registry.get_mut(id) {
                Some(i) => i,
                None => return ToolCallResult::error(format!("Terminal '{}' not found", id)),
            };

            let result = instance.send_mouse(action);
            ToolCallResult::text(serde_json::to_string(&result).unwrap())
        }

        "terminal_read_screen" => {
            let id = match args.get("id").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => return ToolCallResult::error("Missing 'id' parameter".into()),
            };
            let format = args
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("text");

            let instance = match registry.get_mut(id) {
                Some(i) => i,
                None => return ToolCallResult::error(format!("Terminal '{}' not found", id)),
            };

            match format {
                "json" => {
                    // Full JSON cell data
                    let grid = &instance.read_screen();
                    // For JSON format, return cell-level data
                    // Re-read from grid directly
                    ToolCallResult::text(grid.clone())
                }
                _ => {
                    let text = instance.read_screen();
                    ToolCallResult::text(text)
                }
            }
        }

        "terminal_read_rows" => {
            let id = match args.get("id").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => return ToolCallResult::error("Missing 'id' parameter".into()),
            };
            let start = args
                .get("start_row")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let end = args
                .get("end_row")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            let instance = match registry.get(id) {
                Some(i) => i,
                None => return ToolCallResult::error(format!("Terminal '{}' not found", id)),
            };

            ToolCallResult::text(instance.read_rows(start, end))
        }

        "terminal_read_region" => {
            let id = match args.get("id").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => return ToolCallResult::error("Missing 'id' parameter".into()),
            };
            let col1 = args.get("col1").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let row1 = args.get("row1").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let col2 = args.get("col2").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let row2 = args.get("row2").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

            let instance = match registry.get(id) {
                Some(i) => i,
                None => return ToolCallResult::error(format!("Terminal '{}' not found", id)),
            };

            ToolCallResult::text(instance.read_region(col1, row1, col2, row2))
        }

        "terminal_status" => {
            let id = match args.get("id").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => return ToolCallResult::error("Missing 'id' parameter".into()),
            };
            let last_n = args
                .get("last_n_lines")
                .and_then(|v| v.as_u64())
                .unwrap_or(5) as usize;

            let instance = match registry.get(id) {
                Some(i) => i,
                None => return ToolCallResult::error(format!("Terminal '{}' not found", id)),
            };

            let status = instance.get_status(last_n);
            ToolCallResult::text(serde_json::to_string(&status).unwrap())
        }

        "terminal_poll_events" => {
            let id = match args.get("id").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => return ToolCallResult::error("Missing 'id' parameter".into()),
            };

            let instance = match registry.get_mut(id) {
                Some(i) => i,
                None => return ToolCallResult::error(format!("Terminal '{}' not found", id)),
            };

            let events = instance.poll_events();
            ToolCallResult::text(json!({ "events": events }).to_string())
        }

        "terminal_select" => {
            let id = match args.get("id").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => return ToolCallResult::error("Missing 'id' parameter".into()),
            };
            let sc = args.get("start_col").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let sr = args.get("start_row").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let ec = args.get("end_col").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let er = args.get("end_row").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

            let instance = match registry.get_mut(id) {
                Some(i) => i,
                None => return ToolCallResult::error(format!("Terminal '{}' not found", id)),
            };

            let text = instance.select_range(sc, sr, ec, er);
            ToolCallResult::text(json!({ "selected_text": text }).to_string())
        }

        "terminal_scroll" => {
            let id = match args.get("id").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => return ToolCallResult::error("Missing 'id' parameter".into()),
            };
            let action = match args.get("action").and_then(|v| v.as_str()) {
                Some(a) => a,
                None => return ToolCallResult::error("Missing 'action' parameter".into()),
            };

            let instance = match registry.get_mut(id) {
                Some(i) => i,
                None => return ToolCallResult::error(format!("Terminal '{}' not found", id)),
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
                    let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    if text.is_empty() {
                        return ToolCallResult::error("Missing 'text' for search".into());
                    }
                    let (offset, found) = instance.scroll_to_text(text);
                    ToolCallResult::text(
                        json!({ "scroll_offset": offset, "found": found }).to_string(),
                    )
                }
                _ => ToolCallResult::error(format!("Unknown scroll action: {}", action)),
            }
        }

        _ => ToolCallResult::error(format!("Unknown tool: {}", tool_name)),
    }
}
