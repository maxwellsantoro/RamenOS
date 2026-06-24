//! Physical memory layout constants for the kernel.
//!
//! These constants define the memory map used by the kernel for
//! page tables, domain regions, and test allocations.

/// Kernel memory layout constants
pub mod layout {
    /// Start of kernel page tables region
    pub const KERNEL_PAGE_TABLES_START: u64 = 0x1000;

    /// Dummy page table address for tests
    pub const DUMMY_PAGE_TABLE: u64 = 0x5000;

    /// Start of per-domain page tables region
    pub const DOMAIN_PAGE_TABLES_START: u64 = 0x10000;

    /// Size of each domain's page table region
    pub const DOMAIN_PAGE_TABLE_SIZE: u64 = 0x1000;

    /// Maximum number of domains supported
    pub const MAX_DOMAINS: u64 = 16;

    /// Test memory region base (4 GiB)
    pub const TEST_REGION_BASE: u64 = 0x1_0000_0000;

    /// Test region size (256 MiB)
    pub const TEST_REGION_SIZE: u64 = 0x1000_0000;
}
