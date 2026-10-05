//! Bounded ordinary-host consumers. No fixture callback grants a commit permit.
use artifact_store_schema::Manifest;
use artifact_store_schema::editor_save::*;
use artifact_store_schema::signature::{
    SignaturePolicy, SignatureValidationConfig, SignatureValidationResult, TrustedKeys,
    manifest_signing_bytes, validate_manifest_signatures,
};
use kernel_api::cap::Handle;
use kernel_api::generated::desktop_artifact_v1 as artifact;
use kernel_api::ipc::Envelope;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use store_service::editor_store::*;

pub const OLD: &[u8] = b"note=old\n";
pub const NEW: &[u8] = b"note=new\n";
pub const KEY_ID: &str = "ui1.1b-fixture";
pub const KEY_SEED: [u8; 32] = [42; 32]; // Named synthetic signing fixture, no production key.
pub fn hash(bytes: &[u8]) -> Hash32 {
    Sha256::digest(bytes).into()
}
pub fn ok<T, E>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(_) => panic!("required positive consumer failed"),
    }
}
pub fn actor(session: u64, instance: u64) -> EditorActorV0 {
    ok(EditorActorV0::try_new(1, session, 1, instance, 1))
}
pub fn profile(revision: u64) -> FixtureProfile {
    FixtureProfile {
        origin_ms: 1,
        initial_objects: vec![
            InitialObject {
                owner_id: 1,
                object_id: 31,
                generation: 1,
                revision,
                bytes: OLD.to_vec(),
            },
            InitialObject {
                owner_id: 1,
                object_id: 32,
                generation: 1,
                revision: 1,
                bytes: b"other=old\n".to_vec(),
            },
        ],
        fixture_signing_seed: KEY_SEED,
        fixture_key_id: KEY_ID.into(),
    }
}

// Test-private recording is scoped to one actual fixture, never an ambient ledger.
// The fixture's immutable namespace ID is learned from its actual typed fence;
// historical states are annotated with that same immutable ID before hashing.
const MIB: usize = 1024 * 1024;
struct CappedJson {
    bytes: Vec<u8>,
    cap: usize,
}
impl Write for CappedJson {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.cap.saturating_sub(self.bytes.len()) {
            return Err(std::io::Error::other("evidence serialization bound"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn encoded<T: Serialize>(value: &T, cap: usize) -> Vec<u8> {
    let mut writer = CappedJson {
        bytes: Vec::new(),
        cap,
    };
    ok(serde_json::to_writer(&mut writer, value));
    writer.bytes
}
fn lock<T>(value: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    // Only the test-private ledger uses this helper. Recover for bounded Drop
    // cleanup after an assertion panic; unwinding never publishes evidence.
    value.lock().unwrap_or_else(|poison| poison.into_inner())
}
fn thread_id() -> String {
    let id = format!("{:?}", thread::current().id());
    assert!(id.len() <= 64);
    id
}
fn random_id() -> String {
    hex::encode(rand::random::<[u8; 16]>())
}
#[derive(Clone)]
struct RunContext {
    store: PathBuf,
    ui: PathBuf,
    nonce: String,
    birth: String,
    binary_sha: String,
}
impl RunContext {
    fn capture() -> Option<Self> {
        let store = std::env::var_os("RAMEN_DESKTOP_EDITOR_STORE_GATE_EVIDENCE");
        let ui = std::env::var_os("RAMEN_DESKTOP_EDITOR_GATE_EVIDENCE");
        if store.is_none() && ui.is_none() {
            return None;
        }
        let store = PathBuf::from(store.expect("Store evidence root required"));
        let ui = PathBuf::from(ui.expect("UI evidence root required"));
        for path in [&store, &ui] {
            assert!(path.is_absolute());
            assert!(path.as_os_str().len() <= 4096);
            let m = ok(fs::symlink_metadata(path));
            assert!(m.is_dir() && !m.file_type().is_symlink());
        }
        assert!(!store.starts_with(&ui) && !ui.starts_with(&store));
        let nonce = ok(std::env::var("RAMEN_DESKTOP_EDITOR_STORE_GATE_RUN_NONCE"));
        assert_eq!(nonce.len(), 32);
        assert!(
            nonce
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        );
        let birth = own_birth();
        assert!(birth.is_ascii() && !birth.is_empty() && birth.len() <= 128);
        let executable = ok(std::env::current_exe());
        let mut file = ok(OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(executable));
        let before = ok(file.metadata());
        assert!(before.is_file() && before.len() > 0 && before.len() <= 1024 * 1024 * 1024);
        let mut digest = Sha256::new();
        let mut remaining = before.len();
        let mut chunk = [0; 65536];
        while remaining > 0 {
            let n = ok(file.read(&mut chunk[..remaining.min(65536) as usize]));
            assert!(n > 0);
            digest.update(&chunk[..n]);
            remaining -= n as u64;
        }
        assert_eq!(ok(file.read(&mut chunk[..1])), 0);
        let after = ok(file.metadata());
        assert_eq!(
            (
                before.dev(),
                before.ino(),
                before.len(),
                before.mtime(),
                before.mtime_nsec()
            ),
            (
                after.dev(),
                after.ino(),
                after.len(),
                after.mtime(),
                after.mtime_nsec()
            )
        );
        Some(Self {
            store,
            ui,
            nonce,
            birth,
            binary_sha: hex::encode(digest.finalize()),
        })
    }
}
#[cfg(target_os = "linux")]
fn own_birth() -> String {
    let raw = bounded_read(Path::new("/proc/self/stat"), 8192);
    let text = ok(std::str::from_utf8(&raw));
    let fields = text
        .rsplit_once(") ")
        .expect("own proc stat fields")
        .1
        .split_whitespace();
    let fields = bounded_parts(fields, 128);
    assert!(fields.len() > 19 && fields.len() < 128);
    let start = ok(fields[19].parse::<u64>());
    let boot = bounded_read(Path::new("/proc/sys/kernel/random/boot_id"), 64);
    let boot = ok(std::str::from_utf8(&boot)).trim();
    assert!(boot.len() == 36 && boot.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-'));
    format!("linux:{boot}:{start}")
}
#[cfg(target_os = "macos")]
fn own_birth() -> String {
    let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::zeroed();
    let size = std::mem::size_of::<libc::proc_bsdinfo>();
    // Native libproc fills this fixed-size own-process buffer; no PID from input.
    let count = unsafe {
        libc::proc_pidinfo(
            std::process::id() as i32,
            libc::PROC_PIDTBSDINFO,
            0,
            info.as_mut_ptr().cast(),
            size as i32,
        )
    };
    assert_eq!(count, size as i32);
    let info = unsafe { info.assume_init() };
    assert_eq!(info.pbi_pid, std::process::id());
    format!(
        "macos:{}:{}:{}",
        info.pbi_pid, info.pbi_start_tvsec, info.pbi_start_tvusec
    )
}
#[cfg(target_os = "linux")]
fn bounded_parts<'a>(parts: impl Iterator<Item = &'a str>, cap: usize) -> Vec<&'a str> {
    let mut values = Vec::new();
    for value in parts {
        assert!(values.len() < cap);
        values.push(value);
    }
    values
}
fn monotonic_ns() -> u64 {
    let mut time = std::mem::MaybeUninit::<libc::timespec>::zeroed();
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, time.as_mut_ptr()) },
        0
    );
    let time = unsafe { time.assume_init() };
    u64::try_from(time.tv_sec)
        .unwrap()
        .checked_mul(1_000_000_000)
        .unwrap()
        .checked_add(u64::try_from(time.tv_nsec).unwrap())
        .unwrap()
}
#[derive(Clone)]
struct PeerContext {
    actor: EditorActorV0,
    class: EndpointClass,
    object: u64,
}
#[derive(Clone)]
pub struct RecordedPeer {
    actual: EditorPeer,
    context: PeerContext,
}
pub struct RecordedEndpoint {
    actual: EditorEndpoint,
    pub handle: Handle,
    pub peer: RecordedPeer,
}
impl std::ops::Deref for RecordedEndpoint {
    type Target = EditorEndpoint;
    fn deref(&self) -> &EditorEndpoint {
        &self.actual
    }
}
impl RecordedEndpoint {
    fn issued(
        actual: EditorEndpoint,
        actor: EditorActorV0,
        class: EndpointClass,
        object: u64,
    ) -> Self {
        let handle = actual.handle;
        let peer = RecordedPeer {
            actual: actual.peer.clone(),
            context: PeerContext {
                actor,
                class,
                object,
            },
        };
        Self {
            actual,
            handle,
            peer,
        }
    }
}
#[derive(Serialize)]
struct NativeHandle {
    kind: u8,
    index: u32,
    generation: u64,
}
#[derive(Serialize)]
struct NativeEnvelope {
    protocol: u32,
    msg_type: u32,
    handle: NativeHandle,
    payload_len: u32,
    payload: Vec<u8>,
}
fn native_envelope(q: &Envelope) -> NativeEnvelope {
    NativeEnvelope {
        protocol: q.protocol,
        msg_type: q.msg_type,
        handle: NativeHandle {
            kind: q.handle.kind as u8,
            index: q.handle.index,
            generation: q.handle.generation,
        },
        payload_len: q.payload_len,
        payload: q.payload.to_vec(),
    }
}
struct CallRecord {
    ordinal: usize,
    epoch: u32,
    completion: Option<usize>,
    kind: &'static str,
    thread: String,
    peer: PeerContext,
    input_kind: &'static str,
    raw: Option<[u8; 88]>,
    typed: Option<NativeEnvelope>,
    input_sha: String,
    result: Option<Value>,
    elapsed: u64,
    matched: Option<usize>,
}
impl CallRecord {
    fn value(&self) -> Value {
        json!({"call_ordinal":self.ordinal,"producer_epoch":self.epoch,"observed_completion_ordinal":self.completion.expect("actual completion"),"caller_kind":self.kind,"caller_rust_thread_id":self.thread,"actor":self.peer.actor,"endpoint_class":class_name(&self.peer.class),"selected_object_id":self.peer.object,"input_kind":self.input_kind,"request_wire_or_null":self.raw.as_ref().map(|r|r.to_vec()),"typed_envelope_or_null":self.typed,"input_diagnostic_sha256":self.input_sha,"actual_result":self.result.as_ref().expect("actual result"),"elapsed_us":self.elapsed,"matched_public_exchange_index_or_null":self.matched})
    }
}
#[derive(Clone)]
struct LeaseOrigin {
    call: usize,
    epoch: u32,
    peer: PeerContext,
    request_sha: String,
    reply_sha: String,
}
struct CopiedLease {
    origin: LeaseOrigin,
    descriptor: ObjectDescriptor,
    bytes: Vec<u8>,
}
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
struct StateFile {
    relative_path: String,
    kind: &'static str,
    bytes: usize,
    sha256: Hash32,
}
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
struct StateObject {
    object_id: u64,
    selected_generation: u64,
    service_epoch: u64,
    selected_revision: u64,
    selected_content_hash: Hash32,
    dispatches: u32,
    permits: u32,
    transitions: u32,
    journal_sequence: u64,
    journal_hash: Hash32,
    blob_hash: Hash32,
    manifest_hash: Hash32,
    ownership_hash: Hash32,
    quarantined: bool,
    pending_writer_id_or_null: Option<u64>,
}
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
struct StateBody {
    schema_version: u32,
    run_nonce: String,
    fixture_id: String,
    namespace_id: u64,
    root_device: u64,
    root_inode: u64,
    files: Vec<StateFile>,
    objects: Vec<StateObject>,
}
struct SavedState {
    phase: &'static str,
    epoch: u32,
    body: StateBody,
}
struct ArchivedEpoch {
    epoch: u32,
    before: Value,
    after: Value,
    fence: FenceSnapshot,
    raw: Vec<u8>,
}
struct Recorder {
    run: Option<RunContext>,
    id: String,
    root: PathBuf,
    device: u64,
    inode: u64,
    calls: Vec<CallRecord>,
    completion: usize,
    copies: Vec<CopiedLease>,
    origins: BTreeMap<(u32, u64), LeaseOrigin>,
    pending: BTreeMap<usize, JoinHandle<()>>,
    pending_slots: BTreeSet<usize>,
    pending_high: usize,
    joins: Vec<Value>,
    epochs: Vec<ArchivedEpoch>,
    states: Vec<SavedState>,
    negative: Vec<Value>,
    denials: Vec<Value>,
    reopen: Option<Value>,
}
impl Recorder {
    fn identity(&self, case: &str, leg: u32) -> Value {
        let run = self.run.as_ref().expect("actual run context");
        json!({"schema_version":1,"run_nonce":run.nonce,"case":case,"leg":leg,"fixture_id":self.id,"native_test_pid":std::process::id(),"test_process_start_identity":run.birth})
    }
    fn begin(
        &mut self,
        epoch: u32,
        peer: &RecordedPeer,
        q: &Envelope,
        kind: &'static str,
    ) -> usize {
        assert!(
            self.calls.len() < 512 && self.calls.iter().filter(|c| c.epoch == epoch).count() < 256
        );
        let raw = if q.msg_type % 2 == 1 {
            encode_envelope_wire(q).ok()
        } else {
            None
        };
        let typed = raw.is_none().then(|| native_envelope(q));
        let diagnostic = match &raw {
            Some(raw) => raw.to_vec(),
            None => encoded(typed.as_ref().unwrap(), 4096),
        };
        let ordinal = self.calls.len();
        self.calls.push(CallRecord {
            ordinal,
            epoch,
            completion: None,
            kind,
            thread: thread_id(),
            peer: peer.context.clone(),
            input_kind: if raw.is_some() {
                "canonical_request_wire"
            } else {
                "native_typed_envelope"
            },
            raw,
            typed,
            input_sha: hex::encode(hash(&diagnostic)),
            result: None,
            elapsed: 0,
            matched: None,
        });
        ordinal
    }
    fn completed(&mut self, id: usize, result: &Result<Envelope, StoreStatus>, elapsed: u64) {
        let value = match result {
            Ok(r) => {
                json!({"kind":"ok_envelope","returned_wire":ok(encode_envelope_wire(r)).to_vec()})
            }
            Err(e) => json!({"kind":"error","error_status":*e as u32}),
        };
        self.calls[id].thread = thread_id();
        self.calls[id].result = Some(value);
        self.calls[id].elapsed = elapsed;
        self.calls[id].completion = Some(self.completion);
        self.completion += 1;
        if let (Some(q), Ok(r)) = (self.calls[id].raw, *result) {
            let (handle, kind) = match r.msg_type {
                2 if u32_at(&r.payload, 36) == 0 => {
                    (u64_at(&r.payload, 16), SharedObjectKind::SelectedText)
                }
                6 | 8 if u32_at(&r.payload, 32) == 0 => {
                    (u64_at(&r.payload, 24), SharedObjectKind::Receipt)
                }
                _ => (0, SharedObjectKind::DraftSource),
            };
            if handle != 0 {
                assert!(kind != SharedObjectKind::DraftSource && self.origins.len() < 512);
                self.origins.insert(
                    (self.calls[id].epoch, handle),
                    LeaseOrigin {
                        call: id,
                        epoch: self.calls[id].epoch,
                        peer: self.calls[id].peer.clone(),
                        request_sha: hex::encode(hash(&q)),
                        reply_sha: hex::encode(hash(&ok(encode_envelope_wire(&r)))),
                    },
                );
            }
        }
    }
    fn match_epoch(&mut self, epoch: u32, evidence: &StoreEvidence) {
        let mut used = BTreeSet::new();
        for call in self.calls.iter_mut().filter(|c| c.epoch == epoch) {
            let result = call.result.as_ref().expect("completed actual call");
            if result["kind"] == "error" {
                continue;
            }
            let q = call.raw.expect("Ok actual request wire");
            let returned: Vec<u8> = ok(serde_json::from_value(result["returned_wire"].clone()));
            let found = evidence.exchanges.iter().enumerate().find(|(i, e)| {
                !used.contains(i)
                    && e.actor == call.peer.actor
                    && e.endpoint_class == call.peer.class
                    && e.selected_object_id == call.peer.object
                    && e.request_wire == q
                    && (if e.reply_suppressed {
                        u32_at(
                            &returned,
                            20 + match u32_at(&returned, 4) {
                                2 => 36,
                                4 => 16,
                                6 | 8 => 32,
                                _ => panic!("unsupported actual reply"),
                            },
                        ) == 9
                    } else {
                        e.actual_reply_wire
                            .as_ref()
                            .is_some_and(|r| r.as_slice() == returned)
                    })
            });
            let (index, _) = found.expect("same actual compatible public exchange");
            call.matched = Some(index);
            used.insert(index);
        }
        assert_eq!(
            used.len(),
            evidence.exchanges.len(),
            "exact compatible call multiplicity"
        );
    }
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}
#[derive(Clone)]
pub struct RecordedHost {
    actual: StoreHost,
    control: Arc<FixtureController>,
    epoch: u32,
    recorder: Arc<Mutex<Recorder>>,
}
impl RecordedHost {
    pub fn dispatch(&self, peer: &RecordedPeer, q: &Envelope) -> Result<Envelope, StoreStatus> {
        let id = lock(&self.recorder).begin(self.epoch, peer, q, "test_thread");
        self.dispatch_reserved(id, peer, q)
    }
    fn dispatch_reserved(
        &self,
        id: usize,
        peer: &RecordedPeer,
        q: &Envelope,
    ) -> Result<Envelope, StoreStatus> {
        let typed = lock(&self.recorder).calls[id].raw.is_none();
        let before = typed.then(|| capture_state(self));
        let start = Instant::now();
        let result = self.actual.dispatch(&peer.actual, q);
        let elapsed = u64::try_from(start.elapsed().as_micros()).unwrap();
        let after = typed.then(|| capture_state(self));
        let mut ledger = lock(&self.recorder);
        ledger.completed(id, &result, elapsed);
        if let (Some(before), Some(after), Err(status)) = (before, after, &result) {
            assert_eq!(
                before, after,
                "malformed native call unchanged actual scoped state"
            );
            assert!(ledger.negative.len() < 7);
            assert_eq!(
                before,
                ledger
                    .states
                    .iter()
                    .find(|s| s.phase == "selectedtext_before_allocation")
                    .expect("actual raw/typed phase witness")
                    .body
            );
            let ordinal = ledger.negative.len();
            let row = json!({"ordinal":ordinal,"producer_epoch":self.epoch,"stage":"typed_dispatch","input_kind":"typed_envelope","raw_wire_or_null":null,"typed_envelope_or_null":ledger.calls[id].typed,"input_diagnostic_sha256":ledger.calls[id].input_sha,"actual_returned_error_status":*status as u32,"actor_or_null":peer.context.actor,"endpoint_class_or_null":class_name(&peer.context.class),"selected_object_id_or_null":peer.context.object,"before_state_sha256":"","after_state_sha256":""});
            encoded(&row, 4096);
            ledger.negative.push(row);
        }
        result
    }
    pub fn draft_source(
        &self,
        peer: &RecordedPeer,
        base: u64,
        bytes: &[u8],
    ) -> Result<ObjectDescriptor, StoreStatus> {
        self.actual.draft_source(&peer.actual, base, bytes)
    }
    pub fn write_lease(
        &self,
        peer: &RecordedPeer,
        d: &ObjectDescriptor,
    ) -> Result<WriteLease, StoreStatus> {
        self.actual.write_lease(&peer.actual, d)
    }
    pub fn read_lease(
        &self,
        peer: &RecordedPeer,
        d: &ObjectDescriptor,
    ) -> Result<RecordedReadLease, StoreStatus> {
        let actual = self.actual.read_lease(&peer.actual, d)?;
        let origin = lock(&self.recorder)
            .origins
            .get(&(self.epoch, d.handle.pack()))
            .cloned()
            .expect("descriptor issued by actual successful CallId");
        assert_eq!(origin.peer.actor, peer.context.actor);
        assert_eq!(origin.peer.class, peer.context.class);
        assert_eq!(origin.peer.object, peer.context.object);
        Ok(RecordedReadLease {
            actual,
            origin,
            descriptor: d.clone(),
            recorder: self.recorder.clone(),
        })
    }
}
pub struct RecordedReadLease {
    actual: ReadLease,
    origin: LeaseOrigin,
    descriptor: ObjectDescriptor,
    recorder: Arc<Mutex<Recorder>>,
}
impl RecordedReadLease {
    pub fn copy_into(&self, offset: u32, out: &mut [u8]) -> Result<(), StoreStatus> {
        if offset == 0 && out.len() == self.descriptor.byte_len as usize {
            assert!(out.len() <= 4160 && lock(&self.recorder).copies.len() < 256);
        }
        self.actual.copy_into(offset, out)?;
        if offset == 0 && out.len() == self.descriptor.byte_len as usize {
            let mut ledger = lock(&self.recorder);
            assert!(ledger.copies.len() < 256);
            ledger.copies.push(CopiedLease {
                origin: self.origin.clone(),
                descriptor: self.descriptor.clone(),
                bytes: out.to_vec(),
            });
        }
        Ok(())
    }
}
fn capture_state(host: &RecordedHost) -> StateBody {
    let (root, id, nonce, device, inode, namespace) = {
        let r = lock(&host.recorder);
        (
            r.root.clone(),
            r.id.clone(),
            r.run.as_ref().map(|c| c.nonce.clone()).unwrap_or_default(),
            r.device,
            r.inode,
            r.epochs.first().map(|e| e.fence.namespace_id).unwrap_or(0),
        )
    };
    let files = scan(&root.join("owner"))
        .into_iter()
        .map(|f| {
            let row = StateFile {
                relative_path: f.path.to_str().unwrap().into(),
                kind: f.kind,
                bytes: f.bytes.len(),
                sha256: hash(&f.bytes),
            };
            encoded(&row, 512);
            row
        })
        .collect();
    let evidence = ok(host.control.evidence());
    assert_ledger(&evidence);
    assert_eq!(evidence.objects.len(), 2);
    assert!(
        evidence
            .objects
            .windows(2)
            .all(|w| w[0].selected.selected_object_id < w[1].selected.selected_object_id)
    );
    let objects = evidence
        .objects
        .iter()
        .map(|o| StateObject {
            object_id: o.selected.selected_object_id,
            selected_generation: o.selected.selected_generation,
            service_epoch: o.service_epoch,
            selected_revision: o.selected.revision,
            selected_content_hash: o.selected.content_hash,
            dispatches: o.mutation_dispatch_count,
            permits: o.permit_count,
            transitions: o.transition_count,
            journal_sequence: o.journal_sequence,
            journal_hash: o.journal_hash,
            blob_hash: o.blob_hash,
            manifest_hash: o.manifest_hash,
            ownership_hash: o.ownership_hash,
            quarantined: o.mutation_quarantined,
            pending_writer_id_or_null: o.pending_writer_id,
        })
        .collect();
    StateBody {
        schema_version: 1,
        run_nonce: nonce,
        fixture_id: id,
        namespace_id: namespace,
        root_device: device,
        root_inode: inode,
        files,
        objects,
    }
}
fn public_json(e: &StoreEvidence) -> Value {
    assert_ledger(e);
    let exchanges=e.exchanges.iter().map(|x| {let v=json!({"actor":x.actor,"endpoint_class":class_name(&x.endpoint_class),"selected_object_id":x.selected_object_id,"request_wire":x.request_wire.to_vec(),"actual_reply_wire":x.actual_reply_wire.as_ref().map(|r|r.to_vec()),"reply_suppressed":x.reply_suppressed,"operation_id":x.operation_id,"elapsed_us":x.elapsed_us,"observed_ms":x.observed_ms});encoded(&v,2048);v}).collect::<Vec<_>>();
    let objects=e.objects.iter().map(|o| {assert!(o.operations.len()<=16);for record in &o.operations {encoded(record,8192);}let v=json!({"selected":o.selected,"operations":o.operations,"mutation_dispatch_count":o.mutation_dispatch_count,"permit_count":o.permit_count,"transition_count":o.transition_count,"journal_hash":o.journal_hash,"journal_sequence":o.journal_sequence,"blob_hash":o.blob_hash,"manifest_hash":o.manifest_hash,"ownership_hash":o.ownership_hash,"mutation_quarantined":o.mutation_quarantined,"pending_writer_id":o.pending_writer_id,"service_epoch":o.service_epoch,"reopen_witness":o.reopen_witness.as_ref().map(reopen_json)});encoded(&v,147456);v}).collect::<Vec<_>>();
    let io=e.io_events.iter().map(|x| {let v=json!({"selected_object_id":x.selected_object_id,"operation_id":x.operation_id,"writer_id":x.writer_id,"sequence":x.sequence,"point":point_name(&x.point),"outcome":outcome_name(&x.outcome),"artifact_hash":x.artifact_hash});encoded(&v,512);v}).collect::<Vec<_>>();
    let ledger = &e.ledger;
    let l = json!({"sessions":ledger.sessions,"selected_objects":ledger.selected_objects,"original_instances":ledger.original_instances,"endpoints":ledger.endpoints,"shared_objects":ledger.shared_objects,"active_io_workers":ledger.active_io_workers,"active_supervisors":ledger.active_supervisors,"held_service_producers":ledger.held_service_producers,"operations_per_object":ledger.operations_per_object,"cas_entries_per_object":ledger.cas_entries_per_object,"bytes_per_object":ledger.bytes_per_object});
    encoded(&l, 4096);
    for barrier in &e.barriers {
        encoded(barrier, 1024);
    }
    let value = json!({"exchanges":exchanges,"objects":objects,"barriers":e.barriers,"io_events":io,"ledger":l});
    encoded(&value, 2 * MIB);
    value
}
fn reopen_json(w: &ReopenWitness) -> Value {
    json!({"fence_id":w.fence_id,"fence_snapshot_hash":w.fence_snapshot_hash,"selected_object_id":w.selected_object_id,"selected_generation":w.selected_generation,"prior_service_epoch":w.prior_service_epoch,"actual_service_epoch":w.actual_service_epoch,"reopened_journal_sequence":w.reopened_journal_sequence,"reopened_journal_hash":w.reopened_journal_hash})
}

pub struct Fixture {
    root: Option<tempfile::TempDir>,
    pub host: RecordedHost,
    pub control: Arc<FixtureController>,
    tokens: Vec<(Arc<FixtureController>, PauseToken)>,
    fenced: bool,
    finished: bool,
    recorder: Arc<Mutex<Recorder>>,
}
impl Fixture {
    pub fn new() -> Self {
        Self::with_profile(profile(1))
    }
    pub fn with_profile(profile: FixtureProfile) -> Self {
        #[cfg(target_os = "linux")]
        let root = ok(tempfile::Builder::new()
            .prefix("ramenos-editor-store-assertions-")
            .tempdir());
        #[cfg(target_os = "macos")]
        let root = {
            // This host fixture requires the actual Data-volume temporary
            // directory, rather than a /var or /tmp namespace alias.
            let parent = Path::new("/System/Volumes/Data/private/tmp");
            let metadata = ok(fs::symlink_metadata(parent));
            assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
            assert_eq!(
                (metadata.uid(), metadata.gid(), metadata.mode() & 0o7777),
                (0, 0, 0o1777)
            );
            ok(tempfile::Builder::new()
                .prefix("ramenos-editor-store-assertions-")
                .tempdir_in(parent))
        };
        let metadata = ok(fs::symlink_metadata(root.path()));
        #[cfg(target_os = "macos")]
        {
            // Observe this live root before StoreFixture::create can issue
            // any authority. Keep its native path and mount strings intact.
            let directory = ok(OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
                .open(root.path()));
            let opened = ok(directory.metadata());
            assert!(opened.is_dir());
            assert_eq!(
                (opened.dev(), opened.ino()),
                (metadata.dev(), metadata.ino())
            );
            let observation = filesystem_observation(root.path(), &directory, &opened);
            assert_eq!(
                observation["mount_flags"]["read_only"].as_bool(),
                Some(false)
            );
        }
        let run = RunContext::capture();
        let recorder = Arc::new(Mutex::new(Recorder {
            run,
            id: random_id(),
            root: root.path().to_path_buf(),
            device: metadata.dev(),
            inode: metadata.ino(),
            calls: vec![],
            completion: 0,
            copies: vec![],
            origins: BTreeMap::new(),
            pending: BTreeMap::new(),
            pending_slots: BTreeSet::new(),
            pending_high: 0,
            joins: vec![],
            epochs: vec![],
            states: vec![],
            negative: vec![],
            denials: vec![],
            reopen: None,
        }));
        let (actual, control) = ok(StoreFixture::create(&root.path().join("owner"), profile));
        let control = Arc::new(control);
        let host = RecordedHost {
            actual,
            control: control.clone(),
            epoch: 0,
            recorder: recorder.clone(),
        };
        Self {
            root: Some(root),
            host,
            control,
            tokens: vec![],
            fenced: false,
            finished: false,
            recorder,
        }
    }
    pub fn root(&self) -> PathBuf {
        self.root.as_ref().unwrap().path().join("owner")
    }
    pub fn endpoint(
        &self,
        actor: &EditorActorV0,
        object: u64,
        rights: u32,
        ttl: u64,
    ) -> RecordedEndpoint {
        let actual = ok(self.control.issue_editor(GrantSpec {
            actor: actor.clone(),
            selected_object_id: object,
            selected_generation: 1,
            rights,
            ttl_ms: ttl,
        }));
        RecordedEndpoint::issued(actual, actor.clone(), EndpointClass::Artifact, object)
    }
    pub fn recovery(&self, actor: EditorActorV0, object: u64, operation: u64) -> RecordedEndpoint {
        let actual = ok(self
            .control
            .issue_recovery(actor.clone(), object, operation));
        RecordedEndpoint::issued(actual, actor, EndpointClass::RecoveryReceipt, object)
    }
    pub fn arm(&mut self, endpoint: &RecordedEndpoint, point: PausePoint) -> usize {
        let token = ok(self.control.pause_next(endpoint, point));
        assert!(self.tokens.len() < 64);
        self.tokens.push((self.control.clone(), token));
        self.tokens.len() - 1
    }
    pub fn entered(&self, index: usize, operation: u64, permitted: bool) -> BarrierSnapshot {
        let s = ok(self.tokens[index].1.wait_until_entered(2000));
        assert!(s.entered && !s.settled);
        assert_eq!(s.operation_id, operation);
        assert_eq!(s.permit_issued, permitted);
        s
    }
    pub fn release(&self, index: usize) {
        ok(self.tokens[index].0.release(&self.tokens[index].1));
        let s = ok(self.tokens[index].1.wait_until_settled(2000));
        assert!(s.entered && s.released && s.settled);
    }
    pub fn stop(&self, index: usize) {
        ok(self.tokens[index].0.stop_at_phase(&self.tokens[index].1));
        assert!(ok(self.tokens[index].1.wait_until_settled(2000)).settled);
    }
    pub fn fence(&mut self) -> QuiescedStoreOwner {
        assert!(!self.fenced);
        let before = public_json(&ok(self.control.evidence()));
        let token = ok(self.control.quiesce(2000));
        let fence = token.fence_snapshot();
        assert_fence(&fence);
        let after_evidence = ok(self.control.evidence());
        let after = public_json(&after_evidence);
        let raw = encoded(&fence, 2 * MIB); // Direct typed serde bytes, not a Value normalization.
        let mut recorder = lock(&self.recorder);
        assert_eq!(recorder.epochs.len(), self.host.epoch as usize);
        assert!(recorder.epochs.len() < 2);
        recorder.match_epoch(self.host.epoch, &after_evidence);
        recorder.epochs.push(ArchivedEpoch {
            epoch: self.host.epoch,
            before,
            after,
            fence,
            raw,
        });
        self.fenced = true;
        token
    }
    pub fn reopen(&mut self, token: QuiescedStoreOwner) -> FenceSnapshot {
        match self.try_reopen(token) {
            Ok(snapshot) => snapshot,
            Err(_) => panic!("required actual Store reopen failed"),
        }
    }
    pub fn try_reopen(
        &mut self,
        token: QuiescedStoreOwner,
    ) -> Result<FenceSnapshot, ReopenFailure> {
        assert!(self.fenced && self.host.epoch == 0 && lock(&self.recorder).reopen.is_none());
        let snapshot = token.fence_snapshot();
        assert_fence(&snapshot);
        let before = capture_state(&self.host);
        self.save_state("before_reopen", before.clone(), 0);
        let result = StoreFixture::reopen(token);
        match result {
            Ok((actual, control)) => {
                let control = Arc::new(control);
                self.host = RecordedHost {
                    actual,
                    control: control.clone(),
                    epoch: 1,
                    recorder: self.recorder.clone(),
                };
                self.control = control;
                self.fenced = false;
                let after = capture_state(&self.host);
                self.save_state("after_reopen", after, 1);
                let evidence = ok(self.control.evidence());
                let witnesses = evidence
                    .objects
                    .iter()
                    .map(|o| {
                        let w = o.reopen_witness.as_ref().expect("actual reopen witness");
                        let prior = snapshot
                            .objects
                            .iter()
                            .find(|p| p.selected_object_id == o.selected.selected_object_id)
                            .unwrap();
                        assert_eq!(w.fence_snapshot_hash, hash(&encoded(&snapshot, 2 * MIB)));
                        assert_eq!(w.prior_service_epoch, prior.service_epoch_at_fence);
                        assert_eq!(
                            w.actual_service_epoch,
                            prior.reserved_successor_epoch.unwrap()
                        );
                        reopen_json(w)
                    })
                    .collect::<Vec<_>>();
                lock(&self.recorder).reopen = Some(
                    json!({"producer_epoch":0,"before_fence_ref":null,"actual_error_status_or_null":null,"success":true,"returned_fence_byte_len_or_null":null,"returned_fence_sha256_or_null":null,"before_files_state_sha256":"","after_files_state_sha256":"","actual_new_owner_witness_or_null":witnesses}),
                );
                Ok(snapshot)
            }
            Err(failure) => {
                let returned = encoded(&failure.owner.fence_snapshot(), 2 * MIB);
                assert_eq!(returned, encoded(&snapshot, 2 * MIB));
                let after = capture_state(&self.host);
                assert_eq!(before, after, "actual failed reopen unchanged scoped state");
                self.save_state("after_reopen", after, 0);
                lock(&self.recorder).reopen = Some(
                    json!({"producer_epoch":0,"before_fence_ref":null,"actual_error_status_or_null":failure.error.status as u32,"success":false,"returned_fence_byte_len_or_null":returned.len(),"returned_fence_sha256_or_null":hex::encode(hash(&returned)),"before_files_state_sha256":"","after_files_state_sha256":"","actual_new_owner_witness_or_null":null}),
                );
                Err(failure)
            }
        }
    }
    fn save_state(&self, phase: &'static str, body: StateBody, epoch: u32) {
        let cap = if matches!(phase, "before_reopen" | "after_reopen") {
            122880
        } else {
            40960
        };
        encoded(&body, cap - 256);
        let mut ledger = lock(&self.recorder);
        assert!(ledger.states.len() < 5 && ledger.states.iter().all(|s| s.phase != phase));
        ledger.states.push(SavedState { phase, epoch, body });
    }
    pub fn negative_phase(&self, phase: &'static str) {
        self.save_state(phase, capture_state(&self.host), self.host.epoch);
    }
    pub fn raw_negative(&self, raw: &[u8; 88]) -> Result<Envelope, StoreStatus> {
        let before = capture_state(&self.host);
        let result = decode_envelope_wire(raw);
        let after = capture_state(&self.host);
        assert_eq!(before, after);
        let error = *result.as_ref().expect_err("actual raw codec denial");
        let mut ledger = lock(&self.recorder);
        assert!(ledger.negative.len() < 7);
        assert_eq!(
            before,
            ledger
                .states
                .iter()
                .find(|s| s.phase == "selectedtext_before_allocation")
                .expect("actual raw phase witness")
                .body
        );
        let ordinal = ledger.negative.len();
        let record = json!({"ordinal":ordinal,"producer_epoch":self.host.epoch,"stage":"raw_decoder","input_kind":"raw_wire","raw_wire_or_null":raw.to_vec(),"typed_envelope_or_null":null,"input_diagnostic_sha256":hex::encode(hash(raw)),"actual_returned_error_status":error as u32,"actor_or_null":null,"endpoint_class_or_null":null,"selected_object_id_or_null":null,"before_state_sha256":"","after_state_sha256":""});
        encoded(&record, 4096);
        ledger.negative.push(record);
        result
    }
    pub fn deny_aliases(&self, e: &RecordedEndpoint, descriptor: &ObjectDescriptor) {
        assert_eq!(Handle::unpack(descriptor.handle.pack()), descriptor.handle);
        assert_eq!(descriptor.object_generation, descriptor.handle.generation);
        assert!((1..=u16::MAX as u32).contains(&descriptor.handle.index));
        assert!((1..=u32::MAX as u64).contains(&descriptor.handle.generation));
        let phase = match descriptor.kind {
            SharedObjectKind::SelectedText => "selectedtext_before_allocation",
            SharedObjectKind::DraftSource => "draft_after_invalid_submissions",
            SharedObjectKind::Receipt => "receipt_after_commit",
        };
        if !lock(&self.recorder).states.iter().any(|s| s.phase == phase) {
            self.negative_phase(phase);
        }
        let before = capture_state(&self.host);
        assert_eq!(
            before,
            lock(&self.recorder)
                .states
                .iter()
                .find(|s| s.phase == phase)
                .expect("actual lease phase witness")
                .body
        );
        let mut index = descriptor.clone();
        index.handle.index += 1 << 16;
        let mut generation = descriptor.clone();
        generation.handle.generation += 1 << 32;
        let mut both = generation.clone();
        both.object_generation = both.handle.generation;
        let mut mismatch = descriptor.clone();
        mismatch.object_generation += 1;
        for changed in [index, generation, both, mismatch] {
            assert_eq!(changed.handle.pack(), descriptor.handle.pack());
            for method in ["read_lease", "write_lease"] {
                let error = if method == "read_lease" {
                    self.host.read_lease(&e.peer, &changed).err()
                } else {
                    self.host.write_lease(&e.peer, &changed).err()
                }
                .expect("actual typed descriptor denial");
                assert_eq!(error, StoreStatus::Denied);
                let after = capture_state(&self.host);
                assert_eq!(before, after);
                let mut ledger = lock(&self.recorder);
                assert!(ledger.denials.len() < 24);
                let ordinal = ledger.denials.len();
                let row = json!({"ordinal":ordinal,"producer_epoch":self.host.epoch,"recorded_lease_call_ordinal":ordinal,"method":method,"actor":e.peer.context.actor,"endpoint_class":class_name(&e.peer.context.class),"selected_object_id":e.peer.context.object,"canonical_descriptor":descriptor_json(descriptor),"attempted_descriptor":descriptor_json(&changed),"actual_returned_error_status":error as u32,"before_state_sha256":"","after_state_sha256":"","state_phase":phase});
                encoded(&row, 2048);
                ledger.denials.push(row);
            }
        }
    }
    pub fn object(&self, id: u64) -> ObjectEvidence {
        let mut evidence = ok(self.control.evidence());
        assert_ledger(&evidence);
        let index = evidence
            .objects
            .iter()
            .position(|o| o.selected.selected_object_id == id)
            .unwrap();
        evidence.objects.remove(index)
    }
    pub fn counts(&self, object: u64) -> (u32, u32, u32) {
        let o = self.object(object);
        (
            o.mutation_dispatch_count,
            o.permit_count,
            o.transition_count,
        )
    }
    pub fn files(&self) -> Vec<FileWitness> {
        scan(&self.root())
    }
    pub fn journal(&self, object: u64) -> (PathBuf, EditorSelectionJournalV0) {
        let matches: Vec<_> = self
            .files()
            .into_iter()
            .filter_map(|file| {
                if file.path.file_name()?.to_str()? != "selection.json" {
                    return None;
                }
                let parsed = EditorSelectionJournalV0::decode_canonical(&file.bytes).ok()?;
                if parsed.selected_object_id == object {
                    Some((self.root().join(file.path), parsed))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(matches.len(), 1, "one actual canonical journal per object");
        matches.into_iter().next().unwrap()
    }
    pub fn disk_selected(&self, object: u64) -> Vec<u8> {
        let (journal_path, journal) = self.journal(object);
        let evidence = self.object(object);
        assert_eq!(journal.digest().unwrap(), evidence.journal_hash);
        assert_eq!(journal.transition_sequence, evidence.journal_sequence);
        assert_eq!(journal.current, evidence.selected);
        let cas = journal_path.parent().unwrap().join("cas");
        let id = journal.current.content_id().unwrap();
        let blob = bounded_read(&cas.join(format!("{}.blob", id.hash_hex())), 4096);
        let raw_manifest =
            bounded_read(&cas.join(format!("{}.manifest.json", id.hash_hex())), 2048);
        let manifest: Manifest = ok(serde_json::from_slice(&raw_manifest));
        assert_eq!(manifest.content_id, id.as_str());
        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.kind, "editor-selected-private");
        assert_eq!(manifest.channels, ["dev-host-editor"]);
        assert_eq!(manifest.signatures.len(), 1);
        assert_eq!(manifest.size_bytes as usize, blob.len());
        let mut trusted = TrustedKeys::new();
        let key = ed25519_dalek::SigningKey::from_bytes(&KEY_SEED).verifying_key();
        ok(trusted.add_ed25519_key(KEY_ID.into(), key.as_bytes()));
        let config = SignatureValidationConfig {
            policy: SignaturePolicy::RequireSignature,
            trusted_key_ids: vec![KEY_ID.into()],
            max_signature_age_secs: None,
            trusted_keys: trusted,
        };
        assert!(matches!(
            validate_manifest_signatures(
                &manifest.signatures,
                &ok(manifest_signing_bytes(&manifest)),
                &config
            ),
            SignatureValidationResult::Valid
        ));
        let owner_raw = bounded_read(&cas.join(format!("{}.ownership.json", id.hash_hex())), 512);
        let owner: store_service::domain_visibility::ArtifactOwner =
            ok(serde_json::from_slice(&owner_raw));
        assert_eq!(owner.domain_id, journal.owner_id);
        assert!(!owner.is_global);
        assert_eq!(owner.content_id.as_str(), id.as_str());
        assert_eq!(hash(&blob), journal.current.content_hash);
        assert_eq!(
            hash(&ok(serde_json::to_vec(&manifest))),
            journal.current.manifest_hash
        );
        assert_eq!(evidence.blob_hash, hash(&blob));
        assert_eq!(evidence.manifest_hash, journal.current.manifest_hash);
        assert_eq!(evidence.ownership_hash, hash(&owner_raw));
        blob
    }
    pub fn emit(mut self, case: &str, leg: u32, extra: Value) {
        let legs = match case {
            "ui1_1_revoke_paused_key_frame_and_save" => 3,
            "ui1_1_stalled_session_leaves_other_session_usable" => 1,
            "ui1_1_clock_capacity_and_counter_limits_fail_closed" => 16,
            "ui1_1_store_commit_reopen_preserves_prior_blob" => 5,
            "ui1_1_store_same_base_conflict_and_operation_reuse" => 1,
            "ui1_1_store_lost_reply_reconciles_without_mutation_replay" => 4,
            "ui1_1_store_recovery_validates_atomic_selection_and_receipt" => 18,
            _ => panic!("unfrozen evidence case"),
        };
        assert!(leg < legs);
        for (controller, token) in &self.tokens {
            let _ = controller.release(token);
            assert!(ok(token.wait_until_settled(2000)).settled);
        }
        if !self.fenced {
            let _ = self.fence();
        }
        {
            let ledger = lock(&self.recorder);
            assert!(
                ledger.pending.is_empty() && ledger.pending_slots.is_empty(),
                "every original owned caller must actually join"
            );
        }
        let files = self.files();
        let recorder = self.recorder.clone();
        let root = self.root.take().unwrap();
        let (export, id) = {
            let r = lock(&recorder);
            (r.run.is_some(), r.id.clone())
        };
        let current_root = ok(fs::symlink_metadata(root.path()));
        assert!(current_root.is_dir() && !current_root.file_type().is_symlink());
        {
            let r = lock(&recorder);
            assert_eq!(
                (current_root.dev(), current_root.ino()),
                (r.device, r.inode)
            );
        }
        let filesystem = export.then(|| filesystem_json(root.path(), &id));
        let root_path = root.path().to_path_buf();
        self.finished = true;
        drop(self); // Consumes only fixture-owned holders, not external passive aliases.
        let close = root.close();
        let absence = fs::symlink_metadata(&root_path);
        let export = lock(&recorder).run.is_some();
        if export {
            finish_evidence(
                &recorder,
                case,
                leg,
                extra,
                files,
                filesystem.unwrap(),
                (&close, &absence),
            );
        }
        assert!(
            close.is_ok(),
            "explicit TempDir.close failed; retained cleanup error"
        );
        assert!(
            matches!(absence,Err(ref e) if e.kind()==std::io::ErrorKind::NotFound),
            "actual root absence not proven"
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        for (controller, token) in &self.tokens {
            let _ = controller.release(token);
        }
        let joined = self.fenced || self.control.quiesce(2000).is_ok();
        let deadline = Instant::now() + Duration::from_millis(2500);
        loop {
            let complete = {
                let r = lock(&self.recorder);
                r.pending.values().all(JoinHandle::is_finished)
            };
            if complete || Instant::now() >= deadline {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        // Only actual held handles that have finished may be joined here. A timed
        // out cleanup retains the owned root; Drop emits no passing evidence.
        let ready = {
            let mut r = lock(&self.recorder);
            let ids = r
                .pending
                .iter()
                .filter_map(|(id, h)| h.is_finished().then_some(*id))
                .collect::<Vec<_>>();
            assert!(ids.len() <= 64);
            ids.into_iter()
                .map(|id| {
                    r.pending_slots.remove(&id);
                    r.pending.remove(&id).unwrap()
                })
                .collect::<Vec<_>>()
        };
        for handle in ready {
            let _ = handle.join();
        }
        let callers_joined = {
            let r = lock(&self.recorder);
            r.pending.is_empty() && r.pending_slots.is_empty()
        };
        if !joined || !callers_joined {
            if let Some(root) = self.root.take() {
                let retained = root.keep();
                eprintln!(
                    "INCOMPLETE owned fixture cleanup; retained {}",
                    retained.display()
                );
            }
            if !thread::panicking() {
                panic!("INCOMPLETE owned Store cleanup");
            }
        }
    }
}
fn descriptor_json(d: &ObjectDescriptor) -> Value {
    json!({"handle":{"kind":d.handle.kind as u8,"index":d.handle.index,"generation":d.handle.generation},"kind":match d.kind {SharedObjectKind::SelectedText=>"SelectedText",SharedObjectKind::DraftSource=>"DraftSource",SharedObjectKind::Receipt=>"Receipt"},"object_generation":d.object_generation,"byte_len":d.byte_len})
}

pub struct Attempt {
    pub operation: u64,
    pub source: ObjectDescriptor,
    pub request: artifact::Commit,
}
pub fn attempt(
    f: &Fixture,
    endpoint: &RecordedEndpoint,
    actor: &EditorActorV0,
    base: u64,
    bytes: &[u8],
    request: u64,
) -> Attempt {
    let operation = allocate(&f.host, endpoint, actor, base, request);
    let source = ok(f.host.draft_source(&endpoint.peer, base, bytes));
    Attempt {
        operation,
        request: artifact::Commit {
            request_id: request + 1,
            session_id: actor.session_id,
            session_generation: actor.session_generation,
            expected_revision: base,
            operation_id: operation,
            source_shm: source.handle.pack(),
            byte_len: source.byte_len,
            reserved: 0,
        },
        source,
    }
}
pub fn exchange<Q: ArtifactMessage>(
    host: &RecordedHost,
    endpoint: &RecordedEndpoint,
    request: Q,
) -> Envelope {
    let input = ok(request.encode(endpoint.handle));
    let raw = ok(encode_envelope_wire(&input));
    let parsed = ok(decode_envelope_wire(&raw));
    let reply = ok(host.dispatch(&endpoint.peer, &parsed));
    let reply = ok(decode_envelope_wire(&ok(encode_envelope_wire(&reply))));
    bind_reply(&parsed, &reply);
    reply
}
pub fn bind_reply(request: &Envelope, reply: &Envelope) {
    assert_eq!(request.protocol, 368);
    assert!(matches!(request.msg_type, 1 | 3 | 5 | 7));
    assert_eq!(reply.protocol, request.protocol);
    assert_eq!(reply.msg_type, request.msg_type + 1);
    assert_eq!(reply.handle, request.handle);
    assert_eq!(
        &reply.payload[..8],
        &request.payload[..8],
        "actual request correlation"
    );
}
pub fn decode<R: ArtifactMessage>(reply: &Envelope) -> R {
    ok(R::decode(reply))
}
pub fn allocate(
    host: &RecordedHost,
    endpoint: &RecordedEndpoint,
    actor: &EditorActorV0,
    base: u64,
    request: u64,
) -> u64 {
    let reply = exchange(
        host,
        endpoint,
        artifact::AllocateSaveId {
            request_id: request,
            session_id: actor.session_id,
            session_generation: actor.session_generation,
            expected_revision: base,
        },
    );
    let r: artifact::AllocateSaveIdReply = decode(&reply);
    assert_eq!((r.status, r.reserved), (0, 0));
    assert!(r.operation_id > 0);
    r.operation_id
}
pub fn read(
    host: &RecordedHost,
    endpoint: &RecordedEndpoint,
    actor: &EditorActorV0,
    request: u64,
) -> (u64, Vec<u8>) {
    let reply = exchange(
        host,
        endpoint,
        artifact::ReadSelected {
            request_id: request,
            session_id: actor.session_id,
            session_generation: actor.session_generation,
            instance_id: actor.instance_id,
            instance_generation: actor.instance_generation,
        },
    );
    let r: artifact::ReadSelectedReply = decode(&reply);
    assert_eq!(r.status, 0);
    assert_eq!(r.object_generation, Handle::unpack(r.data_shm).generation);
    let descriptor = ObjectDescriptor {
        handle: Handle::unpack(r.data_shm),
        kind: SharedObjectKind::SelectedText,
        object_generation: r.object_generation,
        byte_len: r.byte_len,
    };
    let lease = ok(host.read_lease(&endpoint.peer, &descriptor));
    assert!((64..=4160).contains(&r.byte_len));
    let mut bytes = vec![0; r.byte_len as usize];
    ok(lease.copy_into(0, &mut bytes));
    let header = ok(EditorTextHeaderV0::decode_le(&bytes[..64]));
    ok(header.validate_bytes(&bytes[64..]));
    assert_eq!(header.revision, r.revision);
    (r.revision, bytes[64..].to_vec())
}
pub fn result_fields(reply: &Envelope) -> (u32, u64, u64, u64, u32) {
    match reply.msg_type {
        6 => {
            let r: artifact::CommitReply = decode(reply);
            (
                r.status,
                r.operation_id,
                r.revision,
                r.receipt_shm,
                r.receipt_len,
            )
        }
        8 => {
            let r: artifact::SaveStatusReply = decode(reply);
            (
                r.status,
                r.operation_id,
                r.revision,
                r.receipt_shm,
                r.receipt_len,
            )
        }
        _ => panic!("not a generated commit/status reply"),
    }
}
pub fn receipt(
    host: &RecordedHost,
    endpoint: &RecordedEndpoint,
    reply: &Envelope,
) -> EditorSaveReceiptV0 {
    let (status, operation, revision, raw, len) = result_fields(reply);
    assert_eq!((status, len), (0, 176));
    assert!(raw > 0);
    let handle = Handle::unpack(raw);
    assert_eq!(handle.pack(), raw);
    let descriptor = ObjectDescriptor {
        handle,
        kind: SharedObjectKind::Receipt,
        object_generation: handle.generation,
        byte_len: 176,
    };
    assert!(host.write_lease(&endpoint.peer, &descriptor).is_err());
    let lease = ok(host.read_lease(&endpoint.peer, &descriptor));
    let mut bytes = [0; 176];
    ok(lease.copy_into(0, &mut bytes));
    let result = ok(EditorSaveReceiptV0::decode_le(&bytes));
    assert_eq!(
        (result.operation_id, result.result_revision),
        (operation, revision)
    );
    result
}
pub fn status_reply(
    host: &RecordedHost,
    endpoint: &RecordedEndpoint,
    actor: &EditorActorV0,
    operation: u64,
    request: u64,
) -> Envelope {
    exchange(
        host,
        endpoint,
        artifact::SaveStatus {
            request_id: request,
            session_id: actor.session_id,
            session_generation: actor.session_generation,
            operation_id: operation,
        },
    )
}
pub fn status(reply: &Envelope, offset: usize, expected: StoreStatus) {
    assert_eq!(
        u32::from_le_bytes(reply.payload[offset..offset + 4].try_into().unwrap()),
        expected as u32
    );
}
pub fn redacted(reply: &Envelope, offset: usize, expected: StoreStatus) {
    status(reply, offset, expected);
    for (index, byte) in reply.payload.iter().enumerate() {
        if index >= 8 && !(offset..offset + 4).contains(&index) {
            assert_eq!(*byte, 0, "denial data byte {index}");
        }
    }
}
pub fn unknown(reply: &Envelope, operation: u64) {
    assert_eq!(result_fields(reply), (10, operation, 0, 0, 0));
}
pub struct Pending {
    receiver: Receiver<Envelope>,
    recorder: Arc<Mutex<Recorder>>,
    id: usize,
    request_wire: [u8; 88],
}
impl Pending {
    pub fn start(
        host: &RecordedHost,
        endpoint: &RecordedEndpoint,
        request: artifact::Commit,
    ) -> Self {
        let host = host.clone();
        let peer = endpoint.peer.clone();
        let request_wire = ok(encode_envelope_wire(&ok(request.encode(endpoint.handle))));
        let q = ok(decode_envelope_wire(&request_wire));
        let id = {
            let mut ledger = lock(&host.recorder);
            assert!(ledger.pending_slots.len() < 64);
            let id = ledger.begin(host.epoch, &peer, &q, "pending_thread");
            assert!(ledger.pending_slots.insert(id));
            ledger.pending_high = ledger.pending_high.max(ledger.pending_slots.len());
            id
        };
        let (sender, receiver) = mpsc::channel();
        let recorder = host.recorder.clone();
        let handle = match thread::Builder::new().spawn(move || {
            let result = host.dispatch_reserved(id, &peer, &q);
            let _ = sender.send(ok(result));
        }) {
            Ok(handle) => handle,
            Err(_) => {
                assert!(lock(&recorder).pending_slots.remove(&id));
                panic!("INCOMPLETE actual caller thread spawn failed");
            }
        };
        {
            let mut ledger = lock(&recorder);
            assert!(ledger.pending.len() < 64);
            assert!(ledger.pending.insert(id, handle).is_none());
        }
        Self {
            receiver,
            recorder,
            id,
            request_wire,
        }
    }
    pub fn finish(self) -> Envelope {
        let response = self
            .receiver
            .recv_timeout(Duration::from_millis(2500))
            .unwrap_or_else(|_| panic!("INCOMPLETE caller did not observe bounded response"));
        let deadline = Instant::now() + Duration::from_millis(2500);
        while !lock(&self.recorder)
            .pending
            .get(&self.id)
            .expect("actual held pending caller")
            .is_finished()
        {
            assert!(
                Instant::now() < deadline,
                "INCOMPLETE original caller join deadline"
            );
            thread::sleep(Duration::from_millis(1));
        }
        let handle = lock(&self.recorder)
            .pending
            .remove(&self.id)
            .expect("actual held finished pending caller");
        let thread_id = format!("{:?}", handle.thread().id());
        let joined = handle.join();
        {
            let mut ledger = lock(&self.recorder);
            assert!(ledger.joins.len() < 512);
            let epoch = ledger.calls[self.id].epoch;
            assert!(ledger.pending_slots.remove(&self.id));
            let row = json!({"producer_epoch":epoch,"call_ordinal":self.id,"native_test_pid":std::process::id(),"rust_thread_id":thread_id,"completed_join":true,"join_outcome":if joined.is_ok(){"Returned"}else{"Panicked"}});
            encoded(&row, 512);
            ledger.joins.push(row);
        }
        assert!(joined.is_ok(), "actual owned caller join");
        let response = ok(decode_envelope_wire(&ok(encode_envelope_wire(&response))));
        bind_reply(&ok(decode_envelope_wire(&self.request_wire)), &response);
        response
    }
}
pub fn operation_io(f: &Fixture, object: u64, operation: u64) -> Vec<Value> {
    let evidence = ok(f.control.evidence());
    assert_ledger(&evidence);
    evidence
        .io_events
        .iter()
        .filter(|event| event.selected_object_id == object && event.operation_id == operation)
        .map(|event| {
            json!({
                "object":event.selected_object_id,"operation":event.operation_id,
                "writer":event.writer_id,"sequence":event.sequence,
                "point":point_name(&event.point),"outcome":outcome_name(&event.outcome),
                "artifact_hash":event.artifact_hash
            })
        })
        .collect()
}
pub fn assert_ledger(e: &StoreEvidence) {
    for object in &e.objects {
        for operation in &object.operations {
            if operation.state == EditorOperationStateV0::Allocated {
                assert_eq!(
                    operation.closure,
                    EditorClosureV0::None,
                    "published Allocated record must retain the pure contract's open state"
                );
                assert!(
                    operation.binding.is_none()
                        && operation.permit.is_none()
                        && operation.receipt.is_none()
                        && operation.successor.is_none()
                );
            }
            if operation.state == EditorOperationStateV0::Noncommit && operation.binding.is_none() {
                assert!(
                    operation.permit.is_none()
                        && operation.receipt.is_none()
                        && operation.successor.is_none(),
                    "unsubmitted closure cannot invent a receipt"
                );
            }
        }
    }
    let l = &e.ledger;
    assert!(
        l.sessions <= 2
            && l.selected_objects <= 2
            && l.original_instances <= 16
            && l.endpoints <= 64
            && l.shared_objects <= 32
    );
    assert!(
        l.active_io_workers <= 2
            && l.held_service_producers <= 64
            && l.active_supervisors <= l.held_service_producers
    );
    assert!(
        e.exchanges.len() <= 256
            && e.io_events.len() <= 1024
            && e.barriers.len() <= 64
            && e.objects.len() <= 2
    );
    assert!(
        l.operations_per_object.len() <= 2
            && l.cas_entries_per_object.len() <= 2
            && l.bytes_per_object.len() <= 2
    );
    assert!(
        l.operations_per_object
            .iter()
            .all(|(_, count)| *count <= 16)
    );
    assert!(
        l.cas_entries_per_object
            .iter()
            .all(|(_, count)| *count <= 64)
    );
    assert!(
        l.bytes_per_object
            .iter()
            .all(|(_, count)| *count <= 2 * 1024 * 1024)
    );
}
pub fn assert_fence(f: &FenceSnapshot) {
    assert_eq!((f.schema_version, f.live_ownership_after_join), (1, 0));
    assert!(f.namespace_id > 0 && f.fence_id > 0);
    assert!(f.service_producers.len() <= 64 && f.object_workers.len() <= 2 && f.objects.len() <= 2);
    let mut ids = BTreeSet::new();
    for producer in f.service_producers.iter().chain(&f.object_workers) {
        encoded(producer, 2048);
        assert!(producer.producer_id > 0 && producer.producer_generation > 0);
        assert!(ids.insert((producer.producer_id, producer.producer_generation)));
        assert_eq!(producer.native_pid, std::process::id());
        assert!(!producer.rust_thread_id.is_empty() && producer.rust_thread_id.len() <= 64);
        assert!(producer.completed_join && !producer.live_ownership_after_join);
        assert!(matches!(
            &producer.join_outcome,
            ProducerJoinOutcome::Returned | ProducerJoinOutcome::Panicked
        ));
        if let Some(barrier) = &producer.settled_barrier {
            assert!(barrier.settled);
            assert_eq!(Some(barrier.operation_id), producer.operation_id);
            assert_eq!(Some(barrier.request_id), producer.request_id);
        }
    }
    assert!(
        f.service_producers
            .iter()
            .all(|p| matches!(&p.kind, ProducerKind::Dispatcher | ProducerKind::Supervisor))
    );
    assert!(
        f.object_workers
            .iter()
            .all(|p| matches!(&p.kind, ProducerKind::ObjectIoWorker))
    );
    for o in &f.objects {
        encoded(o, 512 * 1024);
        assert!(
            o.known_durable_operations.len() <= 16
                && o.issued_runtime_permits.len() <= 16
                && o.original_closures.len() <= 16
        );
        assert_eq!(
            o.reserved_successor_epoch,
            o.service_epoch_at_fence.checked_add(1)
        );
        assert!(o.durable_journal_sequence > 0 && o.admission_epoch_min > 0);
        assert!(
            o.known_durable_operations
                .windows(2)
                .all(|r| r[0].allocation.operation_id < r[1].allocation.operation_id)
        );
        assert!(
            o.issued_runtime_permits.windows(2).all(
                |r| r[0].binding.allocation.operation_id < r[1].binding.allocation.operation_id
            )
        );
        assert!(
            o.original_closures
                .windows(2)
                .all(|r| r[0].allocation.operation_id < r[1].allocation.operation_id)
        );
        for record in &o.known_durable_operations {
            ok(record.allocation.validate());
            assert_eq!(
                (
                    record.allocation.actor.owner_id,
                    record.allocation.selected_object_id,
                    record.allocation.selected_generation
                ),
                (o.owner_id, o.selected_object_id, o.selected_generation)
            );
            assert!(record.allocation.operation_id <= o.operation_high_water_min);
            assert!(record.allocated_service_epoch <= o.service_epoch_at_fence);
            if let Some(binding) = &record.binding {
                ok(binding.validate());
                assert_eq!(binding.allocation, record.allocation);
            }
        }
        for permit in &o.issued_runtime_permits {
            ok(permit.validate());
            assert!(
                permit.writer_id <= o.writer_high_water_min
                    && permit.service_epoch <= o.service_epoch_at_fence
                    && permit.admission_epoch <= o.admission_epoch_min
            );
        }
        for closure in &o.original_closures {
            ok(closure.allocation.validate());
            assert!(
                closure.original_request_id > 0
                    && closure.original_producer_id > 0
                    && closure.original_producer_generation > 0
            );
            if let Some(binding) = &closure.submitted_binding {
                ok(binding.validate());
                assert_eq!(binding.allocation, closure.allocation);
            }
        }
    }
    assert!(
        f.service_producers
            .windows(2)
            .all(|p| (p[0].producer_id, p[0].producer_generation)
                < (p[1].producer_id, p[1].producer_generation))
    );
    assert!(
        f.object_workers
            .windows(2)
            .all(|p| (p[0].producer_id, p[0].producer_generation)
                < (p[1].producer_id, p[1].producer_generation))
    );
    assert!(
        f.objects
            .windows(2)
            .all(|p| p[0].selected_object_id < p[1].selected_object_id)
    );
}
pub fn assert_io_completed(f: &Fixture, operation: u64) {
    let evidence = ok(f.control.evidence());
    let object = evidence
        .objects
        .iter()
        .find(|o| {
            o.operations
                .iter()
                .any(|r| r.allocation.operation_id == operation)
        })
        .unwrap();
    let record = object
        .operations
        .iter()
        .find(|r| r.allocation.operation_id == operation)
        .unwrap();
    let permit = record.permit.as_ref().unwrap();
    let completed = evidence
        .io_events
        .iter()
        .filter(|event| {
            event.selected_object_id == object.selected.selected_object_id
                && event.operation_id == operation
                && matches!(&event.outcome, IoOutcome::Completed)
        })
        .collect::<Vec<_>>();
    assert!(
        completed
            .windows(2)
            .all(|events| events[0].sequence < events[1].sequence)
    );
    for point in [
        IoPoint::CandidateIntent,
        IoPoint::CandidateBlobWrite,
        IoPoint::CandidateBlobSync,
        IoPoint::CandidateManifest,
        IoPoint::CandidateOwner,
        IoPoint::CandidateDirectorySync,
        IoPoint::CandidateReadback,
        IoPoint::JournalWrite,
        IoPoint::JournalSync,
        IoPoint::JournalRename,
        IoPoint::JournalDirectorySync,
        IoPoint::JournalReadback,
    ] {
        assert!(
            completed
                .iter()
                .any(|event| point_name(&event.point) == point_name(&point)),
            "missing completed actual IO {}",
            point_name(&point)
        );
    }
    for point in [IoPoint::CandidateBlobWrite, IoPoint::CandidateBlobSync] {
        let event = completed
            .iter()
            .find(|e| point_name(&e.point) == point_name(&point))
            .unwrap();
        assert_eq!(event.writer_id, permit.writer_id);
        assert_eq!(
            event.artifact_hash,
            Some(permit.binding.source.content_hash)
        );
    }
    let manifest = completed
        .iter()
        .find(|e| matches!(&e.point, IoPoint::CandidateManifest))
        .unwrap();
    assert_eq!(manifest.artifact_hash, Some(object.manifest_hash));
    let candidate = completed
        .iter()
        .find(|e| matches!(&e.point, IoPoint::CandidateReadback))
        .unwrap();
    let final_read = completed
        .iter()
        .find(|e| {
            matches!(&e.point, IoPoint::JournalReadback)
                && e.artifact_hash == Some(object.journal_hash)
        })
        .expect("actual selected+receipt journal readback");
    let rename = completed
        .iter()
        .rfind(|e| {
            matches!(&e.point, IoPoint::JournalRename)
                && e.sequence > candidate.sequence
                && e.sequence < final_read.sequence
        })
        .unwrap();
    let directory = completed
        .iter()
        .find(|e| {
            matches!(&e.point, IoPoint::JournalDirectorySync)
                && e.sequence > rename.sequence
                && e.sequence < final_read.sequence
        })
        .unwrap();
    assert!(
        candidate.sequence < rename.sequence
            && rename.sequence < directory.sequence
            && directory.sequence < final_read.sequence
    );
    assert_eq!(final_read.writer_id, permit.writer_id);
}
pub fn barrier_json(b: &BarrierSnapshot) -> Value {
    json!({"request":b.request_id,"operation":b.operation_id,"entered":b.entered,"released":b.released,"settled":b.settled,"permit":b.permit_issued})
}
fn class_name(class: &EndpointClass) -> &'static str {
    match class {
        EndpointClass::Artifact => "artifact",
        EndpointClass::RecoveryReceipt => "recovery_receipt",
    }
}
fn outcome_name(outcome: &IoOutcome) -> &'static str {
    match outcome {
        IoOutcome::Began => "began",
        IoOutcome::Completed => "completed",
        IoOutcome::SimulatedFailure => "simulated_failure",
        IoOutcome::ActualFailure => "actual_failure",
    }
}
fn point_name(point: &IoPoint) -> &'static str {
    match point {
        IoPoint::CandidateIntent => "candidate_intent",
        IoPoint::CandidateBlobWrite => "candidate_blob_write",
        IoPoint::CandidateBlobSync => "candidate_blob_sync",
        IoPoint::CandidateManifest => "candidate_manifest",
        IoPoint::CandidateOwner => "candidate_owner",
        IoPoint::CandidateDirectorySync => "candidate_directory_sync",
        IoPoint::CandidateReadback => "candidate_readback",
        IoPoint::JournalWrite => "journal_write",
        IoPoint::JournalSync => "journal_sync",
        IoPoint::JournalRename => "journal_rename",
        IoPoint::JournalDirectorySync => "journal_directory_sync",
        IoPoint::JournalReadback => "journal_readback",
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct FileWitness {
    pub path: PathBuf,
    pub kind: &'static str,
    pub bytes: Vec<u8>,
}
pub fn bounded_read(path: &Path, max: usize) -> Vec<u8> {
    let mut file = ok(OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path));
    let before = ok(file.metadata());
    assert!(before.is_file() && before.len() <= max as u64);
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    while bytes.len() < max {
        let remaining = max - bytes.len();
        let n = ok(file.read(&mut chunk[..remaining.min(8192)]));
        if n == 0 {
            break;
        }
        assert!(n <= remaining);
        bytes.extend_from_slice(&chunk[..n]);
    }
    // The overlength guard byte is never retained or used as truncated evidence.
    assert_eq!(ok(file.read(&mut chunk[..1])), 0, "bounded read overflow");
    let after = ok(file.metadata());
    assert_eq!(
        (
            before.dev(),
            before.ino(),
            before.len(),
            before.mtime(),
            before.mtime_nsec()
        ),
        (
            after.dev(),
            after.ino(),
            after.len(),
            after.mtime(),
            after.mtime_nsec()
        )
    );
    bytes
}
fn scan(root: &Path) -> Vec<FileWitness> {
    fn visit(
        root: &Path,
        directory: &Path,
        depth: usize,
        dirs: &mut usize,
        bytes_total: &mut usize,
        output: &mut Vec<FileWitness>,
    ) {
        assert!(depth <= 4 && *dirs <= 16);
        let directory_meta = ok(fs::symlink_metadata(directory));
        assert!(directory_meta.is_dir() && !directory_meta.file_type().is_symlink());
        let mut entries = Vec::new();
        for entry in ok(fs::read_dir(directory)) {
            assert!(entries.len() < 160, "directory entry cap before retention");
            entries.push(ok(entry));
        }
        for entry in entries {
            let path = entry.path();
            let metadata = ok(fs::symlink_metadata(&path));
            if metadata.is_dir() {
                assert!(*dirs < 16);
                *dirs += 1;
                visit(root, &path, depth + 1, dirs, bytes_total, output);
            } else {
                assert!(output.len() < 160);
                let relative = ok(path.strip_prefix(root));
                assert!(relative.as_os_str().len() <= 256);
                ok(relative.to_str().ok_or(()));
                let (kind, bytes) = if metadata.file_type().is_symlink() {
                    let target = ok(fs::read_link(&path));
                    let raw = target.as_os_str().as_bytes();
                    assert!(raw.len() <= 4096 && raw.len() <= 4 * MIB - *bytes_total);
                    ("symlink", raw.to_vec())
                } else {
                    assert!(
                        metadata.is_file()
                            && metadata.len() <= 131072
                            && metadata.len() <= (4 * MIB - *bytes_total) as u64
                    );
                    (
                        "regular",
                        bounded_read(&path, 131072.min(4 * MIB - *bytes_total)),
                    )
                };
                assert!(bytes.len() <= 4 * MIB - *bytes_total);
                *bytes_total += bytes.len();
                output.push(FileWitness {
                    path: relative.to_path_buf(),
                    kind,
                    bytes,
                });
            }
        }
    }
    let metadata = ok(fs::symlink_metadata(root));
    assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
    let mut output = vec![];
    visit(root, root, 0, &mut 0, &mut 0, &mut output);
    output.sort_by(|a, b| a.path.cmp(&b.path));
    output
}
pub fn elapsed_ms(start: Instant) -> u128 {
    start.elapsed().as_millis()
}

#[derive(Serialize, Clone)]
struct EmittedFile {
    relative_file: String,
    byte_len: usize,
    sha256: String,
}
struct EvidenceWriter {
    directory: PathBuf,
    files: Vec<EmittedFile>,
    bytes: usize,
}
impl EvidenceWriter {
    fn new(root: &Path, case: &str, leg: u32) -> Self {
        assert!(
            !case.is_empty()
                && case.len() <= 128
                && case.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        );
        let parent = root.join(case);
        match fs::symlink_metadata(&parent) {
            Ok(m) => assert!(m.is_dir() && !m.file_type().is_symlink()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => ok(fs::create_dir(&parent)),
            Err(_) => panic!("evidence case directory denied"),
        }
        let directory = parent.join(format!("store-{leg}"));
        ok(fs::create_dir(&directory));
        Self {
            directory,
            files: vec![],
            bytes: 0,
        }
    }
    fn raw(&mut self, name: &str, bytes: &[u8], cap: usize) -> Value {
        assert!(
            !name.is_empty()
                && name.len() <= 128
                && name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
        );
        assert!(
            bytes.len() <= cap
                && self.files.len() < 426
                && self
                    .bytes
                    .checked_add(bytes.len())
                    .is_some_and(|n| n <= 32 * MIB)
        );
        let row = EmittedFile {
            relative_file: name.into(),
            byte_len: bytes.len(),
            sha256: hex::encode(hash(bytes)),
        };
        assert!(encoded(&row, 192).len() <= 192);
        let mut file = ok(OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(self.directory.join(name)));
        ok(file.write_all(bytes));
        self.bytes += bytes.len();
        self.files.push(row.clone());
        ok(serde_json::to_value(row))
    }
    fn json<T: Serialize>(&mut self, name: &str, value: &T, cap: usize) -> Value {
        let raw = encoded(value, cap);
        self.raw(name, &raw, cap)
    }
}
fn extend_identity(identity: &Value, fields: Value) -> Value {
    let mut result = identity.as_object().unwrap().clone();
    for (name, value) in fields.as_object().unwrap() {
        assert!(result.insert(name.clone(), value.clone()).is_none());
    }
    Value::Object(result)
}
fn fence_ref(epoch: &ArchivedEpoch) -> Value {
    json!({"producer_epoch":epoch.epoch,"relative_file":format!("epoch-{}-fence.json",epoch.epoch),"byte_len":epoch.raw.len(),"sha256":hex::encode(hash(&epoch.raw))})
}
fn finish_evidence(
    recorder: &Arc<Mutex<Recorder>>,
    case: &str,
    leg: u32,
    mut observations: Value,
    files: Vec<FileWitness>,
    filesystem: Value,
    cleanup_results: (&std::io::Result<()>, &std::io::Result<fs::Metadata>),
) {
    let (close, absence) = cleanup_results;
    let mut r = lock(recorder);
    let run = r.run.clone().expect("actual export context");
    let identity = r.identity(case, leg);
    let expected_epochs = if (case == "ui1_1_clock_capacity_and_counter_limits_fail_closed"
        && leg <= 1)
        || (case == "ui1_1_store_commit_reopen_preserves_prior_blob" && leg == 0)
        || (case == "ui1_1_store_lost_reply_reconciles_without_mutation_replay" && leg == 3)
        || (case == "ui1_1_store_recovery_validates_atomic_selection_and_receipt" && leg < 4)
    {
        2
    } else {
        1
    };
    assert_eq!(r.epochs.len(), expected_epochs);
    assert_eq!(r.completion, r.calls.len());
    assert!(r.pending.is_empty() && r.pending_slots.is_empty());
    let negative_fixture = case == "ui1_1_store_commit_reopen_preserves_prior_blob" && leg == 0;
    assert_eq!(r.negative.len(), if negative_fixture { 7 } else { 0 });
    assert_eq!(r.denials.len(), if negative_fixture { 24 } else { 0 });
    let expected_reopen = (case == "ui1_1_clock_capacity_and_counter_limits_fail_closed"
        && matches!(leg, 0 | 1 | 5))
        || (case == "ui1_1_store_commit_reopen_preserves_prior_blob" && leg <= 3)
        || (case == "ui1_1_store_lost_reply_reconciles_without_mutation_replay" && leg == 3)
        || case == "ui1_1_store_recovery_validates_atomic_selection_and_receipt";
    assert_eq!(r.reopen.is_some(), expected_reopen);
    assert_eq!(
        r.states.len(),
        if negative_fixture {
            5
        } else if expected_reopen {
            2
        } else {
            0
        }
    );
    let namespace = r.epochs[0].fence.namespace_id;
    assert!(r.epochs.iter().all(|e| e.fence.namespace_id == namespace));
    for state in &mut r.states {
        assert!(state.body.namespace_id == 0 || state.body.namespace_id == namespace);
        state.body.namespace_id = namespace;
    }
    let state_hashes = r
        .states
        .iter()
        .map(|s| (s.phase, hex::encode(hash(&encoded(&s.body, 122880)))))
        .collect::<BTreeMap<_, _>>();
    let mut states = Vec::new();
    let mut phase_files = 0;
    let mut phase_bytes = 0;
    for state in &r.states {
        let row = json!({"phase":state.phase,"producer_epoch":state.epoch,"state_sha256":state_hashes[state.phase],"state":state.body});
        let raw = encoded(
            &row,
            if matches!(state.phase, "before_reopen" | "after_reopen") {
                122880
            } else {
                40960
            },
        );
        if !matches!(state.phase, "before_reopen" | "after_reopen") {
            phase_files += state.body.files.len();
            phase_bytes += raw.len();
            assert!(phase_files <= 160 && phase_bytes <= 122880);
        }
        assert!(states.len() < 5);
        states.push(row);
    }
    for record in &mut r.negative {
        record["before_state_sha256"] = json!(state_hashes["selectedtext_before_allocation"]);
        record["after_state_sha256"] = record["before_state_sha256"].clone();
    }
    for record in &mut r.denials {
        let phase = record["state_phase"].as_str().unwrap().to_owned();
        record["before_state_sha256"] = json!(state_hashes[phase.as_str()]);
        record["after_state_sha256"] = record["before_state_sha256"].clone();
    }
    let first_ref = fence_ref(&r.epochs[0]);
    if let Some(attempt) = &mut r.reopen {
        attempt["before_fence_ref"] = first_ref.clone();
        attempt["before_files_state_sha256"] = json!(state_hashes["before_reopen"]);
        attempt["after_files_state_sha256"] = json!(state_hashes["after_reopen"]);
    }
    if let Some(map) = observations.as_object_mut() {
        for name in ["fence", "retained_fence"] {
            if map.contains_key(name) {
                map.insert(name.into(), first_ref.clone());
            }
        }
        if case == "ui1_1_clock_capacity_and_counter_limits_fail_closed" && leg == 5 {
            let attempt = r.reopen.as_ref().expect("actual epochMAX attempt");
            assert_eq!(attempt["actual_error_status_or_null"], 5);
            map.insert("epoch_max_fence".into(), first_ref.clone());
            map.insert(
                "epoch_max_reopen_status".into(),
                attempt["actual_error_status_or_null"].clone(),
            );
            map.insert("reserved_successor_epoch".into(), Value::Null);
            map.insert("epoch_max_selected_unchanged".into(), json!(true));
        }
    }
    encoded(&observations, MIB);
    let mut writer = EvidenceWriter::new(&run.store, case, leg);
    let mut copied = Vec::new();
    let mut copied_bytes = 0;
    assert!(files.len() <= 160);
    for (index, file) in files.iter().enumerate() {
        copied_bytes += file.bytes.len();
        assert!(copied_bytes <= 4 * MIB);
        let name = format!("actual-file-{index}.bin");
        writer.raw(&name, &file.bytes, 131072);
        let row = json!({"relative_path":file.path.to_str().unwrap(),"kind":file.kind,"bytes":file.bytes.len(),"sha256":hash(&file.bytes),"evidence_file":name});
        encoded(&row, 512);
        copied.push(row);
    }
    let mut epochs = Vec::new();
    for e in &r.epochs {
        let fr = fence_ref(e);
        writer.raw(fr["relative_file"].as_str().unwrap(), &e.raw, 2 * MIB);
        let object_epochs=e.fence.objects.iter().map(|o|json!({"object_id":o.selected_object_id,"selected_generation":o.selected_generation,"service_epoch":o.service_epoch_at_fence})).collect::<Vec<_>>();
        let mut refs = Vec::new();
        for (stage, evidence) in [("before_quiesce", &e.before), ("after_join", &e.after)] {
            let value = extend_identity(
                &identity,
                json!({"producer_epoch":e.epoch,"snapshot_stage":stage,"namespace_id":e.fence.namespace_id,"profile_hash":e.fence.profile_hash,"fence_id":e.fence.fence_id,"object_service_epochs":object_epochs,"public_evidence":evidence,"fence_ref":fr}),
            );
            refs.push(writer.json(
                &format!("epoch-{}-{}.json", e.epoch, stage.replace('_', "-")),
                &value,
                2 * MIB,
            ));
        }
        epochs.push(json!({"producer_epoch":e.epoch,"before_quiesce":refs[0],"after_join":refs[1],"fence_ref":fr,"namespace_id":e.fence.namespace_id,"profile_hash":e.fence.profile_hash,"fence_id":e.fence.fence_id,"object_service_epochs":object_epochs}));
    }
    let mut leases = Vec::new();
    assert!(r.copies.len() <= 256);
    for (ordinal, copy) in r.copies.iter().enumerate() {
        let name = format!("lease-{ordinal}.bin");
        writer.raw(&name, &copy.bytes, 4160);
        let o = &copy.origin;
        let d = &copy.descriptor;
        assert_eq!(copy.bytes.len(), d.byte_len as usize);
        let row = json!({"ordinal":ordinal,"producer_epoch":o.epoch,"call_ordinal":o.call,"endpoint_class":class_name(&o.peer.class),"actor":o.peer.actor,"selected_object_id":o.peer.object,"kind":match d.kind {SharedObjectKind::SelectedText=>"selected_text",SharedObjectKind::Receipt=>"receipt",_=>panic!("no draft observation authority")},"descriptor_raw_handle":d.handle.pack(),"descriptor_generation":d.object_generation,"descriptor_byte_len":d.byte_len,"actual_request_wire_sha256":o.request_sha,"actual_reply_wire_sha256":o.reply_sha,"relative_file":name,"actual_bytes":copy.bytes.len(),"sha256":hex::encode(hash(&copy.bytes))});
        encoded(&row, 1000);
        leases.push(row);
    }
    let call_rows = r
        .calls
        .iter()
        .map(|c| {
            let row = c.value();
            encoded(&row, 2048);
            row
        })
        .collect::<Vec<_>>();
    let calls = extend_identity(
        &identity,
        json!({"pending_high_water":r.pending_high,"records":call_rows}),
    );
    writer.json("calls.json", &calls, 2 * MIB);
    let leases = extend_identity(&identity, json!({"records":leases}));
    let lease_ref = writer.json("leases.json", &leases, 262144);
    let negative = extend_identity(
        &identity,
        json!({"negative_calls":r.negative,"lease_denials":r.denials,"reopen_attempt":r.reopen,"state_witnesses":states}),
    );
    let negative_ref = writer.json(
        "negative-calls.json",
        &negative,
        if case == "ui1_1_store_commit_reopen_preserves_prior_blob" && leg == 0 {
            524288
        } else {
            262144
        },
    );
    let summary = extend_identity(
        &identity,
        json!({"backend":"real_host_cas","execution":"in_process_store_owner","claim_flags":{"host_cas":true,"integrated_editor_save":false,"target":false,"device_flush":false,"power_loss":false,"containment":false},"epoch_records":epochs,"actual_files":copied,"observations":observations,"lease_manifest_sha256":lease_ref["sha256"],"negative_calls_manifest_sha256":negative_ref["sha256"]}),
    );
    writer.json("actual.json", &summary, 2 * MIB);
    writer
        .files
        .sort_by(|a, b| a.relative_file.cmp(&b.relative_file));
    encoded(&writer.files, 131072);
    encoded(&r.joins, 262144);
    let close_result = match close {
        Ok(()) => json!({"result":"ok"}),
        Err(e) => {
            json!({"result":"error","io_error_kind":format!("{:?}",e.kind()),"raw_os_error_or_null":e.raw_os_error()})
        }
    };
    let absence_result = match absence {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            json!({"probe":"symlink_metadata","after_tempdir_close":true,"result":"not_found","io_error_kind_or_null":"NotFound","raw_os_error_or_null":e.raw_os_error(),"observed_device_or_null":null,"observed_inode_or_null":null})
        }
        Err(e) => {
            json!({"probe":"symlink_metadata","after_tempdir_close":true,"result":"error","io_error_kind_or_null":format!("{:?}",e.kind()),"raw_os_error_or_null":e.raw_os_error(),"observed_device_or_null":null,"observed_inode_or_null":null})
        }
        Ok(m) => {
            json!({"probe":"symlink_metadata","after_tempdir_close":true,"result":"present","io_error_kind_or_null":null,"raw_os_error_or_null":null,"observed_device_or_null":m.dev(),"observed_inode_or_null":m.ino()})
        }
    };
    let cleanup = extend_identity(
        &identity,
        json!({"root_path":r.root.to_str().unwrap(),"root_device":r.device,"root_inode":r.inode,"epoch_terminal_fence_refs":r.epochs.iter().map(fence_ref).collect::<Vec<_>>(),"emitted_files":writer.files,"external_caller_joins":r.joins,"live_store_ownership":0,"fixture_owned_holders_consumed":true,"tempdir_close":close_result,"root_absence":absence_result,"fixture_filesystem":filesystem}),
    );
    // cleanup.json describes all other emitted files, not its own recursive digest.
    let raw = encoded(&cleanup, MIB);
    assert!(
        writer
            .bytes
            .checked_add(raw.len())
            .is_some_and(|n| n <= 32 * MIB)
    );
    let mut file = ok(OpenOptions::new()
        .write(true)
        .create_new(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(writer.directory.join("cleanup.json")));
    ok(file.write_all(&raw));
}
#[cfg(target_os = "linux")]
fn mount_unescape(text: &str) -> PathBuf {
    let mut out = Vec::new();
    let raw = text.as_bytes();
    let mut i = 0;
    assert!(raw.len() <= 4096);
    while i < raw.len() {
        assert!(out.len() < 4096);
        if raw[i] == b'\\' {
            assert!(i + 3 < raw.len());
            let oct = &raw[i + 1..i + 4];
            assert!(oct.iter().all(|b| (b'0'..=b'7').contains(b)));
            out.push((oct[0] - b'0') * 64 + (oct[1] - b'0') * 8 + (oct[2] - b'0'));
            i += 4;
        } else {
            out.push(raw[i]);
            i += 1;
        }
    }
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(std::ffi::OsString::from_vec(out))
}
fn filesystem_json(root: &Path, id: &str) -> Value {
    let directory = ok(OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(root));
    let metadata = ok(directory.metadata());
    assert!(metadata.is_dir());
    let observation = filesystem_observation(root, &directory, &metadata);
    let value = json!({"fixture_id":id,"root_path":root.to_str().unwrap(),"root_device":metadata.dev(),"root_inode":metadata.ino(),"observed_while_root_live":true,"observation_monotonic_ns":monotonic_ns(),"native_test_pid":std::process::id(),"observation":observation});
    encoded(&value, 16384);
    value
}
#[cfg(target_os = "linux")]
fn filesystem_observation(root: &Path, _directory: &fs::File, metadata: &fs::Metadata) -> Value {
    let raw = bounded_read(Path::new("/proc/self/mountinfo"), 4 * MIB);
    let text = ok(std::str::from_utf8(&raw));
    let mut matching = None;
    let mut count = 0;
    for line in text.lines() {
        count += 1;
        assert!(count <= 4096 && line.len() <= 65536);
        let (left, right) = line.split_once(" - ").expect("native mountinfo delimiter");
        let fields = bounded_parts(left.split_whitespace(), 64);
        let suffix = bounded_parts(right.split_whitespace(), 16);
        assert!(fields.len() >= 6 && fields.len() <= 64 && suffix.len() >= 3 && suffix.len() <= 16);
        let mount = mount_unescape(fields[4]);
        if !root.starts_with(&mount) {
            continue;
        }
        if matching
            .as_ref()
            .is_some_and(|(prior, _): &(PathBuf, Value)| {
                prior.as_os_str().len() >= mount.as_os_str().len()
            })
        {
            continue;
        }
        let (major, minor) = fields[2].split_once(':').unwrap();
        let major = ok(major.parse::<u32>());
        let minor = ok(minor.parse::<u32>());
        assert!(suffix[0].len() <= 32);
        let options = bounded_parts(fields[5].split(','), 64);
        assert!(options.len() <= 64);
        let has = |name| options.contains(&name);
        let value = json!({"method":"linux_mountinfo","mount_id":ok(fields[0].parse::<u64>()),"parent_mount_id":ok(fields[1].parse::<u64>()),"device_major":major,"device_minor":minor,"root":mount_unescape(fields[3]).to_str().unwrap(),"mount_point":mount.to_str().unwrap(),"filesystem_type":suffix[0],"mount_flags":{"read_only":has("ro"),"nosuid":has("nosuid"),"nodev":has("nodev"),"noexec":has("noexec")},"matching_line_byte_len":line.len(),"matching_line_sha256":hex::encode(hash(line.as_bytes()))});
        matching = Some((mount, value));
    }
    let observed = matching.expect("actual same-root mountinfo entry").1;
    assert_eq!(
        (
            observed["device_major"].as_u64().unwrap(),
            observed["device_minor"].as_u64().unwrap()
        ),
        (
            libc::major(metadata.dev()) as u64,
            libc::minor(metadata.dev()) as u64
        )
    );
    observed
}
#[cfg(target_os = "macos")]
fn filesystem_observation(root: &Path, directory: &fs::File, _metadata: &fs::Metadata) -> Value {
    use std::os::fd::AsRawFd;
    let mut info = std::mem::MaybeUninit::<libc::statfs>::zeroed();
    assert_eq!(
        unsafe { libc::fstatfs(directory.as_raw_fd(), info.as_mut_ptr()) },
        0
    );
    let info = unsafe { info.assume_init() };
    let name = unsafe { std::ffi::CStr::from_ptr(info.f_fstypename.as_ptr()) }
        .to_str()
        .unwrap();
    let mount = unsafe { std::ffi::CStr::from_ptr(info.f_mntonname.as_ptr()) }
        .to_str()
        .unwrap();
    let flags = info.f_flags as u64;
    let has = |flag: libc::c_int| flags & flag as u64 != 0;
    assert!(name.len() <= 32 && mount.len() <= 4096);
    let mount_path = Path::new(mount);
    assert!(mount_path.is_absolute() && root.starts_with(mount_path));
    assert_eq!(
        std::mem::size_of::<libc::fsid_t>(),
        std::mem::size_of::<[i32; 2]>()
    );
    // Native Darwin fsid_t is the fixed two-int identifier; libc keeps its fields opaque.
    let fsid = unsafe {
        std::ptr::read_unaligned((&info.f_fsid as *const libc::fsid_t).cast::<[i32; 2]>())
    };
    json!({"method":"macos_statfs","filesystem_type":name,"mount_point":mount,"fsid":fsid,"native_mount_flags":flags,"mount_flags":{"read_only":has(libc::MNT_RDONLY),"nosuid":has(libc::MNT_NOSUID),"nodev":has(libc::MNT_NODEV),"noexec":has(libc::MNT_NOEXEC)}})
}
pub struct UiRecord {
    actual: crate::ui_a::Record,
    case: &'static str,
    frames: usize,
    notes: usize,
    run: Option<RunContext>,
    id: String,
}
impl UiRecord {
    pub fn new(case: &'static str) -> Self {
        Self {
            actual: crate::ui_a::Record::new(case),
            case,
            frames: 0,
            notes: 0,
            run: RunContext::capture(),
            id: random_id(),
        }
    }
    pub fn note(&mut self, name: &str, value: &str) {
        assert!(self.notes < 3 && value.len() <= 4096);
        self.notes += 1;
        self.actual.note(name, value);
    }
    pub fn frame(&mut self, frame: desktop_service::editor_dev::ComposedFrame) {
        assert!(self.frames < 16 && frame.bgra.len() == 640 * 568 * 4);
        self.frames += 1;
        self.actual.frame(frame);
    }
    pub fn barrier_note(&mut self, name: &str, b: &desktop_service::editor_dev::BarrierSnapshot) {
        assert!(self.notes < 3);
        self.notes += 1;
        crate::ui_a::barrier_note(&mut self.actual, name, b);
    }
    pub fn finish(
        &mut self,
        controller: &desktop_service::editor_dev::FixtureController,
        index: u32,
    ) {
        if let Some(run) = &self.run {
            let note = json!({"run_nonce":run.nonce,"ui_fixture_id":self.id,"native_test_pid":std::process::id(),"test_process_start_identity":run.birth,"executing_store_test_sha256":hex::encode(hash(include_bytes!("../editor_store.rs"))),"executing_test_binary_sha256":run.binary_sha,"case":self.case,"registry_index":index});
            let raw = encoded(&note, 4096);
            self.actual
                .note("b_run_binding", std::str::from_utf8(&raw).unwrap());
            let directory = run.ui.join(self.case);
            match fs::symlink_metadata(&directory) {
                Ok(m) => assert!(m.is_dir() && !m.file_type().is_symlink()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    ok(fs::create_dir(&directory))
                }
                _ => panic!("UI output directory denied"),
            };
            for name in std::iter::once(format!("registry-{index}.json"))
                .chain((0..self.frames).map(|i| format!("registry-{index}-frame-{i}.bgra")))
            {
                assert!(
                    matches!(fs::symlink_metadata(directory.join(name)),Err(e) if e.kind()==std::io::ErrorKind::NotFound)
                );
            }
        }
        // Fixed source support serialization: <=256 pairs, <=16 frame metadata
        // rows, <=4 notes of4096 bytes and a fixed header fit strictly within1MiB.
        self.actual.finish(controller, index);
    }
}
pub fn write_ui_terminal(case: &str, terminal: &Value) {
    if let Some(run) = RunContext::capture() {
        let raw = encoded(terminal, 4096);
        let mut file = ok(OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(run.ui.join(case).join("frame-terminal-1.json")));
        ok(file.write_all(&raw));
    }
}
