use std::sync::{Arc, Mutex, MutexGuard};

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use turbomcp::prelude::*;

use crate::input::keys::Key;
use crate::input::mouse::MouseAction;
use crate::manager::instance::TerminalInstance;
use crate::manager::registry::TerminalRegistry;

/// Input element for terminal_send_keys: either {text: "..."} or {key: "..."}
#[derive(Debug, Deserialize, JsonSchema)]
pub struct KeyInput {
    /// Raw text to type
    pub text: Option<String>,
    /// Special key name (Enter, Tab, Ctrl+c, etc)
    pub key: Option<String>,
}

#[derive(Clone)]
pub struct NpcTermServer {
    registry: Arc<Mutex<TerminalRegistry>>,
}

impl NpcTermServer {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(Mutex::new(TerminalRegistry::default())),
        }
    }

    #[allow(dead_code)] // Used from binary crate (main.rs)
    pub(crate) fn registry_handle(&self) -> Arc<Mutex<TerminalRegistry>> {
        Arc::clone(&self.registry)
    }

    fn lock_registry(&self) -> Result<MutexGuard<'_, TerminalRegistry>, ToolError> {
        self.registry
            .lock()
            .map_err(|_| ToolError::new("Registry lock poisoned"))
    }
}

fn get_instance<'a>(
    reg: &'a mut TerminalRegistry,
    id: &str,
) -> Result<&'a mut TerminalInstance, ToolError> {
    reg.get_mut(id)
        .ok_or_else(|| ToolError::new(format!("Terminal '{}' not found", id)))
}

#[server(name = "npcterm39", version = "1.2.0")]
impl NpcTermServer {
    /// Create a new terminal instance. Returns {id, cols, rows}. The id is required for all subsequent terminal operations. Available sizes: 80x24 (default), 120x40, 160x40, 200x50.
    #[tool]
    async fn terminal_create(
        &self,
        #[description("Terminal size")] size: Option<String>,
        #[description("Shell path (optional)")] shell: Option<String>,
    ) -> Result<String, ToolError> {
        let (cols, rows) = match size.as_deref().unwrap_or("80x24") {
            "120x40" => (120, 40),
            "160x40" => (160, 40),
            "200x50" => (200, 50),
            _ => (80, 24),
        };
        let mut reg = self.lock_registry()?;
        match reg.create(cols, rows, shell.as_deref()) {
            Ok(id) => Ok(json!({ "id": id, "cols": cols, "rows": rows }).to_string()),
            Err(e) => Err(ToolError::new(e)),
        }
    }

    /// Destroy a terminal instance and kill its PTY process. Returns {success: bool}.
    #[tool]
    async fn terminal_destroy(
        &self,
        #[description("Terminal ID")] id: String,
    ) -> Result<String, ToolError> {
        let mut reg = self.lock_registry()?;
        Ok(json!({ "success": reg.destroy(&id) }).to_string())
    }

    /// List all terminal instances with id, size, state, and running command.
    #[tool]
    async fn terminal_list(&self) -> Result<String, ToolError> {
        let reg = self.lock_registry()?;
        Ok(json!({ "terminals": reg.list() }).to_string())
    }

    /// Send a single keystroke. Supports: a-z, Enter, Tab, Escape, Backspace, Delete, arrows, Home, End, PageUp, PageDown, F1-F12, Ctrl+key, Alt+key, space. For multiple keystrokes or text input, use terminal_send_keys instead.
    #[tool]
    async fn terminal_send_key(
        &self,
        #[description("Terminal ID")] id: String,
        #[description("Key name (e.g. 'Enter', 'Ctrl+c', 'a')")] key: String,
    ) -> Result<String, ToolError> {
        let parsed = Key::from_str(&key).map_err(ToolError::new)?;
        let mut reg = self.lock_registry()?;
        let instance = get_instance(&mut reg, &id)?;
        instance.send_key(parsed).map_err(|e| ToolError::new(format!("Failed to send key: {}", e)))?;
        Ok(json!({ "success": true }).to_string())
    }

    /// Send a batch of text and special keys in one call (preferred over terminal_send_key for multi-step input). Each element is either {"text":"string"} for raw text or {"key":"Enter"} for special keys (Enter, Tab, Escape, Ctrl+c, etc). Example: [{"text":"echo hello"},{"key":"Enter"}]
    #[tool]
    async fn terminal_send_keys(
        &self,
        #[description("Terminal ID")] id: String,
        #[description("Array of text/key input items")] input: Vec<KeyInput>,
    ) -> Result<String, ToolError> {
        let mut reg = self.lock_registry()?;
        let instance = get_instance(&mut reg, &id)?;
        for item in &input {
            if let Some(text) = &item.text {
                instance.write_raw(text.as_bytes())
                    .map_err(|e| ToolError::new(format!("Failed to send text: {}", e)))?;
            } else if let Some(key_str) = &item.key {
                let key = Key::from_str(key_str).map_err(ToolError::new)?;
                instance.send_key_no_flush(key)
                    .map_err(|e| ToolError::new(format!("Failed to send key: {}", e)))?;
            } else {
                return Err(ToolError::new("Each input element must have 'text' or 'key'"));
            }
        }
        let _ = instance.flush_input();
        Ok(json!({ "success": true }).to_string())
    }

    /// Perform a mouse action. col and row required for left_click, right_click, double_click, move, set_position. get_position returns current cursor coords.
    #[tool]
    async fn terminal_mouse(
        &self,
        #[description("Terminal ID")] id: String,
        #[description("Mouse action")] action: String,
        #[description("Column")] col: Option<u64>,
        #[description("Row")] row: Option<u64>,
    ) -> Result<String, ToolError> {
        let col = col.unwrap_or(0) as u16;
        let row = row.unwrap_or(0) as u16;
        let mouse_action = match action.as_str() {
            "left_click" => MouseAction::LeftClick { col, row },
            "right_click" => MouseAction::RightClick { col, row },
            "double_click" => MouseAction::DoubleClick { col, row },
            "move" => MouseAction::MoveTo { col, row },
            "get_position" => MouseAction::GetPosition,
            "set_position" => MouseAction::SetPosition { col, row },
            _ => return Err(ToolError::new(format!("Unknown action: {}", action))),
        };
        let mut reg = self.lock_registry()?;
        let instance = get_instance(&mut reg, &id)?;
        let result = instance.send_mouse(mouse_action);
        serde_json::to_string(&result).map_err(|e| ToolError::new(format!("Serialization failed: {}", e)))
    }

    /// Read terminal screen with coordinate overlay. Use mode 'changes' to see only new output since your last read (default 50 lines, max 200). Default mode 'full' returns the complete screen.
    #[tool]
    async fn terminal_read_screen(
        &self,
        #[description("Terminal ID")] id: String,
        #[description("Read mode: 'full' or 'changes'")] mode: Option<String>,
        #[description("Max lines to return in 'changes' mode (1-200)")] max_lines: Option<u64>,
    ) -> Result<String, ToolError> {
        let mode = mode.as_deref().unwrap_or("full");
        let max_lines = max_lines.map(|v| (v as usize).clamp(1, 200)).unwrap_or(50);
        let mut reg = self.lock_registry()?;
        let instance = get_instance(&mut reg, &id)?;
        match mode {
            "changes" => Ok(instance.read_changes(max_lines)),
            _ => Ok(instance.read_screen()),
        }
    }

    /// Return terminal screen as plain text without coordinates. Lighter output, best for reading content or passing to other tools.
    #[tool]
    async fn terminal_show_screen(
        &self,
        #[description("Terminal ID")] id: String,
    ) -> Result<String, ToolError> {
        let mut reg = self.lock_registry()?;
        let instance = get_instance(&mut reg, &id)?;
        Ok(instance.show_screen())
    }

    /// Read specific rows from terminal screen with coordinate overlay (column headers + row numbers).
    #[tool]
    async fn terminal_read_rows(
        &self,
        #[description("Terminal ID")] id: String,
        #[description("Start row")] start_row: u64,
        #[description("End row")] end_row: u64,
    ) -> Result<String, ToolError> {
        let mut reg = self.lock_registry()?;
        let instance = get_instance(&mut reg, &id)?;
        Ok(instance.read_rows(start_row as usize, end_row as usize))
    }

    /// Read a rectangular region from terminal screen with row numbers.
    #[tool]
    async fn terminal_read_region(
        &self,
        #[description("Terminal ID")] id: String,
        #[description("Start column")] col1: u64,
        #[description("Start row")] row1: u64,
        #[description("End column")] col2: u64,
        #[description("End row")] row2: u64,
    ) -> Result<String, ToolError> {
        let mut reg = self.lock_registry()?;
        let instance = get_instance(&mut reg, &id)?;
        Ok(instance.read_region(col1 as usize, row1 as usize, col2 as usize, row2 as usize))
    }

    /// Get terminal status: process state, running command, cursor position, dirty rows, last N screen lines, pending event count, and scrollback depth. Token-efficient alternative to reading the full screen.
    #[tool]
    async fn terminal_status(
        &self,
        #[description("Terminal ID")] id: String,
        #[description("Number of trailing screen lines to include")] last_n_lines: Option<u64>,
    ) -> Result<String, ToolError> {
        let last_n = last_n_lines.unwrap_or(5) as usize;
        let mut reg = self.lock_registry()?;
        let instance = get_instance(&mut reg, &id)?;
        serde_json::to_string(&instance.get_status(last_n)).map_err(|e| ToolError::new(format!("Serialization failed: {}", e)))
    }

    /// Drain pending terminal events (destructive: events are removed after reading). Event types: CommandFinished, WaitingForInput, Bell, ProcessStateChanged, ScreenChanged.
    #[tool]
    async fn terminal_poll_events(
        &self,
        #[description("Terminal ID")] id: String,
    ) -> Result<String, ToolError> {
        let mut reg = self.lock_registry()?;
        let instance = get_instance(&mut reg, &id)?;
        let events = instance.poll_events();
        Ok(json!({ "events": events }).to_string())
    }

    /// Select text by coordinate range (like click-drag) and return it. Unlike read_region, this is a logical text selection that can span across lines.
    #[tool]
    async fn terminal_select(
        &self,
        #[description("Terminal ID")] id: String,
        #[description("Start column")] start_col: u64,
        #[description("Start row")] start_row: u64,
        #[description("End column")] end_col: u64,
        #[description("End row")] end_row: u64,
    ) -> Result<String, ToolError> {
        let mut reg = self.lock_registry()?;
        let instance = get_instance(&mut reg, &id)?;
        let text = instance.select_range(start_col as usize, start_row as usize, end_col as usize, end_row as usize);
        Ok(json!({ "selected_text": text }).to_string())
    }

    /// Navigate scrollback history: page_up, page_down, or search (requires 'text' param). Returns {scroll_offset} (0 = bottom). Use terminal_read_screen after scrolling to see the result.
    #[tool]
    async fn terminal_scroll(
        &self,
        #[description("Terminal ID")] id: String,
        #[description("Scroll action: page_up, page_down, or search")] action: String,
        #[description("Search text (required for 'search' action)")] text: Option<String>,
    ) -> Result<String, ToolError> {
        let mut reg = self.lock_registry()?;
        let instance = get_instance(&mut reg, &id)?;
        match action.as_str() {
            "page_up" => {
                let offset = instance.scroll_page_up();
                Ok(json!({ "scroll_offset": offset }).to_string())
            }
            "page_down" => {
                let offset = instance.scroll_page_down();
                Ok(json!({ "scroll_offset": offset }).to_string())
            }
            "search" => {
                let search_text = text.as_deref().unwrap_or("");
                if search_text.is_empty() {
                    return Err(ToolError::new("Missing 'text' for search"));
                }
                let (offset, found) = instance.scroll_to_text(search_text);
                Ok(json!({ "scroll_offset": offset, "found": found }).to_string())
            }
            _ => Err(ToolError::new(format!("Unknown scroll action: {}", action))),
        }
    }
}
