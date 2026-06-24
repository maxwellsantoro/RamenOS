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

pub const MSG_NOTIFICATIONS_V1_POST_NOTIFICATION_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PostNotificationReply {
    pub request_id: u64,
    pub status: u32,
    pub notification_id: u64,
    pub reserved: u32,
}
