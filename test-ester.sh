#!/usr/bin/env bash
# Test script: verify ester_test.exe stays alive after JAVELIN fires
set -e
HOST="grunt@192.168.1.107"
PASS="gaspar001"
SSH="sshpass -p $PASS ssh -o StrictHostKeyChecking=no $HOST"
SCP="sshpass -p $PASS scp -o StrictHostKeyChecking=no"

echo "=== [1] Killing any previous ester_test instance ==="
$SSH "taskkill /F /IM ester_test.exe 2>nul || true" || true

echo "=== [2] Launching ester_test.exe ==="
$SSH "cmd /c start /b C:\\Users\\grunt\\Desktop\\ester_test.exe"
echo "    Launched."

echo "=== [3] Checking alive at t=2s (during anti-sandbox sleep) ==="
sleep 2
RESULT=$($SSH "tasklist | findstr /i ester_test" 2>/dev/null || echo "NOT_FOUND")
echo "    $RESULT"

echo "=== [4] Checking alive at t=7s (after env check + delay, JAVELIN about to run) ==="
sleep 5
RESULT=$($SSH "tasklist | findstr /i ester_test" 2>/dev/null || echo "NOT_FOUND")
echo "    $RESULT"

echo "=== [5] Checking alive at t=15s (JAVELIN+stage0 should be running, ESTER looping) ==="
sleep 8
RESULT=$($SSH "tasklist | findstr /i ester_test" 2>/dev/null || echo "NOT_FOUND")
if echo "$RESULT" | grep -qi "ester_test"; then
    echo "    ✅ PROCESS ALIVE — fix confirmed (-x 1 works)"
else
    echo "    ❌ PROCESS DEAD at t=15s — still broken"
fi

echo "=== [6] Checking alive at t=30s ==="
sleep 15
RESULT=$($SSH "tasklist | findstr /i ester_test" 2>/dev/null || echo "NOT_FOUND")
if echo "$RESULT" | grep -qi "ester_test"; then
    echo "    ✅ STILL ALIVE at t=30s"
else
    echo "    ❌ DEAD at t=30s"
fi

echo ""
echo "=== [7] Cleanup — killing process ==="
$SSH "taskkill /F /IM ester_test.exe 2>nul" || true
echo "Done."
