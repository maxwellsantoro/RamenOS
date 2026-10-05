//! UI1.0 host contract acceptance. Requires the explicit desktop_v0_dev feature.
//! No Store, compositor, device, target runtime or process-containment claim.
#![cfg(feature = "desktop_v0_dev")]

use desktop_service::dev::{
    DevConfig, DevDesktop, SessionFixture, Status, WitnessMode, decode_envelope_wire,
    encode_envelope_wire,
};
use kernel_api::cap::{Handle, HandleKind};
use kernel_api::generated::desktop_session_v1 as idl;
use kernel_api::ipc::Envelope;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const OBSERVE: u32 = 1;
const TRUSTED_STATUS: u32 = 8;
const OK: u32 = 0;
const DENIED: u32 = 1;
const INVALID: u32 = 2;
const UNSUPPORTED: u32 = 3;
const STALE: u32 = 4;
const RUNNING: u32 = 2;
const FAULTED: u32 = 4;
const REVOKED: u32 = 5;
const EXPIRED: u32 = 6;

fn witness_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_desktop-launch-witness"))
}

fn witness_hash() -> [u8; 32] {
    // The Cargo-built witness is an immutable test fixture. Cache its expected
    // identity before launch measurements; DevDesktop still independently reads,
    // hashes and verifies the bytes copied to each private executable snapshot.
    // Replacement tests mutate only their own copied fixture, never this binary.
    static PINNED_WITNESS_HASH: OnceLock<[u8; 32]> = OnceLock::new();
    *PINNED_WITNESS_HASH.get_or_init(|| {
        Sha256::digest(std::fs::read(witness_path()).expect("built Rust witness executable")).into()
    })
}

fn desktop(mode: WitnessMode) -> DevDesktop {
    desktop_with_deadline(mode, 5_000)
}

fn desktop_with_deadline(mode: WitnessMode, deadline: u64) -> DevDesktop {
    DevDesktop::new(DevConfig {
        witness_path: witness_path(),
        pinned_sha256: witness_hash(),
        preview_ttl_ms: 100,
        instance_ttl_ms: 1_000,
        child_deadline_ms: deadline,
        witness_mode: mode,
    })
    .expect("bounded default-off host fixture")
}

fn session(service: &mut DevDesktop, owner: u64) -> SessionFixture {
    service
        .register_session(owner, [0x4a; 32], 1)
        .expect("independent fixture session")
}

// Construct generated request values, then initialize every wire byte field by
// field. repr(C) memory/padding and generic memcpy are deliberately not codecs.
struct Payload {
    envelope: Envelope,
    cursor: usize,
}

impl Payload {
    fn new(kind: u32, handle: Handle) -> Self {
        let mut envelope = Envelope::empty(idl::DESKTOP_SESSION_V1_PROTOCOL_ID, kind);
        envelope.handle = handle;
        Self {
            envelope,
            cursor: 0,
        }
    }
    fn bytes(&mut self, bytes: &[u8]) {
        let end = self.cursor.checked_add(bytes.len()).unwrap();
        assert!(end <= 64);
        self.envelope.payload[self.cursor..end].copy_from_slice(bytes);
        self.cursor = end;
    }
    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }
    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }
    fn finish(mut self) -> Envelope {
        self.envelope.payload_len = self.cursor as u32;
        self.envelope
    }
}

fn prepare_request(s: &SessionFixture, rights: u32) -> Envelope {
    let value = idl::PrepareLaunch {
        request_id: 11,
        session_id: s.session_id,
        session_generation: s.session_generation,
        application_hash: witness_hash(),
        requested_rights: rights,
        reserved: 0,
    };
    let mut p = Payload::new(idl::MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH, s.trusted_handle);
    p.u64(value.request_id);
    p.u64(value.session_id);
    p.u64(value.session_generation);
    p.bytes(&value.application_hash);
    p.u32(value.requested_rights);
    p.u32(value.reserved);
    p.finish()
}

fn confirm_request(s: &SessionFixture, plan: &idl::PrepareLaunchReply) -> Envelope {
    let value = idl::ConfirmLaunch {
        request_id: 12,
        session_id: s.session_id,
        session_generation: s.session_generation,
        plan_id: plan.plan_id,
        preview_revision: plan.preview_revision,
    };
    let mut p = Payload::new(idl::MSG_DESKTOP_SESSION_V1_CONFIRM_LAUNCH, s.trusted_handle);
    p.u64(value.request_id);
    p.u64(value.session_id);
    p.u64(value.session_generation);
    p.u64(value.plan_id);
    p.u64(value.preview_revision);
    p.finish()
}

fn cancel_request(s: &SessionFixture, plan: &idl::PrepareLaunchReply) -> Envelope {
    let value = idl::CancelPreview {
        request_id: 13,
        session_id: s.session_id,
        session_generation: s.session_generation,
        plan_id: plan.plan_id,
    };
    let mut p = Payload::new(idl::MSG_DESKTOP_SESSION_V1_CANCEL_PREVIEW, s.trusted_handle);
    p.u64(value.request_id);
    p.u64(value.session_id);
    p.u64(value.session_generation);
    p.u64(value.plan_id);
    p.finish()
}

fn instance_request(kind: u32, s: &SessionFixture, instance: &idl::ConfirmLaunchReply) -> Envelope {
    let fields = match kind {
        idl::MSG_DESKTOP_SESSION_V1_GET_STATUS => {
            let v = idl::GetStatus {
                request_id: 14,
                session_id: s.session_id,
                session_generation: s.session_generation,
                instance_id: instance.instance_id,
                instance_generation: instance.instance_generation,
            };
            [
                v.request_id,
                v.session_id,
                v.session_generation,
                v.instance_id,
                v.instance_generation,
            ]
        }
        idl::MSG_DESKTOP_SESSION_V1_CLOSE_INSTANCE => {
            let v = idl::CloseInstance {
                request_id: 15,
                session_id: s.session_id,
                session_generation: s.session_generation,
                instance_id: instance.instance_id,
                instance_generation: instance.instance_generation,
            };
            [
                v.request_id,
                v.session_id,
                v.session_generation,
                v.instance_id,
                v.instance_generation,
            ]
        }
        idl::MSG_DESKTOP_SESSION_V1_REVOKE_INSTANCE => {
            let v = idl::RevokeInstance {
                request_id: 16,
                session_id: s.session_id,
                session_generation: s.session_generation,
                instance_id: instance.instance_id,
                instance_generation: instance.instance_generation,
            };
            [
                v.request_id,
                v.session_id,
                v.session_generation,
                v.instance_id,
                v.instance_generation,
            ]
        }
        idl::MSG_DESKTOP_SESSION_V1_PREPARE_RESTART => {
            let v = idl::PrepareRestart {
                request_id: 17,
                session_id: s.session_id,
                session_generation: s.session_generation,
                instance_id: instance.instance_id,
                instance_generation: instance.instance_generation,
            };
            [
                v.request_id,
                v.session_id,
                v.session_generation,
                v.instance_id,
                v.instance_generation,
            ]
        }
        _ => panic!("unregistered test request kind"),
    };
    let mut p = Payload::new(kind, s.trusted_handle);
    for field in fields {
        p.u64(field);
    }
    p.finish()
}

fn observe_request(s: &SessionFixture, instance: &idl::ConfirmLaunchReply) -> Envelope {
    let value = idl::ObserveInstance {
        request_id: 18,
        session_id: s.session_id,
        session_generation: s.session_generation,
        instance_id: instance.instance_id,
        instance_generation: instance.instance_generation,
    };
    let mut p = Payload::new(
        idl::MSG_DESKTOP_SESSION_V1_OBSERVE_INSTANCE,
        Handle::unpack(instance.observation_handle),
    );
    for value in [
        value.request_id,
        value.session_id,
        value.session_generation,
        value.instance_id,
        value.instance_generation,
    ] {
        p.u64(value);
    }
    p.finish()
}

fn u64_at(env: &Envelope, offset: usize) -> u64 {
    u64::from_le_bytes(env.payload[offset..offset + 8].try_into().unwrap())
}
fn u32_at(env: &Envelope, offset: usize) -> u32 {
    u32::from_le_bytes(env.payload[offset..offset + 4].try_into().unwrap())
}
fn canonical_reply(env: &Envelope, kind: u32, length: u32) {
    assert_eq!(env.protocol, idl::DESKTOP_SESSION_V1_PROTOCOL_ID);
    assert_eq!(env.msg_type, kind);
    assert_eq!(env.payload_len, length);
    assert!(env.payload[length as usize..].iter().all(|b| *b == 0));
    let bytes = encode_envelope_wire(env).expect("canonical reply encoding");
    let roundtrip = decode_envelope_wire(&bytes).expect("canonical reply decoding");
    assert_eq!(roundtrip.payload, env.payload);
    assert_eq!(roundtrip.handle, env.handle);
}
fn preview_reply(env: &Envelope, kind: u32) -> idl::PrepareLaunchReply {
    canonical_reply(env, kind, 48);
    assert_eq!(u32_at(env, 44), 0);
    let reply = idl::PrepareLaunchReply {
        request_id: u64_at(env, 0),
        plan_id: u64_at(env, 8),
        preview_revision: u64_at(env, 16),
        preview_shm: u64_at(env, 24),
        preview_len: u32_at(env, 32),
        status: u32_at(env, 36),
        granted_rights: u32_at(env, 40),
        reserved: u32_at(env, 44),
    };
    if reply.status == DENIED {
        assert_eq!(
            (reply.plan_id, reply.preview_revision, reply.preview_shm),
            (0, 0, 0)
        );
        assert_eq!((reply.preview_len, reply.granted_rights), (0, 0));
    }
    reply
}
fn launch_reply(env: &Envelope) -> idl::ConfirmLaunchReply {
    canonical_reply(env, idl::MSG_DESKTOP_SESSION_V1_CONFIRM_LAUNCH_REPLY, 48);
    assert_eq!(u32_at(env, 44), 0);
    let reply = idl::ConfirmLaunchReply {
        request_id: u64_at(env, 0),
        instance_id: u64_at(env, 8),
        instance_generation: u64_at(env, 16),
        observation_handle: u64_at(env, 24),
        granted_rights: u32_at(env, 32),
        status: u32_at(env, 36),
        child_pid: u32_at(env, 40),
        reserved: u32_at(env, 44),
    };
    if reply.status == DENIED {
        assert_eq!(
            (
                reply.instance_id,
                reply.instance_generation,
                reply.observation_handle
            ),
            (0, 0, 0)
        );
        assert_eq!((reply.granted_rights, reply.child_pid), (0, 0));
    }
    reply
}
fn status_reply(env: &Envelope, kind: u32) -> idl::GetStatusReply {
    canonical_reply(env, kind, 64);
    let reply = idl::GetStatusReply {
        request_id: u64_at(env, 0),
        instance_id: u64_at(env, 8),
        instance_generation: u64_at(env, 16),
        executable_hash: env.payload[24..56].try_into().unwrap(),
        state: u32_at(env, 56),
        status: u32_at(env, 60),
    };
    if reply.status == DENIED {
        assert_eq!(
            (reply.instance_id, reply.instance_generation, reply.state),
            (0, 0, 0)
        );
        assert_eq!(reply.executable_hash, [0; 32]);
    }
    reply
}
fn short_status(env: &Envelope, kind: u32) -> u32 {
    canonical_reply(env, kind, 16);
    assert_eq!(u32_at(env, 12), 0);
    u32_at(env, 8)
}
fn get_status(
    service: &mut DevDesktop,
    s: &SessionFixture,
    child: &idl::ConfirmLaunchReply,
    now: u64,
) -> idl::GetStatusReply {
    let reply = status_reply(
        &service.dispatch(
            &s.trusted_peer,
            &instance_request(idl::MSG_DESKTOP_SESSION_V1_GET_STATUS, s, child),
            now,
        ),
        idl::MSG_DESKTOP_SESSION_V1_GET_STATUS_REPLY,
    );
    record_child_evidence(service, child.instance_id);
    reply
}
fn prepare(service: &mut DevDesktop, s: &SessionFixture, now: u64) -> idl::PrepareLaunchReply {
    let result = preview_reply(
        &service.dispatch(&s.trusted_peer, &prepare_request(s, OBSERVE), now),
        idl::MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH_REPLY,
    );
    assert_eq!(result.request_id, 11);
    assert_eq!(result.status, OK);
    assert_ne!(result.plan_id, 0);
    assert_eq!(result.granted_rights, OBSERVE);
    assert_eq!(result.preview_len, 136);
    result
}
fn confirm(
    service: &mut DevDesktop,
    s: &SessionFixture,
    plan: &idl::PrepareLaunchReply,
    now: u64,
) -> idl::ConfirmLaunchReply {
    service
        .inject_chrome_confirmation(
            &s.trusted_peer,
            s.session_id,
            s.session_generation,
            plan.plan_id,
            plan.preview_revision,
            now,
        )
        .expect("explicit synthetic trusted Enter press");
    let result = launch_reply(&service.dispatch(&s.trusted_peer, &confirm_request(s, plan), now));
    assert_eq!(result.request_id, 12);
    assert_eq!(result.status, OK);
    assert_ne!(result.child_pid, 0);
    assert_ne!(result.child_pid, std::process::id());
    assert_ne!(result.observation_handle, 0);
    assert_eq!(result.granted_rights, OBSERVE);
    record_spawn(&result);
    // The independent watchdog case must establish disappearance through OS
    // probing before any evidence accessor can drive lazy service progress.
    if std::thread::current().name()
        != Some("ui1_0_absolute_watchdog_reaps_stalled_child_without_dispatch_or_maintenance")
    {
        record_child_evidence(service, result.instance_id);
    }
    result
}
fn launch(service: &mut DevDesktop, s: &SessionFixture) -> idl::ConfirmLaunchReply {
    let plan = prepare(service, s, 0);
    confirm(service, s, &plan, 1)
}
fn await_observation(service: &mut DevDesktop, child: &idl::ConfirmLaunchReply) {
    await_observation_at(service, child, 2);
}
fn await_observation_at(service: &mut DevDesktop, child: &idl::ConfirmLaunchReply, now: u64) {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if service
            .child_evidence(child.instance_id)
            .is_some_and(|e| e.observation_count == 1 && e.exchanges.len() >= 5)
        {
            record_child_evidence(service, child.instance_id);
            return;
        }
        assert!(
            Instant::now() < deadline,
            "actual child did not complete typed observation/denial probes"
        );
        service.maintenance(now);
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn assert_reaped(service: &DevDesktop, child: &idl::ConfirmLaunchReply) {
    let evidence = service
        .child_evidence(child.instance_id)
        .expect("retained child evidence");
    assert!(
        evidence.reaped,
        "retirement must reap child, not just revoke a registry handle"
    );
    assert!(
        !process_alive(child.child_pid),
        "retired child still exists"
    );
    record_child_evidence(service, child.instance_id);
}

fn process_alive(pid: u32) -> bool {
    Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .output()
        .expect("host process-existence probe")
        .status
        .success()
}

// The coordinator reserves this optional gate output root. No evidence IO is
// performed during ordinary tests. Per-case/PID directories prevent parallel
// tests from overwriting each other's actual producer/consumer observations.
fn evidence_directory(pid: u32) -> Option<PathBuf> {
    let root = std::env::var_os("RAMEN_DESKTOP_GATE_EVIDENCE")?;
    let current = std::thread::current();
    let name = current.name().expect("named acceptance-test thread");
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    assert!(safe.starts_with("ui1_0_") && safe.len() <= 160);
    let path = PathBuf::from(root).join(format!("{safe}-{pid}"));
    std::fs::create_dir_all(&path).expect("reserved per-case evidence directory");
    Some(path)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn record_spawn(child: &idl::ConfirmLaunchReply) {
    let Some(path) = evidence_directory(child.child_pid) else {
        return;
    };
    let record = format!(
        "{{\"schema_version\":1,\"scope\":\"default-off-host-fixture\",\"source\":\"confirm-reply\",\"child_pid\":{},\"instance_id\":{},\"instance_generation\":{},\"observation_handle\":{},\"granted_rights\":{},\"pinned_executable_sha256\":\"{}\"}}\n",
        child.child_pid,
        child.instance_id,
        child.instance_generation,
        child.observation_handle,
        child.granted_rights,
        hex(&witness_hash())
    );
    std::fs::write(path.join("spawn.json"), record).expect("retained launch evidence");
}

fn record_child_evidence(service: &DevDesktop, instance_id: u64) {
    if std::env::var_os("RAMEN_DESKTOP_GATE_EVIDENCE").is_none() {
        return;
    }
    let evidence = service
        .child_evidence(instance_id)
        .expect("actual owned-child evidence");
    let path = evidence_directory(evidence.actual_pid).unwrap();
    assert!(
        evidence.exchanges.len() <= 64,
        "gate evidence itself must remain bounded"
    );
    let exchanges: Vec<String> = evidence
        .exchanges
        .iter()
        .map(|(request, reply)| {
            let request = encode_envelope_wire(request).expect("canonical actual child request");
            let reply = encode_envelope_wire(reply).expect("canonical actual parent reply");
            format!(
                "{{\"request_hex\":\"{}\",\"reply_hex\":\"{}\"}}",
                hex(&request),
                hex(&reply)
            )
        })
        .collect();
    let b = &evidence.bootstrap;
    let record = format!(
        "{{\"schema_version\":1,\"scope\":\"default-off-host-fixture\",\"source\":\"actual-owned-child-pipe\",\"actual_pid\":{},\"executable_sha256\":\"{}\",\"session_id\":{},\"session_generation\":{},\"instance_id\":{},\"instance_generation\":{},\"observation_handle\":{},\"granted_rights\":{},\"bootstrap_reserved\":{},\"observation_count\":{},\"exchange_limit_hit\":{},\"reaped\":{},\"exchanges\":[{}]}}\n",
        evidence.actual_pid,
        hex(&evidence.executable_sha256),
        b.session_id,
        b.session_generation,
        b.instance_id,
        b.instance_generation,
        b.observation_handle,
        b.granted_rights,
        b.reserved,
        evidence.observation_count,
        evidence.exchange_limit_hit,
        evidence.reaped,
        exchanges.join(",")
    );
    std::fs::write(path.join("child.json"), record).expect("retained bounded child transcript");
}

fn record_independent_disappearance(child: &idl::ConfirmLaunchReply, cause: &str) {
    let Some(path) = evidence_directory(child.child_pid) else {
        return;
    };
    assert!(matches!(cause, "drop" | "absolute-watchdog"));
    let record = format!(
        "{{\"schema_version\":1,\"scope\":\"default-off-host-fixture\",\"child_pid\":{},\"instance_id\":{},\"cause\":\"{}\",\"kill_zero_process_absent\":true}}\n",
        child.child_pid, child.instance_id, cause
    );
    std::fs::write(path.join("independent-disappearance.json"), record)
        .expect("OS-observed disappearance evidence");
}

#[test]
fn ui1_0_approve_runs_pinned_child_with_exact_observation_grant() {
    let mut service = desktop(WitnessMode::Observe);
    let s = session(&mut service, 10);
    let plan = prepare(&mut service, &s, 0);
    let preview = service
        .preview_snapshot(&s.trusted_peer, plan.preview_shm)
        .unwrap();
    assert_eq!(preview.schema_version, 1);
    assert_eq!(preview.application_hash, witness_hash());
    assert_eq!(preview.selected_fixture_hash, [0x4a; 32]);
    assert_eq!(preview.selected_revision, 1);
    assert_eq!(preview.session_id, s.session_id);
    assert_eq!(preview.session_generation, s.session_generation);
    assert_eq!(preview.plan_id, plan.plan_id);
    assert_eq!(preview.preview_revision, plan.preview_revision);
    assert_eq!(preview.granted_rights, OBSERVE);
    assert_eq!(preview.expires_at_ms, 100);
    let child = confirm(&mut service, &s, &plan, 1);
    await_observation(&mut service, &child);
    let evidence = service.child_evidence(child.instance_id).unwrap();
    assert_eq!(evidence.actual_pid, child.child_pid);
    assert_eq!(evidence.executable_sha256, preview.application_hash);
    assert_eq!(evidence.bootstrap.session_id, s.session_id);
    assert_eq!(evidence.bootstrap.session_generation, s.session_generation);
    assert_eq!(evidence.bootstrap.instance_id, child.instance_id);
    assert_eq!(
        evidence.bootstrap.instance_generation,
        child.instance_generation
    );
    assert_eq!(
        evidence.bootstrap.observation_handle,
        child.observation_handle
    );
    assert_eq!(evidence.bootstrap.granted_rights, preview.granted_rights);
    assert_eq!(evidence.bootstrap.reserved, 0);
    assert_eq!(get_status(&mut service, &s, &child, 2).state, RUNNING);
}

#[test]
fn ui1_0_cancel_and_single_use_confirmation_do_not_spawn_twice() {
    let mut service = desktop(WitnessMode::Observe);
    let s = session(&mut service, 10);
    let canceled = prepare(&mut service, &s, 0);
    service
        .inject_chrome_confirmation(
            &s.trusted_peer,
            s.session_id,
            s.session_generation,
            canceled.plan_id,
            canceled.preview_revision,
            0,
        )
        .unwrap();
    assert_eq!(
        short_status(
            &service.dispatch(&s.trusted_peer, &cancel_request(&s, &canceled), 1),
            idl::MSG_DESKTOP_SESSION_V1_CANCEL_PREVIEW_REPLY
        ),
        OK
    );
    let denied =
        launch_reply(&service.dispatch(&s.trusted_peer, &confirm_request(&s, &canceled), 2));
    assert_eq!(denied.status, STALE);
    assert_eq!(
        (
            denied.instance_id,
            denied.observation_handle,
            denied.child_pid
        ),
        (0, 0, 0)
    );
    let fresh = prepare(&mut service, &s, 3);
    let child = confirm(&mut service, &s, &fresh, 4);
    let replay = launch_reply(&service.dispatch(&s.trusted_peer, &confirm_request(&s, &fresh), 5));
    assert_eq!(replay.status, STALE);
    assert_eq!(
        (
            replay.instance_id,
            replay.observation_handle,
            replay.child_pid
        ),
        (0, 0, 0)
    );
    assert_eq!(
        get_status(&mut service, &s, &child, 6).instance_id,
        child.instance_id
    );
    await_observation_at(&mut service, &child, 6);
}

#[test]
fn ui1_0_confirmation_requires_one_trusted_generation_bound_event() {
    let mut service = desktop(WitnessMode::Observe);
    let a = session(&mut service, 10);
    let b = session(&mut service, 20);
    let plan = prepare(&mut service, &a, 0);
    let no_event = launch_reply(&service.dispatch(&a.trusted_peer, &confirm_request(&a, &plan), 1));
    assert_eq!(no_event.status, DENIED);
    assert_eq!(
        (
            no_event.instance_id,
            no_event.observation_handle,
            no_event.child_pid
        ),
        (0, 0, 0)
    );
    assert!(matches!(
        service.inject_chrome_confirmation(
            &b.trusted_peer,
            a.session_id,
            a.session_generation,
            plan.plan_id,
            plan.preview_revision,
            1
        ),
        Err(Status::Denied)
    ));
    let (no_confirm, _) = service
        .issue_trusted_fixture_peer(10, a.session_id, TRUSTED_STATUS)
        .unwrap();
    assert!(matches!(
        service.inject_chrome_confirmation(
            &no_confirm,
            a.session_id,
            a.session_generation,
            plan.plan_id,
            plan.preview_revision,
            1
        ),
        Err(Status::Denied)
    ));
    assert!(matches!(
        service.inject_chrome_confirmation(
            &a.trusted_peer,
            a.session_id,
            a.session_generation + 1,
            plan.plan_id,
            plan.preview_revision,
            1
        ),
        Err(Status::Stale)
    ));
    let child = confirm(&mut service, &a, &plan, 2);
    let replay = launch_reply(&service.dispatch(&a.trusted_peer, &confirm_request(&a, &plan), 3));
    assert_eq!(replay.status, STALE);
    assert_eq!(
        (
            replay.instance_id,
            replay.observation_handle,
            replay.child_pid
        ),
        (0, 0, 0)
    );
    await_observation_at(&mut service, &child, 3);
}

#[test]
fn ui1_0_actual_child_cannot_launch_restart_or_observe_foreign_instance() {
    let mut service = desktop(WitnessMode::Observe);
    let s = session(&mut service, 10);
    let child = launch(&mut service, &s);
    await_observation(&mut service, &child);
    let foreign_session = session(&mut service, 20);
    let foreign_plan = prepare(&mut service, &foreign_session, 2);
    let foreign_child = confirm(&mut service, &foreign_session, &foreign_plan, 3);
    await_observation_at(&mut service, &foreign_child, 3);
    assert_eq!(
        get_status(&mut service, &foreign_session, &foreign_child, 3).state,
        RUNNING
    );
    // Captured opaque child A identity exercises the same enforcing path as the
    // owned live pipe, targeting the real running B instance and B endpoint.
    let foreign = observe_request(&foreign_session, &foreign_child);
    let denied = status_reply(
        &service.dispatch_recorded_child(child.instance_id, &foreign, 3),
        idl::MSG_DESKTOP_SESSION_V1_OBSERVE_INSTANCE_REPLY,
    );
    assert_eq!(denied.status, DENIED);
    let mut foreign_with_own_handle = foreign;
    foreign_with_own_handle.handle = Handle::unpack(child.observation_handle);
    let denied = status_reply(
        &service.dispatch_recorded_child(child.instance_id, &foreign_with_own_handle, 3),
        idl::MSG_DESKTOP_SESSION_V1_OBSERVE_INSTANCE_REPLY,
    );
    assert_eq!(denied.status, DENIED);
    assert_eq!(
        get_status(&mut service, &foreign_session, &foreign_child, 3).status,
        OK
    );
    assert_eq!(
        get_status(&mut service, &foreign_session, &foreign_child, 3).state,
        RUNNING
    );
    let evidence = service.child_evidence(child.instance_id).unwrap();
    for kind in [
        idl::MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH,
        idl::MSG_DESKTOP_SESSION_V1_CONFIRM_LAUNCH,
        idl::MSG_DESKTOP_SESSION_V1_PREPARE_RESTART,
    ] {
        let (request, reply) = evidence
            .exchanges
            .iter()
            .find(|(r, _)| r.msg_type == kind)
            .expect("actual child privileged probe");
        assert_eq!(request.handle.pack(), child.observation_handle);
        let denied = preview_or_launch_status(reply);
        assert_eq!(denied, DENIED);
    }
    let observations: Vec<_> = evidence
        .exchanges
        .iter()
        .filter(|(r, _)| r.msg_type == idl::MSG_DESKTOP_SESSION_V1_OBSERVE_INSTANCE)
        .collect();
    assert_eq!(observations.len(), 2);
    assert!(
        observations
            .iter()
            .any(|(r, p)| u64_at(r, 8) == s.session_id
                && status_reply(p, idl::MSG_DESKTOP_SESSION_V1_OBSERVE_INSTANCE_REPLY).status
                    == OK)
    );
    assert!(
        observations
            .iter()
            .any(|(r, p)| u64_at(r, 8) != s.session_id
                && status_reply(p, idl::MSG_DESKTOP_SESSION_V1_OBSERVE_INSTANCE_REPLY).status
                    == DENIED)
    );
    for (_, reply) in observations
        .iter()
        .filter(|(r, _)| u64_at(r, 8) != s.session_id)
    {
        let denied = status_reply(reply, idl::MSG_DESKTOP_SESSION_V1_OBSERVE_INSTANCE_REPLY);
        assert_eq!(
            (denied.instance_id, denied.instance_generation, denied.state),
            (0, 0, 0)
        );
        assert_eq!(denied.executable_hash, [0; 32]);
    }
    assert_eq!(get_status(&mut service, &s, &child, 3).state, RUNNING);
    record_child_evidence(&service, child.instance_id);
}

fn preview_or_launch_status(reply: &Envelope) -> u32 {
    match reply.msg_type {
        idl::MSG_DESKTOP_SESSION_V1_CONFIRM_LAUNCH_REPLY => launch_reply(reply).status,
        idl::MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH_REPLY
        | idl::MSG_DESKTOP_SESSION_V1_PREPARE_RESTART_REPLY => {
            preview_reply(reply, reply.msg_type).status
        }
        _ => panic!("wrong probe reply type"),
    }
}

#[test]
fn ui1_0_peer_owner_endpoint_right_kind_and_session_generation_are_enforced() {
    let mut service = desktop(WitnessMode::Observe);
    let a = session(&mut service, 10);
    let b = session(&mut service, 20);
    let request = prepare_request(&a, OBSERVE);
    assert_eq!(
        preview_reply(
            &service.dispatch(&b.trusted_peer, &request, 0),
            idl::MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH_REPLY
        )
        .status,
        DENIED
    );
    let (restricted, handle) = service
        .issue_trusted_fixture_peer(10, a.session_id, TRUSTED_STATUS)
        .unwrap();
    let mut no_right = request;
    no_right.handle = handle;
    assert_eq!(
        preview_reply(
            &service.dispatch(&restricted, &no_right, 0),
            idl::MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH_REPLY
        )
        .status,
        DENIED
    );
    let mut forged_kind = request;
    forged_kind.handle.kind = HandleKind::Shmem;
    assert_eq!(
        preview_reply(
            &service.dispatch(&a.trusted_peer, &forged_kind, 0),
            idl::MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH_REPLY
        )
        .status,
        DENIED
    );
    let mut wrong_handle = request;
    wrong_handle.handle = Handle::INVALID;
    assert_eq!(
        preview_reply(
            &service.dispatch(&a.trusted_peer, &wrong_handle, 0),
            idl::MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH_REPLY
        )
        .status,
        DENIED
    );
    let mut stale = request;
    stale.payload[16..24].copy_from_slice(&(a.session_generation + 1).to_le_bytes());
    assert_eq!(
        preview_reply(
            &service.dispatch(&a.trusted_peer, &stale, 0),
            idl::MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH_REPLY
        )
        .status,
        STALE
    );
    let child = launch(&mut service, &a);
    await_observation(&mut service, &child);
    assert_eq!(get_status(&mut service, &a, &child, 2).status, OK);
    let mut spoofed = instance_request(idl::MSG_DESKTOP_SESSION_V1_GET_STATUS, &a, &child);
    spoofed.handle = b.trusted_handle;
    assert_eq!(
        status_reply(
            &service.dispatch(&b.trusted_peer, &spoofed, 2),
            idl::MSG_DESKTOP_SESSION_V1_GET_STATUS_REPLY
        )
        .status,
        DENIED
    );
    let owned_preview = prepare(&mut service, &b, 2);
    assert!(
        service
            .preview_snapshot(&b.trusted_peer, owned_preview.preview_shm)
            .is_ok()
    );
    assert!(matches!(
        service.preview_snapshot(&a.trusted_peer, owned_preview.preview_shm),
        Err(Status::Denied)
    ));
}

#[test]
fn ui1_0_preview_expiry_policy_and_selected_identity_changes_fail_closed() {
    for change in 0..3 {
        let mut service = desktop(WitnessMode::Observe);
        let s = session(&mut service, 10);
        let old = prepare(&mut service, &s, 0);
        service
            .inject_chrome_confirmation(
                &s.trusted_peer,
                s.session_id,
                s.session_generation,
                old.plan_id,
                old.preview_revision,
                0,
            )
            .unwrap();
        let now = match change {
            0 => 100,
            1 => {
                service.update_policy_revision(s.session_id, 2).unwrap();
                1
            }
            _ => {
                service
                    .update_selected_identity(s.session_id, [0x4b; 32], 2)
                    .unwrap();
                1
            }
        };
        let result =
            launch_reply(&service.dispatch(&s.trusted_peer, &confirm_request(&s, &old), now));
        assert_eq!(result.status, STALE);
        assert_eq!(
            (
                result.instance_id,
                result.observation_handle,
                result.child_pid
            ),
            (0, 0, 0)
        );
        let fresh = prepare(&mut service, &s, now + 1);
        assert_ne!(fresh.plan_id, old.plan_id);
        let child = confirm(&mut service, &s, &fresh, now + 2);
        await_observation_at(&mut service, &child, now + 2);
    }

    // One authorized session advances the shared monotonic fixture clock. A
    // second holder cannot revive its expired plan by supplying an earlier time.
    let mut service = desktop(WitnessMode::Observe);
    let a = session(&mut service, 10);
    let b = session(&mut service, 20);
    let old = prepare(&mut service, &a, 0);
    let b_preview = prepare(&mut service, &b, 100);
    assert!(matches!(
        service.preview_snapshot(&a.trusted_peer, old.preview_shm),
        Err(Status::Stale)
    ));
    assert!(matches!(
        service.inject_chrome_confirmation(
            &a.trusted_peer,
            a.session_id,
            a.session_generation,
            old.plan_id,
            old.preview_revision,
            1
        ),
        Err(Status::Stale)
    ));
    let stale = launch_reply(&service.dispatch(&a.trusted_peer, &confirm_request(&a, &old), 1));
    assert_eq!(stale.status, STALE);
    assert_eq!(
        (
            stale.instance_id,
            stale.observation_handle,
            stale.child_pid,
            stale.granted_rights
        ),
        (0, 0, 0, 0)
    );
    assert_eq!(
        short_status(
            &service.dispatch(&b.trusted_peer, &cancel_request(&b, &b_preview), 1),
            idl::MSG_DESKTOP_SESSION_V1_CANCEL_PREVIEW_REPLY
        ),
        OK
    );

    // Both preview and instance TTLs start at effective registry time, not the
    // stale input value. A's lease therefore expires at 1100, not 1001.
    let fresh = prepare(&mut service, &a, 1);
    assert_eq!(
        service
            .preview_snapshot(&a.trusted_peer, fresh.preview_shm)
            .unwrap()
            .expires_at_ms,
        200
    );
    let first = confirm(&mut service, &a, &fresh, 1);
    await_observation_at(&mut service, &first, 100);
    service.maintenance(1_099);
    assert_eq!(get_status(&mut service, &a, &first, 1).state, RUNNING);
    let b_preview = prepare(&mut service, &b, 1_100);
    let denied = status_reply(
        &service.dispatch_recorded_child(first.instance_id, &observe_request(&a, &first), 1),
        idl::MSG_DESKTOP_SESSION_V1_OBSERVE_INSTANCE_REPLY,
    );
    assert_eq!(
        denied.status, DENIED,
        "captured child clocks must also be clamped under the registry lock"
    );
    let deadline = Instant::now() + Duration::from_secs(2);
    while !service.child_evidence(first.instance_id).unwrap().reaped {
        assert!(
            Instant::now() < deadline,
            "expired child not independently reaped"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_reaped(&service, &first);

    // Maintenance must likewise honor the greatest clock already observed.
    let second = confirm(&mut service, &b, &b_preview, 1);
    await_observation_at(&mut service, &second, 1_100);
    let _advances_clock = prepare(&mut service, &a, 2_100);
    service.maintenance(1);
    assert_eq!(get_status(&mut service, &b, &second, 1).state, EXPIRED);
    assert_reaped(&service, &second);
}

#[test]
fn ui1_0_unknown_rights_and_executable_identity_mismatch_never_launch() {
    let mut service = desktop(WitnessMode::Observe);
    let s = session(&mut service, 10);
    for rights in [0, 2, OBSERVE | 2, u32::MAX] {
        let result = preview_reply(
            &service.dispatch(&s.trusted_peer, &prepare_request(&s, rights), 0),
            idl::MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH_REPLY,
        );
        assert_eq!(result.status, DENIED);
        assert_eq!(
            (result.plan_id, result.preview_shm, result.granted_rights),
            (0, 0, 0)
        );
    }
    let mut mismatch = prepare_request(&s, OBSERVE);
    mismatch.payload[24] ^= 1;
    let result = preview_reply(
        &service.dispatch(&s.trusted_peer, &mismatch, 0),
        idl::MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH_REPLY,
    );
    assert_eq!(result.status, DENIED);
    assert_eq!((result.plan_id, result.preview_shm), (0, 0));
    let child = launch(&mut service, &s);
    await_observation_at(&mut service, &child, 2);
}

#[test]
fn ui1_0_replaced_executable_is_rejected_before_private_snapshot_launch() {
    // A uniquely owned directory avoids editing the built witness or another
    // agent's artifact. The replacement happens before the service snapshots it.
    struct OwnedFixture(PathBuf);
    impl Drop for OwnedFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let path = std::env::temp_dir().join(format!("ramen-ui1-identity-{}", std::process::id()));
    std::fs::create_dir(&path).expect("exclusive temporary fixture directory");
    let owned = OwnedFixture(path);
    let executable = owned.0.join("witness");
    let pinned = witness_hash();
    std::fs::copy(witness_path(), &executable).unwrap();
    let mut changed = std::fs::read(&executable).unwrap();
    changed.extend_from_slice(b"identity-replacement");
    std::fs::write(&executable, changed).unwrap();
    assert!(matches!(
        DevDesktop::new(DevConfig {
            witness_path: executable,
            pinned_sha256: pinned,
            preview_ttl_ms: 100,
            instance_ttl_ms: 1_000,
            child_deadline_ms: 5_000,
            witness_mode: WitnessMode::Observe,
        }),
        Err(Status::Denied) | Err(Status::Invalid)
    ));
    let mut service = desktop(WitnessMode::Observe);
    let s = session(&mut service, 10);
    let child = launch(&mut service, &s);
    await_observation(&mut service, &child);
}

#[test]
fn ui1_0_config_time_and_shared_capacity_exhaustion_fail_closed() {
    for (preview, instance, deadline) in [
        (0, 1_000, 5_000),
        (30_001, 1_000, 5_000),
        (100, 0, 5_000),
        (100, 600_001, 5_000),
        (100, 1_000, 0),
        (100, 1_000, 5_001),
    ] {
        assert!(matches!(
            DevDesktop::new(DevConfig {
                witness_path: witness_path(),
                pinned_sha256: witness_hash(),
                preview_ttl_ms: preview,
                instance_ttl_ms: instance,
                child_deadline_ms: deadline,
                witness_mode: WitnessMode::Observe,
            }),
            Err(Status::Invalid)
        ));
    }
    let mut overflow_service = desktop(WitnessMode::Observe);
    let overflow_session = session(&mut overflow_service, 10);
    let overflow = preview_reply(
        &overflow_service.dispatch(
            &overflow_session.trusted_peer,
            &prepare_request(&overflow_session, OBSERVE),
            u64::MAX - 50,
        ),
        idl::MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH_REPLY,
    );
    assert_eq!(overflow.status, INVALID);
    assert_eq!((overflow.plan_id, overflow.preview_shm), (0, 0));
    let mut service = desktop(WitnessMode::Observe);
    let a = session(&mut service, 10);
    let b = session(&mut service, 20);
    assert!(matches!(
        service.register_session(30, [0x4c; 32], 1),
        Err(Status::Exhausted)
    ));
    let first = launch(&mut service, &a);
    await_observation(&mut service, &first);
    let exhausted = preview_reply(
        &service.dispatch(&a.trusted_peer, &prepare_request(&a, OBSERVE), 2),
        idl::MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH_REPLY,
    );
    assert_eq!(exhausted.status, 5);
    assert_eq!((exhausted.plan_id, exhausted.preview_shm), (0, 0));
    let second = launch(&mut service, &b);
    await_observation(&mut service, &second);
    assert_eq!(get_status(&mut service, &a, &first, 3).state, RUNNING);
    assert_eq!(get_status(&mut service, &b, &second, 3).state, RUNNING);
}

#[test]
fn ui1_0_wire_lengths_protocol_operation_reserved_and_tail_are_rejected() {
    let mut service = desktop(WitnessMode::Observe);
    let s = session(&mut service, 10);
    let good = prepare_request(&s, OBSERVE);
    for length in [0, 63, 65, u32::MAX] {
        let mut bad = good;
        bad.payload_len = length;
        assert_eq!(
            preview_reply(
                &service.dispatch(&s.trusted_peer, &bad, 0),
                idl::MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH_REPLY
            )
            .status,
            INVALID
        );
    }
    let mut reserved = good;
    reserved.payload[60] = 1;
    assert_eq!(
        preview_reply(
            &service.dispatch(&s.trusted_peer, &reserved, 0),
            idl::MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH_REPLY
        )
        .status,
        INVALID
    );
    let plan = prepare(&mut service, &s, 0);
    let mut tail = confirm_request(&s, &plan);
    tail.payload[40] = 1;
    assert_eq!(
        launch_reply(&service.dispatch(&s.trusted_peer, &tail, 1)).status,
        INVALID
    );
    let mut protocol = good;
    protocol.protocol += 1;
    let unsupported = service.dispatch(&s.trusted_peer, &protocol, 1);
    assert_eq!(u32_at(&unsupported, 8), UNSUPPORTED);
    let mut operation = good;
    operation.msg_type = 15;
    assert_eq!(
        u32_at(&service.dispatch(&s.trusted_peer, &operation, 1), 8),
        UNSUPPORTED
    );
    let child = confirm(&mut service, &s, &plan, 2);
    await_observation(&mut service, &child);
}

#[test]
fn ui1_0_raw_frame_rejects_reserved_handle_bits_outer_pad_and_payload_tail() {
    let mut service = desktop(WitnessMode::Observe);
    let s = session(&mut service, 10);
    let good = prepare_request(&s, OBSERVE);
    let wire = encode_envelope_wire(&good).unwrap();
    assert_eq!(wire.len(), 88);
    assert_eq!(decode_envelope_wire(&wire).unwrap().payload, good.payload);
    for byte in [14usize, 84, 85, 86, 87] {
        let mut malformed = wire;
        malformed[byte] |= 1;
        assert!(matches!(
            decode_envelope_wire(&malformed),
            Err(Status::Invalid)
        ));
    }
    let plan = prepare(&mut service, &s, 0);
    let mut tail = encode_envelope_wire(&confirm_request(&s, &plan)).unwrap();
    tail[20 + 40] = 1;
    assert!(matches!(decode_envelope_wire(&tail), Err(Status::Invalid)));
    let mut oversized = wire;
    oversized[16..20].copy_from_slice(&65u32.to_le_bytes());
    assert!(matches!(
        decode_envelope_wire(&oversized),
        Err(Status::Invalid)
    ));
    let mut noncanonical = good;
    noncanonical.handle.generation = 1u64 << 32;
    assert!(matches!(
        encode_envelope_wire(&noncanonical),
        Err(Status::Invalid)
    ));
    let child = confirm(&mut service, &s, &plan, 1);
    await_observation(&mut service, &child);
}

#[test]
fn ui1_0_revoke_close_and_expiry_reap_child_and_retire_old_grants() {
    for action in 0..3 {
        let mut service = desktop(WitnessMode::Observe);
        let s = session(&mut service, 10);
        let child = launch(&mut service, &s);
        await_observation(&mut service, &child);
        let old_handle = Handle::unpack(child.observation_handle);
        assert!(service.is_endpoint_active(old_handle));
        // This replay uses the opaque context captured from that actual child.
        // It is trusted default-off instrumentation, not a live post-kill call.
        assert_eq!(
            status_reply(
                &service.dispatch_recorded_child(
                    child.instance_id,
                    &observe_request(&s, &child),
                    2
                ),
                idl::MSG_DESKTOP_SESSION_V1_OBSERVE_INSTANCE_REPLY
            )
            .status,
            OK
        );
        match action {
            0 => assert_eq!(
                short_status(
                    &service.dispatch(
                        &s.trusted_peer,
                        &instance_request(idl::MSG_DESKTOP_SESSION_V1_REVOKE_INSTANCE, &s, &child),
                        3
                    ),
                    idl::MSG_DESKTOP_SESSION_V1_REVOKE_INSTANCE_REPLY
                ),
                OK
            ),
            1 => assert_eq!(
                short_status(
                    &service.dispatch(
                        &s.trusted_peer,
                        &instance_request(idl::MSG_DESKTOP_SESSION_V1_CLOSE_INSTANCE, &s, &child),
                        3
                    ),
                    idl::MSG_DESKTOP_SESSION_V1_CLOSE_INSTANCE_REPLY
                ),
                OK
            ),
            _ => service.maintenance(1_001),
        }
        let now = if action == 2 { 1_001 } else { 4 };
        let terminal = get_status(&mut service, &s, &child, now);
        assert_eq!(terminal.status, OK);
        assert_eq!(terminal.state, [REVOKED, 3, EXPIRED][action]);
        assert_reaped(&service, &child);
        assert!(!service.is_endpoint_active(old_handle));
        let denied = status_reply(
            &service.dispatch_recorded_child(child.instance_id, &observe_request(&s, &child), now),
            idl::MSG_DESKTOP_SESSION_V1_OBSERVE_INSTANCE_REPLY,
        );
        assert_eq!(denied.status, DENIED);
        assert_eq!(
            (denied.instance_id, denied.instance_generation, denied.state),
            (0, 0, 0)
        );
        assert_eq!(denied.executable_hash, [0; 32]);
        let restart = preview_reply(
            &service.dispatch(
                &s.trusted_peer,
                &instance_request(idl::MSG_DESKTOP_SESSION_V1_PREPARE_RESTART, &s, &child),
                now,
            ),
            idl::MSG_DESKTOP_SESSION_V1_PREPARE_RESTART_REPLY,
        );
        assert_eq!(restart.status, OK);
        let fresh = confirm(&mut service, &s, &restart, now + 1);
        assert_ne!(fresh.instance_id, child.instance_id);
        assert_ne!(fresh.instance_generation, child.instance_generation);
        assert_ne!(fresh.observation_handle, child.observation_handle);
    }
}

#[test]
fn ui1_0_fault_is_visible_and_restart_requires_fresh_confirmation() {
    let mut service = desktop(WitnessMode::Fault);
    let s = session(&mut service, 10);
    let child = launch(&mut service, &s);
    let watchdog = Instant::now() + Duration::from_secs(2);
    loop {
        service.maintenance(2);
        if get_status(&mut service, &s, &child, 2).state == FAULTED {
            break;
        }
        assert!(Instant::now() < watchdog, "faulted child not retired");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_reaped(&service, &child);
    let restart = preview_reply(
        &service.dispatch(
            &s.trusted_peer,
            &instance_request(idl::MSG_DESKTOP_SESSION_V1_PREPARE_RESTART, &s, &child),
            3,
        ),
        idl::MSG_DESKTOP_SESSION_V1_PREPARE_RESTART_REPLY,
    );
    assert_eq!(restart.status, OK);
    assert_eq!(get_status(&mut service, &s, &child, 4).state, FAULTED);
    let next = confirm(&mut service, &s, &restart, 4);
    assert_ne!(next.instance_id, child.instance_id);
    assert_ne!(next.observation_handle, child.observation_handle);
}

#[test]
fn ui1_0_absolute_watchdog_reaps_stalled_child_without_dispatch_or_maintenance() {
    let mut service = desktop_with_deadline(WitnessMode::Stall, 200);
    let s = session(&mut service, 10);
    let child = launch(&mut service, &s);
    // The service receives no further dispatch or fake-clock maintenance. A
    // separate absolute-time watchdog must kill AND reap an infinite child.
    assert!(
        process_alive(child.child_pid),
        "watchdog witness must actually be running"
    );
    let deadline = Instant::now() + Duration::from_secs(2);
    while process_alive(child.child_pid) {
        assert!(
            Instant::now() < deadline,
            "watchdog depends on foreground service progress"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    // Only after the OS process probe establishes disappearance may service
    // accessors be called; a lazy accessor cannot implement this watchdog.
    record_independent_disappearance(&child, "absolute-watchdog");
    assert_reaped(&service, &child);
    assert_eq!(get_status(&mut service, &s, &child, 2).state, FAULTED);
}

#[test]
fn ui1_0_drop_reaps_owned_child() {
    let mut service = desktop_with_deadline(WitnessMode::Stall, 5_000);
    let s = session(&mut service, 10);
    let child = launch(&mut service, &s);
    assert!(
        process_alive(child.child_pid),
        "Drop witness must actually be running"
    );
    record_child_evidence(&service, child.instance_id);
    let started = Instant::now();
    drop(service);
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "Drop waited for child's five-second deadline"
    );
    assert!(
        !process_alive(child.child_pid),
        "Drop must kill and reap, not leave an owned process/zombie"
    );
    record_independent_disappearance(&child, "drop");
}

#[test]
fn ui1_0_history_and_child_exchange_bounds_are_enforced() {
    let mut service = desktop(WitnessMode::Observe);
    let s = session(&mut service, 10);
    let mut history = Vec::new();
    let mut plan = prepare(&mut service, &s, 0);
    for index in 0..16u64 {
        let now = index * 3 + 1;
        let child = confirm(&mut service, &s, &plan, now);
        await_observation_at(&mut service, &child, now);
        assert_eq!(
            short_status(
                &service.dispatch(
                    &s.trusted_peer,
                    &instance_request(idl::MSG_DESKTOP_SESSION_V1_CLOSE_INSTANCE, &s, &child),
                    now + 1
                ),
                idl::MSG_DESKTOP_SESSION_V1_CLOSE_INSTANCE_REPLY
            ),
            OK
        );
        assert_reaped(&service, &child);
        assert!(!service.is_endpoint_active(Handle::unpack(child.observation_handle)));
        history.push(child);
        let last = history.last().unwrap();
        let next = preview_reply(
            &service.dispatch(
                &s.trusted_peer,
                &instance_request(idl::MSG_DESKTOP_SESSION_V1_PREPARE_RESTART, &s, last),
                now + 2,
            ),
            idl::MSG_DESKTOP_SESSION_V1_PREPARE_RESTART_REPLY,
        );
        if index == 15 {
            assert_eq!(
                next.status, 5,
                "17th instance must be refused before spawning"
            );
            assert_eq!(
                (next.plan_id, next.preview_shm, next.granted_rights),
                (0, 0, 0)
            );
        } else {
            assert_eq!(next.status, OK);
            plan = next;
        }
    }
    for child in &history {
        let evidence = service
            .child_evidence(child.instance_id)
            .expect("all 16 terminal records retained");
        assert!(evidence.reaped);
        assert!(evidence.exchanges.len() <= 64);
        assert_eq!(evidence.actual_pid, child.child_pid);
        assert!(!service.is_endpoint_active(Handle::unpack(child.observation_handle)));
    }
    let denied = preview_reply(
        &service.dispatch(&s.trusted_peer, &prepare_request(&s, OBSERVE), 49),
        idl::MSG_DESKTOP_SESSION_V1_PREPARE_LAUNCH_REPLY,
    );
    assert_eq!(
        denied.status, 5,
        "fresh prepare cannot bypass terminal-history capacity"
    );
    assert_eq!(
        (denied.plan_id, denied.preview_shm, denied.granted_rights),
        (0, 0, 0)
    );

    let mut flood_service = desktop(WitnessMode::Flood);
    let a = session(&mut flood_service, 10);
    let b = session(&mut flood_service, 20);
    let first = launch(&mut flood_service, &a);
    let second = launch(&mut flood_service, &b);
    let watchdog = Instant::now() + Duration::from_secs(2);
    loop {
        if flood_service
            .child_evidence(first.instance_id)
            .is_some_and(|e| e.reaped && e.exchange_limit_hit)
        {
            break;
        }
        assert!(
            Instant::now() < watchdog,
            "65th child exchange did not enforce bounded retirement"
        );
        flood_service.maintenance(2);
        std::thread::sleep(Duration::from_millis(5));
    }
    let bounded = flood_service.child_evidence(first.instance_id).unwrap();
    assert!(bounded.exchange_limit_hit);
    assert_eq!(bounded.exchanges.len(), 64);
    assert_eq!(
        bounded.observation_count, 64,
        "65th own observation must not be accepted"
    );
    assert!(bounded.exchanges.iter().all(|(r, p)| r.msg_type
        == idl::MSG_DESKTOP_SESSION_V1_OBSERVE_INSTANCE
        && status_reply(p, idl::MSG_DESKTOP_SESSION_V1_OBSERVE_INSTANCE_REPLY).status == OK));
    assert_reaped(&flood_service, &first);
    assert!(!flood_service.is_endpoint_active(Handle::unpack(first.observation_handle)));
    assert_eq!(get_status(&mut flood_service, &a, &first, 3).state, FAULTED);
    // B's own deliberate Flood fault is independent: its owner remains able to
    // query/recover it, and A's limit cannot exhaust B's session authority.
    assert_eq!(get_status(&mut flood_service, &b, &second, 3).status, OK);
    let recovering_a = preview_reply(
        &flood_service.dispatch(
            &a.trusted_peer,
            &instance_request(idl::MSG_DESKTOP_SESSION_V1_PREPARE_RESTART, &a, &first),
            4,
        ),
        idl::MSG_DESKTOP_SESSION_V1_PREPARE_RESTART_REPLY,
    );
    assert_eq!(recovering_a.status, OK);
    assert_eq!(get_status(&mut flood_service, &b, &second, 4).status, OK);
}

#[test]
fn ui1_0_stalled_or_retired_session_does_not_block_unrelated_session() {
    let mut service = desktop_with_deadline(WitnessMode::Stall, 500);
    let a = session(&mut service, 10);
    let b = session(&mut service, 20);
    let first = launch(&mut service, &a);
    let started = Instant::now();
    let second = launch(&mut service, &b);
    assert_eq!(get_status(&mut service, &b, &second, 2).status, OK);
    assert!(
        started.elapsed() < Duration::from_millis(250),
        "child A stalled unrelated session B"
    );
    assert_eq!(
        short_status(
            &service.dispatch(
                &a.trusted_peer,
                &instance_request(idl::MSG_DESKTOP_SESSION_V1_REVOKE_INSTANCE, &a, &first),
                3
            ),
            idl::MSG_DESKTOP_SESSION_V1_REVOKE_INSTANCE_REPLY
        ),
        OK
    );
    assert_reaped(&service, &first);
    assert_eq!(get_status(&mut service, &b, &second, 4).state, RUNNING);
    let plan = preview_reply(
        &service.dispatch(
            &a.trusted_peer,
            &instance_request(idl::MSG_DESKTOP_SESSION_V1_PREPARE_RESTART, &a, &first),
            4,
        ),
        idl::MSG_DESKTOP_SESSION_V1_PREPARE_RESTART_REPLY,
    );
    assert_eq!(plan.status, OK);
    let replacement = confirm(&mut service, &a, &plan, 5);
    assert_ne!(replacement.instance_id, first.instance_id);
    assert_eq!(get_status(&mut service, &b, &second, 6).state, RUNNING);
}
