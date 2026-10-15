// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/portals/clipboard_v1.toml

// namespace = portal.clipboard, version = 1

pub const CLIPBOARD_V1_PROTOCOL_ID: u32 = 272;

pub const MSG_CLIPBOARD_V1_READ_CLIPBOARD: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadClipboard {
    pub request_id: u64,
    pub format: u32,
}

crate::impl_wire_payload!(ReadClipboard {
    request_id: u64,
    format: u32,
});

pub const MSG_CLIPBOARD_V1_READ_CLIPBOARD_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadClipboardReply {
    pub request_id: u64,
    pub status: u32,
    pub content_id_hash: u64,
    pub size_bytes: u64,
}

crate::impl_wire_payload!(ReadClipboardReply {
    request_id: u64,
    status: u32,
    content_id_hash: u64,
    size_bytes: u64,
});

#[cfg(test)]
#[test]
fn wire_codec_clipboard_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<ReadClipboard>()];
        let value = ReadClipboard::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReadClipboard>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReadClipboard>()];
        let offset = core::mem::offset_of!(ReadClipboard, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadClipboard, format);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReadClipboard initialized bytes");
        let again = ReadClipboard::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.format, again.format);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ReadClipboardReply>()];
        let value = ReadClipboardReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReadClipboardReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReadClipboardReply>()];
        let offset = core::mem::offset_of!(ReadClipboardReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadClipboardReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadClipboardReply, content_id_hash);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadClipboardReply, size_bytes);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReadClipboardReply initialized bytes");
        let again = ReadClipboardReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.content_id_hash, again.content_id_hash);
        assert_eq!(value.size_bytes, again.size_bytes);
    }
}
