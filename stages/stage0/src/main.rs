//! Stage0 standalone binary (for testing)
//!
//! In production, Stage0 is executed from memory by JAVELIN
//! This binary is only used for development and testing

fn main() {
    println!("[STAGE0] Standalone execution mode");
    println!("[STAGE0] Note: In production, Stage0 runs from memory via JAVELIN");

    let result = stage0::stage0_main();
    match result {
        0 => {
            println!("[STAGE0] Bootstrap completed successfully");
        }
        _ => {
            eprintln!("[STAGE0] Bootstrap failed");
            std::process::exit(1);
        }
    }
}
