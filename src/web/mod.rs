pub mod interaction;
pub mod messages;

use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;
use tokio::sync::broadcast;

use crate::manager::registry::TerminalRegistry;

use self::interaction::{InteractionEntry, InteractionLog};
use self::messages::{WsRow, WsServerMessage, WsClientMessage, row_to_ws_row};

/// Handle to a running viewer server, used for shutdown and status queries.
pub struct ViewerHandle {
    pub port: u16,
    pub shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    pub thread_handle: Option<JoinHandle<()>>,
}

/// Thread-safe shared viewer state for MCP tools
pub type SharedViewerHandle = Arc<Mutex<Option<ViewerHandle>>>;

pub fn new_shared_viewer_handle() -> SharedViewerHandle {
    Arc::new(Mutex::new(None))
}

const INDEX_HTML: &str = include_str!("static/index.html");

/// Shared state for the viewer server
#[derive(Clone)]
pub struct ViewerState {
    pub registry: Arc<Mutex<TerminalRegistry>>,
    pub interactions: Arc<Mutex<InteractionLog>>,
    pub broadcast_tx: broadcast::Sender<Arc<WsServerMessage>>,
}

impl ViewerState {
    fn terminal_list_msg(&self) -> Arc<WsServerMessage> {
        let terminals = self.registry.lock().unwrap().list();
        Arc::new(WsServerMessage::TerminalList { terminals })
    }
}

/// Start the viewer HTTP/WebSocket server (blocking — run on dedicated thread).
/// Tries `port` first, then up to `port + 10` if busy.
/// Sends the actual bound port back on `port_tx`, then runs until `shutdown_rx` fires.
pub fn start_viewer(
    state: ViewerState,
    port: u16,
    port_tx: tokio::sync::oneshot::Sender<Result<u16, String>>,
    shutdown_rx: tokio::sync::oneshot::Receiver<()>,
) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("Failed to build viewer tokio runtime");

    rt.block_on(async move {
        let app = Router::new()
            .route("/", get(serve_index))
            .route("/ws", get(ws_upgrade))
            .route("/api/terminals", get(api_terminals))
            .with_state(state);

        // Try ports: port, port+1, ..., port+10
        let mut listener = None;
        let mut bound_port = port;
        for offset in 0..=10u16 {
            let try_port = port.saturating_add(offset);
            match tokio::net::TcpListener::bind(format!("127.0.0.1:{}", try_port)).await {
                Ok(l) => {
                    bound_port = try_port;
                    listener = Some(l);
                    break;
                }
                Err(_) => continue,
            }
        }

        let listener = match listener {
            Some(l) => l,
            None => {
                let _ = port_tx.send(Err(format!(
                    "Could not bind to any port in range {}-{}",
                    port,
                    port.saturating_add(10)
                )));
                return;
            }
        };

        let _ = port_tx.send(Ok(bound_port));

        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("Viewer server error");
    });
}

async fn serve_index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn api_terminals(State(state): State<ViewerState>) -> impl IntoResponse {
    let terminals = state.registry.lock().unwrap().list();
    axum::Json(serde_json::json!({ "terminals": terminals }))
}

async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<ViewerState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: ViewerState) {
    if send_json(&mut socket, &state.terminal_list_msg()).await.is_err() {
        return;
    }

    let recent_entries = {
        let log = state.interactions.lock().unwrap();
        log.recent(50)
    };
    for entry in recent_entries {
        let msg = Arc::new(WsServerMessage::Interaction { entry });
        if send_json(&mut socket, &msg).await.is_err() {
            return;
        }
    }

    let mut rx = state.broadcast_tx.subscribe();
    let mut subscribed_terminal: Option<String> = None;

    loop {
        tokio::select! {
            client_msg = socket.recv() => {
                match client_msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(cmd) = serde_json::from_str::<WsClientMessage>(&text) {
                            match cmd {
                                WsClientMessage::Subscribe { terminal_id } => {
                                    if let Some(snapshot) = build_snapshot(&state.registry, &terminal_id) {
                                        if send_json(&mut socket, &snapshot).await.is_err() {
                                            return;
                                        }
                                    }
                                    subscribed_terminal = Some(terminal_id);
                                }
                                WsClientMessage::ListTerminals => {
                                    if send_json(&mut socket, &state.terminal_list_msg()).await.is_err() {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => return,
                    _ => {}
                }
            }
            broadcast_msg = rx.recv() => {
                match broadcast_msg {
                    Ok(msg) => {
                        let should_send = match msg.as_ref() {
                            WsServerMessage::ScreenUpdate { terminal_id, .. }
                            | WsServerMessage::ScreenSnapshot { terminal_id, .. }
                            | WsServerMessage::TerminalEvent { terminal_id, .. } => {
                                subscribed_terminal.as_ref() == Some(terminal_id)
                            }
                            WsServerMessage::Interaction { .. }
                            | WsServerMessage::TerminalList { .. } => true,
                        };
                        if should_send {
                            if send_json(&mut socket, &msg).await.is_err() {
                                return;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        if let Some(tid) = &subscribed_terminal {
                            if let Some(snapshot) = build_snapshot(&state.registry, tid) {
                                if send_json(&mut socket, &snapshot).await.is_err() {
                                    return;
                                }
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
        }
    }
}

fn build_snapshot(registry: &Arc<Mutex<TerminalRegistry>>, terminal_id: &str) -> Option<Arc<WsServerMessage>> {
    let reg = registry.lock().unwrap();
    let instance = reg.get(terminal_id)?;
    let grid = instance.grid();
    let grid_rows = grid.get_rows();
    let screen_rows: Vec<WsRow> = grid_rows
        .iter()
        .enumerate()
        .map(|(i, row)| row_to_ws_row(i, row))
        .collect();
    Some(Arc::new(WsServerMessage::ScreenSnapshot {
        terminal_id: terminal_id.to_string(),
        cols: instance.cols(),
        rows: instance.rows(),
        screen_rows,
        cursor: grid.cursor,
        state: instance.state(),
    }))
}

async fn send_json(socket: &mut WebSocket, msg: &WsServerMessage) -> Result<(), ()> {
    match serde_json::to_string(msg) {
        Ok(json) => socket.send(Message::Text(json.into())).await.map_err(|_| ()),
        Err(_) => Err(()),
    }
}

/// Called from the tick thread after tick_all() — broadcasts dirty screen updates
pub fn broadcast_updates(
    reg: &mut TerminalRegistry,
    tx: &broadcast::Sender<Arc<WsServerMessage>>,
) {
    if tx.receiver_count() == 0 {
        return;
    }

    for instance in reg.instances_mut() {
        if !instance.has_viewer_dirty() {
            continue;
        }
        let dirty_indices = instance.take_viewer_dirty();
        if dirty_indices.is_empty() {
            continue;
        }

        let grid = instance.grid();
        let grid_rows = grid.get_rows();
        let changed_rows: Vec<WsRow> = dirty_indices
            .iter()
            .filter_map(|&i| grid_rows.get(i).map(|row| row_to_ws_row(i, row)))
            .collect();

        let msg = Arc::new(WsServerMessage::ScreenUpdate {
            terminal_id: instance.id.clone(),
            changed_rows,
            cursor: grid.cursor,
            state: instance.state(),
        });
        let _ = tx.send(msg);
    }
}

/// Broadcast an interaction entry to connected viewers
pub fn broadcast_interaction(
    tx: &broadcast::Sender<Arc<WsServerMessage>>,
    interactions: &Arc<Mutex<InteractionLog>>,
    entry: InteractionEntry,
) {
    let should_broadcast = tx.receiver_count() > 0;
    if let Ok(mut log) = interactions.lock() {
        if should_broadcast {
            log.push(entry.clone());
        } else {
            log.push(entry);
            return;
        }
    }
    let _ = tx.send(Arc::new(WsServerMessage::Interaction { entry }));
}
