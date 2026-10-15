// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/trace_delivery_v3.toml

// namespace = trace.delivery, version = 3

pub const TRACE_DELIVERY_V3_PROTOCOL_ID: u32 = 532;

pub const MSG_TRACE_DELIVERY_V3_COMMIT_TRACE_READ: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CommitTraceRead {
    pub request_id: u64,
    pub trace_cap: u64,
    pub lease_id: u64,
    pub version: u32,
    pub reserved: u32,
}

pub const MSG_TRACE_DELIVERY_V3_COMMIT_TRACE_READ_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CommitTraceReadReply {
    pub request_id: u64,
    pub next_offset: u64,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_TRACE_DELIVERY_V3_READ_TRACE_INTO: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadTraceInto {
    pub request_id: u64,
    pub trace_cap: u64,
    pub destination_shm_cap: u64,
    pub destination_offset: u64,
    pub trace_offset: u64,
    pub length: u32,
    pub version: u32,
    pub reserved: u64,
}

pub const MSG_TRACE_DELIVERY_V3_READ_TRACE_INTO_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadTraceIntoReply {
    pub request_id: u64,
    pub lease_id: u64,
    pub next_offset: u64,
    pub status: u32,
    pub data_len: u32,
}
