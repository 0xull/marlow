use std::io;
use std::fmt;

/// A custom result type for marlow's KVM layer.
pub type Result<T> = std::result::Result<T, Error>;

/// Every fallible operation in the KVM system as used by marlow.
#[derive(Debug)]
pub enum Error {
    /// `open("/dev/kvm")` failed. The wrapped error carries `errno`.
    OpenKvm(io::Error),
    /// The `KVM_GET_API_VERSION` ioctl itself failed.
    GetApiVersion(io::Error),
    /// KVM answered, but with a version this VMM was not written against.
    ApiVersionMismatch { expected: i32, found: i32},
    /// The `KVM_CREATE_VM` ioctl failed
    CreateVM(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::OpenKvm(err) => write!(f, "opening /dev/kvm failed: {err}"),
            Error::GetApiVersion(err) => write!(f, "KVM_GET_API_VERSION failed: {err}"),
            Error::ApiVersionMismatch{ expected, found } => {
                write!(f, "unsupported KVM API version: expected {expected}, found {found}")
            },
            Error::CreateVM(err) => write!(f, "KVM_CREATE_VM failed: {err}")
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::OpenKvm(err) | Error::GetApiVersion(err) | Error::CreateVM(err) => Some(err),
            Error::ApiVersionMismatch { .. } => None,
        }
    }
}
