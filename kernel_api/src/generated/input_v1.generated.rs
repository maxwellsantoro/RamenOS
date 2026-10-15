// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/input_v1.toml

// namespace = harness.input, version = 1

pub const INPUT_V1_PROTOCOL_ID: u32 = 802;

pub const MSG_INPUT_V1_ATTACH: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Attach {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub device_generation: u64,
}

crate::impl_wire_payload!(Attach {
    request_id: u64,
    session_id: u64,
    session_generation: u64,
    device_generation: u64,
});

pub const MSG_INPUT_V1_ATTACH_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AttachReply {
    pub request_id: u64,
    pub queue_generation: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(AttachReply {
    request_id: u64,
    queue_generation: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_INPUT_V1_PRODUCE_KEY: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ProduceKey {
    pub session_id: u64,
    pub session_generation: u64,
    pub queue_generation: u64,
    pub sequence: u64,
    pub usage: u32,
    pub phase: u32,
    pub modifiers: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(ProduceKey {
    session_id: u64,
    session_generation: u64,
    queue_generation: u64,
    sequence: u64,
    usage: u32,
    phase: u32,
    modifiers: u32,
    reserved: u32,
});

pub const MSG_INPUT_V1_PRODUCE_KEY_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ProduceKeyReply {
    pub sequence: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(ProduceKeyReply {
    sequence: u64,
    status: u32,
    reserved: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_input_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<Attach>()];
        let value = Attach::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<Attach>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<Attach>()];
        let offset = core::mem::offset_of!(Attach, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Attach, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Attach, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Attach, device_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "Attach initialized bytes");
        let again = Attach::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(value.device_generation, again.device_generation);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<AttachReply>()];
        let value = AttachReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<AttachReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<AttachReply>()];
        let offset = core::mem::offset_of!(AttachReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AttachReply, queue_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AttachReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AttachReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "AttachReply initialized bytes");
        let again = AttachReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.queue_generation, again.queue_generation);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ProduceKey>()];
        let value = ProduceKey::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ProduceKey>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ProduceKey>()];
        let offset = core::mem::offset_of!(ProduceKey, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ProduceKey, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ProduceKey, queue_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ProduceKey, sequence);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ProduceKey, usage);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ProduceKey, phase);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ProduceKey, modifiers);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ProduceKey, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ProduceKey initialized bytes");
        let again = ProduceKey::decode(&output).unwrap();
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(value.queue_generation, again.queue_generation);
        assert_eq!(value.sequence, again.sequence);
        assert_eq!(value.usage, again.usage);
        assert_eq!(value.phase, again.phase);
        assert_eq!(value.modifiers, again.modifiers);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ProduceKeyReply>()];
        let value = ProduceKeyReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ProduceKeyReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ProduceKeyReply>()];
        let offset = core::mem::offset_of!(ProduceKeyReply, sequence);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ProduceKeyReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ProduceKeyReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ProduceKeyReply initialized bytes");
        let again = ProduceKeyReply::decode(&output).unwrap();
        assert_eq!(value.sequence, again.sequence);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
}
