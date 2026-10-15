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

crate::impl_wire_payload!(CaptureFrame {
    request_id: u64,
    display_id: u32,
    quality: u16,
    reserved: u16,
});

pub const MSG_SCREEN_CAPTURE_V1_CAPTURE_FRAME_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CaptureFrameReply {
    pub request_id: u64,
    pub status: u32,
    pub content_id_hash: u64,
    pub size_bytes: u64,
}

crate::impl_wire_payload!(CaptureFrameReply {
    request_id: u64,
    status: u32,
    content_id_hash: u64,
    size_bytes: u64,
});

#[cfg(test)]
#[test]
fn wire_codec_screen_capture_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<CaptureFrame>()];
        let value = CaptureFrame::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CaptureFrame>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CaptureFrame>()];
        let offset = core::mem::offset_of!(CaptureFrame, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CaptureFrame, display_id);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CaptureFrame, quality);
        expected[offset..offset + <u16 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CaptureFrame, reserved);
        expected[offset..offset + <u16 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CaptureFrame initialized bytes");
        let again = CaptureFrame::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.display_id, again.display_id);
        assert_eq!(value.quality, again.quality);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CaptureFrameReply>()];
        let value = CaptureFrameReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CaptureFrameReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CaptureFrameReply>()];
        let offset = core::mem::offset_of!(CaptureFrameReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CaptureFrameReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CaptureFrameReply, content_id_hash);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CaptureFrameReply, size_bytes);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CaptureFrameReply initialized bytes");
        let again = CaptureFrameReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.content_id_hash, again.content_id_hash);
        assert_eq!(value.size_bytes, again.size_bytes);
    }
}
