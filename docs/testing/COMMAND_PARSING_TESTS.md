# Command Parsing Test Cases

This file demonstrates the command parsing fix and provides test cases for validation.

## Test Scenarios

### 1. Paths with spaces using double quotes
```
Input:  /cmd dir "C:\Program Files"
Parsed: ["/cmd", "dir", "C:\Program Files"]
Sent:   dir "C:\Program Files"
Result:  Path preserved correctly
```

### 2. Paths with spaces using single quotes
```
Input:  /cmd dir 'C:\Program Files (x86)'
Parsed: ["/cmd", "dir", "C:\Program Files (x86)"]
Sent:   dir "C:\Program Files (x86)"
Result:  Path preserved correctly
```

### 3. Relative paths with spaces
```
Input:  /cmd dir "..\..\Program Files (x86)"
Parsed: ["/cmd", "dir", "..\..\Program Files (x86)"]
Sent:   dir "..\..\Program Files (x86)"
Result:  Relative path preserved correctly
```

### 4. Path without quotes (no spaces)
```
Input:  /cmd dir C:\Windows
Parsed: ["/cmd", "dir", "C:\Windows"]
Sent:   dir C:\Windows
Result:  No quotes added unnecessarily
```

### 5. Multiple arguments with spaces
```
Input:  /cmd copy "C:\My Documents\file.txt" "D:\Backup\files\"
Parsed: ["/cmd", "copy", "C:\My Documents\file.txt", "D:\Backup\files\"]
Sent:   copy "C:\My Documents\file.txt" "D:\Backup\files\"
Result:  Both paths preserved correctly
```

### 6. Mixed quoted and unquoted arguments
```
Input:  /cmd type "C:\Users\User Name\file.txt"
Parsed: ["/cmd", "type", "C:\Users\User Name\file.txt"]
Sent:   type "C:\Users\User Name\file.txt"
Result:  Mixed arguments handled correctly
```

### 7. Upload command with spaces
```
Input:  /upload "C:\local file.txt" "C:\remote path\file.txt"
Parsed: ["/upload", "C:\local file.txt", "C:\remote path\file.txt"]
Sent:   __UPLOAD__|C:\local file.txt|C:\remote path\file.txt
Result:  Upload paths preserved correctly
```

### 8. Download command with spaces
```
Input:  /download "C:\Remote Files\document.pdf"
Parsed: ["/download", "C:\Remote Files\document.pdf"]
Sent:   __DOWNLOAD__:C:\Remote Files\document.pdf
Result:  Download path preserved correctly
```

## Edge Cases

### 9. Empty quotes
```
Input:  /cmd echo "" test
Parsed: ["/cmd", "echo", "", "test"]
Sent:   echo "" test
Result:  Empty string preserved
```

### 10. Backslashes at end (Windows paths)
```
Input:  /cmd dir "C:\Program Files\"
Parsed: ["/cmd", "dir", "C:\Program Files\"]
Sent:   dir "C:\Program Files\"
Result:  Trailing backslash preserved
```

### 11. Mixed quote types in same command
```
Input:  /cmd dir 'C:\Users' "C:\Program Files"
Parsed: ["/cmd", "dir", "C:\Users", "C:\Program Files"]
Sent:   dir C:\Users "C:\Program Files"
Result:  Different quote types handled correctly
```

## Agent Obfuscation Handling

After the command is sent to the agent, the argfuscator processes it:

### Example: dir "C:\Program Files"

1. **Received by agent:** `dir "C:\Program Files"`
2. **Parsed by argfuscator:** `["dir", "C:\Program Files"]`
3. **Obfuscation applied:**
   - Random case: `DiR`
   - Caret insertion: `D^i^R`
   - Path preserved: `C:\Program Files` (no modification)
4. **Reconstructed:** `D^i^R "C:\Program Files"`
5. **Executed:** `cmd /C D^i^R "C:\Program Files"`
6. **Result:**  Command executes correctly

## Before vs After

### Before the fix:
```
Input:  /cmd dir "C:\Program Files"
Parsed: ["/cmd", "dir", "\"C:\Program", "Files\""]   BROKEN
Sent:   dir "C:\Program Files"
Agent:  ["dir", "\"C:\Program", "Files\""]           BROKEN
Output: Error - invalid path
```

### After the fix:
```
Input:  /cmd dir "C:\Program Files"
Parsed: ["/cmd", "dir", "C:\Program Files"]          CORRECT
Sent:   dir "C:\Program Files"
Agent:  ["dir", "C:\Program Files"]                  CORRECT
Output: Directory listing successful
```

## Validation

To validate these changes work correctly:

1. Start C2R2 server
2. Connect an agent from a Windows machine
3. Run each test case above
4. Verify commands execute correctly
5. Check that paths with spaces are handled properly

## Security Considerations

- Quote handling does not introduce command injection vulnerabilities
- Paths are properly escaped when reconstructed
- Obfuscation still applies to all commands
- No user input is executed without going through cmd.exe with /C flag
