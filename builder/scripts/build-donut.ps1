# Build Donut from source
# This script compiles donut.exe from the source code to avoid AV detection
# during download.

param(
    [string]$OutputPath = "builder\scripts\donut.exe"
)

$ErrorActionPreference = "Stop"

Write-Host "🍩 Building Donut from source..." -ForegroundColor Cyan

# Check for Visual Studio
$vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (-not (Test-Path $vsWhere)) {
    Write-Host "❌ Visual Studio not found. Please install Visual Studio with C++ tools." -ForegroundColor Red
    exit 1
}

$vsPath = & $vsWhere -latest -property installationPath
$vcvarsPath = "$vsPath\VC\Auxiliary\Build\vcvars64.bat"

if (-not (Test-Path $vcvarsPath)) {
    Write-Host "❌ vcvars64.bat not found. Please install Visual Studio C++ build tools." -ForegroundColor Red
    exit 1
}

# Clone donut repository
$tempDir = Join-Path $env:TEMP "donut-build"
if (Test-Path $tempDir) {
    Remove-Item -Recurse -Force $tempDir
}

Write-Host "📥 Cloning donut repository..." -ForegroundColor Yellow
git clone --depth 1 https://github.com/TheWover/donut.git $tempDir

if (-not (Test-Path "$tempDir\Makefile.msvc")) {
    Write-Host "❌ Failed to clone donut repository" -ForegroundColor Red
    exit 1
}

# Build using nmake
Write-Host "🔨 Compiling donut with MSVC..." -ForegroundColor Yellow

$buildScript = @"
call "$vcvarsPath"
cd /d "$tempDir"
nmake -f Makefile.msvc
"@

$buildScript | Out-File -FilePath "$tempDir\build.bat" -Encoding ASCII
& cmd /c "$tempDir\build.bat"

# Check if build succeeded
$builtExe = "$tempDir\donut.exe"
if (Test-Path $builtExe) {
    # Create output directory if needed
    $outputDir = Split-Path $OutputPath -Parent
    if (-not (Test-Path $outputDir)) {
        New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
    }
    
    # Copy to destination
    Copy-Item $builtExe $OutputPath -Force
    Write-Host "✅ Donut compiled successfully: $OutputPath" -ForegroundColor Green
    
    # Cleanup
    Remove-Item -Recurse -Force $tempDir
} else {
    Write-Host "❌ Failed to compile donut" -ForegroundColor Red
    Write-Host "Check build output above for errors" -ForegroundColor Yellow
    exit 1
}
