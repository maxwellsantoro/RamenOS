#![cfg(feature = "agent_task_v1_dev")]
//! Forged requests use actual host IPC, bypassing the scripted adapter.
use artifact_store_schema::agent_task::{ExecutionBudgetV0, TaskContractV0};
use kernel_api::agent_task_protocol::*;
use kernel_api::generated::agent_task_v1::*;
use kernel_api::ipc::Envelope;
use kernel_api::wire::{read_payload, write_payload};
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use store_service::agent_task::{
    TaskFixture, TaskService, content_hash, exchange, serve_connection,
};
fn id(b: &[u8]) -> String {
    artifact_store_core::hash_bytes(b)
}
fn fixture() -> TaskFixture {
    let schema = include_bytes!("../../../tools/agent_task/fixtures/schema.json").to_vec();
    let policy = include_bytes!("../../../tools/agent_task/fixtures/policy.json").to_vec();
    let validator = wat::parse_str(include_str!(
        "../../../tools/agent_task/fixtures/equality_validator.wat"
    ))
    .unwrap();
    TaskFixture {
        contract: TaskContractV0 {
            schema_version: 1,
            task_id: 17,
            domain_id: 7,
            resource: "workspace:a/config".into(),
            schema_id: id(&schema),
            policy_id: id(&policy),
            validator_id: id(&validator),
            execution_budget: ExecutionBudgetV0 {
                guest_ms: 1500,
                wall_ms: 2500,
                host_call_ms: 1000,
                max_diagnostics_bytes: 4096,
            },
        },
        resource_id: 1,
        input: include_bytes!("../../../tools/agent_task/fixtures/config.json").to_vec(),
        schema,
        policy,
        notes: include_bytes!("../../../tools/agent_task/fixtures/notes.txt").to_vec(),
        validator,
    }
}
fn req<T: Copy>(ty: u32, v: &T) -> Envelope {
    let mut e = Envelope::empty(14, ty);
    write_payload(&mut e, v).unwrap();
    e
}
fn connect(s: &Arc<Mutex<TaskService>>, domain: u64) -> UnixStream {
    let (a, b) = UnixStream::pair().unwrap();
    let s = s.clone();
    std::thread::spawn(move || {
        let _ = serve_connection(s, domain, b);
    });
    a
}
fn grant(s: &Arc<Mutex<TaskService>>, c: &mut UnixStream, rights: u32) -> u64 {
    let policy_cap = s.lock().unwrap().policy_cap();
    let r: RequestGrantReply = read_payload(
        &exchange(
            c,
            &req(
                3,
                &RequestGrant {
                    policy_cap,
                    request_id: 1,
                    task_id: 17,
                    resource_id: 1,
                    lifetime_ms: 60_000,
                    rights,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(r.status, STATUS_OK);
    r.task_cap
}
fn stage(
    s: &Arc<Mutex<TaskService>>,
    c: &mut UnixStream,
    cap: u64,
    b: &[u8],
) -> StageCandidateReply {
    let source = s.lock().unwrap().install_source(7, b).unwrap();
    read_payload(
        &exchange(
            c,
            &req(
                5,
                &StageCandidate {
                    task_cap: cap,
                    request_id: 2,
                    source_shm_cap: source,
                    source_len: b.len() as u32,
                    reserved: 0,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap()
}
fn validate(c: &mut UnixStream, cap: u64, candidate: u64, pin: [u8; 32]) -> ValidateCandidateReply {
    read_payload(
        &exchange(
            c,
            &req(
                7,
                &ValidateCandidate {
                    task_cap: cap,
                    request_id: 3,
                    candidate_cap: candidate,
                    validator_content_id_hash: pin,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap()
}
fn commit(
    c: &mut UnixStream,
    cap: u64,
    candidate: u64,
    revision: u64,
    prior: [u8; 32],
    id: u64,
) -> CommitCandidateReply {
    read_payload(
        &exchange(
            c,
            &req(
                9,
                &CommitCandidate {
                    task_cap: cap,
                    request_id: id,
                    candidate_cap: candidate,
                    expected_revision: revision,
                    expected_content_id_hash: prior,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap()
}
fn open(p: &std::path::Path) -> Arc<Mutex<TaskService>> {
    Arc::new(Mutex::new(
        TaskService::open(
            p,
            fixture(),
            std::env::var_os("RAMEN_TASK_VALIDATOR_WORKER")
                .expect("build/set validator worker")
                .into(),
        )
        .unwrap(),
    ))
}
#[test]
fn useful_repair_and_restart_retry_have_one_durable_effect() {
    let d = tempfile::tempdir().unwrap();
    let s = open(d.path());
    let mut c = connect(&s, 7);
    let cap = grant(&s, &mut c, RIGHT_ALL);
    let snapshot: GetTaskStateReply = read_payload(
        &exchange(
            &mut c,
            &req(
                13,
                &GetTaskState {
                    task_cap: cap,
                    request_id: 96,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap();
    let view: store_service::agent_task::TaskSnapshotV1 = serde_json::from_slice(
        &s.lock()
            .unwrap()
            .read_mapping(7, snapshot.state_shm_cap)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(view.task_id, 17);
    assert_eq!(view.revision, 0);
    s.lock()
        .unwrap()
        .release_mapping(7, snapshot.state_shm_cap)
        .unwrap();
    let mut inputs = Vec::new();
    for (offset, resource_id) in [
        1,
        store_service::agent_task::SCHEMA_RESOURCE,
        store_service::agent_task::NOTES_RESOURCE,
    ]
    .into_iter()
    .enumerate()
    {
        let input: ReadInputReply = read_payload(
            &exchange(
                &mut c,
                &req(
                    1,
                    &ReadInput {
                        task_cap: cap,
                        request_id: 97 + offset as u64,
                        resource_id,
                    },
                ),
            )
            .unwrap(),
        )
        .unwrap();
        let bytes = s
            .lock()
            .unwrap()
            .read_mapping(7, input.input_shm_cap)
            .unwrap();
        assert_eq!(content_hash(&bytes), input.content_id_hash);
        s.lock()
            .unwrap()
            .release_mapping(7, input.input_shm_cap)
            .unwrap();
        inputs.push(bytes);
    }
    let mut candidate: serde_json::Value = serde_json::from_slice(&inputs[0]).unwrap();
    let schema: serde_json::Value = serde_json::from_slice(&inputs[1]).unwrap();
    let original_label = candidate["label"].clone();
    candidate["enabled"] = schema["enabled"].clone();
    assert_eq!(candidate["label"], original_label);
    let repaired = serde_json::to_vec(&candidate).unwrap();
    let fixed = br#"{"enabled":true,"label":"keep"}"#;
    assert_eq!(repaired, fixed);
    let staged = stage(&s, &mut c, cap, &repaired);
    assert_eq!(staged.status, STATUS_OK);
    assert_eq!(
        validate(
            &mut c,
            cap,
            staged.candidate_cap,
            content_hash(&fixture().validator)
        )
        .outcome,
        OUTCOME_VALID
    );
    let r = commit(
        &mut c,
        cap,
        staged.candidate_cap,
        0,
        content_hash(&fixture().input),
        4,
    );
    assert_eq!(r.status, STATUS_OK);
    assert_eq!(r.revision, 1);
    assert_eq!(s.lock().unwrap().accepted_bytes().unwrap(), fixed);
    assert_eq!(
        commit(
            &mut c,
            cap,
            staged.candidate_cap,
            0,
            content_hash(&fixture().input),
            4
        )
        .receipt_cap,
        r.receipt_cap
    );
    drop(c);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while Arc::strong_count(&s) > 1 {
        assert!(
            std::time::Instant::now() < deadline,
            "closed service connection did not finish"
        );
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    drop(s);
    let s = open(d.path());
    let mut c = connect(&s, 7);
    let cap = grant(&s, &mut c, RIGHT_ALL);
    let retry = commit(
        &mut c,
        cap,
        staged.candidate_cap,
        0,
        content_hash(&fixture().input),
        4,
    );
    assert_eq!(retry.status, STATUS_OK);
    assert_eq!(retry.receipt_cap, r.receipt_cap);
    assert_eq!(retry.revision, 1);
    assert_eq!(
        commit(&mut c, cap, staged.candidate_cap, 1, r.content_id_hash, 4).status,
        STATUS_REQUEST_REUSE
    );
    let evidence = s.lock().unwrap().evidence().unwrap();
    let accepted = store_service::agent_task::verify_evidence(
        &evidence,
        &fixture().contract,
        &fixture().input,
    )
    .unwrap();
    assert_eq!(accepted.revision, 1);
    if let Some(path) = std::env::var_os("RAMEN_TASK_EVIDENCE_DIR") {
        let root = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("journal.json"), &evidence).unwrap();
        let report = serde_json::json!({"schema_version":1,"environment":"host","task":"scoped-configuration-repair","accepted":accepted,"contract":fixture().contract,"repair_checked":true,"unrelated_fields_preserved":true,"receipt_replay_checked":true,"target_kernel_enforcement":false,"model_comparison":false});
        std::fs::write(
            root.join("report.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
    s.lock().unwrap().verify_audit().unwrap();
}
#[test]
fn forged_domain_resource_rights_and_object_kind_do_not_disclose_or_mutate() {
    let d = tempfile::tempdir().unwrap();
    let s = open(d.path());
    let mut c = connect(&s, 7);
    let cap = grant(&s, &mut c, RIGHT_ALL);
    let mut foreign = connect(&s, 8);
    let denied: GetTaskStateReply = read_payload(
        &exchange(
            &mut foreign,
            &req(
                13,
                &GetTaskState {
                    task_cap: cap,
                    request_id: 5,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(denied.status, STATUS_DENIED);
    assert_eq!(denied.state_shm_cap, 0);
    assert_eq!(denied.state_len, 0);
    let denied: ReadInputReply = read_payload(
        &exchange(
            &mut c,
            &req(
                1,
                &ReadInput {
                    task_cap: cap,
                    request_id: 6,
                    resource_id: 2,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(denied.status, STATUS_DENIED);
    assert_eq!(denied.content_id_hash, [0; 32]);
    let ro = grant(&s, &mut c, RIGHT_READ | RIGHT_OBSERVE);
    assert_eq!(stage(&s, &mut c, ro, b"private").status, STATUS_DENIED);
    assert_eq!(
        commit(&mut c, cap, cap, 0, [0; 32], 7).status,
        STATUS_DENIED
    );
    assert_eq!(s.lock().unwrap().accepted_bytes().unwrap(), fixture().input);
}
#[test]
fn unvalidated_wrong_pin_invalid_and_revoked_candidates_cannot_commit() {
    let d = tempfile::tempdir().unwrap();
    let s = open(d.path());
    let mut c = connect(&s, 7);
    let cap = grant(&s, &mut c, RIGHT_ALL);
    let prior = content_hash(&fixture().input);
    let valid = stage(&s, &mut c, cap, br#"{"enabled":true,"label":"keep"}"#);
    assert_eq!(
        commit(&mut c, cap, valid.candidate_cap, 0, prior, 10).status,
        STATUS_VALIDATION_FAILED
    );
    assert_eq!(
        validate(&mut c, cap, valid.candidate_cap, [0; 32]).status,
        STATUS_DENIED
    );
    let invalid = stage(&s, &mut c, cap, br#"{"enabled":false,"label":"changed"}"#);
    assert_eq!(
        validate(
            &mut c,
            cap,
            invalid.candidate_cap,
            content_hash(&fixture().validator)
        )
        .outcome,
        OUTCOME_INVALID
    );
    assert_eq!(
        commit(&mut c, cap, invalid.candidate_cap, 0, prior, 11).status,
        STATUS_VALIDATION_FAILED
    );
    let policy_cap = s.lock().unwrap().policy_cap();
    let r: RevokeGrantReply = read_payload(
        &exchange(
            &mut c,
            &req(
                15,
                &RevokeGrant {
                    policy_cap,
                    request_id: 12,
                    task_cap: cap,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(r.status, STATUS_OK);
    assert_eq!(
        commit(&mut c, cap, valid.candidate_cap, 0, prior, 13).status,
        STATUS_DENIED
    );
}

#[test]
fn read_state_mappings_are_scoped_read_only_and_removed_on_revocation() {
    let d = tempfile::tempdir().unwrap();
    let s = open(d.path());
    let mut c = connect(&s, 7);
    let cap = grant(&s, &mut c, RIGHT_ALL);
    let r: ReadInputReply = read_payload(
        &exchange(
            &mut c,
            &req(
                1,
                &ReadInput {
                    task_cap: cap,
                    request_id: 20,
                    resource_id: 1,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(r.status, STATUS_OK);
    assert_eq!(
        s.lock().unwrap().read_mapping(7, r.input_shm_cap).unwrap(),
        fixture().input
    );
    assert_eq!(
        s.lock().unwrap().read_mapping(8, r.input_shm_cap),
        Err(STATUS_DENIED)
    );
    assert_eq!(
        s.lock()
            .unwrap()
            .write_source(7, r.input_shm_cap, &fixture().input),
        Err(STATUS_DENIED)
    );
    let snapshot: GetTaskStateReply = read_payload(
        &exchange(
            &mut c,
            &req(
                13,
                &GetTaskState {
                    task_cap: cap,
                    request_id: 21,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap();
    let bytes = s
        .lock()
        .unwrap()
        .read_mapping(7, snapshot.state_shm_cap)
        .unwrap();
    let state: store_service::agent_task::TaskSnapshotV1 = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(state.validator_id, fixture().contract.validator_id);
    assert_eq!(state.revision, 0);
    let policy_cap = s.lock().unwrap().policy_cap();
    let _: RevokeGrantReply = read_payload(
        &exchange(
            &mut c,
            &req(
                15,
                &RevokeGrant {
                    policy_cap,
                    request_id: 22,
                    task_cap: cap,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        s.lock().unwrap().read_mapping(7, r.input_shm_cap),
        Err(STATUS_DENIED)
    );
    assert_eq!(s.lock().unwrap().mapping_count(), 0);
}
#[test]
fn mutable_source_cannot_change_a_staged_candidate_and_foreign_sources_are_denied() {
    let d = tempfile::tempdir().unwrap();
    let s = open(d.path());
    let mut c = connect(&s, 7);
    let cap = grant(&s, &mut c, RIGHT_ALL);
    let fixed = br#"{"enabled":true,"label":"keep"}"#;
    let source = s.lock().unwrap().install_source(7, fixed).unwrap();
    let r: StageCandidateReply = read_payload(
        &exchange(
            &mut c,
            &req(
                5,
                &StageCandidate {
                    task_cap: cap,
                    request_id: 23,
                    source_shm_cap: source,
                    source_len: fixed.len() as u32,
                    reserved: 0,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap();
    s.lock()
        .unwrap()
        .write_source(7, source, &vec![b'x'; fixed.len()])
        .unwrap();
    assert_eq!(
        validate(
            &mut c,
            cap,
            r.candidate_cap,
            content_hash(&fixture().validator)
        )
        .outcome,
        OUTCOME_VALID
    );
    assert_eq!(
        commit(
            &mut c,
            cap,
            r.candidate_cap,
            0,
            content_hash(&fixture().input),
            24
        )
        .status,
        STATUS_OK
    );
    assert_eq!(s.lock().unwrap().accepted_bytes().unwrap(), fixed);
    let foreign = s.lock().unwrap().install_source(8, b"canary").unwrap();
    let r: StageCandidateReply = read_payload(
        &exchange(
            &mut c,
            &req(
                5,
                &StageCandidate {
                    task_cap: cap,
                    request_id: 25,
                    source_shm_cap: foreign,
                    source_len: 6,
                    reserved: 0,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(r.status, STATUS_DENIED);
    assert_eq!(r.content_id_hash, [0; 32]);
}
#[test]
fn concurrent_preconditions_and_aba_do_not_replace_newer_output_or_repeat_effects() {
    let d = tempfile::tempdir().unwrap();
    let s = open(d.path());
    let mut c = connect(&s, 7);
    let cap = grant(&s, &mut c, RIGHT_ALL);
    let a = stage(&s, &mut c, cap, br#"{"enabled":true,"label":"keep"}"#);
    let b = stage(&s, &mut c, cap, br#"{ "enabled": true, "label": "keep" }"#);
    let pin = content_hash(&fixture().validator);
    assert_eq!(
        validate(&mut c, cap, a.candidate_cap, pin).outcome,
        OUTCOME_VALID
    );
    assert_eq!(
        validate(&mut c, cap, b.candidate_cap, pin).outcome,
        OUTCOME_VALID
    );
    let initial = content_hash(&fixture().input);
    let first = commit(&mut c, cap, a.candidate_cap, 0, initial, 30);
    assert_eq!(first.status, STATUS_OK);
    assert_eq!(
        commit(&mut c, cap, b.candidate_cap, 0, initial, 31).status,
        STATUS_CONFLICT
    );
    let second = commit(&mut c, cap, b.candidate_cap, 1, first.content_id_hash, 32);
    assert_eq!(second.status, STATUS_OK);
    let third = commit(&mut c, cap, a.candidate_cap, 2, second.content_id_hash, 33);
    assert_eq!(third.status, STATUS_OK);
    assert_eq!(third.content_id_hash, first.content_id_hash);
    assert_eq!(
        commit(&mut c, cap, b.candidate_cap, 1, first.content_id_hash, 34).status,
        STATUS_CONFLICT
    );
    assert_eq!(
        commit(&mut c, cap, a.candidate_cap, 0, initial, 30).receipt_cap,
        first.receipt_cap
    );
    let evidence = s.lock().unwrap().evidence().unwrap();
    assert_eq!(
        store_service::agent_task::verify_evidence(
            &evidence,
            &fixture().contract,
            &fixture().input
        )
        .unwrap()
        .revision,
        3
    );
    let mut tampered: serde_json::Value = serde_json::from_slice(&evidence).unwrap();
    tampered["audit"][0]["status"] = 99.into();
    assert!(
        store_service::agent_task::verify_evidence(
            &serde_json::to_vec(&tampered).unwrap(),
            &fixture().contract,
            &fixture().input
        )
        .is_err()
    );
}
#[test]
fn failed_journal_publication_poisoning_recovers_the_previous_accepted_output() {
    let d = tempfile::tempdir().unwrap();
    let s = open(d.path());
    let mut c = connect(&s, 7);
    let cap = grant(&s, &mut c, RIGHT_ALL);
    let a = stage(&s, &mut c, cap, br#"{"enabled":true,"label":"keep"}"#);
    validate(
        &mut c,
        cap,
        a.candidate_cap,
        content_hash(&fixture().validator),
    );
    std::fs::create_dir(d.path().join("task.next")).unwrap();
    assert_eq!(
        commit(
            &mut c,
            cap,
            a.candidate_cap,
            0,
            content_hash(&fixture().input),
            40
        )
        .status,
        STATUS_IO
    );
    assert!(s.lock().unwrap().accepted_bytes().is_err());
    drop(c);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while Arc::strong_count(&s) > 1 {
        assert!(
            std::time::Instant::now() < deadline,
            "closed service connection did not finish"
        );
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    drop(s);
    std::fs::remove_dir(d.path().join("task.next")).unwrap();
    let s = open(d.path());
    assert_eq!(s.lock().unwrap().accepted_bytes().unwrap(), fixture().input);
}
#[test]
fn wrong_fixture_pins_corrupt_journal_and_second_writer_fail_closed() {
    let d = tempfile::tempdir().unwrap();
    let s = open(d.path());
    let worker = std::env::var_os("RAMEN_TASK_VALIDATOR_WORKER").unwrap();
    assert!(TaskService::open(d.path(), fixture(), worker.clone().into()).is_err());
    let mut wrong = fixture();
    wrong.schema.push(b'x');
    assert!(TaskService::open(d.path(), wrong, worker.clone().into()).is_err());
    drop(s);
    std::fs::write(d.path().join("task.json"), b"{}").unwrap();
    assert!(TaskService::open(d.path(), fixture(), worker.into()).is_err());
}
#[test]
fn revocation_during_validator_execution_discards_the_result() {
    let d = tempfile::tempdir().unwrap();
    let mut f = fixture();
    f.validator = wat::parse_str(
        "(module (memory (export \"memory\") 3 3) (func (export \"_start\") (result i32) (loop $l (br $l)) i32.const 0))",
    )
    .unwrap();
    f.contract.validator_id = id(&f.validator);
    f.contract.execution_budget.guest_ms = 300;
    let pin = content_hash(&f.validator);
    let worker = std::env::var_os("RAMEN_TASK_VALIDATOR_WORKER").unwrap();
    let s = Arc::new(Mutex::new(
        TaskService::open(d.path(), f, worker.into()).unwrap(),
    ));
    let mut c = connect(&s, 7);
    let cap = grant(&s, &mut c, RIGHT_ALL);
    let a = stage(&s, &mut c, cap, br#"{"enabled":true,"label":"keep"}"#);
    let job = std::thread::spawn(move || validate(&mut c, cap, a.candidate_cap, pin));
    std::thread::sleep(std::time::Duration::from_millis(75));
    let mut control = connect(&s, 7);
    let policy_cap = s.lock().unwrap().policy_cap();
    let r: RevokeGrantReply = read_payload(
        &exchange(
            &mut control,
            &req(
                15,
                &RevokeGrant {
                    policy_cap,
                    request_id: 41,
                    task_cap: cap,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(r.status, STATUS_OK);
    assert_eq!(job.join().unwrap().status, STATUS_DENIED);
    assert_eq!(s.lock().unwrap().accepted_bytes().unwrap(), fixture().input);
}
#[test]
fn bounded_events_are_scoped_and_expiry_and_revocation_end_delivery() {
    let d = tempfile::tempdir().unwrap();
    let s = open(d.path());
    let mut c = connect(&s, 7);
    let cap = grant(&s, &mut c, RIGHT_ALL);
    let sub: SubscribeTaskReply = read_payload(
        &exchange(
            &mut c,
            &req(
                17,
                &SubscribeTask {
                    task_cap: cap,
                    request_id: 50,
                    event_mask: EVENT_ALL,
                    reserved: 0,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(sub.status, STATUS_OK);
    let a = stage(&s, &mut c, cap, br#"{"enabled":true,"label":"keep"}"#);
    validate(
        &mut c,
        cap,
        a.candidate_cap,
        content_hash(&fixture().validator),
    );
    assert!(s.lock().unwrap().take_events(8).is_empty());
    let native_event = store_service::agent_task::next_event(&mut c).unwrap();
    let event: TaskChangedEvent = read_payload(&native_event).unwrap();
    assert_eq!(event.subscription_cap, sub.subscription_cap);
    assert_eq!(event.event_type, EVENT_VALIDATION_CHANGED);
    validate(
        &mut c,
        cap,
        a.candidate_cap,
        content_hash(&fixture().validator),
    );
    let policy_cap = s.lock().unwrap().policy_cap();
    let _: RevokeGrantReply = read_payload(
        &exchange(
            &mut c,
            &req(
                15,
                &RevokeGrant {
                    policy_cap,
                    request_id: 51,
                    task_cap: cap,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(s.lock().unwrap().take_events(7).is_empty());
    let grant: RequestGrantReply = read_payload(
        &exchange(
            &mut c,
            &req(
                3,
                &RequestGrant {
                    policy_cap,
                    request_id: 52,
                    task_id: 17,
                    resource_id: 1,
                    lifetime_ms: 5,
                    rights: RIGHT_ALL,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(grant.status, 0);
    std::thread::sleep(std::time::Duration::from_millis(10));
    let reply: GetTaskStateReply = read_payload(
        &exchange(
            &mut c,
            &req(
                13,
                &GetTaskState {
                    task_cap: grant.task_cap,
                    request_id: 53,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(reply.status, STATUS_DENIED);
}

#[test]
fn abrupt_process_crash_before_or_after_publication_recovers_one_effect() {
    const ROOT: &str = "RAMEN_TASK_CRASH_ROOT";
    const PHASE: &str = "RAMEN_TASK_CRASH_PHASE";
    if let Some(root) = std::env::var_os(ROOT) {
        let root = std::path::PathBuf::from(root);
        let s = open(&root);
        let mut c = connect(&s, 7);
        let cap = grant(&s, &mut c, RIGHT_ALL);
        let a = stage(&s, &mut c, cap, br#"{"enabled":true,"label":"keep"}"#);
        assert_eq!(
            validate(
                &mut c,
                cap,
                a.candidate_cap,
                content_hash(&fixture().validator)
            )
            .outcome,
            OUTCOME_VALID
        );
        std::fs::write(root.join("client-candidate"), a.candidate_cap.to_string()).unwrap();
        if std::env::var(PHASE).unwrap() == "before" {
            std::fs::create_dir(root.join("task.next")).unwrap();
        }
        // Crash after the service dispatches but before a wire reply is sent.
        let reply = store_service::agent_task::dispatch(
            &s,
            7,
            &req(
                9,
                &CommitCandidate {
                    task_cap: cap,
                    request_id: 70,
                    candidate_cap: a.candidate_cap,
                    expected_revision: 0,
                    expected_content_id_hash: content_hash(&fixture().input),
                },
            ),
        )
        .unwrap();
        let r: CommitCandidateReply = read_payload(&reply).unwrap();
        assert_eq!(
            r.status,
            if std::env::var(PHASE).unwrap() == "before" {
                STATUS_IO
            } else {
                STATUS_OK
            }
        );
        std::process::exit(17); // Bypass all Rust destructors and service shutdown.
    }
    for phase in ["before", "after"] {
        let d = tempfile::tempdir().unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "abrupt_process_crash_before_or_after_publication_recovers_one_effect",
                "--nocapture",
            ])
            .env(ROOT, d.path())
            .env(PHASE, phase)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        let exit = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("crash fixture exceeded outer test watchdog");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert_eq!(exit.code(), Some(17));
        if phase == "before" {
            std::fs::remove_dir(d.path().join("task.next")).unwrap();
        }
        let candidate = std::fs::read_to_string(d.path().join("client-candidate"))
            .unwrap()
            .parse()
            .unwrap();
        let s = open(d.path());
        let mut c = connect(&s, 7);
        let cap = grant(&s, &mut c, RIGHT_ALL);
        if phase == "after" {
            let result = commit(
                &mut c,
                cap,
                candidate,
                0,
                content_hash(&fixture().input),
                70,
            );
            assert_eq!(result.status, STATUS_OK);
            assert_eq!(result.revision, 1);
        } else {
            assert_eq!(s.lock().unwrap().accepted_bytes().unwrap(), fixture().input);
            assert_eq!(
                commit(
                    &mut c,
                    cap,
                    candidate,
                    0,
                    content_hash(&fixture().input),
                    70
                )
                .status,
                STATUS_DENIED
            );
        }
        s.lock().unwrap().verify_audit().unwrap();
    }
}
#[test]
fn guest_and_start_section_loops_are_timed_out_without_accepting_output() {
    for start in [false, true] {
        let d = tempfile::tempdir().unwrap();
        let mut f = fixture();
        let body = if start {
            "(func $init (loop $l (br $l))) (start $init) (func (export \"_start\") (result i32) i32.const 0)"
        } else {
            "(func (export \"_start\") (result i32) (loop $l (br $l)) i32.const 0)"
        };
        f.validator =
            wat::parse_str(format!("(module (memory (export \"memory\") 3 3) {body})")).unwrap();
        f.contract.validator_id = id(&f.validator);
        f.contract.execution_budget.guest_ms = 100;
        let pin = content_hash(&f.validator);
        let worker = std::env::var_os("RAMEN_TASK_VALIDATOR_WORKER").unwrap();
        let s = Arc::new(Mutex::new(
            TaskService::open(d.path(), f, worker.into()).unwrap(),
        ));
        let mut c = connect(&s, 7);
        let cap = grant(&s, &mut c, RIGHT_ALL);
        let a = stage(&s, &mut c, cap, br#"{"enabled":true,"label":"keep"}"#);
        let result = validate(&mut c, cap, a.candidate_cap, pin);
        assert_eq!(result.outcome, OUTCOME_TIMEOUT);
        // A delayed caller can cross this fixture's 100 ms validation TTL.
        // Both classifications must reject publication of the timed-out result.
        assert!(matches!(
            commit(
                &mut c,
                cap,
                a.candidate_cap,
                0,
                content_hash(&fixture().input),
                80
            )
            .status,
            STATUS_VALIDATION_FAILED | STATUS_EXPIRED
        ));
        assert_eq!(s.lock().unwrap().accepted_bytes().unwrap(), fixture().input);
    }
}
#[test]
fn incomplete_transport_frames_have_an_absolute_deadline() {
    use std::io::{Read, Write};
    let d = tempfile::tempdir().unwrap();
    let s = open(d.path());
    let mut c = connect(&s, 7);
    c.write_all(&[14]).unwrap();
    c.set_read_timeout(Some(std::time::Duration::from_secs(3)))
        .unwrap();
    let mut reply = [0; 1];
    assert_eq!(c.read(&mut reply).unwrap(), 0);
}

#[test]
fn a_real_stalled_cas_backend_is_inside_the_validator_watchdog() {
    use std::os::unix::ffi::OsStrExt;
    let d = tempfile::tempdir().unwrap();
    let s = open(d.path());
    let mut c = connect(&s, 7);
    let cap = grant(&s, &mut c, RIGHT_ALL);
    let fixed = br#"{"enabled":true,"label":"keep"}"#;
    let a = stage(&s, &mut c, cap, fixed);
    let file = d
        .path()
        .join("cas")
        .join(format!("{}.blob", hex::encode(a.content_id_hash)));
    std::fs::remove_file(&file).unwrap();
    let name = std::ffi::CString::new(file.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let start = std::time::Instant::now();
    let result = validate(
        &mut c,
        cap,
        a.candidate_cap,
        content_hash(&fixture().validator),
    );
    assert_eq!(result.outcome, OUTCOME_TIMEOUT);
    assert_eq!(result.status, STATUS_TIMEOUT);
    assert!(start.elapsed() < std::time::Duration::from_secs(5));
    std::fs::remove_file(&file).unwrap();
    std::fs::write(&file, fixed).unwrap();
    assert_eq!(
        commit(
            &mut c,
            cap,
            a.candidate_cap,
            0,
            content_hash(&fixture().input),
            90
        )
        .status,
        STATUS_VALIDATION_FAILED
    );
    assert_eq!(s.lock().unwrap().accepted_bytes().unwrap(), fixture().input);
}
#[test]
fn the_policy_artifact_bounds_grants_and_task_notes_cannot_expand_authority() {
    let d = tempfile::tempdir().unwrap();
    let s = open(d.path());
    let mut c = connect(&s, 7);
    let cap = grant(&s, &mut c, RIGHT_ALL);
    let policy_cap = s.lock().unwrap().policy_cap();
    let denied: RequestGrantReply = read_payload(
        &exchange(
            &mut c,
            &req(
                3,
                &RequestGrant {
                    policy_cap,
                    request_id: 91,
                    task_id: 17,
                    resource_id: 1,
                    lifetime_ms: 60_001,
                    rights: RIGHT_ALL,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(denied.status, STATUS_DENIED);
    assert_eq!(denied.task_cap, 0);
    for resource_id in [
        store_service::agent_task::SCHEMA_RESOURCE,
        store_service::agent_task::NOTES_RESOURCE,
    ] {
        let r: ReadInputReply = read_payload(
            &exchange(
                &mut c,
                &req(
                    1,
                    &ReadInput {
                        task_cap: cap,
                        request_id: 92,
                        resource_id,
                    },
                ),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(r.status, 0);
        assert!(
            !s.lock()
                .unwrap()
                .read_mapping(7, r.input_shm_cap)
                .unwrap()
                .is_empty()
        );
        s.lock()
            .unwrap()
            .release_mapping(7, r.input_shm_cap)
            .unwrap();
    }
    let b_canary = d.path().join("workspace-b-canary");
    std::fs::write(&b_canary, b"private canary outside agent task").unwrap();
    let r: ReadInputReply = read_payload(
        &exchange(
            &mut c,
            &req(
                1,
                &ReadInput {
                    task_cap: cap,
                    request_id: 93,
                    resource_id: 999,
                },
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(r.status, STATUS_DENIED);
    assert_eq!(r.content_id_hash, [0; 32]);
    assert_eq!(
        std::fs::read(b_canary).unwrap(),
        b"private canary outside agent task"
    );
}

#[test]
fn worker_concurrency_is_bounded_without_exposing_load_to_foreign_holders() {
    use std::os::unix::ffi::OsStrExt;
    let d = tempfile::tempdir().unwrap();
    let s = open(d.path());
    let mut c = connect(&s, 7);
    let cap = grant(&s, &mut c, RIGHT_ALL);
    let a = stage(&s, &mut c, cap, br#"{"enabled":true,"label":"keep"}"#);
    let file = d
        .path()
        .join("cas")
        .join(format!("{}.blob", hex::encode(a.content_id_hash)));
    std::fs::remove_file(&file).unwrap();
    let name = std::ffi::CString::new(file.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let mut jobs = Vec::new();
    for _ in 0..2 {
        let mut socket = connect(&s, 7);
        jobs.push(std::thread::spawn(move || {
            validate(
                &mut socket,
                cap,
                a.candidate_cap,
                content_hash(&fixture().validator),
            )
        }));
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
    while s.lock().unwrap().active_worker_count() != 2 {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let mut foreign = connect(&s, 8);
    assert_eq!(
        validate(
            &mut foreign,
            cap,
            a.candidate_cap,
            content_hash(&fixture().validator)
        )
        .status,
        STATUS_DENIED
    );
    assert_eq!(
        validate(
            &mut c,
            cap,
            a.candidate_cap,
            content_hash(&fixture().validator)
        )
        .status,
        STATUS_CAPACITY
    );
    for job in jobs {
        assert_eq!(job.join().unwrap().outcome, OUTCOME_TIMEOUT);
    }
    assert_eq!(s.lock().unwrap().active_worker_count(), 0);
}

#[test]
fn forbidden_guest_imports_cannot_execute_even_when_the_program_is_pinned() {
    for (module, name) in [("wasi_snapshot_preview1", "sock_open"), ("task", "check")] {
        let d = tempfile::tempdir().unwrap();
        let mut f = fixture();
        f.validator=wat::parse_str(format!("(module (import \"{module}\" \"{name}\" (func $escape (result i32))) (memory (export \"memory\") 3 3) (func (export \"_start\") (result i32) call $escape))")).unwrap();
        f.contract.validator_id = id(&f.validator);
        let pin = content_hash(&f.validator);
        let worker = std::env::var_os("RAMEN_TASK_VALIDATOR_WORKER").unwrap();
        let s = Arc::new(Mutex::new(
            TaskService::open(d.path(), f, worker.into()).unwrap(),
        ));
        let mut c = connect(&s, 7);
        let cap = grant(&s, &mut c, RIGHT_ALL);
        let a = stage(&s, &mut c, cap, br#"{"enabled":true,"label":"keep"}"#);
        assert_eq!(
            validate(&mut c, cap, a.candidate_cap, pin).outcome,
            OUTCOME_HOST_FAILURE
        );
        assert_eq!(
            commit(
                &mut c,
                cap,
                a.candidate_cap,
                0,
                content_hash(&fixture().input),
                95
            )
            .status,
            STATUS_VALIDATION_FAILED
        );
    }
}
