//! R2C2 es un proyecto de Command & Control para red team engagements en Linux y Windows.

mod encrypt;
use std::env;
use encrypt::generate_agent;
use std::io::{self, Write};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("R2C2 - Red Team Command & Control");
        println!("Uso:");
        println!("  {} --encrypt <shellcode.bin>", args[0]);
        return;
    }

    match args[1].as_str() {
        "--encrypt" => {
            if args.len() < 3 {
                println!("❌ Error: Especifica el archivo de shellcode");
                println!("Uso: {} --encrypt shellcode.bin", args[0]);
                return;
            }

            let shellcode_file = &args[2];
            
            // Solicitar contraseña
            print!("🔐 Ingresa la contraseña para encriptar: ");
            io::stdout().flush().unwrap();
            
            let mut password = String::new();
            io::stdin().read_line(&mut password).unwrap();
            let password = password.trim();

            // Solicitar nombre del agente
            print!("📝 Nombre del agente (sin extensión): ");
            io::stdout().flush().unwrap();
            
            let mut agent_name = String::new();
            io::stdin().read_line(&mut agent_name).unwrap();
            let agent_name = agent_name.trim();

            match generate_agent(shellcode_file, password) {
                Ok(_) => {
                    println!("✅ Agente generado exitosamente");
                    println!("🚀 Compila con: rustc {}.rs -o {}.exe", agent_name, agent_name);
                }
                Err(e) => println!("❌ Error: {}", e),
            }
        }
        _ => {
            println!("❌ Opción no reconocida: {}", args[1]);
            println!("Opciones disponibles: --encrypt");
        }
    }
}
