//! Cryptographic operations for JAVELIN
//!
//! Supports XOR and AES-256-GCM decryption
//! Reuses the same algorithms as builder/dll_encrypt.rs for consistency

/// Supported cryptographic algorithms
#[derive(Debug, Clone, Copy)]
pub enum CryptoAlgorithm {
    /// Simple XOR encryption (fast, minimal dependencies)
    Xor,
    /// AES-256-GCM (stronger, but larger binary)
    Aes256Gcm,
}

/// Decrypts a payload using the specified algorithm
///
/// # Arguments
///
/// * `encrypted` - The encrypted payload bytes
/// * `key` - The decryption key
/// * `algorithm` - The algorithm to use for decryption
///
/// # Returns
///
/// * `Ok(Vec<u8>)` - Decrypted payload
/// * `Err(String)` - Decryption error
pub fn decrypt_payload(
    encrypted: &[u8],
    key: &[u8],
    algorithm: CryptoAlgorithm,
) -> Result<Vec<u8>, String> {
    match algorithm {
        CryptoAlgorithm::Xor => Ok(xor_encrypt(encrypted, key)),
        CryptoAlgorithm::Aes256Gcm => {
            // AES-256-GCM would require aes-gcm crate
            // For now, not implemented to keep binary size small
            // Can be added if needed for enhanced security
            Err("AES-256-GCM not yet implemented in JAVELIN".to_string())
        }
    }
}

/// XOR encryption/decryption (symmetric)
///
/// This is the same algorithm used throughout C2R2-v2:
/// - builder/src/dll_encrypt.rs
/// - dropper-rust/src/shellcode.rs
/// - stages/ester/src/stage_trigger.rs
///
/// **Why XOR**:
/// - Symmetric (same function for encrypt/decrypt)
/// - Fast (no complex math)
/// - Small binary size (no crypto libraries)
/// - Good enough for in-memory payload protection
///
/// **Security note**:
/// XOR is not cryptographically strong, but for in-memory payloads
/// that only exist briefly, it provides sufficient obfuscation
pub fn xor_encrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect()
}

/// Securely zeros memory to prevent forensic recovery
///
/// Uses `volatile_write` to prevent compiler optimization from removing the zeroing
///
/// # Arguments
///
/// * `buffer` - Mutable slice to zero
pub fn secure_zero(buffer: &mut [u8]) {
    for byte in buffer.iter_mut() {
        unsafe {
            std::ptr::write_volatile(byte, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xor_encrypt_decrypt() {
        let data = b"Test payload data";
        let key = b"secret_key";

        let encrypted = xor_encrypt(data, key);
        assert_ne!(encrypted, data.to_vec());

        let decrypted = xor_encrypt(&encrypted, key);
        assert_eq!(decrypted, data.to_vec());
    }

    #[test]
    fn test_decrypt_payload_xor() {
        let data = b"Test payload";
        let key = b"key";

        let encrypted = xor_encrypt(data, key);
        let decrypted = decrypt_payload(&encrypted, key, CryptoAlgorithm::Xor).unwrap();

        assert_eq!(decrypted, data.to_vec());
    }

    #[test]
    fn test_secure_zero() {
        let mut buffer = vec![0xFF; 100];
        secure_zero(&mut buffer);
        
        assert!(buffer.iter().all(|&b| b == 0));
    }
}
