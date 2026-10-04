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

pub const MSG_AGENT_TASK_V1_GET_RECEIPT: u32 = 11;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetReceipt {
    pub task_cap: u64,
    pub request_id: u64,
    pub commit_request_id: u64,
}

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

pub const MSG_AGENT_TASK_V1_GET_TASK_STATE: u32 = 13;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GetTaskState {
    pub task_cap: u64,
    pub request_id: u64,
}

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

pub const MSG_AGENT_TASK_V1_POLL_TASK: u32 = 23;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PollTask {
    pub task_cap: u64,
    pub request_id: u64,
    pub subscription_cap: u64,
}

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

pub const MSG_AGENT_TASK_V1_READ_INPUT: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadInput {
    pub task_cap: u64,
    pub request_id: u64,
    pub resource_id: u64,
}

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

pub const MSG_AGENT_TASK_V1_REVOKE_GRANT: u32 = 15;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RevokeGrant {
    pub policy_cap: u64,
    pub request_id: u64,
    pub task_cap: u64,
}

pub const MSG_AGENT_TASK_V1_REVOKE_GRANT_REPLY: u32 = 16;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RevokeGrantReply {
    pub request_id: u64,
    pub generation: u64,
    pub status: u32,
    pub revoked_count: u32,
}

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

pub const MSG_AGENT_TASK_V1_SUBSCRIBE_TASK: u32 = 17;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SubscribeTask {
    pub task_cap: u64,
    pub request_id: u64,
    pub event_mask: u32,
    pub reserved: u32,
}

pub const MSG_AGENT_TASK_V1_SUBSCRIBE_TASK_PULL: u32 = 21;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SubscribeTaskPull {
    pub task_cap: u64,
    pub request_id: u64,
    pub event_mask: u32,
    pub reserved: u32,
}

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

pub const MSG_AGENT_TASK_V1_UNSUBSCRIBE_TASK: u32 = 25;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct UnsubscribeTask {
    pub task_cap: u64,
    pub request_id: u64,
    pub subscription_cap: u64,
}

pub const MSG_AGENT_TASK_V1_UNSUBSCRIBE_TASK_REPLY: u32 = 26;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct UnsubscribeTaskReply {
    pub request_id: u64,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_AGENT_TASK_V1_VALIDATE_CANDIDATE: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ValidateCandidate {
    pub task_cap: u64,
    pub request_id: u64,
    pub candidate_cap: u64,
    pub validator_content_id_hash: [u8; 32],
}

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

pub const MSG_AGENT_TASK_V1_VALIDATOR_INPUT_HEADER: u32 = 20;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ValidatorInputHeader {
    pub magic: u32,
    pub candidate_len: u32,
    pub schema_len: u32,
    pub reserved: u32,
}
