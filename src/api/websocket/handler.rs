//! WebSocket handler for real-time updates to TUI clients.
//!
//! Connected clients receive push notifications whenever agent state changes,
//! allowing the TUI to update without polling. Messages are broadcast via
//! a Tokio broadcast channel that all WebSocket connections subscribe to.
//!
//! # Protocol
//!
//! - Server → Client: `WsMessage::AgentUpdate`, `AgentDeleted`, `TriageRunUpdate`, `InboxStateUpdate`
//! - Client → Server: `WsMessage::Ping` for keep-alive (optional)

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;

use super::super::types::ApiState;
use super::types::WsMessage;

/// Handles WebSocket upgrade requests and spawns the connection handler.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state.ws_broadcast.subscribe()))
}

async fn handle_socket(socket: WebSocket, mut rx: broadcast::Receiver<WsMessage>) {
    let (mut sender, mut receiver) = socket.split();

    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                        if matches!(ws_msg, WsMessage::Ping) {
                            log::debug!("Received ping from client");
                        }
                    }
                }
                Message::Ping(_) => {
                    log::debug!("Received WebSocket ping");
                }
                Message::Close(_) => {
                    log::info!("WebSocket client disconnected");
                    break;
                }
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    log::info!("WebSocket connection closed");
}
