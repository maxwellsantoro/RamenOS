//! S8 Phase 4: Physical memory management with bitmap allocator.
//!
//! This module provides type-safe physical memory allocation for the kernel.
//! Key components:
//! - `PhysAddr` and `PhysFrame`: Strong types for physical addresses and 4KiB frames
//! - `FrameAllocator`: Trait for architecture-agnostic frame allocation
//! - `BitmapAllocator`: Thread-safe bitmap allocator for shared memory regions
//!
//! ## Design Constraints
//!
//! - **No heap allocation**: Allocators themselves use static arrays
//! - **Type safety**: Physical addresses use newtype wrappers, not raw u64
//! - **Architecture isolation**: FrameAllocator trait abstracts arch-specific boot memory maps
//! - **Concurrency**: Spinlocks protect mutable state when needed
//!
//! # V-010: Thread-Safe Global State
//!
//! **SECURITY FIX (NEW-004)**: The global `FRAME_ALLOCATOR` and `ADDRESS_SPACE_TABLE`
//! are now protected by `spin::Mutex` to prevent data races when SMP is enabled.
//!
//! ## Safety Requirements
//!
//! - **Thread-safe access**: All access must acquire the mutex via `.lock()`
//! - **One-time initialization**: Must be initialized exactly once via `init()`
//! - **No reset**: Once initialized, the allocator state must not be reset
//! - **Concurrent access**: Multiple threads can safely access with proper locking
//!
//! ## Why Mutex?
//!
//! The bitmap allocator uses `spin::Mutex` because:
//! 1. No heap allocation is available during early boot
//! 2. We need a global allocator accessible before process isolation is established
//! 3. Spinlocks are appropriate for kernel code where we can't block
//! 4. Provides safety against data races when SMP is enabled
//!
//! ## Migration from static mut
//!
//! Previously used `static mut` which was only safe for single-threaded early boot.
//! This was a CRITICAL vulnerability (NEW-004) that could cause:
//! - Data races when multiple threads access the allocator
//! - Use-after-free due to unsynchronized deallocation
//! - Memory corruption from concurrent allocation/deallocation
//!
//! The `spin::Mutex` wrapper eliminates these vulnerabilities by ensuring
//! exclusive access to the mutable state.
//!
//! ## SMP Transition Status (2026-02-18)
//!
//! **COMPLETED**: The global `FRAME_ALLOCATOR` and `ADDRESS_SPACE_TABLE` are now
//! SMP-safe with `spin::Mutex` protection. All accesses to these globals are
//! properly locked.
//!
//! **CONSUMERS**: The following modules consume these SMP-safe globals:
//! - `shmem`: Uses `FRAME_ALLOCATOR.lock()` for physical frame allocation
//! - `arch/x86_64/mmu` and `arch/aarch64/mmu`: Use `ADDRESS_SPACE_TABLE.lock()` for page table management
//!
//! **REMAINING DEBT**: Per-domain MMU operations must ensure proper memory barriers
//! for SMP when programming page tables across multiple CPUs.

use crate::boot::BootMemoryMap;
use crate::kprintln;
use spin::Mutex;

pub mod address;
pub mod address_space;
pub mod bitmap;
pub mod boot_pool;
#[cfg(test)]
mod boot_pool_contract_tests;
pub mod bootstrap;
pub mod bump;
pub mod constants;
pub mod frame;
#[cfg(test)]
pub(crate) mod test_support;

pub use address::{PhysAddr, PhysFrame};
pub use address_space::AddressSpaceTable;
pub use bitmap::BitmapAllocator;
pub use bump::BumpAllocator;
pub use frame::FrameAllocator;

/// Global physical frame allocator protected by a spinlock.
///
/// # Security Fix (NEW-004)
///
/// Previously used `static mut` which was vulnerable to data races.
/// Now protected by `spin::Mutex` to ensure thread-safe access.
///
/// # Safety Requirements
///
/// - MUST be initialized exactly once via `init()` before any allocations
/// - All access MUST acquire the mutex via `.lock()`
/// - MUST NOT be reset or reinitialized after first use
/// - Safe for concurrent access from multiple threads after SMP is enabled
///
/// # Usage Pattern
///
/// ```rust,ignore
/// // During boot:
/// mm::init(&boot_memory_map);  // One-time initialization
///
/// // Allocation (acquires lock):
/// let frame = mm::FRAME_ALLOCATOR.lock().allocate();
///
/// // Deallocation (acquires lock):
/// mm::FRAME_ALLOCATOR.lock().deallocate(frame);
/// ```
///
/// # Why spin::Mutex?
///
/// In no_std bare-metal environments, `spin::Mutex` is ideal because:
/// - No heap allocation required
/// - Works in early boot before other synchronization primitives are available
/// - Provides exclusive access preventing data races
/// - Spinlocks are appropriate for kernel code where blocking is not an option
///
/// # S8 Phase 4: Bitmap Allocator
///
/// The bitmap allocator supports deallocation, which is required for shared memory regions.
/// Regions can be closed and their frames returned to the allocator for reuse.
pub static FRAME_ALLOCATOR: Mutex<Option<BitmapAllocator>> = Mutex::new(None);

/// Global address space table for tracking page table roots per domain.
///
/// This table maintains a mapping from domain IDs to their page table root
/// physical addresses. Domain 0 is the kernel domain.
///
/// # Security Fix (NEW-004)
///
/// Previously used `static mut` which was vulnerable to data races.
/// Now protected by `spin::Mutex` to ensure thread-safe access.
///
/// # Safety Requirements
///
/// - MUST be initialized exactly once during kernel boot via `init_address_space_table()`
/// - MUST be initialized before any domain operations
/// - The kernel domain (0) must be initialized with the current page table root
/// - All access MUST acquire the mutex via `.lock()`
/// - Safe for concurrent access from multiple threads after SMP is enabled
pub static ADDRESS_SPACE_TABLE: Mutex<Option<AddressSpaceTable>> = Mutex::new(None);

/// Install only the explicitly held bootstrap allocation, exactly once.
///
/// Firmware remains active on the current x86 path. Conventional memory and
/// boot-services regions are never selected here. The trusted boot-only claim
/// and complete numeric map are validated before state publication; any denial
/// preserves the existing allocator and its outstanding frame ownership.
/// This is separate from RUN0 final-map/retained-range admission.
pub fn init(map: &BootMemoryMap) -> Result<(), bootstrap::BootstrapError> {
    let mut guard = FRAME_ALLOCATOR.lock();
    bootstrap::install_into(&mut guard, map)?;
    let allocator = guard
        .as_ref()
        .expect("successful install publishes allocator");
    kprintln!(
        "mm: allocator ready. total_frames={} available={} bootstrap={:?}",
        allocator.total_frames(),
        allocator.available_frames(),
        map.bootstrap_allocation()
            .map(|allocation| allocation.provenance)
    );
    Ok(())
}

/// Initialize the global address space table with the kernel page table root.
///
/// This function creates a new AddressSpaceTable and initializes the kernel
/// domain (domain 0) with the current page table root.
///
/// # Safety
///
/// MUST be called exactly once during kernel boot, before any domain operations.
///
/// # Panics
///
/// Panics if called more than once (table already initialized).
pub fn init_address_space_table(kernel_pt_root: PhysAddr) {
    let mut table_guard = ADDRESS_SPACE_TABLE.lock();
    if table_guard.is_some() {
        panic!("mm: address space table already initialized");
    }
    let mut table = AddressSpaceTable::new();
    table.init_kernel(kernel_pt_root);
    *table_guard = Some(table);
    kprintln!(
        "mm: address space table initialized with kernel page table root {:?}",
        kernel_pt_root
    );
    drop(table_guard); // Explicitly release lock
}
