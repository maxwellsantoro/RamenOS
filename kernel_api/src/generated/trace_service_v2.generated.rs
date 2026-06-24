// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/trace_service_v2.toml

// namespace = harness.trace, version = 2

pub const TRACE_SERVICE_V2_PROTOCOL_ID: u32 = 530;

pub const MSG_TRACE_SERVICE_V2_TRACE_READ: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TraceRead {
    pub cap_handle: u64,
    pub offset: u64,
    pub max_len: u32,
    pub reserved: u32,
}

pub const MSG_TRACE_SERVICE_V2_TRACE_READ_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TraceReadReply {
    pub cap_handle: u64,
    pub data_len: u32,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_TRACE_SERVICE_V2_TRACE_WRITE: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TraceWrite {
    pub cap_handle: u64,
    pub data_len: u32,
    pub reserved: u32,
    pub reserved2: u32,
}

pub const MSG_TRACE_SERVICE_V2_TRACE_WRITE_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TraceWriteReply {
    pub cap_handle: u64,
    pub status: u32,
    pub reserved: u32,
    pub reserved2: u32,
}
