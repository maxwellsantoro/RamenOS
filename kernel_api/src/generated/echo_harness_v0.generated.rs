// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/echo_harness_v0.toml

// namespace = harness.echo, version = 0

pub const ECHO_HARNESS_V0_PROTOCOL_ID: u32 = 528;

pub const MSG_ECHO_HARNESS_V0_ECHO_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct EchoReply {
    pub request_id: u64,
    pub payload_len: u32,
    pub status: u32,
}

pub const MSG_ECHO_HARNESS_V0_ECHO_REQUEST: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct EchoRequest {
    pub request_id: u64,
    pub payload_len: u32,
    pub reserved: u32,
}
