use std::io::{BufRead, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use crate::manager::registry::TerminalRegistry;

use super::tools;
use super::types::{JsonRpcRequest, JsonRpcResponse};

/// Run the MCP server on stdio (blocking)
pub fn run_stdio_server() {
    let registry = Arc::new(Mutex::new(TerminalRegistry::default()));

    // Background tick thread — processes PTY output every 10ms
    let tick_registry = Arc::clone(&registry);
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(10));
        if let Ok(mut reg) = tick_registry.try_lock() {
            reg.tick_all();
        }
    });

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse::error(None, -32700, format!("Parse error: {}", e));
                let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
                let _ = stdout.flush();
                continue;
            }
        };

        let response = handle_request(&registry, &request);

        if let Some(resp) = response {
            let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
            let _ = stdout.flush();
        }
    }
}

fn handle_request(
    registry: &Arc<Mutex<TerminalRegistry>>,
    request: &JsonRpcRequest,
) -> Option<JsonRpcResponse> {
    match request.method.as_str() {
        "initialize" => {
            let result = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "aiterm39", "version": "1.0.0" }
            });
            Some(JsonRpcResponse::success(request.id.clone(), result))
        }

        "notifications/initialized" => None,

        "tools/list" => {
            let tool_defs = tools::tool_definitions();
            Some(JsonRpcResponse::success(request.id.clone(), json!({ "tools": tool_defs })))
        }

        "tools/call" => {
            let tool_name = request.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = request.params.get("arguments").cloned().unwrap_or(Value::Object(Default::default()));

            let mut reg = registry.lock().unwrap();
            let result = tools::handle_tool_call(&mut reg, tool_name, &args);

            Some(JsonRpcResponse::success(
                request.id.clone(),
                serde_json::to_value(&result).unwrap(),
            ))
        }

        "ping" => Some(JsonRpcResponse::success(request.id.clone(), json!({}))),

        _ => {
            if request.id.is_some() {
                Some(JsonRpcResponse::error(
                    request.id.clone(),
                    -32601,
                    format!("Method not found: {}", request.method),
                ))
            } else {
                None
            }
        }
    }
}
