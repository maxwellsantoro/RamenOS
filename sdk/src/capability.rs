//! Capability handle types for WASM modules
//!
//! Capability handles are 64-bit values that authorize access to
//! RamenOS resources. They are provided by the runner at initialization
//! and must be passed as the first argument to all harness imports.

#![allow(dead_code)]

/// A capability handle authorizing access to a RamenOS resource
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapabilityHandle(u64);

impl CapabilityHandle {
    /// Invalid capability handle (0)
    pub const INVALID: Self = Self(0);

    /// Create a capability handle from a raw u64
    #[inline]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Get the raw u64 value for passing to WASM imports
    #[inline]
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Check if this handle is valid (non-zero)
    #[inline]
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }

    /// Check if this handle is invalid (zero)
    #[inline]
    pub const fn is_invalid(self) -> bool {
        self.0 == 0
    }
}

impl Default for CapabilityHandle {
    fn default() -> Self {
        Self::INVALID
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_handle_is_zero() {
        assert_eq!(CapabilityHandle::INVALID.raw(), 0);
        assert!(CapabilityHandle::INVALID.is_invalid());
        assert!(!CapabilityHandle::INVALID.is_valid());
    }

    #[test]
    fn from_raw_preserves_value() {
        let handle = CapabilityHandle::from_raw(0x1234_5678_9ABC_DEF0);
        assert_eq!(handle.raw(), 0x1234_5678_9ABC_DEF0);
    }

    #[test]
    fn default_is_invalid() {
        let handle: CapabilityHandle = Default::default();
        assert!(handle.is_invalid());
    }

    #[test]
    fn copy_works() {
        let handle = CapabilityHandle::from_raw(42);
        let copied = handle;
        assert_eq!(handle, copied);
    }
}
