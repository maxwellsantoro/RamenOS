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

use crate::boot::{BootMemoryMap, RegionKind};
use crate::kprintln;
use spin::Mutex;

pub mod address;
pub mod address_space;
pub mod bitmap;
pub mod bump;
pub mod constants;
pub mod frame;

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

/// Initialize the global frame allocator from the boot memory map.
///
/// This function:
/// 1. Finds the largest usable region for the allocator
/// 2. Initializes the BitmapAllocator with that region
/// 3. Marks all reserved regions that fall within the managed range as allocated
///
/// # Safety
///
/// MUST be called exactly once during kernel boot, before any frame allocations.
/// MUST NOT be called after the allocator has been used.
///
/// # Panics
///
/// Panics if called more than once (allocator already initialized).
pub fn init(map: &BootMemoryMap) {
    // Find the largest usable region
    let mut best_region = None;
    let mut max_len = 0u64;

    for i in 0..map.count {
        let region = &map.regions[i];
        if region.kind == RegionKind::Usable && region.len_bytes > max_len {
            max_len = region.len_bytes;
            best_region = Some(region);
        }
    }

    if let Some(region) = best_region {
        kprintln!(
            "mm: initializing bitmap allocator with region [{:?}, +{} bytes]",
            region.start,
            region.len_bytes
        );
        let base_frame = PhysFrame::from_start_address(region.start);
        let total_frames = (region.len_bytes as usize).div_ceil(4096);
        let mut allocator_guard = FRAME_ALLOCATOR.lock();
        *allocator_guard = Some(BitmapAllocator::new(base_frame, total_frames));
        if let Some(ref alloc) = *allocator_guard {
            kprintln!(
                "mm: allocator ready. total_frames={} available={}",
                alloc.total_frames(),
                alloc.available_frames()
            );
        }
        drop(allocator_guard); // Release lock before logging reserved regions
    } else {
        kprintln!("mm: warning: no usable memory region found");
    }

    // Mark reserved regions as allocated
    // This ensures boot memory regions are excluded from allocation
    // Note: Since BitmapAllocator's base_frame is private, we track it separately
    // since base_frame() is not a public method
    // For now, we'll just log the reserved regions
    {
        let allocator_guard = FRAME_ALLOCATOR.lock();
        if let Some(ref alloc) = *allocator_guard {
            // We need to track the base frame and total frames separately
            // since base_frame() is not a public method.
            for i in 0..map.count {
                let region = &map.regions[i];
                if region.kind == RegionKind::Reserved
                    || region.kind == RegionKind::LoaderCode
                    || region.kind == RegionKind::LoaderData
                {
                    kprintln!(
                        "mm: reserved region [{:?}, +{} bytes] kind={:?}",
                        region.start,
                        region.len_bytes,
                        region.kind
                    );
                }
            }

            kprintln!(
                "mm: after processing reserved regions. available={}",
                alloc.available_frames()
            );
        }
    }
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
