#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "x86_64")]
mod x86_64;

mod mmu;

#[cfg(target_arch = "aarch64")]
pub use aarch64::{
    KERNEL_END, KERNEL_START, PHYS_MEMORY_END, PHYS_MEMORY_START, PHYS_MMIO_REGION_START,
    get_current_page_table_root, halt, serial,
};
#[cfg(target_arch = "x86_64")]
pub use x86_64::{
    KERNEL_END, KERNEL_START, PHYS_MEMORY_END, PHYS_MEMORY_START, PHYS_MMIO_REGION_START,
    X86_64Mmu, get_current_page_table_root, halt, serial,
};

// Re-export architecture-specific MMU implementations for testing
#[cfg(target_arch = "aarch64")]
pub use aarch64::mmu::AArch64Mmu;

pub use mmu::{CACHE_MODE_UNCACHED, CACHE_MODE_WRITE_BACK, CACHE_MODE_WRITE_COMBINE};
pub use mmu::{Mmu, MmuError, PAGE_SIZE, VirtAddr};
pub use mmu::{RIGHTS_EXECUTE, RIGHTS_READ, RIGHTS_WRITE};
