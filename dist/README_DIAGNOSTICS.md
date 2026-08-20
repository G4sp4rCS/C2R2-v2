#  Herramientas de Diagnóstico - Web Data

Estos ejecutables diagnostican la estructura de la base de datos `Web Data` de Edge/Chrome para ayudar a entender por qué el stealer no detecta tarjetas o direcciones.

##  Archivos

- **debug_webdata.exe** - Analiza la estructura completa de Web Data
- **find_cards.exe** - Encuentra dónde están guardadas las tarjetas y direcciones

##  Uso en la VM Windows

### 1. Copiar archivos a la VM

Transfiere los archivos desde `dist/` a tu VM Windows.

### 2. Ejecutar find_cards.exe (recomendado primero)

```cmd
find_cards.exe
```

**Esto mostrará:**
- Todos los perfiles de Edge encontrados (Default, Profile 1, Profile 2, etc.)
- Qué tablas tienen datos de tarjetas
- Qué tablas tienen datos de direcciones
- Cantidad de registros en cada tabla

**Salida esperada:**
```
 Perfiles encontrados: 1

 Perfil: Default
   Path: C:\Users\...\Microsoft\Edge\User Data\Default\Web Data

 TARJETAS:
    credit_cards: 1 registros  ← Aquí debería aparecer tu tarjeta
      Columnas: guid, name_on_card, expiration_month, ...

 DIRECCIONES:
    addresses: 3 registros  ← Direcciones guardadas
      Columnas: guid, use_count, date_modified, ...
```

### 3. Ejecutar debug_webdata.exe (análisis detallado)

```cmd
debug_webdata.exe
```

**Esto mostrará:**
- Lista de TODAS las tablas en Web Data (50+)
- Schema completo de tablas importantes
- Columnas exactas de cada tabla

##  Interpretación de resultados

###  Si `credit_cards` tiene 0 registros:

**Problema**: Edge no está guardando tarjetas localmente, está usando la nube de Microsoft.

**Solución**:
1. Abre Edge → `edge://settings/payments`
2. Desactiva "Guardar y rellenar métodos de pago"
3. Elimina la tarjeta actual
4. Vuelve a agregar la tarjeta (ahora se guardará localmente)
5. Ejecuta `find_cards.exe` de nuevo
6. Debería aparecer: ` credit_cards: 1 registros`

###  Si `addresses` tiene N registros pero el stealer no los detecta:

**Problema**: El código del stealer está buscando en la tabla incorrecta.

**Solución**: Necesitamos actualizar `autofill.rs` para leer de la tabla `addresses` en lugar de `autofill_profile_addresses`.

###  Si encuentras múltiples perfiles:

El stealer ya está configurado para buscar en Profile 1-5, así que debería detectarlos automáticamente.

##  Reportar resultados

Después de ejecutar `find_cards.exe`, comparte la salida completa para que pueda ajustar el código del stealer según la estructura real de tu base de datos.

##  Notas

- Estos ejecutables son **seguros** y solo **leen** la base de datos
- **NO modifican** ningún archivo
- Crean copias temporales en `%TEMP%` que se eliminan automáticamente
- Requieren permisos de usuario normal (no admin)
