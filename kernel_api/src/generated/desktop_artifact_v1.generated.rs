// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/portals/desktop_artifact_v1.toml

// namespace = portal.desktop_artifact, version = 1

pub const DESKTOP_ARTIFACT_V1_PROTOCOL_ID: u32 = 368;

pub const MSG_DESKTOP_ARTIFACT_V1_ALLOCATE_SAVE_ID: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AllocateSaveId {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub expected_revision: u64,
}

pub const MSG_DESKTOP_ARTIFACT_V1_ALLOCATE_SAVE_ID_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AllocateSaveIdReply {
    pub request_id: u64,
    pub operation_id: u64,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_DESKTOP_ARTIFACT_V1_COMMIT: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Commit {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub expected_revision: u64,
    pub operation_id: u64,
    pub source_shm: u64,
    pub byte_len: u32,
    pub reserved: u32,
}

pub const MSG_DESKTOP_ARTIFACT_V1_COMMIT_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CommitReply {
    pub request_id: u64,
    pub operation_id: u64,
    pub revision: u64,
    pub receipt_shm: u64,
    pub status: u32,
    pub receipt_len: u32,
}

pub const MSG_DESKTOP_ARTIFACT_V1_OBSERVE_ALLOCATION: u32 = 9;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ObserveAllocation {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub original_allocate_request_id: u64,
}

pub const MSG_DESKTOP_ARTIFACT_V1_OBSERVE_ALLOCATION_REPLY: u32 = 10;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ObserveAllocationReply {
    pub request_id: u64,
    pub original_allocate_request_id: u64,
    pub operation_id: u64,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_DESKTOP_ARTIFACT_V1_READ_SELECTED: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadSelected {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
}

pub const MSG_DESKTOP_ARTIFACT_V1_READ_SELECTED_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadSelectedReply {
    pub request_id: u64,
    pub revision: u64,
    pub data_shm: u64,
    pub object_generation: u64,
    pub byte_len: u32,
    pub status: u32,
}

pub const MSG_DESKTOP_ARTIFACT_V1_SAVE_STATUS: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SaveStatus {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub operation_id: u64,
}

pub const MSG_DESKTOP_ARTIFACT_V1_SAVE_STATUS_REPLY: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SaveStatusReply {
    pub request_id: u64,
    pub operation_id: u64,
    pub revision: u64,
    pub receipt_shm: u64,
    pub status: u32,
    pub receipt_len: u32,
}
