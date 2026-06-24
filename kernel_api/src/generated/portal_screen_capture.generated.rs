// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/portals/screen_capture_v1.toml

// namespace = portal.screen_capture, version = 1

pub const SCREEN_CAPTURE_V1_PROTOCOL_ID: u32 = 304;

pub const MSG_SCREEN_CAPTURE_V1_CAPTURE_FRAME: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CaptureFrame {
    pub request_id: u64,
    pub display_id: u32,
    pub quality: u16,
    pub reserved: u16,
}

pub const MSG_SCREEN_CAPTURE_V1_CAPTURE_FRAME_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CaptureFrameReply {
    pub request_id: u64,
    pub status: u32,
    pub content_id_hash: u64,
    pub size_bytes: u64,
}
