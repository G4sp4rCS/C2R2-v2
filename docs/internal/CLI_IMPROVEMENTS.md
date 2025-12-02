# 🎨 CLI Mejorada con Rustyline

## ✨ Nueva Experiencia de Usuario

C2R2 v2.0 ahora cuenta con una CLI moderna y profesional gracias a **`rustyline`**, ofreciendo una experiencia similar a `bash`, `zsh` o `fish`.

---

## ⌨️ **Shortcuts y Atajos de Teclado**

### **Navegación en la Línea**

| Atajo | Acción |
|-------|--------|
| `←` `→` | Mover cursor izquierda/derecha en la línea |
| `Ctrl + A` | Ir al inicio de la línea |
| `Ctrl + E` | Ir al final de la línea |
| `Ctrl + ←` `→` | Saltar palabras |
| `Home` / `End` | Inicio / Fin de línea |

### **Edición de Texto**

| Atajo | Acción |
|-------|--------|
| `Backspace` | Borrar carácter anterior |
| `Delete` | Borrar carácter siguiente |
| `Ctrl + W` | Borrar palabra anterior |
| `Ctrl + K` | Borrar desde cursor hasta final |
| `Ctrl + U` | Borrar toda la línea |
| `Ctrl + T` | Intercambiar caracteres |

### **Historial de Comandos**

| Atajo | Acción |
|-------|--------|
| `↑` | Comando anterior |
| `↓` | Comando siguiente |
| `Ctrl + R` | Búsqueda reversa en historial (interactiva) |
| `Ctrl + S` | Búsqueda hacia adelante en historial |

### **Control de Pantalla**

| Atajo | Acción |
|-------|--------|
| `Ctrl + L` | **Limpiar pantalla** ✨ |
| `Ctrl + C` | Cancelar comando actual / Salir |
| `Ctrl + D` | Salir (EOF) |

### **Otros Atajos**

| Atajo | Acción |
|-------|--------|
| `Tab` | Autocompletado (actualmente no configurado) |
| `Ctrl + Z` | Suspender (Unix) |

---

## 💾 **Persistencia de Historial**

### **Archivo de Historial**
- **Ubicación**: `.c2r2_history` (en el directorio de ejecución)
- **Auto-guardado**: Al salir con `/exit`, `/quit`, `Ctrl+C` o `Ctrl+D`
- **Auto-carga**: Al iniciar el servidor

### **Gestión del Historial**

```bash
# Ver historial
cat .c2r2_history

# Limpiar historial
rm .c2r2_history

# Buscar en historial
grep "/upload" .c2r2_history
```

---

## 🎯 **Ejemplos de Uso**

### **1. Navegación Rápida con Flechas**

```bash
C2R2> /select 1  ↵
✅ Cliente [1]

C2R2[1]> /cmd whoami  ↵
📨 Respuesta: domain\admin

C2R2[1]>  # Presiona ↑ para repetir comando
C2R2[1]> /cmd whoami  # ← El comando se autocompletó
```

### **2. Edición con Ctrl+W**

```bash
C2R2[1]> /upload /wrong/path/file.txt c:\dest.txt
          # Ctrl+W borra "/wrong/path/file.txt"
C2R2[1]> /upload  # ← Quedó listo para escribir path correcto
```

### **3. Buscar en Historial con Ctrl+R**

```bash
C2R2[1]>  # Presiona Ctrl+R
(reverse-i-search)`down': /download c:\passwords.txt
# Escribe "down" y encuentra comandos con "download"
# Presiona Enter para ejecutar o Esc para editar
```

### **4. Limpiar Pantalla con Ctrl+L**

```bash
C2R2[1]> /list
[mucha salida...]

C2R2[1]>  # Presiona Ctrl+L
# ✨ Pantalla limpia, prompt en la parte superior
```

---

## 🔧 **Características Técnicas**

### **Biblioteca: `rustyline` v14.0**

- **GitHub**: https://github.com/kkawakam/rustyline
- **Inspirado en**: GNU Readline (bash)
- **Bindings**: Emacs-style por defecto

### **Ventajas**

✅ **Sin dependencias externas** - Puro Rust
✅ **Cross-platform** - Windows, Linux, macOS
✅ **UTF-8 completo** - Soporte Unicode
✅ **Historial persistente** - Entre sesiones
✅ **Configurable** - Bindings, colores, comportamiento

### **Limitaciones Actuales**

⚠️ **Autocompletado**: No configurado (se puede agregar en el futuro)
⚠️ **Hints**: No muestra sugerencias en tiempo real
⚠️ **Syntax highlighting**: Prompt básico sin colores en input

---

## 🚀 **Mejoras Futuras Posibles**

### **Autocompletado de Comandos**

```rust
// Posible implementación futura
impl Completer for C2R2Completer {
    fn complete(&self, line: &str, pos: usize) -> Result<(usize, Vec<Pair>)> {
        let commands = vec![
            "/help", "/list", "/select", "/cmd", "/cmd_all",
            "/download", "/upload", "/info", "/deselect", "/exit"
        ];
        // Lógica de completado...
    }
}
```

### **Hints Contextuales**

```bash
C2R2[1]> /upload  # ← Hint: <archivo_local> <ruta_remota>
```

### **Syntax Highlighting**

```bash
C2R2[1]> /cmd whoami  # "/cmd" en verde, "whoami" en blanco
```

### **Historial Filtrado por Cliente**

```bash
C2R2[1]> # Historial solo muestra comandos de cliente [1]
```

---

## 📊 **Comparación: Antes vs Ahora**

| Feature | Antes (stdin básico) | Ahora (rustyline) |
|---------|----------------------|-------------------|
| Historial de comandos | ❌ | ✅ ↑/↓ |
| Editar comando | ❌ | ✅ ←/→, Ctrl+A/E |
| Limpiar pantalla | ❌ | ✅ Ctrl+L |
| Buscar en historial | ❌ | ✅ Ctrl+R |
| Borrar palabra | ❌ | ✅ Ctrl+W |
| Persistencia | ❌ | ✅ `.c2r2_history` |
| Salida elegante | ❌ | ✅ Ctrl+C/D |

---

## 🎮 **Tutorial Rápido**

### **1. Iniciar el Servidor**

```bash
./target/release/c2r2-server -p 4444 -b 0.0.0.0 -v
```

### **2. Probar Navegación**

```bash
C2R2> /help        # Escribe y presiona Enter
C2R2> /list        # Presiona ↑ para ver /help
C2R2> /help        # ← Autocompletado con ↑
```

### **3. Probar Edición**

```bash
C2R2> /select 999  # Cliente inexistente
C2R2>              # Presiona ↑, luego Ctrl+W para borrar "999"
C2R2> /select      # Escribe el ID correcto
C2R2> /select 1    # ✅
```

### **4. Probar Ctrl+L**

```bash
C2R2[1]> /list
[salida larga...]

# Presiona Ctrl+L → Pantalla limpia ✨
C2R2[1]>  # Prompt en la parte superior
```

### **5. Probar Ctrl+R**

```bash
C2R2[1]>  # Presiona Ctrl+R
(reverse-i-search)`':  # Escribe "down"
(reverse-i-search)`down': /download c:\passwords.txt
# Presiona Enter para ejecutar
```

---

## ⚙️ **Configuración Avanzada (Futuro)**

### **Archivo de Configuración Potencial**

`.c2r2rc`:
```toml
[readline]
editing_mode = "emacs"  # o "vi"
history_size = 1000
auto_add_history = true
color_mode = "enabled"

[completion]
case_sensitive = false
show_all_if_ambiguous = true

[keybindings]
# Personalizar atajos...
```

---

## 🐛 **Troubleshooting**

### **El historial no se guarda**

```bash
# Verifica permisos del directorio
ls -la .c2r2_history

# Asegúrate de salir correctamente
# ❌ NO: Ctrl+Z o kill -9
# ✅ SÍ: /exit, /quit, Ctrl+C, Ctrl+D
```

### **Ctrl+L no funciona en Windows**

- **Solución**: rustyline soporta Ctrl+L en Windows con terminal moderno (Windows Terminal, ConEmu)
- **Alternativa**: Escribe `cls` o `/help` y desplázate hacia arriba

### **Caracteres raros en el prompt**

- **Causa**: Terminal sin soporte UTF-8
- **Solución**: Usa Windows Terminal, mintty o similar

---

## 📝 **Referencias**

- **rustyline GitHub**: https://github.com/kkawakam/rustyline
- **GNU Readline Manual**: https://tiswww.case.edu/php/chet/readline/rltop.html
- **Emacs Keybindings**: https://www.gnu.org/software/emacs/refcards/pdf/refcard.pdf

---

**¡Disfruta de la nueva CLI mejorada!** 🎉
