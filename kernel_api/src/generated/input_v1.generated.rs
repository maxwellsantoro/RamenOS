// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/input_v1.toml

// namespace = harness.input, version = 1

pub const INPUT_V1_PROTOCOL_ID: u32 = 802;

pub const MSG_INPUT_V1_ATTACH: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Attach {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub device_generation: u64,
}

pub const MSG_INPUT_V1_ATTACH_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AttachReply {
    pub request_id: u64,
    pub queue_generation: u64,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_INPUT_V1_PRODUCE_KEY: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ProduceKey {
    pub session_id: u64,
    pub session_generation: u64,
    pub queue_generation: u64,
    pub sequence: u64,
    pub usage: u32,
    pub phase: u32,
    pub modifiers: u32,
    pub reserved: u32,
}

pub const MSG_INPUT_V1_PRODUCE_KEY_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ProduceKeyReply {
    pub sequence: u64,
    pub status: u32,
    pub reserved: u32,
}
