//! Trace capability handling
//!
//! This module provides types for managing trace access capabilities.
//! Capabilities are used to authorize trace operations and are scoped
//! to specific domain IDs.

use kernel_api::cap::Handle;

/// Trace capability for authorizing trace operations
///
/// A trace capability represents the right to perform trace operations
/// on a specific domain's trace buffer. Capabilities are issued by the
/// kernel's trace service and validated on every operation.
#[derive(Debug, Clone)]
pub struct TraceCapability {
    /// Handle to the capability in the kernel's capability table
    pub handle: Handle,
    /// Domain ID this capability is scoped to
    pub domain_id: u64,
    /// Rights mask (TRACE_RIGHT_READ, TRACE_RIGHT_WRITE, TRACE_RIGHT_ADMIN)
    pub rights: u32,
}

// Trace rights constants (must match kernel_api)
/// Right to read from trace buffer
pub const TRACE_RIGHT_READ: u32 = 0x01;
/// Right to write to trace buffer
pub const TRACE_RIGHT_WRITE: u32 = 0x02;
/// Right to administer trace buffer (create, destroy)
pub const TRACE_RIGHT_ADMIN: u32 = 0x04;
/// All rights
pub const TRACE_RIGHT_ALL: u32 = TRACE_RIGHT_READ | TRACE_RIGHT_WRITE | TRACE_RIGHT_ADMIN;

impl TraceCapability {
    /// Create a new trace capability
    pub fn new(handle: Handle, domain_id: u64, rights: u32) -> Self {
        Self {
            handle,
            domain_id,
            rights,
        }
    }

    /// Check if this capability has read rights
    pub fn can_read(&self) -> bool {
        (self.rights & TRACE_RIGHT_READ) != 0
    }

    /// Check if this capability has write rights
    pub fn can_write(&self) -> bool {
        (self.rights & TRACE_RIGHT_WRITE) != 0
    }

    /// Check if this capability has admin rights
    pub fn is_admin(&self) -> bool {
        (self.rights & TRACE_RIGHT_ADMIN) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_capability_can_read() {
        let cap = TraceCapability::new(Handle::INVALID, 1, TRACE_RIGHT_READ);
        assert!(cap.can_read());
        assert!(!cap.can_write());
        assert!(!cap.is_admin());
    }

    #[test]
    fn trace_capability_can_write() {
        let cap = TraceCapability::new(Handle::INVALID, 1, TRACE_RIGHT_WRITE);
        assert!(!cap.can_read());
        assert!(cap.can_write());
        assert!(!cap.is_admin());
    }

    #[test]
    fn trace_capability_is_admin() {
        let cap = TraceCapability::new(Handle::INVALID, 1, TRACE_RIGHT_ADMIN);
        assert!(!cap.can_read());
        assert!(!cap.can_write());
        assert!(cap.is_admin());
    }

    #[test]
    fn trace_capability_all_rights() {
        let cap = TraceCapability::new(Handle::INVALID, 1, TRACE_RIGHT_ALL);
        assert!(cap.can_read());
        assert!(cap.can_write());
        assert!(cap.is_admin());
    }
}
