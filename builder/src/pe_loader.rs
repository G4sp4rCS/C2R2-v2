//! PE Loader - In-memory execution of Windows PE files
//!
//! This module provides functionality to load and execute a PE file
//! entirely in memory without using the standard Windows loader.
//!
//! **Why this exists**:
//! - Alternative to donut when it's not available
//! - Pure Rust implementation, no external dependencies
//! - Works with any PE file (EXE or DLL)
//!
//! **How it works**:
//! 1. Parse PE headers
//! 2. Allocate memory with proper size
//! 3. Copy sections to correct locations
//! 4. Process relocations
//! 5. Resolve imports
//! 6. Execute entry point

use std::error::Error;
use std::ffi::c_void;

#[cfg(target_os = "windows")]
use winapi::um::libloaderapi::{GetProcAddress, LoadLibraryA};
#[cfg(target_os = "windows")]
use winapi::um::memoryapi::{VirtualAlloc, VirtualProtect};
#[cfg(target_os = "windows")]
use winapi::um::winnt::{
    MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, PAGE_READWRITE,
    IMAGE_DOS_HEADER, IMAGE_NT_HEADERS64, IMAGE_SECTION_HEADER,
    IMAGE_DIRECTORY_ENTRY_BASERELOC, IMAGE_DIRECTORY_ENTRY_IMPORT,
    IMAGE_BASE_RELOCATION, IMAGE_IMPORT_DESCRIPTOR,
    IMAGE_REL_BASED_DIR64, IMAGE_REL_BASED_HIGHLOW,
};

/// PE file magic numbers
const DOS_SIGNATURE: u16 = 0x5A4D;  // "MZ"
const NT_SIGNATURE: u32 = 0x00004550;  // "PE\0\0"

/// Result of PE loading
pub struct LoadedPE {
    pub base_address: *mut c_void,
    pub entry_point: *mut c_void,
    pub size: usize,
}

/// Load and execute a PE file in memory
/// 
/// # Arguments
/// 
/// * `pe_data` - Raw bytes of the PE file
/// 
/// # Returns
/// 
/// * `Ok(())` - PE executed successfully
/// * `Err(_)` - Failed to load or execute
/// 
/// # Safety
/// 
/// This function is inherently unsafe as it executes arbitrary code.
#[cfg(target_os = "windows")]
pub fn load_and_execute_pe(pe_data: &[u8]) -> Result<(), Box<dyn Error>> {
    unsafe {
        // Step 1: Validate and parse PE headers
        let loaded = load_pe_into_memory(pe_data)?;
        
        // Step 2: Execute entry point
        let entry: extern "C" fn() = std::mem::transmute(loaded.entry_point);
        entry();
        
        Ok(())
    }
}

/// Load PE into memory without executing
#[cfg(target_os = "windows")]
pub fn load_pe_into_memory(pe_data: &[u8]) -> Result<LoadedPE, Box<dyn Error>> {
    if pe_data.len() < std::mem::size_of::<IMAGE_DOS_HEADER>() {
        return Err("Invalid PE: too small".into());
    }
    
    unsafe {
        // Parse DOS header
        let dos_header = pe_data.as_ptr() as *const IMAGE_DOS_HEADER;
        if (*dos_header).e_magic != DOS_SIGNATURE {
            return Err("Invalid PE: bad DOS signature".into());
        }
        
        // Parse NT headers
        let nt_headers = pe_data.as_ptr().offset((*dos_header).e_lfanew as isize) 
            as *const IMAGE_NT_HEADERS64;
        if (*nt_headers).Signature != NT_SIGNATURE {
            return Err("Invalid PE: bad NT signature".into());
        }
        
        let optional_header = &(*nt_headers).OptionalHeader;
        let file_header = &(*nt_headers).FileHeader;
        
        // Allocate memory for the image
        let image_size = optional_header.SizeOfImage as usize;
        let image_base = VirtualAlloc(
            std::ptr::null_mut(),
            image_size,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );
        
        if image_base.is_null() {
            return Err("Failed to allocate memory for PE".into());
        }
        
        // Copy headers
        std::ptr::copy_nonoverlapping(
            pe_data.as_ptr(),
            image_base as *mut u8,
            optional_header.SizeOfHeaders as usize,
        );
        
        // Copy sections
        let section_header = (nt_headers as *const u8)
            .offset(std::mem::size_of::<IMAGE_NT_HEADERS64>() as isize)
            as *const IMAGE_SECTION_HEADER;
        
        for i in 0..file_header.NumberOfSections as isize {
            let section = &*section_header.offset(i);
            if section.SizeOfRawData == 0 {
                continue;
            }
            
            let dest = (image_base as *mut u8).offset(section.VirtualAddress as isize);
            let src = pe_data.as_ptr().offset(section.PointerToRawData as isize);
            let size = std::cmp::min(section.SizeOfRawData, section.Misc.VirtualSize()) as usize;
            
            std::ptr::copy_nonoverlapping(src, dest, size);
        }
        
        // Process relocations
        let delta = image_base as isize - optional_header.ImageBase as isize;
        if delta != 0 {
            process_relocations(image_base, nt_headers, delta)?;
        }
        
        // Resolve imports
        resolve_imports(image_base, nt_headers)?;
        
        // Make memory executable
        let mut old_protect: u32 = 0;
        VirtualProtect(
            image_base,
            image_size,
            PAGE_EXECUTE_READWRITE,
            &mut old_protect,
        );
        
        // Calculate entry point
        let entry_point = (image_base as *mut u8)
            .offset(optional_header.AddressOfEntryPoint as isize)
            as *mut c_void;
        
        Ok(LoadedPE {
            base_address: image_base,
            entry_point,
            size: image_size,
        })
    }
}

/// Process PE relocations
#[cfg(target_os = "windows")]
unsafe fn process_relocations(
    image_base: *mut c_void,
    nt_headers: *const IMAGE_NT_HEADERS64,
    delta: isize,
) -> Result<(), Box<dyn Error>> {
    let optional_header = &(*nt_headers).OptionalHeader;
    let reloc_dir = &optional_header.DataDirectory[IMAGE_DIRECTORY_ENTRY_BASERELOC as usize];
    
    if reloc_dir.VirtualAddress == 0 {
        return Ok(()); // No relocations needed
    }
    
    let mut reloc = (image_base as *const u8).offset(reloc_dir.VirtualAddress as isize)
        as *const IMAGE_BASE_RELOCATION;
    let reloc_end = (reloc as *const u8).offset(reloc_dir.Size as isize);
    
    while (reloc as *const u8) < reloc_end && (*reloc).SizeOfBlock != 0 {
        let block_base = (image_base as *mut u8).offset((*reloc).VirtualAddress as isize);
        let entry_count = ((*reloc).SizeOfBlock as usize - 8) / 2;
        let entries = (reloc as *const u8).offset(8) as *const u16;
        
        for i in 0..entry_count as isize {
            let entry = *entries.offset(i);
            let reloc_type = (entry >> 12) as u8;
            let offset = (entry & 0x0FFF) as isize;
            
            match reloc_type as u32 {
                IMAGE_REL_BASED_DIR64 => {
                    let patch_addr = block_base.offset(offset) as *mut u64;
                    *patch_addr = (*patch_addr as isize + delta) as u64;
                }
                IMAGE_REL_BASED_HIGHLOW => {
                    let patch_addr = block_base.offset(offset) as *mut u32;
                    *patch_addr = (*patch_addr as isize + delta) as u32;
                }
                0 => {} // IMAGE_REL_BASED_ABSOLUTE - skip
                _ => {} // Unknown relocation type
            }
        }
        
        reloc = (reloc as *const u8).offset((*reloc).SizeOfBlock as isize)
            as *const IMAGE_BASE_RELOCATION;
    }
    
    Ok(())
}

/// Resolve PE imports
#[cfg(target_os = "windows")]
unsafe fn resolve_imports(
    image_base: *mut c_void,
    nt_headers: *const IMAGE_NT_HEADERS64,
) -> Result<(), Box<dyn Error>> {
    let optional_header = &(*nt_headers).OptionalHeader;
    let import_dir = &optional_header.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT as usize];
    
    if import_dir.VirtualAddress == 0 {
        return Ok(()); // No imports
    }
    
    let mut import_desc = (image_base as *const u8).offset(import_dir.VirtualAddress as isize)
        as *const IMAGE_IMPORT_DESCRIPTOR;
    
    while (*import_desc).Name != 0 {
        let dll_name = (image_base as *const u8).offset((*import_desc).Name as isize)
            as *const i8;
        let dll_handle = LoadLibraryA(dll_name);
        
        if dll_handle.is_null() {
            let name = std::ffi::CStr::from_ptr(dll_name).to_string_lossy();
            return Err(format!("Failed to load DLL: {}", name).into());
        }
        
        // Get the thunk arrays
        let mut orig_thunk = if *(*import_desc).u.OriginalFirstThunk() != 0 {
            (image_base as *const u8).offset(*(*import_desc).u.OriginalFirstThunk() as isize)
                as *const u64
        } else {
            (image_base as *const u8).offset((*import_desc).FirstThunk as isize)
                as *const u64
        };
        
        let mut thunk = (image_base as *mut u8).offset((*import_desc).FirstThunk as isize)
            as *mut u64;
        
        while *orig_thunk != 0 {
            let proc_addr = if *orig_thunk & 0x8000000000000000 != 0 {
                // Import by ordinal
                let ordinal = (*orig_thunk & 0xFFFF) as u16;
                GetProcAddress(dll_handle, ordinal as *const i8)
            } else {
                // Import by name
                let import_by_name = (image_base as *const u8).offset(*orig_thunk as isize);
                let func_name = import_by_name.offset(2) as *const i8;
                GetProcAddress(dll_handle, func_name)
            };
            
            if proc_addr.is_null() {
                return Err("Failed to resolve import".into());
            }
            
            *thunk = proc_addr as u64;
            
            orig_thunk = orig_thunk.offset(1);
            thunk = thunk.offset(1);
        }
        
        import_desc = import_desc.offset(1);
    }
    
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn load_and_execute_pe(_pe_data: &[u8]) -> Result<(), Box<dyn Error>> {
    Err("PE loading is only supported on Windows".into())
}

#[cfg(not(target_os = "windows"))]
pub fn load_pe_into_memory(_pe_data: &[u8]) -> Result<LoadedPE, Box<dyn Error>> {
    Err("PE loading is only supported on Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_invalid_pe() {
        let invalid_data = vec![0u8; 100];
        let result = load_pe_into_memory(&invalid_data);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_too_small() {
        let tiny_data = vec![0u8; 10];
        let result = load_pe_into_memory(&tiny_data);
        assert!(result.is_err());
    }
}
