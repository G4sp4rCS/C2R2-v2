# Connection Troubleshooting

This guide helps diagnose and fix connection issues between C2R2 agents and the server.

---

## Quick Diagnostic Checklist

- [ ] Server is running with `--bind 0.0.0.0`
- [ ] Firewall allows the port (e.g., `ufw allow 4444/tcp`)
- [ ] Port forwarding configured (for internet access)
- [ ] Agent built with correct IP address
- [ ] Agent built in production mode for deployments
- [ ] Not behind CGNAT (or using tunnel)

---

## Problem: Agent Doesn't Connect

### Symptoms
- Agent runs but server shows no incoming connection
- No new client appears in `/list`

### Diagnostic Steps

**1. Verify server is listening correctly:**

```bash
# On server
sudo netstat -tlnp | grep 4444
# Expected: tcp 0 0 0.0.0.0:4444 0.0.0.0:* LISTEN

# If you see 127.0.0.1:4444 instead of 0.0.0.0:4444, restart with:
./c2r2-server --bind 0.0.0.0 --port 4444
```

**2. Test local connectivity:**

```bash
# From server
nc -zv localhost 4444
# Expected: Connection succeeded
```

**3. Test LAN connectivity:**

```bash
# From another device on same network
nc -zv <server_lan_ip> 4444
```

**4. Test WAN connectivity:**

```bash
# From external network (phone data, VPS, etc.)
nc -zv <public_ip> 4444
```

**5. Verify agent configuration:**

```bash
# Check embedded server address
strings agent.exe | grep -E '\d+\.\d+\.\d+\.\d+:\d+'
```

### Solutions

| Issue | Solution |
|-------|----------|
| Server shows `127.0.0.1:4444` | Restart with `--bind 0.0.0.0` |
| LAN test fails | Check firewall: `sudo ufw allow 4444/tcp` |
| WAN test fails | Configure router port forwarding |
| Wrong IP in agent | Rebuild with correct server address |

---

## Problem: Agent Connects Then Disconnects Immediately

### Symptoms
```
⚡ Nuevo cliente [1] desde IP:PORT
❌ Cliente [1] desconectado
```

### Causes & Solutions

**1. TCP Keepalive Issues**

Home routers close idle connections. Agent beacon should prevent this.

```bash
# Check beacon timing - use shorter intervals
C2R2 [1]> /beacon 30:20
```

**2. NAT Traversal Problems**

Without keepalive packets, NAT mappings expire.

Solution: The agent automatically configures TCP keepalive. If issues persist, use shorter beacon intervals.

**3. Firewall Interference**

Some firewalls inspect and drop suspicious TCP connections.

Solution: Try common ports: `443`, `8443`, `80`, `8080`

---

## Problem: Agent Keeps Reconnecting (ID Increments)

### Symptoms
```
⚡ Nuevo cliente [1] desde IP
❌ Cliente [1] desconectado
⚡ Nuevo cliente [2] desde IP
❌ Cliente [2] desconectado
⚡ Nuevo cliente [3] desde IP
...
```

### Cause

Usually a read timeout configuration issue causing the agent to interpret timeouts as disconnections.

### Solution

This was fixed in recent versions. Update to latest agent code. The agent now properly handles:
- TCP read timeouts as expected behavior
- Connection keep-alive
- Graceful reconnection

---

## Problem: Port Forwarding Not Working

### Verification Steps

**1. Check public IP:**
```bash
curl ifconfig.me
```

**2. Check router WAN IP:**

Compare with the public IP above. If different, you may be behind CGNAT.

**3. Test externally:**

Use online port checker:
- https://www.yougetsignal.com/tools/open-ports/
- https://canyouseeme.org/

**4. Verify router configuration:**

```
Protocol: TCP
External Port: 4444
Internal IP: <Server LAN IP>
Internal Port: 4444
```

### CGNAT Detection

If your router's WAN IP differs from your public IP (from `curl ifconfig.me`), you're behind CGNAT.

**Solutions for CGNAT:**
1. Request public IP from ISP (may cost extra)
2. Use ngrok: `ngrok tcp 4444`
3. Use Cloudflare Tunnel
4. Use VPS as relay server
5. Use Tailscale/WireGuard

---

## Problem: Behind Corporate Firewall

### Symptoms
- Agent can't reach external server
- Only web traffic (80/443) allowed

### Solutions

1. **Use standard ports:**
   ```bash
   ./c2r2-server --bind 0.0.0.0 --port 443
   ```

2. **Future: HTTP/HTTPS beacon** (planned feature)

3. **Use authorized proxy** (if available)

---

## Problem: TLS Certificate Issues

### Symptoms
- Connection fails with certificate errors
- "Certificate verify failed" in logs

### Solutions

**1. Generate fresh certificates:**
```bash
./c2r2-server --generate-certs
```

**2. Verify certificate files exist:**
```bash
ls certs/
# Should show: server.crt, server.key
```

---

## Problem: Agent Works Locally But Not Remotely

### Common Causes

1. **Wrong IP in agent:**
   - Built with LAN IP instead of public IP
   - Rebuild with public IP or domain

2. **Firewall blocking:**
   - Server firewall
   - Router firewall
   - ISP blocking port

3. **Port forwarding incomplete:**
   - Missing rule
   - Wrong internal IP
   - Wrong port

### Verification Sequence

```bash
# 1. Server listening correctly?
sudo netstat -tlnp | grep <port>

# 2. Local firewall open?
sudo ufw status

# 3. Test from LAN
nc -zv <server_lan_ip> <port>

# 4. Port forward configured?
# Check router admin panel

# 5. Test from WAN
nc -zv <public_ip> <port>

# 6. Agent has correct IP?
strings agent.exe | grep <public_ip>
```

---

## Problem: ISP Blocking Ports

### Symptoms
- Port forwarding configured
- Firewall open
- Still can't connect externally

### Common Blocked Ports
- 4444 (msfconsole default, known malware)
- 31337 (Back Orifice)
- 6667 (IRC)

### Solution

Use alternate ports:
- 443 (HTTPS - rarely blocked)
- 8443 (alternate HTTPS)
- 8080 (HTTP proxy)
- 53 (DNS - may work)

```bash
# Server
./c2r2-server --bind 0.0.0.0 --port 8443

# Update port forwarding
# Rebuild agent with new port
```

---

## Network Diagnostic Commands

### Server Side (Linux)

```bash
# Check listening ports
sudo netstat -tlnp | grep c2r2
sudo ss -tlnp | grep c2r2

# Check firewall
sudo ufw status
sudo iptables -L -n

# Test port accessibility
nc -l -p 4444  # Listen on port
# From another machine: nc -zv <ip> 4444

# Check public IP
curl ifconfig.me
```

### Client Side (Windows)

```cmd
REM Check network connectivity
ping <server_ip>

REM Test port
Test-NetConnection -ComputerName <server_ip> -Port 4444

REM Check firewall
netsh advfirewall show currentprofile

REM DNS resolution
nslookup <server_domain>
```

---

## Beacon Timing Issues

### Symptom: Commands Take Too Long

The agent only receives commands during beacon check-ins.

**Solution:** Decrease beacon interval (increases detection risk)
```bash
C2R2 [1]> /beacon 30:20  # 30 seconds ±20%
```

### Symptom: Commands Never Arrive

**Possible causes:**
1. Agent disconnected (check `/list`)
2. Network issue
3. Agent crashed

**Solution:** Wait for reconnection or redeploy agent

---

## Logging for Debugging

### Server Logs

```bash
# Enable verbose logging
RUST_LOG=debug ./c2r2-server --bind 0.0.0.0 --port 4444

# Check logs
tail -f logs/c2r2-session.log
```

### Agent Debug Mode

Build in development mode for console output:
```bash
cargo run -p builder -- build-agent --name debug-agent --server IP:PORT
# (no --production flag)
```

---

## Related Documentation

- [Network Deployment Guide](../guides/NETWORK_DEPLOYMENT.md)
- [Raspberry Pi Setup](../guides/RASPBERRY_PI_SETUP.md)
- [Docker Build](../guides/DOCKER.md)

---

**Still having issues?** Check [GitHub Issues](https://github.com/G4sp4rCS/C2R2-v2/issues) or open a new issue with:
- Server output
- Network configuration
- Steps to reproduce
