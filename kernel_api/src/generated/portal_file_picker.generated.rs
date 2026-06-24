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

pub const MSG_FILE_PICKER_V1_CANCEL_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CancelReply {
    pub token: u64,
    pub status: u32,
    pub reserved: u32,
}

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

pub const MSG_FILE_PICKER_V1_OPEN_FILE_RO_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct OpenFileRoReply {
    pub request_id: u64,
    pub token: u64,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_FILE_PICKER_V1_RESOLVE_TOKEN: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ResolveToken {
    pub token: u64,
}

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
