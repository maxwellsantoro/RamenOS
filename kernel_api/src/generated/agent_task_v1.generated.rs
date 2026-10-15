// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/agent_task_v1.toml

// namespace = harness.agent_task, version = 1

pub const AGENT_TASK_V1_PROTOCOL_ID: u32 = 14;

pub const MSG_AGENT_TASK_V1_COMMIT_CANDIDATE: u32 = 9;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CommitCandidate {
    pub task_cap: u64,
    pub request_id: u64,
    pub candidate_cap: u64,
    pub expected_revision: u64,
    pub expected_content_id_hash: [u8; 32],
}

crate::impl_wire_payload!(CommitCandidate {
    task_cap: u64,
    request_id: u64,
    candidate_cap: u64,
    expected_revision: u64,
    expected_content_id_hash: [u8; 32],
});

pub const MSG_AGENT_TASK_V1_COMMIT_CANDIDATE_REPLY: u32 = 10;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CommitCandidateReply {
    pub request_id: u64,
    pub receipt_cap: u64,
    pub revision: u64,
    pub content_id_hash: [u8; 32],
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(CommitCandidateReply {
    request_id: u64,
    receipt_cap: u64,
    revision: u64,
    content_id_hash: [u8; 32],
    status: u32,
    reserved: u32,
});

pub const MSG_AGENT_TASK_V1_GET_RECEIPT: u32 = 11;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetReceipt {
    pub task_cap: u64,
    pub request_id: u64,
    pub commit_request_id: u64,
}

crate::impl_wire_payload!(GetReceipt {
    task_cap: u64,
    request_id: u64,
    commit_request_id: u64,
});

pub const MSG_AGENT_TASK_V1_GET_RECEIPT_REPLY: u32 = 12;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetReceiptReply {
    pub request_id: u64,
    pub commit_request_id: u64,
    pub revision: u64,
    pub content_id_hash: [u8; 32],
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(GetReceiptReply {
    request_id: u64,
    commit_request_id: u64,
    revision: u64,
    content_id_hash: [u8; 32],
    status: u32,
    reserved: u32,
});

pub const MSG_AGENT_TASK_V1_GET_TASK_STATE: u32 = 13;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetTaskState {
    pub task_cap: u64,
    pub request_id: u64,
}

crate::impl_wire_payload!(GetTaskState {
    task_cap: u64,
    request_id: u64,
});

pub const MSG_AGENT_TASK_V1_GET_TASK_STATE_REPLY: u32 = 14;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetTaskStateReply {
    pub request_id: u64,
    pub state_shm_cap: u64,
    pub state_len: u32,
    pub reserved: u32,
    pub status: u32,
    pub reserved2: u32,
}

crate::impl_wire_payload!(GetTaskStateReply {
    request_id: u64,
    state_shm_cap: u64,
    state_len: u32,
    reserved: u32,
    status: u32,
    reserved2: u32,
});

pub const MSG_AGENT_TASK_V1_POLL_TASK: u32 = 23;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PollTask {
    pub task_cap: u64,
    pub request_id: u64,
    pub subscription_cap: u64,
}

crate::impl_wire_payload!(PollTask {
    task_cap: u64,
    request_id: u64,
    subscription_cap: u64,
});

pub const MSG_AGENT_TASK_V1_POLL_TASK_REPLY: u32 = 24;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PollTaskReply {
    pub request_id: u64,
    pub state_shm_cap: u64,
    pub revision: u64,
    pub state_len: u32,
    pub event_mask: u32,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(PollTaskReply {
    request_id: u64,
    state_shm_cap: u64,
    revision: u64,
    state_len: u32,
    event_mask: u32,
    status: u32,
    reserved: u32,
});

pub const MSG_AGENT_TASK_V1_READ_INPUT: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadInput {
    pub task_cap: u64,
    pub request_id: u64,
    pub resource_id: u64,
}

crate::impl_wire_payload!(ReadInput {
    task_cap: u64,
    request_id: u64,
    resource_id: u64,
});

pub const MSG_AGENT_TASK_V1_READ_INPUT_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadInputReply {
    pub request_id: u64,
    pub input_shm_cap: u64,
    pub content_id_hash: [u8; 32],
    pub status: u32,
    pub input_len: u32,
}

crate::impl_wire_payload!(ReadInputReply {
    request_id: u64,
    input_shm_cap: u64,
    content_id_hash: [u8; 32],
    status: u32,
    input_len: u32,
});

pub const MSG_AGENT_TASK_V1_REQUEST_GRANT: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RequestGrant {
    pub policy_cap: u64,
    pub request_id: u64,
    pub task_id: u64,
    pub resource_id: u64,
    pub lifetime_ms: u32,
    pub rights: u32,
}

crate::impl_wire_payload!(RequestGrant {
    policy_cap: u64,
    request_id: u64,
    task_id: u64,
    resource_id: u64,
    lifetime_ms: u32,
    rights: u32,
});

pub const MSG_AGENT_TASK_V1_REQUEST_GRANT_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RequestGrantReply {
    pub request_id: u64,
    pub task_cap: u64,
    pub generation: u64,
    pub expires_at_ms: u64,
    pub status: u32,
    pub rights: u32,
}

crate::impl_wire_payload!(RequestGrantReply {
    request_id: u64,
    task_cap: u64,
    generation: u64,
    expires_at_ms: u64,
    status: u32,
    rights: u32,
});

pub const MSG_AGENT_TASK_V1_REVOKE_GRANT: u32 = 15;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RevokeGrant {
    pub policy_cap: u64,
    pub request_id: u64,
    pub task_cap: u64,
}

crate::impl_wire_payload!(RevokeGrant {
    policy_cap: u64,
    request_id: u64,
    task_cap: u64,
});

pub const MSG_AGENT_TASK_V1_REVOKE_GRANT_REPLY: u32 = 16;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RevokeGrantReply {
    pub request_id: u64,
    pub generation: u64,
    pub status: u32,
    pub revoked_count: u32,
}

crate::impl_wire_payload!(RevokeGrantReply {
    request_id: u64,
    generation: u64,
    status: u32,
    revoked_count: u32,
});

pub const MSG_AGENT_TASK_V1_STAGE_CANDIDATE: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StageCandidate {
    pub task_cap: u64,
    pub request_id: u64,
    pub source_shm_cap: u64,
    pub source_len: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(StageCandidate {
    task_cap: u64,
    request_id: u64,
    source_shm_cap: u64,
    source_len: u32,
    reserved: u32,
});

pub const MSG_AGENT_TASK_V1_STAGE_CANDIDATE_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StageCandidateReply {
    pub request_id: u64,
    pub candidate_cap: u64,
    pub content_id_hash: [u8; 32],
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(StageCandidateReply {
    request_id: u64,
    candidate_cap: u64,
    content_id_hash: [u8; 32],
    status: u32,
    reserved: u32,
});

pub const MSG_AGENT_TASK_V1_SUBSCRIBE_TASK: u32 = 17;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SubscribeTask {
    pub task_cap: u64,
    pub request_id: u64,
    pub event_mask: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(SubscribeTask {
    task_cap: u64,
    request_id: u64,
    event_mask: u32,
    reserved: u32,
});

pub const MSG_AGENT_TASK_V1_SUBSCRIBE_TASK_PULL: u32 = 21;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SubscribeTaskPull {
    pub task_cap: u64,
    pub request_id: u64,
    pub event_mask: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(SubscribeTaskPull {
    task_cap: u64,
    request_id: u64,
    event_mask: u32,
    reserved: u32,
});

pub const MSG_AGENT_TASK_V1_SUBSCRIBE_TASK_PULL_REPLY: u32 = 22;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SubscribeTaskPullReply {
    pub request_id: u64,
    pub subscription_cap: u64,
    pub revision: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(SubscribeTaskPullReply {
    request_id: u64,
    subscription_cap: u64,
    revision: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_AGENT_TASK_V1_SUBSCRIBE_TASK_REPLY: u32 = 18;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SubscribeTaskReply {
    pub request_id: u64,
    pub subscription_cap: u64,
    pub revision: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(SubscribeTaskReply {
    request_id: u64,
    subscription_cap: u64,
    revision: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_AGENT_TASK_V1_TASK_CHANGED_EVENT: u32 = 19;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TaskChangedEvent {
    pub subscription_cap: u64,
    pub revision: u64,
    pub state_shm_cap: u64,
    pub state_len: u32,
    pub event_type: u32,
}

crate::impl_wire_payload!(TaskChangedEvent {
    subscription_cap: u64,
    revision: u64,
    state_shm_cap: u64,
    state_len: u32,
    event_type: u32,
});

pub const MSG_AGENT_TASK_V1_UNSUBSCRIBE_TASK: u32 = 25;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct UnsubscribeTask {
    pub task_cap: u64,
    pub request_id: u64,
    pub subscription_cap: u64,
}

crate::impl_wire_payload!(UnsubscribeTask {
    task_cap: u64,
    request_id: u64,
    subscription_cap: u64,
});

pub const MSG_AGENT_TASK_V1_UNSUBSCRIBE_TASK_REPLY: u32 = 26;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct UnsubscribeTaskReply {
    pub request_id: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(UnsubscribeTaskReply {
    request_id: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_AGENT_TASK_V1_VALIDATE_CANDIDATE: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ValidateCandidate {
    pub task_cap: u64,
    pub request_id: u64,
    pub candidate_cap: u64,
    pub validator_content_id_hash: [u8; 32],
}

crate::impl_wire_payload!(ValidateCandidate {
    task_cap: u64,
    request_id: u64,
    candidate_cap: u64,
    validator_content_id_hash: [u8; 32],
});

pub const MSG_AGENT_TASK_V1_VALIDATE_CANDIDATE_REPLY: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ValidateCandidateReply {
    pub request_id: u64,
    pub diagnostics_shm_cap: u64,
    pub valid_until_ms: u64,
    pub status: u32,
    pub outcome: u32,
    pub diagnostics_len: u32,
    pub diagnostics_flags: u32,
}

crate::impl_wire_payload!(ValidateCandidateReply {
    request_id: u64,
    diagnostics_shm_cap: u64,
    valid_until_ms: u64,
    status: u32,
    outcome: u32,
    diagnostics_len: u32,
    diagnostics_flags: u32,
});

pub const MSG_AGENT_TASK_V1_VALIDATOR_INPUT_HEADER: u32 = 20;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ValidatorInputHeader {
    pub magic: u32,
    pub candidate_len: u32,
    pub schema_len: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(ValidatorInputHeader {
    magic: u32,
    candidate_len: u32,
    schema_len: u32,
    reserved: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_agent_task_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<CommitCandidate>()];
        let value = CommitCandidate::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CommitCandidate>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CommitCandidate>()];
        let offset = core::mem::offset_of!(CommitCandidate, task_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitCandidate, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitCandidate, candidate_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitCandidate, expected_revision);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitCandidate, expected_content_id_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CommitCandidate initialized bytes");
        let again = CommitCandidate::decode(&output).unwrap();
        assert_eq!(value.task_cap, again.task_cap);
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.candidate_cap, again.candidate_cap);
        assert_eq!(value.expected_revision, again.expected_revision);
        assert_eq!(
            value.expected_content_id_hash,
            again.expected_content_id_hash
        );
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CommitCandidateReply>()];
        let value = CommitCandidateReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CommitCandidateReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CommitCandidateReply>()];
        let offset = core::mem::offset_of!(CommitCandidateReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitCandidateReply, receipt_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitCandidateReply, revision);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitCandidateReply, content_id_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitCandidateReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CommitCandidateReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CommitCandidateReply initialized bytes");
        let again = CommitCandidateReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.receipt_cap, again.receipt_cap);
        assert_eq!(value.revision, again.revision);
        assert_eq!(value.content_id_hash, again.content_id_hash);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GetReceipt>()];
        let value = GetReceipt::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetReceipt>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetReceipt>()];
        let offset = core::mem::offset_of!(GetReceipt, task_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetReceipt, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetReceipt, commit_request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetReceipt initialized bytes");
        let again = GetReceipt::decode(&output).unwrap();
        assert_eq!(value.task_cap, again.task_cap);
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.commit_request_id, again.commit_request_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GetReceiptReply>()];
        let value = GetReceiptReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetReceiptReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetReceiptReply>()];
        let offset = core::mem::offset_of!(GetReceiptReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetReceiptReply, commit_request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetReceiptReply, revision);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetReceiptReply, content_id_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetReceiptReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetReceiptReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetReceiptReply initialized bytes");
        let again = GetReceiptReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.commit_request_id, again.commit_request_id);
        assert_eq!(value.revision, again.revision);
        assert_eq!(value.content_id_hash, again.content_id_hash);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GetTaskState>()];
        let value = GetTaskState::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetTaskState>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetTaskState>()];
        let offset = core::mem::offset_of!(GetTaskState, task_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetTaskState, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetTaskState initialized bytes");
        let again = GetTaskState::decode(&output).unwrap();
        assert_eq!(value.task_cap, again.task_cap);
        assert_eq!(value.request_id, again.request_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<GetTaskStateReply>()];
        let value = GetTaskStateReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<GetTaskStateReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<GetTaskStateReply>()];
        let offset = core::mem::offset_of!(GetTaskStateReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetTaskStateReply, state_shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetTaskStateReply, state_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetTaskStateReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetTaskStateReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(GetTaskStateReply, reserved2);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "GetTaskStateReply initialized bytes");
        let again = GetTaskStateReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.state_shm_cap, again.state_shm_cap);
        assert_eq!(value.state_len, again.state_len);
        assert_eq!(value.reserved, again.reserved);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved2, again.reserved2);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<PollTask>()];
        let value = PollTask::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<PollTask>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<PollTask>()];
        let offset = core::mem::offset_of!(PollTask, task_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollTask, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollTask, subscription_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "PollTask initialized bytes");
        let again = PollTask::decode(&output).unwrap();
        assert_eq!(value.task_cap, again.task_cap);
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.subscription_cap, again.subscription_cap);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<PollTaskReply>()];
        let value = PollTaskReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<PollTaskReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<PollTaskReply>()];
        let offset = core::mem::offset_of!(PollTaskReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollTaskReply, state_shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollTaskReply, revision);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollTaskReply, state_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollTaskReply, event_mask);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollTaskReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PollTaskReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "PollTaskReply initialized bytes");
        let again = PollTaskReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.state_shm_cap, again.state_shm_cap);
        assert_eq!(value.revision, again.revision);
        assert_eq!(value.state_len, again.state_len);
        assert_eq!(value.event_mask, again.event_mask);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ReadInput>()];
        let value = ReadInput::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReadInput>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReadInput>()];
        let offset = core::mem::offset_of!(ReadInput, task_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadInput, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadInput, resource_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReadInput initialized bytes");
        let again = ReadInput::decode(&output).unwrap();
        assert_eq!(value.task_cap, again.task_cap);
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.resource_id, again.resource_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ReadInputReply>()];
        let value = ReadInputReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReadInputReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReadInputReply>()];
        let offset = core::mem::offset_of!(ReadInputReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadInputReply, input_shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadInputReply, content_id_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadInputReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadInputReply, input_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReadInputReply initialized bytes");
        let again = ReadInputReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.input_shm_cap, again.input_shm_cap);
        assert_eq!(value.content_id_hash, again.content_id_hash);
        assert_eq!(value.status, again.status);
        assert_eq!(value.input_len, again.input_len);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<RequestGrant>()];
        let value = RequestGrant::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<RequestGrant>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<RequestGrant>()];
        let offset = core::mem::offset_of!(RequestGrant, policy_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestGrant, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestGrant, task_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestGrant, resource_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestGrant, lifetime_ms);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestGrant, rights);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "RequestGrant initialized bytes");
        let again = RequestGrant::decode(&output).unwrap();
        assert_eq!(value.policy_cap, again.policy_cap);
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.task_id, again.task_id);
        assert_eq!(value.resource_id, again.resource_id);
        assert_eq!(value.lifetime_ms, again.lifetime_ms);
        assert_eq!(value.rights, again.rights);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<RequestGrantReply>()];
        let value = RequestGrantReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<RequestGrantReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<RequestGrantReply>()];
        let offset = core::mem::offset_of!(RequestGrantReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestGrantReply, task_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestGrantReply, generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestGrantReply, expires_at_ms);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestGrantReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestGrantReply, rights);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "RequestGrantReply initialized bytes");
        let again = RequestGrantReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.task_cap, again.task_cap);
        assert_eq!(value.generation, again.generation);
        assert_eq!(value.expires_at_ms, again.expires_at_ms);
        assert_eq!(value.status, again.status);
        assert_eq!(value.rights, again.rights);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<RevokeGrant>()];
        let value = RevokeGrant::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<RevokeGrant>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<RevokeGrant>()];
        let offset = core::mem::offset_of!(RevokeGrant, policy_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RevokeGrant, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RevokeGrant, task_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "RevokeGrant initialized bytes");
        let again = RevokeGrant::decode(&output).unwrap();
        assert_eq!(value.policy_cap, again.policy_cap);
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.task_cap, again.task_cap);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<RevokeGrantReply>()];
        let value = RevokeGrantReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<RevokeGrantReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<RevokeGrantReply>()];
        let offset = core::mem::offset_of!(RevokeGrantReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RevokeGrantReply, generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RevokeGrantReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RevokeGrantReply, revoked_count);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "RevokeGrantReply initialized bytes");
        let again = RevokeGrantReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.generation, again.generation);
        assert_eq!(value.status, again.status);
        assert_eq!(value.revoked_count, again.revoked_count);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<StageCandidate>()];
        let value = StageCandidate::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<StageCandidate>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<StageCandidate>()];
        let offset = core::mem::offset_of!(StageCandidate, task_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StageCandidate, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StageCandidate, source_shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StageCandidate, source_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StageCandidate, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "StageCandidate initialized bytes");
        let again = StageCandidate::decode(&output).unwrap();
        assert_eq!(value.task_cap, again.task_cap);
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.source_shm_cap, again.source_shm_cap);
        assert_eq!(value.source_len, again.source_len);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<StageCandidateReply>()];
        let value = StageCandidateReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<StageCandidateReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<StageCandidateReply>()];
        let offset = core::mem::offset_of!(StageCandidateReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StageCandidateReply, candidate_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StageCandidateReply, content_id_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StageCandidateReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(StageCandidateReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "StageCandidateReply initialized bytes");
        let again = StageCandidateReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.candidate_cap, again.candidate_cap);
        assert_eq!(value.content_id_hash, again.content_id_hash);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<SubscribeTask>()];
        let value = SubscribeTask::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<SubscribeTask>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<SubscribeTask>()];
        let offset = core::mem::offset_of!(SubscribeTask, task_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubscribeTask, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubscribeTask, event_mask);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubscribeTask, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "SubscribeTask initialized bytes");
        let again = SubscribeTask::decode(&output).unwrap();
        assert_eq!(value.task_cap, again.task_cap);
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.event_mask, again.event_mask);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<SubscribeTaskPull>()];
        let value = SubscribeTaskPull::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<SubscribeTaskPull>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<SubscribeTaskPull>()];
        let offset = core::mem::offset_of!(SubscribeTaskPull, task_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubscribeTaskPull, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubscribeTaskPull, event_mask);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubscribeTaskPull, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "SubscribeTaskPull initialized bytes");
        let again = SubscribeTaskPull::decode(&output).unwrap();
        assert_eq!(value.task_cap, again.task_cap);
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.event_mask, again.event_mask);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<SubscribeTaskPullReply>()];
        let value = SubscribeTaskPullReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<SubscribeTaskPullReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<SubscribeTaskPullReply>()];
        let offset = core::mem::offset_of!(SubscribeTaskPullReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubscribeTaskPullReply, subscription_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubscribeTaskPullReply, revision);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubscribeTaskPullReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubscribeTaskPullReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "SubscribeTaskPullReply initialized bytes");
        let again = SubscribeTaskPullReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.subscription_cap, again.subscription_cap);
        assert_eq!(value.revision, again.revision);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<SubscribeTaskReply>()];
        let value = SubscribeTaskReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<SubscribeTaskReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<SubscribeTaskReply>()];
        let offset = core::mem::offset_of!(SubscribeTaskReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubscribeTaskReply, subscription_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubscribeTaskReply, revision);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubscribeTaskReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(SubscribeTaskReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "SubscribeTaskReply initialized bytes");
        let again = SubscribeTaskReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.subscription_cap, again.subscription_cap);
        assert_eq!(value.revision, again.revision);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<TaskChangedEvent>()];
        let value = TaskChangedEvent::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<TaskChangedEvent>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<TaskChangedEvent>()];
        let offset = core::mem::offset_of!(TaskChangedEvent, subscription_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(TaskChangedEvent, revision);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(TaskChangedEvent, state_shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(TaskChangedEvent, state_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(TaskChangedEvent, event_type);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "TaskChangedEvent initialized bytes");
        let again = TaskChangedEvent::decode(&output).unwrap();
        assert_eq!(value.subscription_cap, again.subscription_cap);
        assert_eq!(value.revision, again.revision);
        assert_eq!(value.state_shm_cap, again.state_shm_cap);
        assert_eq!(value.state_len, again.state_len);
        assert_eq!(value.event_type, again.event_type);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<UnsubscribeTask>()];
        let value = UnsubscribeTask::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<UnsubscribeTask>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<UnsubscribeTask>()];
        let offset = core::mem::offset_of!(UnsubscribeTask, task_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(UnsubscribeTask, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(UnsubscribeTask, subscription_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "UnsubscribeTask initialized bytes");
        let again = UnsubscribeTask::decode(&output).unwrap();
        assert_eq!(value.task_cap, again.task_cap);
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.subscription_cap, again.subscription_cap);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<UnsubscribeTaskReply>()];
        let value = UnsubscribeTaskReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<UnsubscribeTaskReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<UnsubscribeTaskReply>()];
        let offset = core::mem::offset_of!(UnsubscribeTaskReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(UnsubscribeTaskReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(UnsubscribeTaskReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "UnsubscribeTaskReply initialized bytes");
        let again = UnsubscribeTaskReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ValidateCandidate>()];
        let value = ValidateCandidate::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ValidateCandidate>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ValidateCandidate>()];
        let offset = core::mem::offset_of!(ValidateCandidate, task_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ValidateCandidate, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ValidateCandidate, candidate_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ValidateCandidate, validator_content_id_hash);
        expected[offset..offset + <[u8; 32] as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ValidateCandidate initialized bytes");
        let again = ValidateCandidate::decode(&output).unwrap();
        assert_eq!(value.task_cap, again.task_cap);
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.candidate_cap, again.candidate_cap);
        assert_eq!(
            value.validator_content_id_hash,
            again.validator_content_id_hash
        );
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ValidateCandidateReply>()];
        let value = ValidateCandidateReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ValidateCandidateReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ValidateCandidateReply>()];
        let offset = core::mem::offset_of!(ValidateCandidateReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ValidateCandidateReply, diagnostics_shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ValidateCandidateReply, valid_until_ms);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ValidateCandidateReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ValidateCandidateReply, outcome);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ValidateCandidateReply, diagnostics_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ValidateCandidateReply, diagnostics_flags);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ValidateCandidateReply initialized bytes");
        let again = ValidateCandidateReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.diagnostics_shm_cap, again.diagnostics_shm_cap);
        assert_eq!(value.valid_until_ms, again.valid_until_ms);
        assert_eq!(value.status, again.status);
        assert_eq!(value.outcome, again.outcome);
        assert_eq!(value.diagnostics_len, again.diagnostics_len);
        assert_eq!(value.diagnostics_flags, again.diagnostics_flags);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ValidatorInputHeader>()];
        let value = ValidatorInputHeader::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ValidatorInputHeader>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ValidatorInputHeader>()];
        let offset = core::mem::offset_of!(ValidatorInputHeader, magic);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ValidatorInputHeader, candidate_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ValidatorInputHeader, schema_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ValidatorInputHeader, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ValidatorInputHeader initialized bytes");
        let again = ValidatorInputHeader::decode(&output).unwrap();
        assert_eq!(value.magic, again.magic);
        assert_eq!(value.candidate_len, again.candidate_len);
        assert_eq!(value.schema_len, again.schema_len);
        assert_eq!(value.reserved, again.reserved);
    }
}
