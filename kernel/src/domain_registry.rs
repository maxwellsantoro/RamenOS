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

    /// Get the domain name up to the first null byte.
    ///
    /// Malformed UTF-8 supplied through the public raw name returns an empty
    /// string. Use [`Self::try_name_str`] to distinguish it from an empty name.
    pub fn name_str(&self) -> &str {
        self.try_name_str().unwrap_or("")
    }

    /// Validate the raw name prefix up to the first null byte as UTF-8.
    ///
    /// Bytes after the first null are not part of the name. A valid full array
    /// without a null terminator is also accepted.
    pub fn try_name_str(&self) -> Result<&str, core::str::Utf8Error> {
        let len = self.name.iter().position(|&b| b == 0).unwrap_or(32);
        core::str::from_utf8(&self.name[..len])
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
        // Reserve the null terminator without splitting a UTF-8 character.
        let mut len = core::cmp::min(name_bytes_slice.len(), 31);
        while !name.is_char_boundary(len) {
            len -= 1;
        }
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

    #[test]
    fn review_domain_name_multibyte_truncation_preserves_complete_characters() {
        for character in ["é", "€", "🦀"] {
            let mut bytes = [b'a'; 34];
            let end = 30 + character.len();
            bytes[30..end].copy_from_slice(character.as_bytes());
            let name = core::str::from_utf8(&bytes[..end]).unwrap();
            let mut registry = DomainRegistry::new();
            registry.register(1, name).unwrap();
            let info = registry.get(1).unwrap();
            let stored = &info.name;
            let len = stored.iter().position(|&byte| byte == 0).unwrap_or(32);
            // Inspect bytes with the checked parser: the baseline getter would
            // create an invalid &str for these inputs and must not be invoked.
            assert_eq!(core::str::from_utf8(&stored[..len]).unwrap(), &name[..30]);
            assert!(stored[30..].iter().all(|&byte| byte == 0));
            assert_eq!(info.try_name_str().unwrap(), &name[..30]);
            assert_eq!(info.name_str(), &name[..30]);
        }
    }

    #[test]
    fn review_domain_name_exact_multibyte_boundaries_remain_intact() {
        for character in ["é", "€", "🦀"] {
            let mut bytes = [b'a'; 32];
            let start = 31 - character.len();
            bytes[start..31].copy_from_slice(character.as_bytes());
            let name = core::str::from_utf8(&bytes).unwrap();
            let mut registry = DomainRegistry::new();
            registry.register(1, name).unwrap();
            let info = registry.get(1).unwrap();
            assert_eq!(info.try_name_str().unwrap(), &name[..31]);
            assert_eq!(info.name_str(), &name[..31]);
            assert_eq!(info.name[31], 0);
        }
    }

    #[test]
    fn review_domain_name_raw_constructor_reports_invalid_utf8() {
        for prefix in [&[0xff][..], &[b'a', 0x80][..], &[0xe2, 0x82][..]] {
            let mut bytes = [0; 32];
            bytes[..prefix.len()].copy_from_slice(prefix);
            let info = DomainInfo::new(1, bytes);
            assert!(core::str::from_utf8(&info.name[..prefix.len()]).is_err());
            assert!(info.try_name_str().is_err());
            assert_eq!(info.name_str(), "");
        }
    }

    #[test]
    fn review_domain_name_public_mutation_reports_invalid_utf8() {
        let mut registry = DomainRegistry::new();
        registry.register(1, "valid").unwrap();
        let info = registry.get_mut(1).unwrap();
        info.name[0] = 0xff;
        assert!(core::str::from_utf8(&info.name[..5]).is_err());
        assert!(info.try_name_str().is_err());
        assert_eq!(info.name_str(), "");
        info.name = [0; 32];
        info.name[..8].copy_from_slice(b"repaired");
        assert_eq!(info.try_name_str().unwrap(), "repaired");
        assert_eq!(info.name_str(), "repaired");
    }

    #[test]
    fn review_domain_name_first_null_ignores_invalid_tail_and_preserves_empty() {
        let mut bytes = [0xff; 32];
        bytes[..5].copy_from_slice(b"valid");
        bytes[5] = 0;
        let info = DomainInfo::new(1, bytes);
        assert_eq!(info.try_name_str().unwrap(), "valid");
        assert_eq!(info.name_str(), "valid");

        bytes[0] = 0;
        let empty = DomainInfo::new(2, bytes);
        assert_eq!(empty.try_name_str().unwrap(), "");
        assert_eq!(empty.name_str(), "");

        let mut registry = DomainRegistry::new();
        registry.register(1, "before\0after").unwrap();
        assert_eq!(registry.get(1).unwrap().name_str(), "before");
    }

    #[test]
    fn review_domain_name_full_raw_array_without_null_remains_valid() {
        let info = DomainInfo::new(1, [b'a'; 32]);
        assert_eq!(
            info.try_name_str().unwrap(),
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert_eq!(info.name_str(), "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    }

    #[test]
    fn review_domain_name_denial_preserves_next_registration() {
        let mut registry = DomainRegistry::new();
        registry.register(1, "original").unwrap();
        assert_eq!(
            registry.register(1, "replacement"),
            Err(DomainRegistryError::DomainAlreadyRegistered { id: 1 })
        );
        assert_eq!(
            registry.register(MAX_DOMAINS as u64, "out-of-range"),
            Err(DomainRegistryError::InvalidDomainId {
                id: MAX_DOMAINS as u64,
            })
        );
        registry.register(2, "next-é").unwrap();
        assert_eq!(registry.get(1).unwrap().name_str(), "original");
        assert_eq!(registry.get(2).unwrap().name_str(), "next-é");
    }
}
