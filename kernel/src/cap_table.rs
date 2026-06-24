// V-05/V-06: Kernel capability table with generation counters.
// Static-array-backed, no_std compatible implementation.
//
// # V-008 Thread Safety
//
// **IMPORTANT**: This implementation is NOT thread-safe and MUST only be used
// in single-threaded contexts (e.g., kernel boot process, init task).
//
// Using this table from multiple threads without external synchronization
// will result in data races, use-after-free vulnerabilities, and capability
// forgeries.
//
// Future work (S8+): Add Mutex or RWLock wrapper for multi-domain use cases.
//
// # Additional Hardening: SMP Safety Assertions
//
// Runtime checks detect accidental use after SMP is enabled. Call
// `smp_enabled()` to mark the transition to multi-threaded operation.
// All subsequent capability operations will panic.

use core::sync::atomic::{AtomicBool, Ordering};
use kernel_api::cap::{CapTable, CapTableError, Handle};

const CAP_TABLE_SIZE: usize = 64;

/// Make CAP_TABLE_SIZE accessible for testing
pub const TEST_CAP_TABLE_SIZE: usize = CAP_TABLE_SIZE;

/// Global flag tracking whether SMP has been enabled.
/// Once set to true, all capability table operations will panic.
static SMP_ENABLED: AtomicBool = AtomicBool::new(false);

/// Mark the system as having enabled SMP (multi-threaded operation).
///
/// # Additional Hardening: SMP Safety
///
/// After calling this function, all capability table operations will panic.
/// This prevents accidental use of the single-threaded table in a multi-threaded
/// context, which would cause data races and capability forgeries.
///
/// # When to Call
///
/// Call this function once during kernel initialization, right before:
/// - Starting additional CPU cores
/// - Enabling interrupts for multi-core operation
/// - Spawning kernel threads
///
/// # Safety
///
/// This function should only be called once, and only after all single-threaded
/// capability operations have completed. Typically called late in boot, just
/// before transitioning to multi-threaded operation.
///
/// # Panics
///
/// Panics if called more than once.
pub fn smp_enabled() {
    if SMP_ENABLED.swap(true, Ordering::SeqCst) {
        panic!("smp_enabled() called more than once");
    }
}

/// Check if SMP has been enabled.
///
/// Returns true if the system has transitioned to multi-threaded operation.
#[inline]
pub fn is_smp_enabled() -> bool {
    SMP_ENABLED.load(Ordering::Acquire)
}

/// Test helper: reset SMP state for testing.
#[cfg(test)]
pub fn reset_smp_state_for_test() {
    SMP_ENABLED.store(false, Ordering::Release);
}

/// V-005: CapSlot with explicit alignment for safe unaligned access
/// repr(C) ensures predictable layout across architectures
/// repr(align(8)) ensures 8-byte alignment for u64 generation field
#[repr(C)]
#[repr(align(8))]
#[derive(Debug, Clone, Copy)]
struct CapSlot {
    // V-004: Use u64 for generation counter to prevent practical wrapping
    generation: u64,
    in_use: bool,
    // Domain identity bound to this IPC capability.
    domain_id: u64,
}

impl CapSlot {
    const fn new() -> Self {
        Self {
            generation: 1,
            in_use: false,
            domain_id: 0,
        }
    }
}

/// V-05/V-06: Static-array-backed capability table with generation counters.
/// Rejects stale handles via generation mismatch.
///
/// # V-008: Thread Safety Guarantee
///
/// **NOT thread-safe** - must only be used in single-threaded contexts.
/// All operations assume exclusive access without concurrent modification.
pub struct StaticCapTable {
    slots: [CapSlot; CAP_TABLE_SIZE],
}

impl StaticCapTable {
    pub const fn new() -> Self {
        Self {
            slots: [CapSlot::new(); CAP_TABLE_SIZE],
        }
    }

    /// Allocate a capability and bind it to a caller domain.
    pub fn allocate_for_domain(&mut self, domain_id: u64) -> Result<Handle, CapTableError> {
        // Additional hardening: Panic if used after SMP is enabled
        if is_smp_enabled() {
            panic!("capability table allocate() called after SMP enabled - data race detected");
        }

        for (i, slot) in self.slots.iter_mut().enumerate() {
            if !slot.in_use {
                slot.in_use = true;
                slot.domain_id = domain_id;
                return Ok(Handle {
                    kind: kernel_api::cap::HandleKind::Ipc, // V-16/SC-13: IPC handles
                    index: (i + 1) as u32,                  // Reserve index 0 for INVALID
                    generation: slot.generation,
                });
            }
        }
        Err(CapTableError::TableFull)
    }

    /// Return the bound domain for a valid IPC handle.
    pub fn domain_for(&self, handle: Handle) -> Option<u64> {
        if !self.validate(handle) {
            return None;
        }
        let slot = &self.slots[(handle.index - 1) as usize];
        Some(slot.domain_id)
    }
}

impl Default for StaticCapTable {
    fn default() -> Self {
        Self::new()
    }
}

impl CapTable for StaticCapTable {
    fn allocate(&mut self) -> Result<Handle, CapTableError> {
        // Legacy allocate() binds to kernel domain by default.
        self.allocate_for_domain(0)
    }

    fn validate(&self, handle: Handle) -> bool {
        // Additional hardening: Panic if used after SMP is enabled
        if is_smp_enabled() {
            panic!("capability table validate() called after SMP enabled - data race detected");
        }

        // V-16/SC-13: Reject handles with wrong kind
        if handle.kind != kernel_api::cap::HandleKind::Ipc {
            return false;
        }
        if handle.index == 0 || handle.index > CAP_TABLE_SIZE as u32 {
            return false;
        }
        let slot = &self.slots[(handle.index - 1) as usize];
        slot.in_use && slot.generation == handle.generation
    }

    fn deallocate(&mut self, handle: Handle) -> Result<(), CapTableError> {
        // Additional hardening: Panic if used after SMP is enabled
        if is_smp_enabled() {
            panic!("capability table deallocate() called after SMP enabled - data race detected");
        }

        // V-16/SC-13: Reject handles with wrong kind
        if handle.kind != kernel_api::cap::HandleKind::Ipc {
            return Err(CapTableError::InvalidHandle);
        }
        if handle.index == 0 || handle.index > CAP_TABLE_SIZE as u32 {
            return Err(CapTableError::InvalidHandle);
        }
        let slot = &mut self.slots[(handle.index - 1) as usize];
        if !slot.in_use || slot.generation != handle.generation {
            return Err(CapTableError::StaleHandle);
        }
        slot.in_use = false;
        slot.domain_id = 0;
        slot.generation = slot.generation.wrapping_add(1);
        if slot.generation == 0 {
            slot.generation = 1; // Avoid generation 0 (reserved for invalid)
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::hint::spin_loop;
    use core::sync::atomic::{AtomicBool, Ordering};

    static TEST_LOCK: AtomicBool = AtomicBool::new(false);

    struct TestGuard;

    impl Drop for TestGuard {
        fn drop(&mut self) {
            TEST_LOCK.store(false, Ordering::Release);
        }
    }

    // Test helper to reset SMP state for testing
    fn reset_smp_state() -> TestGuard {
        while TEST_LOCK
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            spin_loop();
        }
        reset_smp_state_for_test();
        TestGuard
    }

    #[test]
    fn allocate_returns_valid_handle() {
        let _guard = reset_smp_state();
        let mut table = StaticCapTable::new();
        let handle = table.allocate().unwrap();
        assert_eq!(handle.kind, kernel_api::cap::HandleKind::Ipc);
        assert!(handle.index > 0);
        assert!(handle.generation > 0);
        assert!(table.validate(handle));
    }

    #[test]
    fn allocate_for_domain_binds_handle_identity() {
        let _guard = reset_smp_state();
        let mut table = StaticCapTable::new();
        let handle = table.allocate_for_domain(42).unwrap();
        assert_eq!(table.domain_for(handle), Some(42));
    }

    #[test]
    fn validate_rejects_invalid_handle() {
        let _guard = reset_smp_state();
        let table = StaticCapTable::new();
        assert!(!table.validate(Handle::INVALID));
        assert!(!table.validate(Handle {
            kind: kernel_api::cap::HandleKind::Ipc,
            index: 0,
            generation: 1
        }));
        assert!(!table.validate(Handle {
            kind: kernel_api::cap::HandleKind::Ipc,
            index: 999,
            generation: 1
        }));
    }

    #[test]
    fn validate_rejects_stale_handle() {
        let _guard = reset_smp_state();
        let mut table = StaticCapTable::new();
        let handle = table.allocate().unwrap();
        table.deallocate(handle).unwrap();
        // Stale handle should be rejected
        assert!(!table.validate(handle));
    }

    #[test]
    fn validate_rejects_wrong_kind_shmem_handle() {
        let _guard = reset_smp_state();
        // V-16/SC-13: Shared memory handles should be rejected by IPC table
        let table = StaticCapTable::new();

        let shmem_handle = kernel_api::cap::Handle {
            kind: kernel_api::cap::HandleKind::Shmem,
            index: 1,
            generation: 1,
        };

        // Shared memory handle should be rejected even if index/generation would match
        assert!(!table.validate(shmem_handle));
    }

    #[test]
    fn validate_rejects_invalid_kind() {
        let _guard = reset_smp_state();
        let table = StaticCapTable::new();

        let invalid_handle = kernel_api::cap::Handle {
            kind: kernel_api::cap::HandleKind::Invalid,
            index: 1,
            generation: 1,
        };

        // Invalid kind should be rejected
        assert!(!table.validate(invalid_handle));
    }

    #[test]
    fn deallocate_increments_generation() {
        let _guard = reset_smp_state();
        let mut table = StaticCapTable::new();
        let handle1 = table.allocate().unwrap();
        let gen1 = handle1.generation;
        table.deallocate(handle1).unwrap();
        let handle2 = table.allocate().unwrap();
        assert_eq!(handle1.index, handle2.index);
        assert!(handle2.generation > gen1);
    }

    #[test]
    fn stale_handle_rejected_via_generation_mismatch() {
        let _guard = reset_smp_state();
        let mut table = StaticCapTable::new();
        let handle1 = table.allocate().unwrap();
        table.deallocate(handle1).unwrap();
        let handle2 = table.allocate().unwrap();
        // handle1 is stale, should be rejected
        assert!(!table.validate(handle1));
        // handle2 is current, should be accepted
        assert!(table.validate(handle2));
    }

    #[test]
    fn deallocate_rejects_stale_handle() {
        let _guard = reset_smp_state();
        let mut table = StaticCapTable::new();
        let handle = table.allocate().unwrap();
        table.deallocate(handle).unwrap();
        // Deallocating a stale handle should fail
        assert!(table.deallocate(handle).is_err());
    }

    #[test]
    fn allocate_fills_table_then_returns_error() {
        let _guard = reset_smp_state();
        let mut table = StaticCapTable::new();
        // Allocate all slots
        for _ in 0..CAP_TABLE_SIZE {
            table.allocate().unwrap();
        }
        // Next allocation should fail
        assert!(table.allocate().is_err());
    }

    #[test]
    fn generation_wraps_avoids_zero() {
        let _guard = reset_smp_state();
        let mut table = StaticCapTable::new();
        let mut handle = table.allocate().unwrap();
        // V-004: Test that generation increments work correctly with u64
        // Force generation to near max to test wrapping behavior
        handle.generation = u64::MAX;
        let slot = &mut table.slots[(handle.index - 1) as usize];
        slot.generation = u64::MAX;
        slot.in_use = true;
        table.deallocate(handle).unwrap();
        let new_handle = table.allocate().unwrap();
        assert_eq!(new_handle.index, handle.index);
        assert_eq!(new_handle.generation, 1); // Should wrap to 1, not 0
    }

    // Additional hardening: SMP safety tests
    // NOTE: Named with zzz_ prefix to ensure these run last (after all other tests)
    // because they leave SMP_ENABLED set to true when they finish

    #[test]
    fn zzz_smp_enabled_transitions_state() {
        let _guard = reset_smp_state();
        assert!(!is_smp_enabled());

        smp_enabled();
        assert!(is_smp_enabled());
    }

    #[test]
    #[should_panic(expected = "smp_enabled() called more than once")]
    fn zzz_smp_enabled_panics_on_double_call() {
        let _guard = reset_smp_state();
        smp_enabled();
        smp_enabled(); // Should panic
    }

    #[test]
    #[should_panic(expected = "capability table allocate() called after SMP enabled")]
    fn zzz_allocate_panics_after_smp_enabled() {
        let _guard = reset_smp_state();
        smp_enabled();

        let mut table = StaticCapTable::new();
        table.allocate().unwrap(); // Should panic
    }

    #[test]
    #[should_panic(expected = "capability table validate() called after SMP enabled")]
    fn zzz_validate_panics_after_smp_enabled() {
        let _guard = reset_smp_state();
        smp_enabled();

        let table = StaticCapTable::new();
        table.validate(Handle::INVALID); // Should panic
    }

    #[test]
    #[should_panic(expected = "capability table deallocate() called after SMP enabled")]
    fn zzz_deallocate_panics_after_smp_enabled() {
        let _guard = reset_smp_state();

        let mut table = StaticCapTable::new();
        let handle = table.allocate().unwrap();

        // Now enable SMP - deallocate should panic
        smp_enabled();
        table.deallocate(handle).unwrap(); // Should panic
        table.deallocate(handle).unwrap(); // Should panic
    }

    #[test]
    fn operations_work_before_smp_enabled() {
        let _guard = reset_smp_state();

        let mut table = StaticCapTable::new();
        let handle = table.allocate().unwrap();
        assert!(table.validate(handle));
        table.deallocate(handle).unwrap();

        // Now enable SMP - operations should still work before we call them
        smp_enabled();
    }

    // Additional hardening: Property-based tests

    #[test]
    fn prop_allocated_handles_are_unique() {
        let _guard = reset_smp_state();
        let mut table = StaticCapTable::new();

        // Allocate many handles and verify they're all unique
        let mut handles = [Handle::INVALID; 100];
        for handle_slot in &mut handles {
            if let Ok(handle) = table.allocate() {
                *handle_slot = handle;
            } else {
                break; // Table full
            }
        }

        // Verify all handles are unique
        for i in 0..100 {
            for j in (i + 1)..100 {
                if handles[i] != Handle::INVALID && handles[j] != Handle::INVALID {
                    assert_ne!(handles[i], handles[j]);
                }
            }
        }
    }

    #[test]
    fn prop_generation_counters_are_monotonic() {
        let _guard = reset_smp_state();
        let mut table = StaticCapTable::new();

        // Allocate and deallocate same slot multiple times
        let mut last_generation = 0u64;

        for _ in 0..10 {
            let handle = table.allocate().unwrap();
            assert_eq!(handle.index, 1); // Should allocate same slot

            // Generation should always increase
            assert!(handle.generation > last_generation);
            last_generation = handle.generation;

            table.deallocate(handle).unwrap();
        }
    }

    #[test]
    fn prop_stale_handles_always_rejected() {
        let _guard = reset_smp_state();
        let mut table = StaticCapTable::new();

        // Allocate/deallocate/allocate cycle multiple times
        for _ in 0..50 {
            let handle1 = table.allocate().unwrap();
            table.deallocate(handle1).unwrap();
            let handle2 = table.allocate().unwrap();

            // handle1 is stale, should be rejected
            assert!(!table.validate(handle1));
            // handle2 is current, should be accepted
            assert!(table.validate(handle2));

            // Clean up for next iteration
            table.deallocate(handle2).unwrap();
        }
    }

    #[test]
    fn prop_double_free_is_rejected() {
        let _guard = reset_smp_state();
        let mut table = StaticCapTable::new();

        for _ in 0..CAP_TABLE_SIZE {
            let handle = table.allocate().unwrap();
            table.deallocate(handle).unwrap();

            // Attempting to deallocate again should fail
            assert!(table.deallocate(handle).is_err());

            // Allocate again to clean up
            let _new_handle = table.allocate().unwrap();
        }
    }

    #[test]
    fn prop_invalid_index_always_rejected() {
        let _guard = reset_smp_state();
        let table = StaticCapTable::new();

        // Test all possible invalid indices
        let invalid_indices = [0, CAP_TABLE_SIZE as u32 + 1, u32::MAX, 999999];

        for invalid_index in invalid_indices {
            let handle = Handle {
                kind: kernel_api::cap::HandleKind::Ipc,
                index: invalid_index,
                generation: 1,
            };
            assert!(!table.validate(handle));
        }
    }

    #[test]
    fn prop_table_exhaustion_is_consistent() {
        let _guard = reset_smp_state();
        let mut table = StaticCapTable::new();

        // Allocate until exhaustion
        let mut allocated = 0;
        while table.allocate().is_ok() {
            allocated += 1;
        }

        // Should have allocated exactly CAP_TABLE_SIZE handles
        assert_eq!(allocated, CAP_TABLE_SIZE);

        // All allocations should fail
        for _ in 0..10 {
            assert!(table.allocate().is_err());
        }
    }

    #[test]
    fn prop_capabilities_not_reusable_after_deallocation() {
        let _guard = reset_smp_state();
        let mut table = StaticCapTable::new();

        // Test multiple allocations/deallocations
        for _ in 0..20 {
            let handle = table.allocate().unwrap();
            let original_index = handle.index;
            let original_generation = handle.generation;

            table.deallocate(handle).unwrap();

            // New handle should have same index but different generation
            let new_handle = table.allocate().unwrap();
            assert_eq!(new_handle.index, original_index);
            assert!(new_handle.generation > original_generation);
            assert!(!table.validate(handle)); // Old handle is invalid
        }
    }
}
