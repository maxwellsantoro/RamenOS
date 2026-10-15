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

crate::impl_wire_payload!(QueryByPath {
    cap_handle: u64,
    request_id: u64,
    path_shm_cap: u64,
    path_len: u32,
});

pub const MSG_SEMANTIC_STORE_V1_QUERY_BY_PATH_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct QueryByPathReply {
    pub request_id: u64,
    pub status: u32,
    pub content_id_hash: [u8; 32],
}

crate::impl_wire_payload!(QueryByPathReply {
    request_id: u64,
    status: u32,
    content_id_hash: [u8; 32],
});

pub const MSG_SEMANTIC_STORE_V1_QUERY_BY_TAG: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct QueryByTag {
    pub cap_handle: u64,
    pub request_id: u64,
    pub tag_shm_cap: u64,
    pub tag_len: u32,
}

crate::impl_wire_payload!(QueryByTag {
    cap_handle: u64,
    request_id: u64,
    tag_shm_cap: u64,
    tag_len: u32,
});

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

crate::impl_wire_payload!(QueryByTagReply {
    request_id: u64,
    status: u32,
    result_shm_cap: u64,
    result_len: u32,
    content_id_count: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_semantic_store_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<QueryByPath>()];
        let value = QueryByPath::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<QueryByPath>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<QueryByPath>()];
        let offset = core::mem::offset_of!(QueryByPath, cap_handle);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryByPath, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryByPath, path_shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryByPath, path_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "QueryByPath initialized bytes");
        let again = QueryByPath::decode(&output).unwrap();
        assert_eq!(value.cap_handle, again.cap_handle);
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.path_shm_cap, again.path_shm_cap);
        assert_eq!(value.path_len, again.path_len);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<QueryByPathReply>()];
        let value = QueryByPathReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<QueryByPathReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<QueryByPathReply>()];
        let offset = core::mem::offset_of!(QueryByPathReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryByPathReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryByPathReply, content_id_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "QueryByPathReply initialized bytes");
        let again = QueryByPathReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.content_id_hash, again.content_id_hash);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<QueryByTag>()];
        let value = QueryByTag::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<QueryByTag>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<QueryByTag>()];
        let offset = core::mem::offset_of!(QueryByTag, cap_handle);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryByTag, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryByTag, tag_shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryByTag, tag_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "QueryByTag initialized bytes");
        let again = QueryByTag::decode(&output).unwrap();
        assert_eq!(value.cap_handle, again.cap_handle);
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.tag_shm_cap, again.tag_shm_cap);
        assert_eq!(value.tag_len, again.tag_len);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<QueryByTagReply>()];
        let value = QueryByTagReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<QueryByTagReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<QueryByTagReply>()];
        let offset = core::mem::offset_of!(QueryByTagReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryByTagReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryByTagReply, result_shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryByTagReply, result_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(QueryByTagReply, content_id_count);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "QueryByTagReply initialized bytes");
        let again = QueryByTagReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.result_shm_cap, again.result_shm_cap);
        assert_eq!(value.result_len, again.result_len);
        assert_eq!(value.content_id_count, again.content_id_count);
    }
}
