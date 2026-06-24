//! Semantic State Service
//!
//! Aggregates OS state into structured snapshots for AI agents.
//! Uses shared memory for snapshot delivery to bypass IPC payload limits.

#![cfg_attr(target_arch = "wasm32", no_std)]

mod snapshot;
mod subscribe;

#[cfg(not(target_arch = "wasm32"))]
mod reactor;

pub use snapshot::{
    FORMAT_JSON, FORMAT_MARKDOWN, STATUS_INVALID_FORMAT, STATUS_OK, STATUS_SHMEM_UNAVAILABLE,
    build_default_snapshot, build_platform_snapshot, domain_inventory_from_manager,
    serialize_snapshot,
};
pub use subscribe::{
    EVENT_MASK_DOMAIN_STATE_CHANGED, EVENT_TYPE_DOMAIN_STATE_CHANGED, STATUS_INVALID_MASK,
    STATUS_OK as SUBSCRIBE_STATUS_OK,
};

#[cfg(not(target_arch = "wasm32"))]
pub use reactor::SemanticReactor;

#[cfg(target_arch = "wasm32")]
extern crate alloc;

#[cfg(target_arch = "wasm32")]
use alloc::vec::Vec;
#[cfg(target_arch = "wasm32")]
use ramen_sdk::generated::harness_shmem_control_v1::{CreateRegionClient, ShmemWriteClient};
#[cfg(target_arch = "wasm32")]
use ramen_sdk::generated::services_semantic_state_v1::GetSnapshotReplyClient;
#[cfg(target_arch = "wasm32")]
use ramen_sdk::{CapabilityHandle, Status};

// Capability handles injected by the runner
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub static mut RAMEN_CAP_SHMEM_CONTROL: CapabilityHandle = CapabilityHandle::INVALID;
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub static mut RAMEN_CAP_SEMANTIC_STATE: CapabilityHandle = CapabilityHandle::INVALID;

/// Deliver one snapshot over shmem and reply via the semantic-state harness.
#[cfg(target_arch = "wasm32")]
fn deliver_snapshot(request_id: u64, format: u32) -> u32 {
    let snapshot = build_default_snapshot();
    let data = match serialize_snapshot(&snapshot, format) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };

    let shmem_cap = unsafe { RAMEN_CAP_SHMEM_CONTROL };
    if !shmem_cap.is_valid() {
        return STATUS_SHMEM_UNAVAILABLE;
    }

    let shm_client = CreateRegionClient::from_cap(shmem_cap.raw());
    let mut reply_buf = [0u8; 64];

    let (status, reply_len) = shm_client.call(1, 0, data.len() as u64, 1, 4096, &mut reply_buf);

    if !status.is_ok() || reply_len < 40 {
        return STATUS_SHMEM_UNAVAILABLE;
    }

    let region_id = match reply_buf[16..24].try_into() {
        Ok(bytes) => u64::from_le_bytes(bytes),
        Err(_) => return STATUS_SHMEM_UNAVAILABLE,
    };
    let phys_addr = match reply_buf[24..32].try_into() {
        Ok(bytes) => u64::from_le_bytes(bytes),
        Err(_) => return STATUS_SHMEM_UNAVAILABLE,
    };

    let writer = ShmemWriteClient::from_cap(shmem_cap.raw());
    let (write_status, _written) = writer.call(
        phys_addr,
        0,
        data.as_ptr() as u64,
        data.len() as u32,
        &mut reply_buf,
    );
    if !write_status.is_ok() {
        return STATUS_SHMEM_UNAVAILABLE;
    }

    let semantic_cap = unsafe { RAMEN_CAP_SEMANTIC_STATE };
    if !semantic_cap.is_valid() {
        return STATUS_SHMEM_UNAVAILABLE;
    }

    let reply_client = GetSnapshotReplyClient::from_cap(semantic_cap.raw());
    let (reply_status, _reply_len) = reply_client.call(
        request_id,
        STATUS_OK,
        region_id,
        data.len() as u64,
        &mut reply_buf,
    );

    if reply_status.is_ok() {
        STATUS_OK
    } else {
        STATUS_SHMEM_UNAVAILABLE
    }
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn _start() {
    let _ = deliver_snapshot(1, FORMAT_MARKDOWN);
}

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

/// Host-target placeholder so workspace builds can type-check the crate graph.
#[cfg(not(target_arch = "wasm32"))]
pub const HOST_TARGET_STUB: bool = true;
