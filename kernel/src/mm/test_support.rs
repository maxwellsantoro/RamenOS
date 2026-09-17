//! Real, page-aligned backing for host tests of physical-memory consumers.
use super::{AddressSpaceTable, BitmapAllocator, PhysAddr, PhysFrame};
use std::sync::{Mutex, MutexGuard, OnceLock};

static TEST_MEMORY: Mutex<()> = Mutex::new(());

pub fn setup() -> MutexGuard<'static, ()> {
    let guard = TEST_MEMORY.lock().unwrap_or_else(|err| err.into_inner());
    static BASE: OnceLock<usize> = OnceLock::new();
    let base = *BASE.get_or_init(|| {
        let layout = std::alloc::Layout::from_size_align(4096 * 4096, 4096).unwrap();
        // Retained for the lifetime of the test process; never used by production.
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        assert!(!ptr.is_null());
        ptr as usize
    });
    *super::FRAME_ALLOCATOR.lock() = Some(BitmapAllocator::new(
        PhysFrame::from_frame_number(base as u64 / 4096),
        4096,
    ));
    let mut spaces = AddressSpaceTable::new();
    for domain in 0..4 {
        spaces.set_root(domain, unsafe { PhysAddr::new(0x5000 + domain * 4096) });
    }
    *super::ADDRESS_SPACE_TABLE.lock() = Some(spaces);
    guard
}
