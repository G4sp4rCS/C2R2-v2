#!/bin/bash
set -e
HOST="grunt@192.168.1.107"
PASS="gaspar001"
SSH="sshpass -p $PASS ssh -o StrictHostKeyChecking=no $HOST"

# Kill any previous instance
$SSH "taskkill /F /IM ester_test.exe 2>nul" 2>/dev/null || true
sleep 1

# Launch
$SSH "cmd /c start /b C:\\Users\\grunt\\Desktop\\ester_test.exe"
echo "Launched. Monitoring every second..."

for i in $(seq 1 15); do
  sleep 1
  OUT=$($SSH "tasklist | findstr /i ester_test" 2>/dev/null || true)
  if [ -n "$OUT" ]; then
    PID=$(echo "$OUT" | awk '{print $2}')
    echo "t=${i}s: ✅ ALIVE (PID=$PID)"
  else
    echo "t=${i}s: ❌ DEAD"
  fi
done
