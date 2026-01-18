//! JAVELIN standalone binary (for testing purposes)
//!
//! In production, JAVELIN is executed from memory by ESTER
//! This binary is only used for development and testing

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
