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

crate::impl_wire_payload!(AttachExecution {
    request_id: u64,
    execution_id: u64,
    capability_token: u64,
});

pub const MSG_EXECUTION_FABRIC_V1_ATTACH_EXECUTION_REPLY: u32 = 10;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AttachExecutionReply {
    pub request_id: u64,
    pub status: u32,
    pub stream_id: u64,
}

crate::impl_wire_payload!(AttachExecutionReply {
    request_id: u64,
    status: u32,
    stream_id: u64,
});

pub const MSG_EXECUTION_FABRIC_V1_CANCEL_EXECUTION: u32 = 11;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CancelExecution {
    pub request_id: u64,
    pub execution_id: u64,
    pub capability_token: u64,
}

crate::impl_wire_payload!(CancelExecution {
    request_id: u64,
    execution_id: u64,
    capability_token: u64,
});

pub const MSG_EXECUTION_FABRIC_V1_CANCEL_EXECUTION_REPLY: u32 = 12;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CancelExecutionReply {
    pub request_id: u64,
    pub status: u32,
}

crate::impl_wire_payload!(CancelExecutionReply {
    request_id: u64,
    status: u32,
});

pub const MSG_EXECUTION_FABRIC_V1_REGISTER_NODE: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RegisterNode {
    pub request_id: u64,
    pub node_manifest_hash: [u8; 32],
    pub capability_token: u64,
}

crate::impl_wire_payload!(RegisterNode {
    request_id: u64,
    node_manifest_hash: [u8; 32],
    capability_token: u64,
});

pub const MSG_EXECUTION_FABRIC_V1_REGISTER_NODE_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RegisterNodeReply {
    pub request_id: u64,
    pub status: u32,
    pub node_id: u64,
}

crate::impl_wire_payload!(RegisterNodeReply {
    request_id: u64,
    status: u32,
    node_id: u64,
});

pub const MSG_EXECUTION_FABRIC_V1_REPORT_NODE_LOAD: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReportNodeLoad {
    pub request_id: u64,
    pub node_id: u64,
    pub load_snapshot_hash: [u8; 32],
    pub capability_token: u64,
}

crate::impl_wire_payload!(ReportNodeLoad {
    request_id: u64,
    node_id: u64,
    load_snapshot_hash: [u8; 32],
    capability_token: u64,
});

pub const MSG_EXECUTION_FABRIC_V1_REPORT_NODE_LOAD_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReportNodeLoadReply {
    pub request_id: u64,
    pub status: u32,
}

crate::impl_wire_payload!(ReportNodeLoadReply {
    request_id: u64,
    status: u32,
});

pub const MSG_EXECUTION_FABRIC_V1_REQUEST_LEASE: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RequestLease {
    pub request_id: u64,
    pub resource_request_hash: [u8; 32],
    pub capability_token: u64,
}

crate::impl_wire_payload!(RequestLease {
    request_id: u64,
    resource_request_hash: [u8; 32],
    capability_token: u64,
});

pub const MSG_EXECUTION_FABRIC_V1_REQUEST_LEASE_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RequestLeaseReply {
    pub request_id: u64,
    pub status: u32,
    pub lease_id: u64,
    pub lease_hash: [u8; 32],
}

crate::impl_wire_payload!(RequestLeaseReply {
    request_id: u64,
    status: u32,
    lease_id: u64,
    lease_hash: [u8; 32],
});

pub const MSG_EXECUTION_FABRIC_V1_SUBMIT_EXECUTION: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SubmitExecution {
    pub request_id: u64,
    pub execution_request_hash: [u8; 32],
    pub lease_id: u64,
    pub capability_token: u64,
}

crate::impl_wire_payload!(SubmitExecution {
    request_id: u64,
    execution_request_hash: [u8; 32],
    lease_id: u64,
    capability_token: u64,
});

pub const MSG_EXECUTION_FABRIC_V1_SUBMIT_EXECUTION_REPLY: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SubmitExecutionReply {
    pub request_id: u64,
    pub status: u32,
    pub execution_id: u64,
    pub execution_trace_hash: [u8; 32],
}

crate::impl_wire_payload!(SubmitExecutionReply {
    request_id: u64,
    status: u32,
    execution_id: u64,
    execution_trace_hash: [u8; 32],
});

#[cfg(test)]
#[test]
fn wire_codec_execution_fabric_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<AttachExecution>()];
        let value = AttachExecution::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<AttachExecution>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<AttachExecution>()];
        let offset = core::mem::offset_of!(AttachExecution, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AttachExecution, execution_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AttachExecution, capability_token);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "AttachExecution initialized bytes");
        let again = AttachExecution::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.execution_id, again.execution_id);
        assert_eq!(value.capability_token, again.capability_token);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<AttachExecutionReply>()];
        let value = AttachExecutionReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<AttachExecutionReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<AttachExecutionReply>()];
        let offset = core::mem::offset_of!(AttachExecutionReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AttachExecutionReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AttachExecutionReply, stream_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "AttachExecutionReply initialized bytes");
        let again = AttachExecutionReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.stream_id, again.stream_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CancelExecution>()];
        let value = CancelExecution::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CancelExecution>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CancelExecution>()];
        let offset = core::mem::offset_of!(CancelExecution, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelExecution, execution_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelExecution, capability_token);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CancelExecution initialized bytes");
        let again = CancelExecution::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.execution_id, again.execution_id);
        assert_eq!(value.capability_token, again.capability_token);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CancelExecutionReply>()];
        let value = CancelExecutionReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CancelExecutionReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CancelExecutionReply>()];
        let offset = core::mem::offset_of!(CancelExecutionReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelExecutionReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CancelExecutionReply initialized bytes");
        let again = CancelExecutionReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<RegisterNode>()];
        let value = RegisterNode::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<RegisterNode>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<RegisterNode>()];
        let offset = core::mem::offset_of!(RegisterNode, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RegisterNode, node_manifest_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RegisterNode, capability_token);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "RegisterNode initialized bytes");
        let again = RegisterNode::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.node_manifest_hash, again.node_manifest_hash);
        assert_eq!(value.capability_token, again.capability_token);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<RegisterNodeReply>()];
        let value = RegisterNodeReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<RegisterNodeReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<RegisterNodeReply>()];
        let offset = core::mem::offset_of!(RegisterNodeReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RegisterNodeReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RegisterNodeReply, node_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "RegisterNodeReply initialized bytes");
        let again = RegisterNodeReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.node_id, again.node_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ReportNodeLoad>()];
        let value = ReportNodeLoad::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReportNodeLoad>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReportNodeLoad>()];
        let offset = core::mem::offset_of!(ReportNodeLoad, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReportNodeLoad, node_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReportNodeLoad, load_snapshot_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReportNodeLoad, capability_token);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReportNodeLoad initialized bytes");
        let again = ReportNodeLoad::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.node_id, again.node_id);
        assert_eq!(value.load_snapshot_hash, again.load_snapshot_hash);
        assert_eq!(value.capability_token, again.capability_token);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ReportNodeLoadReply>()];
        let value = ReportNodeLoadReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReportNodeLoadReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReportNodeLoadReply>()];
        let offset = core::mem::offset_of!(ReportNodeLoadReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReportNodeLoadReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReportNodeLoadReply initialized bytes");
        let again = ReportNodeLoadReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<RequestLease>()];
        let value = RequestLease::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<RequestLease>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<RequestLease>()];
        let offset = core::mem::offset_of!(RequestLease, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestLease, resource_request_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestLease, capability_token);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "RequestLease initialized bytes");
        let again = RequestLease::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.resource_request_hash, again.resource_request_hash);
        assert_eq!(value.capability_token, again.capability_token);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<RequestLeaseReply>()];
        let value = RequestLeaseReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<RequestLeaseReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<RequestLeaseReply>()];
        let offset = core::mem::offset_of!(RequestLeaseReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestLeaseReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestLeaseReply, lease_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestLeaseReply, lease_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "RequestLeaseReply initialized bytes");
        let again = RequestLeaseReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.lease_id, again.lease_id);
        assert_eq!(value.lease_hash, again.lease_hash);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<SubmitExecution>()];
        let value = SubmitExecution::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<SubmitExecution>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<SubmitExecution>()];
        let offset = core::mem::offset_of!(SubmitExecution, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubmitExecution, execution_request_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubmitExecution, lease_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubmitExecution, capability_token);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "SubmitExecution initialized bytes");
        let again = SubmitExecution::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.execution_request_hash, again.execution_request_hash);
        assert_eq!(value.lease_id, again.lease_id);
        assert_eq!(value.capability_token, again.capability_token);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<SubmitExecutionReply>()];
        let value = SubmitExecutionReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<SubmitExecutionReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<SubmitExecutionReply>()];
        let offset = core::mem::offset_of!(SubmitExecutionReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubmitExecutionReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubmitExecutionReply, execution_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubmitExecutionReply, execution_trace_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "SubmitExecutionReply initialized bytes");
        let again = SubmitExecutionReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.execution_id, again.execution_id);
        assert_eq!(value.execution_trace_hash, again.execution_trace_hash);
    }
}
