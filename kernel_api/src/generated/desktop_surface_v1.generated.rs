// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/services/desktop_surface_v1.toml

// namespace = services.desktop_surface, version = 1

pub const DESKTOP_SURFACE_V1_PROTOCOL_ID: u32 = 833;

pub const MSG_DESKTOP_SURFACE_V1_ACQUIRE: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Acquire {
    pub request_id: u64,
    pub surface_id: u64,
    pub surface_generation: u64,
    pub buffer_index: u32,
    pub reserved: u32,
}

pub const MSG_DESKTOP_SURFACE_V1_ACQUIRE_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AcquireReply {
    pub request_id: u64,
    pub mapping_generation: u64,
    pub buffer_shm: u64,
    pub status: u32,
    pub byte_len: u32,
}

pub const MSG_DESKTOP_SURFACE_V1_CONSUME: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Consume {
    pub request_id: u64,
    pub surface_id: u64,
    pub surface_generation: u64,
    pub sequence: u64,
}

pub const MSG_DESKTOP_SURFACE_V1_CONSUME_REPLY: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ConsumeReply {
    pub request_id: u64,
    pub sequence: u64,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_DESKTOP_SURFACE_V1_CREATE: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Create {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub reserved: u32,
}

pub const MSG_DESKTOP_SURFACE_V1_CREATE_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CreateReply {
    pub request_id: u64,
    pub surface_id: u64,
    pub surface_generation: u64,
    pub buffer0_shm: u64,
    pub buffer1_shm: u64,
    pub status: u32,
    pub stride: u32,
    pub format: u32,
    pub reserved: u32,
}

pub const MSG_DESKTOP_SURFACE_V1_DESTROY: u32 = 9;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Destroy {
    pub request_id: u64,
    pub surface_id: u64,
    pub surface_generation: u64,
}

pub const MSG_DESKTOP_SURFACE_V1_DESTROY_REPLY: u32 = 10;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct DestroyReply {
    pub request_id: u64,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_DESKTOP_SURFACE_V1_PRESENT: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Present {
    pub request_id: u64,
    pub surface_id: u64,
    pub surface_generation: u64,
    pub mapping_generation: u64,
    pub sequence: u64,
    pub buffer_index: u32,
    pub reserved: u32,
}

pub const MSG_DESKTOP_SURFACE_V1_PRESENT_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PresentReply {
    pub request_id: u64,
    pub sequence: u64,
    pub status: u32,
    pub reserved: u32,
}
