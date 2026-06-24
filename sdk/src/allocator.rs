//! Simple dynamic bump allocator for WASM modules.
//!
//! Provides a global allocator for no_std WASM modules that grows
//! linear memory on demand using `memory.grow`.
//! Note: This allocator never deallocates.

use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

/// A dynamic bump allocator using WASM memory growth.
pub struct BumpAllocator;

impl BumpAllocator {
    pub const fn new() -> Self {
        Self
    }
}

impl Default for BumpAllocator {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Sync for BumpAllocator {}

#[cfg(target_arch = "wasm32")]
unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        // WASM page size is 64KB
        const PAGE_SIZE: usize = 65536;

        // Note: For a true bump allocator that grows, we'd need to track
        // the current 'next' pointer globally. Since we are in a single-threaded
        // WASM instance, we can use a static mutable or an AtomicUsize.
        static mut NEXT: usize = 0;
        static mut CAPACITY: usize = 0;

        let start = (NEXT + align - 1) & !(align - 1);
        let end = start + size;

        if end > CAPACITY {
            // Need more pages
            let needed = end - CAPACITY;
            let pages = (needed + PAGE_SIZE - 1) / PAGE_SIZE;

            // memory_grow returns previous number of pages
            let prev_pages = core::arch::wasm32::memory_grow(0, pages);
            if prev_pages == usize::MAX {
                return ptr::null_mut();
            }

            CAPACITY = (prev_pages + pages) * PAGE_SIZE;
            if NEXT == 0 {
                NEXT = prev_pages * PAGE_SIZE;
            }
        }

        NEXT = end;
        start as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator never deallocates
    }
}

// Fallback for host tests
#[cfg(not(target_arch = "wasm32"))]
unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        ptr::null_mut()
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}
