use std::sync::{Arc, Mutex, MutexGuard};

// Compile-time check: MCP server version must match Cargo.toml
const _: () = {
    let cargo = env!("CARGO_PKG_VERSION").as_bytes();
    let mcp = b"1.3.3";
    assert!(cargo.len() == mcp.len(), "MCP server version does not match Cargo.toml — update #[server(version)] below");
    let mut i = 0;
    while i < cargo.len() {
        assert!(cargo[i] == mcp[i], "MCP server version does not match Cargo.toml — update #[server(version)] below");
        i += 1;
    }
};

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
#[schemars(inline)]
pub struct KeyInput {
    /// Raw text to type
    pub text: Option<String>,
    /// Special key name (Enter, Tab, Ctrl+c, etc)
    pub key: Option<String>,
}

#[derive(Clone)]
pub struct NpcTermServer {
    registry: Arc<Mutex<TerminalRegistry>>,
    #[cfg(feature = "viewer")]
    broadcast_tx: tokio::sync::broadcast::Sender<Arc<crate::web::messages::WsServerMessage>>,
    #[cfg(feature = "viewer")]
    interactions: Arc<Mutex<crate::web::interaction::InteractionLog>>,
    #[cfg(feature = "viewer")]
    viewer_handle: crate::web::SharedViewerHandle,
}

impl NpcTermServer {
    #[cfg(not(feature = "viewer"))]
    pub fn new() -> Self {
        Self {
            registry: Arc::new(Mutex::new(TerminalRegistry::default())),
        }
    }

    #[cfg(feature = "viewer")]
    pub fn new_with_viewer(
        broadcast_tx: tokio::sync::broadcast::Sender<Arc<crate::web::messages::WsServerMessage>>,
        interactions: Arc<Mutex<crate::web::interaction::InteractionLog>>,
    ) -> Self {
        Self {
            registry: Arc::new(Mutex::new(TerminalRegistry::default())),
            broadcast_tx,
            interactions,
            viewer_handle: crate::web::new_shared_viewer_handle(),
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

    #[cfg(feature = "viewer")]
    fn log_interaction(
        &self,
        tool: &str,
        terminal_id: Option<&str>,
        params: serde_json::Value,
        success: bool,
        summary: Option<String>,
    ) {
        let entry = crate::web::interaction::InteractionEntry {
            timestamp: chrono::Utc::now(),
            tool: tool.to_string(),
            terminal_id: terminal_id.map(|s| s.to_string()),
            params,
            success,
            summary,
        };
        crate::web::broadcast_interaction(&self.broadcast_tx, &self.interactions, entry);
    }

    #[cfg(not(feature = "viewer"))]
    #[inline(always)]
    fn log_interaction(&self, _: &str, _: Option<&str>, _: serde_json::Value, _: bool, _: Option<String>) {}

    #[cfg(feature = "viewer")]
    fn broadcast_terminal_list(&self, reg: &TerminalRegistry) {
        if self.broadcast_tx.receiver_count() > 0 {
            let terminals = reg.list();
            let _ = self.broadcast_tx.send(Arc::new(crate::web::messages::WsServerMessage::TerminalList { terminals }));
        }
    }

    #[cfg(not(feature = "viewer"))]
    #[inline(always)]
    fn broadcast_terminal_list(&self, _: &TerminalRegistry) {}

    #[cfg(feature = "viewer")]
    fn lock_viewer_handle(&self) -> Result<std::sync::MutexGuard<'_, Option<crate::web::ViewerHandle>>, ToolError> {
        self.viewer_handle
            .lock()
            .map_err(|_| ToolError::new("Viewer handle lock poisoned"))
    }
}

fn get_instance<'a>(
    reg: &'a mut TerminalRegistry,
    id: &str,
) -> Result<&'a mut TerminalInstance, ToolError> {
    reg.get_mut(id)
        .ok_or_else(|| ToolError::new(format!("Terminal '{}' not found", id)))
}

#[cfg(feature = "viewer")]
fn viewer_url(port: u16) -> String {
    format!("http://localhost:{}", port)
}

#[cfg(feature = "viewer")]
fn open_browser(url: &str) -> Result<(), String> {
    let mut child = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else if cfg!(target_os = "linux") {
        std::process::Command::new("xdg-open").arg(url).spawn()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd").args(["/c", "start", url]).spawn()
    } else {
        return Err("Unsupported platform".to_string());
    }
    .map_err(|e| format!("Failed to open browser: {}", e))?;
    // Reap the child in a detached thread to avoid zombie processes
    std::thread::spawn(move || { let _ = child.wait(); });
    Ok(())
}

#[server(name = "npcterm39", version = "1.3.3")]
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
        let result = match reg.create(cols, rows, shell.as_deref()) {
            Ok(id) => {
                let resp = json!({ "id": id, "cols": cols, "rows": rows }).to_string();
                self.log_interaction(
                    "terminal_create",
                    Some(&id),
                    json!({ "size": format!("{}x{}", cols, rows) }),
                    true,
                    Some(format!("created {}x{}", cols, rows)),
                );
                Ok(resp)
            }
            Err(e) => Err(ToolError::new(e)),
        };
        self.broadcast_terminal_list(&reg);
        result
    }

    /// Destroy a terminal instance and kill its PTY process. Returns {success: bool}.
    #[tool]
    async fn terminal_destroy(
        &self,
        #[description("Terminal ID")] id: String,
    ) -> Result<String, ToolError> {
        let mut reg = self.lock_registry()?;
        let success = reg.destroy(&id);
        self.log_interaction("terminal_destroy", Some(&id), json!({}), success, Some(if success { "destroyed".into() } else { "not found".into() }));
        self.broadcast_terminal_list(&reg);
        Ok(json!({ "success": success }).to_string())
    }

    /// List all terminal instances with id, size, state, and running command.
    #[tool]
    async fn terminal_list(&self) -> Result<String, ToolError> {
        let reg = self.lock_registry()?;
        let list = reg.list();
        self.log_interaction("terminal_list", None, json!({}), true, Some(format!("{} terminals", list.len())));
        Ok(json!({ "terminals": list }).to_string())
    }

    /// Forward the terminal to an external terminal emulator (Windows Terminal, Terminal.app, gnome-terminal, etc.).
    /// This opens the user's default terminal attached to the PTY, allowing applications that require
    /// direct terminal control (like Textual TUIs) to work correctly.
    #[tool]
    async fn terminal_forward(
        &self,
        #[description("Terminal ID")] id: String,
        #[description("Terminal emulator to use (auto-detected if None: Windows Terminal, Terminal.app, or detected Linux terminal)")] emulator: Option<String>,
    ) -> Result<String, ToolError> {
        let _ = emulator; // Reserved for future use (emulator preference)
        let mut reg = self.lock_registry()?;
        let instance = get_instance(&mut reg, &id)?;
        match instance.forward_terminal() {
            Ok(terminal_name) => {
                self.log_interaction("terminal_forward", Some(&id), json!({}), true, Some(format!("launched {}", terminal_name)));
                Ok(json!({ "success": true, "terminal": terminal_name }).to_string())
            }
            Err(e) => {
                let msg = format!("Failed to launch terminal: {}", e);
                self.log_interaction("terminal_forward", Some(&id), json!({}), false, Some(msg.clone()));
                Err(ToolError::new(msg))
            }
        }
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
        self.log_interaction("terminal_send_key", Some(&id), json!({ "key": key }), true, Some(format!("sent {}", key)));
        Ok(json!({ "success": true }).to_string())
    }

    /// Send a batch of text and special keys in one call (preferred over terminal_send_key for multi-step input). Each element is either {"text":"string"} for raw text or {"key":"Enter"} for special keys (Enter, Tab, Escape, Ctrl+c, etc). Example: [{"text":"echo hello"},{"key":"Enter"}]
    #[tool]
    async fn terminal_send_keys(
        &self,
        #[description("Terminal ID")] id: String,
        #[description("Array of text/key input items")] input: Vec<KeyInput>,
    ) -> Result<String, ToolError> {
        let summary: String = input.iter().map(|item| {
            if let Some(text) = &item.text {
                if text.len() > 30 { format!("'{:.27}...'", text) } else { format!("'{}'", text) }
            } else if let Some(key) = &item.key {
                key.clone()
            } else {
                "?".into()
            }
        }).collect::<Vec<_>>().join(" + ");

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
        self.log_interaction("terminal_send_keys", Some(&id), json!({ "count": input.len() }), true, Some(format!("sent {}", summary)));
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
        self.log_interaction(
            "terminal_mouse",
            Some(&id),
            json!({ "action": action, "col": col, "row": row }),
            true,
            Some(format!("{} at ({}, {})", action, col, row)),
        );
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
        let result = match mode {
            "changes" => instance.read_changes(max_lines),
            _ => instance.read_screen(),
        };
        self.log_interaction("terminal_read_screen", Some(&id), json!({ "mode": mode }), true, Some(format!("read screen ({})", mode)));
        Ok(result)
    }

    /// Return terminal screen as plain text without coordinates. Lighter output, best for reading content or passing to other tools.
    #[tool]
    async fn terminal_show_screen(
        &self,
        #[description("Terminal ID")] id: String,
    ) -> Result<String, ToolError> {
        let mut reg = self.lock_registry()?;
        let instance = get_instance(&mut reg, &id)?;
        let result = instance.show_screen();
        self.log_interaction("terminal_show_screen", Some(&id), json!({}), true, Some("show screen".into()));
        Ok(result)
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
        let result = instance.read_rows(start_row as usize, end_row as usize);
        self.log_interaction("terminal_read_rows", Some(&id), json!({ "start": start_row, "end": end_row }), true, Some(format!("rows {}-{}", start_row, end_row)));
        Ok(result)
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
        let result = instance.read_region(col1 as usize, row1 as usize, col2 as usize, row2 as usize);
        self.log_interaction("terminal_read_region", Some(&id), json!({ "col1": col1, "row1": row1, "col2": col2, "row2": row2 }), true, Some(format!("region ({},{}) to ({},{})", col1, row1, col2, row2)));
        Ok(result)
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
        let status = instance.get_status(last_n);
        self.log_interaction("terminal_status", Some(&id), json!({ "last_n": last_n }), true, Some(format!("status [{}]", status.state)));
        serde_json::to_string(&status).map_err(|e| ToolError::new(format!("Serialization failed: {}", e)))
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
        self.log_interaction("terminal_poll_events", Some(&id), json!({}), true, Some(format!("{} events", events.len())));
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
        self.log_interaction("terminal_select", Some(&id), json!({ "start_col": start_col, "start_row": start_row, "end_col": end_col, "end_row": end_row }), true, Some(format!("selected {} chars", text.len())));
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
        let result = match action.as_str() {
            "page_up" => {
                let offset = instance.scroll_page_up();
                json!({ "scroll_offset": offset }).to_string()
            }
            "page_down" => {
                let offset = instance.scroll_page_down();
                json!({ "scroll_offset": offset }).to_string()
            }
            "search" => {
                let search_text = text.as_deref().unwrap_or("");
                if search_text.is_empty() {
                    return Err(ToolError::new("Missing 'text' for search"));
                }
                let (offset, found) = instance.scroll_to_text(search_text);
                json!({ "scroll_offset": offset, "found": found }).to_string()
            }
            _ => return Err(ToolError::new(format!("Unknown scroll action: {}", action))),
        };
        self.log_interaction("terminal_scroll", Some(&id), json!({ "action": action }), true, Some(format!("scroll {}", action)));
        Ok(result)
    }

    /// Start the web debug viewer. Returns the URL. Default port is 8039; if busy, tries the next 10 ports. If already running, returns the current URL.
    #[tool]
    async fn viewer_start(
        &self,
        #[description("Port to bind (default 8039)")] port: Option<u64>,
    ) -> Result<String, ToolError> {
        #[cfg(not(feature = "viewer"))]
        { let _ = port; return Err(ToolError::new("Viewer feature not enabled. Rebuild with --features viewer")); }

        #[cfg(feature = "viewer")]
        {
            let port = port.map(|p| p as u16).unwrap_or(8039);

            // Check if already running
            {
                let handle = self.lock_viewer_handle()?;
                if let Some(ref h) = *handle {
                    let url = viewer_url(h.port);
                    self.log_interaction("viewer_start", None, json!({ "port": h.port }), true, Some(format!("already running at {}", url)));
                    return Ok(json!({ "url": url, "port": h.port, "status": "already_running" }).to_string());
                }
            }

            let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
            let (port_tx, port_rx) = tokio::sync::oneshot::channel::<Result<u16, String>>();

            let viewer_state = crate::web::ViewerState {
                registry: self.registry_handle(),
                interactions: Arc::clone(&self.interactions),
                broadcast_tx: self.broadcast_tx.clone(),
            };

            let thread_handle = std::thread::spawn(move || {
                crate::web::start_viewer(viewer_state, port, port_tx, shutdown_rx);
            });

            let bound_port = port_rx
                .await
                .map_err(|_| ToolError::new("Viewer thread died before reporting port"))?
                .map_err(|e| ToolError::new(e))?;

            {
                let mut handle = self.lock_viewer_handle()?;
                *handle = Some(crate::web::ViewerHandle {
                    port: bound_port,
                    shutdown_tx: Some(shutdown_tx),
                    thread_handle: Some(thread_handle),
                });
            }

            let url = viewer_url(bound_port);
            self.log_interaction("viewer_start", None, json!({ "port": bound_port }), true, Some(format!("started at {}", url)));
            Ok(json!({ "url": url, "port": bound_port, "status": "started" }).to_string())
        }
    }

    /// Stop the web debug viewer gracefully.
    #[tool]
    async fn viewer_stop(&self) -> Result<String, ToolError> {
        #[cfg(not(feature = "viewer"))]
        { return Err(ToolError::new("Viewer feature not enabled. Rebuild with --features viewer")); }

        #[cfg(feature = "viewer")]
        {
            let mut handle_guard = self.lock_viewer_handle()?;

            match handle_guard.take() {
                Some(mut h) => {
                    let port = h.port;
                    if let Some(tx) = h.shutdown_tx.take() {
                        let _ = tx.send(());
                    }
                    // Release lock before joining so other tools aren't blocked
                    let thread = h.thread_handle.take();
                    drop(handle_guard);
                    if let Some(jh) = thread {
                        let _ = jh.join();
                    }
                    self.log_interaction("viewer_stop", None, json!({ "port": port }), true, Some("stopped".into()));
                    Ok(json!({ "success": true, "status": "stopped" }).to_string())
                }
                None => {
                    self.log_interaction("viewer_stop", None, json!({}), true, Some("not running".into()));
                    Ok(json!({ "success": true, "status": "not_running" }).to_string())
                }
            }
        }
    }

    /// Open the web debug viewer in the system browser. Starts the viewer first if not running.
    #[tool]
    async fn viewer_open(
        &self,
        #[description("Port to bind if starting viewer (default 8039)")] port: Option<u64>,
    ) -> Result<String, ToolError> {
        #[cfg(not(feature = "viewer"))]
        { let _ = port; return Err(ToolError::new("Viewer feature not enabled. Rebuild with --features viewer")); }

        #[cfg(feature = "viewer")]
        {
            // Start viewer if not running, then read the URL from the handle
            if self.lock_viewer_handle()?.is_none() {
                self.viewer_start(port).await?;
            }

            let url = {
                let handle = self.lock_viewer_handle()?;
                let port = handle.as_ref()
                    .map(|h| h.port)
                    .ok_or_else(|| ToolError::new("Viewer failed to start"))?;
                viewer_url(port)
            };

            match open_browser(&url) {
                Ok(()) => {
                    self.log_interaction("viewer_open", None, json!({ "url": &url }), true, Some(format!("opened {}", url)));
                    Ok(json!({ "url": url, "opened": true }).to_string())
                }
                Err(e) => {
                    self.log_interaction("viewer_open", None, json!({ "url": &url }), false, Some(format!("failed: {}", e)));
                    Ok(json!({ "url": url, "opened": false, "error": e }).to_string())
                }
            }
        }
    }
}
