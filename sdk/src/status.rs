//! Status codes returned by WASM imports
//!
//! These codes match the kernel-side status constants and are used
//! to indicate success or failure of harness operations.

#![allow(dead_code)]

/// Status codes returned by harness imports
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Operation completed successfully
    Ok = 0,
    /// Invalid capability handle provided
    InvalidCapability = 1,
    /// Capability does not have required rights
    PermissionDenied = 2,
    /// Invalid argument provided
    InvalidArgument = 3,
    /// Operation would block (for non-blocking calls)
    WouldBlock = 4,
    /// I/O error occurred
    IoError = 5,
    /// Buffer too small for result
    BufferTooSmall = 6,
    /// Resource not found
    NotFound = 7,
    /// Unknown error
    Unknown = -1,
}

impl Status {
    /// Convert from raw i32 status code
    #[inline]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Ok,
            1 => Self::InvalidCapability,
            2 => Self::PermissionDenied,
            3 => Self::InvalidArgument,
            4 => Self::WouldBlock,
            5 => Self::IoError,
            6 => Self::BufferTooSmall,
            7 => Self::NotFound,
            _ => Self::Unknown,
        }
    }

    /// Convert to raw i32 for comparison
    #[inline]
    pub const fn to_raw(self) -> i32 {
        self as i32
    }

    /// Check if status indicates success
    #[inline]
    pub const fn is_ok(self) -> bool {
        matches!(self, Self::Ok)
    }

    /// Check if status indicates an error
    #[inline]
    pub const fn is_err(self) -> bool {
        !self.is_ok()
    }
}

impl From<i32> for Status {
    fn from(raw: i32) -> Self {
        Self::from_raw(raw)
    }
}

impl From<Status> for i32 {
    fn from(status: Status) -> Self {
        status.to_raw()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_status_is_zero() {
        assert_eq!(Status::Ok.to_raw(), 0);
        assert!(Status::Ok.is_ok());
        assert!(!Status::Ok.is_err());
    }

    #[test]
    fn from_raw_roundtrip() {
        for raw in 0..=7 {
            let status = Status::from_raw(raw);
            assert_eq!(status.to_raw(), raw);
        }
    }

    #[test]
    fn unknown_for_invalid_raw() {
        let status = Status::from_raw(999);
        assert_eq!(status, Status::Unknown);
    }

    #[test]
    fn error_statuses_are_errors() {
        assert!(Status::InvalidCapability.is_err());
        assert!(Status::PermissionDenied.is_err());
        assert!(Status::IoError.is_err());
    }
}
