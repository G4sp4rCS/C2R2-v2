// Utilidades compartidas para el stealer
use std::path::PathBuf;

/// Obtiene el directorio de AppData del usuario actual
pub fn get_appdata_local() -> Option<PathBuf> {
    std::env::var("LOCALAPPDATA")
        .ok()
        .map(PathBuf::from)
}

/// Obtiene el directorio de AppData\Roaming del usuario actual
pub fn get_appdata_roaming() -> Option<PathBuf> {
    std::env::var("APPDATA")
        .ok()
        .map(PathBuf::from)
}

/// Encoder Base64 nativo (sin dependencias externas)
pub fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();

    for chunk in input.chunks(3) {
        let b1 = chunk[0];
        let b2 = chunk.get(1).copied().unwrap_or(0);
        let b3 = chunk.get(2).copied().unwrap_or(0);

        result.push(CHARS[(b1 >> 2) as usize] as char);
        result.push(CHARS[(((b1 & 0x03) << 4) | (b2 >> 4)) as usize] as char);

        if chunk.len() > 1 {
            result.push(CHARS[(((b2 & 0x0F) << 2) | (b3 >> 6)) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(CHARS[(b3 & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

/// Decoder Base64 nativo (sin dependencias externas)
pub fn base64_decode(input: &str) -> Result<Vec<u8>, ()> {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = Vec::new();
    let input_bytes: Vec<u8> = input.bytes().filter(|&b| b != b'=' && !b.is_ascii_whitespace()).collect();

    for chunk in input_bytes.chunks(4) {
        let mut buf = [0u8; 4];

        for (i, &byte) in chunk.iter().enumerate() {
            match CHARS.iter().position(|&c| c == byte) {
                Some(pos) => buf[i] = pos as u8,
                None => return Err(()),
            }
        }

        result.push((buf[0] << 2) | (buf[1] >> 4));

        if chunk.len() > 2 {
            result.push((buf[1] << 4) | (buf[2] >> 2));
        }

        if chunk.len() > 3 {
            result.push((buf[2] << 6) | buf[3]);
        }
    }

    Ok(result)
}

/// Verifica si un archivo existe
pub fn file_exists(path: &PathBuf) -> bool {
    path.exists() && path.is_file()
}
