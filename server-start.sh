#!/usr/bin/env bash
# server-start.sh — Run this ON THE VPS to start c2r2-server inside tmux
#
# Usage (on the VPS):
#   ./server-start.sh
#   ./server-start.sh --port 4444 --api-port 5555
#   ./server-start.sh --stop
#   ./server-start.sh --attach

set -euo pipefail

SESSION="c2r2"
DIR="$(cd "$(dirname "$0")" && pwd)"
PORT=4444
API_PORT=5555
ACTION="start"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --port)      PORT="$2";     shift 2 ;;
        --api-port)  API_PORT="$2"; shift 2 ;;
        --stop)      ACTION="stop"; shift   ;;
        --attach)    ACTION="attach"; shift ;;
        --status)    ACTION="status"; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

case "$ACTION" in

    stop)
        tmux kill-session -t "$SESSION" 2>/dev/null && echo "Session '$SESSION' killed." || echo "No session '$SESSION' running."
        pkill -f 'c2r2-server' 2>/dev/null && echo "Process killed." || true
        ;;

    attach)
        tmux attach -t "$SESSION"
        ;;

    status)
        if tmux has-session -t "$SESSION" 2>/dev/null; then
            echo "tmux session '$SESSION': RUNNING"
            pgrep -la c2r2-server || echo "  (process not found)"
        else
            echo "tmux session '$SESSION': NOT running"
        fi
        ;;

    start)
        # Kill existing session/process first
        tmux kill-session -t "$SESSION" 2>/dev/null || true
        pkill -f 'c2r2-server' 2>/dev/null || true
        sleep 1

        mkdir -p "$DIR/logs"

        CMD="$DIR/c2r2-server --bind 0.0.0.0 --port $PORT --api-port $API_PORT 2>&1 | tee -a $DIR/logs/server.log"
        tmux new-session -d -s "$SESSION" "$CMD"
        sleep 2

        if tmux has-session -t "$SESSION" 2>/dev/null; then
            echo "OK  session '$SESSION' started"
            echo "    beacon  :${PORT}  api  :${API_PORT}"
            echo ""
            echo "    attach  : tmux attach -t $SESSION"
            echo "    stop    : $0 --stop"
            echo "    logs    : tail -f $DIR/logs/server.log"
        else
            echo "FAIL: tmux session did not start" >&2
            tail -20 "$DIR/logs/server.log" 2>/dev/null || true
            exit 1
        fi
        ;;
esac
