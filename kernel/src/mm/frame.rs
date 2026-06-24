//! Frame allocator trait.
//!
//! Defines the interface for physical memory frame allocation.
//! Implementations can use different strategies (bump, bitmap, stack).

use super::address::PhysFrame;

/// Trait for allocating and deallocating physical memory frames.
///
/// # Safety
///
/// Implementations must ensure:
/// - `allocate()` returns uniquely owned frames that don't overlap
/// - `deallocate()` only accepts frames previously returned by `allocate()`
/// - After deallocation, the frame may be reallocated by subsequent `allocate()` calls
/// - The allocator must handle concurrent access if used in multi-threaded context
pub unsafe trait FrameAllocator {
    /// Allocate a single physical frame.
    ///
    /// Returns `None` if no frames are available.
    fn allocate(&mut self) -> Option<PhysFrame>;

    /// Deallocate a previously allocated frame.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - `frame` was previously returned by `allocate()` from this allocator
    /// - `frame` has not already been deallocated
    /// - No references to the frame's memory exist (use-after-free prevention)
    /// - The frame is not currently mapped in any page table
    unsafe fn deallocate(&mut self, frame: PhysFrame);

    /// Return the number of available frames.
    ///
    /// This is useful for diagnostics and testing.
    fn available_frames(&self) -> usize;

    /// Return the total number of frames managed by this allocator.
    fn total_frames(&self) -> usize;
}

/// Null allocator that never allocates.
///
/// Useful for testing or as a placeholder before real memory is available.
pub struct NullAllocator;

unsafe impl FrameAllocator for NullAllocator {
    fn allocate(&mut self) -> Option<PhysFrame> {
        None
    }

    unsafe fn deallocate(&mut self, _frame: PhysFrame) {
        // Nothing to deallocate
    }

    fn available_frames(&self) -> usize {
        0
    }

    fn total_frames(&self) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::super::address::PhysFrame;
    use super::*;

    #[test]
    fn null_allocator_never_allocates() {
        let mut alloc = NullAllocator;
        assert!(alloc.allocate().is_none());
    }

    #[test]
    fn null_allocator_reports_zero_frames() {
        let alloc = NullAllocator;
        assert_eq!(alloc.available_frames(), 0);
        assert_eq!(alloc.total_frames(), 0);
    }

    #[test]
    fn null_allocator_deallocate_is_noop() {
        let mut alloc = NullAllocator;
        let frame = PhysFrame::from_frame_number(42);
        unsafe {
            alloc.deallocate(frame);
        }
        // Should not panic
    }
}
