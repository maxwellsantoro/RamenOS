//! Pure wire preflight for the SW0 task contract. No grant checks or dispatch.
//! The host/target backend must separately validate caller authority, all service
//! object bindings, mappings, pins and durable transaction state.

use crate::cap::{Handle, HandleKind};
use crate::generated::agent_task_v1::*;
use crate::ipc::Envelope;
use crate::wire::read_payload;

pub const RIGHT_READ: u32 = 1;
pub const RIGHT_STAGE: u32 = 2;
pub const RIGHT_VALIDATE: u32 = 4;
pub const RIGHT_COMMIT: u32 = 8;
pub const RIGHT_OBSERVE: u32 = 16;
pub const RIGHT_ALL: u32 = 31;
pub const MAX_CANDIDATE_BYTES: u32 = 65_536;
pub const MAX_GRANT_LIFETIME_MS: u32 = 300_000;
pub const EVENT_OUTPUT_CHANGED: u32 = 1;
pub const EVENT_VALIDATION_CHANGED: u32 = 2;
pub const EVENT_ALL: u32 = 3;
pub const STATUS_OK: u32 = 0;
pub const STATUS_DENIED: u32 = 1;
pub const STATUS_INVALID: u32 = 2;
pub const STATUS_CONFLICT: u32 = 3;
pub const STATUS_VALIDATION_FAILED: u32 = 4;
pub const STATUS_EXPIRED: u32 = 5;
pub const STATUS_TIMEOUT: u32 = 6;
pub const STATUS_CAPACITY: u32 = 7;
pub const STATUS_IO: u32 = 8;
pub const STATUS_REQUEST_REUSE: u32 = 9;
pub const STATUS_NOT_FOUND: u32 = 10;
pub const OUTCOME_NOT_RUN: u32 = 0;
pub const OUTCOME_VALID: u32 = 1;
pub const OUTCOME_INVALID: u32 = 2;
pub const OUTCOME_TIMEOUT: u32 = 3;
pub const OUTCOME_HOST_FAILURE: u32 = 4;
pub const DIAGNOSTICS_TRUNCATED: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    Protocol,
    Operation,
    Wire,
    InvalidFields,
}

#[derive(Debug, Clone, Copy)]
pub enum TaskRequest {
    Read(ReadInput),
    Grant(RequestGrant),
    Stage(StageCandidate),
    Validate(ValidateCandidate),
    Commit(CommitCandidate),
    Receipt(GetReceipt),
    State(GetTaskState),
    Revoke(RevokeGrant),
    Subscribe(SubscribeTask),
}

fn fields(valid: bool) -> Result<(), ProtocolError> {
    if valid {
        Ok(())
    } else {
        Err(ProtocolError::InvalidFields)
    }
}

fn payload<T: Copy>(env: &Envelope) -> Result<T, ProtocolError> {
    read_payload(env).map_err(|_| ProtocolError::Wire)
}

/// Parse only request operations; replies/events cannot become requests.
/// Successful preflight is never an authorization decision.
pub fn parse_request(env: &Envelope) -> Result<TaskRequest, ProtocolError> {
    if env.protocol != AGENT_TASK_V1_PROTOCOL_ID {
        return Err(ProtocolError::Protocol);
    }
    match env.msg_type {
        MSG_AGENT_TASK_V1_READ_INPUT => {
            let req: ReadInput = payload(env)?;
            fields(req.task_cap != 0 && req.request_id != 0 && req.resource_id != 0)?;
            Ok(TaskRequest::Read(req))
        }
        MSG_AGENT_TASK_V1_REQUEST_GRANT => {
            let req: RequestGrant = payload(env)?;
            fields(
                req.policy_cap != 0
                    && req.request_id != 0
                    && req.task_id != 0
                    && req.resource_id != 0
                    && req.lifetime_ms != 0
                    && req.lifetime_ms <= MAX_GRANT_LIFETIME_MS
                    && req.rights != 0
                    && req.rights & !RIGHT_ALL == 0,
            )?;
            Ok(TaskRequest::Grant(req))
        }
        MSG_AGENT_TASK_V1_STAGE_CANDIDATE => {
            let req: StageCandidate = payload(env)?;
            let source = Handle::unpack(req.source_shm_cap);
            fields(
                req.task_cap != 0
                    && req.request_id != 0
                    && req.reserved == 0
                    && req.source_len != 0
                    && req.source_len <= MAX_CANDIDATE_BYTES
                    && source.kind == HandleKind::Shmem
                    && source.generation != 0
                    && source.pack() == req.source_shm_cap,
            )?;
            Ok(TaskRequest::Stage(req))
        }
        MSG_AGENT_TASK_V1_VALIDATE_CANDIDATE => {
            let req: ValidateCandidate = payload(env)?;
            fields(req.task_cap != 0 && req.request_id != 0 && req.candidate_cap != 0)?;
            Ok(TaskRequest::Validate(req))
        }
        MSG_AGENT_TASK_V1_COMMIT_CANDIDATE => {
            let req: CommitCandidate = payload(env)?;
            fields(req.task_cap != 0 && req.request_id != 0 && req.candidate_cap != 0)?;
            Ok(TaskRequest::Commit(req))
        }
        MSG_AGENT_TASK_V1_GET_RECEIPT => {
            let req: GetReceipt = payload(env)?;
            fields(req.task_cap != 0 && req.request_id != 0 && req.commit_request_id != 0)?;
            Ok(TaskRequest::Receipt(req))
        }
        MSG_AGENT_TASK_V1_GET_TASK_STATE => {
            let req: GetTaskState = payload(env)?;
            fields(req.task_cap != 0 && req.request_id != 0)?;
            Ok(TaskRequest::State(req))
        }
        MSG_AGENT_TASK_V1_REVOKE_GRANT => {
            let req: RevokeGrant = payload(env)?;
            fields(req.policy_cap != 0 && req.request_id != 0 && req.task_cap != 0)?;
            Ok(TaskRequest::Revoke(req))
        }
        MSG_AGENT_TASK_V1_SUBSCRIBE_TASK => {
            let req: SubscribeTask = payload(env)?;
            fields(
                req.task_cap != 0
                    && req.request_id != 0
                    && req.reserved == 0
                    && req.event_mask != 0
                    && req.event_mask & !EVENT_ALL == 0,
            )?;
            Ok(TaskRequest::Subscribe(req))
        }
        _ => Err(ProtocolError::Operation),
    }
}
