// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/shmem_consent_v2.toml

// namespace = shared_memory.consent, version = 2

pub const SHMEM_CONSENT_V2_PROTOCOL_ID: u32 = 531;

pub const MSG_SHMEM_CONSENT_V2_ACCEPT_REGION: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AcceptRegion {
    pub request_id: u64,
    pub version: u32,
    pub reserved: u32,
    pub offer_id: u64,
}

crate::impl_wire_payload!(AcceptRegion {
    request_id: u64,
    version: u32,
    reserved: u32,
    offer_id: u64,
});

pub const MSG_SHMEM_CONSENT_V2_ACCEPT_REGION_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AcceptRegionReply {
    pub request_id: u64,
    pub mapping_id: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(AcceptRegionReply {
    request_id: u64,
    mapping_id: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_SHMEM_CONSENT_V2_CANCEL_REGION_OFFER: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CancelRegionOffer {
    pub request_id: u64,
    pub version: u32,
    pub reserved: u32,
    pub offer_id: u64,
}

crate::impl_wire_payload!(CancelRegionOffer {
    request_id: u64,
    version: u32,
    reserved: u32,
    offer_id: u64,
});

pub const MSG_SHMEM_CONSENT_V2_CANCEL_REGION_OFFER_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CancelRegionOfferReply {
    pub request_id: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(CancelRegionOfferReply {
    request_id: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_SHMEM_CONSENT_V2_OFFER_REGION: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct OfferRegion {
    pub request_id: u64,
    pub version: u32,
    pub reserved: u32,
    pub region_id: u64,
    pub shm_cap: u64,
    pub target_domain_id: u64,
    pub rights: u32,
    pub cache_mode: u32,
}

crate::impl_wire_payload!(OfferRegion {
    request_id: u64,
    version: u32,
    reserved: u32,
    region_id: u64,
    shm_cap: u64,
    target_domain_id: u64,
    rights: u32,
    cache_mode: u32,
});

pub const MSG_SHMEM_CONSENT_V2_OFFER_REGION_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct OfferRegionReply {
    pub request_id: u64,
    pub offer_id: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(OfferRegionReply {
    request_id: u64,
    offer_id: u64,
    status: u32,
    reserved: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_shmem_consent_v2_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<AcceptRegion>()];
        let value = AcceptRegion::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<AcceptRegion>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<AcceptRegion>()];
        let offset = core::mem::offset_of!(AcceptRegion, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcceptRegion, version);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcceptRegion, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcceptRegion, offer_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "AcceptRegion initialized bytes");
        let again = AcceptRegion::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.version, again.version);
        assert_eq!(value.reserved, again.reserved);
        assert_eq!(value.offer_id, again.offer_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<AcceptRegionReply>()];
        let value = AcceptRegionReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<AcceptRegionReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<AcceptRegionReply>()];
        let offset = core::mem::offset_of!(AcceptRegionReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcceptRegionReply, mapping_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcceptRegionReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcceptRegionReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "AcceptRegionReply initialized bytes");
        let again = AcceptRegionReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.mapping_id, again.mapping_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CancelRegionOffer>()];
        let value = CancelRegionOffer::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CancelRegionOffer>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CancelRegionOffer>()];
        let offset = core::mem::offset_of!(CancelRegionOffer, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelRegionOffer, version);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelRegionOffer, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelRegionOffer, offer_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CancelRegionOffer initialized bytes");
        let again = CancelRegionOffer::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.version, again.version);
        assert_eq!(value.reserved, again.reserved);
        assert_eq!(value.offer_id, again.offer_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CancelRegionOfferReply>()];
        let value = CancelRegionOfferReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CancelRegionOfferReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CancelRegionOfferReply>()];
        let offset = core::mem::offset_of!(CancelRegionOfferReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelRegionOfferReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelRegionOfferReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CancelRegionOfferReply initialized bytes");
        let again = CancelRegionOfferReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<OfferRegion>()];
        let value = OfferRegion::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<OfferRegion>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<OfferRegion>()];
        let offset = core::mem::offset_of!(OfferRegion, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegion, version);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegion, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegion, region_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegion, shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegion, target_domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegion, rights);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegion, cache_mode);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "OfferRegion initialized bytes");
        let again = OfferRegion::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.version, again.version);
        assert_eq!(value.reserved, again.reserved);
        assert_eq!(value.region_id, again.region_id);
        assert_eq!(value.shm_cap, again.shm_cap);
        assert_eq!(value.target_domain_id, again.target_domain_id);
        assert_eq!(value.rights, again.rights);
        assert_eq!(value.cache_mode, again.cache_mode);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<OfferRegionReply>()];
        let value = OfferRegionReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<OfferRegionReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<OfferRegionReply>()];
        let offset = core::mem::offset_of!(OfferRegionReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegionReply, offer_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegionReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OfferRegionReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "OfferRegionReply initialized bytes");
        let again = OfferRegionReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.offer_id, again.offer_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
}
