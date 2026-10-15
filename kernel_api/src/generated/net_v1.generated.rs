// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/net_v1.toml

// namespace = harness.net, version = 1

pub const NET_V1_PROTOCOL_ID: u32 = 800;

pub const MSG_NET_V1_RECEIVE_PACKET: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReceivePacket {
    pub request_id: u64,
    pub buffer_shm_cap: u64,
    pub buffer_offset: u64,
    pub buffer_len: u32,
}

crate::impl_wire_payload!(ReceivePacket {
    request_id: u64,
    buffer_shm_cap: u64,
    buffer_offset: u64,
    buffer_len: u32,
});

pub const MSG_NET_V1_RECEIVE_PACKET_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReceivePacketReply {
    pub request_id: u64,
    pub status: u32,
    pub bytes_received: u32,
}

crate::impl_wire_payload!(ReceivePacketReply {
    request_id: u64,
    status: u32,
    bytes_received: u32,
});

pub const MSG_NET_V1_SEND_PACKET: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SendPacket {
    pub request_id: u64,
    pub data_shm_cap: u64,
    pub data_offset: u64,
    pub data_len: u32,
}

crate::impl_wire_payload!(SendPacket {
    request_id: u64,
    data_shm_cap: u64,
    data_offset: u64,
    data_len: u32,
});

pub const MSG_NET_V1_SEND_PACKET_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SendPacketReply {
    pub request_id: u64,
    pub status: u32,
    pub bytes_sent: u32,
}

crate::impl_wire_payload!(SendPacketReply {
    request_id: u64,
    status: u32,
    bytes_sent: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_net_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<ReceivePacket>()];
        let value = ReceivePacket::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReceivePacket>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReceivePacket>()];
        let offset = core::mem::offset_of!(ReceivePacket, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReceivePacket, buffer_shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReceivePacket, buffer_offset);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReceivePacket, buffer_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReceivePacket initialized bytes");
        let again = ReceivePacket::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.buffer_shm_cap, again.buffer_shm_cap);
        assert_eq!(value.buffer_offset, again.buffer_offset);
        assert_eq!(value.buffer_len, again.buffer_len);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ReceivePacketReply>()];
        let value = ReceivePacketReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReceivePacketReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReceivePacketReply>()];
        let offset = core::mem::offset_of!(ReceivePacketReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReceivePacketReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReceivePacketReply, bytes_received);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReceivePacketReply initialized bytes");
        let again = ReceivePacketReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.bytes_received, again.bytes_received);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<SendPacket>()];
        let value = SendPacket::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<SendPacket>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<SendPacket>()];
        let offset = core::mem::offset_of!(SendPacket, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SendPacket, data_shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SendPacket, data_offset);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SendPacket, data_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "SendPacket initialized bytes");
        let again = SendPacket::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.data_shm_cap, again.data_shm_cap);
        assert_eq!(value.data_offset, again.data_offset);
        assert_eq!(value.data_len, again.data_len);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<SendPacketReply>()];
        let value = SendPacketReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<SendPacketReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<SendPacketReply>()];
        let offset = core::mem::offset_of!(SendPacketReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SendPacketReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SendPacketReply, bytes_sent);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "SendPacketReply initialized bytes");
        let again = SendPacketReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.bytes_sent, again.bytes_sent);
    }
}
