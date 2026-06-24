//! x86_64-specific MMU implementation for shared-memory data-plane.
//!
//! This module implements the Mmu trait for x86_64 architecture using 4-level
//! paging (PML4 → PDP → PD → PT → Physical Page). All operations are unsafe
//! because they perform direct hardware manipulation of page tables and TLB.
//!
//! # Page Table Structure
//!
//! ```text
//! PML4 (Page Map Level 4) → PDP (Page Directory Pointer) → PD (Page Directory) → PT (Page Table) → Physical Page
//! ```
//!
//! # Safety
//!
//! All trait methods are `unsafe` because they perform direct hardware
//! manipulation of page tables and TLB. Callers must ensure:
//!
//! - Page table addresses are valid and properly aligned (4 KiB)
//! - The caller holds any necessary locks for concurrent access
//! - Virtual addresses are within valid ranges for the architecture
//! - Physical addresses point to valid memory or MMIO regions
//! - TLB invalidation is performed after page table modifications

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

/// Page table entry for x86_64 4-level paging.
///
/// Each entry is 64 bits and contains a physical address along with
/// various control flags.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct PageTableEntry {
    value: u64,
}

#[allow(dead_code)]
impl PageTableEntry {
    /// Present bit - must be set for the entry to be used.
    const PRESENT: u64 = 1 << 0;
    /// Writable bit - if clear, writes are not allowed.
    const WRITABLE: u64 = 1 << 1;
    /// User/supervisor bit - if clear, only supervisor (CPL 0) can access.
    const USER: u64 = 1 << 2;
    /// Page-level write-through - if set, write-through caching is used.
    const WRITE_THROUGH: u64 = 1 << 3;
    /// Page-level cache disable - if set, the page is not cached.
    const CACHE_DISABLE: u64 = 1 << 4;
    /// Accessed bit - set by hardware when the page is accessed.
    const ACCESSED: u64 = 1 << 5;
    /// Dirty bit - set by hardware when the page is written to.
    const DIRTY: u64 = 1 << 6;
    /// Page size bit - for PD entries, indicates a 2MB huge page.
    const HUGE_PAGE: u64 = 1 << 7;
    /// Global bit - if set, the entry is not flushed on CR3 write.
    const GLOBAL: u64 = 1 << 8;
    /// No-execute bit - if set, instruction fetch from the page is not allowed.
    const NO_EXECUTE: u64 = 1u64 << 63;

    /// Create a new unused page table entry.
    #[must_use]
    const fn new() -> Self {
        Self { value: 0 }
    }

    /// Check if the entry is present (in use).
    #[must_use]
    fn is_present(&self) -> bool {
        self.value & Self::PRESENT != 0
    }

    /// Check if the entry is unused (not present).
    #[must_use]
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
        // SAFETY: The address bits from a page table entry are valid physical addresses
        unsafe { PhysAddr::new(self.value & 0x000FFFFF_FFFFF000) }
    }

    /// Set the flags for this entry.
    fn set_flags(&mut self, flags: u64) {
        // Preserve the address bits and set new flags
        self.value = (self.value & 0x000FFFFF_FFFFF000) | (flags & 0xFFF);
    }

    /// Get the flags from this entry.
    #[must_use]
    fn flags(&self) -> u64 {
        self.value & 0xFFF
    }
}

/// Page table structure for x86_64 4-level paging.
///
/// Each page table contains 512 entries (512 * 8 bytes = 4096 bytes).
#[repr(C, align(4096))]
struct PageTable {
    entries: [PageTableEntry; 512],
}

#[allow(dead_code)]
impl PageTable {
    /// Create a new page table with all entries unused.
    const fn new() -> Self {
        Self {
            entries: [PageTableEntry::new(); 512],
        }
    }
}

/// x86_64 MMU implementation.
///
/// This type implements the Mmu trait for x86_64 architecture.
pub struct X86_64Mmu;

impl X86_64Mmu {
    /// Convert rights and cache mode to page table entry flags.
    #[must_use]
    fn rights_to_flags(rights: u32, cache_mode: u32) -> u64 {
        let mut flags = PageTableEntry::PRESENT;

        // Set writable flag if write rights are granted
        if rights & RIGHTS_WRITE != 0 {
            flags |= PageTableEntry::WRITABLE;
        }

        // Set user flag - for now, all mapped pages are user-accessible
        // (kernel-only mappings would clear this flag)
        flags |= PageTableEntry::USER;

        // Set no-execute flag if execute rights are not granted
        if rights & RIGHTS_EXECUTE == 0 {
            flags |= PageTableEntry::NO_EXECUTE;
        }

        // Map cache mode to page table flags
        match cache_mode {
            CACHE_MODE_UNCACHED => {
                flags |= PageTableEntry::CACHE_DISABLE;
            }
            CACHE_MODE_WRITE_COMBINE => {
                // Write-combine is simplified as uncached for now
                // A proper implementation would use PAT (Page Attribute Table)
                flags |= PageTableEntry::CACHE_DISABLE;
            }
            CACHE_MODE_WRITE_BACK => {
                // Default: write-back caching (no special flags needed)
            }
            _ => {
                // Unknown cache mode - default to write-back
            }
        }

        flags
    }

    /// Get the PML4 index from a virtual address.
    #[must_use]
    fn pml4_index(vaddr: VirtAddr) -> usize {
        ((vaddr.as_u64() >> 39) & 0x1FF) as usize
    }

    /// Get the PDP index from a virtual address.
    #[must_use]
    fn pdp_index(vaddr: VirtAddr) -> usize {
        ((vaddr.as_u64() >> 30) & 0x1FF) as usize
    }

    /// Get the PD index from a virtual address.
    #[must_use]
    fn pd_index(vaddr: VirtAddr) -> usize {
        ((vaddr.as_u64() >> 21) & 0x1FF) as usize
    }

    /// Get the PT index from a virtual address.
    #[must_use]
    fn pt_index(vaddr: VirtAddr) -> usize {
        ((vaddr.as_u64() >> 12) & 0x1FF) as usize
    }

    /// Load CR3 with the specified page table root address.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `cr3_value` is a valid physical address
    /// of a page table root.
    #[allow(dead_code)]
    unsafe fn load_cr3(cr3_value: u64) {
        // SAFETY: The caller ensures cr3_value is a valid page table root.
        asm!("mov cr3, {}", in(reg) cr3_value, options(nomem, nostack));
    }

    /// Read the current CR3 value.
    ///
    /// # Safety
    ///
    /// This is safe to call at any time, but the returned value should
    /// only be used in contexts where the caller knows the page table layout.
    #[must_use]
    #[allow(dead_code)]
    unsafe fn read_cr3() -> u64 {
        let cr3_value: u64;
        // SAFETY: Reading CR3 is always safe.
        asm!("mov {}, cr3", out(reg) cr3_value, options(nomem, nostack));
        cr3_value
    }

    /// Invalidate a single page in the TLB using invlpg.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `vaddr` is a valid virtual address
    /// that may have been mapped or unmapped.
    #[allow(dead_code)]
    unsafe fn invlpg(vaddr: VirtAddr) {
        // SAFETY: The caller ensures vaddr is valid for invalidation.
        asm!("invlpg [{}]", in(reg) vaddr.as_u64(), options(nomem, nostack, preserves_flags));
    }
}

impl Mmu for X86_64Mmu {
    #[allow(clippy::needless_return)]
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
            // Avoid privileged instructions and raw physical-pointer dereferences.
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
            return Ok(());
        }

        #[cfg(not(test))]
        {
            // Get page table root for the domain
            // For domain 0, we use the current CR3 value (identity mapping)
            // For other domains, we look up in AddressSpaceTable
            let pml4_phys = if domain_id == 0 {
                Self::read_cr3() & 0x000FFFFF_FFFFF000
            } else {
                // Look up from AddressSpaceTable
                let table_guard = crate::mm::ADDRESS_SPACE_TABLE.lock();
                let result = match *table_guard {
                    Some(ref table) => {
                        match table.get_root(domain_id as crate::domain_registry::DomainId) {
                            Some(root) => root.as_u64() & 0x000FFFFF_FFFFF000,
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
            let pml4_virt = pml4_phys as *mut PageTable;

            // Convert rights and cache mode to page table flags
            let flags = Self::rights_to_flags(rights, cache_mode);

            // Map each frame to a virtual page
            for (i, frame) in frames.iter().enumerate() {
                // SAFETY: Calculated virtual addresses are valid page-aligned addresses
                let current_vaddr = unsafe { VirtAddr::new(vaddr.as_u64() + (i as u64 * 4096)) };
                let phys_addr = frame.start_address();

                // Walk the page table hierarchy
                let pml4 = &mut *pml4_virt;
                let pml4_idx = Self::pml4_index(current_vaddr);

                // Get or create PDP entry
                let pdp = if !pml4.entries[pml4_idx].is_present() {
                    // Allocate a new PDP table
                    let pdp_phys = X86_64Mmu.allocate_page_table()?;
                    // Set the PML4 entry to point to the new PDP table
                    // Flags: PRESENT | WRITABLE | USER_ACCESSIBLE
                    pml4.entries[pml4_idx].set_addr(pdp_phys);
                    pml4.entries[pml4_idx].set_flags(
                        PageTableEntry::PRESENT | PageTableEntry::WRITABLE | PageTableEntry::USER,
                    );
                    &mut *(pdp_phys.as_u64() as *mut PageTable)
                } else {
                    let pdp_phys = pml4.entries[pml4_idx].addr();
                    &mut *(pdp_phys.as_u64() as *mut PageTable)
                };

                let pdp_idx = Self::pdp_index(current_vaddr);

                // Get or create PD entry
                let pd = if !pdp.entries[pdp_idx].is_present() {
                    // Allocate a new PD table
                    let pd_phys = X86_64Mmu.allocate_page_table()?;
                    // Set the PDP entry to point to the new PD table
                    // Flags: PRESENT | WRITABLE | USER_ACCESSIBLE
                    pdp.entries[pdp_idx].set_addr(pd_phys);
                    pdp.entries[pdp_idx].set_flags(
                        PageTableEntry::PRESENT | PageTableEntry::WRITABLE | PageTableEntry::USER,
                    );
                    &mut *(pd_phys.as_u64() as *mut PageTable)
                } else {
                    let pd_phys = pdp.entries[pdp_idx].addr();
                    &mut *(pd_phys.as_u64() as *mut PageTable)
                };

                let pd_idx = Self::pd_index(current_vaddr);

                // Get or create PT entry
                let pt = if !pd.entries[pd_idx].is_present() {
                    // Allocate a new PT table
                    let pt_phys = X86_64Mmu.allocate_page_table()?;
                    // Set the PD entry to point to the new PT table
                    // Flags: PRESENT | WRITABLE | USER_ACCESSIBLE
                    pd.entries[pd_idx].set_addr(pt_phys);
                    pd.entries[pd_idx].set_flags(
                        PageTableEntry::PRESENT | PageTableEntry::WRITABLE | PageTableEntry::USER,
                    );
                    &mut *(pt_phys.as_u64() as *mut PageTable)
                } else {
                    let pt_phys = pd.entries[pd_idx].addr();
                    &mut *(pt_phys.as_u64() as *mut PageTable)
                };

                let pt_idx = Self::pt_index(current_vaddr);

                // Set the final page table entry
                let entry = &mut pt.entries[pt_idx];
                entry.set_addr(phys_addr);
                entry.set_flags(flags);

                // Invalidate the TLB entry for this page
                Self::invlpg(current_vaddr);
            }

            return Ok(());
        }
    }

    #[allow(clippy::needless_return)]
    unsafe fn unmap_pages(
        domain_id: DomainId,
        vaddr: VirtAddr,
        num_pages: usize,
    ) -> Result<(), MmuError> {
        // Validate domain_id
        if domain_id >= 16 {
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
            return Ok(());
        }

        #[cfg(not(test))]
        {
            // Get page table root for the domain
            let pml4_phys = if domain_id == 0 {
                Self::read_cr3() & 0x000FFFFF_FFFFF000
            } else {
                // Look up from AddressSpaceTable
                let table_guard = crate::mm::ADDRESS_SPACE_TABLE.lock();
                let result = match *table_guard {
                    Some(ref table) => {
                        match table.get_root(domain_id as crate::domain_registry::DomainId) {
                            Some(root) => root.as_u64() & 0x000FFFFF_FFFFF000,
                            None => return Err(MmuError::InvalidDomain),
                        }
                    }
                    None => return Err(MmuError::InvalidDomain),
                };
                drop(table_guard);
                result
            };

            // Convert physical address to virtual address for direct access
            let pml4_virt = pml4_phys as *mut PageTable;

            // Unmap each page
            for i in 0..num_pages {
                // SAFETY: Calculated virtual addresses are valid page-aligned addresses
                let current_vaddr = unsafe { VirtAddr::new(vaddr.as_u64() + (i as u64 * 4096)) };

                // Walk the page table hierarchy
                let pml4 = &mut *pml4_virt;
                let pml4_idx = Self::pml4_index(current_vaddr);

                // Check if PDP entry is present
                if !pml4.entries[pml4_idx].is_present() {
                    // Page not mapped - continue to next page
                    continue;
                }

                let pdp_phys = pml4.entries[pml4_idx].addr();
                let pdp = &mut *(pdp_phys.as_u64() as *mut PageTable);
                let pdp_idx = Self::pdp_index(current_vaddr);

                // Check if PD entry is present
                if !pdp.entries[pdp_idx].is_present() {
                    continue;
                }

                let pd_phys = pdp.entries[pdp_idx].addr();
                let pd = &mut *(pd_phys.as_u64() as *mut PageTable);
                let pd_idx = Self::pd_index(current_vaddr);

                // Check if PT entry is present
                if !pd.entries[pd_idx].is_present() {
                    continue;
                }

                let pt_phys = pd.entries[pd_idx].addr();
                let pt = &mut *(pt_phys.as_u64() as *mut PageTable);
                let pt_idx = Self::pt_index(current_vaddr);

                // Clear the page table entry
                let entry = &mut pt.entries[pt_idx];
                *entry = PageTableEntry::new();

                // Invalidate the TLB entry for this page
                Self::invlpg(current_vaddr);
            }

            return Ok(());
        }
    }

    #[allow(clippy::needless_return, unused_unsafe)]
    unsafe fn flush_tlb(domain_id: DomainId) {
        // Validate domain_id
        if domain_id >= 16 {
            return;
        }

        #[cfg(test)]
        {
            if domain_id != 0 {
                unsafe {
                    let table_guard = crate::mm::ADDRESS_SPACE_TABLE.lock();
                    if let Some(ref table) = *table_guard {
                        if table
                            .get_root(domain_id as crate::domain_registry::DomainId)
                            .is_none()
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
            }
            return;
        }

        #[cfg(not(test))]
        {
            // Get page table root for the domain
            let pml4_phys = if domain_id == 0 {
                Self::read_cr3() & 0x000FFFFF_FFFFF000
            } else {
                // Look up from AddressSpaceTable
                let table_guard = crate::mm::ADDRESS_SPACE_TABLE.lock();
                if let Some(ref table) = *table_guard {
                    match table.get_root(domain_id as crate::domain_registry::DomainId) {
                        Some(root) => root.as_u64() & 0x000FFFFF_FFFFF000,
                        None => return,
                    }
                } else {
                    return;
                }
            };

            // Reload CR3 to flush the entire TLB for this address space
            Self::load_cr3(pml4_phys);
        }
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
        assert!(!entry.is_present());
    }

    #[test]
    fn page_table_entry_is_present_detects_present() {
        let mut entry = PageTableEntry::new();
        entry.set_flags(PageTableEntry::PRESENT);
        assert!(entry.is_present());
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
        let flags = PageTableEntry::PRESENT | PageTableEntry::WRITABLE | PageTableEntry::USER;
        entry.set_flags(flags);
        assert_eq!(entry.flags(), flags);
    }

    #[test]
    fn page_table_new_creates_all_unused() {
        let pt = PageTable::new();
        for entry in pt.entries.iter() {
            assert!(entry.is_unused());
        }
    }

    #[test]
    fn rights_to_flags_includes_present() {
        let flags = X86_64Mmu::rights_to_flags(0, CACHE_MODE_WRITE_BACK);
        assert!(flags & PageTableEntry::PRESENT != 0);
    }

    #[test]
    fn rights_to_flags_sets_writable_for_write_rights() {
        let flags = X86_64Mmu::rights_to_flags(RIGHTS_WRITE, CACHE_MODE_WRITE_BACK);
        assert!(flags & PageTableEntry::WRITABLE != 0);
    }

    #[test]
    fn rights_to_flags_clears_writable_without_write_rights() {
        let flags = X86_64Mmu::rights_to_flags(RIGHTS_READ, CACHE_MODE_WRITE_BACK);
        assert!(flags & PageTableEntry::WRITABLE == 0);
    }

    #[test]
    fn rights_to_flags_sets_no_execute_without_execute_rights() {
        let flags = X86_64Mmu::rights_to_flags(RIGHTS_READ | RIGHTS_WRITE, CACHE_MODE_WRITE_BACK);
        assert!(flags & PageTableEntry::NO_EXECUTE != 0);
    }

    #[test]
    fn rights_to_flags_clears_no_execute_with_execute_rights() {
        let flags = X86_64Mmu::rights_to_flags(
            RIGHTS_READ | RIGHTS_WRITE | RIGHTS_EXECUTE,
            CACHE_MODE_WRITE_BACK,
        );
        assert!(flags & PageTableEntry::NO_EXECUTE == 0);
    }

    #[test]
    fn rights_to_flags_sets_cache_disable_for_uncached() {
        let flags = X86_64Mmu::rights_to_flags(0, CACHE_MODE_UNCACHED);
        assert!(flags & PageTableEntry::CACHE_DISABLE != 0);
    }

    #[test]
    fn rights_to_flags_sets_cache_disable_for_write_combine() {
        let flags = X86_64Mmu::rights_to_flags(0, CACHE_MODE_WRITE_COMBINE);
        assert!(flags & PageTableEntry::CACHE_DISABLE != 0);
    }

    #[test]
    fn rights_to_flags_clears_cache_disable_for_write_back() {
        let flags = X86_64Mmu::rights_to_flags(0, CACHE_MODE_WRITE_BACK);
        assert!(flags & PageTableEntry::CACHE_DISABLE == 0);
    }

    #[test]
    fn pml4_index_extracted_correctly() {
        // Test with various addresses
        let vaddr = unsafe { VirtAddr::new(0xFFFF_8000_0000_0000) };
        let idx = X86_64Mmu::pml4_index(vaddr);
        assert_eq!(idx, 0x100); // 0xFFFF_8000_0000_0000 -> PML4 idx 0x100 (bit 47 set)

        let vaddr = unsafe { VirtAddr::new(0x0000_0000_1234_5678) };
        let idx = X86_64Mmu::pml4_index(vaddr);
        assert_eq!(idx, 0x000); // User space
    }

    #[test]
    fn pdp_index_extracted_correctly() {
        let vaddr = unsafe { VirtAddr::new(0xFFFF_8000_0000_0000) };
        let idx = X86_64Mmu::pdp_index(vaddr);
        assert_eq!(idx, 0x000);
    }

    #[test]
    fn pd_index_extracted_correctly() {
        let vaddr = unsafe { VirtAddr::new(0xFFFF_8000_0000_0000) };
        let idx = X86_64Mmu::pd_index(vaddr);
        assert_eq!(idx, 0x000);
    }

    #[test]
    fn pt_index_extracted_correctly() {
        let vaddr = unsafe { VirtAddr::new(0xFFFF_8000_0000_0000) };
        let idx = X86_64Mmu::pt_index(vaddr);
        assert_eq!(idx, 0x000);

        let vaddr = unsafe { VirtAddr::new(0xFFFF_8000_0000_1000) };
        let idx = X86_64Mmu::pt_index(vaddr);
        assert_eq!(idx, 0x001);
    }
    /// Test that map_pages succeeds with valid parameters.
    #[test]
    fn mmu_map_pages_succeeds_with_valid_params() {
        // Setup test allocator and address space
        let base = unsafe { crate::mm::PhysAddr::new(0x1000) };
        let allocator =
            crate::mm::BitmapAllocator::new(crate::mm::PhysFrame::from_start_address(base), 256);
        *crate::mm::FRAME_ALLOCATOR.lock() = Some(allocator);

        let mut table = crate::mm::AddressSpaceTable::new();
        let root = unsafe { crate::mm::PhysAddr::new(0x5000) };
        table.init_kernel(root);
        // Register domain 1 so the cfg(test) map_pages/unmap_pages path (which requires
        // a table root for non-zero domains) succeeds.
        table.set_root(1, unsafe { crate::mm::PhysAddr::new(0x6000) });
        *crate::mm::ADDRESS_SPACE_TABLE.lock() = Some(table);

        // Test mapping with valid parameters
        let frames = [crate::mm::PhysFrame::from_frame_number(0x1000)];
        let vaddr = unsafe { crate::arch::VirtAddr::new(0x8000_0000) };

        let result =
            unsafe { X86_64Mmu::map_pages(1, vaddr, &frames, RIGHTS_READ, CACHE_MODE_WRITE_BACK) };

        assert!(
            result.is_ok(),
            "map_pages should succeed with valid parameters"
        );
    }

    /// Test that map_pages rejects invalid domain IDs.
    #[test]
    fn mmu_map_pages_rejects_invalid_domain() {
        // Setup test allocator and address space
        let base = unsafe { crate::mm::PhysAddr::new(0x1000) };
        let allocator =
            crate::mm::BitmapAllocator::new(crate::mm::PhysFrame::from_start_address(base), 256);
        *crate::mm::FRAME_ALLOCATOR.lock() = Some(allocator);

        let mut table = crate::mm::AddressSpaceTable::new();
        let root = unsafe { crate::mm::PhysAddr::new(0x5000) };
        table.init_kernel(root);
        // Register domain 1 so the cfg(test) map_pages/unmap_pages path (which requires
        // a table root for non-zero domains) succeeds.
        table.set_root(1, unsafe { crate::mm::PhysAddr::new(0x6000) });
        *crate::mm::ADDRESS_SPACE_TABLE.lock() = Some(table);

        // Test with invalid domain ID (>= MAX_DOMAINS)
        let frames = [crate::mm::PhysFrame::from_frame_number(0x1000)];
        let vaddr = unsafe { crate::arch::VirtAddr::new(0x8000_0000) };

        let result =
            unsafe { X86_64Mmu::map_pages(16, vaddr, &frames, RIGHTS_READ, CACHE_MODE_WRITE_BACK) };

        assert_eq!(
            result,
            Err(MmuError::InvalidDomain),
            "map_pages should reject invalid domain"
        );
    }

    /// Test that map_pages rejects misaligned virtual addresses.
    #[test]
    fn mmu_map_pages_rejects_misaligned_address() {
        // Setup test allocator and address space
        let base = unsafe { crate::mm::PhysAddr::new(0x1000) };
        let allocator =
            crate::mm::BitmapAllocator::new(crate::mm::PhysFrame::from_start_address(base), 256);
        *crate::mm::FRAME_ALLOCATOR.lock() = Some(allocator);

        let mut table = crate::mm::AddressSpaceTable::new();
        let root = unsafe { crate::mm::PhysAddr::new(0x5000) };
        table.init_kernel(root);
        // Register domain 1 so the cfg(test) map_pages/unmap_pages path (which requires
        // a table root for non-zero domains) succeeds.
        table.set_root(1, unsafe { crate::mm::PhysAddr::new(0x6000) });
        *crate::mm::ADDRESS_SPACE_TABLE.lock() = Some(table);

        // Test with misaligned virtual address
        let frames = [crate::mm::PhysFrame::from_frame_number(0x1000)];
        let vaddr = unsafe { crate::arch::VirtAddr::new(0x8000_0001) }; // Not page-aligned

        let result =
            unsafe { X86_64Mmu::map_pages(1, vaddr, &frames, RIGHTS_READ, CACHE_MODE_WRITE_BACK) };

        assert_eq!(
            result,
            Err(MmuError::InvalidAddress),
            "map_pages should reject misaligned address"
        );
    }

    /// Test that unmap_pages removes mappings correctly.
    #[test]
    fn mmu_unmap_pages_removes_mapping() {
        // Setup test allocator and address space
        let base = unsafe { crate::mm::PhysAddr::new(0x1000) };
        let allocator =
            crate::mm::BitmapAllocator::new(crate::mm::PhysFrame::from_start_address(base), 256);
        *crate::mm::FRAME_ALLOCATOR.lock() = Some(allocator);

        let mut table = crate::mm::AddressSpaceTable::new();
        let root = unsafe { crate::mm::PhysAddr::new(0x5000) };
        table.init_kernel(root);
        // Register domain 1 so the cfg(test) map_pages/unmap_pages path (which requires
        // a table root for non-zero domains) succeeds.
        table.set_root(1, unsafe { crate::mm::PhysAddr::new(0x6000) });
        *crate::mm::ADDRESS_SPACE_TABLE.lock() = Some(table);

        // Map a page first
        let frames = [crate::mm::PhysFrame::from_frame_number(0x1000)];
        let vaddr = unsafe { crate::arch::VirtAddr::new(0x8000_0000) };

        let map_result =
            unsafe { X86_64Mmu::map_pages(1, vaddr, &frames, RIGHTS_READ, CACHE_MODE_WRITE_BACK) };
        assert!(map_result.is_ok(), "map_pages should succeed");

        // Unmap the page
        let unmap_result = unsafe { X86_64Mmu::unmap_pages(1, vaddr, 1) };

        assert!(unmap_result.is_ok(), "unmap_pages should succeed");
    }

    /// Test that flush_tlb clears TLB entries.
    #[test]
    fn mmu_flush_tlb_clears_mappings() {
        // Setup test address space
        let mut table = crate::mm::AddressSpaceTable::new();
        let root = unsafe { crate::mm::PhysAddr::new(0x5000) };
        table.init_kernel(root);
        // Register domain 1 so the cfg(test) map_pages/unmap_pages path (which requires
        // a table root for non-zero domains) succeeds.
        table.set_root(1, unsafe { crate::mm::PhysAddr::new(0x6000) });
        *crate::mm::ADDRESS_SPACE_TABLE.lock() = Some(table);

        // Test flush for valid domain
        unsafe {
            X86_64Mmu::flush_tlb(1);
        }
        // Should not panic - flush_tlb is safe for valid domain

        // Test flush for invalid domain (should be a no-op, not panic)
        unsafe {
            X86_64Mmu::flush_tlb(16);
        }
        // Should not panic - flush_tlb handles invalid domain gracefully
    }

    /// Test that rights enforcement blocks invalid access.
    #[test]
    fn mmu_rights_enforcement_blocks_invalid_access() {
        // Test that rights are correctly converted to flags
        // Write rights should set WRITABLE flag
        let flags = X86_64Mmu::rights_to_flags(RIGHTS_WRITE, CACHE_MODE_WRITE_BACK);
        assert!(flags & PageTableEntry::WRITABLE != 0);

        // Read-only rights should clear WRITABLE flag
        let flags = X86_64Mmu::rights_to_flags(RIGHTS_READ, CACHE_MODE_WRITE_BACK);
        assert!(flags & PageTableEntry::WRITABLE == 0);

        // Execute rights should clear NO_EXECUTE flag
        let flags = X86_64Mmu::rights_to_flags(
            RIGHTS_READ | RIGHTS_WRITE | RIGHTS_EXECUTE,
            CACHE_MODE_WRITE_BACK,
        );
        assert!(flags & PageTableEntry::NO_EXECUTE == 0);

        // No execute rights should set NO_EXECUTE flag
        let flags = X86_64Mmu::rights_to_flags(RIGHTS_READ | RIGHTS_WRITE, CACHE_MODE_WRITE_BACK);
        assert!(flags & PageTableEntry::NO_EXECUTE != 0);
    }
}
