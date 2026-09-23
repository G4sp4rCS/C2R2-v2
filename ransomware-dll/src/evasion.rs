/// Evasion techniques module
/// Anti-debugging, VM detection, and sandbox evasion

#[cfg(target_os = "windows")]
pub fn check_debugger() -> bool {
    use winapi::um::debugapi::IsDebuggerPresent;

    unsafe { IsDebuggerPresent() != 0 }
}

#[cfg(target_os = "windows")]
pub fn check_analysis_tools() -> bool {
    use std::ffi::CStr;
    use winapi::shared::minwindef::FALSE;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::tlhelp32::{
        CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
    };

    // List of common analysis tools
    let blacklist = [
        "ollydbg.exe",
        "x64dbg.exe",
        "x32dbg.exe",
        "windbg.exe",
        "idaq.exe",
        "idaq64.exe",
        "ida.exe",
        "ida64.exe",
        "processhacker.exe",
        "procexp.exe",
        "procexp64.exe",
        "procmon.exe",
        "procmon64.exe",
        "tcpview.exe",
        "wireshark.exe",
        "fiddler.exe",
        "httpdebugger.exe",
        "cheatengine-i386.exe",
        "cheatengine-x86_64.exe",
        "frida-server.exe",
        "frida-helper-32.exe",
        "frida-helper-64.exe",
    ];

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == winapi::um::handleapi::INVALID_HANDLE_VALUE {
            return false;
        }

        let mut pe: PROCESSENTRY32 = std::mem::zeroed();
        pe.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

        if Process32First(snapshot, &mut pe) == FALSE {
            CloseHandle(snapshot);
            return false;
        }

        loop {
            let process_name = CStr::from_ptr(pe.szExeFile.as_ptr() as *const i8);
            if let Ok(name) = process_name.to_str() {
                let name_lower = name.to_lowercase();
                for tool in &blacklist {
                    if name_lower == *tool {
                        CloseHandle(snapshot);
                        return true;
                    }
                }
            }

            if Process32Next(snapshot, &mut pe) == FALSE {
                break;
            }
        }

        CloseHandle(snapshot);
    }

    false
}

#[cfg(target_os = "windows")]
pub fn check_vm() -> bool {
    use winapi::um::sysinfoapi::{GetSystemInfo, SYSTEM_INFO};

    unsafe {
        let mut system_info: SYSTEM_INFO = std::mem::zeroed();
        GetSystemInfo(&mut system_info);

        // Check for common VM indicators
        // Low CPU count can indicate VM
        if system_info.dwNumberOfProcessors < 2 {
            return true;
        }
    }

    // Check for VM-related files
    let vm_paths = [
        "C:\\windows\\system32\\drivers\\vmmouse.sys",
        "C:\\windows\\system32\\drivers\\vmhgfs.sys",
        "C:\\windows\\system32\\drivers\\VBoxMouse.sys",
        "C:\\windows\\system32\\drivers\\VBoxGuest.sys",
        "C:\\windows\\system32\\vboxdisp.dll",
        "C:\\windows\\system32\\vboxhook.dll",
    ];

    for path in &vm_paths {
        if std::path::Path::new(path).exists() {
            return true;
        }
    }

    false
}

#[cfg(target_os = "windows")]
pub fn should_execute() -> bool {
    // Return false if any evasion check fails (detected)
    if check_debugger() {
        return false;
    }

    if check_analysis_tools() {
        return false;
    }

    if check_vm() {
        return false;
    }

    true
}

#[cfg(not(target_os = "windows"))]
pub fn should_execute() -> bool {
    true
}

#[cfg(not(target_os = "windows"))]
pub fn check_debugger() -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub fn check_analysis_tools() -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub fn check_vm() -> bool {
    false
}
