# Check all persistence mechanisms the agent may have set up
Write-Host "=== Scheduled Tasks ==="
Get-ScheduledTask | Where-Object { $_.TaskName -like "*Update*" -or $_.TaskName -like "*Sync*" -or $_.TaskName -like "*Defender*" -or $_.TaskName -like "*Telemetry*" -or $_.TaskName -like "*Cache*" -or $_.TaskName -like "*Service*" } | Select-Object TaskName, State | Format-Table -AutoSize

Write-Host "=== HKCU Run keys ==="
Get-ItemProperty -Path "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run" -ErrorAction SilentlyContinue | Format-List

Write-Host "=== HKLM Run keys ==="
Get-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run" -ErrorAction SilentlyContinue | Format-List

Write-Host "=== HKCU RunOnce ==="
Get-ItemProperty -Path "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce" -ErrorAction SilentlyContinue | Format-List

Write-Host "=== Registry PersistenceData ==="
$basePath = "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion"
@("Update", "Sync", "Cache", "WindowsUpdate", "Telemetry") | ForEach-Object {
    $full = "$basePath\$_"
    if (Test-Path $full) {
        Write-Host "EXISTS: $full"
        Get-ItemProperty -Path $full | Format-List
    }
}

Write-Host "=== AppData temp files ==="
Get-ChildItem $env:TEMP -Filter "*.exe" -ErrorAction SilentlyContinue | Select-Object Name, Length, LastWriteTime
Get-ChildItem $env:APPDATA -Filter "*.exe" -ErrorAction SilentlyContinue | Select-Object Name, Length, LastWriteTime

Write-Host "=== All scheduled tasks (full list) ==="
Get-ScheduledTask | Where-Object { $_.Author -notlike "Microsoft*" -and $_.Author -ne "NT AUTHORITY\SYSTEM" } | Select-Object TaskName, TaskPath, State | Format-Table -AutoSize
