# Anti-Sandbox and Anti-Analysis Implementation

## Overview

This document describes the comprehensive anti-sandbox and anti-analysis features implemented in the C2R2-v2 agent. These features are **only active in production mode** and provide multiple layers of detection to evade automated malware analysis environments.

## Conditional Compilation

All anti-sandbox features are conditionally compiled using:

```rust
#[cfg(all(feature = "production", target_os = "windows"))]
```

This means:
- **Development Mode**: All checks are disabled (always return `false`)
- **Production Mode**: Full anti-analysis suite is enabled
- **Windows Only**: Checks only run on Windows targets

## Features Implemented

### 1. VM Detection

The agent detects if it's running inside a Virtual Machine using multiple techniques:

#### System Manufacturer Detection
- Checks for VM vendors: VMware, VirtualBox, QEMU, Hyper-V, Xen, Parallels
- Uses `wmic computersystem get manufacturer`

#### Registry Key Detection
Checks for VM-specific registry keys:
- VMware: `HKLM\SOFTWARE\VMware, Inc.\VMware Tools`
- VirtualBox: `HKLM\SOFTWARE\Oracle\VirtualBox Guest Additions`
- And more...

#### File System Detection
Looks for VM-specific files and drivers:
- `C:\windows\System32\Drivers\Vmmouse.sys` (VMware)
- `C:\windows\System32\Drivers\VBoxMouse.sys` (VirtualBox)
- And more...

#### MAC Address Pattern Detection
Checks for VM-specific MAC address prefixes:
- `00:05:69`, `00:0c:29`, `00:1c:14`, `00:50:56` (VMware)
- `08:00:27` (VirtualBox)
- `52:54:00` (QEMU/KVM)
- `00:15:5d` (Hyper-V)

### 2. Sandbox Detection

Identifies common malware analysis sandboxes:

#### Process Detection
Looks for analysis tools running on the system:
- Sandbox processes: `vboxray.exe`, `vmwaretray.exe`, `sandboxiedcomlaunch.exe`
- Monitoring tools: `procmon.exe`, `procexp.exe`, `wireshark.exe`, `fiddler.exe`
- Debuggers: `ollydbg.exe`, `ida.exe`, `ida64.exe`, `x64dbg.exe`, `windbg.exe`

#### File System Artifacts
Checks for sandbox-specific paths:
- `C:\analysis`
- `C:\sandbox`
- `C:\sample.exe`
- `C:\malware.exe`

#### Wine Detection
Detects Windows emulation layer (used by some Linux-based sandboxes):
- Checks registry: `HKCU\Software\Wine`

### 3. Resource-Based Detection

Many sandboxes allocate minimal resources to speed up analysis:

#### Memory Detection
- Flags systems with **< 4GB RAM** as suspicious
- Uses `wmic computersystem get totalphysicalmemory`

#### CPU Core Detection
- Flags systems with **< 2 CPU cores** as suspicious
- Uses `wmic cpu get numberofcores`

#### Disk Size Detection
- Flags systems with **< 60GB disk** as suspicious
- Uses `wmic logicaldisk where DeviceID='C:' get size`

### 4. Debugger Detection

Multiple techniques to detect debuggers:

#### IsDebuggerPresent API
- Uses Windows API `IsDebuggerPresent()` function
- Standard detection method

#### PEB BeingDebugged Flag
- Direct check of Process Environment Block (PEB)
- Uses inline assembly to access PEB:
  ```rust
  std::arch::asm!(
      "mov {}, gs:[0x60]",  // Access PEB via TEB
      out(reg) peb,
  );
  ```
- Checks BeingDebugged flag at offset 0x02

### 5. Time Acceleration Detection

Some sandboxes artificially accelerate time to speed up analysis:

```rust
pub fn detect_time_acceleration() -> bool {
    let start = Instant::now();
    thread::sleep(Duration::from_secs(1));
    let elapsed = start.elapsed();
    
    // If less than 900ms elapsed, time was accelerated
    if elapsed.as_millis() < 900 {
        return true;
    }
    false
}
```

## Integration

### Agent Startup

The anti-sandbox checks run at the very beginning of `main()`:

```rust
fn main() {
    // In production mode, perform comprehensive anti-analysis checks
    #[cfg(feature = "production")]
    {
        if evasion::run_anti_sandbox_checks() {
            // Sandbox detected - exit silently
            std::process::exit(0);
        }
    }
    
    // Normal agent execution continues...
}
```

### Behavior on Detection

If **any** sandbox indicator is detected:
1. Agent exits **silently** with `std::process::exit(0)`
2. No error messages or logs are generated
3. No network connections are made
4. Prevents analysis in automated environments

## Building

### Development Build (No Anti-Sandbox)
```bash
cd agent
cargo build --release --features dev
# OR
cargo build --release  # dev is default
```

### Production Build (With Anti-Sandbox)
```bash
cd agent
cargo build --release --no-default-features --features production
```

### Using Builder Tool
```bash
cd builder

# Development agent (no anti-sandbox)
cargo run --release -- build-agent --name agent-dev --server 192.168.1.10:4444

# Production agent (with anti-sandbox)
cargo run --release -- build-agent --name agent-prod --server 192.168.1.10:4444 --production
```

## Testing

### Verifying Dev Mode (Anti-Sandbox Disabled)
```bash
cargo build --release --features dev
# Agent runs normally even in VM/sandbox
```

### Verifying Production Mode (Anti-Sandbox Enabled)
```bash
cargo build --release --no-default-features --features production
# Agent exits silently if VM/sandbox/debugger detected
```

### Testing on Real Hardware
The agent will run normally on real hardware that doesn't match sandbox characteristics:
- Physical RAM ≥ 4GB
- CPU cores ≥ 2
- Disk size ≥ 60GB
- No VM artifacts
- No debugger attached
- No sandbox processes

## Evasion Techniques Inspired By

The implementation was inspired by techniques from:
- **DarkFunct/Rust-Ransomware**: VM and sandbox detection patterns
- **Idov31/rustomware**: Multi-layered detection approach
- **1N73LL1G3NC3x/Nightmangle**: Time-based and resource-based checks

## Detection Matrix

| Check Type | Method | Detection Threshold |
|------------|--------|-------------------|
| VM Vendor | System manufacturer | VMware, VBox, QEMU, Hyper-V, Xen, Parallels |
| VM Registry | Registry keys | VM-specific keys present |
| VM Files | File system | VM driver files exist |
| VM MAC | Network interface | Known VM MAC prefixes |
| Sandbox Process | Process list | Analysis tools running |
| Sandbox Files | File system | Sandbox paths exist |
| Wine | Registry | Wine registry keys |
| Low RAM | System info | < 4GB |
| Low CPU | System info | < 2 cores |
| Small Disk | System info | < 60GB |
| Debugger API | IsDebuggerPresent | Non-zero return |
| Debugger PEB | Direct memory | BeingDebugged flag set |
| Time Accel | Sleep timing | < 900ms for 1s sleep |

## OPSEC Considerations

### When to Use Production Mode
✅ **Use production mode for:**
- Real red team operations
- Adversary simulations
- Production deployments
- Any live engagement

❌ **Don't use production mode for:**
- Local testing and development
- Debugging issues
- Learning and experimentation
- Safe lab environments

### Limitations
While comprehensive, these checks are **not foolproof**:
- Advanced sandboxes may evade detection
- Bare-metal analysis systems will not be detected
- Custom VM configurations may not match patterns
- Future analysis tools may bypass these checks

### Defense in Depth
Anti-sandbox is just one layer. Combine with:
- Network traffic obfuscation
- Command obfuscation (already implemented via ArgFuscator)
- Beacon timing with jitter
- AMSI/ETW bypasses (already implemented)
- String obfuscation (already implemented via obfstr)

## Code Organization

### Files Modified
- **agent/src/evasion.rs**: Main anti-sandbox implementation (~500 new lines)
- **agent/src/main.rs**: Integration at startup
- **agent/src/syscalls.rs**: Windows-only conditional compilation fixes
- **agent/Cargo.toml**: Added winapi features (debugapi, sysinfoapi)

### Public API
```rust
// Main entry point - runs all checks
pub fn run_anti_sandbox_checks() -> bool

// Individual check functions (production + windows only)
pub fn is_sandbox() -> bool
pub fn detect_time_acceleration() -> bool

// Helper functions (all private, production + windows only)
fn detect_vm() -> bool
fn detect_sandbox_artifacts() -> bool
fn detect_low_resources() -> bool
fn detect_debugger() -> bool
// ... and many more helper functions
```

## Security Considerations

### Information Disclosure
The anti-sandbox checks use Windows commands that may be logged:
- `wmic` queries for system info
- `tasklist` for process enumeration
- `reg query` for registry checks
- `getmac` for MAC addresses

These are **normal system commands** and typically don't raise alerts, but be aware they may appear in:
- Windows Event Logs
- EDR telemetry
- Sysmon logs

### Exit Behavior
On detection, the agent:
- Exits with code 0 (success)
- No error messages
- No exceptions
- No network activity

This makes it appear as if the agent ran and completed normally.

## Future Enhancements

Potential improvements for future versions:
- [ ] CPUID-based VM detection
- [ ] Hardware performance counter checks
- [ ] User interaction detection (mouse movement, keyboard)
- [ ] Disk I/O performance checks
- [ ] Advanced PEB/TEB analysis
- [ ] Parent process validation
- [ ] Digital signature verification of system files
- [ ] Recent file access pattern analysis

## References

- [Windows Anti-Debug Reference](https://anti-debug.checkpoint.com/)
- [Al-Khaser: Anti-Malware Techniques](https://github.com/LordNoteworthy/al-khaser)
- [Pafish: Sandbox Detection](https://github.com/a0rtega/pafish)
- [Rust Ransomware Samples](https://github.com/DarkFunct/Rust-Ransomware)

---

**⚠️ Legal Notice**: These anti-sandbox features are designed for authorized security testing and red team operations only. Use responsibly and only on systems you have permission to test.
