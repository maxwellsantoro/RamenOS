// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/domain_manager_v1.toml

// namespace = domain.manager, version = 1

pub const DOMAIN_MANAGER_V1_PROTOCOL_ID: u32 = 768;

pub const MSG_DOMAIN_MANAGER_V1_GET_DOMAIN_GRANT_HANDLES: u32 = 15;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetDomainGrantHandles {
    pub request_id: u64,
    pub domain_id: u64,
}

pub const MSG_DOMAIN_MANAGER_V1_GET_DOMAIN_GRANT_HANDLES_REPLY: u32 = 16;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetDomainGrantHandlesReply {
    pub request_id: u64,
    pub domain_id: u64,
    pub status: u32,
    pub count: u32,
    pub entry0_export_id: u16,
    pub entry0_reserved: u16,
    pub entry0_handle: u64,
    pub entry1_export_id: u16,
    pub entry1_reserved: u16,
    pub entry1_handle: u64,
}

pub const MSG_DOMAIN_MANAGER_V1_GET_DOMAIN_STATUS: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetDomainStatus {
    pub request_id: u64,
    pub domain_id: u64,
}

pub const MSG_DOMAIN_MANAGER_V1_GET_DOMAIN_STATUS_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetDomainStatusReply {
    pub request_id: u64,
    pub domain_id: u64,
    pub status: u32,
    pub state: u32,
    pub generation: u32,
    pub restart_count: u32,
}

pub const MSG_DOMAIN_MANAGER_V1_GRANT_CAPABILITIES: u32 = 11;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GrantCapabilities {
    pub request_id: u64,
    pub domain_id: u64,
    pub content_id_hash: [u8; 32],
}

pub const MSG_DOMAIN_MANAGER_V1_GRANT_CAPABILITIES_REPLY: u32 = 12;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GrantCapabilitiesReply {
    pub request_id: u64,
    pub domain_id: u64,
    pub status: u32,
    pub handle_count: u32,
    pub reserved: u32,
    pub reserved2: u32,
}

pub const MSG_DOMAIN_MANAGER_V1_LIST_DOMAINS: u32 = 9;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ListDomains {
    pub request_id: u64,
}

pub const MSG_DOMAIN_MANAGER_V1_LIST_DOMAINS_REPLY: u32 = 10;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ListDomainsReply {
    pub request_id: u64,
    pub total_domains: u32,
    pub running_domains: u32,
    pub restarting_domains: u32,
    pub stopped_domains: u32,
}

pub const MSG_DOMAIN_MANAGER_V1_REPORT_EXIT: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReportExit {
    pub domain_id: u64,
    pub exit_code: u32,
    pub reason: u32,
}

pub const MSG_DOMAIN_MANAGER_V1_REPORT_EXIT_REPLY: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReportExitReply {
    pub domain_id: u64,
    pub action: u32,
    pub generation: u32,
    pub restart_count: u32,
}

pub const MSG_DOMAIN_MANAGER_V1_REVOKE_CAPABILITIES: u32 = 13;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RevokeCapabilities {
    pub request_id: u64,
    pub domain_id: u64,
}

pub const MSG_DOMAIN_MANAGER_V1_REVOKE_CAPABILITIES_REPLY: u32 = 14;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RevokeCapabilitiesReply {
    pub request_id: u64,
    pub domain_id: u64,
    pub status: u32,
    pub revoked_count: u32,
}

pub const MSG_DOMAIN_MANAGER_V1_START_DOMAIN: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StartDomain {
    pub request_id: u64,
    pub domain_id: u64,
    pub runner_kind: u32,
    pub restart_policy: u32,
}

pub const MSG_DOMAIN_MANAGER_V1_START_DOMAIN_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StartDomainReply {
    pub request_id: u64,
    pub domain_id: u64,
    pub status: u32,
    pub generation: u32,
}

pub const MSG_DOMAIN_MANAGER_V1_STOP_DOMAIN: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StopDomain {
    pub request_id: u64,
    pub domain_id: u64,
}

pub const MSG_DOMAIN_MANAGER_V1_STOP_DOMAIN_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StopDomainReply {
    pub request_id: u64,
    pub domain_id: u64,
    pub status: u32,
    pub generation: u32,
}
