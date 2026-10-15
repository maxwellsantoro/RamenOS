// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/portals/desktop_artifact_v1.toml

// namespace = portal.desktop_artifact, version = 1

pub const DESKTOP_ARTIFACT_V1_PROTOCOL_ID: u32 = 368;

pub const MSG_DESKTOP_ARTIFACT_V1_ALLOCATE_SAVE_ID: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AllocateSaveId {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub expected_revision: u64,
}

crate::impl_wire_payload!(AllocateSaveId {
    request_id: u64,
    session_id: u64,
    session_generation: u64,
    expected_revision: u64,
});

pub const MSG_DESKTOP_ARTIFACT_V1_ALLOCATE_SAVE_ID_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AllocateSaveIdReply {
    pub request_id: u64,
    pub operation_id: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(AllocateSaveIdReply {
    request_id: u64,
    operation_id: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_DESKTOP_ARTIFACT_V1_COMMIT: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Commit {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub expected_revision: u64,
    pub operation_id: u64,
    pub source_shm: u64,
    pub byte_len: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(Commit {
    request_id: u64,
    session_id: u64,
    session_generation: u64,
    expected_revision: u64,
    operation_id: u64,
    source_shm: u64,
    byte_len: u32,
    reserved: u32,
});

pub const MSG_DESKTOP_ARTIFACT_V1_COMMIT_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CommitReply {
    pub request_id: u64,
    pub operation_id: u64,
    pub revision: u64,
    pub receipt_shm: u64,
    pub status: u32,
    pub receipt_len: u32,
}

crate::impl_wire_payload!(CommitReply {
    request_id: u64,
    operation_id: u64,
    revision: u64,
    receipt_shm: u64,
    status: u32,
    receipt_len: u32,
});

pub const MSG_DESKTOP_ARTIFACT_V1_OBSERVE_ALLOCATION: u32 = 9;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ObserveAllocation {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub original_allocate_request_id: u64,
}

crate::impl_wire_payload!(ObserveAllocation {
    request_id: u64,
    session_id: u64,
    session_generation: u64,
    original_allocate_request_id: u64,
});

pub const MSG_DESKTOP_ARTIFACT_V1_OBSERVE_ALLOCATION_REPLY: u32 = 10;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ObserveAllocationReply {
    pub request_id: u64,
    pub original_allocate_request_id: u64,
    pub operation_id: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(ObserveAllocationReply {
    request_id: u64,
    original_allocate_request_id: u64,
    operation_id: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_DESKTOP_ARTIFACT_V1_READ_SELECTED: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadSelected {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
}

crate::impl_wire_payload!(ReadSelected {
    request_id: u64,
    session_id: u64,
    session_generation: u64,
    instance_id: u64,
    instance_generation: u64,
});

pub const MSG_DESKTOP_ARTIFACT_V1_READ_SELECTED_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadSelectedReply {
    pub request_id: u64,
    pub revision: u64,
    pub data_shm: u64,
    pub object_generation: u64,
    pub byte_len: u32,
    pub status: u32,
}

crate::impl_wire_payload!(ReadSelectedReply {
    request_id: u64,
    revision: u64,
    data_shm: u64,
    object_generation: u64,
    byte_len: u32,
    status: u32,
});

pub const MSG_DESKTOP_ARTIFACT_V1_SAVE_STATUS: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SaveStatus {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub operation_id: u64,
}

crate::impl_wire_payload!(SaveStatus {
    request_id: u64,
    session_id: u64,
    session_generation: u64,
    operation_id: u64,
});

pub const MSG_DESKTOP_ARTIFACT_V1_SAVE_STATUS_REPLY: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SaveStatusReply {
    pub request_id: u64,
    pub operation_id: u64,
    pub revision: u64,
    pub receipt_shm: u64,
    pub status: u32,
    pub receipt_len: u32,
}

crate::impl_wire_payload!(SaveStatusReply {
    request_id: u64,
    operation_id: u64,
    revision: u64,
    receipt_shm: u64,
    status: u32,
    receipt_len: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_desktop_artifact_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<AllocateSaveId>()];
        let value = AllocateSaveId::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<AllocateSaveId>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<AllocateSaveId>()];
        let offset = core::mem::offset_of!(AllocateSaveId, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AllocateSaveId, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AllocateSaveId, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AllocateSaveId, expected_revision);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "AllocateSaveId initialized bytes");
        let again = AllocateSaveId::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(value.expected_revision, again.expected_revision);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<AllocateSaveIdReply>()];
        let value = AllocateSaveIdReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<AllocateSaveIdReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<AllocateSaveIdReply>()];
        let offset = core::mem::offset_of!(AllocateSaveIdReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AllocateSaveIdReply, operation_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AllocateSaveIdReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AllocateSaveIdReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "AllocateSaveIdReply initialized bytes");
        let again = AllocateSaveIdReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.operation_id, again.operation_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<Commit>()];
        let value = Commit::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<Commit>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<Commit>()];
        let offset = core::mem::offset_of!(Commit, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Commit, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Commit, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Commit, expected_revision);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Commit, operation_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Commit, source_shm);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Commit, byte_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Commit, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "Commit initialized bytes");
        let again = Commit::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(value.expected_revision, again.expected_revision);
        assert_eq!(value.operation_id, again.operation_id);
        assert_eq!(value.source_shm, again.source_shm);
        assert_eq!(value.byte_len, again.byte_len);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CommitReply>()];
        let value = CommitReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CommitReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CommitReply>()];
        let offset = core::mem::offset_of!(CommitReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitReply, operation_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitReply, revision);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitReply, receipt_shm);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitReply, receipt_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CommitReply initialized bytes");
        let again = CommitReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.operation_id, again.operation_id);
        assert_eq!(value.revision, again.revision);
        assert_eq!(value.receipt_shm, again.receipt_shm);
        assert_eq!(value.status, again.status);
        assert_eq!(value.receipt_len, again.receipt_len);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ObserveAllocation>()];
        let value = ObserveAllocation::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ObserveAllocation>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ObserveAllocation>()];
        let offset = core::mem::offset_of!(ObserveAllocation, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ObserveAllocation, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ObserveAllocation, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ObserveAllocation, original_allocate_request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ObserveAllocation initialized bytes");
        let again = ObserveAllocation::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(
            value.original_allocate_request_id,
            again.original_allocate_request_id
        );
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ObserveAllocationReply>()];
        let value = ObserveAllocationReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ObserveAllocationReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ObserveAllocationReply>()];
        let offset = core::mem::offset_of!(ObserveAllocationReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ObserveAllocationReply, original_allocate_request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ObserveAllocationReply, operation_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ObserveAllocationReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ObserveAllocationReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ObserveAllocationReply initialized bytes");
        let again = ObserveAllocationReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(
            value.original_allocate_request_id,
            again.original_allocate_request_id
        );
        assert_eq!(value.operation_id, again.operation_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ReadSelected>()];
        let value = ReadSelected::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReadSelected>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReadSelected>()];
        let offset = core::mem::offset_of!(ReadSelected, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadSelected, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadSelected, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadSelected, instance_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadSelected, instance_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReadSelected initialized bytes");
        let again = ReadSelected::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(value.instance_id, again.instance_id);
        assert_eq!(value.instance_generation, again.instance_generation);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ReadSelectedReply>()];
        let value = ReadSelectedReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReadSelectedReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReadSelectedReply>()];
        let offset = core::mem::offset_of!(ReadSelectedReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadSelectedReply, revision);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadSelectedReply, data_shm);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadSelectedReply, object_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadSelectedReply, byte_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadSelectedReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReadSelectedReply initialized bytes");
        let again = ReadSelectedReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.revision, again.revision);
        assert_eq!(value.data_shm, again.data_shm);
        assert_eq!(value.object_generation, again.object_generation);
        assert_eq!(value.byte_len, again.byte_len);
        assert_eq!(value.status, again.status);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<SaveStatus>()];
        let value = SaveStatus::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<SaveStatus>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<SaveStatus>()];
        let offset = core::mem::offset_of!(SaveStatus, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SaveStatus, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SaveStatus, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SaveStatus, operation_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "SaveStatus initialized bytes");
        let again = SaveStatus::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(value.operation_id, again.operation_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<SaveStatusReply>()];
        let value = SaveStatusReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<SaveStatusReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<SaveStatusReply>()];
        let offset = core::mem::offset_of!(SaveStatusReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SaveStatusReply, operation_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SaveStatusReply, revision);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SaveStatusReply, receipt_shm);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SaveStatusReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SaveStatusReply, receipt_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "SaveStatusReply initialized bytes");
        let again = SaveStatusReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.operation_id, again.operation_id);
        assert_eq!(value.revision, again.revision);
        assert_eq!(value.receipt_shm, again.receipt_shm);
        assert_eq!(value.status, again.status);
        assert_eq!(value.receipt_len, again.receipt_len);
    }
}
