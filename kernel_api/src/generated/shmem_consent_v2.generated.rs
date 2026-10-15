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

pub const MSG_SHMEM_CONSENT_V2_ACCEPT_REGION_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AcceptRegionReply {
    pub request_id: u64,
    pub mapping_id: u64,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_SHMEM_CONSENT_V2_CANCEL_REGION_OFFER: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CancelRegionOffer {
    pub request_id: u64,
    pub version: u32,
    pub reserved: u32,
    pub offer_id: u64,
}

pub const MSG_SHMEM_CONSENT_V2_CANCEL_REGION_OFFER_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CancelRegionOfferReply {
    pub request_id: u64,
    pub status: u32,
    pub reserved: u32,
}

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

pub const MSG_SHMEM_CONSENT_V2_OFFER_REGION_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct OfferRegionReply {
    pub request_id: u64,
    pub offer_id: u64,
    pub status: u32,
    pub reserved: u32,
}
