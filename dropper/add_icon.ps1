# Script para añadir icono a agent.exe
# Uso: .\add_icon.ps1 <ruta_al_exe> [icono.ico]

param(
    [Parameter(Mandatory=$true)]
    [string]$ExePath,
    
    [Parameter(Mandatory=$false)]
    [string]$IconPath = ""
)

# Verificar que el exe existe
if (-not (Test-Path $ExePath)) {
    Write-Host "[❌] Error: No se encuentra el archivo: $ExePath" -ForegroundColor Red
    exit 1
}

Write-Host "[🔧] Añadiendo icono a: $ExePath" -ForegroundColor Cyan

# 1. Descargar rcedit si no existe
$rceditPath = Join-Path $PSScriptRoot "rcedit.exe"
if (-not (Test-Path $rceditPath)) {
    Write-Host "[📥] Descargando rcedit.exe..." -ForegroundColor Yellow
    $rceditUrl = "https://github.com/electron/rcedit/releases/download/v2.0.0/rcedit-x64.exe"
    try {
        Invoke-WebRequest -Uri $rceditUrl -OutFile $rceditPath -UseBasicParsing
        Write-Host "[✅] rcedit.exe descargado" -ForegroundColor Green
    } catch {
        Write-Host "[❌] Error descargando rcedit: $_" -ForegroundColor Red
        exit 1
    }
}

# 2. Obtener/descargar icono
if ($IconPath -eq "") {
    # Intentar usar pdf_icon.ico si existe
    $pdfIconPath = Join-Path $PSScriptRoot "pdf_icon.ico"
    if (Test-Path $pdfIconPath) {
        $IconPath = $pdfIconPath
        Write-Host "[🎨] Usando pdf_icon.ico" -ForegroundColor Green
    } else {
        $IconPath = Join-Path $PSScriptRoot "default_icon.ico"
    }
    
    if (-not (Test-Path $IconPath)) {
        Write-Host "[🎨] Creando icono por defecto..." -ForegroundColor Yellow
        
        # Crear icono simple usando .NET
        try {
            Add-Type -AssemblyName System.Drawing
            
            $bitmap = New-Object System.Drawing.Bitmap(32, 32)
            $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
            
            # Fondo rojo PDF
            $redBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(220, 50, 50))
            $graphics.FillRectangle($redBrush, 0, 0, 32, 32)
            
            # Borde blanco
            $whitePen = New-Object System.Drawing.Pen([System.Drawing.Color]::White, 2)
            $graphics.DrawRectangle($whitePen, 2, 2, 28, 28)
            
            # Texto PDF
            $whiteBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
            $font = New-Object System.Drawing.Font("Arial", 7, [System.Drawing.FontStyle]::Bold)
            $graphics.DrawString("PDF", $font, $whiteBrush, 6, 11)
            
            # Guardar como ICO
            $icon = [System.Drawing.Icon]::FromHandle($bitmap.GetHicon())
            $fileStream = [System.IO.File]::Create($IconPath)
            $icon.Save($fileStream)
            $fileStream.Close()
            
            # Limpiar recursos
            $graphics.Dispose()
            $bitmap.Dispose()
            $icon.Dispose()
            $redBrush.Dispose()
            $whiteBrush.Dispose()
            $whitePen.Dispose()
            $font.Dispose()
            
            Write-Host "[✅] Icono creado: default_icon.ico" -ForegroundColor Green
        } catch {
            Write-Host "[❌] Error creando icono: $_" -ForegroundColor Red
            Write-Host "[💡] Descarga un .ico manualmente y úsalo con: .\add_icon.ps1 <exe> <icon.ico>" -ForegroundColor Yellow
            exit 1
        }
    }
} elseif (-not (Test-Path $IconPath)) {
    Write-Host "[❌] Error: No se encuentra el icono: $IconPath" -ForegroundColor Red
    exit 1
}

# 3. Aplicar icono al exe
Write-Host "[🎨] Aplicando icono..." -ForegroundColor Cyan
try {
    & $rceditPath $ExePath --set-icon $IconPath
    if ($LASTEXITCODE -eq 0) {
        Write-Host "[✅] Icono añadido exitosamente a: $ExePath" -ForegroundColor Green
        Write-Host "[📁] Archivo modificado: $(Get-Item $ExePath | Select-Object -ExpandProperty FullName)" -ForegroundColor Gray
    } else {
        Write-Host "[❌] rcedit falló con código: $LASTEXITCODE" -ForegroundColor Red
        exit 1
    }
} catch {
    Write-Host "[❌] Error ejecutando rcedit: $_" -ForegroundColor Red
    exit 1
}

# OPCIÓN 2: Añadir en build.rs del proyecto (permanente)
<#
Crear archivo agent/build.rs con:

#[cfg(windows)]
extern crate winres;

fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon.ico");  // Añadir icon.ico al proyecto
        res.compile().unwrap();
    }
}

Añadir a agent/Cargo.toml:
[build-dependencies]
winres = "0.1"
#>

Write-Host ""
Write-Host "[💡] Tip: Para icono permanente, editar agent/build.rs" -ForegroundColor Cyan
Write-Host "[💡] O usa: .\add_icon.ps1 <exe> <icono.ico>" -ForegroundColor Cyan
