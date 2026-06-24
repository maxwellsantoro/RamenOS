//! Physical memory address types.
//!
//! Provides type-safe wrappers for physical addresses and 4KiB-aligned frames.
//! These newtype wrappers prevent physical addresses from being confused with
//! virtual addresses or arbitrary integers.

use core::fmt;

/// Page size for x86_64 and aarch64: 4 KiB (4096 bytes).
pub const PAGE_SIZE: u64 = 4096;

/// Strong type for a physical address.
///
/// Wraps a raw `u64` to provide type safety and prevent accidental mixing
/// with virtual addresses or other integer types.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(u64);

impl PhysAddr {
    /// Create a new physical address from a raw u64.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - `addr` is a valid physical memory address
    /// - If used for memory access, the address is properly aligned
    /// - The address is within the physical memory range supported by the system
    /// - For MMIO addresses, the corresponding device is present and configured
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
    pub const fn offset_from(self, other: PhysAddr) -> u64 {
        assert!(self.0 >= other.0, "offset underflow");
        self.0 - other.0
    }
}

/// Strong type for a 4KiB physical frame.
///
/// Represents a page-aligned 4KiB region of physical memory.
/// Used by frame allocators to manage physical memory.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysFrame {
    start_address: PhysAddr,
}

impl PhysFrame {
    /// Create a physical frame from a start address.
    ///
    /// # Panics
    ///
    /// Panics if the address is not page-aligned.
    pub fn from_start_address(address: PhysAddr) -> Self {
        assert!(
            address.is_page_aligned(),
            "frame start address must be page-aligned"
        );
        Self {
            start_address: address,
        }
    }

    /// Returns the start address of this frame.
    pub const fn start_address(self) -> PhysAddr {
        self.start_address
    }

    /// Returns the physical address as a raw u64.
    pub const fn as_u64(self) -> u64 {
        self.start_address.0
    }

    /// Returns the frame number (address divided by page size).
    pub const fn frame_number(self) -> u64 {
        self.start_address.0 / PAGE_SIZE
    }

    /// Create a frame from a frame number.
    pub const fn from_frame_number(n: u64) -> Self {
        Self {
            start_address: PhysAddr(n * PAGE_SIZE),
        }
    }

    /// Calculate the frame index from a base frame.
    ///
    /// # Panics
    ///
    /// Panics if `self` is before `base`.
    pub const fn index_from(self, base: PhysFrame) -> u64 {
        assert!(
            self.start_address.0 >= base.start_address.0,
            "frame index underflow"
        );
        (self.start_address.0 - base.start_address.0) / PAGE_SIZE
    }
}

impl fmt::Display for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PhysAddr(0x{:x})", self.0)
    }
}

impl fmt::Display for PhysFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PhysFrame(start=0x{:x}, num={})",
            self.start_address.0,
            self.frame_number()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phys_addr_new_wraps_value() {
        let addr = unsafe { PhysAddr::new(0x1000) };
        assert_eq!(addr.as_u64(), 0x1000);
    }

    #[test]
    fn phys_addr_align_down_rounds_to_page() {
        let addr = unsafe { PhysAddr::new(0x1234) };
        let aligned = addr.align_down_to_page();
        assert_eq!(aligned.as_u64(), 0x1000);
    }

    #[test]
    fn phys_addr_align_up_rounds_to_page() {
        let addr = unsafe { PhysAddr::new(0x1234) };
        let aligned = addr.align_up_to_page();
        assert_eq!(aligned.as_u64(), 0x2000);
    }

    #[test]
    fn phys_addr_is_aligned_detects_alignment() {
        let aligned = unsafe { PhysAddr::new(0x1000) };
        let misaligned = unsafe { PhysAddr::new(0x1001) };

        assert!(aligned.is_page_aligned());
        assert!(!misaligned.is_page_aligned());
    }

    #[test]
    fn phys_addr_offset_calculates_distance() {
        let addr1 = unsafe { PhysAddr::new(0x1000) };
        let addr2 = unsafe { PhysAddr::new(0x1500) };
        assert_eq!(addr2.offset_from(addr1), 0x500);
    }

    #[test]
    #[should_panic(expected = "offset underflow")]
    fn phys_addr_offset_panics_on_underflow() {
        let addr1 = unsafe { PhysAddr::new(0x2000) };
        let addr2 = unsafe { PhysAddr::new(0x1000) };
        let _ = addr2.offset_from(addr1);
    }

    #[test]
    fn phys_frame_from_start_address_requires_alignment() {
        let aligned = unsafe { PhysAddr::new(0x1000) };
        let frame = PhysFrame::from_start_address(aligned);
        assert_eq!(frame.start_address().as_u64(), 0x1000);
    }

    #[test]
    #[should_panic(expected = "must be page-aligned")]
    fn phys_frame_from_start_address_panics_on_misalignment() {
        let misaligned = unsafe { PhysAddr::new(0x1001) };
        let _ = PhysFrame::from_start_address(misaligned);
    }

    #[test]
    fn phys_frame_frame_number_divides_by_page_size() {
        let addr = unsafe { PhysAddr::new(0x1000) };
        let frame = PhysFrame::from_start_address(addr);
        assert_eq!(frame.frame_number(), 1);
    }

    #[test]
    fn phys_frame_from_frame_number_multiplies_by_page_size() {
        let frame = PhysFrame::from_frame_number(42);
        assert_eq!(frame.start_address().as_u64(), 42 * PAGE_SIZE);
    }

    #[test]
    fn phys_frame_index_from_calculates_offset() {
        let base = PhysFrame::from_frame_number(10);
        let frame = PhysFrame::from_frame_number(15);
        assert_eq!(frame.index_from(base), 5);
    }

    #[test]
    #[should_panic(expected = "frame index underflow")]
    fn phys_frame_index_from_panics_on_underflow() {
        let base = PhysFrame::from_frame_number(15);
        let frame = PhysFrame::from_frame_number(10);
        let _ = frame.index_from(base);
    }
}
