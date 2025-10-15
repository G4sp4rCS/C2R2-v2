//! R2C2 es un proyecto de Command & Control para red team engagements en Linux y Windows.

mod encrypt;

use clap::Parser;
use encrypt::generate_agent;

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

    println!("🔧 C2R2 Agent Builder v2.0 - Direct Connection");
    println!("🏷️  Agente: {}", args.name);
    println!("🌐 Servidor C2: {}", args.server);
    println!("{}", "-".repeat(50));

    match generate_agent(&args.name, &args.server) {
        Ok(_) => {
            println!("✅ Agente generado exitosamente");
        }
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            std::process::exit(1);
        }
    }
}
