#!/usr/bin/env bash
# Wait for VM to come back, then monitor C2 for agent reconnection
echo "Waiting for VM to come back online after reboot..."
for i in $(seq 1 36); do
  sleep 5
  if ssh -o StrictHostKeyChecking=no -o BatchMode=yes -o ConnectTimeout=3 grunt@192.168.1.107 "echo UP" 2>/dev/null | grep -q UP; then
    echo "t=$((i*5))s: VM is BACK ONLINE"
    break
  else
    echo "t=$((i*5))s: VM unreachable (rebooting...)"
  fi
done

echo ""
echo "=== Waiting 60s for HKCU Run key to fire (30s delay + chain) ==="
for i in $(seq 1 12); do
  sleep 5
  AGENTS=$(ssh -o BatchMode=yes root@192.168.2.7 "pgrep -c c2r2-server > /dev/null && tail -5 ~/c2r2/logs/server.log 2>/dev/null" 2>/dev/null)
  echo "t=$((i*5))s (post-login): --- server log ---"
  echo "$AGENTS"
done

echo ""
echo "=== Final C2 log (last 20 lines) ==="
ssh -o BatchMode=yes root@192.168.2.7 "tail -20 ~/c2r2/logs/server.log"
