try {
    $r = Invoke-WebRequest -Uri "http://192.168.2.7:5555/api/stage0/ester" -UseBasicParsing -TimeoutSec 8
    Write-Host "C2_REACHABLE: $($r.StatusCode) ($($r.RawContentLength) bytes)"
} catch {
    Write-Host "C2_UNREACHABLE: $($_.Exception.Message)"
}

try {
    $r2 = Test-NetConnection -ComputerName 192.168.2.7 -Port 5555 -WarningAction SilentlyContinue
    Write-Host "TCP_5555: $($r2.TcpTestSucceeded)"
} catch {
    Write-Host "TCP_5555: FAIL"
}

try {
    $r3 = Test-NetConnection -ComputerName 192.168.2.7 -Port 4444 -WarningAction SilentlyContinue
    Write-Host "TCP_4444: $($r3.TcpTestSucceeded)"
} catch {
    Write-Host "TCP_4444: FAIL"
}

Write-Host "HOSTNAME: $env:COMPUTERNAME"
Write-Host "USER: $env:USERNAME"
