#!/usr/bin/env python3
"""
Script para descargar iconos de alta calidad desde iconarchive.com
y convertirlos al formato .ico requerido por Windows

DEPENDENCIAS:
    pip install pillow requests

USO:
    python download_icon.py pdf          # Descarga icono de PDF
    python download_icon.py word         # Descarga icono de Word
    python download_icon.py excel        # Descarga icono de Excel
    python download_icon.py folder       # Descarga icono de carpeta
    python download_icon.py windows      # Descarga icono de Windows
"""

import argparse
import io
import os
import requests
import sys

try:
    from PIL import Image
except ImportError:
    print("[!] Error: Pillow no está instalado")
    print("[*] Instalar con: pip install pillow")
    sys.exit(1)

# URLs de iconos de alta calidad (256x256)
ICON_URLS = {
    'pdf': 'https://icons.iconarchive.com/icons/carlosjj/microsoft-office-2013/256/Adobe-Acrobat-PDF-icon.png',
    'word': 'https://icons.iconarchive.com/icons/carlosjj/microsoft-office-2013/256/Word-icon.png',
    'excel': 'https://icons.iconarchive.com/icons/carlosjj/microsoft-office-2013/256/Excel-icon.png',
    'folder': 'https://icons.iconarchive.com/icons/iconarchive/yellow-legacy/256/Folder-icon.png',
    'windows': 'https://icons.iconarchive.com/icons/dakirby309/windows-8-metro/256/Folders-Windows-Folder-Metro-icon.png',
    'chrome': 'https://icons.iconarchive.com/icons/google/chrome/256/Google-Chrome-icon.png',
    'edge': 'https://icons.iconarchive.com/icons/papirus-team/papirus-apps/256/microsoft-edge-icon.png',
}

def download_icon(icon_type, output_path='icon.ico'):
    """
    Descarga un icono y lo convierte a formato .ico
    
    Args:
        icon_type: Tipo de icono (pdf, word, excel, etc.)
        output_path: Ruta de salida del archivo .ico
    """
    if icon_type not in ICON_URLS:
        print(f"[!] Error: Tipo de icono '{icon_type}' no soportado")
        print(f"[*] Tipos disponibles: {', '.join(ICON_URLS.keys())}")
        return False
    
    url = ICON_URLS[icon_type]
    
    try:
        print(f"[*] Descargando icono {icon_type}...")
        print(f"[*] URL: {url}")
        
        # Descargar imagen
        response = requests.get(url, timeout=10)
        response.raise_for_status()
        
        # Abrir imagen con Pillow
        img = Image.open(io.BytesIO(response.content))
        
        print(f"[*] Imagen descargada: {img.size[0]}x{img.size[1]} pixels")
        
        # Convertir a RGBA si no lo está
        if img.mode != 'RGBA':
            img = img.convert('RGBA')
        
        # Crear múltiples tamaños para el .ico (Windows usa diferentes tamaños)
        # Tamaños: 16x16, 32x32, 48x48, 64x64, 128x128, 256x256
        sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
        
        # Redimensionar para cada tamaño
        images = []
        for size in sizes:
            resized = img.resize(size, Image.Resampling.LANCZOS)
            images.append(resized)
        
        # Guardar como .ico con múltiples tamaños
        print(f"[*] Guardando icono en: {output_path}")
        img.save(output_path, format='ICO', sizes=sizes)
        
        print(f"[+] ✅ Icono guardado exitosamente: {output_path}")
        print(f"[*] Tamaños incluidos: {', '.join([f'{s[0]}x{s[1]}' for s in sizes])}")
        
        return True
        
    except requests.RequestException as e:
        print(f"[!] Error descargando icono: {e}")
        return False
    except Exception as e:
        print(f"[!] Error procesando imagen: {e}")
        return False

def create_custom_ico(image_path, output_path='icon.ico'):
    """
    Convierte una imagen existente (PNG, JPG) a formato .ico
    
    Args:
        image_path: Ruta de la imagen de entrada
        output_path: Ruta de salida del archivo .ico
    """
    try:
        print(f"[*] Convirtiendo {image_path} a {output_path}...")
        
        img = Image.open(image_path)
        
        if img.mode != 'RGBA':
            img = img.convert('RGBA')
        
        sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
        
        img.save(output_path, format='ICO', sizes=sizes)
        
        print(f"[+] ✅ Icono creado: {output_path}")
        return True
        
    except Exception as e:
        print(f"[!] Error: {e}")
        return False

def main():
    parser = argparse.ArgumentParser(description='Descargador de iconos para Windows')
    parser.add_argument('type', nargs='?', help='Tipo de icono (pdf, word, excel, etc.)')
    parser.add_argument('--output', '-o', default='icon.ico', help='Archivo de salida')
    parser.add_argument('--custom', '-c', help='Convertir imagen personalizada a .ico')
    parser.add_argument('--list', '-l', action='store_true', help='Listar iconos disponibles')
    
    args = parser.parse_args()
    
    if args.list:
        print("[*] Iconos disponibles:")
        for icon_type in ICON_URLS.keys():
            print(f"    - {icon_type}")
        return
    
    if args.custom:
        if not os.path.exists(args.custom):
            print(f"[!] Error: No se encontró {args.custom}")
            sys.exit(1)
        create_custom_ico(args.custom, args.output)
        return
    
    if not args.type:
        print("[!] Error: Debes especificar un tipo de icono")
        print(f"[*] Tipos disponibles: {', '.join(ICON_URLS.keys())}")
        print(f"[*] Uso: {sys.argv[0]} <tipo>")
        sys.exit(1)
    
    download_icon(args.type, args.output)

if __name__ == '__main__':
    main()
