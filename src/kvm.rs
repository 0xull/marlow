use std::{
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
};

use crate::{
    error::{Error, Result},
    memory::GuestMemory,
};

/// KVM's ioctl "magic" byte.
const KVMIO: u64 = 0xAE;

// Field widths and shifts matching the kernel's ioctl request encoding
// (asm-generic).
const IOC_TYPESHIFT: u64 = 8;
const IOC_DIRSHIFT: u64 = 30;
const IOC_SIZESHIFT: u64 = 16;
const IOC_NONE: u64 = 0;
const IOC_WRITE: u64 = 1;

/// Rebuild of the kernel's `_IOC(dir, type, nr, size)` macro. Direction is
/// stated from userspace's point of view.
const fn ioc(dir: u64, nr: u64, size: u64) -> u64 {
    (dir << IOC_DIRSHIFT) | (KVMIO << IOC_TYPESHIFT) | (size << IOC_SIZESHIFT) | nr
}

/// `_IO(KVMIO, nr)`, no payload, hence no direction and size.
const fn io(nr: u64) -> u64 {
    ioc(IOC_NONE, nr, 0)
}

/// `_IO(KVMIO, nr, size)`, userspace writes payload of `size` bytes in.
const fn iow(nr: u64, size: u64) -> u64 {
    ioc(IOC_WRITE, nr, size)
}

/// `KVM_GET_API_VERSION` = `_IO(KVMIO, 0x00)` = 0xAE00.
const KVM_GET_API_VERSION: u64 = io(0x00);
/// `KVM_CREATE_VM` = `_IO(KVMIO, 0x01)` = 0xAE01.
const KVM_CREATE_VM: u64 = io(0x01);
/// `KVM_SET_USER_MEMORY_REGION` = `_IOW(KVMIO, 0x46, region)` = 0x4020AE46.
const KVM_SET_USER_MEMORY_REGION: u64 =
    iow(0x46, std::mem::size_of::<KvmUserspaceMemoryRegion>() as u64);

/// The supported API version this VMM is written against.
const KVM_API_VERSION: i32 = 12;

/// x86_64 default machine type for `KVM_CREATE_VM`.
const DEFAULT_MACHINE_TYPE: u64 = 0;

/// Mirror of `struct kvm_userspace_memory_region`.
#[repr(C)]
struct KvmUserspaceMemoryRegion {
    slot: u32,
    flags: u32,
    guest_phys_addr: u64,
    memory_size: u64,
    userspace_addr: u64,
}

/// Owns the descriptor returned by `open("/dev/kvm")`.
pub struct Kvm {
    fd: OwnedFd,
}

impl Kvm {
    /// Opens `/dev/kvm`, verifies the API version and returns a handle.
    pub fn new() -> Result<Self> {
        let raw: RawFd =
            unsafe { libc::open(c"/dev/kvm".as_ptr(), libc::O_RDWR | libc::O_CLOEXEC) };
        if raw < 0 {
            return Err(Error::OpenKvm(std::io::Error::last_os_error()));
        }

        let fd = unsafe { OwnedFd::from_raw_fd(raw) };
        let kvm = Kvm { fd };

        let version = kvm.api_version()?;
        if version != KVM_API_VERSION {
            return Err(Error::ApiVersionMismatch {
                expected: KVM_API_VERSION,
                found: version,
            });
        }
        Ok(kvm)
    }

    /// Issues `KVM_GET_API_VERSION` on the system descriptor.
    fn api_version(&self) -> Result<i32> {
        let ret = unsafe { libc::ioctl(self.fd.as_raw_fd(), KVM_GET_API_VERSION as _, 0_u64) };
        if ret < 0 {
            return Err(Error::GetApiVersion(std::io::Error::last_os_error()));
        }
        Ok(ret)
    }

    /// Issues `KVM_CREATE_VM` and returns an owning `VM` handle.
    pub fn create_vm(&self) -> Result<Vm> {
        let raw = unsafe {
            libc::ioctl(
                self.fd.as_raw_fd(),
                KVM_CREATE_VM as _,
                DEFAULT_MACHINE_TYPE,
            )
        };
        if raw < 0 {
            return Err(Error::CreateVM(std::io::Error::last_os_error()));
        }
        let fd = unsafe { OwnedFd::from_raw_fd(raw) };
        Ok(Vm {
            fd,
            regions: Vec::new(),
        })
    }
}

/// Owns a KVM's VM descriptor and the guest memory regions
/// registered against it.
///
/// Field order is load-bearing. `regions` should unmapped only after VM fd's
/// (kernel) references to the host mappings are gone.
pub struct Vm {
    fd: OwnedFd,
    regions: Vec<GuestMemory>,
}

impl Vm {
    pub fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }

    /// Consumes and registers `memory` as memslot `slot`, its unmap is ordered
    /// after VM descriptor's close.
    pub fn set_user_memory_region(&mut self, slot: u32, memory: GuestMemory) -> Result<()> {
        let region = KvmUserspaceMemoryRegion {
            slot,
            flags: 0,
            guest_phys_addr: memory.guest_physical_addr(),
            memory_size: memory.size() as u64,
            userspace_addr: memory.host_addr(),
        };

        let ret = unsafe {
            libc::ioctl(
                self.fd.as_raw_fd(),
                KVM_SET_USER_MEMORY_REGION as _,
                &region as *const KvmUserspaceMemoryRegion,
            )
        };
        if ret < 0 {
            return Err(Error::SetUserMemoryRegion(io::Error::last_os_error()));
        }
        self.regions.push(memory);
        Ok(())
    }

    /// Borrows a registered region by its position in registration order.
    pub fn memory(&self, index: usize) -> Option<&GuestMemory> {
        self.regions.get(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_numbers_match_kernel_encoding() {
        assert_eq!(KVM_GET_API_VERSION, 0xAE00);
        assert_eq!(KVM_CREATE_VM, 0xAE01);
        assert_eq!(KVM_SET_USER_MEMORY_REGION, 0x4020AE46);
    }

    #[test]
    fn memory_region_struct_is_32_bytes() {
        assert_eq!(std::mem::size_of::<KvmUserspaceMemoryRegion>(), 32);
    }
}
