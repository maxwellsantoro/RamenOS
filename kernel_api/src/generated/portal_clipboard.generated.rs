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

pub const MSG_CLIPBOARD_V1_READ_CLIPBOARD_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadClipboardReply {
    pub request_id: u64,
    pub status: u32,
    pub content_id_hash: u64,
    pub size_bytes: u64,
}
