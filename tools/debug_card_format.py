#!/usr/bin/env python3
"""
Analiza el formato de la encriptación de tarjetas de Firefox
"""
import json
import base64
import sys
from pathlib import Path

def analyze_card_encryption(json_file: str):
    """Analiza formato de cc-number-encrypted"""
    
    with open(json_file, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    credit_cards = data.get("creditCards", [])
    
    if not credit_cards:
        print("[!] No credit cards found")
        return
    
    for idx, card in enumerate(credit_cards, 1):
        print(f"\n[Card #{idx}]")
        print(f"  Name: {card.get('cc-name', 'N/A')}")
        
        encrypted_number = card.get("cc-number-encrypted", "")
        if not encrypted_number:
            print("  [!] No cc-number-encrypted field")
            continue
        
        print(f"  Encrypted (Base64): {encrypted_number[:80]}...")
        
        # Decodificar Base64
        try:
            encrypted_bytes = base64.b64decode(encrypted_number)
            print(f"  Encrypted (bytes): {len(encrypted_bytes)} bytes")
            print(f"  Hex dump (first 32 bytes):")
            hex_dump = ' '.join(f'{b:02x}' for b in encrypted_bytes[:32])
            print(f"    {hex_dump}")
            
            # Verificar prefijo NSS común
            if encrypted_bytes[:2] == b'\x30\x32':  # "02" en hex (ASN.1 SEQUENCE)
                print("  ✅ Tiene prefijo ASN.1 SEQUENCE (0x30 0x32)")
            elif encrypted_bytes[:2] == b'MD':  # Prefijo MDI de NSS
                print("  ✅ Tiene prefijo NSS MDI")
            else:
                print(f"  ⚠️ Prefijo desconocido: {encrypted_bytes[:4].hex()}")
            
            # Analizar estructura ASN.1
            if encrypted_bytes[0] == 0x30:  # SEQUENCE
                length = encrypted_bytes[1]
                print(f"  ASN.1 SEQUENCE length: {length}")
                
                # OID
                oid_len = 0
                if encrypted_bytes[2] == 0x06:  # OBJECT IDENTIFIER
                    oid_len = encrypted_bytes[3]
                    oid_bytes = encrypted_bytes[4:4+oid_len]
                    print(f"  OID: {oid_bytes.hex()}")
                
                # Encrypted data
                octet_pos = 4 + oid_len
                if encrypted_bytes[octet_pos] == 0x04:  # OCTET STRING
                    data_len = encrypted_bytes[octet_pos + 1]
                    actual_encrypted = encrypted_bytes[octet_pos + 2:octet_pos + 2 + data_len]
                    print(f"  Actual encrypted data: {len(actual_encrypted)} bytes")
                    print(f"    {actual_encrypted[:16].hex()}...")
        
        except Exception as e:
            print(f"  [!] Error decoding: {e}")

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python debug_card_format.py <autofill-profiles.json>")
        sys.exit(1)
    
    analyze_card_encryption(sys.argv[1])
