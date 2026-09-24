use std::io;

use crate::{Error, Result};

/// x86_64 base page size.
pub const PAGE_SIZE: usize = 4096;

/// Owns one `mmap` region backing a slice of the guest's
/// physical address space.
pub struct GuestMemory {
    ptr: *mut u8,
    size: usize,
    guest_physical_addr: u64,
}

impl GuestMemory {
    /// Maps `size` bytes of anonymous, zero-filled, demand-paged host memory to
    /// back the guest physical range starting at `guest_physical_addr`.
    pub fn new(guest_physical_addr: u64, size: usize) -> Result<Self> {
        assert!(
            size > 0 && size % PAGE_SIZE == 0,
            "size must be a positive page multiple"
        );
        assert!(
            guest_physical_addr % PAGE_SIZE as u64 == 0,
            "guest base must be page aligned"
        );

        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE,
                -1,
                0,
            )
        };

        if ptr == libc::MAP_FAILED {
            return Err(Error::Mmap(io::Error::last_os_error()));
        }

        Ok(GuestMemory {
            ptr: ptr as *mut u8,
            size,
            guest_physical_addr,
        })
    }

    /// Host virtual base of the mapping.
    pub fn host_addr(&self) -> u64 {
        self.ptr as u64
    }

    /// Guest physical base this region backs.
    pub fn guest_physical_addr(&self) -> u64 {
        self.guest_physical_addr
    }

    /// Size of the region in bytes.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Copies `data` into the region at `offset`.
    pub fn write(&self, offset: usize, data: &[u8]) -> Result<()> {
        check_bounds(offset, data.len(), self.size)?;
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), self.ptr.add(offset), data.len());
        }
        Ok(())
    }

    /// Copies `out.len()` bytes from the region at `offset` into `out`.
    pub fn read(&self, offset: usize, out: &mut [u8]) -> Result<()> {
        check_bounds(offset, out.len(), self.size)?;
        unsafe {
            std::ptr::copy_nonoverlapping(self.ptr.add(offset), out.as_mut_ptr(), out.len());
        }
        Ok(())
    }
}

impl Drop for GuestMemory {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.ptr as *mut libc::c_void, self.size);
        }
    }
}

fn check_bounds(offset: usize, len: usize, size: usize) -> Result<()> {
    match offset.checked_add(len) {
        Some(end) if end <= size => Ok(()),
        _ => Err(Error::OutOfBound { offset, len, size }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_bounds_accepts_in_range_and_rejects_overflow() {
        assert!(check_bounds(0, 16, 16).is_ok());
        assert!(check_bounds(10, 6, 16).is_ok());
        assert!(check_bounds(10, 7, 16).is_err());
        assert!(check_bounds(usize::MAX, 1, 16).is_err());
    }
}
