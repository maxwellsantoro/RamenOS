// V-012 Phase 3: Trace Capability-Based Access Control
//
// This module provides fine-grained permission management for trace operations.
// Trace capabilities control access to per-domain trace buffers with READ/WRITE/ADMIN rights.

use crate::domain_registry::{DomainId, MAX_DOMAINS};

use core::fmt;
use core::sync::atomic::{AtomicBool, Ordering};
use kernel_api::cap::{is_canonical_generation, next_generation_or_retired};

// Re-export trace rights from kernel_api
pub use kernel_api::trace::{
    TRACE_RIGHT_ADMIN, TRACE_RIGHT_ALL, TRACE_RIGHT_READ, TRACE_RIGHT_WRITE,
};

/// Typed error for trace capability table operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceCapError {
    InvalidDomainId { id: DomainId },
    NoFreeSlots,
    InvalidIndex { index: usize },
    SlotNotInUse { index: usize },
}

impl fmt::Display for TraceCapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDomainId { id } => {
                write!(f, "invalid trace capability domain id {id}")
            }
            Self::NoFreeSlots => write!(f, "no free trace capability slots"),
            Self::InvalidIndex { index } => write!(f, "invalid trace capability index {index}"),
            Self::SlotNotInUse { index } => {
                write!(f, "trace capability slot {index} is not in use")
            }
        }
    }
}

/// Trace capability with domain-scoped rights
#[repr(C)]
#[repr(align(8))]
#[derive(Debug, Clone, Copy)]
pub struct TraceCap {
    /// Domain ID this capability applies to
    pub domain_id: DomainId,

    /// Rights mask (bitwise OR of TRACE_RIGHT_* constants)
    pub rights_mask: u8,

    /// Canonical wire generation, or zero for a permanently retired slot
    pub generation: u64,

    /// Whether this capability slot is in use
    pub in_use: bool,
}

impl TraceCap {
    /// Create a new trace capability
    pub const fn new(domain_id: DomainId, rights_mask: u8, generation: u64) -> Self {
        Self {
            domain_id,
            rights_mask,
            generation,
            in_use: true,
        }
    }

    /// Check if this capability has a specific right
    pub fn has_right(&self, right: u8) -> bool {
        (self.rights_mask & right) == right
    }

    /// Check if this capability can read traces
    pub fn can_read(&self) -> bool {
        self.has_right(TRACE_RIGHT_READ)
    }

    /// Check if this capability can write traces
    pub fn can_write(&self) -> bool {
        self.has_right(TRACE_RIGHT_WRITE)
    }

    /// Check if this capability has admin rights
    pub fn can_admin(&self) -> bool {
        self.has_right(TRACE_RIGHT_ADMIN)
    }
}

/// Trace capability table
///
/// V-012 Phase 3: Stores trace capabilities with domain-scoped rights.
/// This is a separate table from the generic capability table to allow
/// trace-specific operations without polluting the generic interface.
pub struct TraceCapTable {
    /// Trace capability slots
    slots: [TraceCap; MAX_DOMAINS],

    /// SMP safety flag
    smp_enabled: AtomicBool,
}

impl TraceCapTable {
    /// Create a new trace capability table
    pub const fn new() -> Self {
        const EMPTY_CAP: TraceCap = TraceCap {
            domain_id: 0,
            rights_mask: 0,
            generation: 1,
            in_use: false,
        };

        Self {
            slots: [EMPTY_CAP; MAX_DOMAINS],
            smp_enabled: AtomicBool::new(false),
        }
    }

    /// Allocate a trace capability for a domain with specified rights
    ///
    /// # Arguments
    /// - `domain_id`: Domain ID to grant trace access to
    /// - `rights_mask`: Bitwise OR of TRACE_RIGHT_* constants
    ///
    /// # Returns
    /// `Ok(index)` on success, or a typed [`TraceCapError`] on failure.
    pub fn allocate(
        &mut self,
        domain_id: DomainId,
        rights_mask: u8,
    ) -> Result<usize, TraceCapError> {
        // Check SMP safety
        if self.smp_enabled.load(Ordering::Acquire) {
            panic!("TraceCapTable::allocate called after SMP enabled");
        }

        // Validate domain_id
        if domain_id >= MAX_DOMAINS as DomainId {
            return Err(TraceCapError::InvalidDomainId { id: domain_id });
        }

        // Find free slot
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if !slot.in_use && is_canonical_generation(slot.generation) {
                *slot = TraceCap::new(domain_id, rights_mask, slot.generation);
                return Ok(i);
            }
        }

        Err(TraceCapError::NoFreeSlots)
    }

    /// Deallocate a trace capability
    ///
    /// # Arguments
    /// - `index`: Slot index to deallocate
    ///
    /// # Returns
    /// `Ok(())` on success, or a typed [`TraceCapError`] on failure.
    pub fn deallocate(&mut self, index: usize) -> Result<(), TraceCapError> {
        // Check SMP safety
        if self.smp_enabled.load(Ordering::Acquire) {
            panic!("TraceCapTable::deallocate called after SMP enabled");
        }

        if index >= MAX_DOMAINS {
            return Err(TraceCapError::InvalidIndex { index });
        }

        if !self.slots[index].in_use {
            return Err(TraceCapError::SlotNotInUse { index });
        }

        // Exhaustion permanently retires the slot instead of reusing wire generations.
        self.slots[index].generation = next_generation_or_retired(self.slots[index].generation);
        self.slots[index].in_use = false;

        Ok(())
    }

    /// Get a trace capability by index
    ///
    /// # Arguments
    /// - `index`: Slot index
    ///
    /// # Returns
    /// `Some(&TraceCap)` if slot exists and is in use, `None` otherwise
    pub fn get(&self, index: usize) -> Option<&TraceCap> {
        if index >= MAX_DOMAINS {
            return None;
        }

        let slot = &self.slots[index];
        if slot.in_use && is_canonical_generation(slot.generation) {
            Some(slot)
        } else {
            None
        }
    }

    /// Check if a capability has a specific right
    ///
    /// # Arguments
    /// - `index`: Capability slot index
    /// - `right`: Right to check (TRACE_RIGHT_*)
    ///
    /// # Returns
    /// `true` if capability exists and has the right, `false` otherwise
    pub fn check_right(&self, index: usize, right: u8) -> bool {
        self.get(index)
            .map(|cap| cap.has_right(right))
            .unwrap_or(false)
    }

    /// Grant additional rights to a trace capability
    ///
    /// # Arguments
    /// - `index`: Capability slot index
    /// - `rights`: Rights to grant (bitwise OR of TRACE_RIGHT_*)
    ///
    /// # Returns
    /// `Ok(())` on success, or a typed [`TraceCapError`] on failure.
    pub fn grant_rights(&mut self, index: usize, rights: u8) -> Result<(), TraceCapError> {
        // Check SMP safety
        if self.smp_enabled.load(Ordering::Acquire) {
            panic!("TraceCapTable::grant_rights called after SMP enabled");
        }

        if index >= MAX_DOMAINS {
            return Err(TraceCapError::InvalidIndex { index });
        }

        if !self.slots[index].in_use {
            return Err(TraceCapError::SlotNotInUse { index });
        }

        self.slots[index].rights_mask |= rights;
        Ok(())
    }

    /// Revoke specific rights from a trace capability
    ///
    /// # Arguments
    /// - `index`: Capability slot index
    /// - `rights`: Rights to revoke (bitwise OR of TRACE_RIGHT_*)
    ///
    /// # Returns
    /// `Ok(())` on success, or a typed [`TraceCapError`] on failure.
    pub fn revoke_rights(&mut self, index: usize, rights: u8) -> Result<(), TraceCapError> {
        // Check SMP safety
        if self.smp_enabled.load(Ordering::Acquire) {
            panic!("TraceCapTable::revoke_rights called after SMP enabled");
        }

        if index >= MAX_DOMAINS {
            return Err(TraceCapError::InvalidIndex { index });
        }

        if !self.slots[index].in_use {
            return Err(TraceCapError::SlotNotInUse { index });
        }

        self.slots[index].rights_mask &= !rights;
        Ok(())
    }

    /// Get the domain ID for a trace capability
    ///
    /// # Arguments
    /// - `index`: Capability slot index
    ///
    /// # Returns
    /// `Some(domain_id)` if capability exists, `None` otherwise
    pub fn get_domain_id(&self, index: usize) -> Option<DomainId> {
        self.get(index).map(|cap| cap.domain_id)
    }

    /// Mark the system as having enabled SMP
    ///
    /// After calling this, all mutating operations will panic.
    pub fn smp_enabled(&mut self) {
        if self.smp_enabled.swap(true, Ordering::SeqCst) {
            panic!("smp_enabled() called more than once");
        }
    }

    /// Check if SMP has been enabled
    pub fn is_smp_enabled(&self) -> bool {
        self.smp_enabled.load(Ordering::Acquire)
    }
}

impl Default for TraceCapTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Global trace capability table instance
static mut TRACE_CAP_TABLE: TraceCapTable = TraceCapTable::new();

/// Get the global trace capability table
///
/// # Safety
/// This function provides mutable access to a global static.
/// It should only be called during single-threaded kernel boot.
pub unsafe fn global_trace_cap_table() -> &'static mut TraceCapTable {
    &mut TRACE_CAP_TABLE
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_trace_cap_succeeds() {
        let mut table = TraceCapTable::new();

        let index = table.allocate(0, TRACE_RIGHT_READ);
        assert!(index.is_ok());

        let cap = table.get(index.unwrap());
        assert!(cap.is_some());
        let cap = cap.unwrap();
        assert_eq!(cap.domain_id, 0);
        assert_eq!(cap.rights_mask, TRACE_RIGHT_READ);
    }

    #[test]
    fn allocate_rejects_invalid_domain() {
        let mut table = TraceCapTable::new();

        let index = table.allocate(MAX_DOMAINS as DomainId, TRACE_RIGHT_READ);
        assert!(index.is_err());
    }

    #[test]
    fn deallocate_frees_slot() {
        let mut table = TraceCapTable::new();

        let index = table.allocate(0, TRACE_RIGHT_READ).unwrap();
        assert!(table.deallocate(index).is_ok());

        // Slot should no longer be in use
        assert!(table.get(index).is_none());
    }

    #[test]
    fn deallocate_increments_generation() {
        let mut table = TraceCapTable::new();

        let index = table.allocate(0, TRACE_RIGHT_READ).unwrap();
        let gen_before = table.get(index).unwrap().generation;

        table.deallocate(index).unwrap();

        // Re-allocate the same slot
        table.allocate(1, TRACE_RIGHT_WRITE).unwrap();
        let gen_after = table.get(index).unwrap().generation;

        assert_eq!(gen_after, gen_before + 1);
    }

    #[test]
    fn wire_generation_last_legal_retires_and_preserves_other_domain() {
        use kernel_api::cap::{Handle, HandleKind};
        let mut table = TraceCapTable::new();
        table.slots[0].generation = u32::MAX as u64;
        let last_index = table.allocate(1, TRACE_RIGHT_READ).unwrap();
        let last = Handle {
            kind: HandleKind::Trace,
            index: last_index as u32 + 1,
            generation: table.get(last_index).unwrap().generation,
        };
        assert_eq!(last.generation, u32::MAX as u64);
        assert_eq!(Handle::unpack(last.pack()), last);
        let unrelated = table.allocate(2, TRACE_RIGHT_WRITE).unwrap();
        table.deallocate(last_index).unwrap();
        assert_eq!(
            table.slots[last_index].generation, 0,
            "exhausted slot must retire"
        );
        assert!(table.get(last_index).is_none());
        assert!(!table.check_right(last_index, TRACE_RIGHT_READ));
        let next_index = table.allocate(3, TRACE_RIGHT_ADMIN).unwrap();
        assert_ne!(next_index, last_index);
        let next = Handle {
            kind: HandleKind::Trace,
            index: next_index as u32 + 1,
            generation: table.get(next_index).unwrap().generation,
        };
        assert_eq!(Handle::unpack(next.pack()), next);
        assert_eq!(table.get_domain_id(next_index), Some(3));
        assert!(table.check_right(next_index, TRACE_RIGHT_ADMIN));
        assert_eq!(table.get_domain_id(unrelated), Some(2));
        assert!(table.check_right(unrelated, TRACE_RIGHT_WRITE));
    }

    #[test]
    fn wire_generation_retired_slots_exhaust_capacity() {
        let mut table = TraceCapTable::new();
        for slot in &mut table.slots {
            slot.generation = 0;
        }
        assert_eq!(
            table.allocate(1, TRACE_RIGHT_READ),
            Err(TraceCapError::NoFreeSlots)
        );
    }

    #[test]
    fn wire_generation_allocation_skips_noncanonical_free_slots() {
        let mut table = TraceCapTable::new();
        table.slots[0].generation = 0;
        table.slots[1].generation = u32::MAX as u64 + 1;
        let index = table.allocate(1, TRACE_RIGHT_READ).unwrap();
        assert_eq!(index, 2);
        assert_eq!(table.get(index).unwrap().generation, 1);
    }

    #[test]
    fn wire_generation_get_denies_noncanonical_live_slots() {
        let mut table = TraceCapTable::new();
        let index = table.allocate(1, TRACE_RIGHT_READ).unwrap();
        for generation in [0, u32::MAX as u64 + 1, u64::MAX] {
            table.slots[index].generation = generation;
            assert!(table.get(index).is_none());
            assert_eq!(table.get_domain_id(index), None);
            assert!(!table.check_right(index, TRACE_RIGHT_READ));
        }
    }

    #[test]
    fn grant_rights_adds_permissions() {
        let mut table = TraceCapTable::new();

        let index = table.allocate(0, TRACE_RIGHT_READ).unwrap();

        assert!(table.grant_rights(index, TRACE_RIGHT_WRITE).is_ok());

        let cap = table.get(index).unwrap();
        assert!(cap.has_right(TRACE_RIGHT_READ));
        assert!(cap.has_right(TRACE_RIGHT_WRITE));
        assert!(!cap.has_right(TRACE_RIGHT_ADMIN));
    }

    #[test]
    fn revoke_rights_removes_permissions() {
        let mut table = TraceCapTable::new();

        let index = table.allocate(0, TRACE_RIGHT_ALL).unwrap();

        assert!(table.revoke_rights(index, TRACE_RIGHT_WRITE).is_ok());

        let cap = table.get(index).unwrap();
        assert!(cap.has_right(TRACE_RIGHT_READ));
        assert!(!cap.has_right(TRACE_RIGHT_WRITE));
        assert!(cap.has_right(TRACE_RIGHT_ADMIN));
    }

    #[test]
    fn check_right_validates_permissions() {
        let mut table = TraceCapTable::new();

        let index = table
            .allocate(0, TRACE_RIGHT_READ | TRACE_RIGHT_WRITE)
            .unwrap();

        assert!(table.check_right(index, TRACE_RIGHT_READ));
        assert!(table.check_right(index, TRACE_RIGHT_WRITE));
        assert!(!table.check_right(index, TRACE_RIGHT_ADMIN));
    }

    #[test]
    fn get_domain_id_returns_correct_domain() {
        let mut table = TraceCapTable::new();

        let index = table.allocate(5, TRACE_RIGHT_READ).unwrap();

        assert_eq!(table.get_domain_id(index), Some(5));
    }

    #[test]
    fn trace_cap_convenience_methods() {
        let cap = TraceCap::new(0, TRACE_RIGHT_ALL, 1);

        assert!(cap.can_read());
        assert!(cap.can_write());
        assert!(cap.can_admin());

        let cap_read_only = TraceCap::new(0, TRACE_RIGHT_READ, 1);
        assert!(cap_read_only.can_read());
        assert!(!cap_read_only.can_write());
        assert!(!cap_read_only.can_admin());
    }

    #[test]
    fn allocate_multiple_caps() {
        let mut table = TraceCapTable::new();

        let idx1 = table.allocate(0, TRACE_RIGHT_READ).unwrap();
        let idx2 = table.allocate(1, TRACE_RIGHT_WRITE).unwrap();
        let idx3 = table.allocate(2, TRACE_RIGHT_ADMIN).unwrap();

        // All should be different indices
        assert_ne!(idx1, idx2);
        assert_ne!(idx2, idx3);

        // Verify each has correct rights
        assert!(table.check_right(idx1, TRACE_RIGHT_READ));
        assert!(table.check_right(idx2, TRACE_RIGHT_WRITE));
        assert!(table.check_right(idx3, TRACE_RIGHT_ADMIN));
    }

    // Note: SMP panic tests cannot be run in no_std without std::panic::catch_unwind
    // The smp_enabled() functionality is tested implicitly by other tests
    // that verify the table works correctly before SMP is enabled.
}
