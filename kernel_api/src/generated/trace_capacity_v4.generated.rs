// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/trace_capacity_v4.toml

// namespace = trace.capacity, version = 4

pub const TRACE_CAPACITY_V4_PROTOCOL_ID: u32 = 534;

pub const MSG_TRACE_CAPACITY_V4_CREATE_FIXED_TRACE_BUFFER: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CreateFixedTraceBuffer {
    pub request_id: u64,
    pub domain_id: u64,
    pub max_delivery_bytes: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(CreateFixedTraceBuffer {
    request_id: u64,
    domain_id: u64,
    max_delivery_bytes: u32,
    reserved: u32,
});

pub const MSG_TRACE_CAPACITY_V4_CREATE_FIXED_TRACE_BUFFER_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CreateFixedTraceBufferReply {
    pub request_id: u64,
    pub status: u32,
    pub retention_events: u32,
    pub trace_cap: u64,
    pub retention_bytes: u32,
    pub max_delivery_bytes: u32,
}

crate::impl_wire_payload!(CreateFixedTraceBufferReply {
    request_id: u64,
    status: u32,
    retention_events: u32,
    trace_cap: u64,
    retention_bytes: u32,
    max_delivery_bytes: u32,
});

pub const MSG_TRACE_CAPACITY_V4_GET_FIXED_TRACE_INFO: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetFixedTraceInfo {
    pub request_id: u64,
    pub trace_cap: u64,
}

crate::impl_wire_payload!(GetFixedTraceInfo {
    request_id: u64,
    trace_cap: u64,
});

pub const MSG_TRACE_CAPACITY_V4_GET_FIXED_TRACE_INFO_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetFixedTraceInfoReply {
    pub request_id: u64,
    pub status: u32,
    pub retention_events: u32,
    pub domain_id: u64,
    pub retention_bytes: u32,
    pub max_delivery_bytes: u32,
    pub read_offset: u64,
    pub write_offset: u64,
}

crate::impl_wire_payload!(GetFixedTraceInfoReply {
    request_id: u64,
    status: u32,
    retention_events: u32,
    domain_id: u64,
    retention_bytes: u32,
    max_delivery_bytes: u32,
    read_offset: u64,
    write_offset: u64,
});

#[cfg(test)]
#[test]
fn wire_codec_trace_capacity_v4_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<CreateFixedTraceBuffer>()];
        let value = CreateFixedTraceBuffer::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CreateFixedTraceBuffer>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CreateFixedTraceBuffer>()];
        let offset = core::mem::offset_of!(CreateFixedTraceBuffer, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateFixedTraceBuffer, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateFixedTraceBuffer, max_delivery_bytes);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateFixedTraceBuffer, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CreateFixedTraceBuffer initialized bytes");
        let again = CreateFixedTraceBuffer::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.max_delivery_bytes, again.max_delivery_bytes);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CreateFixedTraceBufferReply>()];
        let value = CreateFixedTraceBufferReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CreateFixedTraceBufferReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CreateFixedTraceBufferReply>()];
        let offset = core::mem::offset_of!(CreateFixedTraceBufferReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateFixedTraceBufferReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateFixedTraceBufferReply, retention_events);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateFixedTraceBufferReply, trace_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateFixedTraceBufferReply, retention_bytes);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateFixedTraceBufferReply, max_delivery_bytes);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(
            output, expected,
            "CreateFixedTraceBufferReply initialized bytes"
        );
        let again = CreateFixedTraceBufferReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.retention_events, again.retention_events);
        assert_eq!(value.trace_cap, again.trace_cap);
        assert_eq!(value.retention_bytes, again.retention_bytes);
        assert_eq!(value.max_delivery_bytes, again.max_delivery_bytes);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GetFixedTraceInfo>()];
        let value = GetFixedTraceInfo::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetFixedTraceInfo>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetFixedTraceInfo>()];
        let offset = core::mem::offset_of!(GetFixedTraceInfo, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetFixedTraceInfo, trace_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetFixedTraceInfo initialized bytes");
        let again = GetFixedTraceInfo::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.trace_cap, again.trace_cap);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GetFixedTraceInfoReply>()];
        let value = GetFixedTraceInfoReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetFixedTraceInfoReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetFixedTraceInfoReply>()];
        let offset = core::mem::offset_of!(GetFixedTraceInfoReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetFixedTraceInfoReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetFixedTraceInfoReply, retention_events);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetFixedTraceInfoReply, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetFixedTraceInfoReply, retention_bytes);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetFixedTraceInfoReply, max_delivery_bytes);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetFixedTraceInfoReply, read_offset);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetFixedTraceInfoReply, write_offset);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetFixedTraceInfoReply initialized bytes");
        let again = GetFixedTraceInfoReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.retention_events, again.retention_events);
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.retention_bytes, again.retention_bytes);
        assert_eq!(value.max_delivery_bytes, again.max_delivery_bytes);
        assert_eq!(value.read_offset, again.read_offset);
        assert_eq!(value.write_offset, again.write_offset);
    }
}
