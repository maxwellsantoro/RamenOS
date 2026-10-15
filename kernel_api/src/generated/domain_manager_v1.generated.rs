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

crate::impl_wire_payload!(GetDomainGrantHandles {
    request_id: u64,
    domain_id: u64,
});

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

crate::impl_wire_payload!(GetDomainGrantHandlesReply {
    request_id: u64,
    domain_id: u64,
    status: u32,
    count: u32,
    entry0_export_id: u16,
    entry0_reserved: u16,
    entry0_handle: u64,
    entry1_export_id: u16,
    entry1_reserved: u16,
    entry1_handle: u64,
});

pub const MSG_DOMAIN_MANAGER_V1_GET_DOMAIN_STATUS: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetDomainStatus {
    pub request_id: u64,
    pub domain_id: u64,
}

crate::impl_wire_payload!(GetDomainStatus {
    request_id: u64,
    domain_id: u64,
});

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

crate::impl_wire_payload!(GetDomainStatusReply {
    request_id: u64,
    domain_id: u64,
    status: u32,
    state: u32,
    generation: u32,
    restart_count: u32,
});

pub const MSG_DOMAIN_MANAGER_V1_GRANT_CAPABILITIES: u32 = 11;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GrantCapabilities {
    pub request_id: u64,
    pub domain_id: u64,
    pub content_id_hash: [u8; 32],
}

crate::impl_wire_payload!(GrantCapabilities {
    request_id: u64,
    domain_id: u64,
    content_id_hash: [u8; 32],
});

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

crate::impl_wire_payload!(GrantCapabilitiesReply {
    request_id: u64,
    domain_id: u64,
    status: u32,
    handle_count: u32,
    reserved: u32,
    reserved2: u32,
});

pub const MSG_DOMAIN_MANAGER_V1_LIST_DOMAINS: u32 = 9;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ListDomains {
    pub request_id: u64,
}

crate::impl_wire_payload!(ListDomains { request_id: u64 });

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

crate::impl_wire_payload!(ListDomainsReply {
    request_id: u64,
    total_domains: u32,
    running_domains: u32,
    restarting_domains: u32,
    stopped_domains: u32,
});

pub const MSG_DOMAIN_MANAGER_V1_REPORT_EXIT: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReportExit {
    pub domain_id: u64,
    pub exit_code: u32,
    pub reason: u32,
}

crate::impl_wire_payload!(ReportExit {
    domain_id: u64,
    exit_code: u32,
    reason: u32,
});

pub const MSG_DOMAIN_MANAGER_V1_REPORT_EXIT_REPLY: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReportExitReply {
    pub domain_id: u64,
    pub action: u32,
    pub generation: u32,
    pub restart_count: u32,
}

crate::impl_wire_payload!(ReportExitReply {
    domain_id: u64,
    action: u32,
    generation: u32,
    restart_count: u32,
});

pub const MSG_DOMAIN_MANAGER_V1_REVOKE_CAPABILITIES: u32 = 13;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RevokeCapabilities {
    pub request_id: u64,
    pub domain_id: u64,
}

crate::impl_wire_payload!(RevokeCapabilities {
    request_id: u64,
    domain_id: u64,
});

pub const MSG_DOMAIN_MANAGER_V1_REVOKE_CAPABILITIES_REPLY: u32 = 14;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RevokeCapabilitiesReply {
    pub request_id: u64,
    pub domain_id: u64,
    pub status: u32,
    pub revoked_count: u32,
}

crate::impl_wire_payload!(RevokeCapabilitiesReply {
    request_id: u64,
    domain_id: u64,
    status: u32,
    revoked_count: u32,
});

pub const MSG_DOMAIN_MANAGER_V1_START_DOMAIN: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StartDomain {
    pub request_id: u64,
    pub domain_id: u64,
    pub runner_kind: u32,
    pub restart_policy: u32,
}

crate::impl_wire_payload!(StartDomain {
    request_id: u64,
    domain_id: u64,
    runner_kind: u32,
    restart_policy: u32,
});

pub const MSG_DOMAIN_MANAGER_V1_START_DOMAIN_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StartDomainReply {
    pub request_id: u64,
    pub domain_id: u64,
    pub status: u32,
    pub generation: u32,
}

crate::impl_wire_payload!(StartDomainReply {
    request_id: u64,
    domain_id: u64,
    status: u32,
    generation: u32,
});

pub const MSG_DOMAIN_MANAGER_V1_STOP_DOMAIN: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StopDomain {
    pub request_id: u64,
    pub domain_id: u64,
}

crate::impl_wire_payload!(StopDomain {
    request_id: u64,
    domain_id: u64,
});

pub const MSG_DOMAIN_MANAGER_V1_STOP_DOMAIN_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StopDomainReply {
    pub request_id: u64,
    pub domain_id: u64,
    pub status: u32,
    pub generation: u32,
}

crate::impl_wire_payload!(StopDomainReply {
    request_id: u64,
    domain_id: u64,
    status: u32,
    generation: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_domain_manager_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<GetDomainGrantHandles>()];
        let value = GetDomainGrantHandles::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetDomainGrantHandles>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetDomainGrantHandles>()];
        let offset = core::mem::offset_of!(GetDomainGrantHandles, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetDomainGrantHandles, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetDomainGrantHandles initialized bytes");
        let again = GetDomainGrantHandles::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GetDomainGrantHandlesReply>()];
        let value = GetDomainGrantHandlesReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetDomainGrantHandlesReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetDomainGrantHandlesReply>()];
        let offset = core::mem::offset_of!(GetDomainGrantHandlesReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetDomainGrantHandlesReply, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetDomainGrantHandlesReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetDomainGrantHandlesReply, count);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetDomainGrantHandlesReply, entry0_export_id);
        expected[offset..offset + <u16 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetDomainGrantHandlesReply, entry0_reserved);
        expected[offset..offset + <u16 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetDomainGrantHandlesReply, entry0_handle);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetDomainGrantHandlesReply, entry1_export_id);
        expected[offset..offset + <u16 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetDomainGrantHandlesReply, entry1_reserved);
        expected[offset..offset + <u16 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetDomainGrantHandlesReply, entry1_handle);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(
            output, expected,
            "GetDomainGrantHandlesReply initialized bytes"
        );
        let again = GetDomainGrantHandlesReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.count, again.count);
        assert_eq!(value.entry0_export_id, again.entry0_export_id);
        assert_eq!(value.entry0_reserved, again.entry0_reserved);
        assert_eq!(value.entry0_handle, again.entry0_handle);
        assert_eq!(value.entry1_export_id, again.entry1_export_id);
        assert_eq!(value.entry1_reserved, again.entry1_reserved);
        assert_eq!(value.entry1_handle, again.entry1_handle);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GetDomainStatus>()];
        let value = GetDomainStatus::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetDomainStatus>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetDomainStatus>()];
        let offset = core::mem::offset_of!(GetDomainStatus, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetDomainStatus, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetDomainStatus initialized bytes");
        let again = GetDomainStatus::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GetDomainStatusReply>()];
        let value = GetDomainStatusReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetDomainStatusReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetDomainStatusReply>()];
        let offset = core::mem::offset_of!(GetDomainStatusReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetDomainStatusReply, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetDomainStatusReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetDomainStatusReply, state);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetDomainStatusReply, generation);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetDomainStatusReply, restart_count);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetDomainStatusReply initialized bytes");
        let again = GetDomainStatusReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.state, again.state);
        assert_eq!(value.generation, again.generation);
        assert_eq!(value.restart_count, again.restart_count);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GrantCapabilities>()];
        let value = GrantCapabilities::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GrantCapabilities>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GrantCapabilities>()];
        let offset = core::mem::offset_of!(GrantCapabilities, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GrantCapabilities, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GrantCapabilities, content_id_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GrantCapabilities initialized bytes");
        let again = GrantCapabilities::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.content_id_hash, again.content_id_hash);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GrantCapabilitiesReply>()];
        let value = GrantCapabilitiesReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GrantCapabilitiesReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GrantCapabilitiesReply>()];
        let offset = core::mem::offset_of!(GrantCapabilitiesReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GrantCapabilitiesReply, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GrantCapabilitiesReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GrantCapabilitiesReply, handle_count);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GrantCapabilitiesReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GrantCapabilitiesReply, reserved2);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GrantCapabilitiesReply initialized bytes");
        let again = GrantCapabilitiesReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.handle_count, again.handle_count);
        assert_eq!(value.reserved, again.reserved);
        assert_eq!(value.reserved2, again.reserved2);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ListDomains>()];
        let value = ListDomains::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ListDomains>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ListDomains>()];
        let offset = core::mem::offset_of!(ListDomains, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ListDomains initialized bytes");
        let again = ListDomains::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ListDomainsReply>()];
        let value = ListDomainsReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ListDomainsReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ListDomainsReply>()];
        let offset = core::mem::offset_of!(ListDomainsReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ListDomainsReply, total_domains);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ListDomainsReply, running_domains);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ListDomainsReply, restarting_domains);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ListDomainsReply, stopped_domains);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ListDomainsReply initialized bytes");
        let again = ListDomainsReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.total_domains, again.total_domains);
        assert_eq!(value.running_domains, again.running_domains);
        assert_eq!(value.restarting_domains, again.restarting_domains);
        assert_eq!(value.stopped_domains, again.stopped_domains);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ReportExit>()];
        let value = ReportExit::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReportExit>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReportExit>()];
        let offset = core::mem::offset_of!(ReportExit, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReportExit, exit_code);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReportExit, reason);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReportExit initialized bytes");
        let again = ReportExit::decode(&output).unwrap();
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.exit_code, again.exit_code);
        assert_eq!(value.reason, again.reason);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ReportExitReply>()];
        let value = ReportExitReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReportExitReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReportExitReply>()];
        let offset = core::mem::offset_of!(ReportExitReply, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReportExitReply, action);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReportExitReply, generation);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReportExitReply, restart_count);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReportExitReply initialized bytes");
        let again = ReportExitReply::decode(&output).unwrap();
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.action, again.action);
        assert_eq!(value.generation, again.generation);
        assert_eq!(value.restart_count, again.restart_count);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<RevokeCapabilities>()];
        let value = RevokeCapabilities::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<RevokeCapabilities>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<RevokeCapabilities>()];
        let offset = core::mem::offset_of!(RevokeCapabilities, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RevokeCapabilities, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "RevokeCapabilities initialized bytes");
        let again = RevokeCapabilities::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<RevokeCapabilitiesReply>()];
        let value = RevokeCapabilitiesReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<RevokeCapabilitiesReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<RevokeCapabilitiesReply>()];
        let offset = core::mem::offset_of!(RevokeCapabilitiesReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RevokeCapabilitiesReply, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RevokeCapabilitiesReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RevokeCapabilitiesReply, revoked_count);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(
            output, expected,
            "RevokeCapabilitiesReply initialized bytes"
        );
        let again = RevokeCapabilitiesReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.revoked_count, again.revoked_count);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<StartDomain>()];
        let value = StartDomain::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<StartDomain>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<StartDomain>()];
        let offset = core::mem::offset_of!(StartDomain, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StartDomain, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StartDomain, runner_kind);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StartDomain, restart_policy);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "StartDomain initialized bytes");
        let again = StartDomain::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.runner_kind, again.runner_kind);
        assert_eq!(value.restart_policy, again.restart_policy);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<StartDomainReply>()];
        let value = StartDomainReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<StartDomainReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<StartDomainReply>()];
        let offset = core::mem::offset_of!(StartDomainReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StartDomainReply, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StartDomainReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StartDomainReply, generation);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "StartDomainReply initialized bytes");
        let again = StartDomainReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.generation, again.generation);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<StopDomain>()];
        let value = StopDomain::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<StopDomain>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<StopDomain>()];
        let offset = core::mem::offset_of!(StopDomain, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StopDomain, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "StopDomain initialized bytes");
        let again = StopDomain::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<StopDomainReply>()];
        let value = StopDomainReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<StopDomainReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<StopDomainReply>()];
        let offset = core::mem::offset_of!(StopDomainReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StopDomainReply, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StopDomainReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StopDomainReply, generation);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "StopDomainReply initialized bytes");
        let again = StopDomainReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.generation, again.generation);
    }
}
