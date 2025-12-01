#!/usr/bin/env python3
"""
Crear .lnk que ejecuta health-check.bat correctamente
"""

import os
import sys
from pathlib import Path

try:
    import win32com.client
except ImportError:
    print("[❌] Instalar: pip install pywin32")
    sys.exit(1)

def create_lnk(bat_path, output_lnk, icon_path=None):
    """Crear .lnk que ejecuta el BAT"""
    
    bat_path = str(Path(bat_path).absolute())
    output_lnk = str(Path(output_lnk).absolute())
    working_dir = str(Path(bat_path).parent)
    
    print(f"[🔗] Creando acceso directo...")
    print(f"    BAT: {bat_path}")
    print(f"    LNK: {output_lnk}")
    print(f"    Dir: {working_dir}")
    
    shell = win32com.client.Dispatch("WScript.Shell")
    shortcut = shell.CreateShortcut(output_lnk)
    
    # Target: cmd.exe con /c (ejecuta y cierra)
    shortcut.TargetPath = r"C:\Windows\System32\cmd.exe"
    
    # Arguments: /c ejecuta el comando y cierra la ventana
    # start /b = ejecuta en background sin abrir nueva ventana
    bat_name = Path(bat_path).name
    shortcut.Arguments = f'/c "start /b {bat_name}"'
    
    # Working directory: donde está el BAT (CRÍTICO para encontrar PDF)
    shortcut.WorkingDirectory = working_dir
    
    # Ventana normal (no minimizada, porque start /b ya lo oculta)
    shortcut.WindowStyle = 1  # Normal
    
    # Icono
    if icon_path and os.path.exists(icon_path):
        shortcut.IconLocation = f"{os.path.abspath(icon_path)},0"
        print(f"    Icono: {icon_path}")
    
    shortcut.Save()
    
    print(f"[✅] LNK creado exitosamente")
    print(f"\n[🧪] Para probar:")
    print(f"    1. Doble click en {Path(output_lnk).name}")
    print(f"    2. Revisar: notepad %TEMP%\\bat_debug.txt")
    print(f"    3. Verificar que se abrió el PDF")

if __name__ == '__main__':
    import argparse
    
    parser = argparse.ArgumentParser(description='Crear .lnk para ejecutar BAT')
    parser.add_argument('--bat', required=True, help='Ruta al health-check.bat')
    parser.add_argument('--output', required=True, help='Nombre del .lnk de salida')
    parser.add_argument('--icon', help='Ruta al icono .ico')
    
    args = parser.parse_args()
    
    if not os.path.exists(args.bat):
        print(f"[❌] No se encuentra: {args.bat}")
        sys.exit(1)
    
    create_lnk(args.bat, args.output, args.icon)
