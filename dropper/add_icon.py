#!/usr/bin/env python3
"""
add_icon.py - Herramienta para añadir iconos a ejecutables Windows

Utilidad para modificar el icono de archivos .exe de forma automática.
Descarga rcedit si no existe y soporta múltiples formatos de imagen.
"""

import argparse
import os
import sys
import subprocess
import requests
from pathlib import Path
from PIL import Image
import io


class IconManager:
    """Gestor de iconos para ejecutables Windows"""
    
    def __init__(self, verbose=False):
        self.verbose = verbose
        self.script_dir = Path(__file__).parent
        self.rcedit_path = self.script_dir / "rcedit.exe"
        
    def log(self, message, level="info"):
        """Logging con colores"""
        colors = {
            "info": "\033[36m",      # Cyan
            "success": "\033[92m",   # Green
            "warning": "\033[93m",   # Yellow
            "error": "\033[91m",     # Red
            "reset": "\033[0m"
        }
        
        icons = {
            "info": "🔧",
            "success": "✅",
            "warning": "⚠️",
            "error": "❌"
        }
        
        if level in colors:
            print(f"{icons.get(level, '')} {colors[level]}{message}{colors['reset']}")
        else:
            print(message)
    
    def ensure_rcedit(self):
        """Descargar rcedit.exe si no existe"""
        if self.rcedit_path.exists():
            if self.verbose:
                self.log(f"rcedit.exe encontrado: {self.rcedit_path}", "info")
            return True
        
        self.log("Descargando rcedit.exe...", "warning")
        rcedit_url = "https://github.com/electron/rcedit/releases/download/v2.0.0/rcedit-x64.exe"
        
        try:
            response = requests.get(rcedit_url, timeout=30)
            response.raise_for_status()
            
            with open(self.rcedit_path, 'wb') as f:
                f.write(response.content)
            
            self.log(f"rcedit.exe descargado: {self.rcedit_path}", "success")
            return True
            
        except Exception as e:
            self.log(f"Error descargando rcedit: {e}", "error")
            return False
    
    def download_default_icon(self, output_path):
        """Descargar icono PDF por defecto desde Wikipedia"""
        self.log("Descargando icono PDF desde Wikipedia...", "info")
        
        icon_url = "https://upload.wikimedia.org/wikipedia/commons/thumb/8/87/PDF_file_icon.svg/256px-PDF_file_icon.svg.png"
        
        try:
            response = requests.get(icon_url, timeout=30)
            response.raise_for_status()
            
            # Guardar PNG temporal
            temp_png = self.script_dir / "temp_icon.png"
            with open(temp_png, 'wb') as f:
                f.write(response.content)
            
            # Convertir a ICO
            self.convert_to_ico(temp_png, output_path)
            
            # Limpiar temporal
            temp_png.unlink(missing_ok=True)
            
            self.log(f"Icono descargado y convertido: {output_path}", "success")
            return True
            
        except Exception as e:
            self.log(f"Error descargando icono: {e}", "error")
            return False
    
    def convert_to_ico(self, input_image, output_ico, sizes=None):
        """Convertir imagen (PNG/JPG/BMP) a ICO válido"""
        if sizes is None:
            sizes = [16, 32, 48, 256]
        
        if self.verbose:
            self.log(f"Convirtiendo {input_image} a {output_ico}...", "info")
        
        try:
            # Abrir imagen fuente
            source_img = Image.open(input_image)
            
            # Convertir a RGBA si es necesario
            if source_img.mode != 'RGBA':
                source_img = source_img.convert('RGBA')
            
            # Crear lista de imágenes en diferentes tamaños
            icon_images = []
            for size in sizes:
                resized = source_img.resize((size, size), Image.Resampling.LANCZOS)
                icon_images.append(resized)
            
            # Guardar como ICO
            icon_images[0].save(
                output_ico,
                format='ICO',
                sizes=[(img.width, img.height) for img in icon_images],
                append_images=icon_images[1:]
            )
            
            if self.verbose:
                self.log(f"ICO creado con tamaños: {sizes}", "success")
            
            return True
            
        except Exception as e:
            self.log(f"Error convirtiendo imagen: {e}", "error")
            return False
    
    def set_icon(self, exe_path, icon_path):
        """Aplicar icono a ejecutable usando rcedit"""
        if not Path(exe_path).exists():
            self.log(f"Ejecutable no encontrado: {exe_path}", "error")
            return False
        
        if not Path(icon_path).exists():
            self.log(f"Icono no encontrado: {icon_path}", "error")
            return False
        
        self.log(f"Aplicando icono a: {exe_path}", "info")
        
        try:
            cmd = [str(self.rcedit_path), str(exe_path), "--set-icon", str(icon_path)]
            
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                check=False
            )
            
            if result.returncode == 0:
                self.log(f"Icono aplicado exitosamente: {exe_path}", "success")
                return True
            else:
                self.log(f"rcedit falló: {result.stderr}", "error")
                return False
                
        except Exception as e:
            self.log(f"Error ejecutando rcedit: {e}", "error")
            return False
    
    def get_icon_info(self, exe_path):
        """Obtener información del icono actual de un ejecutable"""
        if not Path(exe_path).exists():
            self.log(f"Ejecutable no encontrado: {exe_path}", "error")
            return None
        
        try:
            # rcedit no tiene opción para extraer, pero podemos verificar con --list-resources
            cmd = [str(self.rcedit_path), str(exe_path), "--list-resources"]
            
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                check=False
            )
            
            if result.returncode == 0:
                print(f"\n📋 Recursos de {exe_path}:")
                print(result.stdout)
                return True
            else:
                self.log("No se pudo obtener información de recursos", "warning")
                return False
                
        except Exception as e:
            self.log(f"Error obteniendo info: {e}", "error")
            return False


def main():
    parser = argparse.ArgumentParser(
        description="🎨 Herramienta para añadir iconos a ejecutables Windows",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Ejemplos de uso:
  # Usar icono por defecto (PDF)
  python add_icon.py agent.exe
  
  # Usar icono personalizado
  python add_icon.py agent.exe --icon custom.ico
  
  # Convertir PNG/JPG a ICO
  python add_icon.py agent.exe --icon image.png --convert
  
  # Ver información de recursos
  python add_icon.py agent.exe --info
  
  # Modo verboso
  python add_icon.py agent.exe -v
        """
    )
    
    parser.add_argument(
        'exe',
        help='Ruta al ejecutable .exe'
    )
    
    parser.add_argument(
        '-i', '--icon',
        help='Ruta al archivo de icono (.ico, .png, .jpg). Si no se especifica, usa icono PDF por defecto'
    )
    
    parser.add_argument(
        '-c', '--convert',
        action='store_true',
        help='Convertir imagen a formato ICO antes de aplicar'
    )
    
    parser.add_argument(
        '--sizes',
        nargs='+',
        type=int,
        default=[16, 32, 48, 256],
        help='Tamaños de icono a generar (default: 16 32 48 256)'
    )
    
    parser.add_argument(
        '--download',
        choices=['pdf', 'word', 'excel', 'zip', 'txt'],
        help='Descargar icono predefinido (pdf, word, excel, zip, txt)'
    )
    
    parser.add_argument(
        '--info',
        action='store_true',
        help='Mostrar información de recursos del ejecutable'
    )
    
    parser.add_argument(
        '-v', '--verbose',
        action='store_true',
        help='Modo verboso'
    )
    
    parser.add_argument(
        '--version',
        action='version',
        version='%(prog)s 1.0.0'
    )
    
    args = parser.parse_args()
    
    # Crear manager
    manager = IconManager(verbose=args.verbose)
    
    # Verificar/descargar rcedit
    if not manager.ensure_rcedit():
        return 1
    
    # Modo info
    if args.info:
        manager.get_icon_info(args.exe)
        return 0
    
    # Determinar icono a usar
    icon_path = None
    
    if args.download:
        # Descargar icono predefinido
        icon_urls = {
            'pdf': 'https://upload.wikimedia.org/wikipedia/commons/thumb/8/87/PDF_file_icon.svg/256px-PDF_file_icon.svg.png',
            'word': 'https://upload.wikimedia.org/wikipedia/commons/thumb/f/fd/Microsoft_Word_logo_%282019%E2%80%93present%29.svg/256px-Microsoft_Word_logo_%282019%E2%80%93present%29.svg.png',
            'excel': 'https://upload.wikimedia.org/wikipedia/commons/thumb/7/73/Microsoft_Excel_2013-2019_logo.svg/256px-Microsoft_Excel_2013-2019_logo.svg.png',
            'zip': 'https://upload.wikimedia.org/wikipedia/commons/thumb/6/69/Folder_zip_icon.png/256px-Folder_zip_icon.png',
            'txt': 'https://upload.wikimedia.org/wikipedia/commons/thumb/4/48/Text-txt.svg/256px-Text-txt.svg.png'
        }
        
        icon_path = manager.script_dir / f"{args.download}_icon.ico"
        
        if not icon_path.exists():
            manager.log(f"Descargando icono {args.download}...", "info")
            
            try:
                response = requests.get(icon_urls[args.download], timeout=30)
                response.raise_for_status()
                
                temp_png = manager.script_dir / "temp_download.png"
                with open(temp_png, 'wb') as f:
                    f.write(response.content)
                
                manager.convert_to_ico(temp_png, icon_path, sizes=args.sizes)
                temp_png.unlink(missing_ok=True)
                
            except Exception as e:
                manager.log(f"Error descargando icono: {e}", "error")
                return 1
    
    elif args.icon:
        icon_path = Path(args.icon)
        
        # Si necesita conversión o no es ICO
        if args.convert or icon_path.suffix.lower() != '.ico':
            converted_ico = manager.script_dir / f"{icon_path.stem}.ico"
            
            if manager.convert_to_ico(icon_path, converted_ico, sizes=args.sizes):
                icon_path = converted_ico
            else:
                return 1
    
    else:
        # Usar icono PDF por defecto
        icon_path = manager.script_dir / "pdf_icon.ico"
        
        if not icon_path.exists():
            if not manager.download_default_icon(icon_path):
                return 1
    
    # Aplicar icono
    if manager.set_icon(args.exe, icon_path):
        manager.log(f"\n✨ Listo! Icono aplicado a {args.exe}", "success")
        return 0
    else:
        return 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print("\n\n⚠️  Operación cancelada por el usuario")
        sys.exit(1)
    except Exception as e:
        print(f"\n❌ Error inesperado: {e}", file=sys.stderr)
        sys.exit(1)
