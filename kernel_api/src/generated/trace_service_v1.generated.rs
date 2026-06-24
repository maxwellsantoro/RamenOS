// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/trace_service_v1.toml

// namespace = trace.service, version = 1

pub const TRACE_SERVICE_V1_PROTOCOL_ID: u32 = 9;

pub const MSG_TRACE_SERVICE_V1_CREATE_TRACE_BUFFER: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CreateTraceBuffer {
    pub request_id: u64,
    pub domain_id: u64,
    pub size: u32,
}

pub const MSG_TRACE_SERVICE_V1_CREATE_TRACE_BUFFER_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CreateTraceBufferReply {
    pub request_id: u64,
    pub status: u32,
    pub trace_cap: u64,
}

pub const MSG_TRACE_SERVICE_V1_DESTROY_TRACE_BUFFER: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct DestroyTraceBuffer {
    pub request_id: u64,
    pub trace_cap: u64,
}

pub const MSG_TRACE_SERVICE_V1_DESTROY_TRACE_BUFFER_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct DestroyTraceBufferReply {
    pub request_id: u64,
    pub status: u32,
}

pub const MSG_TRACE_SERVICE_V1_GET_TRACE_INFO: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetTraceInfo {
    pub request_id: u64,
    pub trace_cap: u64,
}

pub const MSG_TRACE_SERVICE_V1_GET_TRACE_INFO_REPLY: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetTraceInfoReply {
    pub request_id: u64,
    pub status: u32,
    pub domain_id: u64,
    pub size: u32,
    pub read_offset: u64,
    pub write_offset: u64,
}

pub const MSG_TRACE_SERVICE_V1_READ_TRACE: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadTrace {
    pub request_id: u64,
    pub trace_cap: u64,
    pub offset: u64,
    pub length: u32,
}

pub const MSG_TRACE_SERVICE_V1_READ_TRACE_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadTraceReply {
    pub request_id: u64,
    pub status: u32,
    pub data_len: u32,
}
