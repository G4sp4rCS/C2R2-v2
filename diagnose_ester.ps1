taskkill /F /IM ester_test.exe 2>nul
Start-Sleep 1

Start-Process C:\Users\grunt\Desktop\ester_test.exe

Write-Host "t=0s: launched"
Start-Sleep 2

$proc = Get-Process ester_test -ErrorAction SilentlyContinue
if ($proc) { Write-Host "t=2s: ALIVE (PID $($proc.Id))" } else { Write-Host "t=2s: DEAD" }

Start-Sleep 5
$proc = Get-Process ester_test -ErrorAction SilentlyContinue
if ($proc) { Write-Host "t=7s: ALIVE (PID $($proc.Id)) -- JAVELIN should have fired" } else { Write-Host "t=7s: DEAD" }

Start-Sleep 8
$proc = Get-Process ester_test -ErrorAction SilentlyContinue
if ($proc) { Write-Host "t=15s: ALIVE -- fix confirmed!" } else { Write-Host "t=15s: DEAD -- still broken" }

Start-Sleep 15
$proc = Get-Process ester_test -ErrorAction SilentlyContinue
if ($proc) { Write-Host "t=30s: ALIVE -- victory!" } else { Write-Host "t=30s: DEAD" }
