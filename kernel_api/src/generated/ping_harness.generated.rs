// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/ping_harness.toml

// namespace = harness.ping, version = 1

pub const PING_HARNESS_PROTOCOL_ID: u32 = 1;

pub const MSG_PING_HARNESS_PING: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Ping {
    pub nonce: u64,
}

crate::impl_wire_payload!(Ping { nonce: u64 });

pub const MSG_PING_HARNESS_PONG: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Pong {
    pub nonce: u64,
}

crate::impl_wire_payload!(Pong { nonce: u64 });

#[cfg(test)]
#[test]
fn wire_codec_ping_harness_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<Ping>()];
        let value = Ping::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<Ping>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<Ping>()];
        let offset = core::mem::offset_of!(Ping, nonce);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "Ping initialized bytes");
        let again = Ping::decode(&output).unwrap();
        assert_eq!(value.nonce, again.nonce);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<Pong>()];
        let value = Pong::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<Pong>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<Pong>()];
        let offset = core::mem::offset_of!(Pong, nonce);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "Pong initialized bytes");
        let again = Pong::decode(&output).unwrap();
        assert_eq!(value.nonce, again.nonce);
    }
}
