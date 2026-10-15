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

crate::impl_wire_payload!(ExportDisplay {
    request_id: u64,
    domain_id: u64,
    display_cap_token_high: u64,
    display_cap_token_low: u64,
    width: u32,
    height: u32,
});

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

crate::impl_wire_payload!(ExportDisplayReply {
    request_id: u64,
    domain_id: u64,
    surface_id: u64,
    status: u32,
    stride: u32,
    format: u32,
    reserved: u32,
});

pub const MSG_GPU_QUARANTINE_V1_REPORT_SCANOUT: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReportScanout {
    pub request_id: u64,
    pub domain_id: u64,
    pub surface_id: u64,
    pub frame_seq: u64,
}

crate::impl_wire_payload!(ReportScanout {
    request_id: u64,
    domain_id: u64,
    surface_id: u64,
    frame_seq: u64,
});

pub const MSG_GPU_QUARANTINE_V1_REPORT_SCANOUT_REPLY: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReportScanoutReply {
    pub request_id: u64,
    pub acked_frame_seq: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(ReportScanoutReply {
    request_id: u64,
    acked_frame_seq: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_GPU_QUARANTINE_V1_START_QUARANTINE_DOMAIN: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StartQuarantineDomain {
    pub request_id: u64,
    pub domain_id: u64,
    pub restart_policy: u32,
    pub gpu_profile: u32,
}

crate::impl_wire_payload!(StartQuarantineDomain {
    request_id: u64,
    domain_id: u64,
    restart_policy: u32,
    gpu_profile: u32,
});

pub const MSG_GPU_QUARANTINE_V1_START_QUARANTINE_DOMAIN_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StartQuarantineDomainReply {
    pub request_id: u64,
    pub domain_id: u64,
    pub status: u32,
    pub generation: u32,
}

crate::impl_wire_payload!(StartQuarantineDomainReply {
    request_id: u64,
    domain_id: u64,
    status: u32,
    generation: u32,
});

pub const MSG_GPU_QUARANTINE_V1_STOP_QUARANTINE_DOMAIN: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StopQuarantineDomain {
    pub request_id: u64,
    pub domain_id: u64,
}

crate::impl_wire_payload!(StopQuarantineDomain {
    request_id: u64,
    domain_id: u64,
});

pub const MSG_GPU_QUARANTINE_V1_STOP_QUARANTINE_DOMAIN_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StopQuarantineDomainReply {
    pub request_id: u64,
    pub domain_id: u64,
    pub status: u32,
    pub generation: u32,
}

crate::impl_wire_payload!(StopQuarantineDomainReply {
    request_id: u64,
    domain_id: u64,
    status: u32,
    generation: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_gpu_quarantine_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<ExportDisplay>()];
        let value = ExportDisplay::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ExportDisplay>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ExportDisplay>()];
        let offset = core::mem::offset_of!(ExportDisplay, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ExportDisplay, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ExportDisplay, display_cap_token_high);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ExportDisplay, display_cap_token_low);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ExportDisplay, width);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ExportDisplay, height);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ExportDisplay initialized bytes");
        let again = ExportDisplay::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.display_cap_token_high, again.display_cap_token_high);
        assert_eq!(value.display_cap_token_low, again.display_cap_token_low);
        assert_eq!(value.width, again.width);
        assert_eq!(value.height, again.height);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ExportDisplayReply>()];
        let value = ExportDisplayReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ExportDisplayReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ExportDisplayReply>()];
        let offset = core::mem::offset_of!(ExportDisplayReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ExportDisplayReply, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ExportDisplayReply, surface_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ExportDisplayReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ExportDisplayReply, stride);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ExportDisplayReply, format);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ExportDisplayReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ExportDisplayReply initialized bytes");
        let again = ExportDisplayReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.surface_id, again.surface_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.stride, again.stride);
        assert_eq!(value.format, again.format);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ReportScanout>()];
        let value = ReportScanout::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReportScanout>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReportScanout>()];
        let offset = core::mem::offset_of!(ReportScanout, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReportScanout, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReportScanout, surface_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReportScanout, frame_seq);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReportScanout initialized bytes");
        let again = ReportScanout::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.surface_id, again.surface_id);
        assert_eq!(value.frame_seq, again.frame_seq);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ReportScanoutReply>()];
        let value = ReportScanoutReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReportScanoutReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReportScanoutReply>()];
        let offset = core::mem::offset_of!(ReportScanoutReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReportScanoutReply, acked_frame_seq);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReportScanoutReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReportScanoutReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReportScanoutReply initialized bytes");
        let again = ReportScanoutReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.acked_frame_seq, again.acked_frame_seq);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<StartQuarantineDomain>()];
        let value = StartQuarantineDomain::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<StartQuarantineDomain>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<StartQuarantineDomain>()];
        let offset = core::mem::offset_of!(StartQuarantineDomain, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StartQuarantineDomain, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StartQuarantineDomain, restart_policy);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StartQuarantineDomain, gpu_profile);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "StartQuarantineDomain initialized bytes");
        let again = StartQuarantineDomain::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.restart_policy, again.restart_policy);
        assert_eq!(value.gpu_profile, again.gpu_profile);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<StartQuarantineDomainReply>()];
        let value = StartQuarantineDomainReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<StartQuarantineDomainReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<StartQuarantineDomainReply>()];
        let offset = core::mem::offset_of!(StartQuarantineDomainReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StartQuarantineDomainReply, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StartQuarantineDomainReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StartQuarantineDomainReply, generation);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(
            output, expected,
            "StartQuarantineDomainReply initialized bytes"
        );
        let again = StartQuarantineDomainReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.generation, again.generation);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<StopQuarantineDomain>()];
        let value = StopQuarantineDomain::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<StopQuarantineDomain>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<StopQuarantineDomain>()];
        let offset = core::mem::offset_of!(StopQuarantineDomain, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StopQuarantineDomain, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "StopQuarantineDomain initialized bytes");
        let again = StopQuarantineDomain::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<StopQuarantineDomainReply>()];
        let value = StopQuarantineDomainReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<StopQuarantineDomainReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<StopQuarantineDomainReply>()];
        let offset = core::mem::offset_of!(StopQuarantineDomainReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StopQuarantineDomainReply, domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StopQuarantineDomainReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StopQuarantineDomainReply, generation);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(
            output, expected,
            "StopQuarantineDomainReply initialized bytes"
        );
        let again = StopQuarantineDomainReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.domain_id, again.domain_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.generation, again.generation);
    }
}
