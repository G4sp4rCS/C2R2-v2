# Script para descargar icono PDF real

param(
    [string]$OutputPath = "pdf_icon.ico"
)

Write-Host "[📥] Descargando icono PDF..." -ForegroundColor Cyan

# Lista de URLs de iconos PDF (intentar en orden)
$iconUrls = @(
    "https://www.iconarchive.com/download/i98397/paomedia/small-n-flat/file-pdf.ico",
    "https://icons.iconarchive.com/icons/dtafalonso/android-lollipop/512/PDF-icon.png"
)

$success = $false

foreach ($url in $iconUrls) {
    try {
        Write-Host "[🔄] Intentando: $url" -ForegroundColor Gray
        
        if ($url -like "*.png") {
            # Si es PNG, descargarlo y convertir a ICO con magick o usar como está
            $tempPng = "temp_icon.png"
            Invoke-WebRequest -Uri $url -OutFile $tempPng -UseBasicParsing -TimeoutSec 10
            
            # Intentar usar ImageMagick si está disponible
            if (Get-Command magick -ErrorAction SilentlyContinue) {
                magick convert $tempPng -resize 256x256 $OutputPath
                Remove-Item $tempPng -Force
            } else {
                # Usar PNG directamente (algunos iconos Windows aceptan PNG)
                Write-Host "[⚠️] ImageMagick no encontrado. Descarga .ico directamente" -ForegroundColor Yellow
                continue
            }
        } else {
            # Descargar ICO directamente
            Invoke-WebRequest -Uri $url -OutFile $OutputPath -UseBasicParsing -TimeoutSec 10
        }
        
        if (Test-Path $OutputPath) {
            Write-Host "[✅] Icono descargado: $OutputPath" -ForegroundColor Green
            $success = $true
            break
        }
    } catch {
        Write-Host "[⚠️] Falló: $_" -ForegroundColor Yellow
    }
}

if (-not $success) {
    Write-Host "[❌] No se pudo descargar icono automáticamente" -ForegroundColor Red
    Write-Host "[💡] Descarga manualmente un .ico desde:" -ForegroundColor Cyan
    Write-Host "     https://icons8.com/icons/set/pdf" -ForegroundColor Gray
    Write-Host "     https://www.flaticon.com/free-icons/pdf" -ForegroundColor Gray
    exit 1
}
