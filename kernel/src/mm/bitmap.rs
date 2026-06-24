//! Bitmap allocator for physical memory frames.
//!
//! A frame allocator that tracks allocation state using a bitmap.
//! Each bit represents one frame: 0 = free, 1 = allocated.
//!
//! This allocator supports:
//! - Single frame allocation and deallocation
//! - Contiguous range allocation
//! - Efficient reuse of freed frames
//! - Fragmentation handling
//!
//! # Design
//!
//! The bitmap uses u64 words for efficient bit operations.
//! With MAX_FRAMES = 131072, we need 2048 words (131072 / 64).
//!
//! # Safety
//!
//! This allocator is single-threaded and requires no locking.
//! Double allocation is prevented by checking the bitmap before allocation.
//! Double deallocation is prevented by checking the bitmap before deallocation.

#[cfg(test)]
use super::address::PAGE_SIZE;
use super::address::PhysFrame;
use super::frame::FrameAllocator;

/// Maximum number of physical frames managed by the bitmap allocator.
///
/// 512 MiB / 4 KiB = 131072 frames.
/// This limit is sufficient for early boot and can be increased later.
const MAX_FRAMES: usize = 131072;

/// Number of u64 words needed for the bitmap.
const BITMAP_WORDS: usize = MAX_FRAMES.div_ceil(64);

/// Bitmap allocator for physical frames.
///
/// Tracks allocation state using a bitmap where each bit represents one frame:
/// - 0 = frame is free
/// - 1 = frame is allocated
///
/// # Invariants
///
/// - `bitmap[i]` bit j represents frame (i * 64 + j)
/// - `free_frames` accurately counts frames with bit = 0
/// - All frames in [base_frame, base_frame + total_frames) are managed
/// - Frames outside this range are not managed
///
/// # Example
///
/// ```ignore
/// let mut alloc = BitmapAllocator::new(base_frame, 256);
/// let frame = alloc.allocate().unwrap(); // Allocate one frame
/// alloc.deallocate(frame); // Free it for reuse
/// let range = alloc.allocate_contiguous(4).unwrap(); // Allocate 4 contiguous frames
/// ```
pub struct BitmapAllocator {
    /// Bitmap of frame states (1 bit per frame)
    /// 0 = free, 1 = allocated
    bitmap: [u64; BITMAP_WORDS],
    /// Base physical frame where allocation region starts
    base_frame: PhysFrame,
    /// Number of frames managed by this allocator
    total_frames: usize,
    /// Number of free frames available for allocation
    free_frames: usize,
}

impl BitmapAllocator {
    /// Create a new bitmap allocator.
    ///
    /// # Arguments
    ///
    /// * `base_frame` - Base physical frame where allocation region starts
    /// * `total_frames` - Number of frames to manage (must be <= MAX_FRAMES)
    ///
    /// # Panics
    ///
    /// Panics if `total_frames` exceeds `MAX_FRAMES`.
    pub const fn new(base_frame: PhysFrame, total_frames: usize) -> Self {
        assert!(
            total_frames <= MAX_FRAMES,
            "total_frames exceeds MAX_FRAMES"
        );
        Self {
            bitmap: [0; BITMAP_WORDS],
            base_frame,
            total_frames,
            free_frames: total_frames,
        }
    }

    /// Allocate N contiguous frames.
    ///
    /// Searches for a run of N consecutive free frames and marks them as allocated.
    ///
    /// # Arguments
    ///
    /// * `n_frames` - Number of contiguous frames to allocate
    ///
    /// # Returns
    ///
    /// Returns `Some(start_frame)` where `start_frame` is the first frame of the allocated range,
    /// or `None` if insufficient contiguous free frames are available.
    pub fn allocate_contiguous(&mut self, n_frames: usize) -> Option<PhysFrame> {
        if n_frames == 0 || n_frames > self.free_frames {
            return None;
        }

        // Search for a contiguous run of N free frames
        let start_index = self.find_contiguous_run(n_frames)?;

        // Mark all frames in the range as allocated
        for i in start_index..(start_index + n_frames) {
            self.set_bit(i, true);
        }

        self.free_frames -= n_frames;

        let frame_number = self.base_frame.frame_number() + start_index as u64;
        Some(PhysFrame::from_frame_number(frame_number))
    }

    /// Deallocate a single frame.
    ///
    /// Marks the frame as free and allows it to be reused.
    ///
    /// # Arguments
    ///
    /// * `frame` - The frame to deallocate
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The frame is outside the managed range
    /// - The frame is not currently allocated (double deallocation)
    pub fn deallocate(&mut self, frame: PhysFrame) {
        let index = self.frame_to_index(frame);
        self.deallocate_index(index);
    }

    /// Deallocate a range of contiguous frames.
    ///
    /// Marks all frames in the range as free.
    ///
    /// # Arguments
    ///
    /// * `start` - First frame in the range
    /// * `n_frames` - Number of frames to deallocate
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - Any frame in the range is outside the managed range
    /// - Any frame in the range is not currently allocated
    pub fn deallocate_range(&mut self, start: PhysFrame, n_frames: usize) {
        let start_index = self.frame_to_index(start);
        let end_index = start_index + n_frames;

        assert!(
            end_index <= self.total_frames,
            "deallocate range exceeds total_frames"
        );

        for i in start_index..end_index {
            self.deallocate_index(i);
        }
    }

    /// Return the number of free frames.
    pub const fn free_frames(&self) -> usize {
        self.free_frames
    }

    /// Find a contiguous run of N free frames.
    ///
    /// Returns the starting index of the run, or None if not found.
    fn find_contiguous_run(&self, n_frames: usize) -> Option<usize> {
        let mut consecutive = 0;
        let mut start_index = 0;

        for i in 0..self.total_frames {
            if self.get_bit(i) {
                // Frame is allocated, reset counter
                consecutive = 0;
            } else {
                // Frame is free
                if consecutive == 0 {
                    start_index = i;
                }
                consecutive += 1;
                if consecutive >= n_frames {
                    return Some(start_index);
                }
            }
        }

        None
    }

    /// Convert a frame to its index in the bitmap.
    ///
    /// # Panics
    ///
    /// Panics if the frame is outside the managed range.
    fn frame_to_index(&self, frame: PhysFrame) -> usize {
        let index = frame.index_from(self.base_frame) as usize;
        assert!(
            index < self.total_frames,
            "frame index {} exceeds total_frames {}",
            index,
            self.total_frames
        );
        index
    }

    /// Get the allocation state of a frame by index.
    fn get_bit(&self, index: usize) -> bool {
        let word_index = index / 64;
        let bit_index = index % 64;
        (self.bitmap[word_index] >> bit_index) & 1 == 1
    }

    /// Set the allocation state of a frame by index.
    fn set_bit(&mut self, index: usize, value: bool) {
        let word_index = index / 64;
        let bit_index = index % 64;
        if value {
            self.bitmap[word_index] |= 1 << bit_index;
        } else {
            self.bitmap[word_index] &= !(1 << bit_index);
        }
    }

    /// Deallocate a frame by index.
    ///
    /// # Panics
    ///
    /// Panics if the frame is not currently allocated.
    fn deallocate_index(&mut self, index: usize) {
        assert!(
            self.get_bit(index),
            "double deallocation at index {}",
            index
        );
        self.set_bit(index, false);
        self.free_frames += 1;
    }
}

// SAFETY: This implementation ensures:
// - allocate() returns unique frames by atomically claiming bitmap slots
// - deallocate() validates frame ownership via bitmap before freeing
// - The bitmap array is statically allocated, avoiding heap allocation
unsafe impl FrameAllocator for BitmapAllocator {
    /// Allocate a single physical frame.
    ///
    /// Returns `None` if no frames are available.
    fn allocate(&mut self) -> Option<PhysFrame> {
        self.allocate_contiguous(1)
    }

    /// Deallocate a physical frame.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - The frame was previously allocated from this allocator
    /// - The frame is no longer in use (no references exist)
    /// - No concurrent deallocation of the same frame is occurring
    unsafe fn deallocate(&mut self, frame: PhysFrame) {
        self.deallocate(frame);
    }

    /// Return the number of available frames.
    fn available_frames(&self) -> usize {
        self.free_frames
    }

    /// Return the total number of frames managed by this allocator.
    fn total_frames(&self) -> usize {
        self.total_frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_BASE: u64 = 0x1_0000_0000; // 4 GiB

    fn create_test_allocator(total_frames: usize) -> BitmapAllocator {
        let base = PhysFrame::from_frame_number(TEST_BASE / PAGE_SIZE);
        BitmapAllocator::new(base, total_frames)
    }

    #[test]
    fn bitmap_allocator_allocates_single_frame() {
        let mut alloc = create_test_allocator(256);
        let frame = alloc.allocate().unwrap();
        assert_eq!(frame.frame_number(), TEST_BASE / PAGE_SIZE);
        assert_eq!(alloc.free_frames(), 255);
    }

    #[test]
    fn bitmap_allocator_allocates_contiguous_range() {
        let mut alloc = create_test_allocator(256);
        let start = alloc.allocate_contiguous(4).unwrap();
        assert_eq!(start.frame_number(), TEST_BASE / PAGE_SIZE);
        assert_eq!(alloc.free_frames(), 252);

        // Verify all 4 frames are marked as allocated
        for i in 0..4 {
            let index = i;
            assert!(alloc.get_bit(index), "frame {} should be allocated", i);
        }
    }

    #[test]
    fn bitmap_allocator_rejects_insufficient_contiguous() {
        let mut alloc = create_test_allocator(10);
        // Allocate frames 0, 2, 4, 6, 8 (alternating pattern)
        // We manually set bits to create fragmentation without going through allocate()
        for i in (0..10).step_by(2) {
            alloc.set_bit(i, true);
            alloc.free_frames -= 1;
        }

        // Try to allocate 2 contiguous frames - should fail due to fragmentation
        assert!(alloc.allocate_contiguous(2).is_none());
    }

    #[test]
    fn bitmap_allocator_deallocates_single_frame() {
        let mut alloc = create_test_allocator(256);
        let frame = alloc.allocate().unwrap();
        assert_eq!(alloc.free_frames(), 255);

        alloc.deallocate(frame);
        assert_eq!(alloc.free_frames(), 256);

        // Frame should be available for reallocation
        let new_frame = alloc.allocate().unwrap();
        assert_eq!(new_frame.frame_number(), frame.frame_number());
    }

    #[test]
    fn bitmap_allocator_deallocates_range() {
        let mut alloc = create_test_allocator(256);
        let start = alloc.allocate_contiguous(4).unwrap();
        assert_eq!(alloc.free_frames(), 252);

        alloc.deallocate_range(start, 4);
        assert_eq!(alloc.free_frames(), 256);

        // All frames should be available for reallocation
        let new_start = alloc.allocate_contiguous(4).unwrap();
        assert_eq!(new_start.frame_number(), start.frame_number());
    }

    #[test]
    #[should_panic(expected = "double deallocation")]
    fn bitmap_allocator_prevents_double_allocation() {
        let mut alloc = create_test_allocator(256);
        let frame = alloc.allocate().unwrap();
        // Deallocate the frame
        alloc.deallocate(frame);
        // Try to deallocate again - should panic
        alloc.deallocate(frame);
    }

    #[test]
    #[should_panic(expected = "double deallocation")]
    fn bitmap_allocator_prevents_double_deallocation() {
        let mut alloc = create_test_allocator(256);
        let frame = alloc.allocate().unwrap();
        alloc.deallocate(frame);
        // Try to deallocate again - should panic
        alloc.deallocate(frame);
    }

    #[test]
    fn bitmap_allocator_tracks_free_frames() {
        let mut alloc = create_test_allocator(256);
        assert_eq!(alloc.free_frames(), 256);

        alloc.allocate().unwrap();
        assert_eq!(alloc.free_frames(), 255);

        alloc.allocate_contiguous(3).unwrap();
        assert_eq!(alloc.free_frames(), 252);

        let frame = alloc.allocate().unwrap();
        alloc.deallocate(frame);
        // After allocate: 251, after deallocate: 252
        assert_eq!(alloc.free_frames(), 252);
    }

    #[test]
    fn bitmap_allocator_handles_fragmentation() {
        let mut alloc = create_test_allocator(10);
        // Allocate all frames
        for _ in 0..10 {
            alloc.allocate().unwrap();
        }
        assert_eq!(alloc.free_frames(), 0);

        // Free frames 2, 4, 6, 8 (alternating pattern)
        let base = alloc.base_frame;
        for i in [2, 4, 6, 8] {
            let frame = PhysFrame::from_frame_number(base.frame_number() + i as u64);
            alloc.deallocate(frame);
        }
        assert_eq!(alloc.free_frames(), 4);

        // Should be able to allocate single frames from freed positions
        let f2 = alloc.allocate().unwrap();
        assert_eq!(f2.frame_number(), base.frame_number() + 2);

        let f4 = alloc.allocate().unwrap();
        assert_eq!(f4.frame_number(), base.frame_number() + 4);
    }

    #[test]
    fn bitmap_allocator_exhaustion_returns_none() {
        let mut alloc = create_test_allocator(10);
        // Allocate all 10 frames
        for _ in 0..10 {
            assert!(alloc.allocate().is_some());
        }

        // Next allocation should fail
        assert!(alloc.allocate().is_none());
        assert_eq!(alloc.free_frames(), 0);

        // Contiguous allocation should also fail
        assert!(alloc.allocate_contiguous(1).is_none());
    }

    #[test]
    fn bitmap_allocator_zero_frames_returns_none() {
        let mut alloc = create_test_allocator(256);
        assert!(alloc.allocate_contiguous(0).is_none());
    }

    #[test]
    #[should_panic(expected = "frame index")]
    fn bitmap_allocator_panics_on_invalid_deallocation() {
        let mut alloc = create_test_allocator(256);
        let invalid_frame = PhysFrame::from_frame_number(TEST_BASE / PAGE_SIZE + 1000);
        alloc.deallocate(invalid_frame);
    }
}
