mod terminal;
mod input;
mod screen;
mod status;
mod manager;
mod mcp;
#[cfg(feature = "viewer")]
mod web;

use std::time::Duration;
use turbomcp::prelude::*;
use turbomcp_server::ProtocolVersion;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Parse CLI args
    let env_no_markers = std::env::var("NPCTERM_NO_MARKERS").is_ok();
    let mut cli_no_markers = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--version" | "-v" => {
                println!("npcterm {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "--no-markers" => {
                cli_no_markers = true;
            }
            _ => {}
        }
    }
    if env_no_markers || cli_no_markers {
        crate::set_no_markers(true);
    }

    #[cfg(feature = "viewer")]
    let (broadcast_tx, interactions) = {
        let (tx, _) = tokio::sync::broadcast::channel(64);
        let interactions = std::sync::Arc::new(std::sync::Mutex::new(
            web::interaction::InteractionLog::default(),
        ));
        (tx, interactions)
    };

    #[cfg(feature = "viewer")]
    let server = mcp::NpcTermServer::new_with_viewer(
        broadcast_tx.clone(),
        std::sync::Arc::clone(&interactions),
    );
    #[cfg(not(feature = "viewer"))]
    let server = mcp::NpcTermServer::new();

    // Background tick thread — processes PTY output every 10ms
    let tick_registry = server.registry_handle();
    #[cfg(feature = "viewer")]
    let tick_broadcast = broadcast_tx;

    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(10));
        if let Ok(mut reg) = tick_registry.try_lock() {
            reg.tick_all();
            #[cfg(feature = "viewer")]
            web::broadcast_updates(&mut reg, &tick_broadcast);
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
