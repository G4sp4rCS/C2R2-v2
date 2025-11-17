# Privilege Escalation: /elevate Command

## Overview

The `/elevate` command allows executing commands with administrator privileges on the target Windows system by triggering a UAC (User Account Control) prompt.

## Usage

```bash
C2R2 [ID]> /elevate <command>
```

### Examples

```bash
# Check elevated privileges
C2R2 [1]> /elevate whoami /priv

# Run administrative command
C2R2 [1]> /elevate net user Administrator newPassword123!

# Execute PowerShell with admin rights
C2R2 [1]> /elevate powershell Get-LocalGroupMember Administrators
```

## How It Works

1. **Server Side**: The `/elevate` command sends a `__ELEVATE__:<command>` message to the selected agent
2. **Agent Side**: The agent receives the command and:
   - Applies ArgFuscator obfuscation to the command
   - Uses PowerShell's `Start-Process` cmdlet with `-Verb RunAs` parameter
   - This triggers the Windows UAC prompt on the target system
   - If the user approves, the command executes with elevated privileges
   - Output is captured and sent back to the C2 server

## Technical Details

### PowerShell Elevation Mechanism

The agent uses the following PowerShell approach:

```powershell
Start-Process cmd.exe -ArgumentList '/c "<command> > %TEMP%\elevated_output.txt 2>&1"' -Verb RunAs -Wait -WindowStyle Hidden
```

- `Start-Process` - Launches a new process
- `-Verb RunAs` - Triggers UAC elevation prompt
- `-Wait` - Waits for the elevated process to complete
- `-WindowStyle Hidden` - Attempts to hide the command window (UAC prompt always visible)
- Output is redirected to a temp file and then retrieved

### Security Considerations

**Non-Silent Operation:**
- The UAC prompt will **always be visible** to the user
- This is by design and required by Windows security architecture
- Silent privilege escalation would require exploiting LPE (Local Privilege Escalation) vulnerabilities

**User Interaction Required:**
- Target user must click "Yes" on the UAC prompt
- If user clicks "No" or times out, the command fails
- This makes the operation non-stealthy but legitimate

**Social Engineering Context:**
- Best used when you have physical access or social engineering context
- Consider timing the elevation when user is actively using the system
- Alternatively, can be combined with a legitimate-looking installer/loader

## Alternative Approaches (Not Implemented)

As mentioned in the original issue, other potential approaches include:

1. **Legitimate Installer**: Package the agent as part of a legitimate-looking installer that requests UAC upfront
2. **Manual LPE Exploitation**: Use specific LPE exploits for vulnerable systems (requires case-by-case analysis)
3. **Service Manipulation**: If agent already has service privileges, can create/modify services

## Limitations

- **Requires User Approval**: UAC prompt must be accepted
- **Visible to User**: Cannot be completely stealthy
- **Detection Risk**: UAC prompts may be logged by EDR/SIEM
- **No Bypass**: Does not bypass Windows security controls

## Best Practices

1. **Timing**: Execute during active user sessions when prompts are expected
2. **Context**: Use with social engineering or legitimate cover story
3. **Fallback**: Have alternative methods ready if elevation fails
4. **OpSec**: Consider detection risks of UAC events in monitored environments

## Integration with Persistence

The `/elevate` command can be combined with `/persist` to establish admin-level persistence:

```bash
# First elevate privileges
C2R2 [1]> /elevate whoami /priv

# Then establish WMI persistence (requires admin)
C2R2 [1]> /persist wmi
```

## Response Format

### Success
```
__INFO__:Comando elevado ejecutado exitosamente
<command output>
```

### Failure (User Rejected UAC)
```
__ERROR__:Error al elevar comando (¿Usuario rechazó UAC?)
<error details>
```

### PowerShell Error
```
__ERROR__:Error ejecutando PowerShell para elevación: <error>
```

## See Also

- `/persist` - Establish persistence mechanisms
- `/cmd` - Execute regular commands without elevation
- [Windows UAC Documentation](https://docs.microsoft.com/en-us/windows/security/identity-protection/user-account-control/)
