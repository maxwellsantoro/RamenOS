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

crate::impl_wire_payload!(GetBlob {
    request_id: u64,
    content_id_hash: [u8; 32],
    capability_token: u64,
});

pub const MSG_STORE_SERVICE_V1_GET_BLOB_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetBlobReply {
    pub request_id: u64,
    pub status: u32,
    pub blob_shm_cap: u64,
    pub blob_size: u64,
}

crate::impl_wire_payload!(GetBlobReply {
    request_id: u64,
    status: u32,
    blob_shm_cap: u64,
    blob_size: u64,
});

pub const MSG_STORE_SERVICE_V1_GET_MANIFEST: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetManifest {
    pub request_id: u64,
    pub content_id_hash: [u8; 32],
    pub capability_token: u64,
}

crate::impl_wire_payload!(GetManifest {
    request_id: u64,
    content_id_hash: [u8; 32],
    capability_token: u64,
});

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

crate::impl_wire_payload!(GetManifestReply {
    request_id: u64,
    status: u32,
    schema_version: u32,
    size_bytes: u64,
    manifest_shm_cap: u64,
    manifest_size: u64,
});

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

crate::impl_wire_payload!(IngestArtifact {
    request_id: u64,
    kind: u32,
    channel: u32,
    src_shm_cap: u64,
    src_len: u64,
    capability_token: u64,
});

pub const MSG_STORE_SERVICE_V1_INGEST_ARTIFACT_REPLY: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct IngestArtifactReply {
    pub request_id: u64,
    pub status: u32,
    pub content_id_hash: [u8; 32],
    pub size_bytes: u64,
}

crate::impl_wire_payload!(IngestArtifactReply {
    request_id: u64,
    status: u32,
    content_id_hash: [u8; 32],
    size_bytes: u64,
});

pub const MSG_STORE_SERVICE_V1_VERIFY_ARTIFACT: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct VerifyArtifact {
    pub request_id: u64,
    pub content_id_hash: [u8; 32],
    pub capability_token: u64,
}

crate::impl_wire_payload!(VerifyArtifact {
    request_id: u64,
    content_id_hash: [u8; 32],
    capability_token: u64,
});

pub const MSG_STORE_SERVICE_V1_VERIFY_ARTIFACT_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct VerifyArtifactReply {
    pub request_id: u64,
    pub status: u32,
    pub valid: u32,
}

crate::impl_wire_payload!(VerifyArtifactReply {
    request_id: u64,
    status: u32,
    valid: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_store_service_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<GetBlob>()];
        let value = GetBlob::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetBlob>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetBlob>()];
        let offset = core::mem::offset_of!(GetBlob, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetBlob, content_id_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetBlob, capability_token);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetBlob initialized bytes");
        let again = GetBlob::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.content_id_hash, again.content_id_hash);
        assert_eq!(value.capability_token, again.capability_token);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GetBlobReply>()];
        let value = GetBlobReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetBlobReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetBlobReply>()];
        let offset = core::mem::offset_of!(GetBlobReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetBlobReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetBlobReply, blob_shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetBlobReply, blob_size);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetBlobReply initialized bytes");
        let again = GetBlobReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.blob_shm_cap, again.blob_shm_cap);
        assert_eq!(value.blob_size, again.blob_size);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GetManifest>()];
        let value = GetManifest::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetManifest>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetManifest>()];
        let offset = core::mem::offset_of!(GetManifest, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetManifest, content_id_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetManifest, capability_token);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetManifest initialized bytes");
        let again = GetManifest::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.content_id_hash, again.content_id_hash);
        assert_eq!(value.capability_token, again.capability_token);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GetManifestReply>()];
        let value = GetManifestReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetManifestReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetManifestReply>()];
        let offset = core::mem::offset_of!(GetManifestReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetManifestReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetManifestReply, schema_version);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetManifestReply, size_bytes);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetManifestReply, manifest_shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetManifestReply, manifest_size);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetManifestReply initialized bytes");
        let again = GetManifestReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.schema_version, again.schema_version);
        assert_eq!(value.size_bytes, again.size_bytes);
        assert_eq!(value.manifest_shm_cap, again.manifest_shm_cap);
        assert_eq!(value.manifest_size, again.manifest_size);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<IngestArtifact>()];
        let value = IngestArtifact::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<IngestArtifact>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<IngestArtifact>()];
        let offset = core::mem::offset_of!(IngestArtifact, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(IngestArtifact, kind);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(IngestArtifact, channel);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(IngestArtifact, src_shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(IngestArtifact, src_len);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(IngestArtifact, capability_token);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "IngestArtifact initialized bytes");
        let again = IngestArtifact::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.kind, again.kind);
        assert_eq!(value.channel, again.channel);
        assert_eq!(value.src_shm_cap, again.src_shm_cap);
        assert_eq!(value.src_len, again.src_len);
        assert_eq!(value.capability_token, again.capability_token);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<IngestArtifactReply>()];
        let value = IngestArtifactReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<IngestArtifactReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<IngestArtifactReply>()];
        let offset = core::mem::offset_of!(IngestArtifactReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(IngestArtifactReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(IngestArtifactReply, content_id_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(IngestArtifactReply, size_bytes);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "IngestArtifactReply initialized bytes");
        let again = IngestArtifactReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.content_id_hash, again.content_id_hash);
        assert_eq!(value.size_bytes, again.size_bytes);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<VerifyArtifact>()];
        let value = VerifyArtifact::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<VerifyArtifact>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<VerifyArtifact>()];
        let offset = core::mem::offset_of!(VerifyArtifact, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(VerifyArtifact, content_id_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(VerifyArtifact, capability_token);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "VerifyArtifact initialized bytes");
        let again = VerifyArtifact::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.content_id_hash, again.content_id_hash);
        assert_eq!(value.capability_token, again.capability_token);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<VerifyArtifactReply>()];
        let value = VerifyArtifactReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<VerifyArtifactReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<VerifyArtifactReply>()];
        let offset = core::mem::offset_of!(VerifyArtifactReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(VerifyArtifactReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(VerifyArtifactReply, valid);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "VerifyArtifactReply initialized bytes");
        let again = VerifyArtifactReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.valid, again.valid);
    }
}
