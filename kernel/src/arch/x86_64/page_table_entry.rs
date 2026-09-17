//! Pure x86 page-entry encoding, also tested on non-x86 hosts.
#![allow(dead_code)]
use crate::mm::address::PhysAddr;

/// Page table entry for x86_64 4-level paging.
///
/// Each entry is 64 bits and contains a physical address along with
/// various control flags.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct PageTableEntry {
    pub(crate) value: u64,
}

#[allow(dead_code)]
impl PageTableEntry {
    /// Physical-address bits, excluding both low flags and high NX.
    const ADDRESS_MASK: u64 = 0x000F_FFFF_FFFF_F000;
    const FLAG_MASK: u64 = 0xFFF | Self::NO_EXECUTE;

    /// Present bit - must be set for the entry to be used.
    pub(crate) const PRESENT: u64 = 1 << 0;
    /// Writable bit - if clear, writes are not allowed.
    pub(crate) const WRITABLE: u64 = 1 << 1;
    /// User/supervisor bit - if clear, only supervisor (CPL 0) can access.
    pub(crate) const USER: u64 = 1 << 2;
    /// Page-level write-through - if set, write-through caching is used.
    pub(crate) const WRITE_THROUGH: u64 = 1 << 3;
    /// Page-level cache disable - if set, the page is not cached.
    pub(crate) const CACHE_DISABLE: u64 = 1 << 4;
    /// Accessed bit - set by hardware when the page is accessed.
    pub(crate) const ACCESSED: u64 = 1 << 5;
    /// Dirty bit - set by hardware when the page is written to.
    pub(crate) const DIRTY: u64 = 1 << 6;
    /// Page size bit - for PD entries, indicates a 2MB huge page.
    pub(crate) const HUGE_PAGE: u64 = 1 << 7;
    /// Global bit - if set, the entry is not flushed on CR3 write.
    pub(crate) const GLOBAL: u64 = 1 << 8;
    /// No-execute bit - if set, instruction fetch from the page is not allowed.
    pub(crate) const NO_EXECUTE: u64 = 1u64 << 63;

    /// Create a new unused page table entry.
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self { value: 0 }
    }

    /// Check if the entry is present (in use).
    #[must_use]
    pub(crate) fn is_present(&self) -> bool {
        self.value & Self::PRESENT != 0
    }

    /// Check if the entry is unused (not present).
    #[must_use]
    pub(crate) fn is_unused(&self) -> bool {
        self.value == 0
    }

    /// Set the physical address for this entry.
    ///
    /// # Panics
    ///
    /// Panics if the address is not page-aligned.
    pub(crate) fn set_addr(&mut self, addr: PhysAddr) {
        assert!(
            addr.is_page_aligned(),
            "Page table entry address must be page-aligned"
        );
        // Clear the lower 12 bits (page offset) and set the new address
        self.value = (self.value & Self::FLAG_MASK) | (addr.as_u64() & Self::ADDRESS_MASK);
    }

    /// Get the physical address from this entry.
    #[must_use]
    pub(crate) fn addr(&self) -> PhysAddr {
        // Exclude flags at both ends of the entry.
        // SAFETY: The address bits from a page table entry are valid physical addresses
        unsafe { PhysAddr::new(self.value & Self::ADDRESS_MASK) }
    }

    /// Set the flags for this entry.
    pub(crate) fn set_flags(&mut self, flags: u64) {
        // Preserve the address bits and set new flags
        self.value = (self.value & Self::ADDRESS_MASK) | (flags & Self::FLAG_MASK);
    }

    /// Get the flags from this entry.
    #[must_use]
    pub(crate) fn flags(&self) -> u64 {
        self.value & Self::FLAG_MASK
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn review_nx_survives_entry_updates() {
        let mut entry = PageTableEntry::new();
        entry.set_addr(unsafe { PhysAddr::new(0x1000) });
        entry.set_flags(PageTableEntry::PRESENT | PageTableEntry::NO_EXECUTE);
        assert_ne!(entry.value & PageTableEntry::NO_EXECUTE, 0);
        entry.set_addr(unsafe { PhysAddr::new(0x2000) });
        assert_ne!(entry.value & PageTableEntry::NO_EXECUTE, 0);
        assert_eq!(entry.addr().as_u64(), 0x2000);
        assert_ne!(entry.flags() & PageTableEntry::NO_EXECUTE, 0);
        entry.set_flags(PageTableEntry::PRESENT);
        assert_eq!(entry.value & PageTableEntry::NO_EXECUTE, 0);
        assert_eq!(entry.addr().as_u64(), 0x2000);
    }
}
