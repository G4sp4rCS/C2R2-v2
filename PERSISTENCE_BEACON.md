# Persistence and Stealthy Communication Implementation

## Overview

This document describes the implementation of persistence mechanisms and modern C2 beacon communication patterns to avoid heuristic detection by EDR/AV solutions.

## Problem Statement

Modern security solutions (Windows Defender, EDR) use heuristic analysis to detect C2 communication patterns:
- **Constant connection attempts**: Predictable reconnection every 10 seconds
- **Regular ping intervals**: Fixed 30-second keep-alive patterns
- **Network behavior**: Constant traffic patterns are easily fingerprinted

## Solution: Modern C2 Beacon Pattern

### Beacon Communication

Inspired by Cobalt Strike, Havoc, and other modern C2 frameworks, we implemented:

#### 1. **Configurable Beacon Intervals**
Instead of constant connections, the agent "sleeps" between beacons:
- Default: 60 seconds between connection attempts
- Configurable via server command: `/beacon 120:40` (120s ±40% jitter)

#### 2. **Jitter Implementation**
Random variance added to beacon times to break pattern detection:
- Default: ±30% jitter
- Example: 60s beacon becomes random interval between 42-78 seconds
- Uses SystemTime as pseudo-random seed for unpredictability

#### 3. **Exponential Backoff**
On connection failure, wait time increases progressively:
- First retry: 10 seconds
- Second retry: 20 seconds  
- Third retry: 40 seconds
- ...continues doubling up to max (600 seconds / 10 minutes)
- Prevents noisy reconnection storms that trigger alerts

#### 4. **Anti-Sandbox Sleep**
Sophisticated sleep mechanism that defeats sandbox acceleration:
- Divides sleep into random chunks (1-5 seconds)
- Prevents sandbox from detecting and skipping long sleeps
- Makes dynamic analysis more difficult

### Configuration

The beacon system can be configured via:

**Server Command:**
```
/beacon <interval:jitter>
```

**Examples:**
- `/beacon 60:30` - 60 seconds with ±30% jitter (default)
- `/beacon 120:40` - 120 seconds with ±40% jitter (more stealthy)
- `/beacon 300:50` - 5 minutes with ±50% jitter (very stealthy)

**Agent Configuration:**
The agent uses `BeaconConfig` struct:
```rust
pub struct BeaconConfig {
    pub interval: u64,              // Base interval in seconds
    pub jitter_percent: u32,        // Jitter percentage (0-100)
    pub max_retry_interval: u64,    // Max backoff time
    pub initial_retry_interval: u64, // Initial retry time
}
```

## Persistence Mechanisms

Multiple persistence methods implemented, from simple to APT-like:

### 1. Registry Run Key (Simple)
**Method:** `registry`  
**Location:** `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`  
**Privileges Required:** User  
**Detection Level:** Medium-High (commonly monitored)

**Characteristics:**
- Executes on user login
- Easy to implement and reliable
- Commonly monitored by security tools
- Names mimicked from legitimate software:
  - "Windows Security Update"
  - "System Runtime Service"
  - "Windows Defender Update"
  - "Microsoft Compatibility Telemetry"

### 2. Scheduled Task (Sophisticated)
**Method:** `task`  
**Privileges Required:** User  
**Detection Level:** Medium (less commonly monitored)

**Characteristics:**
- Uses `schtasks` command
- Executes on logon with HIGHEST privileges
- Appears as legitimate scheduled task
- Names mimicked from real tasks:
  - "MicrosoftEdgeUpdateTaskMachineCore"
  - "GoogleUpdateTaskMachineUA"
  - "Adobe Acrobat Update Task"

### 3. WMI Event Subscription (APT-like) ⚠️
**Method:** `wmi`  
**Privileges Required:** Administrator (usually)  
**Detection Level:** Low (APT technique)

**Characteristics:**
- Uses Windows Management Instrumentation
- Creates event filter + consumer + binding
- Triggers every 2 hours based on system events
- Very stealthy, hard to detect without specialized tools
- Preferred method for advanced threats
- Names mimicked from system components:
  - "SCM Event Log Consumer"
  - "BfeOnServiceStartTypeChange"
  - "WUAU Service Status"

**Implementation:**
```powershell
# Event Filter: Trigger on system performance data
$Query = "SELECT * FROM __InstanceModificationEvent WITHIN 7200 
          WHERE TargetInstance ISA 'Win32_PerfFormattedData_PerfOS_System'"

# Consumer: Execute agent binary
CommandLineEventConsumer -> Execute agent

# Binding: Link filter to consumer
```

### 4. Startup Folder (Fallback)
**Method:** `startup`  
**Location:** `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup`  
**Privileges Required:** User  
**Detection Level:** High (easily visible)

**Characteristics:**
- Simplest method
- Executes on user login
- Easy to detect but reliable
- Good fallback if other methods fail

## Agent Installation

Before establishing persistence, the agent:

1. **Copies itself to `%APPDATA%`** with legitimate-looking name:
   - Subdirectory: `.svchost`, `.RuntimeBroker`, etc. (hidden)
   - Filename: `svchost.exe`, `RuntimeBroker.exe`, etc.
   - Names selected pseudo-randomly based on PID

2. **Sets hidden attribute** on directory

3. **Uses copied binary** for persistence (not original)

## Usage

### Server Commands

**Establish Persistence:**
```bash
# Select a client first
/select 1

# Choose persistence method
/persist registry    # Simple, commonly monitored
/persist task        # Sophisticated, less monitored  
/persist wmi         # APT-like, very stealthy (requires admin)
/persist startup     # Fallback, easily visible
```

**Remove Persistence:**
```bash
/persist_remove      # Cleans all persistence methods
```

**Configure Beacon:**
```bash
/beacon 120:40       # 120s with ±40% jitter
```

### Agent Responses

**Success:**
```
__SUCCESS__:Persistencia Registry Run establecida: HKCU\...\Run\Windows Security Update
__SUCCESS__:Persistencia Scheduled Task establecida: MicrosoftEdgeUpdateTaskMachineCore
__SUCCESS__:Persistencia WMI Event establecida: SCM Event Log Consumer
```

**Errors:**
```
__ERROR__:Error estableciendo persistencia: Access denied
__ERROR__:Método de persistencia inválido. Usar: registry|task|wmi|startup
```

## Security Considerations

### Evasion Techniques Used

1. **Mimicry**: All persistence names mimic legitimate Windows/software components
2. **Hidden directories**: Agent installed in hidden subdirectories
3. **Beacon jitter**: Unpredictable communication timing
4. **Exponential backoff**: Avoids noisy reconnection patterns
5. **Pseudo-random naming**: Different names per installation based on PID

### Detection Risks

| Method | Detection Risk | Notes |
|--------|---------------|-------|
| Registry Run | Medium-High | Monitored by most AV/EDR |
| Scheduled Task | Medium | Less commonly monitored |
| WMI Events | Low | APT technique, specialized detection needed |
| Startup Folder | High | Easily visible to users |

### Best Practices

1. **Use WMI for critical operations**: Lowest detection rate, requires admin
2. **Use Task for standard ops**: Good balance of stealth/reliability
3. **Avoid Startup folder**: Only as last resort
4. **Configure long beacon intervals**: 2-5 minutes for production
5. **High jitter on production**: 40-50% jitter for operational use
6. **Test persistence**: Use `/persist_remove` to verify cleanup

## Implementation Details

### File Structure

```
agent/src/
├── main.rs           # Main agent, command handling
├── beacon.rs         # Beacon/jitter implementation
├── persistence.rs    # Persistence methods
├── config.rs         # Configuration
└── evasion.rs        # AMSI/ETW bypass
```

### Key Functions

**Beacon Module:**
- `calculate_beacon_interval()`: Calculates next beacon with jitter
- `calculate_retry_interval()`: Exponential backoff calculation
- `beacon_sleep()`: Interruptible sleep in chunks
- `anti_sandbox_sleep()`: Anti-analysis sleep technique

**Persistence Module:**
- `establish_persistence()`: Main persistence handler
- `install_agent()`: Copies agent to %APPDATA%
- `persist_registry_run()`: Registry Run key method
- `persist_scheduled_task()`: Scheduled Task method
- `persist_wmi_event()`: WMI Event Subscription method
- `persist_startup_folder()`: Startup folder method
- `remove_persistence()`: Cleanup all methods

## Testing

### Manual Testing

1. **Test beacon intervals:**
   ```bash
   # Watch agent logs for timing
   /beacon 30:50  # Should vary between 15-45 seconds
   ```

2. **Test persistence:**
   ```bash
   /persist task
   # Reboot VM
   # Agent should reconnect automatically
   ```

3. **Test cleanup:**
   ```bash
   /persist_remove
   # Verify registry/tasks are cleaned
   ```

### Verification

**Check Registry:**
```cmd
reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run"
```

**Check Scheduled Tasks:**
```cmd
schtasks /query /fo LIST /v | findstr "MicrosoftEdge"
```

**Check WMI Events:**
```powershell
Get-WmiObject -Namespace root\subscription -Class __EventFilter
Get-WmiObject -Namespace root\subscription -Class CommandLineEventConsumer
```

**Check Startup Folder:**
```cmd
dir "%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup"
```

## Future Enhancements

1. **DLL Hijacking**: More advanced persistence via DLL search order
2. **Service Installation**: Persistent system service
3. **COM Object Hijacking**: Advanced registry persistence
4. **Beacon encryption**: Encrypt beacon traffic to avoid IDS
5. **Domain fronting**: Hide C2 traffic behind CDNs
6. **Persistent beacon config**: Save config to file for reuse

## References

- **Cobalt Strike**: Industry-standard C2 framework
- **Havoc Framework**: Modern open-source C2
- **MITRE ATT&CK**:
  - T1547.001: Registry Run Keys
  - T1053.005: Scheduled Task
  - T1546.003: WMI Event Subscription
- **Windows Internals**: Understanding WMI persistence
