use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use tracing::{debug, warn};

use crate::routes::AppState;

pub async fn ws_interfaces(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.tx.subscribe();
    debug!("websocket client connected");

    loop {
        match rx.recv().await {
            Ok(snapshot) => {
                let payload = match serde_json::to_string(&snapshot) {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("failed to serialize snapshot for websocket: {e}");
                        continue;
                    }
                };
                if socket.send(Message::Text(payload)).await.is_err() {
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                warn!("websocket client lagged, skipped {skipped} snapshots");
                continue;
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }

    debug!("websocket client disconnected");
}
