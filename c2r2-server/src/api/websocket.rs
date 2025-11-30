//! WebSocket Handler for Real-time Events
//! 
//! This module handles WebSocket connections for team clients
//! to receive real-time events from the server.

use axum::{
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::IntoResponse,
    http::HeaderMap,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;

use super::models::ServerEvent;
use super::state::ApiState;

/// WebSocket upgrade handler for events endpoint
pub async fn events_handler(
    State(state): State<Arc<ApiState>>,
    ws: WebSocketUpgrade,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Extract token from query string or header
    // For WebSocket, we accept token in the Authorization header or as a query param
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string());
    
    ws.on_upgrade(move |socket| handle_websocket(socket, state, token))
}

/// Handle WebSocket connection
async fn handle_websocket(socket: WebSocket, state: Arc<ApiState>, token: Option<String>) {
    // Validate token
    if let Some(ref t) = token {
        if !state.validate_token(t).await {
            // Invalid token - close connection
            return;
        }
    } else {
        // No token - close connection
        return;
    }
    
    let (mut sender, mut receiver) = socket.split();
    
    // Subscribe to server events
    let mut event_rx = state.event_tx.subscribe();
    
    // Send initial state - list of all connected agents
    let agents = state.get_agents().await;
    for agent in agents {
        let event = ServerEvent::AgentConnected(agent);
        if let Ok(json) = serde_json::to_string(&event) {
            if sender.send(Message::Text(json.into())).await.is_err() {
                return;
            }
        }
    }
    
    // Spawn task to receive messages from client (heartbeat/commands)
    let _state_clone = state.clone();
    let _token_clone = token.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            match result {
                Ok(Message::Text(text)) => {
                    // Handle client messages (e.g., ping/pong, commands)
                    if text == "ping" {
                        // Client heartbeat - we'll respond in the send loop
                    }
                }
                Ok(Message::Ping(_data)) => {
                    // Handled automatically by axum
                }
                Ok(Message::Pong(_)) => {
                    // Pong received
                }
                Ok(Message::Close(_)) => {
                    break;
                }
                Err(_) => {
                    break;
                }
                _ => {}
            }
        }
    });
    
    // Send events to client
    let send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Receive server events and forward to client
                result = event_rx.recv() => {
                    match result {
                        Ok(event) => {
                            if let Ok(json) = serde_json::to_string(&event) {
                                if sender.send(Message::Text(json.into())).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            // Client is too slow, some events were dropped
                            let msg = ServerEvent::ServerMessage {
                                level: "warning".to_string(),
                                message: format!("Dropped {} events due to slow connection", n),
                            };
                            if let Ok(json) = serde_json::to_string(&msg) {
                                let _ = sender.send(Message::Text(json.into())).await;
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
                // Send periodic ping to keep connection alive
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                    if sender.send(Message::Ping(vec![].into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    });
    
    // Wait for either task to finish
    tokio::select! {
        _ = recv_task => {}
        _ = send_task => {}
    }
}
