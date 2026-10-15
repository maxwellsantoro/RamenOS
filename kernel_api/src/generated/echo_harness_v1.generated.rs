// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/echo_harness_v1.toml

// namespace = harness.echo, version = 1

pub const ECHO_HARNESS_V1_PROTOCOL_ID: u32 = 529;

pub const MSG_ECHO_HARNESS_V1_ECHO_REPLY: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct EchoReply {
    pub cap_handle: u64,
    pub request_id: u64,
    pub status: u32,
    pub payload_len: u32,
}

crate::impl_wire_payload!(EchoReply {
    cap_handle: u64,
    request_id: u64,
    status: u32,
    payload_len: u32,
});

pub const MSG_ECHO_HARNESS_V1_ECHO_REPLY_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct EchoReplyReply {
    pub cap_handle: u64,
    pub request_id: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(EchoReplyReply {
    cap_handle: u64,
    request_id: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_ECHO_HARNESS_V1_ECHO_REQUEST: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct EchoRequest {
    pub cap_handle: u64,
    pub request_id: u64,
    pub payload_len: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(EchoRequest {
    cap_handle: u64,
    request_id: u64,
    payload_len: u32,
    reserved: u32,
});

pub const MSG_ECHO_HARNESS_V1_ECHO_REQUEST_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct EchoRequestReply {
    pub cap_handle: u64,
    pub request_id: u64,
    pub payload_len: u32,
    pub status: u32,
}

crate::impl_wire_payload!(EchoRequestReply {
    cap_handle: u64,
    request_id: u64,
    payload_len: u32,
    status: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_echo_harness_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<EchoReply>()];
        let value = EchoReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<EchoReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<EchoReply>()];
        let offset = core::mem::offset_of!(EchoReply, cap_handle);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(EchoReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(EchoReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(EchoReply, payload_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "EchoReply initialized bytes");
        let again = EchoReply::decode(&output).unwrap();
        assert_eq!(value.cap_handle, again.cap_handle);
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.payload_len, again.payload_len);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<EchoReplyReply>()];
        let value = EchoReplyReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<EchoReplyReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<EchoReplyReply>()];
        let offset = core::mem::offset_of!(EchoReplyReply, cap_handle);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(EchoReplyReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(EchoReplyReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(EchoReplyReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "EchoReplyReply initialized bytes");
        let again = EchoReplyReply::decode(&output).unwrap();
        assert_eq!(value.cap_handle, again.cap_handle);
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<EchoRequest>()];
        let value = EchoRequest::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<EchoRequest>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<EchoRequest>()];
        let offset = core::mem::offset_of!(EchoRequest, cap_handle);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(EchoRequest, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(EchoRequest, payload_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(EchoRequest, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "EchoRequest initialized bytes");
        let again = EchoRequest::decode(&output).unwrap();
        assert_eq!(value.cap_handle, again.cap_handle);
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.payload_len, again.payload_len);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<EchoRequestReply>()];
        let value = EchoRequestReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<EchoRequestReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<EchoRequestReply>()];
        let offset = core::mem::offset_of!(EchoRequestReply, cap_handle);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(EchoRequestReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(EchoRequestReply, payload_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(EchoRequestReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "EchoRequestReply initialized bytes");
        let again = EchoRequestReply::decode(&output).unwrap();
        assert_eq!(value.cap_handle, again.cap_handle);
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.payload_len, again.payload_len);
        assert_eq!(value.status, again.status);
    }
}
