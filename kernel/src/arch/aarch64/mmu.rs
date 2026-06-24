//! aarch64-specific MMU implementation for shared-memory data-plane.
//!
//! This module implements the Mmu trait for aarch64 architecture using 4-level
//! paging (L0 → L1 → L2 → L3 → Physical Page). All operations are unsafe
//! because they perform direct hardware manipulation of page tables and TLB.
//!
//! # Page Table Structure
//!
//! ```text
//! L0 (Translation Table Level 0) → L1 (Level 1) → L2 (Level 2) → L3 (Level 3) → Physical Page
//! ```
//!
//! # Safety
//!
//! All trait methods are `unsafe` because they perform direct hardware
//! manipulation of page tables and TLB. Callers must ensure:
//!
//! - Page table addresses are valid and properly aligned (4 KiB for granule)
//! - The caller holds any necessary locks for concurrent access
//! - Virtual addresses are within valid ranges for the architecture
//! - Physical addresses point to valid memory or MMIO regions
//! - TLB invalidation (TLBI) and barriers (DSB/ISB) are performed after modifications
#![cfg_attr(not(target_arch = "aarch64"), allow(dead_code, unused_imports))]

use core::arch::asm;

#[cfg(test)]
use crate::arch::mmu::RIGHTS_READ;
use crate::arch::mmu::{
    CACHE_MODE_UNCACHED, CACHE_MODE_WRITE_BACK, CACHE_MODE_WRITE_COMBINE, Mmu, MmuError,
    RIGHTS_EXECUTE, RIGHTS_WRITE, VirtAddr,
};
use crate::domain_registry::{DomainId, MAX_DOMAINS};
use crate::mm::address::PhysAddr;
use crate::mm::address::PhysFrame;
use crate::mm::frame::FrameAllocator;

/// Page table entry for aarch64 4-level paging.
///
/// Each entry is 64 bits and contains a physical address along with
/// various control flags.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct PageTableEntry {
    value: u64,
}

impl PageTableEntry {
    /// Valid bit - must be set for the entry to be used.
    const VALID: u64 = 1 << 0;
    /// Table descriptor bit - indicates next-level table.
    const TABLE: u64 = 1 << 1;
    /// Block/page descriptor at levels where the table bit is clear.
    #[cfg(test)]
    const BLOCK: u64 = 0;
    /// Access flag - must be set for access.
    const AF: u64 = 1 << 10;
    /// Inner shareable.
    const SH_INNER: u64 = 0b11 << 8;
    /// Read/write at EL1.
    const AP_RW: u64 = 0b00 << 6;
    /// Read-only at EL1.
    const AP_RO: u64 = 0b10 << 6;
    /// Normal memory attribute.
    const ATTRINDX_NORMAL: u64 = 0b111 << 2;
    /// Device memory attribute.
    const ATTRINDX_DEVICE: u64 = 0b000 << 2;
    /// Non-cacheable memory attribute.
    const ATTRINDX_NC: u64 = 0b010 << 2;
    /// Privileged execute-never.
    const PXN: u64 = 1 << 53;
    /// Unprivileged execute-never.
    const UXN: u64 = 1 << 54;
    /// Create a new unused page table entry.
    #[must_use]
    const fn new() -> Self {
        Self { value: 0 }
    }

    /// Check if the entry is valid (in use).
    #[must_use]
    fn is_valid(&self) -> bool {
        self.value & Self::VALID != 0
    }

    /// Check if the entry is unused (not valid).
    #[must_use]
    #[cfg(test)]
    fn is_unused(&self) -> bool {
        self.value == 0
    }

    /// Set the physical address for this entry.
    ///
    /// # Panics
    ///
    /// Panics if the address is not page-aligned.
    fn set_addr(&mut self, addr: PhysAddr) {
        assert!(
            addr.is_page_aligned(),
            "Page table entry address must be page-aligned"
        );
        // Clear the lower 12 bits (page offset) and set the new address
        self.value = (self.value & 0xFFF) | addr.as_u64();
    }

    /// Get the physical address from this entry.
    #[must_use]
    fn addr(&self) -> PhysAddr {
        // Mask out the lower 12 bits (flags) to get the physical address
        // For aarch64, bits [47:12] contain the physical address
        // SAFETY: The masked value is a valid physical address from page table entry
        unsafe { PhysAddr::new(self.value & 0x0000_FFFF_FFFF_F000) }
    }

    /// Set the flags for this entry.
    fn set_flags(&mut self, flags: u64) {
        // Preserve the address bits and set new flags
        self.value = (self.value & 0x0000_FFFF_FFFF_F000) | (flags & 0xFFF);
    }

    /// Get the flags from this entry.
    #[must_use]
    #[cfg(test)]
    fn flags(&self) -> u64 {
        self.value & 0xFFF
    }

    /// Check if this entry is a table descriptor.
    #[must_use]
    #[cfg(test)]
    fn is_table(&self) -> bool {
        self.is_valid() && (self.value & Self::TABLE != 0)
    }

    /// Check if this entry is a block descriptor.
    #[must_use]
    #[cfg(test)]
    fn is_block(&self) -> bool {
        self.is_valid() && (self.value & Self::TABLE == 0)
    }
}

/// Page table structure for aarch64 4-level paging.
///
/// Each page table contains 512 entries (512 * 8 bytes = 4096 bytes).
#[repr(C, align(4096))]
struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    /// Create a new page table with all entries unused.
    #[cfg(test)]
    const fn new() -> Self {
        Self {
            entries: [PageTableEntry::new(); 512],
        }
    }
}

/// aarch64 MMU implementation.
///
/// This type implements the Mmu trait for aarch64 architecture.
pub struct AArch64Mmu;

#[cfg_attr(test, allow(dead_code))]
impl AArch64Mmu {
    /// Convert rights and cache mode to page table entry flags.
    #[must_use]
    fn rights_to_flags(rights: u32, cache_mode: u32) -> u64 {
        let mut flags = PageTableEntry::VALID | PageTableEntry::AF;

        // Access permissions
        if rights & RIGHTS_WRITE != 0 {
            flags |= PageTableEntry::AP_RW;
        } else {
            flags |= PageTableEntry::AP_RO;
        }

        // Execute permissions
        if rights & RIGHTS_EXECUTE == 0 {
            flags |= PageTableEntry::PXN | PageTableEntry::UXN;
        }

        // Shareability
        flags |= PageTableEntry::SH_INNER;

        // Cache mode mapping
        let attridx = match cache_mode {
            CACHE_MODE_UNCACHED => PageTableEntry::ATTRINDX_DEVICE,
            CACHE_MODE_WRITE_COMBINE => PageTableEntry::ATTRINDX_NC,
            CACHE_MODE_WRITE_BACK => PageTableEntry::ATTRINDX_NORMAL,
            _ => PageTableEntry::ATTRINDX_NORMAL,
        };
        flags |= attridx;

        flags
    }

    /// Get the L0 index from a virtual address.
    #[must_use]
    fn l0_index(vaddr: VirtAddr) -> usize {
        ((vaddr.as_u64() >> 39) & 0x1FF) as usize
    }

    /// Get the L1 index from a virtual address.
    #[must_use]
    fn l1_index(vaddr: VirtAddr) -> usize {
        ((vaddr.as_u64() >> 30) & 0x1FF) as usize
    }

    /// Get the L2 index from a virtual address.
    #[must_use]
    fn l2_index(vaddr: VirtAddr) -> usize {
        ((vaddr.as_u64() >> 21) & 0x1FF) as usize
    }

    /// Get the L3 index from a virtual address.
    #[must_use]
    fn l3_index(vaddr: VirtAddr) -> usize {
        ((vaddr.as_u64() >> 12) & 0x1FF) as usize
    }

    /// Load TTBR0_EL1 with the specified page table root address.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `ttbr0_value` is a valid physical address
    /// of a page table root.
    unsafe fn load_ttbr0(ttbr0_value: u64) {
        // SAFETY: The caller ensures ttbr0_value is a valid page table root.
        asm!("msr ttbr0_el1, {}", in(reg) ttbr0_value, options(nomem, nostack));
    }

    /// Read the current TTBR0_EL1 value.
    ///
    /// # Safety
    ///
    /// This is safe to call at any time, but the returned value should
    /// only be used in contexts where the caller knows the page table layout.
    #[must_use]
    unsafe fn read_ttbr0() -> u64 {
        let ttbr0_value: u64;
        // SAFETY: Reading TTBR0 is always safe.
        asm!("mrs {}, ttbr0_el1", out(reg) ttbr0_value, options(nomem, nostack));
        ttbr0_value
    }

    /// Invalidate all TLB entries (TLBI VMALLE1IS).
    ///
    /// # Safety
    ///
    /// The caller must ensure that no code is executing from pages
    /// that will be invalidated.
    #[inline(always)]
    unsafe fn tlbi_vmalle1is() {
        // SAFETY: The caller ensures it's safe to invalidate all TLB entries.
        asm!("tlbi vmalle1is", options(nomem, nostack));
    }

    /// Invalidate TLB entry for a specific address (TLBI VAAE1IS).
    ///
    /// # Safety
    ///
    /// The caller must ensure that `vaddr` is a valid virtual address
    /// that may have been mapped or unmapped.
    #[inline(always)]
    unsafe fn tlbi_vaae1is(vaddr: VirtAddr) {
        // SAFETY: The caller ensures vaddr is valid for invalidation.
        asm!("tlbi vaae1is, {}", in(reg) vaddr.as_u64() >> 12, options(nomem, nostack));
    }

    /// Data synchronization barrier inner shareable (DSB IS).
    ///
    /// # Safety
    ///
    /// This is safe to call at any time.
    #[inline(always)]
    unsafe fn dsb_is() {
        // SAFETY: DSB IS is always safe.
        asm!("dsb ish", options(nomem, nostack));
    }

    /// Instruction synchronization barrier (ISB).
    ///
    /// # Safety
    ///
    /// This is safe to call at any time.
    #[inline(always)]
    unsafe fn isb() {
        // SAFETY: ISB is always safe.
        asm!("isb", options(nomem, nostack));
    }
}

impl Mmu for AArch64Mmu {
    unsafe fn map_pages(
        domain_id: DomainId,
        vaddr: VirtAddr,
        frames: &[PhysFrame],
        rights: u32,
        cache_mode: u32,
    ) -> Result<(), MmuError> {
        // Validate domain_id and get page table root
        // Note: For now, we use a simple approach - assume domain 0 has a known page table
        // In a full implementation, we would use AddressSpaceTable to look up the root
        if domain_id >= MAX_DOMAINS as u64 {
            return Err(MmuError::InvalidDomain);
        }

        // Validate virtual address alignment
        if !vaddr.is_page_aligned() {
            return Err(MmuError::InvalidAddress);
        }

        // Validate frames slice is not empty
        if frames.is_empty() {
            return Err(MmuError::InvalidAddress);
        }

        #[cfg(test)]
        {
            // Host tests validate control-plane behavior, not real page table mutation.
            // Avoid dereferencing physical addresses or issuing privileged instructions.
            if domain_id != 0 {
                let table_guard = crate::mm::ADDRESS_SPACE_TABLE.lock();
                if let Some(ref table) = *table_guard {
                    if table
                        .get_root(domain_id as crate::domain_registry::DomainId)
                        .is_none()
                    {
                        return Err(MmuError::InvalidDomain);
                    }
                } else {
                    return Err(MmuError::InvalidDomain);
                }
                drop(table_guard);
            }
            let _ = (vaddr, rights, cache_mode);
            Ok(())
        }

        #[cfg(not(test))]
        {
            // Get page table root for the domain
            // For domain 0, we use the current TTBR0 value (identity mapping)
            // For other domains, we look up in AddressSpaceTable
            let l0_phys = if domain_id == 0 {
                Self::read_ttbr0() & 0x0000_FFFF_FFFF_F000
            } else {
                // Look up from AddressSpaceTable
                let table_guard = crate::mm::ADDRESS_SPACE_TABLE.lock();
                let result = match *table_guard {
                    Some(ref table) => {
                        match table.get_root(domain_id as crate::domain_registry::DomainId) {
                            Some(root) => root.as_u64() & 0x0000_FFFF_FFFF_F000,
                            None => return Err(MmuError::InvalidDomain),
                        }
                    }
                    None => return Err(MmuError::InvalidDomain),
                };
                drop(table_guard);
                result
            };

            // Convert physical address to virtual address for direct access
            // This assumes identity mapping for the kernel
            let l0_virt = l0_phys as *mut PageTable;

            // Convert rights and cache mode to page table flags
            let flags = Self::rights_to_flags(rights, cache_mode);

            // Map each frame to a virtual page
            for (i, frame) in frames.iter().enumerate() {
                // SAFETY: Calculated virtual address is within the valid range for this mapping
                let current_vaddr = unsafe { VirtAddr::new(vaddr.as_u64() + (i as u64 * 4096)) };
                let phys_addr = frame.start_address();

                // Walk the page table hierarchy
                let l0 = &mut *l0_virt;
                let l0_idx = Self::l0_index(current_vaddr);

                // Get or create L1 entry
                let l1 = if !l0.entries[l0_idx].is_valid() {
                    // Allocate a new L1 table
                    let l1_phys = AArch64Mmu.allocate_page_table()?;
                    // Set the L0 entry to point to the new L1 table
                    // Flags: VALID | TABLE (for next-level table descriptor)
                    l0.entries[l0_idx].set_addr(l1_phys);
                    l0.entries[l0_idx].set_flags(PageTableEntry::VALID | PageTableEntry::TABLE);
                    &mut *(l1_phys.as_u64() as *mut PageTable)
                } else {
                    let l1_phys = l0.entries[l0_idx].addr();
                    &mut *(l1_phys.as_u64() as *mut PageTable)
                };

                let l1_idx = Self::l1_index(current_vaddr);

                // Get or create L2 entry
                let l2 = if !l1.entries[l1_idx].is_valid() {
                    // Allocate a new L2 table
                    let l2_phys = AArch64Mmu.allocate_page_table()?;
                    // Set the L1 entry to point to the new L2 table
                    // Flags: VALID | TABLE (for next-level table descriptor)
                    l1.entries[l1_idx].set_addr(l2_phys);
                    l1.entries[l1_idx].set_flags(PageTableEntry::VALID | PageTableEntry::TABLE);
                    &mut *(l2_phys.as_u64() as *mut PageTable)
                } else {
                    let l2_phys = l1.entries[l1_idx].addr();
                    &mut *(l2_phys.as_u64() as *mut PageTable)
                };

                let l2_idx = Self::l2_index(current_vaddr);

                // Get or create L3 entry
                let l3 = if !l2.entries[l2_idx].is_valid() {
                    // Allocate a new L3 table
                    let l3_phys = AArch64Mmu.allocate_page_table()?;
                    // Set the L2 entry to point to the new L3 table
                    // Flags: VALID | TABLE (for next-level table descriptor)
                    l2.entries[l2_idx].set_addr(l3_phys);
                    l2.entries[l2_idx].set_flags(PageTableEntry::VALID | PageTableEntry::TABLE);
                    &mut *(l3_phys.as_u64() as *mut PageTable)
                } else {
                    let l3_phys = l2.entries[l2_idx].addr();
                    &mut *(l3_phys.as_u64() as *mut PageTable)
                };

                let l3_idx = Self::l3_index(current_vaddr);

                // Set the final page table entry
                let entry = &mut l3.entries[l3_idx];
                entry.set_addr(phys_addr);
                entry.set_flags(flags);

                // Invalidate the TLB entry for this page
                Self::tlbi_vaae1is(current_vaddr);
            }

            // Ensure TLB invalidation is complete
            Self::dsb_is();
            Self::isb();

            Ok(())
        }
    }

    unsafe fn unmap_pages(
        domain_id: DomainId,
        vaddr: VirtAddr,
        num_pages: usize,
    ) -> Result<(), MmuError> {
        // Validate domain_id
        if domain_id >= MAX_DOMAINS as u64 {
            return Err(MmuError::InvalidDomain);
        }

        // Validate virtual address alignment
        if !vaddr.is_page_aligned() {
            return Err(MmuError::InvalidAddress);
        }

        // Validate num_pages is not zero
        if num_pages == 0 {
            return Err(MmuError::InvalidAddress);
        }

        #[cfg(test)]
        {
            if domain_id != 0 {
                let table_guard = crate::mm::ADDRESS_SPACE_TABLE.lock();
                if let Some(ref table) = *table_guard {
                    if table
                        .get_root(domain_id as crate::domain_registry::DomainId)
                        .is_none()
                    {
                        return Err(MmuError::InvalidDomain);
                    }
                } else {
                    return Err(MmuError::InvalidDomain);
                }
                drop(table_guard);
            }
            let _ = (vaddr, num_pages);
            Ok(())
        }

        #[cfg(not(test))]
        {
            // Get page table root for the domain
            let l0_phys = if domain_id == 0 {
                Self::read_ttbr0() & 0x0000_FFFF_FFFF_F000
            } else {
                // Look up from AddressSpaceTable
                let table_guard = crate::mm::ADDRESS_SPACE_TABLE.lock();
                let result = match *table_guard {
                    Some(ref table) => {
                        match table.get_root(domain_id as crate::domain_registry::DomainId) {
                            Some(root) => root.as_u64() & 0x0000_FFFF_FFFF_F000,
                            None => return Err(MmuError::InvalidDomain),
                        }
                    }
                    None => return Err(MmuError::InvalidDomain),
                };
                drop(table_guard);
                result
            };

            // Convert physical address to virtual address for direct access
            let l0_virt = l0_phys as *mut PageTable;

            // Unmap each page
            for i in 0..num_pages {
                // SAFETY: Calculated virtual address is within the valid range for this mapping
                let current_vaddr = unsafe { VirtAddr::new(vaddr.as_u64() + (i as u64 * 4096)) };

                // Walk the page table hierarchy
                let l0 = &mut *l0_virt;
                let l0_idx = Self::l0_index(current_vaddr);

                // Check if L1 entry is present
                if !l0.entries[l0_idx].is_valid() {
                    // Page not mapped - continue to next page
                    continue;
                }

                let l1_phys = l0.entries[l0_idx].addr();
                let l1 = &mut *(l1_phys.as_u64() as *mut PageTable);
                let l1_idx = Self::l1_index(current_vaddr);

                // Check if L2 entry is present
                if !l1.entries[l1_idx].is_valid() {
                    continue;
                }

                let l2_phys = l1.entries[l1_idx].addr();
                let l2 = &mut *(l2_phys.as_u64() as *mut PageTable);
                let l2_idx = Self::l2_index(current_vaddr);

                // Check if L3 entry is present
                if !l2.entries[l2_idx].is_valid() {
                    continue;
                }

                let l3_phys = l2.entries[l2_idx].addr();
                let l3 = &mut *(l3_phys.as_u64() as *mut PageTable);
                let l3_idx = Self::l3_index(current_vaddr);

                // Clear the page table entry
                let entry = &mut l3.entries[l3_idx];
                *entry = PageTableEntry::new();

                // Invalidate the TLB entry for this page
                Self::tlbi_vaae1is(current_vaddr);
            }

            // Ensure TLB invalidation is complete
            Self::dsb_is();
            Self::isb();

            Ok(())
        }
    }

    unsafe fn flush_tlb(domain_id: DomainId) {
        #[cfg(not(test))]
        {
            // Validate domain_id
            if domain_id >= 16 {
                return;
            }

            // Get page table root for the domain
            let l0_phys = if domain_id == 0 {
                Self::read_ttbr0() & 0x0000_FFFF_FFFF_F000
            } else {
                // Look up from AddressSpaceTable (protected by Mutex)
                let table_guard = crate::mm::ADDRESS_SPACE_TABLE.lock();
                match *table_guard {
                    Some(ref table) => {
                        match table.get_root(domain_id as crate::domain_registry::DomainId) {
                            Some(root) => root.as_u64() & 0x0000_FFFF_FFFF_F000,
                            None => return,
                        }
                    }
                    None => return,
                }
            };

            // Invalidate all TLB entries for this address space
            Self::tlbi_vmalle1is();

            // Ensure TLB invalidation is complete
            Self::dsb_is();
            Self::isb();

            // Reload TTBR0 to ensure the page table is active
            Self::load_ttbr0(l0_phys);
        }

        #[cfg(test)]
        let _ = domain_id;
    }

    fn allocate_page_table(&self) -> Result<PhysAddr, MmuError> {
        // Allocate a single 4 KiB frame from the global allocator
        // SAFETY: The mutex ensures exclusive access to the allocator
        let mut allocator_guard = crate::mm::FRAME_ALLOCATOR.lock();
        let frame_opt = match *allocator_guard {
            Some(ref mut allocator) => FrameAllocator::allocate(allocator),
            None => return Err(MmuError::AllocationFailed),
        };
        drop(allocator_guard);

        match frame_opt {
            Some(frame) => {
                let phys_addr = PhysFrame::start_address(frame);

                // Zero-initialize the page table frame
                // SAFETY: The physical address is valid and page-aligned (from allocator).
                // We assume identity mapping for converting physical to virtual addresses.
                // We zero the entire 4 KiB page table (512 entries).
                unsafe {
                    let virt_ptr = phys_addr.as_u64() as *mut u8;
                    virt_ptr.write_bytes(0, crate::mm::address::PAGE_SIZE as usize);
                }

                Ok(phys_addr)
            }
            None => Err(MmuError::AllocationFailed),
        }
    }

    unsafe fn free_page_table(&self, phys_addr: PhysAddr) -> Result<(), MmuError> {
        // Convert physical address to frame
        let frame = PhysFrame::from_start_address(phys_addr);

        // Deallocate the frame back to the global allocator
        // SAFETY: The caller ensures the page table is no longer in use
        // SAFETY: The mutex ensures exclusive access to the allocator
        let mut allocator_guard = crate::mm::FRAME_ALLOCATOR.lock();
        match *allocator_guard {
            Some(ref mut allocator) => {
                FrameAllocator::deallocate(allocator, frame);
                Ok(())
            }
            None => Err(MmuError::AllocationFailed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_table_entry_new_creates_unused_entry() {
        let entry = PageTableEntry::new();
        assert!(entry.is_unused());
        assert!(!entry.is_valid());
    }

    #[test]
    fn page_table_entry_is_valid_detects_valid() {
        let mut entry = PageTableEntry::new();
        entry.set_flags(PageTableEntry::VALID);
        assert!(entry.is_valid());
    }

    #[test]
    fn page_table_entry_set_addr_requires_alignment() {
        let mut entry = PageTableEntry::new();
        let aligned = unsafe { PhysAddr::new(0x1000) };
        entry.set_addr(aligned);
        assert_eq!(entry.addr().as_u64(), 0x1000);
    }

    #[test]
    #[should_panic(expected = "must be page-aligned")]
    fn page_table_entry_set_addr_panics_on_misalignment() {
        let mut entry = PageTableEntry::new();
        let misaligned = unsafe { PhysAddr::new(0x1001) };
        entry.set_addr(misaligned);
    }

    #[test]
    fn page_table_entry_flags_preserved() {
        let mut entry = PageTableEntry::new();
        let flags = PageTableEntry::VALID | PageTableEntry::AF;
        entry.set_flags(flags);
        assert_eq!(entry.flags(), flags);
    }

    #[test]
    fn page_table_entry_is_table_detects_table() {
        let mut entry = PageTableEntry::new();
        entry.set_flags(PageTableEntry::VALID | PageTableEntry::TABLE);
        assert!(entry.is_table());
        assert!(!entry.is_block());
    }

    #[test]
    fn page_table_entry_is_block_detects_block() {
        let mut entry = PageTableEntry::new();
        entry.set_flags(PageTableEntry::VALID | PageTableEntry::BLOCK);
        assert!(entry.is_block());
        assert!(!entry.is_table());
    }

    #[test]
    fn page_table_new_creates_all_unused() {
        let pt = PageTable::new();
        for entry in pt.entries.iter() {
            assert!(entry.is_unused());
        }
    }

    #[test]
    fn rights_to_flags_includes_valid_and_af() {
        let flags = AArch64Mmu::rights_to_flags(0, CACHE_MODE_WRITE_BACK);
        assert!(flags & PageTableEntry::VALID != 0);
        assert!(flags & PageTableEntry::AF != 0);
    }

    #[test]
    fn rights_to_flags_sets_rw_for_write_rights() {
        let flags = AArch64Mmu::rights_to_flags(RIGHTS_WRITE, CACHE_MODE_WRITE_BACK);
        assert_eq!(flags & (0b11 << 6), PageTableEntry::AP_RW);
    }

    #[test]
    fn rights_to_flags_sets_ro_without_write_rights() {
        let flags = AArch64Mmu::rights_to_flags(RIGHTS_READ, CACHE_MODE_WRITE_BACK);
        assert!(flags & PageTableEntry::AP_RO != 0);
    }

    #[test]
    fn rights_to_flags_sets_execute_never_without_execute_rights() {
        let flags = AArch64Mmu::rights_to_flags(RIGHTS_READ | RIGHTS_WRITE, CACHE_MODE_WRITE_BACK);
        assert!(flags & PageTableEntry::PXN != 0);
        assert!(flags & PageTableEntry::UXN != 0);
    }

    #[test]
    fn rights_to_flags_clears_execute_never_with_execute_rights() {
        let flags = AArch64Mmu::rights_to_flags(
            RIGHTS_READ | RIGHTS_WRITE | RIGHTS_EXECUTE,
            CACHE_MODE_WRITE_BACK,
        );
        assert!(flags & PageTableEntry::PXN == 0);
        assert!(flags & PageTableEntry::UXN == 0);
    }

    #[test]
    fn rights_to_flags_sets_device_for_uncached() {
        let flags = AArch64Mmu::rights_to_flags(0, CACHE_MODE_UNCACHED);
        assert_eq!(flags & (0b111 << 2), PageTableEntry::ATTRINDX_DEVICE);
    }

    #[test]
    fn rights_to_flags_sets_nc_for_write_combine() {
        let flags = AArch64Mmu::rights_to_flags(0, CACHE_MODE_WRITE_COMBINE);
        assert!(flags & PageTableEntry::ATTRINDX_NC != 0);
    }

    #[test]
    fn rights_to_flags_sets_normal_for_write_back() {
        let flags = AArch64Mmu::rights_to_flags(0, CACHE_MODE_WRITE_BACK);
        assert!(flags & PageTableEntry::ATTRINDX_NORMAL != 0);
    }

    #[test]
    fn rights_to_flags_sets_inner_shareable() {
        let flags = AArch64Mmu::rights_to_flags(0, CACHE_MODE_WRITE_BACK);
        assert!(flags & PageTableEntry::SH_INNER != 0);
    }

    #[test]
    fn l0_index_extracted_correctly() {
        // Test with various addresses
        let vaddr = unsafe { VirtAddr::new(0xFFFF_8000_0000_0000) };
        let idx = AArch64Mmu::l0_index(vaddr);
        assert_eq!(idx, 0x100); // Higher half canonical address space split

        let vaddr = unsafe { VirtAddr::new(0x0000_0000_1234_5678) };
        let idx = AArch64Mmu::l0_index(vaddr);
        assert_eq!(idx, 0x000); // User space
    }

    #[test]
    fn l1_index_extracted_correctly() {
        let vaddr = unsafe { VirtAddr::new(0xFFFF_8000_0000_0000) };
        let idx = AArch64Mmu::l1_index(vaddr);
        assert_eq!(idx, 0x000);
    }

    #[test]
    fn l2_index_extracted_correctly() {
        let vaddr = unsafe { VirtAddr::new(0xFFFF_8000_0000_0000) };
        let idx = AArch64Mmu::l2_index(vaddr);
        assert_eq!(idx, 0x000);
    }

    #[test]
    fn l3_index_extracted_correctly() {
        let vaddr = unsafe { VirtAddr::new(0xFFFF_8000_0000_0000) };
        let idx = AArch64Mmu::l3_index(vaddr);
        assert_eq!(idx, 0x000);

        let vaddr = unsafe { VirtAddr::new(0xFFFF_8000_0000_1000) };
        let idx = AArch64Mmu::l3_index(vaddr);
        assert_eq!(idx, 0x001);
    }
}
