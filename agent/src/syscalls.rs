// Direct Syscalls - Bypass completo de hooks en user-mode
// Ejecuta syscalls directamente sin pasar por ntdll.dll

use std::arch::asm;
use winapi::shared::ntdef::{NTSTATUS, HANDLE, PVOID};

/// Syscall numbers para Windows 10/11 (x64)
/// Estos números cambian entre versiones de Windows
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum SyscallNumber {
    NtAllocateVirtualMemory = 0x18,
    NtProtectVirtualMemory = 0x50,
}

/// Ejecuta NtAllocateVirtualMemory via syscall directo
/// Bypasses TODOS los hooks en ntdll.dll
#[cfg(target_arch = "x86_64")]
pub unsafe fn nt_allocate_virtual_memory(
    process_handle: HANDLE,
    base_address: *mut PVOID,
    zero_bits: usize,
    region_size: *mut usize,
    allocation_type: u32,
    protect: u32,
) -> NTSTATUS {
    // Wrapper interno que ejecuta el syscall
    // Esta función DEBE ser extern "system" para que respete la calling convention de Windows
    #[allow(improper_ctypes_definitions)]
    unsafe extern "system" fn syscall_wrapper(
        process_handle: HANDLE,
        base_address: *mut PVOID,
        zero_bits: usize,
        region_size: *mut usize,
        allocation_type: u32,
        protect: u32,
    ) -> NTSTATUS {
        let syscall_num: u32 = 0x18;
        let mut status: i32;
        
        // Ahora allocation_type y protect están CORRECTAMENTE en el stack
        asm!(
            "mov r10, rcx",         // Syscall convention
            "mov eax, {syscall:e}", // Load syscall number
            "syscall",              // Execute
            syscall = in(reg) syscall_num,
            in("rcx") process_handle,
            in("rdx") base_address,
            in("r8") zero_bits,
            in("r9") region_size,
            // allocation_type en [rsp+0x28]
            // protect en [rsp+0x30]
            lateout("eax") status,
        );
        
        status
    }
    
    // Llamar al wrapper (esto garantiza el stack layout correcto)
    syscall_wrapper(
        process_handle,
        base_address,
        zero_bits,
        region_size,
        allocation_type,
        protect,
    )
}

/// Ejecuta NtProtectVirtualMemory via syscall directo
#[cfg(target_arch = "x86_64")]
pub unsafe fn nt_protect_virtual_memory(
    process_handle: HANDLE,
    base_address: *mut PVOID,
    region_size: *mut usize,
    new_protect: u32,
    old_protect: *mut u32,
) -> NTSTATUS {
    let syscall_num = SyscallNumber::NtProtectVirtualMemory as u32;
    let mut status: i32;

    asm!(
        "mov r10, rcx",
        "mov eax, {syscall:e}",
        "syscall",
        syscall = in(reg) syscall_num,
        in("rcx") process_handle,
        in("rdx") base_address,
        in("r8") region_size,
        in("r9") new_protect,
        // old_protect va en el stack
        lateout("eax") status,
        options(nostack)
    );

    status
}
