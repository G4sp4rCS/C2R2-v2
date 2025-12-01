#!/usr/bin/env python3
"""
Convertir agent.exe a shellcode usando pe_to_shellcode
"""

import sys
import os

try:
    from pefile import PE
except ImportError:
    print("[!] Instalar: pip install pefile")
    sys.exit(1)

def pe_to_shellcode_simple(pe_path, output_path):
    """
    Método simple: extraer secciones y crear loader básico
    NOTA: Esto NO es reflective loading completo, solo extrae el código
    """
    print(f"[*] Leyendo PE: {pe_path}")
    
    with open(pe_path, 'rb') as f:
        pe_data = f.read()
    
    print(f"[*] Tamaño PE: {len(pe_data)} bytes")
    
    # Por ahora, simplemente guardamos el PE raw
    # Para un shellcode completo necesitamos Donut o sRDI
    with open(output_path, 'wb') as f:
        f.write(pe_data)
    
    print(f"[+] PE raw guardado en: {output_path}")
    print(f"\n[!] IMPORTANTE:")
    print(f"    Este es el PE raw, NO shellcode position-independent")
    print(f"    Para convertir a shellcode real usa:")
    print(f"    1. Donut: https://github.com/TheWover/donut")
    print(f"    2. sRDI: https://github.com/monoxgas/sRDI")
    print(f"    3. pe_to_shellcode: https://github.com/hasherezade/pe_to_shellcode")

if __name__ == '__main__':
    import argparse
    
    parser = argparse.ArgumentParser(description='Convertir PE a shellcode (requiere Donut)')
    parser.add_argument('--input', required=True, help='agent.exe')
    parser.add_argument('--output', required=True, help='agent.bin (shellcode)')
    
    args = parser.parse_args()
    
    if not os.path.exists(args.input):
        print(f"[!] No se encuentra: {args.input}")
        sys.exit(1)
    
    pe_to_shellcode_simple(args.input, args.output)
    
    print(f"\n[💡] Para convertir a shellcode real con Donut:")
    print(f"    1. Descargar: https://github.com/TheWover/donut/releases")
    print(f"    2. Ejecutar: donut.exe -f {args.input} -o {args.output}")
    print(f"    3. Esto genera shellcode position-independent listo para inyectar")
