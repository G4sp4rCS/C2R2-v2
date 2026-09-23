$wl = "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon"
Set-ItemProperty -Path $wl -Name "AutoAdminLogon"   -Value "1"       -Force
Set-ItemProperty -Path $wl -Name "DefaultUserName"  -Value "grunt"   -Force
Set-ItemProperty -Path $wl -Name "DefaultPassword"  -Value "gaspar001" -Force
Set-ItemProperty -Path $wl -Name "DefaultDomainName" -Value "."      -Force
Write-Host "AUTO_LOGIN_CONFIGURED"
