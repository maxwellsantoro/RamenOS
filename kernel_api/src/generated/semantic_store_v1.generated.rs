// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/semantic_store_v1.toml

// namespace = harness.semantic_store, version = 1

pub const SEMANTIC_STORE_V1_PROTOCOL_ID: u32 = 12;

pub const MSG_SEMANTIC_STORE_V1_QUERY_BY_PATH: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct QueryByPath {
    pub cap_handle: u64,
    pub request_id: u64,
    pub path_shm_cap: u64,
    pub path_len: u32,
}

pub const MSG_SEMANTIC_STORE_V1_QUERY_BY_PATH_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct QueryByPathReply {
    pub request_id: u64,
    pub status: u32,
    pub content_id_hash: [u8; 32],
}

pub const MSG_SEMANTIC_STORE_V1_QUERY_BY_TAG: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct QueryByTag {
    pub cap_handle: u64,
    pub request_id: u64,
    pub tag_shm_cap: u64,
    pub tag_len: u32,
}

pub const MSG_SEMANTIC_STORE_V1_QUERY_BY_TAG_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct QueryByTagReply {
    pub request_id: u64,
    pub status: u32,
    pub result_shm_cap: u64,
    pub result_len: u32,
    pub content_id_count: u32,
}
