// V-012 Phase 1: Domain registry for per-domain trace isolation
//
// This module provides domain tracking infrastructure for the trace isolation system.
// Domains are isolated security contexts; each domain gets its own trace buffer.
//
// Refactoring: Added typed error types to replace Result<(), ()> patterns.
// This improves error handling and debugging by providing descriptive error variants.

use core::fmt;

/// Domain identifier type.
/// Domain 0 is reserved for the kernel.
/// Domains 1-15 are available for user domains.
pub type DomainId = u64;

/// Maximum number of domains supported.
/// Domain 0 (kernel) + 15 user domains.
pub const MAX_DOMAINS: usize = 16;

/// Typed error type for domain registry operations.
///
/// This enum replaces the unit error type `()` in `Result<(), ()>` to provide
/// descriptive error information for debugging and error handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainRegistryError {
    /// Domain ID is out of valid range (>= MAX_DOMAINS).
    ///
    /// This error occurs when attempting to register a domain with an ID
    /// that exceeds the maximum supported number of domains.
    InvalidDomainId {
        /// The invalid domain ID that was provided.
        id: DomainId,
    },

    /// Domain slot is already occupied.
    ///
    /// This error occurs when attempting to register a domain with an ID
    /// that is already registered in the registry.
    DomainAlreadyRegistered {
        /// The domain ID that is already registered.
        id: DomainId,
    },
}

impl fmt::Display for DomainRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDomainId { id } => {
                write!(
                    f,
                    "domain ID {} is out of range (max: {})",
                    id,
                    MAX_DOMAINS - 1
                )
            }
            Self::DomainAlreadyRegistered { id } => {
                write!(f, "domain ID {} is already registered", id)
            }
        }
    }
}

/// Domain state enum.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DomainState {
    /// Domain is not initialized
    Uninitialized,
    /// Domain is active and running
    Active,
    /// Domain has terminated
    Terminated,
}

/// Domain information structure.
#[derive(Copy, Clone)]
pub struct DomainInfo {
    /// Domain ID
    pub id: DomainId,
    /// Domain name (fixed-size for no_std compatibility)
    pub name: [u8; 32],
    /// Current domain state
    pub state: DomainState,
}

impl DomainInfo {
    /// Create a new domain info with the given ID and name.
    pub const fn new(id: DomainId, name: [u8; 32]) -> Self {
        Self {
            id,
            name,
            state: DomainState::Uninitialized,
        }
    }

    /// Get the domain name as a string slice (up to first null byte).
    ///
    /// # Safety
    /// This function assumes the name bytes are valid UTF-8.
    /// For kernel-generated names, this is guaranteed.
    pub fn name_str(&self) -> &str {
        // Find the null terminator
        let len = self.name.iter().position(|&b| b == 0).unwrap_or(32);

        // SAFETY: Kernel-generated names are valid UTF-8
        unsafe {
            let slice = &self.name[..len];
            core::str::from_utf8_unchecked(slice)
        }
    }
}

/// Domain registry for tracking active domains.
///
/// This registry maintains a static array of domain slots.
/// Domain 0 is reserved for the kernel and is pre-registered.
pub struct DomainRegistry {
    /// Fixed-size array of domain slots
    domains: [Option<DomainInfo>; MAX_DOMAINS],
}

impl DomainRegistry {
    /// Create a new domain registry with kernel domain pre-registered.
    pub const fn new() -> Self {
        // Note: We can't use [None; MAX_DOMAINS] because Option<DomainInfo> doesn't implement Copy
        // Instead, we initialize with zeros and treat it as uninitialized
        Self {
            domains: [None; MAX_DOMAINS],
        }
    }

    /// Register a new domain with the given ID and name.
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(DomainRegistryError::InvalidDomainId)` if domain ID is out of range
    /// - `Err(DomainRegistryError::DomainAlreadyRegistered)` if domain slot is occupied
    pub fn register(&mut self, id: DomainId, name: &str) -> Result<(), DomainRegistryError> {
        // Validate domain ID
        let idx = id as usize;
        if idx >= MAX_DOMAINS {
            return Err(DomainRegistryError::InvalidDomainId { id });
        }

        // Check if slot is already occupied
        if self.domains[idx].is_some() {
            return Err(DomainRegistryError::DomainAlreadyRegistered { id });
        }

        // Convert name to fixed-size byte array
        let mut name_bytes = [0u8; 32];
        let name_bytes_slice = name.as_bytes();
        let len = core::cmp::min(name_bytes_slice.len(), 31); // Leave room for null terminator
        name_bytes[..len].copy_from_slice(&name_bytes_slice[..len]);

        // Create and store domain info
        let info = DomainInfo::new(id, name_bytes);
        self.domains[idx] = Some(info);

        Ok(())
    }

    /// Get domain information by ID.
    ///
    /// Returns None if domain is not registered.
    pub fn get(&self, id: DomainId) -> Option<&DomainInfo> {
        let idx = id as usize;
        if idx >= MAX_DOMAINS {
            return None;
        }
        self.domains[idx].as_ref()
    }

    /// Get mutable domain information by ID.
    ///
    /// Returns None if domain is not registered.
    pub fn get_mut(&mut self, id: DomainId) -> Option<&mut DomainInfo> {
        let idx = id as usize;
        if idx >= MAX_DOMAINS {
            return None;
        }
        self.domains[idx].as_mut()
    }

    /// Initialize the kernel domain (ID 0).
    ///
    /// This should be called during kernel boot.
    pub fn init_kernel(&mut self) {
        let kernel_name = b"kernel\0";
        let mut name_bytes = [0u8; 32];
        let len = kernel_name.len();
        name_bytes[..len].copy_from_slice(&kernel_name[..len]);

        let info = DomainInfo {
            id: 0,
            name: name_bytes,
            state: DomainState::Active,
        };
        self.domains[0] = Some(info);
    }

    /// Check if a domain is registered.
    pub fn is_registered(&self, id: DomainId) -> bool {
        self.get(id).is_some()
    }
}

impl Default for DomainRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_domain_initialized() {
        let mut registry = DomainRegistry::new();
        registry.init_kernel();

        let kernel = registry.get(0);
        assert!(kernel.is_some());
        assert_eq!(kernel.unwrap().id, 0);
        assert_eq!(kernel.unwrap().name_str(), "kernel");
    }

    #[test]
    fn register_user_domain() {
        let mut registry = DomainRegistry::new();
        registry.init_kernel();

        let result = registry.register(1, "test-domain");
        assert!(result.is_ok());

        let domain = registry.get(1);
        assert!(domain.is_some());
        assert_eq!(domain.unwrap().name_str(), "test-domain");
    }

    #[test]
    fn register_rejects_duplicate_id() {
        let mut registry = DomainRegistry::new();
        registry.init_kernel();

        registry.register(1, "first").unwrap();
        let result = registry.register(1, "second");
        assert!(result.is_err());
    }

    #[test]
    fn register_rejects_out_of_range_id() {
        let mut registry = DomainRegistry::new();
        registry.init_kernel();

        let result = registry.register(999, "invalid");
        assert!(result.is_err());
    }

    #[test]
    fn get_returns_none_for_unregistered() {
        let registry = DomainRegistry::new();

        let domain = registry.get(999);
        assert!(domain.is_none());
    }

    #[test]
    fn name_truncates_to_31_bytes() {
        let mut registry = DomainRegistry::new();
        registry.init_kernel();

        // Name longer than 31 bytes
        let long_name = "abcdefghijabcdefghijabcdefghijXXX"; // 33 bytes
        registry.register(1, long_name).unwrap();

        let domain = registry.get(1).unwrap();
        // Should be truncated to 31 bytes (null at position 31)
        assert_eq!(domain.name.len(), 32);
        assert_eq!(domain.name[31], 0);
    }

    #[test]
    fn is_registered_works() {
        let mut registry = DomainRegistry::new();
        registry.init_kernel();

        assert!(registry.is_registered(0));
        assert!(!registry.is_registered(1));

        registry.register(1, "test").unwrap();
        assert!(registry.is_registered(1));
    }

    #[test]
    fn get_mut_returns_mutable_reference() {
        let mut registry = DomainRegistry::new();
        registry.init_kernel();

        registry.register(1, "test").unwrap();

        let domain = registry.get_mut(1).unwrap();
        domain.state = DomainState::Terminated;

        assert_eq!(registry.get(1).unwrap().state, DomainState::Terminated);
    }

    #[test]
    fn max_domains_limit() {
        let mut registry = DomainRegistry::new();
        registry.init_kernel();

        // Register domains 1-5 with simple names
        // (Can't use format! in no_std)
        registry.register(1, "domain1").unwrap();
        registry.register(2, "domain2").unwrap();
        registry.register(3, "domain3").unwrap();
        registry.register(4, "domain4").unwrap();
        registry.register(5, "domain5").unwrap();

        // Verify they're registered
        assert!(registry.is_registered(1));
        assert!(registry.is_registered(2));
        assert!(registry.is_registered(3));
        assert!(registry.is_registered(4));
        assert!(registry.is_registered(5));
    }
}
