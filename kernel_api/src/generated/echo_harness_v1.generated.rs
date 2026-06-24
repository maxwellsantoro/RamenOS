// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/echo_harness_v1.toml

// namespace = harness.echo, version = 1

pub const ECHO_HARNESS_V1_PROTOCOL_ID: u32 = 529;

pub const MSG_ECHO_HARNESS_V1_ECHO_REPLY: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct EchoReply {
    pub cap_handle: u64,
    pub request_id: u64,
    pub status: u32,
    pub payload_len: u32,
}

pub const MSG_ECHO_HARNESS_V1_ECHO_REPLY_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct EchoReplyReply {
    pub cap_handle: u64,
    pub request_id: u64,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_ECHO_HARNESS_V1_ECHO_REQUEST: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct EchoRequest {
    pub cap_handle: u64,
    pub request_id: u64,
    pub payload_len: u32,
    pub reserved: u32,
}

pub const MSG_ECHO_HARNESS_V1_ECHO_REQUEST_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct EchoRequestReply {
    pub cap_handle: u64,
    pub request_id: u64,
    pub payload_len: u32,
    pub status: u32,
}
