// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/portals/desktop_session_v1.toml

// namespace = portal.desktop_session, version = 1

pub const DESKTOP_SESSION_V1_PROTOCOL_ID: u32 = 336;

pub const MSG_DESKTOP_SESSION_V1_CANCEL_PREVIEW: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CancelPreview {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub plan_id: u64,
}

pub const MSG_DESKTOP_SESSION_V1_CANCEL_PREVIEW_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CancelPreviewReply {
    pub request_id: u64,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_DESKTOP_SESSION_V1_CLOSE_INSTANCE: u32 = 9;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CloseInstance {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
}

pub const MSG_DESKTOP_SESSION_V1_CLOSE_INSTANCE_REPLY: u32 = 10;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CloseInstanceReply {
    pub request_id: u64,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_DESKTOP_SESSION_V1_CONFIRM_LAUNCH: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ConfirmLaunch {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub plan_id: u64,
    pub preview_revision: u64,
}

pub const MSG_DESKTOP_SESSION_V1_CONFIRM_LAUNCH_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ConfirmLaunchReply {
    pub request_id: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
    pub observation_handle: u64,
    pub granted_rights: u32,
    pub status: u32,
    pub child_pid: u32,
    pub reserved: u32,
}

pub const MSG_DESKTOP_SESSION_V1_GET_STATUS: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetStatus {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
}

pub const MSG_DESKTOP_SESSION_V1_GET_STATUS_REPLY: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetStatusReply {
    pub request_id: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
    pub executable_hash: [u8; 32],
    pub state: u32,
    pub status: u32,
}

pub const MSG_DESKTOP_SESSION_V1_INSTANCE_BOOTSTRAP: u32 = 19;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct InstanceBootstrap {
    pub session_id: u64,
    pub session_generation: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
    pub observation_handle: u64,
    pub granted_rights: u32,
    pub reserved: u32,
}

pub const MSG_DESKTOP_SESSION_V1_OBSERVE_INSTANCE: u32 = 17;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ObserveInstance {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
}

pub const MSG_DESKTOP_SESSION_V1_OBSERVE_INSTANCE_REPLY: u32 = 18;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ObserveInstanceReply {
    pub request_id: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
    pub executable_hash: [u8; 32],
    pub state: u32,
    pub status: u32,
}

pub const MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PrepareLaunch {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub application_hash: [u8; 32],
    pub requested_rights: u32,
    pub reserved: u32,
}

pub const MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PrepareLaunchReply {
    pub request_id: u64,
    pub plan_id: u64,
    pub preview_revision: u64,
    pub preview_shm: u64,
    pub preview_len: u32,
    pub status: u32,
    pub granted_rights: u32,
    pub reserved: u32,
}

pub const MSG_DESKTOP_SESSION_V1_PREPARE_RESTART: u32 = 13;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PrepareRestart {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
}

pub const MSG_DESKTOP_SESSION_V1_PREPARE_RESTART_REPLY: u32 = 14;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PrepareRestartReply {
    pub request_id: u64,
    pub plan_id: u64,
    pub preview_revision: u64,
    pub preview_shm: u64,
    pub preview_len: u32,
    pub status: u32,
    pub granted_rights: u32,
    pub reserved: u32,
}

pub const MSG_DESKTOP_SESSION_V1_REVOKE_INSTANCE: u32 = 11;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RevokeInstance {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
}

pub const MSG_DESKTOP_SESSION_V1_REVOKE_INSTANCE_REPLY: u32 = 12;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RevokeInstanceReply {
    pub request_id: u64,
    pub status: u32,
    pub reserved: u32,
}
