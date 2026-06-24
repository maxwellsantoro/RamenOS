#![no_std]
#![allow(static_mut_refs)]

// Enable std for test builds (needed for Vec in tests)
#[cfg(test)]
extern crate std;

// Day 0 kernel is a library crate only.
// Agent will add the real boot/entry/linker pieces for Slice S0.

pub mod arch;
pub mod boot;
pub mod cap_table;
pub mod domain_registry;
mod hil_provenance;
pub mod serial;
pub mod trace_cap;
pub mod trace_ring; // V-012 Phase 3: Trace capability-based access control
pub mod trace_service; // V-012 Phase 4: Kernel-side trace service

#[cfg(any(feature = "test_protocols", test))]
mod block_harness;
mod init;
mod ipc_v0;
pub mod mm;
#[cfg(any(feature = "test_protocols", test))]
mod net_harness;
pub mod shmem;

pub mod cap {
    pub use kernel_api::cap::*;
}

pub mod ipc {
    pub use kernel_api::ipc::*;
}

pub mod trace {
    pub use kernel_api::trace::*;
}

/// Example "kernel-internal" function used by early bring-up tests.
pub fn kernel_banner_tag() -> u32 {
    trace::TAG_BOOT
}
