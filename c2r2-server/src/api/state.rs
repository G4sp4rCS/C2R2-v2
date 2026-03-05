//! API State Management
//!
//! Shared state between the API handlers and the main server.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc, RwLock};

use super::models::{AgentInfo, ServerEvent};

pub type ClientId = u64;

/// Token for authenticated sessions
#[derive(Debug, Clone)]
pub struct AuthToken {
    pub token: String,
    pub username: String,
    pub created_at: Instant,
}

/// Shared state for the API
pub struct ApiState {
    /// Connected agents (shared with main server)
    pub agents: Arc<RwLock<HashMap<ClientId, AgentState>>>,

    /// Channel to send commands to agents
    pub command_tx: Arc<RwLock<HashMap<ClientId, mpsc::UnboundedSender<String>>>>,

    /// Broadcast channel for server events (to WebSocket clients)
    pub event_tx: broadcast::Sender<ServerEvent>,

    /// Active authentication tokens
    pub auth_tokens: Arc<RwLock<HashMap<String, AuthToken>>>,

    /// Fingerprints of disconnected agents: "hostname|username" → old ClientId
    /// Used to reassign the same ID when an agent reconnects
    pub disconnected_fingerprints: Arc<RwLock<HashMap<String, ClientId>>>,

    /// API password (configured at startup)
    pub api_password: String,

    /// Server start time
    pub start_time: Instant,

    /// Verbose mode
    pub verbose: bool,
}

/// Agent state stored in the API
#[derive(Debug, Clone)]
pub struct AgentState {
    pub info: AgentInfo,
    pub tx: mpsc::UnboundedSender<String>,
}

impl ApiState {
    pub fn new(api_password: String, verbose: bool) -> Self {
        let (event_tx, _) = broadcast::channel(1024);

        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            command_tx: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            auth_tokens: Arc::new(RwLock::new(HashMap::new())),
            disconnected_fingerprints: Arc::new(RwLock::new(HashMap::new())),
            api_password,
            start_time: Instant::now(),
            verbose,
        }
    }

    /// Add a new agent
    pub async fn add_agent(
        &self,
        id: ClientId,
        info: AgentInfo,
        tx: mpsc::UnboundedSender<String>,
    ) {
        {
            let mut agents = self.agents.write().await;
            agents.insert(
                id,
                AgentState {
                    info: info.clone(),
                    tx: tx.clone(),
                },
            );
        }
        {
            let mut command_tx = self.command_tx.write().await;
            command_tx.insert(id, tx);
        }

        // Broadcast event to team clients
        let _ = self.event_tx.send(ServerEvent::AgentConnected(info));
    }

    /// Remove an agent (saves fingerprint for reconnection)
    pub async fn remove_agent(&self, id: ClientId) {
        // Save fingerprint before removing so we can recognize reconnections
        {
            let agents = self.agents.read().await;
            if let Some(agent) = agents.get(&id) {
                if let (Some(hostname), Some(username)) =
                    (&agent.info.hostname, &agent.info.username)
                {
                    let fingerprint = format!("{}|{}", hostname, username);
                    let mut fps = self.disconnected_fingerprints.write().await;
                    fps.insert(fingerprint, id);
                }
            }
        }
        {
            let mut agents = self.agents.write().await;
            agents.remove(&id);
        }
        {
            let mut command_tx = self.command_tx.write().await;
            command_tx.remove(&id);
        }

        // Broadcast event to team clients
        let _ = self.event_tx.send(ServerEvent::AgentDisconnected { id });
    }

    /// Check if a hostname+username matches a previously disconnected agent.
    /// Returns the old ClientId if found (and removes the entry).
    pub async fn check_reconnection(&self, hostname: &str, username: &str) -> Option<ClientId> {
        let fingerprint = format!("{}|{}", hostname, username);
        let mut fps = self.disconnected_fingerprints.write().await;
        fps.remove(&fingerprint)
    }

    /// Reassign an agent from `old_id` to `new_id`.
    /// Moves the agent entry in all maps and notifies team clients.
    pub async fn reassign_agent_id(
        &self,
        old_id: ClientId,
        new_id: ClientId,
        new_tx: mpsc::UnboundedSender<String>,
    ) {
        // Remove old_id entry, update its id field, insert under new_id
        {
            let mut agents = self.agents.write().await;
            if let Some(mut agent_state) = agents.remove(&old_id) {
                agent_state.info.id = new_id;
                agent_state.tx = new_tx.clone();
                agents.insert(new_id, agent_state);
            }
        }
        {
            let mut command_tx = self.command_tx.write().await;
            command_tx.remove(&old_id);
            command_tx.insert(new_id, new_tx);
        }

        // Notify team clients: remove old temporary ID, announce reconnection under real ID
        let _ = self.event_tx.send(ServerEvent::AgentDisconnected { id: old_id });
        let agents = self.agents.read().await;
        if let Some(agent) = agents.get(&new_id) {
            let _ = self
                .event_tx
                .send(ServerEvent::AgentConnected(agent.info.clone()));
        }
    }

    /// Update agent info
    pub async fn update_agent_info(&self, id: ClientId, updater: impl FnOnce(&mut AgentInfo)) {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(&id) {
            updater(&mut agent.info);
            // Broadcast updated info
            let _ = self
                .event_tx
                .send(ServerEvent::AgentUpdated(agent.info.clone()));
        }
    }

    /// Get all agents
    pub async fn get_agents(&self) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        agents.values().map(|a| a.info.clone()).collect()
    }

    /// Get a specific agent
    pub async fn get_agent(&self, id: ClientId) -> Option<AgentInfo> {
        let agents = self.agents.read().await;
        agents.get(&id).map(|a| a.info.clone())
    }

    /// Send a command to an agent
    pub async fn send_command(&self, id: ClientId, command: String) -> Result<(), String> {
        let command_tx = self.command_tx.read().await;
        if let Some(tx) = command_tx.get(&id) {
            tx.send(command)
                .map_err(|e| format!("Failed to send command: {}", e))
        } else {
            Err(format!("Agent {} not found", id))
        }
    }

    /// Send a command to all agents
    pub async fn send_command_all(&self, command: String) -> Vec<(ClientId, Result<(), String>)> {
        let command_tx = self.command_tx.read().await;
        command_tx
            .iter()
            .map(|(id, tx)| {
                let result = tx
                    .send(command.clone())
                    .map_err(|e| format!("Failed to send command: {}", e));
                (*id, result)
            })
            .collect()
    }

    /// Broadcast an event to team clients
    pub fn broadcast_event(&self, event: ServerEvent) {
        let _ = self.event_tx.send(event);
    }

    /// Validate an auth token
    pub async fn validate_token(&self, token: &str) -> bool {
        let tokens = self.auth_tokens.read().await;
        tokens.contains_key(token)
    }

    /// Create a new auth token
    pub async fn create_token(&self, username: String) -> String {
        let token = uuid::Uuid::new_v4().to_string();
        let auth_token = AuthToken {
            token: token.clone(),
            username,
            created_at: Instant::now(),
        };

        let mut tokens = self.auth_tokens.write().await;
        tokens.insert(token.clone(), auth_token);
        token
    }

    /// Remove an auth token
    pub async fn remove_token(&self, token: &str) {
        let mut tokens = self.auth_tokens.write().await;
        tokens.remove(token);
    }
}
