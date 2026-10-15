// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/portals/file_picker_v1.toml

// namespace = portal.file_picker, version = 1

pub const FILE_PICKER_V1_PROTOCOL_ID: u32 = 256;

pub const MSG_FILE_PICKER_V1_CANCEL: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Cancel {
    pub token: u64,
}

crate::impl_wire_payload!(Cancel { token: u64 });

pub const MSG_FILE_PICKER_V1_CANCEL_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CancelReply {
    pub token: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(CancelReply {
    token: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_FILE_PICKER_V1_OPEN_FILE_RO: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct OpenFileRo {
    pub request_id: u64,
    pub purpose: u32,
    pub allow_multiple: u8,
    pub reserved0: u8,
    pub reserved1: u16,
}

crate::impl_wire_payload!(OpenFileRo {
    request_id: u64,
    purpose: u32,
    allow_multiple: u8,
    reserved0: u8,
    reserved1: u16,
});

pub const MSG_FILE_PICKER_V1_OPEN_FILE_RO_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct OpenFileRoReply {
    pub request_id: u64,
    pub token: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(OpenFileRoReply {
    request_id: u64,
    token: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_FILE_PICKER_V1_RESOLVE_TOKEN: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ResolveToken {
    pub token: u64,
}

crate::impl_wire_payload!(ResolveToken { token: u64 });

pub const MSG_FILE_PICKER_V1_RESOLVE_TOKEN_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ResolveTokenReply {
    pub token: u64,
    pub status: u32,
    pub reserved: u32,
    pub content_id_hash: u64,
    pub size_bytes: u64,
}

crate::impl_wire_payload!(ResolveTokenReply {
    token: u64,
    status: u32,
    reserved: u32,
    content_id_hash: u64,
    size_bytes: u64,
});

#[cfg(test)]
#[test]
fn wire_codec_file_picker_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<Cancel>()];
        let value = Cancel::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<Cancel>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<Cancel>()];
        let offset = core::mem::offset_of!(Cancel, token);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "Cancel initialized bytes");
        let again = Cancel::decode(&output).unwrap();
        assert_eq!(value.token, again.token);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CancelReply>()];
        let value = CancelReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CancelReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CancelReply>()];
        let offset = core::mem::offset_of!(CancelReply, token);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CancelReply initialized bytes");
        let again = CancelReply::decode(&output).unwrap();
        assert_eq!(value.token, again.token);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<OpenFileRo>()];
        let value = OpenFileRo::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<OpenFileRo>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<OpenFileRo>()];
        let offset = core::mem::offset_of!(OpenFileRo, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OpenFileRo, purpose);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OpenFileRo, allow_multiple);
        expected[offset..offset + <u8 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OpenFileRo, reserved0);
        expected[offset..offset + <u8 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OpenFileRo, reserved1);
        expected[offset..offset + <u16 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "OpenFileRo initialized bytes");
        let again = OpenFileRo::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.purpose, again.purpose);
        assert_eq!(value.allow_multiple, again.allow_multiple);
        assert_eq!(value.reserved0, again.reserved0);
        assert_eq!(value.reserved1, again.reserved1);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<OpenFileRoReply>()];
        let value = OpenFileRoReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<OpenFileRoReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<OpenFileRoReply>()];
        let offset = core::mem::offset_of!(OpenFileRoReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OpenFileRoReply, token);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OpenFileRoReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(OpenFileRoReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "OpenFileRoReply initialized bytes");
        let again = OpenFileRoReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.token, again.token);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ResolveToken>()];
        let value = ResolveToken::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ResolveToken>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ResolveToken>()];
        let offset = core::mem::offset_of!(ResolveToken, token);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ResolveToken initialized bytes");
        let again = ResolveToken::decode(&output).unwrap();
        assert_eq!(value.token, again.token);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ResolveTokenReply>()];
        let value = ResolveTokenReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ResolveTokenReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ResolveTokenReply>()];
        let offset = core::mem::offset_of!(ResolveTokenReply, token);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ResolveTokenReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ResolveTokenReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ResolveTokenReply, content_id_hash);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ResolveTokenReply, size_bytes);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ResolveTokenReply initialized bytes");
        let again = ResolveTokenReply::decode(&output).unwrap();
        assert_eq!(value.token, again.token);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
        assert_eq!(value.content_id_hash, again.content_id_hash);
        assert_eq!(value.size_bytes, again.size_bytes);
    }
}
