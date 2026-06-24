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

pub const MSG_NET_V1_RECEIVE_PACKET_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReceivePacketReply {
    pub request_id: u64,
    pub status: u32,
    pub bytes_received: u32,
}

pub const MSG_NET_V1_SEND_PACKET: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SendPacket {
    pub request_id: u64,
    pub data_shm_cap: u64,
    pub data_offset: u64,
    pub data_len: u32,
}

pub const MSG_NET_V1_SEND_PACKET_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SendPacketReply {
    pub request_id: u64,
    pub status: u32,
    pub bytes_sent: u32,
}
