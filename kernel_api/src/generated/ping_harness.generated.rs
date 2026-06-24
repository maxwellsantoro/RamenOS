// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/ping_harness.toml

// namespace = harness.ping, version = 1

pub const PING_HARNESS_PROTOCOL_ID: u32 = 1;

pub const MSG_PING_HARNESS_PING: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Ping {
    pub nonce: u64,
}

pub const MSG_PING_HARNESS_PONG: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Pong {
    pub nonce: u64,
}
