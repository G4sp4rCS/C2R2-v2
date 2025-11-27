# Script para convertir PNG/imagen a ICO válido para Windows

param(
    [string]$InputImage = "pdf_temp.png",
    [string]$OutputIco = "pdf_icon.ico"
)

Add-Type -AssemblyName System.Drawing

Write-Host "[🔄] Convirtiendo $InputImage a $OutputIco..." -ForegroundColor Cyan

try {
    # Cargar imagen
    if (-not (Test-Path $InputImage)) {
        Write-Host "[📥] Descargando icono PDF desde Wikipedia..." -ForegroundColor Yellow
        $url = "https://upload.wikimedia.org/wikipedia/commons/thumb/8/87/PDF_file_icon.svg/256px-PDF_file_icon.svg.png"
        Invoke-WebRequest -Uri $url -OutFile $InputImage -UseBasicParsing -TimeoutSec 30
    }
    
    $sourceImage = [System.Drawing.Bitmap]::FromFile((Resolve-Path $InputImage).Path)
    
    # Crear diferentes tamaños para el ICO (formato estándar)
    $sizes = @(16, 32, 48, 256)
    
    # Crear archivo ICO manualmente con estructura correcta
    $memoryStream = New-Object System.IO.MemoryStream
    $binaryWriter = New-Object System.IO.BinaryWriter($memoryStream)
    
    # ICONDIR header
    $binaryWriter.Write([uint16]0)  # Reserved, must be 0
    $binaryWriter.Write([uint16]1)  # Type: 1 = ICO
    $binaryWriter.Write([uint16]$sizes.Count)  # Number of images
    
    # Preparar imágenes y calcular offsets
    $imageDataList = @()
    $currentOffset = 6 + ($sizes.Count * 16)  # Header + ICONDIRENTRY array
    
    foreach ($size in $sizes) {
        # Crear bitmap del tamaño requerido
        $bitmap = New-Object System.Drawing.Bitmap($size, $size)
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
        $graphics.DrawImage($sourceImage, 0, 0, $size, $size)
        
        # Convertir a PNG en memoria
        $pngStream = New-Object System.IO.MemoryStream
        $bitmap.Save($pngStream, [System.Drawing.Imaging.ImageFormat]::Png)
        $pngData = $pngStream.ToArray()
        $pngStream.Dispose()
        
        # ICONDIRENTRY
        $binaryWriter.Write([byte]($size -eq 256 ? 0 : $size))  # Width (0 = 256)
        $binaryWriter.Write([byte]($size -eq 256 ? 0 : $size))  # Height (0 = 256)
        $binaryWriter.Write([byte]0)   # Color palette
        $binaryWriter.Write([byte]0)   # Reserved
        $binaryWriter.Write([uint16]1) # Color planes
        $binaryWriter.Write([uint16]32) # Bits per pixel
        $binaryWriter.Write([uint32]$pngData.Length) # Image size
        $binaryWriter.Write([uint32]$currentOffset)  # Image offset
        
        $imageDataList += $pngData
        $currentOffset += $pngData.Length
        
        $graphics.Dispose()
        $bitmap.Dispose()
    }
    
    # Escribir datos de imágenes
    foreach ($imageData in $imageDataList) {
        $binaryWriter.Write($imageData)
    }
    
    # Guardar archivo ICO
    $icoData = $memoryStream.ToArray()
    [System.IO.File]::WriteAllBytes((Resolve-Path $OutputIco -ErrorAction SilentlyContinue).Path ?? (Join-Path (Get-Location) $OutputIco), $icoData)
    
    $binaryWriter.Dispose()
    $memoryStream.Dispose()
    $sourceImage.Dispose()
    
    Write-Host "[✅] Icono creado exitosamente: $OutputIco" -ForegroundColor Green
    
    # Verificar tamaño
    $iconFile = Get-Item $OutputIco
    Write-Host "[📊] Tamaño: $($iconFile.Length) bytes" -ForegroundColor Gray
    Write-Host "[📊] Tamaños incluidos: $($sizes -join ', ') px" -ForegroundColor Gray
    
} catch {
    Write-Host "[❌] Error: $_" -ForegroundColor Red
    Write-Host "[🔍] Stack trace: $($_.Exception.StackTrace)" -ForegroundColor DarkGray
    exit 1
}
