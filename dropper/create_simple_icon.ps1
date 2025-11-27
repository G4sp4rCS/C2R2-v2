Add-Type -AssemblyName System.Drawing

# Crear un icono simple de 32x32 con texto "PDF"
$bitmap = New-Object System.Drawing.Bitmap(32, 32)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)

# Fondo rojo
$redBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(220, 50, 50))
$graphics.FillRectangle($redBrush, 0, 0, 32, 32)

# Texto blanco "PDF"
$whiteBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
$font = New-Object System.Drawing.Font("Arial", 8, [System.Drawing.FontStyle]::Bold)
$graphics.DrawString("PDF", $font, $whiteBrush, 4, 11)

# Guardar como ICO
$iconPath = Join-Path $PSScriptRoot "simple_pdf.ico"
$icon = [System.Drawing.Icon]::FromHandle($bitmap.GetHicon())
$fileStream = [System.IO.File]::Create($iconPath)
$icon.Save($fileStream)
$fileStream.Close()

# Limpiar
$graphics.Dispose()
$bitmap.Dispose()
$icon.Dispose()
$redBrush.Dispose()
$whiteBrush.Dispose()
$font.Dispose()

Write-Host "[✅] Icono creado: $iconPath" -ForegroundColor Green
return $iconPath
