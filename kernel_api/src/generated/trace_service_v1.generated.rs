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

crate::impl_wire_payload!(CreateTraceBuffer {
    request_id: u64,
    domain_id: u64,
    size: u32,
});

pub const MSG_TRACE_SERVICE_V1_CREATE_TRACE_BUFFER_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CreateTraceBufferReply {
    pub request_id: u64,
    pub status: u32,
    pub trace_cap: u64,
}

crate::impl_wire_payload!(CreateTraceBufferReply {
    request_id: u64,
    status: u32,
    trace_cap: u64,
});

pub const MSG_TRACE_SERVICE_V1_DESTROY_TRACE_BUFFER: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct DestroyTraceBuffer {
    pub request_id: u64,
    pub trace_cap: u64,
}

crate::impl_wire_payload!(DestroyTraceBuffer {
    request_id: u64,
    trace_cap: u64,
});

pub const MSG_TRACE_SERVICE_V1_DESTROY_TRACE_BUFFER_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct DestroyTraceBufferReply {
    pub request_id: u64,
    pub status: u32,
}

crate::impl_wire_payload!(DestroyTraceBufferReply {
    request_id: u64,
    status: u32,
});

pub const MSG_TRACE_SERVICE_V1_GET_TRACE_INFO: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetTraceInfo {
    pub request_id: u64,
    pub trace_cap: u64,
}

crate::impl_wire_payload!(GetTraceInfo {
    request_id: u64,
    trace_cap: u64,
});

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

crate::impl_wire_payload!(GetTraceInfoReply {
    request_id: u64,
    status: u32,
    domain_id: u64,
    size: u32,
    read_offset: u64,
    write_offset: u64,
});

pub const MSG_TRACE_SERVICE_V1_READ_TRACE: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadTrace {
    pub request_id: u64,
    pub trace_cap: u64,
    pub offset: u64,
    pub length: u32,
}

crate::impl_wire_payload!(ReadTrace {
    request_id: u64,
    trace_cap: u64,
    offset: u64,
    length: u32,
});

pub const MSG_TRACE_SERVICE_V1_READ_TRACE_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadTraceReply {
    pub request_id: u64,
    pub status: u32,
    pub data_len: u32,
}

crate::impl_wire_payload!(ReadTraceReply {
    request_id: u64,
    status: u32,
    data_len: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_trace_service_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<CreateTraceBuffer>()];
        let value = CreateTraceBuffer::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CreateTraceBuffer>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CreateTraceBuffer>()];
        let offset = core::mem::offset_of!(CreateTraceBuffer, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateTraceBuffer, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateTraceBuffer, size);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CreateTraceBuffer initialized bytes");
        let again = CreateTraceBuffer::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.size, again.size);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CreateTraceBufferReply>()];
        let value = CreateTraceBufferReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CreateTraceBufferReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CreateTraceBufferReply>()];
        let offset = core::mem::offset_of!(CreateTraceBufferReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateTraceBufferReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateTraceBufferReply, trace_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CreateTraceBufferReply initialized bytes");
        let again = CreateTraceBufferReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.trace_cap, again.trace_cap);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<DestroyTraceBuffer>()];
        let value = DestroyTraceBuffer::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<DestroyTraceBuffer>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<DestroyTraceBuffer>()];
        let offset = core::mem::offset_of!(DestroyTraceBuffer, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(DestroyTraceBuffer, trace_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "DestroyTraceBuffer initialized bytes");
        let again = DestroyTraceBuffer::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.trace_cap, again.trace_cap);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<DestroyTraceBufferReply>()];
        let value = DestroyTraceBufferReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<DestroyTraceBufferReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<DestroyTraceBufferReply>()];
        let offset = core::mem::offset_of!(DestroyTraceBufferReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(DestroyTraceBufferReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(
            output, expected,
            "DestroyTraceBufferReply initialized bytes"
        );
        let again = DestroyTraceBufferReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GetTraceInfo>()];
        let value = GetTraceInfo::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetTraceInfo>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetTraceInfo>()];
        let offset = core::mem::offset_of!(GetTraceInfo, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetTraceInfo, trace_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetTraceInfo initialized bytes");
        let again = GetTraceInfo::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.trace_cap, again.trace_cap);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GetTraceInfoReply>()];
        let value = GetTraceInfoReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetTraceInfoReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetTraceInfoReply>()];
        let offset = core::mem::offset_of!(GetTraceInfoReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetTraceInfoReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetTraceInfoReply, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetTraceInfoReply, size);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetTraceInfoReply, read_offset);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetTraceInfoReply, write_offset);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetTraceInfoReply initialized bytes");
        let again = GetTraceInfoReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.size, again.size);
        assert_eq!(value.read_offset, again.read_offset);
        assert_eq!(value.write_offset, again.write_offset);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ReadTrace>()];
        let value = ReadTrace::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReadTrace>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReadTrace>()];
        let offset = core::mem::offset_of!(ReadTrace, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadTrace, trace_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadTrace, offset);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadTrace, length);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReadTrace initialized bytes");
        let again = ReadTrace::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.trace_cap, again.trace_cap);
        assert_eq!(value.offset, again.offset);
        assert_eq!(value.length, again.length);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ReadTraceReply>()];
        let value = ReadTraceReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReadTraceReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReadTraceReply>()];
        let offset = core::mem::offset_of!(ReadTraceReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadTraceReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadTraceReply, data_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReadTraceReply initialized bytes");
        let again = ReadTraceReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.data_len, again.data_len);
    }
}
