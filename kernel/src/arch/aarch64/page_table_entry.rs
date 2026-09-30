//! Pure AArch64 descriptor encoding, tested on either host architecture.
use crate::arch::mmu::{
    CACHE_MODE_UNCACHED, CACHE_MODE_WRITE_BACK, CACHE_MODE_WRITE_COMBINE, RIGHTS_EXECUTE,
    RIGHTS_WRITE,
};
use crate::mm::address::PhysAddr;
/// Page table entry for aarch64 4-level paging.
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
    const ADDRESS_MASK: u64 = 0x0000_FFFF_FFFF_F000;
    const FLAG_MASK: u64 = 0xFFF | Self::PXN | Self::UXN;
    /// Convert rights and cache mode to page table entry flags.
    #[must_use]
    pub(crate) fn page_flags(rights: u32, cache_mode: u32) -> u64 {
        let mut flags = PageTableEntry::VALID | PageTableEntry::TABLE | PageTableEntry::AF;

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

    /// Valid bit - must be set for the entry to be used.
    pub(crate) const VALID: u64 = 1 << 0;
    /// Table descriptor bit - indicates next-level table.
    pub(crate) const TABLE: u64 = 1 << 1;
    /// Block/page descriptor at levels where the table bit is clear.
    #[cfg(test)]
    pub(crate) const BLOCK: u64 = 0;
    /// Access flag - must be set for access.
    pub(crate) const AF: u64 = 1 << 10;
    /// Inner shareable.
    pub(crate) const SH_INNER: u64 = 0b11 << 8;
    /// Read/write at EL1.
    pub(crate) const AP_RW: u64 = 0b00 << 6;
    /// Read-only at EL1.
    pub(crate) const AP_RO: u64 = 0b10 << 6;
    /// Normal memory attribute.
    pub(crate) const ATTRINDX_NORMAL: u64 = 0b111 << 2;
    /// Device memory attribute.
    pub(crate) const ATTRINDX_DEVICE: u64 = 0b000 << 2;
    /// Non-cacheable memory attribute.
    pub(crate) const ATTRINDX_NC: u64 = 0b010 << 2;
    /// Privileged execute-never.
    pub(crate) const PXN: u64 = 1 << 53;
    /// Unprivileged execute-never.
    pub(crate) const UXN: u64 = 1 << 54;
    /// Create a new unused page table entry.
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self { value: 0 }
    }

    /// Check if the entry is valid (in use).
    #[must_use]
    pub(crate) fn is_valid(&self) -> bool {
        self.value & Self::VALID != 0
    }

    /// Check if the entry is unused (not valid).
    #[must_use]
    #[cfg(test)]
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
        // Mask out the lower 12 bits (flags) to get the physical address
        // For aarch64, bits [47:12] contain the physical address
        // SAFETY: The masked value is a valid physical address from page table entry
        unsafe { PhysAddr::new(self.value & Self::ADDRESS_MASK) }
    }

    /// Set the flags for this entry.
    pub(crate) fn set_flags(&mut self, flags: u64) {
        // Preserve the address bits and set new flags
        self.value = (self.value & Self::ADDRESS_MASK) | (flags & Self::FLAG_MASK);
    }

    /// Get the flags from this entry.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn flags(&self) -> u64 {
        self.value & Self::FLAG_MASK
    }

    /// Check if this entry is a table descriptor.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn is_table(&self) -> bool {
        self.is_valid() && (self.value & Self::TABLE != 0)
    }

    /// Check if this entry is a block descriptor.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn is_block(&self) -> bool {
        self.is_valid() && (self.value & Self::TABLE == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::mmu::RIGHTS_READ;
    #[test]
    fn review_aarch64_leaf_encoding_preserves_permissions() {
        let mut entry = PageTableEntry::new();
        entry.set_addr(unsafe { PhysAddr::new(0x5000_0000) });
        entry.set_flags(PageTableEntry::page_flags(
            RIGHTS_READ,
            CACHE_MODE_WRITE_BACK,
        ));
        assert_eq!(
            entry.value & 3,
            3,
            "L3 pages require the page descriptor type"
        );
        assert_eq!(
            entry.value & (PageTableEntry::PXN | PageTableEntry::UXN),
            PageTableEntry::PXN | PageTableEntry::UXN
        );
        entry.set_addr(unsafe { PhysAddr::new(0x6000_0000) });
        assert_eq!(entry.addr().as_u64(), 0x6000_0000);
        assert_eq!(
            entry.value & (PageTableEntry::PXN | PageTableEntry::UXN),
            PageTableEntry::PXN | PageTableEntry::UXN
        );
        entry.set_flags(PageTableEntry::page_flags(
            RIGHTS_READ | RIGHTS_EXECUTE,
            CACHE_MODE_WRITE_BACK,
        ));
        assert_eq!(entry.value & (PageTableEntry::PXN | PageTableEntry::UXN), 0);
        assert_eq!(entry.addr().as_u64(), 0x6000_0000);
    }
}
