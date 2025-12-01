# Uso del Builder para Configurar Agentes

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

### 2. `build-agent` - Compilar desde Código Fuente

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

## 🎯 ¿Cuál Usar?

### Usa `patch-agent` si:
- Descargaste un release de GitHub
- No tienes Rust instalado
- Solo necesitas cambiar la IP/Puerto
- Eres un usuario final, no desarrollador
- Quieres configurar agentes rápidamente

### Usa `build-agent` si:
- Tienes el código fuente completo
- Tienes Rust + MinGW instalados
- Necesitas cambiar modo dev/prod
- Eres un desarrollador del proyecto
- Quieres modificar features del agente

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

### Escenario 2: Desarrollador con código fuente

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

## 📚 Referencias

- **GitHub Releases:** https://github.com/G4sp4rCS/C2R2-v2/releases
- **Documentación Completa:** Ver README.md principal
- **Soporte:** Abrir issue en GitHub
