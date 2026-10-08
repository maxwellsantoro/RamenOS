//! Strict initialized LE codecs for the registered editor v1 interfaces.
use crate::dev::Status;
use kernel_api::generated::{
    desktop_artifact_v1, desktop_editor_session_v1, desktop_focus_v1, desktop_surface_v1, input_v1,
};
use kernel_api::{
    cap::{Handle, HandleKind},
    ipc::Envelope,
};

pub trait EditorMessage: Sized {
    const PROTOCOL: u32;
    const KIND: u32;
    const LENGTH: usize;
    fn encode(&self, endpoint: Handle) -> Result<Envelope, Status>;
    fn decode(frame: &Envelope) -> Result<Self, Status>;
}

trait Field: Sized {
    const SIZE: usize;
    fn write(&self, bytes: &mut [u8]);
    fn read(bytes: &[u8]) -> Self;
}
impl Field for u32 {
    const SIZE: usize = 4;
    fn write(&self, bytes: &mut [u8]) {
        bytes.copy_from_slice(&self.to_le_bytes());
    }
    fn read(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().expect("fixed u32 field"))
    }
}
impl Field for u64 {
    const SIZE: usize = 8;
    fn write(&self, bytes: &mut [u8]) {
        bytes.copy_from_slice(&self.to_le_bytes());
    }
    fn read(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().expect("fixed u64 field"))
    }
}
impl Field for [u8; 32] {
    const SIZE: usize = 32;
    fn write(&self, bytes: &mut [u8]) {
        bytes.copy_from_slice(self);
    }
    fn read(bytes: &[u8]) -> Self {
        bytes.try_into().expect("fixed hash field")
    }
}
fn put<T: Field>(payload: &mut [u8; 64], cursor: &mut usize, value: &T) {
    value.write(&mut payload[*cursor..*cursor + T::SIZE]);
    *cursor += T::SIZE;
}
fn take<T: Field>(payload: &[u8; 64], cursor: &mut usize) -> T {
    let value = T::read(&payload[*cursor..*cursor + T::SIZE]);
    *cursor += T::SIZE;
    value
}
macro_rules! message {
    ($module:ident, $name:ident, $protocol:expr, $kind:expr, $length:expr, {$($field:ident),+}) => {
        impl EditorMessage for $module::$name {
            const PROTOCOL: u32 = $protocol;
            const KIND: u32 = $kind;
            const LENGTH: usize = $length;
            fn encode(&self, endpoint: Handle) -> Result<Envelope, Status> {
                let mut env = Envelope::empty(Self::PROTOCOL, Self::KIND);
                env.handle = endpoint;
                env.payload_len = Self::LENGTH as u32;
                let mut cursor = 0;
                $(put(&mut env.payload, &mut cursor, &self.$field);)+
                debug_assert_eq!(cursor, Self::LENGTH);
                validate_editor_envelope(&env)?;
                Ok(env)
            }
            fn decode(env: &Envelope) -> Result<Self, Status> {
                validate_editor_envelope(env)?;
                if env.protocol != Self::PROTOCOL || env.msg_type != Self::KIND { return Err(Status::Unsupported); }
                let mut cursor = 0;
                let value = Self { $($field: take(&env.payload, &mut cursor)),+ };
                debug_assert_eq!(cursor, Self::LENGTH);
                Ok(value)
            }
        }
    };
}

pub(super) fn layout(protocol: u32, kind: u32) -> Option<(usize, Option<usize>)> {
    match (protocol, kind) {
        (802, 1) => Some((32, None)),
        (802, 2) => Some((24, Some(20))),
        (802, 3) => Some((48, Some(44))),
        (802, 4) => Some((16, Some(12))),
        (832, 1) => Some((56, None)),
        (832, 2) => Some((32, Some(28))),
        (832, 3) => Some((48, None)),
        (832, 4) => Some((40, None)),
        (833, 1) => Some((56, Some(52))),
        (833, 2) => Some((56, Some(52))),
        (833, 3) => Some((32, Some(28))),
        (833, 4) => Some((32, None)),
        (833, 5) => Some((48, Some(44))),
        (833, 6) => Some((24, Some(20))),
        (833, 7) => Some((32, None)),
        (833, 8) => Some((24, Some(20))),
        (833, 9) => Some((24, None)),
        (833, 10) => Some((16, Some(12))),
        (352, 1) => Some((64, Some(60))),
        (352, 2) => Some((48, Some(44))),
        (352, 3) => Some((32, None)),
        (352, 4) => Some((16, Some(12))),
        (352, 5) => Some((40, None)),
        (352, 6) => Some((48, None)),
        (352, 7) => Some((40, None)),
        (352, 8) => Some((48, None)),
        (352, 9) => Some((40, None)),
        (352, 10) => Some((16, Some(12))),
        (352, 11) => Some((40, None)),
        (352, 12) => Some((16, Some(12))),
        (352, 13) => Some((40, None)),
        (352, 14) => Some((48, Some(44))),
        (352, 15) => Some((40, None)),
        (352, 16) => Some((32, Some(28))),
        (352, 17) => Some((48, None)),
        (368, 1) => Some((40, None)),
        (368, 2) => Some((40, None)),
        (368, 3) => Some((32, None)),
        (368, 4) => Some((24, Some(20))),
        (368, 5) => Some((56, Some(52))),
        (368, 6) => Some((40, None)),
        (368, 7) => Some((32, None)),
        (368, 8) => Some((40, None)),
        (368, 9) => Some((32, None)),
        (368, 10) => Some((32, Some(28))),
        _ => None,
    }
}
fn status_offset(protocol: u32, kind: u32) -> Option<usize> {
    match (protocol, kind) {
        (802, 2) => Some(16),
        (802, 4) => Some(8),
        (832, 2) => Some(24),
        (832, 4) => Some(36),
        (833, 2) => Some(40),
        (833, 4) => Some(24),
        (833, 6) => Some(16),
        (833, 8) => Some(16),
        (833, 10) => Some(8),
        (352, 2) => Some(36),
        (352, 4) => Some(8),
        (352, 6) => Some(44),
        (352, 8) => Some(44),
        (352, 10) => Some(8),
        (352, 12) => Some(8),
        (352, 14) => Some(36),
        (352, 16) => Some(24),
        (352, 17) => Some(44),
        (368, 2) => Some(36),
        (368, 4) => Some(16),
        (368, 6) => Some(32),
        (368, 8) => Some(32),
        (368, 10) => Some(24),
        _ => None,
    }
}

pub(super) fn validate_editor_envelope(env: &Envelope) -> Result<(), Status> {
    let (length, reserved) = layout(env.protocol, env.msg_type).ok_or(Status::Unsupported)?;
    if env.payload_len as usize != length
        || env.payload[length..].iter().any(|&v| v != 0)
        || reserved.is_some_and(|offset| env.payload[offset..offset + 4].iter().any(|&v| v != 0))
    {
        return Err(Status::Invalid);
    }
    if status_offset(env.protocol, env.msg_type).is_some_and(|offset| {
        u32::from_le_bytes(
            env.payload[offset..offset + 4]
                .try_into()
                .expect("fixed status"),
        ) > 11
    }) {
        return Err(Status::Invalid);
    }
    let handle = env.handle;
    if handle != Handle::INVALID
        && (handle.kind == HandleKind::Invalid
            || handle.index == 0
            || handle.index > u16::MAX as u32
            || handle.generation == 0
            || handle.generation > u32::MAX as u64)
    {
        return Err(Status::Invalid);
    }
    Ok(())
}

pub fn encode_envelope_wire(env: &Envelope) -> Result<[u8; 88], Status> {
    validate_editor_envelope(env)?;
    let mut wire = [0; 88];
    wire[..4].copy_from_slice(&env.protocol.to_le_bytes());
    wire[4..8].copy_from_slice(&env.msg_type.to_le_bytes());
    wire[8..16].copy_from_slice(&env.handle.pack().to_le_bytes());
    wire[16..20].copy_from_slice(&env.payload_len.to_le_bytes());
    wire[20..84].copy_from_slice(&env.payload);
    Ok(wire)
}
pub fn decode_envelope_wire(wire: &[u8]) -> Result<Envelope, Status> {
    if wire.len() != 88 {
        return Err(Status::Invalid);
    }
    let packed = u64::from_le_bytes(wire[8..16].try_into().expect("fixed handle"));
    if packed & 0x00ff_0000_0000_0000 != 0 || packed >> 56 > 3 || wire[84..].iter().any(|&v| v != 0)
    {
        return Err(Status::Invalid);
    }
    let mut env = Envelope::empty(
        u32::from_le_bytes(wire[..4].try_into().expect("fixed protocol")),
        u32::from_le_bytes(wire[4..8].try_into().expect("fixed kind")),
    );
    env.handle = Handle::unpack(packed);
    env.payload_len = u32::from_le_bytes(wire[16..20].try_into().expect("fixed length"));
    env.payload.copy_from_slice(&wire[20..84]);
    if env.handle.pack() != packed {
        return Err(Status::Invalid);
    }
    validate_editor_envelope(&env)?;
    Ok(env)
}

message!(input_v1, Attach, 802, 1, 32, {request_id, session_id, session_generation, device_generation});
message!(input_v1, AttachReply, 802, 2, 24, {request_id, queue_generation, status, reserved});
message!(input_v1, ProduceKey, 802, 3, 48, {session_id, session_generation, queue_generation, sequence, usage, phase, modifiers, reserved});
message!(input_v1, ProduceKeyReply, 802, 4, 16, {sequence, status, reserved});
message!(desktop_focus_v1, Assign, 832, 1, 56, {request_id, session_id, session_generation, instance_id, instance_generation, surface_id, surface_generation});
message!(desktop_focus_v1, AssignReply, 832, 2, 32, {request_id, focus_epoch, service_epoch, status, reserved});
message!(desktop_focus_v1, PollKeys, 832, 3, 48, {request_id, session_id, session_generation, instance_id, instance_generation, focus_epoch});
message!(desktop_focus_v1, PollKeysReply, 832, 4, 40, {request_id, sequence, focus_epoch, usage, phase, modifiers, status});
message!(desktop_surface_v1, Create, 833, 1, 56, {request_id, session_id, session_generation, instance_id, instance_generation, width, height, format, reserved});
message!(desktop_surface_v1, CreateReply, 833, 2, 56, {request_id, surface_id, surface_generation, buffer0_shm, buffer1_shm, status, stride, format, reserved});
message!(desktop_surface_v1, Acquire, 833, 3, 32, {request_id, surface_id, surface_generation, buffer_index, reserved});
message!(desktop_surface_v1, AcquireReply, 833, 4, 32, {request_id, mapping_generation, buffer_shm, status, byte_len});
message!(desktop_surface_v1, Present, 833, 5, 48, {request_id, surface_id, surface_generation, mapping_generation, sequence, buffer_index, reserved});
message!(desktop_surface_v1, PresentReply, 833, 6, 24, {request_id, sequence, status, reserved});
message!(desktop_surface_v1, Consume, 833, 7, 32, {request_id, surface_id, surface_generation, sequence});
message!(desktop_surface_v1, ConsumeReply, 833, 8, 24, {request_id, sequence, status, reserved});
message!(desktop_surface_v1, Destroy, 833, 9, 24, {request_id, surface_id, surface_generation});
message!(desktop_surface_v1, DestroyReply, 833, 10, 16, {request_id, status, reserved});
message!(desktop_editor_session_v1, PrepareLaunch, 352, 1, 64, {request_id, session_id, session_generation, application_hash, requested_rights, reserved});
message!(desktop_editor_session_v1, PrepareLaunchReply, 352, 2, 48, {request_id, plan_id, preview_revision, preview_shm, preview_len, status, granted_rights, reserved});
message!(desktop_editor_session_v1, CancelPreview, 352, 3, 32, {request_id, session_id, session_generation, plan_id});
message!(desktop_editor_session_v1, CancelPreviewReply, 352, 4, 16, {request_id, status, reserved});
message!(desktop_editor_session_v1, ConfirmLaunch, 352, 5, 40, {request_id, session_id, session_generation, plan_id, preview_revision});
message!(desktop_editor_session_v1, ConfirmLaunchReply, 352, 6, 48, {request_id, session_generation, instance_id, instance_generation, grants_shm, grants_len, status});
message!(desktop_editor_session_v1, GetStatus, 352, 7, 40, {request_id, session_id, session_generation, instance_id, instance_generation});
message!(desktop_editor_session_v1, GetStatusReply, 352, 8, 48, {request_id, instance_id, instance_generation, focus_epoch, service_epoch, state, status});
message!(desktop_editor_session_v1, CloseInstance, 352, 9, 40, {request_id, session_id, session_generation, instance_id, instance_generation});
message!(desktop_editor_session_v1, CloseInstanceReply, 352, 10, 16, {request_id, status, reserved});
message!(desktop_editor_session_v1, RevokeInstance, 352, 11, 40, {request_id, session_id, session_generation, instance_id, instance_generation});
message!(desktop_editor_session_v1, RevokeInstanceReply, 352, 12, 16, {request_id, status, reserved});
message!(desktop_editor_session_v1, PrepareRestart, 352, 13, 40, {request_id, session_id, session_generation, instance_id, instance_generation});
message!(desktop_editor_session_v1, PrepareRestartReply, 352, 14, 48, {request_id, plan_id, preview_revision, preview_shm, preview_len, status, granted_rights, reserved});
message!(desktop_editor_session_v1, ObserveFocus, 352, 15, 40, {request_id, session_id, session_generation, instance_id, instance_generation});
message!(desktop_editor_session_v1, ObserveFocusReply, 352, 16, 32, {request_id, focus_epoch, service_epoch, status, reserved});
message!(desktop_editor_session_v1, InstanceBootstrap, 352, 17, 48, {session_id, session_generation, instance_id, instance_generation, grants_shm, grants_len, status});
message!(desktop_artifact_v1, ReadSelected, 368, 1, 40, {request_id, session_id, session_generation, instance_id, instance_generation});
message!(desktop_artifact_v1, ReadSelectedReply, 368, 2, 40, {request_id, revision, data_shm, object_generation, byte_len, status});
message!(desktop_artifact_v1, AllocateSaveId, 368, 3, 32, {request_id, session_id, session_generation, expected_revision});
message!(desktop_artifact_v1, AllocateSaveIdReply, 368, 4, 24, {request_id, operation_id, status, reserved});
message!(desktop_artifact_v1, Commit, 368, 5, 56, {request_id, session_id, session_generation, expected_revision, operation_id, source_shm, byte_len, reserved});
message!(desktop_artifact_v1, CommitReply, 368, 6, 40, {request_id, operation_id, revision, receipt_shm, status, receipt_len});
message!(desktop_artifact_v1, SaveStatus, 368, 7, 32, {request_id, session_id, session_generation, operation_id});
message!(desktop_artifact_v1, SaveStatusReply, 368, 8, 40, {request_id, operation_id, revision, receipt_shm, status, receipt_len});
message!(desktop_artifact_v1, ObserveAllocation, 368, 9, 32, {request_id, session_id, session_generation, original_allocate_request_id});
message!(desktop_artifact_v1, ObserveAllocationReply, 368, 10, 32, {request_id, original_allocate_request_id, operation_id, status, reserved});
