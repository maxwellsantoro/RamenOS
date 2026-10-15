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

crate::impl_wire_payload!(CommitTraceRead {
    request_id: u64,
    trace_cap: u64,
    lease_id: u64,
    version: u32,
    reserved: u32,
});

pub const MSG_TRACE_DELIVERY_V3_COMMIT_TRACE_READ_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CommitTraceReadReply {
    pub request_id: u64,
    pub next_offset: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(CommitTraceReadReply {
    request_id: u64,
    next_offset: u64,
    status: u32,
    reserved: u32,
});

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

crate::impl_wire_payload!(ReadTraceInto {
    request_id: u64,
    trace_cap: u64,
    destination_shm_cap: u64,
    destination_offset: u64,
    trace_offset: u64,
    length: u32,
    version: u32,
    reserved: u64,
});

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

crate::impl_wire_payload!(ReadTraceIntoReply {
    request_id: u64,
    lease_id: u64,
    next_offset: u64,
    status: u32,
    data_len: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_trace_delivery_v3_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<CommitTraceRead>()];
        let value = CommitTraceRead::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CommitTraceRead>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CommitTraceRead>()];
        let offset = core::mem::offset_of!(CommitTraceRead, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitTraceRead, trace_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitTraceRead, lease_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitTraceRead, version);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitTraceRead, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CommitTraceRead initialized bytes");
        let again = CommitTraceRead::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.trace_cap, again.trace_cap);
        assert_eq!(value.lease_id, again.lease_id);
        assert_eq!(value.version, again.version);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CommitTraceReadReply>()];
        let value = CommitTraceReadReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CommitTraceReadReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CommitTraceReadReply>()];
        let offset = core::mem::offset_of!(CommitTraceReadReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitTraceReadReply, next_offset);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitTraceReadReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitTraceReadReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CommitTraceReadReply initialized bytes");
        let again = CommitTraceReadReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.next_offset, again.next_offset);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ReadTraceInto>()];
        let value = ReadTraceInto::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReadTraceInto>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReadTraceInto>()];
        let offset = core::mem::offset_of!(ReadTraceInto, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadTraceInto, trace_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadTraceInto, destination_shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadTraceInto, destination_offset);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadTraceInto, trace_offset);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadTraceInto, length);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadTraceInto, version);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadTraceInto, reserved);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReadTraceInto initialized bytes");
        let again = ReadTraceInto::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.trace_cap, again.trace_cap);
        assert_eq!(value.destination_shm_cap, again.destination_shm_cap);
        assert_eq!(value.destination_offset, again.destination_offset);
        assert_eq!(value.trace_offset, again.trace_offset);
        assert_eq!(value.length, again.length);
        assert_eq!(value.version, again.version);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ReadTraceIntoReply>()];
        let value = ReadTraceIntoReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReadTraceIntoReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReadTraceIntoReply>()];
        let offset = core::mem::offset_of!(ReadTraceIntoReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadTraceIntoReply, lease_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadTraceIntoReply, next_offset);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadTraceIntoReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadTraceIntoReply, data_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReadTraceIntoReply initialized bytes");
        let again = ReadTraceIntoReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.lease_id, again.lease_id);
        assert_eq!(value.next_offset, again.next_offset);
        assert_eq!(value.status, again.status);
        assert_eq!(value.data_len, again.data_len);
    }
}
