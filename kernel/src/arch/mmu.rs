//! Architecture-agnostic MMU trait for shared-memory data-plane.
//!
//! This module defines the MMU trait that provides an abstraction for
//! programming page tables across different architectures (x86_64, aarch64).
//!
//! # Safety
//!
//! All trait methods are `unsafe` because they perform direct hardware
//! manipulation of page tables and TLB entries. The caller must ensure:
//! - Valid domain IDs
//! - Properly aligned virtual addresses
//! - Valid physical frames
//! - Correct rights and cache mode values

use crate::domain_registry::DomainId;
use crate::mm::address::PhysAddr;
use crate::mm::address::PhysFrame;

/// Page size for x86_64 and aarch64: 4 KiB (4096 bytes).
pub const PAGE_SIZE: u64 = 4096;

/// Virtual address wrapper type.
///
/// Provides type safety for virtual addresses, preventing them from being
/// confused with physical addresses or arbitrary integers.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(u64);

impl VirtAddr {
    /// Create a new virtual address.
    ///
    /// # Safety
    ///
    /// The caller must ensure the address is valid for the current context.
    pub const unsafe fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Returns the raw address value.
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Align this address down to the nearest page boundary.
    pub const fn align_down_to_page(self) -> Self {
        Self(self.0 & !(PAGE_SIZE - 1))
    }

    /// Align this address up to the nearest page boundary.
    pub const fn align_up_to_page(self) -> Self {
        Self((self.0 + PAGE_SIZE - 1) & !(PAGE_SIZE - 1))
    }

    /// Check if this address is page-aligned.
    pub const fn is_page_aligned(self) -> bool {
        self.0 & (PAGE_SIZE - 1) == 0
    }

    /// Calculate the offset from this address to another.
    ///
    /// # Panics
    ///
    /// Panics if `other` is less than `self`.
    pub const fn offset_from(self, other: VirtAddr) -> u64 {
        assert!(self.0 >= other.0, "offset underflow");
        self.0 - other.0
    }
}

/// MMU error types.
///
/// Represents errors that can occur during MMU operations.
#[derive(Debug, PartialEq)]
pub enum MmuError {
    /// The specified domain ID is invalid or not registered.
    InvalidDomain,
    /// The virtual address is invalid or not properly aligned.
    InvalidAddress,
    /// Failed to allocate memory for page table structures.
    AllocationFailed,
    /// The requested operation was denied due to permission restrictions.
    PermissionDenied,
}

/// Memory access rights constants.
///
/// These flags define the access permissions for mapped pages.
pub const RIGHTS_READ: u32 = 0x1;
pub const RIGHTS_WRITE: u32 = 0x2;
pub const RIGHTS_EXECUTE: u32 = 0x4;

/// Cache mode constants.
///
/// These flags define the caching behavior for mapped pages.
pub const CACHE_MODE_UNCACHED: u32 = 0;
pub const CACHE_MODE_WRITE_COMBINE: u32 = 1;
pub const CACHE_MODE_WRITE_BACK: u32 = 2;

/// Architecture-agnostic MMU trait.
///
/// This trait provides an abstraction for programming page tables across
/// different architectures. It allows the kernel to manage memory mappings
/// for domains in a portable way.
///
/// # Safety
///
/// All methods are `unsafe` because they perform direct hardware manipulation.
/// The caller must ensure:
/// - Valid domain IDs
/// - Properly aligned virtual addresses
/// - Valid physical frames
/// - Correct rights and cache mode values
pub trait Mmu {
    /// Map physical frames to virtual addresses for a domain.
    ///
    /// This method maps a contiguous range of physical frames to a contiguous
    /// range of virtual addresses with the specified access rights and cache mode.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - `domain_id` is a valid, registered domain
    /// - `vaddr` is page-aligned
    /// - All frames in `frames` are valid physical frames
    /// - `rights` is a valid combination of RIGHTS_* constants
    /// - `cache_mode` is one of the CACHE_MODE_* constants
    /// - The virtual address range does not overlap existing mappings
    ///
    /// # Errors
    ///
    /// Returns `MmuError::InvalidDomain` if the domain ID is invalid.
    /// Returns `MmuError::InvalidAddress` if the virtual address is not page-aligned.
    /// Returns `MmuError::AllocationFailed` if page table allocation fails.
    /// Returns `MmuError::PermissionDenied` if the operation is not allowed.
    unsafe fn map_pages(
        domain_id: DomainId,
        vaddr: VirtAddr,
        frames: &[PhysFrame],
        rights: u32,
        cache_mode: u32,
    ) -> Result<(), MmuError>;

    /// Unmap virtual address range for a domain.
    ///
    /// This method removes a contiguous range of virtual page mappings for a domain.
    /// The physical frames are not deallocated and may be reused.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - `domain_id` is a valid, registered domain
    /// - `vaddr` is page-aligned
    /// - `num_pages` is non-zero
    /// - The virtual address range is currently mapped
    /// - No code is executing from the unmapped pages
    ///
    /// # Errors
    ///
    /// Returns `MmuError::InvalidDomain` if the domain ID is invalid.
    /// Returns `MmuError::InvalidAddress` if the virtual address is not page-aligned.
    unsafe fn unmap_pages(
        domain_id: DomainId,
        vaddr: VirtAddr,
        num_pages: usize,
    ) -> Result<(), MmuError>;

    /// Flush TLB entries for a domain.
    ///
    /// This method invalidates TLB entries for the specified domain, ensuring
    /// that subsequent memory accesses use the updated page table mappings.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - `domain_id` is a valid, registered domain
    /// - No code is executing from pages that will be invalidated
    unsafe fn flush_tlb(domain_id: DomainId);

    /// Allocate a new page table (4 KiB frame).
    ///
    /// This method allocates a physical frame for use as a page table.
    /// The frame is zero-initialized to ensure all entries are initially unused.
    ///
    /// # Security Fix (NEW-004)
    ///
    /// The global FRAME_ALLOCATOR is now protected by `spin::Mutex` to ensure
    /// thread-safe access. This method acquires the mutex before allocating.
    ///
    /// # Preconditions
    ///
    /// The caller must ensure:
    /// - The global FRAME_ALLOCATOR has been initialized
    /// - The mutex will be acquired automatically by this method
    ///
    /// # Returns
    ///
    /// Returns `Ok(phys_addr)` with the physical address of the allocated page table,
    /// or `Err(MmuError::AllocationFailed)` if allocation fails.
    fn allocate_page_table(&self) -> Result<PhysAddr, MmuError>;

    /// Free a previously allocated page table.
    ///
    /// This method returns a page table frame to the allocator for reuse.
    ///
    /// # Arguments
    ///
    /// * `phys_addr` - Physical address of the page table to free
    ///
    /// # Security Fix (NEW-004)
    ///
    /// The global FRAME_ALLOCATOR is now protected by `spin::Mutex` to ensure
    /// thread-safe access. This method acquires the mutex before deallocating.
    ///
    /// # Preconditions
    ///
    /// The caller must ensure:
    /// - The global FRAME_ALLOCATOR has been initialized
    /// - The mutex will be acquired automatically by this method
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - The page table is no longer in use (not referenced by any active page table)
    /// - The page table was previously allocated via `allocate_page_table()`
    /// - No TLB entries reference the page table being freed
    unsafe fn free_page_table(&self, phys_addr: PhysAddr) -> Result<(), MmuError>;
}
