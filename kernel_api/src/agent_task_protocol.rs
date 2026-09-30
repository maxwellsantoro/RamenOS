//! Pure wire preflight for the SW0 task contract. No grant checks or dispatch.
//! The host/target backend must separately validate caller authority, all service
//! object bindings, mappings, pins and durable transaction state.

use crate::cap::{Handle, HandleKind};
use crate::generated::agent_task_v1::*;
use crate::ipc::Envelope;
use crate::wire::read_payload;

pub const VALIDATOR_MEMORY_MAGIC: u32 = 0x3154_5652;
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

/// Consumer preflight for a specific request's reply. This validates syntax and
/// denied-field redaction, not transport authenticity or service authority.
pub fn validate_reply(
    env: &Envelope,
    request_type: u32,
    request_id: u64,
) -> Result<(), ProtocolError> {
    if env.protocol != AGENT_TASK_V1_PROTOCOL_ID
        || env.msg_type
            != request_type
                .checked_add(1)
                .ok_or(ProtocolError::Operation)?
    {
        return Err(ProtocolError::Protocol);
    }
    let (len, status_offset, reserved): (usize, usize, &[usize]) = match request_type {
        1 => (56, 48, &[]),
        3 => (40, 32, &[]),
        5 => (56, 48, &[52]),
        7 => (40, 24, &[]),
        9 | 11 => (64, 56, &[60]),
        13 => (32, 24, &[20, 28]),
        15 => (24, 16, &[]),
        17 => (32, 24, &[28]),
        _ => return Err(ProtocolError::Operation),
    };
    fields(env.payload_len as usize == len)?;
    let word = |offset: usize| {
        u32::from_le_bytes(
            env.payload[offset..offset + 4]
                .try_into()
                .expect("fixed payload"),
        )
    };
    let wide = |offset: usize| {
        u64::from_le_bytes(
            env.payload[offset..offset + 8]
                .try_into()
                .expect("fixed payload"),
        )
    };
    fields(
        wide(0) == request_id
            && request_id != 0
            && reserved.iter().all(|&offset| word(offset) == 0),
    )?;
    let status = word(status_offset);
    fields(status <= STATUS_NOT_FOUND)?;
    if status == STATUS_DENIED || status == STATUS_EXPIRED {
        fields(
            env.payload[8..len]
                .iter()
                .enumerate()
                .all(|(offset, &byte)| {
                    (status_offset..status_offset + 4).contains(&(offset + 8)) || byte == 0
                }),
        )?;
        return Ok(());
    }
    if request_type == 7 {
        fields(word(28) <= OUTCOME_HOST_FAILURE && word(36) & !DIAGNOSTICS_TRUNCATED == 0)?;
    }
    if matches!(request_type, 1 | 7 | 13) {
        let count = word(match request_type {
            1 => 52,
            7 => 32,
            _ => 16,
        });
        fields(count <= if request_type == 13 { 4096 } else { 65_536 })?;
        let handle = Handle::unpack(wide(8));
        fields(if count == 0 {
            wide(8) == 0
        } else {
            handle.kind == HandleKind::Shmem && handle.generation != 0 && handle.pack() == wide(8)
        })?;
        if request_type == 7 && word(28) == OUTCOME_VALID {
            fields(status == STATUS_OK && word(36) == 0)?;
        }
    }
    if status == STATUS_OK {
        match request_type {
            1 | 7 | 13 => {
                let count = word(match request_type {
                    1 => 52,
                    7 => 32,
                    _ => 16,
                });
                fields(count <= if request_type == 13 { 4096 } else { 65_536 })?;
                let handle = Handle::unpack(wide(8));
                fields(if count == 0 {
                    wide(8) == 0 && request_type == 7
                } else {
                    handle.kind == HandleKind::Shmem
                        && handle.generation != 0
                        && handle.pack() == wide(8)
                })?;
                if request_type == 7 {
                    fields(
                        word(28) != OUTCOME_NOT_RUN && (word(28) != OUTCOME_VALID || word(36) == 0),
                    )?;
                }
            }
            3 => {
                fields(
                    wide(8) != 0
                        && wide(16) != 0
                        && wide(24) != 0
                        && word(36) != 0
                        && word(36) & !RIGHT_ALL == 0,
                )?;
            }
            5 => {
                fields(wide(8) != 0)?;
            }
            9 | 11 => {
                fields(wide(8) != 0 && wide(16) != 0)?;
            }
            15 => {
                fields(wide(8) != 0)?;
            }
            17 => {
                fields(wide(8) != 0)?;
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn validate_event(env: &Envelope) -> Result<(), ProtocolError> {
    if env.protocol != AGENT_TASK_V1_PROTOCOL_ID
        || env.msg_type != MSG_AGENT_TASK_V1_TASK_CHANGED_EVENT
    {
        return Err(ProtocolError::Protocol);
    }
    let event: TaskChangedEvent = payload(env)?;
    let handle = Handle::unpack(event.state_shm_cap);
    fields(
        event.subscription_cap != 0
            && event.state_len != 0
            && event.state_len <= 4096
            && matches!(
                event.event_type,
                EVENT_OUTPUT_CHANGED | EVENT_VALIDATION_CHANGED
            )
            && handle.kind == HandleKind::Shmem
            && handle.generation != 0
            && handle.pack() == event.state_shm_cap,
    )
}
