// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/portals/notifications_v1.toml

// namespace = portal.notifications, version = 1

pub const NOTIFICATIONS_V1_PROTOCOL_ID: u32 = 288;

pub const MSG_NOTIFICATIONS_V1_POST_NOTIFICATION: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PostNotification {
    pub request_id: u64,
    pub channel: u32,
    pub title_len: u32,
    pub body_len: u32,
}

crate::impl_wire_payload!(PostNotification {
    request_id: u64,
    channel: u32,
    title_len: u32,
    body_len: u32,
});

pub const MSG_NOTIFICATIONS_V1_POST_NOTIFICATION_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PostNotificationReply {
    pub request_id: u64,
    pub status: u32,
    pub notification_id: u64,
    pub reserved: u32,
}

crate::impl_wire_payload!(PostNotificationReply {
    request_id: u64,
    status: u32,
    notification_id: u64,
    reserved: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_notifications_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<PostNotification>()];
        let value = PostNotification::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<PostNotification>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<PostNotification>()];
        let offset = core::mem::offset_of!(PostNotification, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PostNotification, channel);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PostNotification, title_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PostNotification, body_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "PostNotification initialized bytes");
        let again = PostNotification::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.channel, again.channel);
        assert_eq!(value.title_len, again.title_len);
        assert_eq!(value.body_len, again.body_len);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<PostNotificationReply>()];
        let value = PostNotificationReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<PostNotificationReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<PostNotificationReply>()];
        let offset = core::mem::offset_of!(PostNotificationReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PostNotificationReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PostNotificationReply, notification_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PostNotificationReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "PostNotificationReply initialized bytes");
        let again = PostNotificationReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.notification_id, again.notification_id);
        assert_eq!(value.reserved, again.reserved);
    }
}
