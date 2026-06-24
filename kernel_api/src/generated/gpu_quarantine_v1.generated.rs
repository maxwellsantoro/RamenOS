// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/gpu_quarantine_v1.toml

// namespace = gpu.quarantine, version = 1

pub const GPU_QUARANTINE_V1_PROTOCOL_ID: u32 = 784;

pub const MSG_GPU_QUARANTINE_V1_EXPORT_DISPLAY: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ExportDisplay {
    pub request_id: u64,
    pub domain_id: u64,
    pub display_cap_token_high: u64,
    pub display_cap_token_low: u64,
    pub width: u32,
    pub height: u32,
}

pub const MSG_GPU_QUARANTINE_V1_EXPORT_DISPLAY_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ExportDisplayReply {
    pub request_id: u64,
    pub domain_id: u64,
    pub surface_id: u64,
    pub status: u32,
    pub stride: u32,
    pub format: u32,
    pub reserved: u32,
}

pub const MSG_GPU_QUARANTINE_V1_REPORT_SCANOUT: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReportScanout {
    pub request_id: u64,
    pub domain_id: u64,
    pub surface_id: u64,
    pub frame_seq: u64,
}

pub const MSG_GPU_QUARANTINE_V1_REPORT_SCANOUT_REPLY: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReportScanoutReply {
    pub request_id: u64,
    pub acked_frame_seq: u64,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_GPU_QUARANTINE_V1_START_QUARANTINE_DOMAIN: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StartQuarantineDomain {
    pub request_id: u64,
    pub domain_id: u64,
    pub restart_policy: u32,
    pub gpu_profile: u32,
}

pub const MSG_GPU_QUARANTINE_V1_START_QUARANTINE_DOMAIN_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StartQuarantineDomainReply {
    pub request_id: u64,
    pub domain_id: u64,
    pub status: u32,
    pub generation: u32,
}

pub const MSG_GPU_QUARANTINE_V1_STOP_QUARANTINE_DOMAIN: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StopQuarantineDomain {
    pub request_id: u64,
    pub domain_id: u64,
}

pub const MSG_GPU_QUARANTINE_V1_STOP_QUARANTINE_DOMAIN_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StopQuarantineDomainReply {
    pub request_id: u64,
    pub domain_id: u64,
    pub status: u32,
    pub generation: u32,
}
