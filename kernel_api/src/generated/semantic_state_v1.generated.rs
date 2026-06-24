// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/services/semantic_state_v1.toml

// namespace = services.semantic_state, version = 1

pub const SEMANTIC_STATE_V1_PROTOCOL_ID: u32 = 10;

pub const MSG_SEMANTIC_STATE_V1_GET_SNAPSHOT: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetSnapshot {
    pub cap_handle: u64,
    pub request_id: u64,
    pub format: u32,
}

pub const MSG_SEMANTIC_STATE_V1_GET_SNAPSHOT_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetSnapshotReply {
    pub request_id: u64,
    pub status: u32,
    pub shm_cap: u64,
    pub shm_size: u64,
}

pub const MSG_SEMANTIC_STATE_V1_STATE_CHANGED_EVENT: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StateChangedEvent {
    pub subscription_id: u64,
    pub event_type: u32,
    pub shm_cap: u64,
}

pub const MSG_SEMANTIC_STATE_V1_SUBSCRIBE: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Subscribe {
    pub request_id: u64,
    pub event_mask: u32,
}

pub const MSG_SEMANTIC_STATE_V1_SUBSCRIBE_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SubscribeReply {
    pub request_id: u64,
    pub status: u32,
    pub subscription_id: u64,
}
