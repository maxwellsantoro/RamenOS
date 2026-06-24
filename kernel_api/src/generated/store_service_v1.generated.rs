// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/services/store_service_v1.toml

// namespace = store.service, version = 1

pub const STORE_SERVICE_V1_PROTOCOL_ID: u32 = 1024;

pub const MSG_STORE_SERVICE_V1_GET_BLOB: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetBlob {
    pub request_id: u64,
    pub content_id_hash: [u8; 32],
    pub capability_token: u64,
}

pub const MSG_STORE_SERVICE_V1_GET_BLOB_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetBlobReply {
    pub request_id: u64,
    pub status: u32,
    pub blob_shm_cap: u64,
    pub blob_size: u64,
}

pub const MSG_STORE_SERVICE_V1_GET_MANIFEST: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetManifest {
    pub request_id: u64,
    pub content_id_hash: [u8; 32],
    pub capability_token: u64,
}

pub const MSG_STORE_SERVICE_V1_GET_MANIFEST_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetManifestReply {
    pub request_id: u64,
    pub status: u32,
    pub schema_version: u32,
    pub size_bytes: u64,
    pub manifest_shm_cap: u64,
    pub manifest_size: u64,
}

pub const MSG_STORE_SERVICE_V1_INGEST_ARTIFACT: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct IngestArtifact {
    pub request_id: u64,
    pub kind: u32,
    pub channel: u32,
    pub src_shm_cap: u64,
    pub src_len: u64,
    pub capability_token: u64,
}

pub const MSG_STORE_SERVICE_V1_INGEST_ARTIFACT_REPLY: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct IngestArtifactReply {
    pub request_id: u64,
    pub status: u32,
    pub content_id_hash: [u8; 32],
    pub size_bytes: u64,
}

pub const MSG_STORE_SERVICE_V1_VERIFY_ARTIFACT: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct VerifyArtifact {
    pub request_id: u64,
    pub content_id_hash: [u8; 32],
    pub capability_token: u64,
}

pub const MSG_STORE_SERVICE_V1_VERIFY_ARTIFACT_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct VerifyArtifactReply {
    pub request_id: u64,
    pub status: u32,
    pub valid: u32,
}
