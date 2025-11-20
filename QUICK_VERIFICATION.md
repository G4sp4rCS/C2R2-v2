# Quick Verification: Is the Fix Working?

## 5-Minute Test

### Step 1: Check Current Code (2 minutes)

```bash
cd /home/runner/work/C2R2-v2/C2R2-v2
grep -A 10 "Err(e) =>" agent/src/main.rs | grep -A 8 "Si es timeout"
```

**Expected output:**
```rust
// Si es timeout, simplemente continuar esperando comandos
// Esto previene reconexiones innecesarias cuando no hay actividad
if e.kind() == ErrorKind::TimedOut || e.kind() == ErrorKind::WouldBlock {
    debug_print!("DEBUG: Read timeout, continuando...");
    continue;
}
```

✅ If you see `continue;` → Fix is applied  
❌ If you see `Err(_) => break` → Old code still present

### Step 2: Build and Deploy (15 minutes)

```bash
cd builder
cargo build --release
cargo run --release -- build-agent \
  --name test-reconnection-fix \
  --server "YOUR_IP:4444" \
  --production
```

Deploy `test-reconnection-fix.exe` to test machine.

### Step 3: Quick Connection Test (10 minutes)

1. **Start server:**
   ```bash
   ./c2r2-server --bind 0.0.0.0 --port 4444
   ```

2. **Run agent on test machine**

3. **Note connection time and ID:**
   ```
   ? Nuevo cliente [1] desde IP:PORT at 18:00:00
   ```

4. **Wait 6 minutes** (set timer)

5. **Check status:**
   ```bash
   C2R2> /list
   ```

**Expected (GOOD):**
```
? 1 cliente(s) conectado(s)
? ID ? Dirección ? Conectado ?
? 1  ? IP:PORT   ? 18:00:00  ?  ← Same ID and time
```

**Failure (BAD):**
```
? Nuevo cliente [2] desde IP:PORT at 18:06:00  ← Different ID
```

## What to Look For

### ✅ Success Indicators
- Client ID stays at [1]
- No disconnection messages
- Connection time unchanged after 6+ minutes
- Commands execute successfully at any time

### ❌ Failure Indicators
- Client ID increments ([1] → [2] → [3])
- "Cliente [X] desconectado" messages
- Connection time resets every ~5 minutes
- Pattern repeats

## If Test Fails

1. **Verify correct binary deployed:**
   - Check file timestamp matches recent build
   - Delete old binaries to avoid confusion

2. **Check for multiple processes:**
   ```powershell
   tasklist | findstr /i "agent"
   ```
   Should only show one instance

3. **Review server logs:**
   Look for disconnect/reconnect pattern

4. **Verify code changes:**
   ```bash
   git diff 5b20cd2 agent/src/main.rs
   ```
   Should show ErrorKind handling

## Quick Debugging

### Enable Debug Output

Build without `--production`:
```bash
cargo run --release -- build-agent \
  --name debug-agent \
  --server "IP:4444"
```

Run and watch for:
- `DEBUG: Read timeout, continuando...` → Good, timeout handled
- `DEBUG: Conexión cerrada` → Only on real disconnect

## Success Definition

✅ Agent stays connected as [1] for 10+ minutes  
✅ No reconnection messages  
✅ Commands work after 10+ minutes idle

---

**Time Required:** ~30 minutes total  
**Pass Criteria:** Client ID remains stable for 10+ minutes
