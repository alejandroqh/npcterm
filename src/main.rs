mod terminal;
mod input;
mod screen;
mod status;
mod manager;
mod mcp;

use std::time::Duration;
use turbomcp::prelude::*;
use turbomcp_server::ProtocolVersion;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Some(arg) = std::env::args().nth(1) {
        if arg == "--version" || arg == "-v" {
            println!("npcterm {}", env!("CARGO_PKG_VERSION"));
            return;
        }
    }

    let server = mcp::NpcTermServer::new();

    // Background tick thread — processes PTY output every 10ms
    let tick_registry = server.registry_handle();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(10));
        if let Ok(mut reg) = tick_registry.try_lock() {
            reg.tick_all();
        }
    });

    let protocol = ProtocolConfig {
        preferred_version: ProtocolVersion::V2025_11_25,
        supported_versions: vec![
            ProtocolVersion::Unknown("2024-11-05".into()),
            ProtocolVersion::V2025_06_18,
            ProtocolVersion::V2025_11_25,
        ],
        allow_fallback: false,
    };

    server
        .builder()
        .with_protocol(protocol)
        .serve()
        .await
        .expect("MCP server error");
}
