#!/usr/bin/env python3
"""
Firefox Credit Cards Parser
Extrae tarjetas de crédito cifradas de autofill-profiles.json

Uso:
    python parse_firefox_cards.py harvested/firefox/<profile>/autofill-profiles.json
"""

import sys
import json
import base64
from pathlib import Path


def parse_firefox_cards(json_file):
    """
    Parse autofill-profiles.json y extrae info de tarjetas
    
    Args:
        json_file: Ruta al archivo autofill-profiles.json
    """
    json_path = Path(json_file)
    
    if not json_path.exists():
        print(f"[!] ERROR: File not found: {json_file}")
        return
    
    # Leer JSON
    with open(json_path, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    # Extraer tarjetas
    cards = data.get('creditCards', [])
    
    if not cards:
        print("[!] No credit cards found in file")
        return
    
    print(f"\n[+] Found {len(cards)} credit card(s):\n")
    print("="*60)
    
    for i, card in enumerate(cards, 1):
        print(f"\n[Card #{i}]")
        print(f"  Name: {card.get('cc-name', 'N/A')}")
        print(f"  Type: {card.get('cc-type', 'N/A')}")
        print(f"  Expiration: {card.get('cc-exp-month', '?')}/{card.get('cc-exp-year', '?')}")
        
        # Número de tarjeta (puede estar enmascarado o cifrado)
        masked_number = card.get('cc-number', '')
        encrypted_number = card.get('cc-number-encrypted', '')
        
        if masked_number:
            print(f"  Number (masked): {masked_number}")
        
        if encrypted_number:
            print(f"  Number (encrypted): {encrypted_number[:50]}... ({len(encrypted_number)} chars)")
            print(f"  ⚠️  ENCRYPTED - Requires NSS decrypt with Master Password")
            print(f"      (This is the 1% edge case - user has Master Password)")
        
        # Metadata
        print(f"  Times used: {card.get('timesUsed', 0)}")
        print(f"  Created: {card.get('timeCreated', 'N/A')}")
        print(f"  Last modified: {card.get('timeLastModified', 'N/A')}")
    
    print("\n" + "="*60)
    print("\n[!] NOTE:")
    print("    Firefox credit cards are encrypted with NSS (Network Security Services)")
    print("    If Master Password is set, cannot decrypt without it")
    print("    This is a security feature - only 1% of users have it enabled")
    print("\n    Recommendation:")
    print("    - For 99% of users: Cards would decrypt with key4.db")
    print("    - For 1% with Master Password: Brute-force or social engineering")


def main():
    if len(sys.argv) < 2:
        print("Usage: python parse_firefox_cards.py <autofill-profiles.json>")
        print("\nExample:")
        print("  python parse_firefox_cards.py harvested/firefox/foqs9fmi.default-release/autofill-profiles.json")
        sys.exit(1)
    
    json_file = sys.argv[1]
    parse_firefox_cards(json_file)


if __name__ == "__main__":
    main()
