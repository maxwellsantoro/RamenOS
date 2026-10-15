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

crate::impl_wire_payload!(CancelPreview {
    request_id: u64,
    session_id: u64,
    session_generation: u64,
    plan_id: u64,
});

pub const MSG_DESKTOP_SESSION_V1_CANCEL_PREVIEW_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CancelPreviewReply {
    pub request_id: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(CancelPreviewReply {
    request_id: u64,
    status: u32,
    reserved: u32,
});

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

crate::impl_wire_payload!(CloseInstance {
    request_id: u64,
    session_id: u64,
    session_generation: u64,
    instance_id: u64,
    instance_generation: u64,
});

pub const MSG_DESKTOP_SESSION_V1_CLOSE_INSTANCE_REPLY: u32 = 10;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CloseInstanceReply {
    pub request_id: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(CloseInstanceReply {
    request_id: u64,
    status: u32,
    reserved: u32,
});

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

crate::impl_wire_payload!(ConfirmLaunch {
    request_id: u64,
    session_id: u64,
    session_generation: u64,
    plan_id: u64,
    preview_revision: u64,
});

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

crate::impl_wire_payload!(ConfirmLaunchReply {
    request_id: u64,
    instance_id: u64,
    instance_generation: u64,
    observation_handle: u64,
    granted_rights: u32,
    status: u32,
    child_pid: u32,
    reserved: u32,
});

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

crate::impl_wire_payload!(GetStatus {
    request_id: u64,
    session_id: u64,
    session_generation: u64,
    instance_id: u64,
    instance_generation: u64,
});

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

crate::impl_wire_payload!(GetStatusReply {
    request_id: u64,
    instance_id: u64,
    instance_generation: u64,
    executable_hash: [u8; 32],
    state: u32,
    status: u32,
});

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

crate::impl_wire_payload!(InstanceBootstrap {
    session_id: u64,
    session_generation: u64,
    instance_id: u64,
    instance_generation: u64,
    observation_handle: u64,
    granted_rights: u32,
    reserved: u32,
});

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

crate::impl_wire_payload!(ObserveInstance {
    request_id: u64,
    session_id: u64,
    session_generation: u64,
    instance_id: u64,
    instance_generation: u64,
});

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

crate::impl_wire_payload!(ObserveInstanceReply {
    request_id: u64,
    instance_id: u64,
    instance_generation: u64,
    executable_hash: [u8; 32],
    state: u32,
    status: u32,
});

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

crate::impl_wire_payload!(PrepareLaunch {
    request_id: u64,
    session_id: u64,
    session_generation: u64,
    application_hash: [u8; 32],
    requested_rights: u32,
    reserved: u32,
});

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

crate::impl_wire_payload!(PrepareLaunchReply {
    request_id: u64,
    plan_id: u64,
    preview_revision: u64,
    preview_shm: u64,
    preview_len: u32,
    status: u32,
    granted_rights: u32,
    reserved: u32,
});

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

crate::impl_wire_payload!(PrepareRestart {
    request_id: u64,
    session_id: u64,
    session_generation: u64,
    instance_id: u64,
    instance_generation: u64,
});

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

crate::impl_wire_payload!(PrepareRestartReply {
    request_id: u64,
    plan_id: u64,
    preview_revision: u64,
    preview_shm: u64,
    preview_len: u32,
    status: u32,
    granted_rights: u32,
    reserved: u32,
});

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

crate::impl_wire_payload!(RevokeInstance {
    request_id: u64,
    session_id: u64,
    session_generation: u64,
    instance_id: u64,
    instance_generation: u64,
});

pub const MSG_DESKTOP_SESSION_V1_REVOKE_INSTANCE_REPLY: u32 = 12;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RevokeInstanceReply {
    pub request_id: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(RevokeInstanceReply {
    request_id: u64,
    status: u32,
    reserved: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_desktop_session_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<CancelPreview>()];
        let value = CancelPreview::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CancelPreview>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CancelPreview>()];
        let offset = core::mem::offset_of!(CancelPreview, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelPreview, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelPreview, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelPreview, plan_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CancelPreview initialized bytes");
        let again = CancelPreview::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(value.plan_id, again.plan_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CancelPreviewReply>()];
        let value = CancelPreviewReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CancelPreviewReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CancelPreviewReply>()];
        let offset = core::mem::offset_of!(CancelPreviewReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelPreviewReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CancelPreviewReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CancelPreviewReply initialized bytes");
        let again = CancelPreviewReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CloseInstance>()];
        let value = CloseInstance::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CloseInstance>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CloseInstance>()];
        let offset = core::mem::offset_of!(CloseInstance, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CloseInstance, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CloseInstance, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CloseInstance, instance_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CloseInstance, instance_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CloseInstance initialized bytes");
        let again = CloseInstance::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(value.instance_id, again.instance_id);
        assert_eq!(value.instance_generation, again.instance_generation);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CloseInstanceReply>()];
        let value = CloseInstanceReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CloseInstanceReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CloseInstanceReply>()];
        let offset = core::mem::offset_of!(CloseInstanceReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CloseInstanceReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CloseInstanceReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CloseInstanceReply initialized bytes");
        let again = CloseInstanceReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ConfirmLaunch>()];
        let value = ConfirmLaunch::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ConfirmLaunch>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ConfirmLaunch>()];
        let offset = core::mem::offset_of!(ConfirmLaunch, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ConfirmLaunch, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ConfirmLaunch, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ConfirmLaunch, plan_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ConfirmLaunch, preview_revision);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ConfirmLaunch initialized bytes");
        let again = ConfirmLaunch::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(value.plan_id, again.plan_id);
        assert_eq!(value.preview_revision, again.preview_revision);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ConfirmLaunchReply>()];
        let value = ConfirmLaunchReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ConfirmLaunchReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ConfirmLaunchReply>()];
        let offset = core::mem::offset_of!(ConfirmLaunchReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ConfirmLaunchReply, instance_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ConfirmLaunchReply, instance_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ConfirmLaunchReply, observation_handle);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ConfirmLaunchReply, granted_rights);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ConfirmLaunchReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ConfirmLaunchReply, child_pid);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ConfirmLaunchReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ConfirmLaunchReply initialized bytes");
        let again = ConfirmLaunchReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.instance_id, again.instance_id);
        assert_eq!(value.instance_generation, again.instance_generation);
        assert_eq!(value.observation_handle, again.observation_handle);
        assert_eq!(value.granted_rights, again.granted_rights);
        assert_eq!(value.status, again.status);
        assert_eq!(value.child_pid, again.child_pid);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GetStatus>()];
        let value = GetStatus::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetStatus>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetStatus>()];
        let offset = core::mem::offset_of!(GetStatus, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetStatus, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetStatus, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetStatus, instance_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetStatus, instance_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetStatus initialized bytes");
        let again = GetStatus::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(value.instance_id, again.instance_id);
        assert_eq!(value.instance_generation, again.instance_generation);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GetStatusReply>()];
        let value = GetStatusReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetStatusReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetStatusReply>()];
        let offset = core::mem::offset_of!(GetStatusReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetStatusReply, instance_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetStatusReply, instance_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetStatusReply, executable_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetStatusReply, state);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetStatusReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetStatusReply initialized bytes");
        let again = GetStatusReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.instance_id, again.instance_id);
        assert_eq!(value.instance_generation, again.instance_generation);
        assert_eq!(value.executable_hash, again.executable_hash);
        assert_eq!(value.state, again.state);
        assert_eq!(value.status, again.status);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<InstanceBootstrap>()];
        let value = InstanceBootstrap::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<InstanceBootstrap>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<InstanceBootstrap>()];
        let offset = core::mem::offset_of!(InstanceBootstrap, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(InstanceBootstrap, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(InstanceBootstrap, instance_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(InstanceBootstrap, instance_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(InstanceBootstrap, observation_handle);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(InstanceBootstrap, granted_rights);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(InstanceBootstrap, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "InstanceBootstrap initialized bytes");
        let again = InstanceBootstrap::decode(&output).unwrap();
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(value.instance_id, again.instance_id);
        assert_eq!(value.instance_generation, again.instance_generation);
        assert_eq!(value.observation_handle, again.observation_handle);
        assert_eq!(value.granted_rights, again.granted_rights);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ObserveInstance>()];
        let value = ObserveInstance::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ObserveInstance>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ObserveInstance>()];
        let offset = core::mem::offset_of!(ObserveInstance, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ObserveInstance, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ObserveInstance, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ObserveInstance, instance_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ObserveInstance, instance_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ObserveInstance initialized bytes");
        let again = ObserveInstance::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(value.instance_id, again.instance_id);
        assert_eq!(value.instance_generation, again.instance_generation);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ObserveInstanceReply>()];
        let value = ObserveInstanceReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ObserveInstanceReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ObserveInstanceReply>()];
        let offset = core::mem::offset_of!(ObserveInstanceReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ObserveInstanceReply, instance_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ObserveInstanceReply, instance_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ObserveInstanceReply, executable_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ObserveInstanceReply, state);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ObserveInstanceReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ObserveInstanceReply initialized bytes");
        let again = ObserveInstanceReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.instance_id, again.instance_id);
        assert_eq!(value.instance_generation, again.instance_generation);
        assert_eq!(value.executable_hash, again.executable_hash);
        assert_eq!(value.state, again.state);
        assert_eq!(value.status, again.status);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<PrepareLaunch>()];
        let value = PrepareLaunch::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<PrepareLaunch>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<PrepareLaunch>()];
        let offset = core::mem::offset_of!(PrepareLaunch, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareLaunch, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareLaunch, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareLaunch, application_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareLaunch, requested_rights);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareLaunch, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "PrepareLaunch initialized bytes");
        let again = PrepareLaunch::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(value.application_hash, again.application_hash);
        assert_eq!(value.requested_rights, again.requested_rights);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<PrepareLaunchReply>()];
        let value = PrepareLaunchReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<PrepareLaunchReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<PrepareLaunchReply>()];
        let offset = core::mem::offset_of!(PrepareLaunchReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareLaunchReply, plan_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareLaunchReply, preview_revision);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareLaunchReply, preview_shm);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareLaunchReply, preview_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareLaunchReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareLaunchReply, granted_rights);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareLaunchReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "PrepareLaunchReply initialized bytes");
        let again = PrepareLaunchReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.plan_id, again.plan_id);
        assert_eq!(value.preview_revision, again.preview_revision);
        assert_eq!(value.preview_shm, again.preview_shm);
        assert_eq!(value.preview_len, again.preview_len);
        assert_eq!(value.status, again.status);
        assert_eq!(value.granted_rights, again.granted_rights);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<PrepareRestart>()];
        let value = PrepareRestart::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<PrepareRestart>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<PrepareRestart>()];
        let offset = core::mem::offset_of!(PrepareRestart, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareRestart, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareRestart, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareRestart, instance_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareRestart, instance_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "PrepareRestart initialized bytes");
        let again = PrepareRestart::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(value.instance_id, again.instance_id);
        assert_eq!(value.instance_generation, again.instance_generation);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<PrepareRestartReply>()];
        let value = PrepareRestartReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<PrepareRestartReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<PrepareRestartReply>()];
        let offset = core::mem::offset_of!(PrepareRestartReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareRestartReply, plan_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareRestartReply, preview_revision);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareRestartReply, preview_shm);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareRestartReply, preview_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareRestartReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareRestartReply, granted_rights);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PrepareRestartReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "PrepareRestartReply initialized bytes");
        let again = PrepareRestartReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.plan_id, again.plan_id);
        assert_eq!(value.preview_revision, again.preview_revision);
        assert_eq!(value.preview_shm, again.preview_shm);
        assert_eq!(value.preview_len, again.preview_len);
        assert_eq!(value.status, again.status);
        assert_eq!(value.granted_rights, again.granted_rights);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<RevokeInstance>()];
        let value = RevokeInstance::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<RevokeInstance>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<RevokeInstance>()];
        let offset = core::mem::offset_of!(RevokeInstance, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RevokeInstance, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RevokeInstance, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RevokeInstance, instance_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RevokeInstance, instance_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "RevokeInstance initialized bytes");
        let again = RevokeInstance::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(value.instance_id, again.instance_id);
        assert_eq!(value.instance_generation, again.instance_generation);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<RevokeInstanceReply>()];
        let value = RevokeInstanceReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<RevokeInstanceReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<RevokeInstanceReply>()];
        let offset = core::mem::offset_of!(RevokeInstanceReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RevokeInstanceReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RevokeInstanceReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "RevokeInstanceReply initialized bytes");
        let again = RevokeInstanceReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
}
