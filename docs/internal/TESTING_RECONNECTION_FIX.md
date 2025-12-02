# Testing Guide: Reconnection Fix

## Overview

This document describes how to test the fix for the constant reconnection issue where agents would disconnect every 5 minutes and reconnect with a new client ID.

## Pre-requisites

- Windows VM or machine for testing
- C2R2 server accessible from the test machine
- Ability to build the agent with the fix

## Build the Fixed Agent

```bash
cd /home/runner/work/C2R2-v2/C2R2-v2/builder

# Build agent with the fix
cargo run --release -- build-agent \
  --name test-agent-fixed \
  --server "SERVER_IP:4444" \
  --production
```

## Test Procedure

### Test 1: Basic Connection Stability (15 minutes)

**Objective**: Verify agent stays connected for extended period without commands

1. **Start the server:**
   ```bash
   cd c2r2-server
   cargo run --release -- --bind 0.0.0.0 --port 4444 --verbose
   ```

2. **Deploy and run the agent** on test machine

3. **Observe initial connection:**
   ```
   C2R2> 
   ? Nuevo cliente [1] desde 192.168.1.1:xxxxx
   ```
   Note the client ID (should be 1)

4. **Wait 6 minutes without sending commands**
   - Set a timer for 6 minutes
   - Do NOT send any commands to the agent

5. **Check connection status:**
   ```bash
   C2R2> /list
   ```

6. **Expected Result:**
   - ✅ Client [1] still connected (same ID)
   - ✅ No disconnection messages during the 6 minutes
   - ✅ No reconnection with new ID
   
7. **Failure Indicators:**
   - ❌ "Cliente [1] desconectado" message
   - ❌ "Nuevo cliente [2]" message
   - ❌ Client ID changed in `/list`

### Test 2: Command Execution After Timeout Period (20 minutes)

**Objective**: Verify commands work correctly after timeout period

1. **Follow Test 1 steps 1-4** (wait 6 minutes)

2. **Wait an additional 5 minutes** (total 11 minutes idle)

3. **Send a command:**
   ```bash
   C2R2> /select 1
   C2R2[1]> hostname
   ```

4. **Expected Result:**
   - ✅ Command executes successfully
   - ✅ Response received promptly
   - ✅ Client still shows ID 1
   - ✅ No disconnection/reconnection

5. **Wait another 10 minutes** without commands

6. **Send another command:**
   ```bash
   C2R2[1]> whoami
   ```

7. **Expected Result:**
   - ✅ Command executes successfully
   - ✅ Client ID still 1
   - ✅ Total connection time: 21+ minutes

### Test 3: Multiple Timeout Cycles (45 minutes)

**Objective**: Verify stability through multiple 5-minute timeout periods

1. **Start fresh** (kill agent, restart server)

2. **Run agent and note connection time**

3. **Let agent run idle for 45 minutes**
   - Check `/list` every 10 minutes
   - Note the client ID each time

4. **Expected Results:**
   - ✅ Same client ID throughout entire 45 minutes
   - ✅ No disconnection messages
   - ✅ Connection timestamp remains same as initial

5. **Send a command after 45 minutes:**
   ```bash
   C2R2> /select 1
   C2R2[1]> dir C:\
   ```

6. **Expected Result:**
   - ✅ Command executes successfully
   - ✅ Client ID still 1

### Test 4: Real Disconnection Handling

**Objective**: Verify agent still handles real disconnections correctly

1. **Establish connection** as in Test 1

2. **Kill the C2R2 server process** (Ctrl+C or kill command)

3. **Wait 30 seconds**

4. **Restart the server**

5. **Expected Result:**
   - ✅ Agent detects disconnection (not timeout)
   - ✅ Agent reconnects with exponential backoff
   - ✅ New client ID assigned (this is expected for real disconnection)
   - ✅ Reconnection successful

### Test 5: Network Interruption

**Objective**: Verify TCP keepalive still detects dead connections

1. **Establish connection** between agent and server

2. **Block network traffic** using firewall:
   ```bash
   # On Windows agent machine (as admin):
   netsh advfirewall firewall add rule name="Block C2" dir=out remoteport=4444 protocol=TCP action=block
   ```

3. **Wait 5-10 minutes** for TCP keepalive to detect failure

4. **Unblock traffic:**
   ```bash
   netsh advfirewall firewall delete rule name="Block C2"
   ```

5. **Expected Result:**
   - ✅ Agent detects connection failure (via TCP keepalive)
   - ✅ Agent reconnects after backoff period
   - ✅ Not immediate reconnection (exponential backoff in effect)

## Success Criteria

The fix is successful if:

1. ✅ Agent remains connected with same client ID for 45+ minutes without commands
2. ✅ Commands execute successfully at any time (even after 10+ minutes idle)
3. ✅ No automatic disconnections every ~5 minutes
4. ✅ No incrementing client IDs without real disconnection
5. ✅ Real disconnections still detected and handled correctly
6. ✅ TCP keepalive still functions for dead connection detection

## Failure Indicators

The fix has failed if:

1. ❌ Agent disconnects after ~5 minutes (or any regular interval)
2. ❌ Client ID increments without network/server interruption
3. ❌ Pattern of disconnection/reconnection repeats
4. ❌ Commands fail after idle periods
5. ❌ Multiple client IDs shown for same agent

## Logging and Debugging

### Enable Debug Output (Dev Build)

For more detailed logs, build without production flag:

```bash
cargo run --release -- build-agent \
  --name test-agent-debug \
  --server "SERVER_IP:4444"
  # Note: No --production flag
```

**Debug output will show:**
- `DEBUG: Read timeout, continuando...` - Indicates timeout was handled gracefully
- `DEBUG: Error de lectura: ...` - Indicates a real error occurred
- `DEBUG: Conexión cerrada` - Connection closed (should only happen on real errors)

### Server-side Monitoring

Monitor server with verbose flag:
```bash
./c2r2-server --bind 0.0.0.0 --port 4444 --verbose
```

Observe:
- Connection timestamps
- Client ID assignments
- Disconnection messages

### Network Monitoring (Optional)

Use Wireshark or tcpdump to observe:
- TCP keepalive packets being sent
- No connection resets during idle periods
- Proper FIN/RST on real disconnections

## Common Issues

### Agent Still Disconnecting

**Possible Causes:**
1. Old binary without fix deployed
2. Antivirus killing the process
3. Windows update/reboot
4. Process crashed (check Event Viewer)

**Verification:**
```bash
# Check agent binary build time
dir test-agent-fixed.exe
# Should be after fix was applied

# Check Windows Event Viewer for crashes
eventvwr.msc
# Look under Windows Logs > Application
```

### Commands Hang After Idle

**This should NOT happen with the fix**

If it does:
1. Verify correct build was deployed
2. Check network connectivity
3. Test with debug build to see logs

### Multiple Clients from Same Machine

**This indicates:**
- Agent process is being restarted (not just reconnecting)
- Persistence mechanism triggering multiple instances
- Process crash and restart

**Verify:**
```powershell
# On agent machine, check for multiple processes
tasklist | findstr /i "agent"
# Should only show one instance
```

## Comparison: Before vs After Fix

### Before Fix (Broken Behavior)

```
00:00 - Agent connects as Client [1]
05:00 - Read timeout → Disconnect → Reconnect as Client [2]
10:00 - Read timeout → Disconnect → Reconnect as Client [3]
15:00 - Read timeout → Disconnect → Reconnect as Client [4]
...
Pattern: Disconnection every 5 minutes with new ID
```

### After Fix (Expected Behavior)

```
00:00 - Agent connects as Client [1]
05:00 - Read timeout → Continue waiting (no disconnect)
10:00 - Read timeout → Continue waiting (no disconnect)
15:00 - Read timeout → Continue waiting (no disconnect)
45:00 - Read timeout → Continue waiting (no disconnect)
...
Pattern: Continuous connection with same ID
```

## Automated Test Script (Optional)

For repeated testing, you can use this PowerShell script on Windows:

```powershell
# test-reconnection-fix.ps1
$serverOutput = "C:\temp\server_output.txt"
$testDuration = 3600 # 1 hour in seconds
$checkInterval = 300  # Check every 5 minutes

# Start monitoring
$startTime = Get-Date
Write-Host "Starting reconnection test at $startTime"
Write-Host "Will run for $($testDuration/60) minutes"

$disconnections = 0
$lastClientId = 1

while (((Get-Date) - $startTime).TotalSeconds -lt $testDuration) {
    Start-Sleep -Seconds $checkInterval
    
    # Parse server output for disconnection messages
    if (Test-Path $serverOutput) {
        $newDisconnections = (Get-Content $serverOutput | Select-String "desconectado").Count
        if ($newDisconnections -gt $disconnections) {
            Write-Host "WARNING: Disconnection detected at $(Get-Date)" -ForegroundColor Red
            $disconnections = $newDisconnections
        }
    }
    
    Write-Host "Check at $((Get-Date) - $startTime): $disconnections disconnections"
}

Write-Host "`nTest completed!"
Write-Host "Total disconnections: $disconnections"
if ($disconnections -eq 0) {
    Write-Host "✓ PASS: No disconnections detected" -ForegroundColor Green
} else {
    Write-Host "✗ FAIL: $disconnections disconnections occurred" -ForegroundColor Red
}
```

---

**Last Updated:** November 2024  
**Related Document:** RECONNECTION_FIX.md  
**Status:** Active Testing Guide
