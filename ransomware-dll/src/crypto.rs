use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
/// Cryptographic operations module
/// This module handles encryption and decryption operations using AES-256-CBC and ChaCha20-Poly1305
use aes::Aes256;
use chacha20poly1305::{
    aead::{Aead, OsRng},
    ChaCha20Poly1305, Nonce,
};
use rand::Rng;

/// Generate a random 32-byte encryption key
pub fn generate_key() -> [u8; 32] {
    let mut rng = rand::thread_rng();
    let mut key = [0u8; 32];
    rng.fill(&mut key);
    key
}

/// Generate a random 16-byte initialization vector (IV)
pub fn generate_iv() -> [u8; 16] {
    let mut rng = rand::thread_rng();
    let mut iv = [0u8; 16];
    rng.fill(&mut iv);
    iv
}

/// Generate a random 12-byte nonce for ChaCha20-Poly1305
pub fn generate_nonce() -> [u8; 12] {
    let mut rng = rand::thread_rng();
    let mut nonce = [0u8; 12];
    rng.fill(&mut nonce);
    nonce
}

/// Encrypt data using ChaCha20-Poly1305 (AEAD - more modern and secure)
/// Returns the encrypted data with nonce prepended
pub fn encrypt_data_chacha(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    let cipher = ChaCha20Poly1305::new(key.into());
    let nonce_bytes = generate_nonce();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| format!("ChaCha20 encryption failed: {:?}", e))?;

    // Prepend nonce to ciphertext
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// Decrypt data using ChaCha20-Poly1305
/// Expects nonce to be prepended to the encrypted data
pub fn decrypt_data_chacha(encrypted: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    if encrypted.len() < 12 {
        return Err("Data too short".to_string());
    }

    // Extract nonce from the beginning
    let nonce = Nonce::from_slice(&encrypted[..12]);
    let ciphertext = &encrypted[12..];

    let cipher = ChaCha20Poly1305::new(key.into());
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("ChaCha20 decryption failed: {:?}", e))?;

    Ok(plaintext)
}

/// Encrypt data using AES-256-CBC
/// Returns the encrypted data with IV prepended
pub fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    let iv = generate_iv();
    let cipher = Aes256::new(key.into());

    // Pad the data to block size (16 bytes)
    let mut buffer = data.to_vec();
    let padding_needed = 16 - (buffer.len() % 16);
    if padding_needed != 16 {
        buffer.extend(vec![padding_needed as u8; padding_needed]);
    } else {
        buffer.extend(vec![16u8; 16]);
    }

    // Encrypt using CBC mode manually
    let mut previous_block = iv;
    let mut encrypted = Vec::new();

    for chunk in buffer.chunks_exact(16) {
        // XOR with previous block (CBC mode)
        let mut block_data = [0u8; 16];
        for i in 0..16 {
            block_data[i] = chunk[i] ^ previous_block[i];
        }

        // Encrypt the block
        let mut block = aes::cipher::generic_array::GenericArray::from(block_data);
        cipher.encrypt_block(&mut block);

        encrypted.extend_from_slice(&block);
        previous_block = *block.as_ref();
    }

    // Prepend IV to encrypted data
    let mut result = iv.to_vec();
    result.extend_from_slice(&encrypted);

    Ok(result)
}

/// Decrypt data using AES-256-CBC
/// Expects IV to be prepended to the encrypted data
pub fn decrypt_data(encrypted: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    if encrypted.len() < 16 {
        return Err("Data too short".to_string());
    }

    // Extract IV from the beginning
    let mut iv_array = [0u8; 16];
    iv_array.copy_from_slice(&encrypted[..16]);
    let data = &encrypted[16..];

    if data.len() % 16 != 0 {
        return Err("Invalid encrypted data length".to_string());
    }

    let cipher = Aes256::new(key.into());

    // Decrypt using CBC mode manually
    let mut previous_block = iv_array;
    let mut decrypted = Vec::new();

    for chunk in data.chunks_exact(16) {
        // Decrypt the block
        let mut block = aes::cipher::generic_array::GenericArray::clone_from_slice(chunk);
        let mut encrypted_block = [0u8; 16];
        encrypted_block.copy_from_slice(chunk);

        cipher.decrypt_block(&mut block);

        // XOR with previous block (CBC mode)
        let mut block_data = [0u8; 16];
        for i in 0..16 {
            block_data[i] = block[i] ^ previous_block[i];
        }

        decrypted.extend_from_slice(&block_data);
        previous_block = encrypted_block;
    }

    // Remove padding
    if let Some(&padding_len) = decrypted.last() {
        if padding_len as usize <= 16 && padding_len > 0 {
            let len = decrypted.len();
            if len >= padding_len as usize {
                decrypted.truncate(len - padding_len as usize);
            }
        }
    }

    Ok(decrypted)
}
