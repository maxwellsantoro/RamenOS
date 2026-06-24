//! Address space table for tracking page table roots per domain.
//!
//! This module provides a simple lookup table that maps domain IDs to their
//! page table root physical addresses (CR3 on x86_64, TTBR0 on aarch64).
//! This enables the kernel to program the MMU when switching between domains.

use crate::domain_registry::{DomainId, MAX_DOMAINS};
use crate::mm::address::PhysAddr;

/// Address space table for tracking page table roots per domain.
///
/// This table maintains a mapping from domain IDs to their page table root
/// physical addresses. Domain 0 is the kernel domain and is special-cased.
/// Uninitialized domains return None for their page table root.
///
/// # Design Principles
///
/// - **Static allocation:** No heap usage (V-010 constraint)
/// - **Bounds checking:** Panics on invalid domain_id
/// - **Fail-closed:** None returned for uninitialized domains
/// - **Kernel-first:** Domain 0 is the kernel domain
pub struct AddressSpaceTable {
    /// Page table root for each domain.
    /// None = domain not initialized.
    roots: [Option<PhysAddr>; MAX_DOMAINS],
}

impl AddressSpaceTable {
    /// Create a new address space table with all roots uninitialized.
    ///
    /// All domains start with None page table roots, indicating they are
    /// not yet initialized. The kernel domain (0) should be initialized
    /// via `init_kernel()` after the kernel's page tables are set up.
    pub const fn new() -> Self {
        Self {
            roots: [None; MAX_DOMAINS],
        }
    }

    /// Set the page table root for a domain.
    ///
    /// # Panics
    ///
    /// Panics if `domain_id` is out of range (>= MAX_DOMAINS).
    pub fn set_root(&mut self, domain_id: DomainId, root: PhysAddr) {
        let idx = domain_id as usize;
        if idx >= MAX_DOMAINS {
            panic!(
                "AddressSpaceTable: invalid domain_id {} (max: {})",
                domain_id, MAX_DOMAINS
            );
        }
        self.roots[idx] = Some(root);
    }

    /// Get the page table root for a domain.
    ///
    /// Returns None if the domain has not been initialized.
    ///
    /// # Panics
    ///
    /// Panics if `domain_id` is out of range (>= MAX_DOMAINS).
    pub fn get_root(&self, domain_id: DomainId) -> Option<PhysAddr> {
        let idx = domain_id as usize;
        if idx >= MAX_DOMAINS {
            panic!(
                "AddressSpaceTable: invalid domain_id {} (max: {})",
                domain_id, MAX_DOMAINS
            );
        }
        self.roots[idx]
    }

    /// Initialize the kernel's page table root.
    ///
    /// Domain 0 is the kernel domain. This is a convenience method that
    /// sets the page table root for domain 0.
    ///
    /// # Panics
    ///
    /// Panics if the kernel domain (0) is already initialized.
    pub fn init_kernel(&mut self, root: PhysAddr) {
        const KERNEL_DOMAIN_ID: DomainId = 0;
        if self.roots[KERNEL_DOMAIN_ID as usize].is_some() {
            panic!("AddressSpaceTable: kernel domain (0) already initialized");
        }
        self.set_root(KERNEL_DOMAIN_ID, root);
    }

    /// Clear a domain's page table root.
    ///
    /// This marks the domain as uninitialized.
    ///
    /// # Panics
    ///
    /// Panics if `domain_id` is out of range (>= MAX_DOMAINS).
    pub fn clear_root(&mut self, domain_id: DomainId) {
        let idx = domain_id as usize;
        if idx >= MAX_DOMAINS {
            panic!(
                "AddressSpaceTable: invalid domain_id {} (max: {})",
                domain_id, MAX_DOMAINS
            );
        }
        self.roots[idx] = None;
    }
}

impl Default for AddressSpaceTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_space_table_initializes_kernel_root() {
        let mut table = AddressSpaceTable::new();
        let root = unsafe { PhysAddr::new(0x1000) };

        // Initially, kernel root should be None
        assert_eq!(table.get_root(0), None);

        // Initialize kernel root
        table.init_kernel(root);

        // Kernel root should now be set
        assert_eq!(table.get_root(0), Some(root));
    }

    #[test]
    fn address_space_table_sets_domain_root() {
        let mut table = AddressSpaceTable::new();
        let root1 = unsafe { PhysAddr::new(0x2000) };
        let root2 = unsafe { PhysAddr::new(0x3000) };

        // Set roots for domains 1 and 2
        table.set_root(1, root1);
        table.set_root(2, root2);

        // Verify they are set correctly
        assert_eq!(table.get_root(1), Some(root1));
        assert_eq!(table.get_root(2), Some(root2));

        // Verify domain 0 is still None (not initialized)
        assert_eq!(table.get_root(0), None);
    }

    #[test]
    fn address_space_table_gets_domain_root() {
        let mut table = AddressSpaceTable::new();
        let root = unsafe { PhysAddr::new(0x4000) };

        // Before setting, should return None
        assert_eq!(table.get_root(5), None);

        // After setting, should return Some(root)
        table.set_root(5, root);
        assert_eq!(table.get_root(5), Some(root));
    }

    #[test]
    #[should_panic(expected = "AddressSpaceTable: invalid domain_id")]
    fn address_space_table_rejects_invalid_domain_set() {
        let mut table = AddressSpaceTable::new();
        let root = unsafe { PhysAddr::new(0x5000) };
        table.set_root(MAX_DOMAINS as DomainId, root);
    }

    #[test]
    #[should_panic(expected = "AddressSpaceTable: invalid domain_id")]
    fn address_space_table_rejects_invalid_domain_get() {
        let table = AddressSpaceTable::new();
        table.get_root(MAX_DOMAINS as DomainId);
    }

    #[test]
    #[should_panic(expected = "AddressSpaceTable: invalid domain_id")]
    fn address_space_table_rejects_invalid_domain_clear() {
        let mut table = AddressSpaceTable::new();
        table.clear_root(MAX_DOMAINS as DomainId);
    }

    #[test]
    fn address_space_table_clears_domain_root() {
        let mut table = AddressSpaceTable::new();
        let root = unsafe { PhysAddr::new(0x6000) };

        // Set root
        table.set_root(3, root);
        assert_eq!(table.get_root(3), Some(root));

        // Clear root
        table.clear_root(3);
        assert_eq!(table.get_root(3), None);
    }

    #[test]
    #[should_panic(expected = "AddressSpaceTable: kernel domain (0) already initialized")]
    fn address_space_table_init_kernel_panics_if_already_initialized() {
        let mut table = AddressSpaceTable::new();
        let root1 = unsafe { PhysAddr::new(0x7000) };
        let root2 = unsafe { PhysAddr::new(0x8000) };

        // First init should succeed
        table.init_kernel(root1);
        assert_eq!(table.get_root(0), Some(root1));

        // Second init should panic (original root should still be set)
        table.init_kernel(root2);
    }
}
