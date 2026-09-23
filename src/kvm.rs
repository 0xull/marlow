use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

use crate::error::{Error, Result};

/// KVM's ioctl "magic" byte.
const KVMIO: u64 = 0xAE;

/// Rebuild of the kernel's _IO(type, nr) macro for requests that carry
/// no argument payload.
const fn io(nr: u64) -> u64 {
    (KVMIO << 8) | nr
}

/// `KVM_GET_API_VERSION` = `_IO(KVMIO, 0x00)` = 0xAE00.
const KVM_GET_API_VERSION: u64 = io(0x00);
/// `KVM_CREATE_VM` = `_IO(KVMIO, 0x01)` = 0xAE01.
const KVM_CREATE_VM: u64 = io(0x01);

/// The supported API version this VMM is written against.
const KVM_API_VERSION: i32 = 12;

/// x86_64 default machine type for `KVM_CREATE_VM`.
const DEFAULT_MACHINE_TYPE: u64 = 0;

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
        let ret = unsafe {
            libc::ioctl(self.fd.as_raw_fd(), KVM_GET_API_VERSION as _, 0_u64)
        };
        if ret < 0 {
            return Err(Error::GetApiVersion(std::io::Error::last_os_error()));
        }
        Ok(ret)
    }

    /// Issues `KVM_CREATE_VM` and returns an owning `VM` handle.
    pub fn create_vm(&self) -> Result<Vm> {
        let raw = unsafe {
            libc::ioctl(self.fd.as_raw_fd(), KVM_CREATE_VM as _, DEFAULT_MACHINE_TYPE)
        };
        if raw < 0 {
            return Err(Error::CreateVM(std::io::Error::last_os_error()))
        }
        let fd = unsafe { OwnedFd::from_raw_fd(raw) };
        Ok(Vm { fd })
    }
}

pub struct Vm {
    fd: OwnedFd,
}

impl Vm {
    /// Borrows the raw VM descriptor for issuing ioctl commands against it.
    pub fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}
