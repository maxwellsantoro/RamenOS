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

crate::impl_wire_payload!(TraceRead {
    cap_handle: u64,
    offset: u64,
    max_len: u32,
    reserved: u32,
});

pub const MSG_TRACE_SERVICE_V2_TRACE_READ_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TraceReadReply {
    pub cap_handle: u64,
    pub data_len: u32,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(TraceReadReply {
    cap_handle: u64,
    data_len: u32,
    status: u32,
    reserved: u32,
});

pub const MSG_TRACE_SERVICE_V2_TRACE_WRITE: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TraceWrite {
    pub cap_handle: u64,
    pub data_len: u32,
    pub reserved: u32,
    pub reserved2: u32,
}

crate::impl_wire_payload!(TraceWrite {
    cap_handle: u64,
    data_len: u32,
    reserved: u32,
    reserved2: u32,
});

pub const MSG_TRACE_SERVICE_V2_TRACE_WRITE_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TraceWriteReply {
    pub cap_handle: u64,
    pub status: u32,
    pub reserved: u32,
    pub reserved2: u32,
}

crate::impl_wire_payload!(TraceWriteReply {
    cap_handle: u64,
    status: u32,
    reserved: u32,
    reserved2: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_trace_service_v2_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<TraceRead>()];
        let value = TraceRead::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<TraceRead>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<TraceRead>()];
        let offset = core::mem::offset_of!(TraceRead, cap_handle);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(TraceRead, offset);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(TraceRead, max_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(TraceRead, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "TraceRead initialized bytes");
        let again = TraceRead::decode(&output).unwrap();
        assert_eq!(value.cap_handle, again.cap_handle);
        assert_eq!(value.offset, again.offset);
        assert_eq!(value.max_len, again.max_len);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<TraceReadReply>()];
        let value = TraceReadReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<TraceReadReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<TraceReadReply>()];
        let offset = core::mem::offset_of!(TraceReadReply, cap_handle);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(TraceReadReply, data_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(TraceReadReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(TraceReadReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "TraceReadReply initialized bytes");
        let again = TraceReadReply::decode(&output).unwrap();
        assert_eq!(value.cap_handle, again.cap_handle);
        assert_eq!(value.data_len, again.data_len);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<TraceWrite>()];
        let value = TraceWrite::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<TraceWrite>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<TraceWrite>()];
        let offset = core::mem::offset_of!(TraceWrite, cap_handle);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(TraceWrite, data_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(TraceWrite, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(TraceWrite, reserved2);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "TraceWrite initialized bytes");
        let again = TraceWrite::decode(&output).unwrap();
        assert_eq!(value.cap_handle, again.cap_handle);
        assert_eq!(value.data_len, again.data_len);
        assert_eq!(value.reserved, again.reserved);
        assert_eq!(value.reserved2, again.reserved2);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<TraceWriteReply>()];
        let value = TraceWriteReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<TraceWriteReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<TraceWriteReply>()];
        let offset = core::mem::offset_of!(TraceWriteReply, cap_handle);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(TraceWriteReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(TraceWriteReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(TraceWriteReply, reserved2);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "TraceWriteReply initialized bytes");
        let again = TraceWriteReply::decode(&output).unwrap();
        assert_eq!(value.cap_handle, again.cap_handle);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
        assert_eq!(value.reserved2, again.reserved2);
    }
}
