// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/services/desktop_focus_v1.toml

// namespace = services.desktop_focus, version = 1

pub const DESKTOP_FOCUS_V1_PROTOCOL_ID: u32 = 832;

pub const MSG_DESKTOP_FOCUS_V1_ASSIGN: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Assign {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
    pub surface_id: u64,
    pub surface_generation: u64,
}

crate::impl_wire_payload!(Assign {
    request_id: u64,
    session_id: u64,
    session_generation: u64,
    instance_id: u64,
    instance_generation: u64,
    surface_id: u64,
    surface_generation: u64,
});

pub const MSG_DESKTOP_FOCUS_V1_ASSIGN_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AssignReply {
    pub request_id: u64,
    pub focus_epoch: u64,
    pub service_epoch: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(AssignReply {
    request_id: u64,
    focus_epoch: u64,
    service_epoch: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_DESKTOP_FOCUS_V1_POLL_KEYS: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PollKeys {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
    pub focus_epoch: u64,
}

crate::impl_wire_payload!(PollKeys {
    request_id: u64,
    session_id: u64,
    session_generation: u64,
    instance_id: u64,
    instance_generation: u64,
    focus_epoch: u64,
});

pub const MSG_DESKTOP_FOCUS_V1_POLL_KEYS_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PollKeysReply {
    pub request_id: u64,
    pub sequence: u64,
    pub focus_epoch: u64,
    pub usage: u32,
    pub phase: u32,
    pub modifiers: u32,
    pub status: u32,
}

crate::impl_wire_payload!(PollKeysReply {
    request_id: u64,
    sequence: u64,
    focus_epoch: u64,
    usage: u32,
    phase: u32,
    modifiers: u32,
    status: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_desktop_focus_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<Assign>()];
        let value = Assign::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<Assign>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<Assign>()];
        let offset = core::mem::offset_of!(Assign, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Assign, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Assign, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Assign, instance_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Assign, instance_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Assign, surface_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Assign, surface_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "Assign initialized bytes");
        let again = Assign::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(value.instance_id, again.instance_id);
        assert_eq!(value.instance_generation, again.instance_generation);
        assert_eq!(value.surface_id, again.surface_id);
        assert_eq!(value.surface_generation, again.surface_generation);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<AssignReply>()];
        let value = AssignReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<AssignReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<AssignReply>()];
        let offset = core::mem::offset_of!(AssignReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AssignReply, focus_epoch);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AssignReply, service_epoch);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AssignReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AssignReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "AssignReply initialized bytes");
        let again = AssignReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.focus_epoch, again.focus_epoch);
        assert_eq!(value.service_epoch, again.service_epoch);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<PollKeys>()];
        let value = PollKeys::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<PollKeys>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<PollKeys>()];
        let offset = core::mem::offset_of!(PollKeys, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollKeys, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollKeys, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollKeys, instance_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollKeys, instance_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollKeys, focus_epoch);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "PollKeys initialized bytes");
        let again = PollKeys::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(value.instance_id, again.instance_id);
        assert_eq!(value.instance_generation, again.instance_generation);
        assert_eq!(value.focus_epoch, again.focus_epoch);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<PollKeysReply>()];
        let value = PollKeysReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<PollKeysReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<PollKeysReply>()];
        let offset = core::mem::offset_of!(PollKeysReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollKeysReply, sequence);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollKeysReply, focus_epoch);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollKeysReply, usage);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollKeysReply, phase);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollKeysReply, modifiers);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollKeysReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "PollKeysReply initialized bytes");
        let again = PollKeysReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.sequence, again.sequence);
        assert_eq!(value.focus_epoch, again.focus_epoch);
        assert_eq!(value.usage, again.usage);
        assert_eq!(value.phase, again.phase);
        assert_eq!(value.modifiers, again.modifiers);
        assert_eq!(value.status, again.status);
    }
}
