// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/services/semantic_state_v1.toml

// namespace = services.semantic_state, version = 1

pub const SEMANTIC_STATE_V1_PROTOCOL_ID: u32 = 10;

pub const MSG_SEMANTIC_STATE_V1_GET_SNAPSHOT: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetSnapshot {
    pub cap_handle: u64,
    pub request_id: u64,
    pub format: u32,
}

crate::impl_wire_payload!(GetSnapshot {
    cap_handle: u64,
    request_id: u64,
    format: u32,
});

pub const MSG_SEMANTIC_STATE_V1_GET_SNAPSHOT_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetSnapshotReply {
    pub request_id: u64,
    pub status: u32,
    pub shm_cap: u64,
    pub shm_size: u64,
}

crate::impl_wire_payload!(GetSnapshotReply {
    request_id: u64,
    status: u32,
    shm_cap: u64,
    shm_size: u64,
});

pub const MSG_SEMANTIC_STATE_V1_STATE_CHANGED_EVENT: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StateChangedEvent {
    pub subscription_id: u64,
    pub event_type: u32,
    pub shm_cap: u64,
}

crate::impl_wire_payload!(StateChangedEvent {
    subscription_id: u64,
    event_type: u32,
    shm_cap: u64,
});

pub const MSG_SEMANTIC_STATE_V1_SUBSCRIBE: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Subscribe {
    pub request_id: u64,
    pub event_mask: u32,
}

crate::impl_wire_payload!(Subscribe {
    request_id: u64,
    event_mask: u32,
});

pub const MSG_SEMANTIC_STATE_V1_SUBSCRIBE_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SubscribeReply {
    pub request_id: u64,
    pub status: u32,
    pub subscription_id: u64,
}

crate::impl_wire_payload!(SubscribeReply {
    request_id: u64,
    status: u32,
    subscription_id: u64,
});

#[cfg(test)]
#[test]
fn wire_codec_semantic_state_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<GetSnapshot>()];
        let value = GetSnapshot::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetSnapshot>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetSnapshot>()];
        let offset = core::mem::offset_of!(GetSnapshot, cap_handle);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetSnapshot, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetSnapshot, format);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetSnapshot initialized bytes");
        let again = GetSnapshot::decode(&output).unwrap();
        assert_eq!(value.cap_handle, again.cap_handle);
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.format, again.format);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GetSnapshotReply>()];
        let value = GetSnapshotReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetSnapshotReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetSnapshotReply>()];
        let offset = core::mem::offset_of!(GetSnapshotReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetSnapshotReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetSnapshotReply, shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetSnapshotReply, shm_size);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetSnapshotReply initialized bytes");
        let again = GetSnapshotReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.shm_cap, again.shm_cap);
        assert_eq!(value.shm_size, again.shm_size);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<StateChangedEvent>()];
        let value = StateChangedEvent::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<StateChangedEvent>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<StateChangedEvent>()];
        let offset = core::mem::offset_of!(StateChangedEvent, subscription_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StateChangedEvent, event_type);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StateChangedEvent, shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "StateChangedEvent initialized bytes");
        let again = StateChangedEvent::decode(&output).unwrap();
        assert_eq!(value.subscription_id, again.subscription_id);
        assert_eq!(value.event_type, again.event_type);
        assert_eq!(value.shm_cap, again.shm_cap);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<Subscribe>()];
        let value = Subscribe::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<Subscribe>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<Subscribe>()];
        let offset = core::mem::offset_of!(Subscribe, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Subscribe, event_mask);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "Subscribe initialized bytes");
        let again = Subscribe::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.event_mask, again.event_mask);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<SubscribeReply>()];
        let value = SubscribeReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<SubscribeReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<SubscribeReply>()];
        let offset = core::mem::offset_of!(SubscribeReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubscribeReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubscribeReply, subscription_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "SubscribeReply initialized bytes");
        let again = SubscribeReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.subscription_id, again.subscription_id);
    }
}
