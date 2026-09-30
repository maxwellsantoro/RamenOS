//! A1.0 wire preflight. This does not claim backend authority enforcement.
use core::mem::size_of;
use kernel_api::agent_task_protocol::{ProtocolError, RIGHT_ALL, parse_request};
use kernel_api::cap::{Handle, HandleKind};
use kernel_api::generated::agent_task_v1::*;
use kernel_api::ipc::Envelope;
use kernel_api::wire::{read_payload, write_payload};

fn envelope<T: Copy>(msg_type: u32, payload: &T) -> Envelope {
    let mut env = Envelope::empty(AGENT_TASK_V1_PROTOCOL_ID, msg_type);
    write_payload(&mut env, payload).unwrap();
    env
}

#[test]
fn all_messages_fit_the_fixed_control_plane_without_implicit_padding() {
    macro_rules! size {
        ($($ty:ty => $expected:expr),+ $(,)?) => {$({
            assert_eq!(size_of::<$ty>(), $expected);
            assert!(size_of::<$ty>() <= Envelope::empty(0, 0).payload.len());
        })+};
    }
    size!(
        ReadInput => 24, ReadInputReply => 56,
        RequestGrant => 40, RequestGrantReply => 40,
        StageCandidate => 32, StageCandidateReply => 56,
        ValidateCandidate => 56, ValidateCandidateReply => 40,
        CommitCandidate => 64, CommitCandidateReply => 64,
        GetReceipt => 24, GetReceiptReply => 64,
        GetTaskState => 16, GetTaskStateReply => 32,
        RevokeGrant => 24, RevokeGrantReply => 24,
        SubscribeTask => 24, SubscribeTaskReply => 32,
        TaskChangedEvent => 32,
    );
}

#[test]
fn commit_preserves_both_revision_and_content_preconditions() {
    let request = CommitCandidate {
        task_cap: 101,
        request_id: 7,
        candidate_cap: 202,
        expected_revision: 0x0102_0304_0506_0708,
        expected_content_id_hash: [0xAB; 32],
    };
    let env = envelope(MSG_AGENT_TASK_V1_COMMIT_CANDIDATE, &request);
    assert_eq!(env.payload_len, 64);
    assert!(parse_request(&env).is_ok());
    let decoded: CommitCandidate = read_payload(&env).unwrap();
    assert_eq!(decoded.task_cap, 101);
    assert_eq!(decoded.candidate_cap, 202);
    assert_eq!(decoded.request_id, 7);
    assert_eq!(decoded.expected_revision, request.expected_revision);
    assert_eq!(
        decoded.expected_content_id_hash,
        request.expected_content_id_hash
    );
}

#[test]
fn validation_requests_cannot_supply_an_outcome_or_attestation() {
    let request = ValidateCandidate {
        task_cap: 101,
        request_id: 8,
        candidate_cap: 202,
        validator_content_id_hash: [0xCC; 32],
    };
    let mut env = envelope(MSG_AGENT_TASK_V1_VALIDATE_CANDIDATE, &request);
    assert!(parse_request(&env).is_ok());
    env.payload[56] = 1; // Attempt to append caller-selected success.
    env.payload_len = 57;
    assert_eq!(parse_request(&env).unwrap_err(), ProtocolError::Wire);
}

#[test]
fn unknown_protocol_operations_and_reply_injection_fail_closed() {
    let request = GetTaskState {
        task_cap: 101,
        request_id: 1,
    };
    let mut env = envelope(MSG_AGENT_TASK_V1_GET_TASK_STATE, &request);
    env.protocol += 1;
    assert_eq!(parse_request(&env).unwrap_err(), ProtocolError::Protocol);
    env.protocol = AGENT_TASK_V1_PROTOCOL_ID;
    for msg_type in [
        0,
        1000,
        MSG_AGENT_TASK_V1_READ_INPUT_REPLY,
        MSG_AGENT_TASK_V1_REQUEST_GRANT_REPLY,
        MSG_AGENT_TASK_V1_STAGE_CANDIDATE_REPLY,
        MSG_AGENT_TASK_V1_VALIDATE_CANDIDATE_REPLY,
        MSG_AGENT_TASK_V1_COMMIT_CANDIDATE_REPLY,
        MSG_AGENT_TASK_V1_GET_RECEIPT_REPLY,
        MSG_AGENT_TASK_V1_GET_TASK_STATE_REPLY,
        MSG_AGENT_TASK_V1_REVOKE_GRANT_REPLY,
        MSG_AGENT_TASK_V1_SUBSCRIBE_TASK_REPLY,
        MSG_AGENT_TASK_V1_TASK_CHANGED_EVENT,
    ] {
        env.msg_type = msg_type;
        assert_eq!(parse_request(&env).unwrap_err(), ProtocolError::Operation);
    }
}

#[test]
fn grant_requests_are_bounded_and_do_not_declare_caller_identity() {
    let mut request = RequestGrant {
        policy_cap: 101,
        request_id: 1,
        task_id: 10,
        resource_id: 20,
        lifetime_ms: 35_000,
        rights: RIGHT_ALL,
    };
    assert!(parse_request(&envelope(MSG_AGENT_TASK_V1_REQUEST_GRANT, &request)).is_ok());
    for fault in 0..8 {
        let original = request;
        match fault {
            0 => request.policy_cap = 0,
            1 => request.request_id = 0,
            2 => request.task_id = 0,
            3 => request.resource_id = 0,
            4 => request.lifetime_ms = 0,
            5 => request.lifetime_ms = 300_001,
            6 => request.rights = 0,
            7 => request.rights = RIGHT_ALL | (1 << 31),
            _ => unreachable!(),
        }
        assert!(parse_request(&envelope(MSG_AGENT_TASK_V1_REQUEST_GRANT, &request)).is_err());
        request = original;
    }
}

#[test]
fn staging_requires_a_canonical_shmem_handle_and_bounded_bytes() {
    let mut request = StageCandidate {
        task_cap: 101,
        request_id: 1,
        source_shm_cap: Handle {
            kind: HandleKind::Shmem,
            index: 1,
            generation: 1,
        }
        .pack(),
        source_len: 1024,
        reserved: 0,
    };
    assert!(parse_request(&envelope(MSG_AGENT_TASK_V1_STAGE_CANDIDATE, &request)).is_ok());
    for fault in 0..7 {
        let original = request;
        match fault {
            0 => request.source_shm_cap = Handle::INVALID.pack(),
            1 => {
                request.source_shm_cap = Handle {
                    kind: HandleKind::Ipc,
                    index: 1,
                    generation: 1,
                }
                .pack()
            }
            2 => request.source_shm_cap |= 1 << 48,
            3 => request.source_len = 0,
            4 => request.source_len = 65_537,
            5 => request.reserved = 1,
            6 => {
                request.source_shm_cap = Handle {
                    kind: HandleKind::Shmem,
                    index: 1,
                    generation: 0,
                }
                .pack()
            }
            _ => unreachable!(),
        }
        assert!(parse_request(&envelope(MSG_AGENT_TASK_V1_STAGE_CANDIDATE, &request)).is_err());
        request = original;
    }
}

#[test]
fn malformed_lengths_and_zero_handles_do_not_reach_dispatch() {
    let request = GetReceipt {
        task_cap: 101,
        request_id: 9,
        commit_request_id: 3,
    };
    let env = envelope(MSG_AGENT_TASK_V1_GET_RECEIPT, &request);
    for len in [0, 23, 25, 65, u32::MAX] {
        let mut malformed = env;
        malformed.payload_len = len;
        assert_eq!(parse_request(&malformed).unwrap_err(), ProtocolError::Wire);
    }
    let missing = GetReceipt {
        task_cap: 0,
        ..request
    };
    assert_eq!(
        parse_request(&envelope(MSG_AGENT_TASK_V1_GET_RECEIPT, &missing)).unwrap_err(),
        ProtocolError::InvalidFields
    );
}

#[test]
fn replies_bind_results_to_the_original_request_and_revision() {
    let reply = CommitCandidateReply {
        request_id: 7,
        receipt_cap: 303,
        revision: 4,
        content_id_hash: [0xDD; 32],
        status: 0,
        reserved: 0,
    };
    let env = envelope(MSG_AGENT_TASK_V1_COMMIT_CANDIDATE_REPLY, &reply);
    let decoded: CommitCandidateReply = read_payload(&env).unwrap();
    assert_eq!(decoded.request_id, 7);
    assert_eq!(decoded.revision, 4);
    assert_eq!(decoded.content_id_hash, [0xDD; 32]);
    assert_eq!(env.payload_len, 64);
    assert_eq!(decoded.reserved, 0);
}

#[test]
fn subscriptions_only_accept_declared_task_events() {
    let request = SubscribeTask {
        task_cap: 101,
        request_id: 1,
        event_mask: 3,
        reserved: 0,
    };
    assert!(parse_request(&envelope(MSG_AGENT_TASK_V1_SUBSCRIBE_TASK, &request)).is_ok());
    for mask in [0, 4, u32::MAX] {
        let invalid = SubscribeTask {
            event_mask: mask,
            ..request
        };
        assert!(parse_request(&envelope(MSG_AGENT_TASK_V1_SUBSCRIBE_TASK, &invalid)).is_err());
    }
    let invalid = SubscribeTask {
        reserved: 1,
        ..request
    };
    assert!(parse_request(&envelope(MSG_AGENT_TASK_V1_SUBSCRIBE_TASK, &invalid)).is_err());
}
