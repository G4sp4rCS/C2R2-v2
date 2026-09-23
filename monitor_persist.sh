#!/usr/bin/env bash
# Monitor VM reboot + wait for persistence to fire + check C2 reconnection
echo "=== Reboot 2 (auto-login configured) ==="
echo "Waiting for VM to come back..."

ONLINE=false
for i in $(seq 1 40); do
  sleep 5
  if ssh -o StrictHostKeyChecking=no -o BatchMode=yes -o ConnectTimeout=3 grunt@192.168.1.107 "echo UP" 2>/dev/null | grep -q UP; then
    echo "t=$((i*5))s: VM ONLINE"
    ONLINE=true
    break
  else
    echo "t=$((i*5))s: still rebooting..."
  fi
done

if [ "$ONLINE" = false ]; then
  echo "VM didn't come back in time!"
  exit 1
fi

echo ""
echo "=== Waiting for auto-login + Run key (ping -n 31 = 30s delay) ==="
echo "Expected: ~35-45s after login until ester fires"

for i in $(seq 1 20); do
  sleep 5

  # Check VM processes
  PROCS=$(ssh -o StrictHostKeyChecking=no -o BatchMode=yes -o ConnectTimeout=3 grunt@192.168.1.107 \
    "tasklist | findstr /i 'MsEdge curl conhost ester'" 2>/dev/null)

  # Check C2 alive (HTTP)
  HTTP=$(curl -s -o /dev/null -w "%{http_code}" http://192.168.2.7:5555/api/stage0/ester --connect-timeout 3 2>/dev/null)

  echo "t=$((i*5))s | C2-HTTP:${HTTP} | VM procs:"
  if [ -n "$PROCS" ]; then
    echo "  $PROCS"
  else
    echo "  (none)"
  fi
done

echo ""
echo "=== Final check: C2 server log ==="
ssh -o StrictHostKeyChecking=no -o BatchMode=yes root@192.168.2.7 "tail -30 ~/c2r2/logs/server.log" 2>/dev/null || echo "VPS SSH unavailable - checking HTTP only"
curl -s "http://192.168.2.7:5555/api/agents" 2>/dev/null || true
