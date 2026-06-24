// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/capsule_control_v0.toml

// namespace = capsule.control, version = 0

pub const CAPSULE_CONTROL_V0_PROTOCOL_ID: u32 = 512;

pub const MSG_CAPSULE_CONTROL_V0_HEALTH: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Health {
    pub session_id: u64,
}

pub const MSG_CAPSULE_CONTROL_V0_HEALTH_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct HealthReply {
    pub session_id: u64,
    pub status: u32,
    pub error_count: u32,
}

pub const MSG_CAPSULE_CONTROL_V0_HELLO: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Hello {
    pub capsule_id: u64,
    pub backend_caps: u32,
    pub version_major: u16,
    pub version_minor: u16,
}

pub const MSG_CAPSULE_CONTROL_V0_HELLO_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct HelloReply {
    pub session_id: u64,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_CAPSULE_CONTROL_V0_SHUTDOWN: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Shutdown {
    pub session_id: u64,
    pub reason: u32,
    pub reserved: u32,
}

pub const MSG_CAPSULE_CONTROL_V0_SHUTDOWN_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ShutdownReply {
    pub session_id: u64,
    pub status: u32,
    pub reserved: u32,
}
