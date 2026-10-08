// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/services/desktop_focus_v1.toml

// namespace = services.desktop_focus, version = 1

pub const DESKTOP_FOCUS_V1_PROTOCOL_ID: u32 = 832;

pub const MSG_DESKTOP_FOCUS_V1_ASSIGN: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Assign {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
    pub surface_id: u64,
    pub surface_generation: u64,
}

pub const MSG_DESKTOP_FOCUS_V1_ASSIGN_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AssignReply {
    pub request_id: u64,
    pub focus_epoch: u64,
    pub service_epoch: u64,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_DESKTOP_FOCUS_V1_POLL_KEYS: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PollKeys {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
    pub focus_epoch: u64,
}

pub const MSG_DESKTOP_FOCUS_V1_POLL_KEYS_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PollKeysReply {
    pub request_id: u64,
    pub sequence: u64,
    pub focus_epoch: u64,
    pub usage: u32,
    pub phase: u32,
    pub modifiers: u32,
    pub status: u32,
}
