# 📝 Sistema de Logging C2R2

## Descripción General

C2R2 Server implementa un **sistema completo de logging** que registra **TODAS las interacciones** entre el operador y los agentes en archivos de texto, incluyendo:

- ✅ Comandos enviados por el operador
- ✅ Outputs/respuestas completas de los agentes
- ✅ Transferencias de archivos (upload/download)
- ✅ Conexiones y desconexiones de agentes
- ✅ Información del sistema (SYSINFO)
- ✅ Errores y eventos importantes

## 📂 Ubicación de Logs

Todos los logs se guardan en el directorio **`logs/`** (creado automáticamente):

```
C2R2/
├── logs/
│   ├── c2r2-session.log.2025-10-15    # Log del día actual
│   ├── c2r2-session.log.2025-10-14    # Log del día anterior
│   └── c2r2-session.log.2025-10-13    # Logs más antiguos...
```

### Rotación Automática

- Los logs rotan **diariamente** (nuevo archivo cada día)
- Formato de archivo: `c2r2-session.log.YYYY-MM-DD`
- Los logs antiguos **NO se eliminan automáticamente** (auditoría completa)

## 📋 Formato de Logs

### Formato General

```
TIMESTAMP [NIVEL] [ID_Cliente] Mensaje
```

**Ejemplo:**
```
2025-10-15T14:32:10.123456Z [INFO] [1] Comando /cmd: whoami
2025-10-15T14:32:10.456789Z [INFO] [1] OUTPUT:
nt authority\system
```

### Niveles de Log

| Nivel   | Uso                                          | Color  |
|---------|----------------------------------------------|--------|
| `INFO`  | Eventos normales (comandos, conexiones)     | Blanco |
| `WARN`  | Advertencias (desconexiones)                | Amarillo|
| `ERROR` | Errores (fallas de envío, decodificación)   | Rojo   |
| `DEBUG` | Información detallada (tamaño de respuestas)| Gris   |

## 🎯 Eventos Logueados

### 1. Inicio/Cierre del Servidor

**Inicio:**
```
[INFO] ╔══════════════════════════════════════════════════════════════╗
[INFO] ║          C2R2 Server v2.0 - Session Started                ║
[INFO] ║          Listening: 0.0.0.0:4444                           ║
[INFO] ╚══════════════════════════════════════════════════════════════╝
```

**Cierre (por comando /exit):**
```
[INFO] ═══════════════════════════════════════════════════════════
[INFO] Server cerrado por comando /exit del operador
[INFO] ═══════════════════════════════════════════════════════════
```

**Cierre (por Ctrl+C):**
```
[INFO] ═══════════════════════════════════════════════════════════
[INFO] Server cerrado por Ctrl+C del operador
[INFO] ═══════════════════════════════════════════════════════════
```

### 2. Conexiones de Agentes

```
[INFO] Nueva conexión: [1] desde 192.168.1.100:54321
```

### 3. Información del Sistema (SYSINFO)

```
[INFO] [1] SYSINFO hostname: DESKTOP-ABC123
[INFO] [1] SYSINFO username: john.doe
[INFO] [1] SYSINFO OS: Windows 10 Pro
[INFO] [1] SYSINFO privileges: Admin
```

### 4. Comandos Ejecutados

**Comando individual (/cmd):**
```
[INFO] [1] Comando /cmd: whoami
[INFO] [1] OUTPUT:
nt authority\system
```

**Comando broadcast (/cmd_all):**
```
[INFO] Comando /cmd_all: ipconfig /all (a 3 clientes)
```

### 5. Transferencia de Archivos

**Download:**
```
[INFO] [1] Comando /download: c:\Windows\System32\drivers\etc\hosts
[INFO] [1] Recibiendo archivo descargado
[INFO] [1] Archivo descargado: hosts (824 bytes) -> downloads/hosts
```

**Upload:**
```
[INFO] [1] Comando /upload: ./payload.exe -> c:\Users\Public\payload.exe (51200 bytes)
[INFO] [1] Éxito: Archivo subido correctamente
```

### 6. Errores

**Error de comando:**
```
[ERROR] [1] Error enviando comando: channel closed
```

**Error de archivo:**
```
[ERROR] [1] Error leyendo archivo local 'missing.txt': No such file or directory
```

**Error de decodificación:**
```
[ERROR] [1] Error decodificando base64: Invalid character in base64
```

### 7. Desconexiones

```
[WARN] Cliente [1] desconectado
```

## 🔧 Configuración

### Cambiar Nivel de Log

Por defecto, el nivel es **INFO**. Para ver más detalles (DEBUG), ejecuta:

**Linux/macOS:**
```bash
export RUST_LOG=debug
./target/release/c2r2-server -p 4444 -b 0.0.0.0
```

**Windows (PowerShell):**
```powershell
$env:RUST_LOG="debug"
.\target\release\c2r2-server.exe -p 4444 -b 0.0.0.0
```

**Windows (CMD):**
```cmd
set RUST_LOG=debug
target\release\c2r2-server.exe -p 4444 -b 0.0.0.0
```

### Niveles Disponibles

- `error` - Solo errores críticos
- `warn` - Advertencias + errores
- `info` - **Predeterminado** (comandos, eventos, errores)
- `debug` - Información detallada técnica
- `trace` - Todo (muy verboso, no recomendado)

## 📖 Ejemplos de Uso

### Ver logs en tiempo real (Linux/macOS)

```bash
tail -f logs/c2r2-session.log.$(date +%Y-%m-%d)
```

### Ver logs del día actual (Windows PowerShell)

```powershell
Get-Content "logs\c2r2-session.log.$(Get-Date -Format 'yyyy-MM-dd')" -Wait
```

### Buscar comandos específicos

**Linux/macOS:**
```bash
grep "Comando /cmd:" logs/c2r2-session.log.*
```

**Windows PowerShell:**
```powershell
Select-String "Comando /cmd:" logs\c2r2-session.log.*
```

### Filtrar por cliente específico

**Linux/macOS:**
```bash
grep "\[5\]" logs/c2r2-session.log.2025-10-15
```

**Windows PowerShell:**
```powershell
Select-String "\[5\]" logs\c2r2-session.log.2025-10-15
```

### Extraer solo outputs de comandos

**Linux/macOS:**
```bash
grep -A 20 "OUTPUT:" logs/c2r2-session.log.2025-10-15
```

**Windows PowerShell:**
```powershell
Select-String "OUTPUT:" logs\c2r2-session.log.2025-10-15 -Context 0,20
```

## 🔍 Análisis Forense

### Timeline completo de una sesión

Los logs permiten reconstruir **completamente** una sesión de C2:

1. **Inicio:** Timestamp de inicio del servidor
2. **Conexiones:** Qué agentes se conectaron y cuándo
3. **SYSINFO:** Información del sistema de cada agente
4. **Comandos:** Todos los comandos ejecutados en orden cronológico
5. **Outputs:** Todas las respuestas recibidas (completas)
6. **Archivos:** Qué archivos se descargaron/subieron
7. **Errores:** Problemas encontrados durante la sesión
8. **Cierre:** Timestamp de cierre y motivo (comando/Ctrl+C/Ctrl+D)

### Ejemplo de timeline

```
2025-10-15T10:00:00.000Z [INFO] Server Started - Listening: 0.0.0.0:4444
2025-10-15T10:05:23.456Z [INFO] Nueva conexión: [1] desde 192.168.1.100:54321
2025-10-15T10:05:24.123Z [INFO] [1] SYSINFO hostname: WORKSTATION-01
2025-10-15T10:05:24.234Z [INFO] [1] SYSINFO username: admin
2025-10-15T10:05:24.345Z [INFO] [1] SYSINFO OS: Windows 10 Enterprise
2025-10-15T10:05:24.456Z [INFO] [1] SYSINFO privileges: Admin
2025-10-15T10:07:10.111Z [INFO] [1] Comando /cmd: whoami
2025-10-15T10:07:10.222Z [INFO] [1] OUTPUT:
workstation-01\admin
2025-10-15T10:10:45.333Z [INFO] [1] Comando /download: c:\important.docx
2025-10-15T10:10:46.444Z [INFO] [1] Recibiendo archivo descargado
2025-10-15T10:10:47.555Z [INFO] [1] Archivo descargado: important.docx (45678 bytes)
2025-10-15T10:15:00.666Z [WARN] Cliente [1] desconectado
2025-10-15T10:20:00.777Z [INFO] Server cerrado por comando /exit del operador
```

## 🛡️ Seguridad y Privacidad

### ⚠️ IMPORTANTE

Los logs contienen **información sensible**:
- Comandos ejecutados (pueden incluir credenciales)
- Outputs completos (datos del sistema comprometido)
- Información de archivos transferidos
- IPs y hostnames de agentes

### Recomendaciones

1. **Proteger el directorio `logs/`:**
   ```bash
   chmod 700 logs/
   ```

2. **Cifrar logs antiguos:**
   ```bash
   gpg -c logs/c2r2-session.log.2025-10-14
   rm logs/c2r2-session.log.2025-10-14
   ```

3. **Eliminar logs después de engagement:**
   ```bash
   shred -vfz -n 10 logs/*.log.*
   ```

4. **NO commitear logs a Git:**
   - Ya incluido en `.gitignore`
   - Verificar con: `git status`

## 📊 Estadísticas

El sistema de logging permite análisis post-operación:

- **Duración total de la sesión**
- **Número de agentes comprometidos**
- **Comandos más ejecutados**
- **Cantidad de archivos exfiltrados**
- **Tasa de errores/comandos fallidos**
- **Tiempo promedio de respuesta**

## 🔧 Troubleshooting

### Los logs no se están creando

1. Verificar permisos del directorio:
   ```bash
   ls -la logs/
   ```

2. Verificar que el servidor inició correctamente:
   ```bash
   ./target/release/c2r2-server -v  # modo verbose
   ```

### Los logs están vacíos

- El nivel de log puede estar en `error` (muy restrictivo)
- Cambiar a `info`: `export RUST_LOG=info`

### Logs muy grandes

- Los logs rotan diariamente, pero no se eliminan
- Limpiar logs antiguos manualmente:
  ```bash
  find logs/ -name "*.log.*" -mtime +30 -delete  # Eliminar > 30 días
  ```

## 📚 Referencias Técnicas

- **Crate:** `tracing` v0.1
- **Crate:** `tracing-subscriber` v0.3
- **Crate:** `tracing-appender` v0.2
- **Rotación:** Diaria (Rotation::DAILY)
- **Buffer:** Non-blocking (no afecta performance)
- **Formato:** Texto plano sin ANSI colors

---

**Última actualización:** 15 de Octubre, 2025  
**Versión C2R2:** v2.0 (Direct Connection)
