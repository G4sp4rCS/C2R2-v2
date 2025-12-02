# C2R2-v2 Documentation

Welcome to the C2R2-v2 (Command & Control Rust Reloaded) documentation. This is a modular offensive security framework written in Rust, designed for authorized security testing and educational purposes.

## ⚠️ Legal Disclaimer

**FOR EDUCATIONAL AND AUTHORIZED SECURITY TESTING PURPOSES ONLY**

This tool is provided for security researchers, penetration testers, and educational purposes. Any unauthorized use of this software to compromise systems you do not own or have explicit permission to test is illegal and unethical.

The authors and contributors assume no liability for misuse or damages caused by this software. By using C2R2-v2, you agree to use it only on systems you own or have written authorization to test.

---

## 📚 Documentation Index

### Getting Started
| Document | Description |
|----------|-------------|
| [Installation Guide](INSTALLATION.md) | Prerequisites, building, and deployment |
| [Usage Guide](USAGE.md) | Command reference and operational workflows |
| [Quick Start with Docker](guides/DOCKER.md) | Build everything with Docker in minutes |

### Architecture & Design
| Document | Description |
|----------|-------------|
| [System Architecture](ARCHITECTURE.md) | Component design, data flow, and protocols |
| [Modules Documentation](MODULES.md) | Module system and stealer capabilities |
| [API Reference](API.md) | Developer API for extending C2R2 |

### Deployment Guides
| Document | Description |
|----------|-------------|
| [Docker Build](guides/DOCKER.md) | Docker-based compilation |
| [Network Deployment](guides/NETWORK_DEPLOYMENT.md) | LAN, WAN, and port forwarding setup |
| [Raspberry Pi Setup](guides/RASPBERRY_PI_SETUP.md) | Complete Pi deployment guide |

### Features
| Document | Description |
|----------|-------------|
| [Evasion Techniques](features/EVASION.md) | Anti-sandbox, anti-VM, and AV bypass |
| [Persistence Mechanisms](features/PERSISTENCE.md) | Registry, scheduled tasks, WMI, startup |
| [Credential Stealer](features/STEALER.md) | Browser, Discord, Telegram, wallet harvesting |
| [Ransomware Module](features/RANSOMWARE.md) | File encryption capabilities |
| [Dropper System](features/DROPPER.md) | Social engineering and payload delivery |
| [Privilege Escalation](features/ELEVATE.md) | UAC bypass and elevation techniques |

### Development
| Document | Description |
|----------|-------------|
| [Development Guide](DEVELOPMENT.md) | Project structure, building, and debugging |
| [Contributing Guidelines](CONTRIBUTING.md) | How to contribute to the project |
| [Security Considerations](SECURITY.md) | OPSEC, threat model, and best practices |

### Troubleshooting
| Document | Description |
|----------|-------------|
| [Connection Issues](troubleshooting/CONNECTION.md) | Agent connection and network problems |
| [Build Issues](troubleshooting/BUILD.md) | Compilation and cross-compilation fixes |
| [Common Problems (ES)](troubleshooting/SOLUCION_PROBLEMAS_ES.md) | Spanish troubleshooting guide |

### Component Documentation
| Component | README |
|-----------|--------|
| Builder | [builder/README.md](../builder/README.md) |
| Team Client | [team-client/README.md](../team-client/README.md) |
| Stealer DLL | *See [Modules](MODULES.md)* |
| Ransomware DLL | [ransomware-dll/README.md](../ransomware-dll/README.md) |
| Dropper (Rust) | [dropper-rust/README.md](../dropper-rust/README.md) |

---

## 🎯 What is C2R2-v2?

C2R2-v2 is a modular offensive security suite written in Rust, inspired by frameworks like Havoc C2 and Cobalt Strike. It provides:

- **Lightweight Agent** (~60KB) with beacon communication
- **Modular Architecture** - Load capabilities on-demand via encrypted modules
- **Cross-Platform Builder** - Build Windows agents from Linux/WSL
- **Team Client** - GUI for multi-operator deployments via SSH tunnel
- **Advanced Evasion** - Direct syscalls, string obfuscation, anti-analysis
- **TLS 1.3 Encryption** - Secure agent-server communications

## 🏗️ Architecture Overview

```
┌─────────────────────┐                    ┌─────────────────────┐
│  Operator Machine   │      SSH (22)      │   Red Team Server   │
│  ┌───────────────┐  │  ════════════════> │  ┌───────────────┐  │
│  │ Team Client   │──┼────────────────────┼──│  C2 Server    │  │
│  │   (GUI)       │  │                    │  │  API:5555     │  │
│  └───────────────┘  │                    │  │  Agents:4444  │  │
└─────────────────────┘                    │  └───────┬───────┘  │
                                           │   🔐 TLS Encrypted  │
                                           │          ▼          │
                                           │  ┌───────────────┐  │
                                           │  │    Agent      │──┼─► Dynamic Module Loading
                                           │  │  (agent.exe)  │  │
                                           │  └───────────────┘  │
                                           └─────────────────────┘
```

### Components

| Component | Description |
|-----------|-------------|
| **Agent** | Lightweight Windows implant with beacon communication |
| **C2 Server** | Async multi-client server with CLI and REST/WebSocket API |
| **Builder** | Agent generation and module encryption tool |
| **Team Client** | Python GUI for operators (SSH-tunneled API access) |
| **Stealer DLL** | Modular credential harvesting |
| **Ransomware DLL** | File encryption module |

## 🚀 Quick Start

### Docker (Recommended)

```bash
./docker-build.sh --ip 192.168.1.10 --port 4444 --production
# All binaries in dist/
```

### Manual Build

```bash
# 1. Build stealer module
./build-stealer.sh

# 2. Encrypt module and build agent
cd builder
cargo run --release -- encrypt-module
cargo run --release -- build-agent --name agent --server 192.168.1.10:4444 --production

# 3. Start server
cd ../c2r2-server
./target/release/c2r2-server --bind 0.0.0.0 --port 4444
```

See [Installation Guide](INSTALLATION.md) for detailed instructions.

## 📋 Available Commands

```
📋 Client Management:
   /list                      - List connected clients
   /select <id>               - Select a client
   /info <id>                 - Show client details

💻 Command Execution:
   /cmd <command>             - Execute on selected client
   /cmd_all <command>         - Execute on ALL clients

📁 File Operations:
   /download <path>           - Download from agent
   /upload <local> <remote>   - Upload to agent

🔧 Advanced Operations:
   /harvest                   - Harvest credentials
   /elevate                   - UAC elevation (prompt bombing)
   /persist <method>          - Establish persistence
   /beacon <int:jit>          - Configure beacon timing
```

## 🔒 Security Features

| Feature | Description |
|---------|-------------|
| Direct Syscalls | Bypass userland API hooks (EDR evasion) |
| String Obfuscation | Compile-time encryption with `obfstr` |
| Module Encryption | AES-256-GCM encrypted capability modules |
| Beacon with Jitter | Randomized check-in timing |
| Anti-Analysis | VM, sandbox, and debugger detection |
| TLS 1.3 | Encrypted agent-server communications |

## 📝 Development Status

**Current Version: 2.0.0**

### ✅ Implemented
- TLS encrypted beacon communication
- Multi-client server with CLI and API
- Command execution with obfuscation
- File transfer (upload/download)
- Credential harvesting (browsers, Discord, Telegram, wallets)
- Multiple persistence mechanisms
- Docker build system
- Team Client GUI

### 🔮 Planned
- HTTP/HTTPS/DNS C2 channels
- Process injection module
- Lateral movement capabilities
- Web-based C2 interface

---

## 🤝 Contributing

See [Contributing Guidelines](CONTRIBUTING.md) for how to contribute.

## 📄 License

MIT License - See [LICENSE](../LICENSE) for details.

---

**⚠️ Remember: With great power comes great responsibility. Use this tool ethically and legally.**
