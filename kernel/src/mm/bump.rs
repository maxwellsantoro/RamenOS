//! Bump allocator for early boot memory allocation.
//!
//! This allocator provides simple linear allocation without deallocation
//! support. It is suitable for early boot phases when memory is abundant
//! and the complexity of tracking freed frames is unnecessary.
//!
//! ## Design Decision
//!
//! The bump allocator remains in the codebase because:
//! 1. Early boot (before bitmap allocator initialization) needs simple allocation
//! 2. Some test scenarios don't require deallocation
//! 3. It serves as a reference implementation of `FrameAllocator`
//!
//! ## Limitations
//!
//! - No deallocation support (frames are never returned to the pool)
//! - Memory cannot be reclaimed until the allocator is discarded
//! - Not suitable for long-running systems without periodic reset
//!
//! For production use cases requiring frame reuse, see
//! [`BitmapAllocator`](crate::mm::bitmap::BitmapAllocator).
//!
//! ## Security Note
//!
//! The `reset()` function was intentionally removed as part of vulnerability
//! V-009 remediation. Resetting an allocator while frames are still in use
//! leads to use-after-free vulnerabilities.

use super::address::{PAGE_SIZE, PhysAddr, PhysFrame};
use super::frame::FrameAllocator;

/// Maximum number of physical frames managed by the bump allocator.
///
/// 512 MiB / 4 KiB = 131072 frames.
/// This limit is sufficient for early boot and can be increased later.
const MAX_FRAMES: usize = 131072;

/// Bump allocator for physical frames.
///
/// Maintains a contiguous region of physical memory [base, base + size)
/// and allocates frames by moving a next pointer forward. Never frees.
///
/// # Invariants
///
/// - `next_frame_index <= total_frames`
/// - All frames in [base, next_frame_index) are allocated
/// - All frames in [next_frame_index, total_frames) are free
pub struct BumpAllocator {
    /// Base physical frame where allocation region starts.
    base_frame: PhysFrame,
    /// Number of frames managed by this allocator.
    total_frames: usize,
    /// Index of next free frame (0 = base_frame).
    next_frame_index: usize,
}

impl BumpAllocator {
    /// Create a new uninitialized bump allocator.
    ///
    /// This creates an allocator with no backing memory region.
    /// Call `add_region` to initialize it with actual memory.
    pub const fn new() -> Self {
        Self {
            base_frame: PhysFrame::from_frame_number(0),
            total_frames: 0,
            next_frame_index: 0,
        }
    }

    /// Add a memory region to the allocator.
    ///
    /// # Arguments
    ///
    /// * `start` - Physical start address of the memory region (must be page-aligned)
    /// * `size_bytes` - Size of the memory region in bytes (must be multiple of page size)
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - Base address is not page-aligned
    /// - Size is not a multiple of page size
    /// - Size exceeds MAX_FRAMES * PAGE_SIZE
    /// - Region already added
    pub fn add_region(&mut self, start: PhysAddr, size_bytes: usize) {
        assert!(self.total_frames == 0, "allocator region already set");
        assert!(
            start.as_u64().is_multiple_of(PAGE_SIZE),
            "base address must be page-aligned"
        );
        assert!(
            size_bytes.is_multiple_of(PAGE_SIZE as usize),
            "size must be multiple of page size"
        );

        let total_frames = size_bytes / PAGE_SIZE as usize;
        assert!(
            total_frames <= MAX_FRAMES,
            "memory region too large (max {} frames)",
            MAX_FRAMES
        );

        self.base_frame = PhysFrame::from_start_address(start);
        self.total_frames = total_frames;
        self.next_frame_index = 0;
    }
}

impl Default for BumpAllocator {
    fn default() -> Self {
        Self::new()
    }
}

// V-009: reset() function removed - resetting allocator is unsafe and leads to use-after-free

// SAFETY: This implementation ensures:
// - allocate() returns unique frames by monotonically incrementing next_frame
// - deallocation is a no-op (bump allocator doesn't support freeing)
// - The allocator is suitable for early boot when deallocation isn't needed
// - For production use requiring deallocation, see BitmapAllocator
unsafe impl FrameAllocator for BumpAllocator {
    fn allocate(&mut self) -> Option<PhysFrame> {
        if self.next_frame_index >= self.total_frames {
            return None;
        }

        let frame_index = self.next_frame_index;
        self.next_frame_index += 1;

        let frame_number = self.base_frame.frame_number() + frame_index as u64;
        Some(PhysFrame::from_frame_number(frame_number))
    }

    /// No-op deallocation for bump allocator.
    ///
    /// # Safety
    ///
    /// This is a no-op because the bump allocator does not track allocations.
    /// Frames allocated by this allocator cannot be reused until the allocator
    /// is discarded. This is intentional for early boot simplicity.
    unsafe fn deallocate(&mut self, _frame: PhysFrame) {
        // Intentional no-op: bump allocator does not support deallocation
    }

    fn available_frames(&self) -> usize {
        self.total_frames - self.next_frame_index
    }

    fn total_frames(&self) -> usize {
        self.total_frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_REGION_BASE: u64 = 0x1_0000_0000; // 4 GiB
    const TEST_REGION_SIZE: u64 = 1024 * 1024; // 1 MiB = 256 frames

    #[test]
    fn bump_allocator_new_creates_uninitialized_allocator() {
        let alloc = BumpAllocator::new();
        assert_eq!(alloc.total_frames(), 0);
        assert_eq!(alloc.available_frames(), 0);
    }

    #[test]
    fn bump_allocator_add_region_initializes_allocator() {
        let mut alloc = BumpAllocator::new();
        alloc.add_region(
            unsafe { PhysAddr::new(TEST_REGION_BASE) },
            TEST_REGION_SIZE as usize,
        );
        assert_eq!(alloc.total_frames(), 256);
        assert_eq!(alloc.available_frames(), 256);
    }

    #[test]
    #[should_panic(expected = "must be page-aligned")]
    fn bump_allocator_add_region_panics_on_misaligned_base() {
        let mut alloc = BumpAllocator::new();
        alloc.add_region(unsafe { PhysAddr::new(0x1001) }, PAGE_SIZE as usize);
    }

    #[test]
    #[should_panic(expected = "must be multiple of page size")]
    fn bump_allocator_add_region_panics_on_non_page_multiple_size() {
        let mut alloc = BumpAllocator::new();
        alloc.add_region(
            unsafe { PhysAddr::new(TEST_REGION_BASE) },
            PAGE_SIZE as usize + 1,
        );
    }

    #[test]
    fn bump_allocator_allocates_sequential_frames() {
        let mut alloc = BumpAllocator::new();
        alloc.add_region(
            unsafe { PhysAddr::new(TEST_REGION_BASE) },
            TEST_REGION_SIZE as usize,
        );

        let frame1 = alloc.allocate().unwrap();
        let frame2 = alloc.allocate().unwrap();
        let frame3 = alloc.allocate().unwrap();

        assert_eq!(frame1.frame_number(), 0x1_0000_0000 / PAGE_SIZE);
        assert_eq!(frame2.frame_number(), 0x1_0000_0000 / PAGE_SIZE + 1);
        assert_eq!(frame3.frame_number(), 0x1_0000_0000 / PAGE_SIZE + 2);
        assert_eq!(alloc.available_frames(), 253);
    }

    #[test]
    fn bump_allocator_exhaustion_returns_none() {
        let mut alloc = BumpAllocator::new();
        alloc.add_region(
            unsafe { PhysAddr::new(TEST_REGION_BASE) },
            (PAGE_SIZE * 10) as usize,
        );

        // Allocate all 10 frames
        for _ in 0..10 {
            assert!(alloc.allocate().is_some());
        }

        // Next allocation should fail
        assert!(alloc.allocate().is_none());
        assert_eq!(alloc.available_frames(), 0);
    }

    #[test]
    fn bump_allocator_deallocate_is_noop() {
        let mut alloc = BumpAllocator::new();
        alloc.add_region(
            unsafe { PhysAddr::new(TEST_REGION_BASE) },
            TEST_REGION_SIZE as usize,
        );

        let frame = alloc.allocate().unwrap();
        assert_eq!(alloc.available_frames(), 255);

        unsafe {
            alloc.deallocate(frame);
        }

        // Available frames unchanged (no-op)
        assert_eq!(alloc.available_frames(), 255);
    }

    // V-009: reset() test removed - reset() function removed for safety

    #[test]
    fn bump_allocator_tracks_total_frames() {
        let mut alloc = BumpAllocator::new();
        alloc.add_region(
            unsafe { PhysAddr::new(TEST_REGION_BASE) },
            TEST_REGION_SIZE as usize,
        );
        assert_eq!(alloc.total_frames(), 256);
    }

    #[test]
    fn bump_allocator_large_region() {
        let mut alloc = BumpAllocator::new();
        alloc.add_region(
            unsafe { PhysAddr::new(0x1000) },
            (MAX_FRAMES as u64 * PAGE_SIZE) as usize,
        );
        assert_eq!(alloc.total_frames(), MAX_FRAMES);
    }

    #[test]
    #[should_panic(expected = "memory region too large")]
    fn bump_allocator_panics_on_too_large_region() {
        let mut alloc = BumpAllocator::new();
        alloc.add_region(
            unsafe { PhysAddr::new(TEST_REGION_BASE) },
            ((MAX_FRAMES as u64 + 1) * PAGE_SIZE) as usize,
        );
    }
}
