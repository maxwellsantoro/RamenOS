// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/shmem_outcome_v3.toml

// namespace = shared_memory.outcome, version = 3

pub const SHMEM_OUTCOME_V3_PROTOCOL_ID: u32 = 533;

pub const MSG_SHMEM_OUTCOME_V3_ACCEPT_REGION_TRACKED: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AcceptRegionTracked {
    pub request_id: u64,
    pub version: u32,
    pub reserved: u32,
    pub offer_id: u64,
}

crate::impl_wire_payload!(AcceptRegionTracked {
    request_id: u64,
    version: u32,
    reserved: u32,
    offer_id: u64,
});

pub const MSG_SHMEM_OUTCOME_V3_ACCEPT_REGION_TRACKED_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AcceptRegionTrackedReply {
    pub request_id: u64,
    pub offer_id: u64,
    pub mapping_id: u64,
    pub status: u32,
    pub state: u32,
}

crate::impl_wire_payload!(AcceptRegionTrackedReply {
    request_id: u64,
    offer_id: u64,
    mapping_id: u64,
    status: u32,
    state: u32,
});

pub const MSG_SHMEM_OUTCOME_V3_ACKNOWLEDGE_REGION_OUTCOME: u32 = 11;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AcknowledgeRegionOutcome {
    pub request_id: u64,
    pub version: u32,
    pub reserved: u32,
    pub offer_id: u64,
}

crate::impl_wire_payload!(AcknowledgeRegionOutcome {
    request_id: u64,
    version: u32,
    reserved: u32,
    offer_id: u64,
});

pub const MSG_SHMEM_OUTCOME_V3_ACKNOWLEDGE_REGION_OUTCOME_REPLY: u32 = 12;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AcknowledgeRegionOutcomeReply {
    pub request_id: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(AcknowledgeRegionOutcomeReply {
    request_id: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_SHMEM_OUTCOME_V3_CANCEL_TRACKED_OFFER: u32 = 9;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CancelTrackedOffer {
    pub request_id: u64,
    pub version: u32,
    pub reserved: u32,
    pub offer_id: u64,
}

crate::impl_wire_payload!(CancelTrackedOffer {
    request_id: u64,
    version: u32,
    reserved: u32,
    offer_id: u64,
});

pub const MSG_SHMEM_OUTCOME_V3_CANCEL_TRACKED_OFFER_REPLY: u32 = 10;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CancelTrackedOfferReply {
    pub request_id: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(CancelTrackedOfferReply {
    request_id: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_SHMEM_OUTCOME_V3_OFFER_REGION_TRACKED: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct OfferRegionTracked {
    pub request_id: u64,
    pub version: u32,
    pub reserved: u32,
    pub operation_key: u64,
    pub region_id: u64,
    pub shm_cap: u64,
    pub target_domain_id: u64,
    pub rights: u32,
    pub cache_mode: u32,
}

crate::impl_wire_payload!(OfferRegionTracked {
    request_id: u64,
    version: u32,
    reserved: u32,
    operation_key: u64,
    region_id: u64,
    shm_cap: u64,
    target_domain_id: u64,
    rights: u32,
    cache_mode: u32,
});

pub const MSG_SHMEM_OUTCOME_V3_OFFER_REGION_TRACKED_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct OfferRegionTrackedReply {
    pub request_id: u64,
    pub operation_key: u64,
    pub offer_id: u64,
    pub status: u32,
    pub state: u32,
}

crate::impl_wire_payload!(OfferRegionTrackedReply {
    request_id: u64,
    operation_key: u64,
    offer_id: u64,
    status: u32,
    state: u32,
});

pub const MSG_SHMEM_OUTCOME_V3_QUERY_ACCEPTED_REGION: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct QueryAcceptedRegion {
    pub request_id: u64,
    pub version: u32,
    pub reserved: u32,
    pub offer_id: u64,
}

crate::impl_wire_payload!(QueryAcceptedRegion {
    request_id: u64,
    version: u32,
    reserved: u32,
    offer_id: u64,
});

pub const MSG_SHMEM_OUTCOME_V3_QUERY_ACCEPTED_REGION_REPLY: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct QueryAcceptedRegionReply {
    pub request_id: u64,
    pub offer_id: u64,
    pub mapping_id: u64,
    pub status: u32,
    pub state: u32,
}

crate::impl_wire_payload!(QueryAcceptedRegionReply {
    request_id: u64,
    offer_id: u64,
    mapping_id: u64,
    status: u32,
    state: u32,
});

pub const MSG_SHMEM_OUTCOME_V3_QUERY_OFFERED_REGION: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct QueryOfferedRegion {
    pub request_id: u64,
    pub version: u32,
    pub reserved: u32,
    pub operation_key: u64,
}

crate::impl_wire_payload!(QueryOfferedRegion {
    request_id: u64,
    version: u32,
    reserved: u32,
    operation_key: u64,
});

pub const MSG_SHMEM_OUTCOME_V3_QUERY_OFFERED_REGION_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct QueryOfferedRegionReply {
    pub request_id: u64,
    pub operation_key: u64,
    pub offer_id: u64,
    pub mapping_id: u64,
    pub status: u32,
    pub state: u32,
}

crate::impl_wire_payload!(QueryOfferedRegionReply {
    request_id: u64,
    operation_key: u64,
    offer_id: u64,
    mapping_id: u64,
    status: u32,
    state: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_shmem_outcome_v3_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<AcceptRegionTracked>()];
        let value = AcceptRegionTracked::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<AcceptRegionTracked>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<AcceptRegionTracked>()];
        let offset = core::mem::offset_of!(AcceptRegionTracked, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcceptRegionTracked, version);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcceptRegionTracked, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcceptRegionTracked, offer_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "AcceptRegionTracked initialized bytes");
        let again = AcceptRegionTracked::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.version, again.version);
        assert_eq!(value.reserved, again.reserved);
        assert_eq!(value.offer_id, again.offer_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<AcceptRegionTrackedReply>()];
        let value = AcceptRegionTrackedReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<AcceptRegionTrackedReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<AcceptRegionTrackedReply>()];
        let offset = core::mem::offset_of!(AcceptRegionTrackedReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcceptRegionTrackedReply, offer_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcceptRegionTrackedReply, mapping_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcceptRegionTrackedReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcceptRegionTrackedReply, state);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(
            output, expected,
            "AcceptRegionTrackedReply initialized bytes"
        );
        let again = AcceptRegionTrackedReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.offer_id, again.offer_id);
        assert_eq!(value.mapping_id, again.mapping_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.state, again.state);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<AcknowledgeRegionOutcome>()];
        let value = AcknowledgeRegionOutcome::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<AcknowledgeRegionOutcome>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<AcknowledgeRegionOutcome>()];
        let offset = core::mem::offset_of!(AcknowledgeRegionOutcome, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcknowledgeRegionOutcome, version);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcknowledgeRegionOutcome, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcknowledgeRegionOutcome, offer_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(
            output, expected,
            "AcknowledgeRegionOutcome initialized bytes"
        );
        let again = AcknowledgeRegionOutcome::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.version, again.version);
        assert_eq!(value.reserved, again.reserved);
        assert_eq!(value.offer_id, again.offer_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<AcknowledgeRegionOutcomeReply>()];
        let value = AcknowledgeRegionOutcomeReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<AcknowledgeRegionOutcomeReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<AcknowledgeRegionOutcomeReply>()];
        let offset = core::mem::offset_of!(AcknowledgeRegionOutcomeReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcknowledgeRegionOutcomeReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcknowledgeRegionOutcomeReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(
            output, expected,
            "AcknowledgeRegionOutcomeReply initialized bytes"
        );
        let again = AcknowledgeRegionOutcomeReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CancelTrackedOffer>()];
        let value = CancelTrackedOffer::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CancelTrackedOffer>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CancelTrackedOffer>()];
        let offset = core::mem::offset_of!(CancelTrackedOffer, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelTrackedOffer, version);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelTrackedOffer, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelTrackedOffer, offer_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CancelTrackedOffer initialized bytes");
        let again = CancelTrackedOffer::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.version, again.version);
        assert_eq!(value.reserved, again.reserved);
        assert_eq!(value.offer_id, again.offer_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CancelTrackedOfferReply>()];
        let value = CancelTrackedOfferReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CancelTrackedOfferReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CancelTrackedOfferReply>()];
        let offset = core::mem::offset_of!(CancelTrackedOfferReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelTrackedOfferReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelTrackedOfferReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(
            output, expected,
            "CancelTrackedOfferReply initialized bytes"
        );
        let again = CancelTrackedOfferReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<OfferRegionTracked>()];
        let value = OfferRegionTracked::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<OfferRegionTracked>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<OfferRegionTracked>()];
        let offset = core::mem::offset_of!(OfferRegionTracked, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegionTracked, version);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegionTracked, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegionTracked, operation_key);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegionTracked, region_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegionTracked, shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegionTracked, target_domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegionTracked, rights);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegionTracked, cache_mode);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "OfferRegionTracked initialized bytes");
        let again = OfferRegionTracked::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.version, again.version);
        assert_eq!(value.reserved, again.reserved);
        assert_eq!(value.operation_key, again.operation_key);
        assert_eq!(value.region_id, again.region_id);
        assert_eq!(value.shm_cap, again.shm_cap);
        assert_eq!(value.target_domain_id, again.target_domain_id);
        assert_eq!(value.rights, again.rights);
        assert_eq!(value.cache_mode, again.cache_mode);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<OfferRegionTrackedReply>()];
        let value = OfferRegionTrackedReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<OfferRegionTrackedReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<OfferRegionTrackedReply>()];
        let offset = core::mem::offset_of!(OfferRegionTrackedReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegionTrackedReply, operation_key);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegionTrackedReply, offer_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegionTrackedReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegionTrackedReply, state);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(
            output, expected,
            "OfferRegionTrackedReply initialized bytes"
        );
        let again = OfferRegionTrackedReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.operation_key, again.operation_key);
        assert_eq!(value.offer_id, again.offer_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.state, again.state);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<QueryAcceptedRegion>()];
        let value = QueryAcceptedRegion::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<QueryAcceptedRegion>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<QueryAcceptedRegion>()];
        let offset = core::mem::offset_of!(QueryAcceptedRegion, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryAcceptedRegion, version);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryAcceptedRegion, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryAcceptedRegion, offer_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "QueryAcceptedRegion initialized bytes");
        let again = QueryAcceptedRegion::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.version, again.version);
        assert_eq!(value.reserved, again.reserved);
        assert_eq!(value.offer_id, again.offer_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<QueryAcceptedRegionReply>()];
        let value = QueryAcceptedRegionReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<QueryAcceptedRegionReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<QueryAcceptedRegionReply>()];
        let offset = core::mem::offset_of!(QueryAcceptedRegionReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryAcceptedRegionReply, offer_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryAcceptedRegionReply, mapping_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryAcceptedRegionReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryAcceptedRegionReply, state);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(
            output, expected,
            "QueryAcceptedRegionReply initialized bytes"
        );
        let again = QueryAcceptedRegionReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.offer_id, again.offer_id);
        assert_eq!(value.mapping_id, again.mapping_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.state, again.state);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<QueryOfferedRegion>()];
        let value = QueryOfferedRegion::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<QueryOfferedRegion>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<QueryOfferedRegion>()];
        let offset = core::mem::offset_of!(QueryOfferedRegion, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryOfferedRegion, version);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryOfferedRegion, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryOfferedRegion, operation_key);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "QueryOfferedRegion initialized bytes");
        let again = QueryOfferedRegion::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.version, again.version);
        assert_eq!(value.reserved, again.reserved);
        assert_eq!(value.operation_key, again.operation_key);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<QueryOfferedRegionReply>()];
        let value = QueryOfferedRegionReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<QueryOfferedRegionReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<QueryOfferedRegionReply>()];
        let offset = core::mem::offset_of!(QueryOfferedRegionReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryOfferedRegionReply, operation_key);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryOfferedRegionReply, offer_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryOfferedRegionReply, mapping_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryOfferedRegionReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryOfferedRegionReply, state);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(
            output, expected,
            "QueryOfferedRegionReply initialized bytes"
        );
        let again = QueryOfferedRegionReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.operation_key, again.operation_key);
        assert_eq!(value.offer_id, again.offer_id);
        assert_eq!(value.mapping_id, again.mapping_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.state, again.state);
    }
}
