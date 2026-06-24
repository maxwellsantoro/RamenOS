// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/services/execution_fabric_v1.toml

// namespace = services.execution_fabric, version = 1

pub const EXECUTION_FABRIC_V1_PROTOCOL_ID: u32 = 11;

pub const MSG_EXECUTION_FABRIC_V1_ATTACH_EXECUTION: u32 = 9;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AttachExecution {
    pub request_id: u64,
    pub execution_id: u64,
    pub capability_token: u64,
}

pub const MSG_EXECUTION_FABRIC_V1_ATTACH_EXECUTION_REPLY: u32 = 10;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AttachExecutionReply {
    pub request_id: u64,
    pub status: u32,
    pub stream_id: u64,
}

pub const MSG_EXECUTION_FABRIC_V1_CANCEL_EXECUTION: u32 = 11;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CancelExecution {
    pub request_id: u64,
    pub execution_id: u64,
    pub capability_token: u64,
}

pub const MSG_EXECUTION_FABRIC_V1_CANCEL_EXECUTION_REPLY: u32 = 12;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CancelExecutionReply {
    pub request_id: u64,
    pub status: u32,
}

pub const MSG_EXECUTION_FABRIC_V1_REGISTER_NODE: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RegisterNode {
    pub request_id: u64,
    pub node_manifest_hash: [u8; 32],
    pub capability_token: u64,
}

pub const MSG_EXECUTION_FABRIC_V1_REGISTER_NODE_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RegisterNodeReply {
    pub request_id: u64,
    pub status: u32,
    pub node_id: u64,
}

pub const MSG_EXECUTION_FABRIC_V1_REPORT_NODE_LOAD: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReportNodeLoad {
    pub request_id: u64,
    pub node_id: u64,
    pub load_snapshot_hash: [u8; 32],
    pub capability_token: u64,
}

pub const MSG_EXECUTION_FABRIC_V1_REPORT_NODE_LOAD_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReportNodeLoadReply {
    pub request_id: u64,
    pub status: u32,
}

pub const MSG_EXECUTION_FABRIC_V1_REQUEST_LEASE: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RequestLease {
    pub request_id: u64,
    pub resource_request_hash: [u8; 32],
    pub capability_token: u64,
}

pub const MSG_EXECUTION_FABRIC_V1_REQUEST_LEASE_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RequestLeaseReply {
    pub request_id: u64,
    pub status: u32,
    pub lease_id: u64,
    pub lease_hash: [u8; 32],
}

pub const MSG_EXECUTION_FABRIC_V1_SUBMIT_EXECUTION: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SubmitExecution {
    pub request_id: u64,
    pub execution_request_hash: [u8; 32],
    pub lease_id: u64,
    pub capability_token: u64,
}

pub const MSG_EXECUTION_FABRIC_V1_SUBMIT_EXECUTION_REPLY: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SubmitExecutionReply {
    pub request_id: u64,
    pub status: u32,
    pub execution_id: u64,
    pub execution_trace_hash: [u8; 32],
}
