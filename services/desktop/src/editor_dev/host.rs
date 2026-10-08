//! Trusted, default-off host registry. These checks establish no process containment.
use super::{EditorMessage, decode_envelope_wire, encode_envelope_wire};
use crate::dev::Status;
use kernel_api::cap::{Handle, HandleKind};
use kernel_api::generated::desktop_editor_session_v1 as editor;
use kernel_api::ipc::Envelope;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, mpsc};
use std::time::{Duration, Instant};

#[cfg(feature = "editor_adapter_assertions_v0_dev")]
#[path = "owner_private_assertions.rs"]
mod owner_private_assertions;

const FRAME_BYTES: usize = 640 * 480 * 4;
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EndpointClass {
    SelfStatus = 1,
    FocusRead = 2,
    Surface = 3,
    Artifact = 4,
    Chrome = 5,
    InputProducer = 6,
    Compositor = 7,
    RecoveryReceipt = 8,
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ObjectKind {
    Preview = 1,
    Grants = 2,
    SelectedText = 3,
    DraftSource = 4,
    SurfaceBuffer = 5,
    Receipt = 6,
    ComposedFrame = 7,
}
#[repr(u32)]
pub enum PixelFormat {
    Bgra8888 = 1,
}
#[repr(u32)]
pub enum KeyPhase {
    Press = 1,
    Release = 2,
    Reset = 3,
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum InstanceState {
    Starting = 1,
    Running = 2,
    Exited = 3,
    Faulted = 4,
    Revoked = 5,
    Expired = 6,
}
#[derive(Clone, Copy)]
#[repr(u32)]
pub enum ReceiptOutcome {
    Committed = 0,
    DefinitiveNoncommit = 1,
}
#[derive(Clone)]
pub struct PeerContext {
    registry: Arc<Mutex<State>>,
    endpoint: u64,
    session: u64,
    instance: u64,
    class: EndpointClass,
}
#[derive(Clone)]
pub struct Endpoint {
    pub peer: PeerContext,
    pub handle: Handle,
}
#[derive(Clone, Copy)]
pub struct ObjectDescriptor {
    pub handle: Handle,
    pub kind: ObjectKind,
    pub object_generation: u64,
    pub byte_len: u32,
}
pub struct HostConfig {
    pub preview_ttl_ms: u64,
    pub instance_ttl_ms: u64,
    pub font_rows: [[u8; 16]; 96],
}
#[derive(Clone)]
pub struct HostDesktop {
    #[cfg(feature = "editor_native_read_v0_dev")]
    native: Option<Arc<super::native_authority::NativeAuthority>>,
    state: Arc<Mutex<State>>,
    clock: Arc<AtomicU64>,
    counters: Arc<Counters>,
}
pub struct FixtureController {
    host: HostDesktop,
}
pub struct SessionFixture {
    pub session_id: u64,
    pub session_generation: u64,
    pub chrome: Endpoint,
    pub input: Endpoint,
    pub compositor: Endpoint,
    pub device_generation: u64,
}
pub struct EditorBindings {
    pub bootstrap: Envelope,
    pub self_status: Endpoint,
    pub focus_read: Endpoint,
    pub surface: Endpoint,
    pub artifact: Endpoint,
}
pub struct ComposedFrame {
    pub session_id: u64,
    pub instance_id: u64,
    pub focus_epoch: u64,
    pub sequence: u64,
    pub bgra: Vec<u8>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PausePoint {
    BeforeKeyDelivery,
    BeforePresent,
    BeforeCommitPermit,
    AfterCommitPermit,
    BeforeReadReply,
}
pub struct PauseToken {
    registry: Arc<Mutex<State>>,
    barrier: Arc<Barrier>,
}
#[derive(Clone)]
pub struct BarrierSnapshot {
    pub request_id: u64,
    pub operation_id: u64,
    pub entered: bool,
    pub settled: bool,
    pub permit_issued: bool,
}
#[derive(Clone, Copy)]
pub enum ServiceKind {
    Focus,
    Compositor,
    Artifact,
}
pub enum CounterKind {
    Identity,
    FocusEpoch,
    QueueSequence,
    MappingGeneration,
    SelectedRevision,
    ServiceEpoch,
    TimeOrigin,
}
pub struct ResourceLedger {
    pub sessions: u32,
    pub selected_objects: u32,
    pub instances: u32,
    pub endpoints: u32,
    pub shared_objects: u32,
    pub queued_keys: u32,
    pub operations_per_object: Vec<(u64, u32)>,
}
pub struct Evidence {
    pub exchanges: Vec<(Envelope, Envelope)>,
    pub ledger: ResourceLedger,
    pub permit_count: u32,
    pub transition_count: u32,
    pub mutation_dispatch_count: u32,
    pub volatile_backend: bool,
}
pub struct ReadLease {
    host: HostDesktop,
    peer: PeerContext,
    descriptor: ObjectDescriptor,
    frozen_sequence: Option<u64>,
    #[cfg(feature = "editor_native_preview_v0_dev")]
    _preview_retention: Option<Arc<()>>,
}
#[derive(Clone)]
pub struct WriteLease {
    host: HostDesktop,
    peer: PeerContext,
    descriptor: ObjectDescriptor,
    mapping: u64,
}

#[derive(Default)]
struct Counters {
    permits: AtomicU32,
    transitions: AtomicU32,
    dispatches: AtomicU32,
}
struct FrameReservation {
    state: Arc<Mutex<State>>,
    finished: bool,
}
impl Drop for FrameReservation {
    fn drop(&mut self) {
        if !self.finished {
            self.state.lock().unwrap().pending_frames -= 1;
        }
    }
}
struct RequestLife {
    ticket: Arc<()>,
    #[cfg(feature = "editor_native_preview_v0_dev")]
    entered: Instant,
    deadline: Instant,
    closed: std::sync::atomic::AtomicBool,
}
struct Barrier {
    point: PausePoint,
    session: u64,
    state: Mutex<BarrierState>,
    changed: Condvar,
}
struct BarrierState {
    snapshot: BarrierSnapshot,
    released: bool,
}
impl Barrier {
    fn enter(&self, request_id: u64, operation_id: u64, permit: bool) {
        let mut s = self.state.lock().unwrap();
        s.snapshot = BarrierSnapshot {
            request_id,
            operation_id,
            entered: true,
            settled: false,
            permit_issued: permit,
        };
        self.changed.notify_all();
    }
    fn wait_release(&self) {
        let mut s = self.state.lock().unwrap();
        while !s.released {
            s = self.changed.wait(s).unwrap();
        }
    }
    fn settle(&self) {
        let mut s = self.state.lock().unwrap();
        s.snapshot.settled = true;
        self.changed.notify_all();
    }
}
impl PauseToken {
    fn wait(&self, timeout_ms: u64, settled: bool) -> Result<BarrierSnapshot, Status> {
        if !(1..=2000).contains(&timeout_ms) {
            return Err(Status::Invalid);
        }
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let mut s = self.barrier.state.lock().unwrap();
        while if settled {
            !s.snapshot.settled
        } else {
            !s.snapshot.entered
        } {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(Status::Timeout);
            }
            let (next, timed) = self.barrier.changed.wait_timeout(s, remaining).unwrap();
            s = next;
            if timed.timed_out()
                && !(if settled {
                    s.snapshot.settled
                } else {
                    s.snapshot.entered
                })
            {
                return Err(Status::Timeout);
            }
        }
        Ok(s.snapshot.clone())
    }
    pub fn wait_until_entered(&self, timeout_ms: u64) -> Result<BarrierSnapshot, Status> {
        self.wait(timeout_ms, false)
    }
    pub fn wait_until_settled(&self, timeout_ms: u64) -> Result<BarrierSnapshot, Status> {
        self.wait(timeout_ms, true)
    }
}

#[derive(Clone)]
struct EndpointInfo {
    session: u64,
    instance: u64,
    class: EndpointClass,
    resource: u64,
    epoch: u64,
    rights: u32,
}
struct Instance {
    session: u64,
    generation: u64,
    expires: u64,
    state: InstanceState,
    bindings: [u64; 4],
    grants: u64,
    surface: Surface,
    recovery: bool,
}
struct Surface {
    id: u64,
    generation: u64,
    buffers: [u64; 2],
    created: bool,
    destroyed: bool,
    sequence: u64,
    frozen: Option<(u32, u64, Arc<Vec<u8>>)>,
    epoch: u64,
}
#[derive(Clone, Copy)]
struct Key {
    #[cfg(feature = "editor_native_save_v0_dev")]
    entered: Instant,
    sequence: u64,
    usage: u32,
    phase: u32,
    modifiers: u32,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Route {
    Launcher,
    Preview,
    App,
    Recovery,
}
struct GrantShape {
    instance: u64,
    generation: u64,
    plan: u64,
    revision: u64,
    expires: u64,
    endpoints: [u64; 4],
    surface: u64,
    surface_generation: u64,
}
struct Preview {
    plan: u64,
    revision: u64,
    expires: u64,
    policy: u64,
    application: [u8; 32],
    manifest: [u8; 32],
    selected_revision: u64,
    selected_hash: [u8; 32],
    object: u64,
    release_seen: bool,
    selector: Option<(u32, bool)>,
    approval: bool,
    invalid: bool,
}
#[cfg(feature = "editor_native_preview_v0_dev")]
struct PreinstanceFrameSetup {
    pin: Arc<super::native_preview::PinLife>,
    pin_view: super::PinView,
    plan: u64,
    revision: u64,
    object: u64,
    expires: u64,
    policy: u64,
    session_generation: u64,
    application: [u8; 32],
    manifest: [u8; 32],
}
struct Session {
    owner: u64,
    generation: u64,
    application: [u8; 32],
    manifest: [u8; 32],
    policy: u64,
    chrome: u64,
    input: u64,
    compositor: u64,
    device: u64,
    queue_generation: u64,
    next_sequence: u64,
    attached: bool,
    pressed: BTreeSet<u32>,
    suppressed: BTreeSet<u32>,
    modifiers: u32,
    keys: VecDeque<Key>,
    reset_pending: bool,
    focus: u64,
    focused: Option<u64>,
    route: Route,
    preview: Option<Preview>,
    pending_selector: Option<(u64, u32, bool)>,
    last_plan: u64,
    object: Arc<ObjectAdmissionState>,
    receipt_object: u64,
}
#[derive(Clone)]
enum Data {
    Immutable(Arc<Vec<u8>>),
    Mutable(Arc<Mutex<MutableData>>),
}
struct MutableData {
    bytes: Vec<u8>,
    mapping: u64,
    writable: bool,
    frozen: bool,
}
#[derive(Clone)]
struct Object {
    session: u64,
    instance: u64,
    kind: ObjectKind,
    generation: u64,
    base: u64,
    operation: u64,
    data: Data,
}
struct State {
    #[cfg(feature = "editor_native_save_v0_dev")]
    save_mode: bool,
    #[cfg(feature = "editor_native_save_v0_dev")]
    save_pending: BTreeMap<u64, Arc<super::native_preview::PendingStoreInstance>>,
    #[cfg(feature = "editor_native_save_v0_dev")]
    save_chrome: BTreeMap<u64, super::NativeSaveChromeObservation>,

    #[cfg(feature = "editor_native_preview_v0_dev")]
    preview_pins: BTreeMap<u64, Arc<super::native_preview::PinLife>>,
    #[cfg(feature = "editor_native_preview_v0_dev")]
    preview_instances: BTreeMap<u64, Arc<super::native_preview::AttachmentLife>>,
    #[cfg(feature = "editor_native_preview_v0_dev")]
    chrome_records: BTreeMap<u64, super::PreviewChromeRecordObservation>,
    #[cfg(feature = "editor_native_preview_v0_dev")]
    terminal_versions: BTreeMap<u64, u64>,
    #[cfg(feature = "editor_native_read_v0_dev")]
    native: Option<Arc<super::native_authority::NativeAuthority>>,
    config: HostConfig,
    sessions: BTreeMap<u64, Session>,
    instances: BTreeMap<u64, Instance>,
    endpoints: BTreeMap<u64, EndpointInfo>,
    objects: BTreeMap<u64, Object>,
    next_identity: u64,
    focus_counter: u64,
    mapping_counter: u64,
    service_epoch: u64,
    artifact_epoch: u64,
    queue_exhausted: bool,
    pauses: Vec<Arc<Barrier>>,
    exchanges: Vec<(Envelope, Envelope)>,
    trace_exhausted: bool,
    composed_frames: u32,
    pending_frames: u32,
    frame_exhausted: bool,
}
fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn ascii(bytes: &[u8]) -> bool {
    bytes.len() <= 4096
        && bytes
            .iter()
            .all(|b| (32..=126).contains(b) || matches!(b, 9 | 10))
}
fn q64(q: &Envelope, o: usize) -> u64 {
    u64::from_le_bytes(q.payload[o..o + 8].try_into().unwrap())
}
fn q32(q: &Envelope, o: usize) -> u32 {
    u32::from_le_bytes(q.payload[o..o + 4].try_into().unwrap())
}
fn put64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn put32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn text_bytes(object: u64, revision: u64, bytes: &[u8]) -> Vec<u8> {
    let mut out = vec![0; 64 + bytes.len()];
    let len = out.len() as u32;
    put32(&mut out, 0, 1);
    put32(&mut out, 4, len);
    put64(&mut out, 8, object);
    put64(&mut out, 16, revision);
    out[24..56].copy_from_slice(&hash(bytes));
    put32(&mut out, 56, bytes.len() as u32);
    out[64..].copy_from_slice(bytes);
    out
}

impl State {
    fn shared_object_limit(&self) -> usize {
        #[cfg(feature = "editor_native_save_v0_dev")]
        if self.save_mode {
            // Set only with the actual Save Gate installed by new_store_save.
            // Sixteen retained grants and their two real surface buffers need
            // 48 rows, plus the bounded current per-session permission previews.
            return 64;
        }
        32
    }
    fn reserve_identities(&self, count: u64) -> Result<(), Status> {
        if self
            .next_identity
            .checked_add(count)
            .is_none_or(|n| n > u16::MAX as u64)
        {
            return Err(Status::Exhausted);
        }
        Ok(())
    }
    fn identity(&mut self) -> Result<u64, Status> {
        let n = self.next_identity.checked_add(1).ok_or(Status::Exhausted)?;
        if n > u32::MAX as u64 {
            return Err(Status::Exhausted);
        }
        self.next_identity = n;
        Ok(n)
    }
    fn handle(&mut self, kind: HandleKind) -> Result<Handle, Status> {
        let n = self.identity()?;
        if n > u16::MAX as u64 {
            return Err(Status::Exhausted);
        }
        Ok(Handle {
            kind,
            index: n as u32,
            generation: n,
        })
    }
    fn endpoint(
        &mut self,
        session: u64,
        instance: u64,
        class: EndpointClass,
        resource: u64,
        epoch: u64,
        rights: u32,
    ) -> Result<u64, Status> {
        if self.endpoints.len() >= 64 {
            return Err(Status::Exhausted);
        }
        let key = self.handle(HandleKind::Ipc)?.pack();
        self.endpoints.insert(
            key,
            EndpointInfo {
                session,
                instance,
                class,
                resource,
                epoch,
                rights,
            },
        );
        Ok(key)
    }
    fn object(
        &mut self,
        session: u64,
        instance: u64,
        kind: ObjectKind,
        base: u64,
        operation: u64,
        data: Data,
    ) -> Result<u64, Status> {
        if self.objects.len() >= self.shared_object_limit() {
            return Err(Status::Exhausted);
        }
        let h = self.handle(HandleKind::Shmem)?;
        let key = h.pack();
        self.objects.insert(
            key,
            Object {
                session,
                instance,
                kind,
                generation: h.generation,
                base,
                operation,
                data,
            },
        );
        Ok(key)
    }
    fn retire_focus(&mut self, sid: u64) {
        self.endpoints.retain(|_, e| {
            e.session != sid
                || !matches!(
                    e.class,
                    EndpointClass::FocusRead | EndpointClass::InputProducer
                )
        });
        if let Some(session) = self.sessions.get_mut(&sid) {
            session.attached = false;
            session.keys.clear();
            session.pressed.clear();
            session.suppressed.clear();
            session.modifiers = 0;
            session.reset_pending = false;
            session.pending_selector = None;
            session.focused = None;
            session.route = Route::Launcher;
            if let Some(p) = &mut session.preview {
                p.invalid = true;
                p.approval = false;
            }
        }
    }
    fn retire_surface(&mut self, id: u64) {
        let i = self.instances.get_mut(&id).unwrap();
        let surface = i.surface.id;
        let buffers = i.surface.buffers;
        i.surface.created = false;
        i.surface.destroyed = true;
        i.surface.frozen = None;
        self.endpoints
            .retain(|_, e| !(e.class == EndpointClass::Surface && e.resource == surface));
        for key in buffers {
            if let Some(Object {
                data: Data::Mutable(data),
                ..
            }) = self.objects.remove(&key)
            {
                data.lock().unwrap().writable = false;
            }
        }
    }
    fn retire_closed_source(&mut self, sid: u64, operation: u64) {
        let object = self.sessions[&sid].object.clone();
        let binding = {
            let o = object.inner.lock().unwrap();
            o.operations
                .get(&operation)
                .filter(|op| op.closed && !op.permitted)
                .and_then(|op| {
                    Some((
                        op.source?,
                        op.instance,
                        op.generation,
                        op.expected,
                        op.source_len,
                        op.source_hash?,
                    ))
                })
        };
        let Some((key, instance, generation, expected, len, digest)) = binding else {
            return;
        };
        let Some(source) = self.objects.get(&key) else {
            return;
        };
        if source.session != sid
            || source.instance != instance
            || source.kind != ObjectKind::DraftSource
            || source.generation != Handle::unpack(key).generation
            || source.base != expected
            || self
                .instances
                .get(&instance)
                .is_none_or(|i| i.generation != generation)
        {
            return;
        }
        let Data::Mutable(data) = &source.data else {
            return;
        };
        let data = data.lock().unwrap();
        if !data.frozen
            || data.writable
            || data.bytes.len() != len as usize
            || data.bytes.len() < 64
            || hash(&data.bytes[64..]) != digest
        {
            return;
        }
        drop(data);
        // Retire the identity, never unfreeze it. Retained writer aliases cannot
        // become a new submitted source and the old operation remains closed.
        self.objects.remove(&key);
    }
    fn next_focus(&mut self, sid: u64) -> Result<u64, Status> {
        match self.focus_counter.checked_add(1) {
            Some(next) => Ok(next),
            None => {
                self.retire_focus(sid);
                Err(Status::Exhausted)
            }
        }
    }
    fn shift_focus(&mut self, sid: u64, route: Route, focused: Option<u64>) -> Result<(), Status> {
        let next = self.next_focus(sid)?;
        self.focus_counter = next;
        let s = self.sessions.get_mut(&sid).ok_or(Status::Denied)?;
        s.focus = next;
        s.route = route;
        s.focused = focused;
        s.keys.clear();
        s.suppressed = s.pressed.clone();
        Ok(())
    }
    #[cfg(feature = "editor_native_save_v0_dev")]
    fn release_save_pending(&mut self, id: u64) {
        if let Some(pending) = self.save_pending.remove(&id) {
            // The current State pin is released only if it is this exact setup
            // owner. A newer preview in the same session is independent.
            if let Some(sid) = self.instances.get(&id).map(|i| i.session) {
                if self
                    .preview_pins
                    .get(&sid)
                    .is_some_and(|pin| Arc::ptr_eq(pin, &pending.setup._pin))
                {
                    self.preview_pins.remove(&sid);
                }
            }
            // No Gate is held here. Caller/error/setup owners keep their Arcs.
            drop(pending);
        }
    }
    fn retire(&mut self, id: u64, state: InstanceState) {
        #[cfg(feature = "editor_native_preview_v0_dev")]
        self.preview_instances.remove(&id);
        #[cfg(feature = "editor_native_read_v0_dev")]
        if let Some(native) = &self.native {
            native.retire(id);
        }
        #[cfg(feature = "editor_native_save_v0_dev")]
        self.release_save_pending(id);
        if let Some(i) = self.instances.get_mut(&id) {
            if matches!(i.state, InstanceState::Starting | InstanceState::Running) {
                i.state = state;
                i.recovery = true;
                i.surface.frozen = None;
                let sid = i.session;
                self.endpoints.retain(|_, e| e.instance != id);
                for object in self.objects.values().filter(|o| o.instance == id) {
                    if let Data::Mutable(data) = &object.data {
                        data.lock().unwrap().writable = false;
                    }
                }
                self.objects.retain(|_, o| o.instance != id);
                if let Some(s) = self.sessions.get_mut(&sid) {
                    s.object.inner.lock().unwrap().live.remove(&id);
                    s.keys.clear();
                    s.suppressed = s.pressed.clone();
                    s.focused = None;
                    s.route = Route::Recovery;
                    if let Some(p) = &mut s.preview {
                        p.invalid = true;
                        p.approval = false;
                    }
                }
            }
        }
    }
    fn expire(&mut self, now: u64) {
        #[cfg(feature = "editor_native_read_v0_dev")]
        if let Some(native) = &self.native {
            native.advance(now);
        }
        #[cfg(feature = "editor_native_preview_v0_dev")]
        self.settle_preview_retirements();
        let expired: Vec<_> = self
            .instances
            .iter()
            .filter(|(_, i)| {
                matches!(i.state, InstanceState::Starting | InstanceState::Running)
                    && now >= i.expires
            })
            .map(|(id, _)| *id)
            .collect();
        for id in expired {
            self.retire(id, InstanceState::Expired);
        }
        for s in self.sessions.values_mut() {
            if let Some(p) = &mut s.preview {
                if now >= p.expires {
                    p.invalid = true;
                    p.approval = false;
                }
            }
        }
    }
    fn auth(&self, peer: &PeerContext) -> Result<EndpointInfo, Status> {
        let e = self.endpoints.get(&peer.endpoint).ok_or(Status::Stale)?;
        if e.session != peer.session || e.instance != peer.instance || e.class != peer.class {
            return Err(Status::Denied);
        }
        Ok(e.clone())
    }
}

pub struct SelectedObjectId {
    admission: Arc<ObjectAdmissionState>,
}
pub struct ObjectAdmissionState {
    inner: Mutex<Admission>,
}
struct Admission {
    owner: u64,
    session: u64,
    session_generation: u64,
    id: u64,
    generation: u64,
    revision: u64,
    bytes: Arc<Vec<u8>>,
    epoch: u64,
    live: BTreeMap<u64, (u64, u64)>,
    mutation_retired: bool,
    operations: BTreeMap<u64, Operation>,
}
struct Operation {
    instance: u64,
    generation: u64,
    expected: u64,
    source: Option<u64>,
    source_hash: Option<[u8; 32]>,
    source_len: u32,
    request: Option<Arc<()>>,
    permitted: bool,
    closed: bool,
    receipt: Option<ReceiptSnapshot>,
}
pub struct CommitClaim {
    object: Arc<ObjectAdmissionState>,
    clock: Arc<AtomicU64>,
    counters: Arc<Counters>,
    instance: u64,
    generation: u64,
    operation: u64,
    expected: u64,
    source: u64,
    source_len: u32,
    ticket: Arc<()>,
    bytes: Vec<u8>,
    deadline: Instant,
}
pub struct CommitPermit {
    object: Arc<ObjectAdmissionState>,
    clock: Arc<AtomicU64>,
    counters: Arc<Counters>,
    operation: u64,
    bytes: Vec<u8>,
    receipt: ReceiptSnapshot,
}
pub struct ArtifactSnapshot {
    pub revision: u64,
    pub content_hash: [u8; 32],
    pub bytes: Vec<u8>,
}
#[derive(Clone)]
pub struct ReceiptSnapshot {
    pub schema_version: u32,
    pub total_len: u32,
    pub outcome: ReceiptOutcome,
    pub owner_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub original_instance_id: u64,
    pub original_instance_generation: u64,
    pub selected_object_id: u64,
    pub selected_generation: u64,
    pub operation_id: u64,
    pub expected_revision: u64,
    pub result_revision: u64,
    pub backend_service_epoch: u64,
    pub committed_at_ms: u64,
    pub expected_content_hash: [u8; 32],
    pub source_content_hash: [u8; 32],
}
pub trait SelectedArtifactBackend: Send + Sync {
    fn read(&self, object: &SelectedObjectId) -> Result<ArtifactSnapshot, Status>;
    fn admit(&self, claim: CommitClaim) -> Result<CommitPermit, Status>;
    fn finish(&self, permit: CommitPermit) -> Result<ReceiptSnapshot, Status>;
    fn original_receipt(
        &self,
        object: &SelectedObjectId,
        operation_id: u64,
    ) -> Result<ReceiptSnapshot, Status>;
}
struct VolatileBackend;
impl SelectedArtifactBackend for VolatileBackend {
    fn read(&self, object: &SelectedObjectId) -> Result<ArtifactSnapshot, Status> {
        let o = object.admission.inner.lock().unwrap();
        Ok(ArtifactSnapshot {
            revision: o.revision,
            content_hash: hash(&o.bytes),
            bytes: o.bytes.to_vec(),
        })
    }
    fn admit(&self, c: CommitClaim) -> Result<CommitPermit, Status> {
        let mut o = c.object.inner.lock().unwrap();
        if o.mutation_retired {
            return Err(Status::Stale);
        }
        let live = o.live.get(&c.instance).ok_or(Status::Stale)?;
        if live.0 != c.generation || c.clock.load(Ordering::SeqCst) >= live.1 {
            return Err(Status::Stale);
        }
        let op = o.operations.get(&c.operation).ok_or(Status::Denied)?;
        if op.closed || Instant::now() >= c.deadline {
            o.operations.get_mut(&c.operation).unwrap().closed = true;
            return Err(Status::Timeout);
        }
        if op.instance != c.instance
            || op.generation != c.generation
            || op.expected != c.expected
            || op.source != Some(c.source)
            || op.source_len != c.source_len
            || op.source_hash != Some(hash(&c.bytes))
            || op
                .request
                .as_ref()
                .is_none_or(|ticket| !Arc::ptr_eq(ticket, &c.ticket))
        {
            return Err(Status::Denied);
        }
        if o.revision != c.expected {
            return Err(Status::Conflict);
        }
        if o.operations
            .values()
            .any(|p| p.permitted && p.receipt.is_none())
        {
            return Err(Status::NotReady);
        }
        let next = match o.revision.checked_add(1) {
            Some(next) => next,
            None => {
                o.mutation_retired = true;
                return Err(Status::Exhausted);
            }
        };
        let receipt = ReceiptSnapshot {
            schema_version: 1,
            total_len: 176,
            outcome: ReceiptOutcome::Committed,
            owner_id: o.owner,
            session_id: o.session,
            session_generation: o.session_generation,
            original_instance_id: c.instance,
            original_instance_generation: c.generation,
            selected_object_id: o.id,
            selected_generation: o.generation,
            operation_id: c.operation,
            expected_revision: c.expected,
            result_revision: next,
            backend_service_epoch: o.epoch,
            committed_at_ms: 0,
            expected_content_hash: hash(&o.bytes),
            source_content_hash: hash(&c.bytes),
        };
        // This absolute clock check and the transition to permitted linearize
        // under the same mutex as definitive response closure.
        if Instant::now() >= c.deadline {
            o.operations.get_mut(&c.operation).unwrap().closed = true;
            return Err(Status::Timeout);
        }
        let op = o.operations.get_mut(&c.operation).unwrap();
        op.permitted = true;
        op.source = Some(c.source);
        op.source_hash = Some(hash(&c.bytes));
        op.source_len = c.source_len;
        c.counters.permits.fetch_add(1, Ordering::SeqCst);
        drop(o);
        Ok(CommitPermit {
            object: c.object,
            clock: c.clock,
            counters: c.counters,
            operation: c.operation,
            bytes: c.bytes,
            receipt,
        })
    }
    fn finish(&self, mut p: CommitPermit) -> Result<ReceiptSnapshot, Status> {
        let mut o = p.object.inner.lock().unwrap();
        let op = o.operations.get(&p.operation).ok_or(Status::Internal)?;
        if !op.permitted || op.receipt.is_some() {
            return Err(Status::Internal);
        }
        o.bytes = Arc::new(p.bytes);
        o.revision = p.receipt.result_revision;
        p.receipt.committed_at_ms = p.clock.load(Ordering::SeqCst);
        o.operations.get_mut(&p.operation).unwrap().receipt = Some(p.receipt.clone());
        p.counters.transitions.fetch_add(1, Ordering::SeqCst);
        Ok(p.receipt)
    }
    fn original_receipt(
        &self,
        object: &SelectedObjectId,
        id: u64,
    ) -> Result<ReceiptSnapshot, Status> {
        object
            .admission
            .inner
            .lock()
            .unwrap()
            .operations
            .get(&id)
            .and_then(|o| o.receipt.clone())
            .ok_or(Status::Unknown)
    }
}

fn reply(q: &Envelope, class: EndpointClass, status: Status) -> Envelope {
    let (protocol, kind) = if super::codec::layout(q.protocol, q.msg_type).is_some() {
        (
            q.protocol,
            if q.msg_type == 17 {
                8
            } else if q.msg_type.is_multiple_of(2) {
                q.msg_type
            } else {
                q.msg_type + 1
            },
        )
    } else {
        match class {
            EndpointClass::InputProducer => (802, 4),
            EndpointClass::FocusRead => (832, 4),
            EndpointClass::Surface => (833, 4),
            EndpointClass::Artifact | EndpointClass::RecoveryReceipt => (368, 2),
            EndpointClass::Chrome => (352, 2),
            _ => (352, 8),
        }
    };
    let mut r = Envelope::empty(protocol, kind);
    r.payload_len = super::codec::layout(protocol, kind).unwrap().0 as u32;
    let correlation = if q.protocol == 802 && q.msg_type == 3 {
        q64(q, 24)
    } else {
        q64(q, 0)
    };
    put64(&mut r.payload, 0, correlation);
    let offset = match (protocol, kind) {
        (802, 2) => 16,
        (802, 4) => 8,
        (832, 2) => 24,
        (832, 4) => 36,
        (833, 2) => 40,
        (833, 4) => 24,
        (833, 6 | 8) => 16,
        (833, 10) => 8,
        (352, 2 | 14) => 36,
        (352, 6 | 8) => 44,
        (352, 4 | 10 | 12) => 8,
        (352, 16) => 24,
        (368, 2) => 36,
        (368, 4) => 16,
        (368, 6 | 8) => 32,
        (368, 10) => 24,
        _ => 44,
    };
    put32(&mut r.payload, offset, status as u32);
    r
}
impl HostDesktop {
    pub fn new(config: HostConfig) -> Result<(Self, FixtureController), Status> {
        if !(1..=30000).contains(&config.preview_ttl_ms)
            || !(1..=600000).contains(&config.instance_ttl_ms)
        {
            return Err(Status::Invalid);
        }
        let host = Self {
            #[cfg(feature = "editor_native_read_v0_dev")]
            native: None,
            state: Arc::new(Mutex::new(State {
                #[cfg(feature = "editor_native_save_v0_dev")]
                save_mode: false,
                #[cfg(feature = "editor_native_save_v0_dev")]
                save_pending: BTreeMap::new(),
                #[cfg(feature = "editor_native_save_v0_dev")]
                save_chrome: BTreeMap::new(),
                #[cfg(feature = "editor_native_read_v0_dev")]
                native: None,
                #[cfg(feature = "editor_native_preview_v0_dev")]
                preview_pins: BTreeMap::new(),
                #[cfg(feature = "editor_native_preview_v0_dev")]
                preview_instances: BTreeMap::new(),
                #[cfg(feature = "editor_native_preview_v0_dev")]
                chrome_records: BTreeMap::new(),
                #[cfg(feature = "editor_native_preview_v0_dev")]
                terminal_versions: BTreeMap::new(),
                config,
                sessions: BTreeMap::new(),
                instances: BTreeMap::new(),
                endpoints: BTreeMap::new(),
                objects: BTreeMap::new(),
                next_identity: 0,
                focus_counter: 0,
                mapping_counter: 0,
                service_epoch: 1,
                artifact_epoch: 1,
                queue_exhausted: false,
                pauses: Vec::new(),
                exchanges: Vec::new(),
                trace_exhausted: false,
                composed_frames: 0,
                pending_frames: 0,
                frame_exhausted: false,
            })),
            clock: Arc::new(AtomicU64::new(0)),
            counters: Arc::new(Counters::default()),
        };
        Ok((host.clone(), FixtureController { host }))
    }
    pub(super) fn font_rows(&self) -> [[u8; 16]; 96] {
        self.state.lock().unwrap().config.font_rows
    }
    fn endpoint(&self, key: u64) -> Result<Endpoint, Status> {
        let s = self.state.lock().unwrap();
        let e = s.endpoints.get(&key).ok_or(Status::Stale)?;
        Ok(Endpoint {
            peer: PeerContext {
                registry: self.state.clone(),
                endpoint: key,
                session: e.session,
                instance: e.instance,
                class: e.class,
            },
            handle: Handle::unpack(key),
        })
    }
    fn advance(&self, now: u64) -> u64 {
        let old = self.clock.fetch_max(now, Ordering::SeqCst);
        let current = old.max(now);
        self.state.lock().unwrap().expire(current);
        current
    }
    fn authorize(
        &self,
        peer: &PeerContext,
        q: &Envelope,
        now: u64,
    ) -> Result<EndpointInfo, Status> {
        #[cfg(feature = "editor_native_preview_v0_dev")]
        if self.preview_mode() && !Arc::ptr_eq(&self.state, &peer.registry) {
            return Err(Status::Denied);
        }
        self.advance(now);
        let mut s = self.state.lock().unwrap();
        s.expire(self.clock.load(Ordering::SeqCst));
        self.authorize_locked(&s, peer, q)
    }
    fn authorize_locked(
        &self,
        s: &State,
        peer: &PeerContext,
        q: &Envelope,
    ) -> Result<EndpointInfo, Status> {
        if !Arc::ptr_eq(&self.state, &peer.registry) || q.handle.pack() != peer.endpoint {
            return Err(Status::Denied);
        }
        let class = peer.class;
        let permitted = matches!(
            (q.protocol, q.msg_type, class),
            (802, 1 | 3, EndpointClass::InputProducer)
                | (832, 1, EndpointClass::Chrome)
                | (832, 3, EndpointClass::FocusRead)
                | (833, 1 | 3 | 5 | 9, EndpointClass::Surface)
                | (833, 7, EndpointClass::Compositor)
                | (833, 9, EndpointClass::Chrome)
                | (352, 1 | 3 | 5 | 7 | 9 | 11 | 13, EndpointClass::Chrome)
                | (352, 7 | 15, EndpointClass::SelfStatus)
                | (368, 1 | 3 | 5 | 7, EndpointClass::Artifact)
                | (368, 7, EndpointClass::RecoveryReceipt)
        );
        if !permitted {
            #[cfg(feature = "editor_native_save_v0_dev")]
            if s.save_mode
                && matches!(
                    class,
                    EndpointClass::InputProducer | EndpointClass::Compositor
                )
            {
                // A supported wire shape at this authentic endpoint can still
                // name a route this endpoint does not implement. Authenticate
                // its actual session before returning the route-only denial.
                let sid = if q.protocol == 802 && q.msg_type == 3 {
                    q64(q, 0)
                } else if q.protocol == 833 && q.msg_type != 1 {
                    peer.session
                } else {
                    q64(q, 8)
                };
                if sid != peer.session {
                    return Err(Status::Denied);
                }
                s.auth(peer)?;
                let session = s.sessions.get(&sid).ok_or(Status::Denied)?;
                if session.object.inner.lock().unwrap().owner != session.owner {
                    return Err(Status::Denied);
                }
                if !(q.protocol == 833 && q.msg_type != 1)
                    && q64(
                        q,
                        if q.protocol == 802 && q.msg_type == 3 {
                            8
                        } else {
                            16
                        },
                    ) != session.generation
                {
                    return Err(Status::Stale);
                }
                return Err(Status::Unsupported);
            }
            return Err(Status::Denied);
        }
        let sid = if q.protocol == 802 && q.msg_type == 3 {
            q64(q, 0)
        } else if q.protocol == 833 && q.msg_type != 1 {
            peer.session
        } else {
            q64(q, 8)
        };
        if sid != peer.session {
            return Err(Status::Denied);
        }
        let e = s.auth(peer)?;
        let session = s.sessions.get(&sid).ok_or(Status::Denied)?;
        if session.object.inner.lock().unwrap().owner != session.owner {
            return Err(Status::Denied);
        }
        #[cfg(feature = "editor_native_preview_v0_dev")]
        if e.instance != 0 && s.preview_instances.contains_key(&e.instance) {
            let native = s.native.as_ref().ok_or(Status::Denied)?;
            let gate = native.gate.lock().map_err(|_| Status::Internal)?;
            gate.preview
                .as_ref()
                .ok_or(Status::Denied)?
                .admit_instance(e.instance)?;
        }
        if e.instance != 0 {
            let epoch = if e.class == EndpointClass::Artifact {
                s.artifact_epoch
            } else {
                s.service_epoch
            };
            if e.epoch != epoch {
                return Err(Status::Stale);
            }
            if e.class == EndpointClass::Surface
                && e.epoch != s.instances[&e.instance].surface.epoch
            {
                return Err(Status::Stale);
            }
        }
        if !(q.protocol == 833 && q.msg_type != 1) {
            let generation = q64(
                q,
                if q.protocol == 802 && q.msg_type == 3 {
                    8
                } else {
                    16
                },
            );
            if generation != session.generation {
                return Err(Status::Stale);
            }
        }
        if matches!(
            (q.protocol, q.msg_type),
            (352, 7 | 9 | 11 | 13 | 15) | (832, 1 | 3) | (833, 1) | (368, 1)
        ) {
            let instance = q64(q, 24);
            if e.instance != 0 && instance != e.instance {
                return Err(Status::Denied);
            }
            let i = s.instances.get(&instance).ok_or(Status::Denied)?;
            if i.session != sid {
                return Err(Status::Denied);
            }
            if q64(q, 32) != i.generation {
                return Err(Status::Stale);
            }
        }
        if e.instance != 0 {
            let i = s.instances.get(&e.instance).ok_or(Status::Stale)?;
            if !matches!(i.state, InstanceState::Starting | InstanceState::Running) {
                return Err(Status::Stale);
            }
        }
        let right = match (q.protocol, q.msg_type) {
            (352, 1) => 1,
            (352, 3) => 4,
            (352, 5) => 2,
            (352, 7) => {
                if class == EndpointClass::Chrome {
                    8
                } else {
                    1
                }
            }
            (352, 9) => 16,
            (352, 11) => 32,
            (352, 13) => 64,
            (352, 15) => 1,
            (832, 1) => 2,
            (833, 1) => 1,
            (833, 3) => 2,
            (833, 5) => 4,
            (833, 9) => {
                if class == EndpointClass::Chrome {
                    32
                } else {
                    8
                }
            }
            (368, 1) => 1,
            (368, 3 | 5) => 2,
            (368, 7) => 4,
            _ => 1,
        };
        if e.rights & right != right {
            return Err(Status::Denied);
        }
        Ok(e)
    }
    fn effect<'a>(
        &'a self,
        peer: &PeerContext,
        q: &Envelope,
        life: &RequestLife,
    ) -> Result<MutexGuard<'a, State>, Status> {
        let mut s = self.state.lock().unwrap();
        s.expire(self.clock.load(Ordering::SeqCst));
        self.authorize_locked(&s, peer, q)?;
        if life.closed.load(Ordering::SeqCst) || Instant::now() >= life.deadline {
            return Err(Status::Timeout);
        }
        Ok(s)
    }
    pub fn dispatch(&self, peer: &PeerContext, q: &Envelope, now: u64) -> Envelope {
        let entered = Instant::now();
        let life = Arc::new(RequestLife {
            ticket: Arc::new(()),
            #[cfg(feature = "editor_native_preview_v0_dev")]
            entered,
            deadline: entered + Duration::from_millis(1000),
            closed: std::sync::atomic::AtomicBool::new(false),
        });
        let checked = encode_envelope_wire(q).and_then(|b| decode_envelope_wire(&b));
        let mut response = reply(q, peer.class, Status::NotReady);
        let failure = match checked {
            Err(e) => Some(e),
            Ok(_) if q.msg_type.is_multiple_of(2) || q.msg_type == 17 => Some(Status::Unsupported),
            Ok(_) => {
                let admitted = self.authorize(peer, q, now);
                #[cfg(feature = "editor_native_preview_v0_dev")]
                if admitted.is_ok()
                    && self.preview_mode()
                    && q.protocol == 352
                    && matches!(q.msg_type, 1 | 5)
                {
                    Some(Status::Unsupported)
                } else {
                    admitted.err()
                }
                #[cfg(not(feature = "editor_native_preview_v0_dev"))]
                {
                    admitted.err()
                }
            }
        };
        if let Some(error) = failure {
            response = reply(q, peer.class, error);
        }
        let slot = {
            let mut s = self.state.lock().unwrap();
            if s.exchanges.len() >= s.trace_limit() {
                s.trace_exhausted = true;
                return reply(q, peer.class, Status::Exhausted);
            }
            let n = s.exchanges.len();
            s.exchanges.push((*q, response));
            n
        };
        if failure.is_none() {
            let point = match (q.protocol, q.msg_type) {
                (832, 3) => Some(PausePoint::BeforeKeyDelivery),
                (833, 5) => Some(PausePoint::BeforePresent),
                (368, 1) => Some(PausePoint::BeforeReadReply),
                _ => None,
            };
            let barrier = {
                let mut s = self.state.lock().unwrap();
                s.pauses
                    .iter()
                    .position(|b| {
                        b.session == peer.session
                            && (Some(b.point) == point
                                || q.protocol == 368
                                    && q.msg_type == 5
                                    && matches!(
                                        b.point,
                                        PausePoint::BeforeCommitPermit
                                            | PausePoint::AfterCommitPermit
                                    ))
                    })
                    .map(|p| s.pauses.remove(p))
            };
            if let Some(barrier) = barrier {
                let (send, receive) = mpsc::channel();
                let h = self.clone();
                let p = peer.clone();
                let request = *q;
                let b = barrier.clone();
                let worker_life = life.clone();
                std::thread::spawn(move || {
                    let r = h
                        .perform(&p, &request, now, Some(&b), &worker_life)
                        .unwrap_or_else(|e| reply(&request, p.class, e));
                    b.settle();
                    let _ = send.send(r);
                });
                response = match receive
                    .recv_timeout(life.deadline.saturating_duration_since(Instant::now()))
                {
                    Ok(r) => r,
                    Err(_) => {
                        // Serialize response closure with ordinary effects, and commit
                        // admission with the exact same object mutex used to issue permits.
                        let mut s = self.state.lock().unwrap();
                        s.expire(self.clock.load(Ordering::SeqCst));
                        life.closed.store(true, Ordering::SeqCst);
                        let mut outcome =
                            if q.protocol == 368 && q.msg_type == 5 {
                                let generation = s
                                    .instances
                                    .get(&peer.instance)
                                    .filter(|i| i.session == peer.session)
                                    .map(|i| i.generation);
                                let mut o = s.sessions[&peer.session].object.inner.lock().unwrap();
                                match o.operations.get_mut(&q64(q, 32)) {
                                    Some(op)
                                        if op.instance == peer.instance
                                            && Some(op.generation) == generation
                                            && op.expected == q64(q, 24)
                                            && op.source.is_none_or(|source| {
                                                source == q64(q, 40) && op.source_len == q32(q, 48)
                                            }) =>
                                    {
                                        if op.request.as_ref().is_some_and(|ticket| {
                                            !Arc::ptr_eq(ticket, &life.ticket)
                                        }) {
                                            // A delayed duplicate cannot close the original dispatch
                                            // or expose its pending outcome through Artifact/2.
                                            Err(Status::NotReady)
                                        } else if op.permitted {
                                            Ok(true)
                                        } else {
                                            op.request = Some(life.ticket.clone());
                                            op.closed = true;
                                            Ok(false)
                                        }
                                    }
                                    _ => Err(Status::Denied),
                                }
                            } else {
                                Ok(false)
                            };
                        if outcome == Err(Status::NotReady) {
                            if let Err(error) = self.authorize_locked(&s, peer, q) {
                                outcome = Err(error);
                            }
                        }
                        if outcome == Ok(false) && q.protocol == 368 && q.msg_type == 5 {
                            s.retire_closed_source(peer.session, q64(q, 32));
                        }
                        drop(s);
                        let status = match outcome {
                            Ok(true) => Status::Unknown,
                            Ok(false) => Status::Timeout,
                            Err(error) => error,
                        };
                        let mut r = reply(q, peer.class, status);
                        if outcome == Ok(true) {
                            put64(&mut r.payload, 8, q64(q, 32));
                        }
                        r
                    }
                };
            } else {
                response = self
                    .perform(peer, q, now, None, &life)
                    .unwrap_or_else(|e| reply(q, peer.class, e));
            }
        }
        self.state.lock().unwrap().exchanges[slot].1 = response;
        response
    }
    fn perform(
        &self,
        peer: &PeerContext,
        q: &Envelope,
        now: u64,
        barrier: Option<&Arc<Barrier>>,
        life: &RequestLife,
    ) -> Result<Envelope, Status> {
        if q.protocol == 368 && q.msg_type == 5 {
            return self.commit(peer, q, now, barrier, life);
        }
        if let Some(b) = barrier {
            b.enter(q64(q, 0), 0, false);
            b.wait_release();
            if life.closed.load(Ordering::SeqCst) {
                return Err(Status::Timeout);
            }
        }
        self.authorize(peer, q, now)?;
        let mut r = reply(q, peer.class, Status::Ok);
        match (q.protocol, q.msg_type) {
            (802, 1) => {
                let mut s = self.effect(peer, q, life)?;
                let sid = peer.session;
                if q64(q, 24) != s.sessions[&sid].device {
                    return Err(Status::Stale);
                }
                let generation = s.identity()?;
                let session = s.sessions.get_mut(&sid).unwrap();
                session.queue_generation = generation;
                session.next_sequence = 1;
                session.attached = true;
                put64(&mut r.payload, 8, generation);
            }
            (802, 3) => return self.produce(peer, q, now, life),
            (832, 1) => {
                let mut s = self.effect(peer, q, life)?;
                let id = q64(q, 24);
                let i = &s.instances[&id];
                if !matches!(i.state, InstanceState::Starting | InstanceState::Running)
                    || !i.surface.created
                    || i.surface.id != q64(q, 40)
                    || i.surface.generation != q64(q, 48)
                {
                    return Err(Status::Stale);
                }
                s.shift_focus(peer.session, Route::App, Some(id))?;
                put64(&mut r.payload, 8, s.sessions[&peer.session].focus);
                put64(&mut r.payload, 16, s.service_epoch);
            }
            (832, 3) => {
                let mut s = self.effect(peer, q, life)?;
                let session = s.sessions.get_mut(&peer.session).unwrap();
                if session.focused != Some(peer.instance) || session.focus != q64(q, 40) {
                    return Err(Status::Stale);
                }
                let key = session.keys.pop_front().ok_or(Status::NotReady)?;
                if key.phase == 3 {
                    session.reset_pending = false;
                }
                put64(&mut r.payload, 8, key.sequence);
                put64(&mut r.payload, 16, session.focus);
                put32(&mut r.payload, 24, key.usage);
                put32(&mut r.payload, 28, key.phase);
                put32(&mut r.payload, 32, key.modifiers);
            }
            (352, 1 | 13) => return self.prepare(peer, q, now, life),
            (352, 3) => {
                let mut s = self.effect(peer, q, life)?;
                let session = s.sessions.get_mut(&peer.session).unwrap();
                let p = session.preview.as_mut().ok_or(Status::Stale)?;
                if p.plan != q64(q, 24) {
                    return Err(Status::Stale);
                }
                p.invalid = true;
                p.approval = false;
            }
            (352, 5) => return self.confirm(peer, q, now, life),
            (352, 7) => {
                let s = self.effect(peer, q, life)?;
                let i = &s.instances[&q64(q, 24)];
                put64(&mut r.payload, 8, q64(q, 24));
                put64(&mut r.payload, 16, i.generation);
                put64(&mut r.payload, 24, s.sessions[&peer.session].focus);
                put64(&mut r.payload, 32, s.service_epoch);
                put32(&mut r.payload, 40, i.state as u32);
            }
            (352, 15) => {
                let s = self.effect(peer, q, life)?;
                let session = &s.sessions[&peer.session];
                if session.focused != Some(peer.instance) {
                    return Err(Status::NotReady);
                }
                put64(&mut r.payload, 8, session.focus);
                put64(&mut r.payload, 16, s.service_epoch);
            }
            (352, 9 | 11) => self.effect(peer, q, life)?.retire(
                q64(q, 24),
                if q.msg_type == 9 {
                    InstanceState::Exited
                } else {
                    InstanceState::Revoked
                },
            ),
            (833, 1 | 3 | 5 | 7 | 9) => return self.surface(peer, q, life),
            (368, 1) => return self.read_selected(peer, q, life),
            (368, 3) => {
                let mut s = self.effect(peer, q, life)?;
                let object = s.sessions[&peer.session].object.clone();
                let mut o = object.inner.lock().unwrap();
                if o.mutation_retired {
                    return Err(Status::Stale);
                }
                if o.operations
                    .values()
                    .any(|op| op.permitted && op.receipt.is_none())
                {
                    return Err(Status::NotReady);
                }
                if o.revision != q64(q, 24) {
                    return Err(Status::Conflict);
                }
                if o.operations.len() >= 16 {
                    return Err(Status::Exhausted);
                }
                let id = s.identity()?;
                let generation = s.instances[&peer.instance].generation;
                o.operations.insert(
                    id,
                    Operation {
                        instance: peer.instance,
                        generation,
                        expected: q64(q, 24),
                        source: None,
                        source_hash: None,
                        source_len: 0,
                        request: None,
                        permitted: false,
                        closed: false,
                        receipt: None,
                    },
                );
                put64(&mut r.payload, 8, id);
            }
            (368, 7) => return self.save_status(peer, q, life),
            _ => return Err(Status::Unsupported),
        }
        Ok(r)
    }

    fn grant_bytes(s: &State, sid: u64, grant: GrantShape) -> Vec<u8> {
        let GrantShape {
            instance,
            generation,
            plan,
            revision,
            expires,
            endpoints,
            surface,
            surface_generation,
        } = grant;
        let session = &s.sessions[&sid];
        let object = session.object.inner.lock().unwrap();
        let mut out = vec![0; 464];
        for (offset, value) in [(0, 1), (4, 464), (8, 63), (12, 4)] {
            put32(&mut out, offset, value);
        }
        for (n, value) in [
            sid,
            session.generation,
            instance,
            generation,
            object.id,
            object.generation,
            object.revision,
            session.policy,
            plan,
            revision,
            expires,
            s.service_epoch,
        ]
        .into_iter()
        .enumerate()
        {
            put64(&mut out, 16 + n * 8, value);
        }
        out[112..144].copy_from_slice(&session.application);
        out[144..176].copy_from_slice(&session.manifest);
        out[176..208].copy_from_slice(&hash(&object.bytes));
        for (n, value) in [4096, 640, 480, 2560, 1, 64, 1, 0].into_iter().enumerate() {
            put32(&mut out, 208 + n * 4, value);
        }
        for (n, key) in endpoints.into_iter().enumerate() {
            let offset = 240 + n * 56;
            let (resource, resource_generation, epoch, rights) = match n {
                0 | 1 => (instance, generation, s.service_epoch, 1),
                2 => (surface, surface_generation, s.service_epoch, 15),
                _ => (object.id, object.generation, s.artifact_epoch, 7),
            };
            for (k, value) in [key, resource, resource_generation, epoch, expires]
                .into_iter()
                .enumerate()
            {
                put64(&mut out, offset + k * 8, value);
            }
            put32(&mut out, offset + 40, n as u32 + 1);
            put32(&mut out, offset + 44, rights);
            put32(&mut out, offset + 48, 1);
        }
        out
    }
    fn prepare(
        &self,
        peer: &PeerContext,
        q: &Envelope,
        now: u64,
        life: &RequestLife,
    ) -> Result<Envelope, Status> {
        let now = self.clock.load(Ordering::SeqCst).max(now);
        let mut s = self.effect(peer, q, life)?;
        let sid = peer.session;
        let selector = s.sessions.get_mut(&sid).unwrap().pending_selector.take();
        if q.msg_type == 1
            && (q.payload[24..56] != s.sessions[&sid].application || q32(q, 56) != 63)
        {
            return Err(Status::Denied);
        }
        let object = s.sessions[&sid].object.clone();
        if object
            .inner
            .lock()
            .unwrap()
            .operations
            .values()
            .any(|o| o.permitted && o.receipt.is_none())
        {
            return Err(Status::NotReady);
        }
        let expires = now
            .checked_add(s.config.preview_ttl_ms)
            .ok_or(Status::Exhausted)?;
        s.reserve_identities(3)?;
        s.next_focus(sid)?;
        let old = s.sessions[&sid].preview.as_ref().map(|p| p.object);
        if let Some(key) = old {
            s.objects.remove(&key);
        }
        if s.objects.len() >= s.shared_object_limit() {
            return Err(Status::Exhausted);
        }
        let plan = s.identity()?;
        let revision = s.identity()?;
        let bytes = Self::grant_bytes(
            &s,
            sid,
            GrantShape {
                instance: 0,
                generation: 0,
                plan,
                revision,
                expires,
                endpoints: [0; 4],
                surface: 0,
                surface_generation: 0,
            },
        );
        let descriptor = s.object(
            sid,
            0,
            ObjectKind::Preview,
            0,
            0,
            Data::Immutable(Arc::new(bytes)),
        )?;
        let selected = object.inner.lock().unwrap();
        let selected_revision = selected.revision;
        let selected_hash = hash(&selected.bytes);
        drop(selected);
        let session = s.sessions.get_mut(&sid).unwrap();
        session.last_plan = plan;
        session.preview = Some(Preview {
            plan,
            revision,
            expires,
            policy: session.policy,
            application: session.application,
            manifest: session.manifest,
            selected_revision,
            selected_hash,
            object: descriptor,
            release_seen: false,
            selector: selector
                .filter(|(sequence, _, _)| *sequence == q64(q, 0))
                .map(|(_, usage, control)| (usage, control)),
            approval: false,
            invalid: false,
        });
        s.shift_focus(sid, Route::Preview, None)?;
        let mut r = reply(q, peer.class, Status::Ok);
        put64(&mut r.payload, 8, plan);
        put64(&mut r.payload, 16, revision);
        put64(&mut r.payload, 24, descriptor);
        put32(&mut r.payload, 32, 464);
        put32(&mut r.payload, 40, 63);
        Ok(r)
    }
    fn confirm(
        &self,
        peer: &PeerContext,
        q: &Envelope,
        now: u64,
        life: &RequestLife,
    ) -> Result<Envelope, Status> {
        let mut s = self.effect(peer, q, life)?;
        let sid = peer.session;
        let session = &s.sessions[&sid];
        let p = session.preview.as_ref().ok_or(Status::Stale)?;
        let object = session.object.clone();
        let o = object.inner.lock().unwrap();
        if p.plan != q64(q, 24)
            || p.revision != q64(q, 32)
            || p.invalid
            || now.max(self.clock.load(Ordering::SeqCst)) >= p.expires
            || p.policy != session.policy
            || p.application != session.application
            || p.manifest != session.manifest
            || p.selected_revision != o.revision
            || p.selected_hash != hash(&o.bytes)
        {
            return Err(Status::Stale);
        }
        if !p.approval {
            return Err(Status::Denied);
        }
        drop(o);
        if s.instances.len() >= 16
            || s.endpoints.len() + 4 > 64
            || s.objects.len() + 5 > s.shared_object_limit()
        {
            return Err(Status::Exhausted);
        }
        let expires = self
            .clock
            .load(Ordering::SeqCst)
            .checked_add(s.config.instance_ttl_ms)
            .ok_or(Status::Exhausted)?;
        let plan = p.plan;
        let revision = p.revision;
        let old = p.object;
        s.reserve_identities(9)?;
        let id = s.identity()?;
        let generation = s.identity()?;
        let surface = s.identity()?;
        let surface_generation = s.identity()?;
        let epoch = s.service_epoch;
        let selected_id = object.inner.lock().unwrap().id;
        let artifact_epoch = s.artifact_epoch;
        let endpoints = [
            s.endpoint(sid, id, EndpointClass::SelfStatus, id, epoch, 1)?,
            s.endpoint(sid, id, EndpointClass::FocusRead, id, epoch, 1)?,
            s.endpoint(sid, id, EndpointClass::Surface, surface, epoch, 15)?,
            s.endpoint(
                sid,
                id,
                EndpointClass::Artifact,
                selected_id,
                artifact_epoch,
                7,
            )?,
        ];
        let bytes = Self::grant_bytes(
            &s,
            sid,
            GrantShape {
                instance: id,
                generation,
                plan,
                revision,
                expires,
                endpoints,
                surface,
                surface_generation,
            },
        );
        let grants = s.object(
            sid,
            id,
            ObjectKind::Grants,
            0,
            0,
            Data::Immutable(Arc::new(bytes)),
        )?;
        s.instances.insert(
            id,
            Instance {
                session: sid,
                generation,
                expires,
                state: InstanceState::Starting,
                bindings: endpoints,
                grants,
                surface: Surface {
                    id: surface,
                    generation: surface_generation,
                    buffers: [0; 2],
                    created: false,
                    destroyed: false,
                    sequence: 0,
                    frozen: None,
                    epoch,
                },
                recovery: false,
            },
        );
        object
            .inner
            .lock()
            .unwrap()
            .live
            .insert(id, (generation, expires));
        s.objects.remove(&old);
        let session = s.sessions.get_mut(&sid).unwrap();
        session.preview.as_mut().unwrap().invalid = true;
        session.preview.as_mut().unwrap().approval = false;
        session.route = Route::App;
        let mut r = reply(q, peer.class, Status::Ok);
        put64(&mut r.payload, 8, session.generation);
        put64(&mut r.payload, 16, id);
        put64(&mut r.payload, 24, generation);
        put64(&mut r.payload, 32, grants);
        put32(&mut r.payload, 40, 464);
        Ok(r)
    }
    fn produce(
        &self,
        peer: &PeerContext,
        q: &Envelope,
        now: u64,
        life: &RequestLife,
    ) -> Result<Envelope, Status> {
        enum Action {
            None,
            Prepare,
            Restart(u64, u64),
            Confirm(u64, u64),
            Close(u64, u64),
        }
        let mut action = Action::None;
        let mut s = self.effect(peer, q, life)?;
        let sid = peer.session;
        if s.queue_exhausted {
            s.retire_focus(sid);
            return Err(Status::Exhausted);
        }
        let session = s.sessions.get_mut(&sid).unwrap();
        if !session.attached || session.queue_generation != q64(q, 16) {
            return Err(Status::Stale);
        }
        if session.reset_pending {
            return Err(Status::NotReady);
        }
        if q64(q, 24) != session.next_sequence {
            return Err(Status::Invalid);
        }
        let next = match session.next_sequence.checked_add(1) {
            Some(next) => next,
            None => {
                s.retire_focus(sid);
                return Err(Status::Exhausted);
            }
        };
        let (usage, phase, mods) = (q32(q, 32), q32(q, 36), q32(q, 40));
        if mods & !3 != 0 || !(1..=3).contains(&phase) {
            return Err(Status::Invalid);
        }
        if phase == 3 {
            if usage != 0 || mods != 0 {
                return Err(Status::Invalid);
            }
            session.pressed.clear();
            session.suppressed.clear();
            session.modifiers = 0;
            session.keys.clear();
            session.keys.push_back(Key {
                #[cfg(feature = "editor_native_save_v0_dev")]
                entered: life.entered,
                sequence: q64(q, 24),
                usage: 0,
                phase: 3,
                modifiers: 0,
            });
            session.next_sequence = next;
            session.reset_pending = true;
            if let Some(p) = &mut session.preview {
                p.invalid = true;
                p.approval = false;
            }
            #[cfg(feature = "editor_native_preview_v0_dev")]
            if self.preview_mode() {
                s.retire_preview_setup(sid);
            }
            if s.sessions[&sid].route != Route::App {
                let session = s.sessions.get_mut(&sid).unwrap();
                session.pending_selector = None;
                session.reset_pending = false;
                s.shift_focus(sid, Route::Launcher, None)?;
            }
            return Ok(reply(q, peer.class, Status::Ok));
        }
        if !matches!(usage,4..=49|51..=56|74|77|79..=82|224|225) {
            #[cfg(feature = "editor_native_save_v0_dev")]
            if s.save_mode {
                return Err(Status::Invalid);
            }
            return Err(Status::Unsupported);
        }
        if phase == 1 && session.pressed.contains(&usage)
            || phase == 2 && !session.pressed.contains(&usage)
        {
            return Err(Status::Invalid);
        }
        let bit = match usage {
            224 => 1,
            225 => 2,
            _ => 0,
        };
        let expected = if phase == 1 {
            session.modifiers | bit
        } else {
            session.modifiers & !bit
        };
        if mods != expected {
            return Err(Status::Invalid);
        }
        session.next_sequence = next;
        if session.keys.len() >= 64 {
            session.keys.clear();
            session.pressed.clear();
            session.suppressed.clear();
            session.modifiers = 0;
            session.reset_pending = true;
            session.keys.push_back(Key {
                #[cfg(feature = "editor_native_save_v0_dev")]
                entered: life.entered,
                sequence: q64(q, 24),
                usage: 0,
                phase: 3,
                modifiers: 0,
            });
            if let Some(p) = &mut session.preview {
                p.invalid = true;
                p.approval = false;
            }
            #[cfg(feature = "editor_native_preview_v0_dev")]
            if self.preview_mode() {
                s.retire_preview_setup(sid);
            }
            if s.sessions[&sid].route != Route::App {
                let session = s.sessions.get_mut(&sid).unwrap();
                session.pending_selector = None;
                session.reset_pending = false;
                s.shift_focus(sid, Route::Launcher, None)?;
            }
            return Ok(reply(q, peer.class, Status::Ok));
        }
        session.modifiers = expected;
        if phase == 1 {
            session.pressed.insert(usage);
        } else {
            session.pressed.remove(&usage);
        }
        if phase == 2 && session.route == Route::Preview {
            if let Some(p) = &mut session.preview {
                if !p.invalid {
                    if let Some((selector, control)) = p.selector {
                        p.release_seen = !session.pressed.contains(&selector)
                            && (!control || !session.pressed.contains(&224));
                    }
                }
            }
        }
        let suppressed = if phase == 2 {
            session.suppressed.remove(&usage)
        } else {
            session.suppressed.contains(&usage)
        };
        if suppressed {
            return Ok(reply(q, peer.class, Status::Ok));
        }
        let delivery = expected
            & !(if session.suppressed.contains(&224) {
                1
            } else {
                0
            })
            & !(if session.suppressed.contains(&225) {
                2
            } else {
                0
            });
        let route = session.route;
        #[cfg(feature = "editor_native_preview_v0_dev")]
        if self.preview_mode() && route != Route::App && phase == 1 && usage == 40 && mods == 0 {
            self.mint_preview_ticket_locked(&mut s, sid, q, now, route, life)?;
            return Ok(reply(q, peer.class, Status::Ok));
        }
        if phase == 1 && delivery & 1 != 0 && matches!(usage, 41 | 20 | 21) {
            if usage == 41 {
                s.shift_focus(sid, Route::Launcher, None)?;
            } else {
                let id = s.sessions[&sid]
                    .focused
                    .or_else(|| {
                        s.instances
                            .iter()
                            .rev()
                            .find(|(_, i)| i.session == sid)
                            .map(|(id, _)| *id)
                    })
                    .ok_or(Status::NotReady)?;
                let generation = s.instances[&id].generation;
                #[cfg(feature = "editor_native_save_v0_dev")]
                if usage == 21 && s.save_mode {
                    // Native Save restart discards this actual instance and setup.
                    // Fresh approval must come from a later accepted Enter after R
                    // release; the volatile PrepareRestart wire path is not used.
                    s.retire(id, InstanceState::Revoked);
                    s.retire_preview_setup(sid);
                    s.shift_focus(sid, Route::Launcher, None)?;
                    s.sessions.get_mut(&sid).unwrap().pending_selector =
                        Some((q64(q, 24), 21, true));
                } else {
                    action = if usage == 20 {
                        Action::Close(id, generation)
                    } else {
                        Action::Restart(id, generation)
                    };
                }
                #[cfg(not(feature = "editor_native_save_v0_dev"))]
                {
                    action = if usage == 20 {
                        Action::Close(id, generation)
                    } else {
                        Action::Restart(id, generation)
                    };
                }
            }
        } else if route != Route::App {
            if phase == 1 && usage == 40 && mods == 0 {
                if route == Route::Launcher {
                    action = Action::Prepare;
                } else if route == Route::Preview {
                    let session = s.sessions.get_mut(&sid).unwrap();
                    if let Some(p) = &mut session.preview {
                        if !p.invalid && p.release_seen {
                            p.approval = true;
                            action = Action::Confirm(p.plan, p.revision);
                        }
                    }
                }
            }
            if phase == 1 && usage == 41 {
                if let Some(p) = &mut s.sessions.get_mut(&sid).unwrap().preview {
                    p.invalid = true;
                    p.approval = false;
                }
                s.shift_focus(sid, Route::Launcher, None)?;
            }
        } else {
            s.sessions.get_mut(&sid).unwrap().keys.push_back(Key {
                #[cfg(feature = "editor_native_save_v0_dev")]
                entered: life.entered,
                sequence: q64(q, 24),
                usage,
                phase,
                modifiers: delivery,
            });
        }
        if matches!(action, Action::Prepare | Action::Restart(_, _)) {
            s.sessions.get_mut(&sid).unwrap().pending_selector =
                Some((q64(q, 24), usage, mods & 1 != 0));
        }
        let session = &s.sessions[&sid];
        let chrome = session.chrome;
        let generation = session.generation;
        let application = session.application;
        drop(s);
        let endpoint = self.endpoint(chrome)?;
        match action {
            Action::None => {}
            Action::Prepare => {
                let request = editor::PrepareLaunch {
                    request_id: q64(q, 24),
                    session_id: sid,
                    session_generation: generation,
                    application_hash: application,
                    requested_rights: 63,
                    reserved: 0,
                }
                .encode(endpoint.handle)?;
                self.dispatch(&endpoint.peer, &request, now);
            }
            Action::Restart(id, ig) => {
                let request = editor::PrepareRestart {
                    request_id: q64(q, 24),
                    session_id: sid,
                    session_generation: generation,
                    instance_id: id,
                    instance_generation: ig,
                }
                .encode(endpoint.handle)?;
                self.dispatch(&endpoint.peer, &request, now);
            }
            Action::Close(id, ig) => {
                let request = editor::CloseInstance {
                    request_id: q64(q, 24),
                    session_id: sid,
                    session_generation: generation,
                    instance_id: id,
                    instance_generation: ig,
                }
                .encode(endpoint.handle)?;
                self.dispatch(&endpoint.peer, &request, now);
            }
            Action::Confirm(plan, preview_revision) => {
                let request = editor::ConfirmLaunch {
                    request_id: q64(q, 24),
                    session_id: sid,
                    session_generation: generation,
                    plan_id: plan,
                    preview_revision,
                }
                .encode(endpoint.handle)?;
                self.dispatch(&endpoint.peer, &request, now);
            }
        }
        Ok(reply(q, peer.class, Status::Ok))
    }
    fn surface(
        &self,
        peer: &PeerContext,
        q: &Envelope,
        life: &RequestLife,
    ) -> Result<Envelope, Status> {
        let mut s = self.effect(peer, q, life)?;
        let id = if q.msg_type == 1 {
            peer.instance
        } else {
            let surface = q64(q, 8);
            let (id, i) = s
                .instances
                .iter()
                .find(|(_, i)| i.surface.id == surface)
                .ok_or(Status::Stale)?;
            if i.session != peer.session || peer.instance != 0 && peer.instance != *id {
                return Err(Status::Denied);
            }
            if q64(q, 16) != i.surface.generation {
                return Err(Status::Stale);
            }
            *id
        };
        let mut r = reply(q, peer.class, Status::Ok);
        match q.msg_type {
            1 => {
                if q32(q, 40) != 640 || q32(q, 44) != 480 || q32(q, 48) != 1 {
                    return Err(Status::Invalid);
                }
                if s.instances[&id].surface.destroyed {
                    return Err(Status::Stale);
                }
                if s.instances[&id].surface.created {
                    return Err(Status::Exhausted);
                }
                if s.objects.len() + 2 > s.shared_object_limit() {
                    return Err(Status::Exhausted);
                }
                let data = || {
                    Data::Mutable(Arc::new(Mutex::new(MutableData {
                        bytes: vec![255; FRAME_BYTES],
                        mapping: 0,
                        writable: false,
                        frozen: false,
                    })))
                };
                s.reserve_identities(2)?;
                let buffers = [
                    s.object(peer.session, id, ObjectKind::SurfaceBuffer, 0, 0, data())?,
                    s.object(peer.session, id, ObjectKind::SurfaceBuffer, 0, 0, data())?,
                ];
                let i = s.instances.get_mut(&id).unwrap();
                i.surface.buffers = buffers;
                i.surface.created = true;
                i.state = InstanceState::Running;
                put64(&mut r.payload, 8, i.surface.id);
                put64(&mut r.payload, 16, i.surface.generation);
                put64(&mut r.payload, 24, buffers[0]);
                put64(&mut r.payload, 32, buffers[1]);
                put32(&mut r.payload, 44, 2560);
                put32(&mut r.payload, 48, 1);
            }
            3 => {
                let index = q32(q, 24);
                if index > 1 {
                    return Err(Status::Invalid);
                }
                let i = &s.instances[&id];
                if !i.surface.created {
                    return Err(Status::NotReady);
                }
                if i.surface.frozen.is_some() {
                    return Err(Status::NotReady);
                }
                let key = i.surface.buffers[index as usize];
                let object = s.objects[&key].clone();
                let mapping = match s.mapping_counter.checked_add(1) {
                    Some(mapping) => mapping,
                    None => {
                        s.retire_surface(id);
                        return Err(Status::Exhausted);
                    }
                };
                s.mapping_counter = mapping;
                let Data::Mutable(data) = object.data else {
                    return Err(Status::Internal);
                };
                let mut data = data.lock().unwrap();
                if data.writable {
                    return Err(Status::NotReady);
                }
                data.mapping = mapping;
                data.writable = true;
                data.frozen = false;
                put64(&mut r.payload, 8, mapping);
                put64(&mut r.payload, 16, key);
                put32(&mut r.payload, 28, FRAME_BYTES as u32);
            }
            5 => {
                let index = q32(q, 40);
                if index > 1 {
                    return Err(Status::Invalid);
                }
                let i = &s.instances[&id];
                if q64(q, 32) <= i.surface.sequence {
                    return Err(Status::Stale);
                }
                if i.surface.frozen.is_some() {
                    return Err(Status::NotReady);
                }
                let key = i.surface.buffers[index as usize];
                let object = s.objects.get(&key).ok_or(Status::Stale)?.clone();
                let Data::Mutable(data) = object.data else {
                    return Err(Status::Internal);
                };
                #[cfg(feature = "editor_native_save_v0_dev")]
                let native_owner = if s.save_mode {
                    Some(s.native.as_ref().ok_or(Status::Internal)?.clone())
                } else {
                    None
                };
                #[cfg(feature = "editor_native_save_v0_dev")]
                let mut native_gate = match &native_owner {
                    Some(a) => Some(a.gate.lock().map_err(|_| Status::Internal)?),
                    None => None,
                };
                let mut data = data.lock().unwrap();
                if !data.writable || data.mapping != q64(q, 24) {
                    return Err(Status::Stale);
                }
                #[cfg(feature = "editor_native_save_v0_dev")]
                if s.save_mode {
                    if let Some(g) = &mut native_gate {
                        if let Some(save) = &mut g.save {
                            for d in save
                                .documents
                                .values_mut()
                                .filter(|d| d.view.actor.instance_id == id)
                            {
                                d.frame = None;
                                if d.view.phase == super::NativeEditorPhase::Saved {
                                    d.view.phase = super::NativeEditorPhase::Unsaved;
                                }
                            }
                        }
                    }
                }
                data.writable = false;
                data.frozen = true;
                let frozen = Arc::new(std::mem::replace(&mut data.bytes, vec![255; FRAME_BYTES]));
                let i = s.instances.get_mut(&id).unwrap();
                i.surface.sequence = q64(q, 32);
                i.surface.frozen = Some((index, q64(q, 32), frozen));
                put64(&mut r.payload, 8, q64(q, 32));
            }
            7 => {
                s.consume_frozen(id, q64(q, 24))?;
                put64(&mut r.payload, 8, q64(q, 24));
            }
            9 => {
                let i = s.instances.get_mut(&id).unwrap();
                let buffers = i.surface.buffers;
                i.surface.created = false;
                i.surface.destroyed = true;
                i.surface.frozen = None;
                for key in buffers {
                    if let Some(Object {
                        data: Data::Mutable(data),
                        ..
                    }) = s.objects.remove(&key)
                    {
                        data.lock().unwrap().writable = false;
                    }
                }
            }
            _ => return Err(Status::Unsupported),
        }
        Ok(r)
    }

    fn read_selected(
        &self,
        peer: &PeerContext,
        q: &Envelope,
        life: &RequestLife,
    ) -> Result<Envelope, Status> {
        let mut s = self.effect(peer, q, life)?;
        let object = s.sessions[&peer.session].object.clone();
        let o = object.inner.lock().unwrap();
        let revision = o.revision;
        let data = Arc::new(text_bytes(o.id, revision, &o.bytes));
        drop(o);
        let old = s
            .objects
            .iter()
            .find(|(_, o)| o.instance == peer.instance && o.kind == ObjectKind::SelectedText)
            .map(|(key, o)| (*key, o.base));
        let key = if let Some((key, base)) = old {
            if base == revision {
                key
            } else {
                s.reserve_identities(1)?;
                s.objects.remove(&key);
                s.object(
                    peer.session,
                    peer.instance,
                    ObjectKind::SelectedText,
                    revision,
                    0,
                    Data::Immutable(data.clone()),
                )?
            }
        } else {
            s.object(
                peer.session,
                peer.instance,
                ObjectKind::SelectedText,
                revision,
                0,
                Data::Immutable(data.clone()),
            )?
        };
        let mut r = reply(q, peer.class, Status::Ok);
        put64(&mut r.payload, 8, revision);
        put64(&mut r.payload, 16, key);
        put64(&mut r.payload, 24, s.objects[&key].generation);
        put32(&mut r.payload, 32, data.len() as u32);
        Ok(r)
    }
    pub fn draft_source(
        &self,
        peer: &PeerContext,
        expected_revision: u64,
        bytes: &[u8],
        now: u64,
    ) -> Result<ObjectDescriptor, Status> {
        if !ascii(bytes) {
            return Err(if bytes.len() > 4096 {
                Status::Exhausted
            } else {
                Status::Invalid
            });
        }
        self.advance(now);
        if !Arc::ptr_eq(&self.state, &peer.registry) || peer.class != EndpointClass::Artifact {
            return Err(Status::Denied);
        }
        let mut s = self.state.lock().unwrap();
        s.expire(self.clock.load(Ordering::SeqCst));
        let e = s.auth(peer)?;
        if e.rights & 2 == 0 {
            return Err(Status::Denied);
        }
        let o = s.sessions[&peer.session].object.inner.lock().unwrap();
        if o.mutation_retired {
            return Err(Status::Stale);
        }
        if o.revision != expected_revision {
            return Err(Status::Conflict);
        }
        if o.operations
            .values()
            .any(|o| o.permitted && o.receipt.is_none())
        {
            return Err(Status::NotReady);
        }
        let selected = o.id;
        drop(o);
        let old = s
            .objects
            .iter()
            .find(|(_, o)| o.instance == peer.instance && o.kind == ObjectKind::DraftSource)
            .map(|(k, o)| (*k, o.data.clone()));
        s.reserve_identities(1)?;
        if old.is_none() && s.objects.len() >= s.shared_object_limit() {
            return Err(Status::Exhausted);
        }
        if let Some((key, Data::Mutable(data))) = old {
            if data.lock().unwrap().frozen {
                return Err(Status::NotReady);
            }
            data.lock().unwrap().writable = false;
            s.objects.remove(&key);
        }
        let data = Data::Mutable(Arc::new(Mutex::new(MutableData {
            bytes: text_bytes(selected, expected_revision, bytes),
            mapping: 0,
            writable: true,
            frozen: false,
        })));
        let key = s.object(
            peer.session,
            peer.instance,
            ObjectKind::DraftSource,
            expected_revision,
            0,
            data,
        )?;
        let object = &s.objects[&key];
        Ok(ObjectDescriptor {
            handle: Handle::unpack(key),
            kind: ObjectKind::DraftSource,
            object_generation: object.generation,
            byte_len: (64 + bytes.len()) as u32,
        })
    }
    fn commit(
        &self,
        peer: &PeerContext,
        q: &Envelope,
        now: u64,
        barrier: Option<&Arc<Barrier>>,
        life: &RequestLife,
    ) -> Result<Envelope, Status> {
        self.authorize(peer, q, now)?;
        let (object, generation, content, prior) = {
            let s = self.effect(peer, q, life)?;
            let object = s.sessions[&peer.session].object.clone();
            let generation = s.instances[&peer.instance].generation;
            let o = object.inner.lock().unwrap();
            let op = o.operations.get(&q64(q, 32)).ok_or(Status::Denied)?;
            if op.instance != peer.instance
                || op.generation != generation
                || op.expected != q64(q, 24)
            {
                return Err(Status::Denied);
            }
            if op.source.is_some() && (op.source != Some(q64(q, 40)) || op.source_len != q32(q, 48))
            {
                return Err(Status::Denied);
            }
            if op.permitted {
                let prior = Some(op.receipt.clone());
                drop(o);
                (object, generation, None, prior)
            } else {
                if o.mutation_retired {
                    return Err(Status::Stale);
                }
                if op.closed {
                    return Err(Status::Timeout);
                }
                drop(o);
                let source = s.objects.get(&q64(q, 40)).ok_or(Status::Stale)?;
                if source.session != peer.session
                    || source.instance != peer.instance
                    || source.kind != ObjectKind::DraftSource
                {
                    return Err(Status::Denied);
                }
                if source.base != q64(q, 24) {
                    return Err(Status::Invalid);
                }
                let Data::Mutable(data) = &source.data else {
                    return Err(Status::Invalid);
                };
                let mut data = data.lock().unwrap();
                if data.frozen {
                    return Err(Status::NotReady);
                }
                if !data.writable {
                    return Err(Status::Stale);
                }
                let bytes = &data.bytes;
                if bytes.len() != q32(q, 48) as usize
                    || bytes.len() < 64
                    || bytes.len() > 4160
                    || u32::from_le_bytes(bytes[..4].try_into().unwrap()) != 1
                    || u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize != bytes.len()
                    || u64::from_le_bytes(bytes[8..16].try_into().unwrap())
                        != object.inner.lock().unwrap().id
                    || u64::from_le_bytes(bytes[16..24].try_into().unwrap()) != q64(q, 24)
                    || u32::from_le_bytes(bytes[56..60].try_into().unwrap()) as usize
                        != bytes.len() - 64
                    || bytes[60..64] != [0; 4]
                    || bytes[24..56] != hash(&bytes[64..])
                    || !ascii(&bytes[64..])
                {
                    return Err(Status::Invalid);
                }
                let content = bytes[64..].to_vec();
                {
                    let mut o = object.inner.lock().unwrap();
                    let op = o.operations.get_mut(&q64(q, 32)).unwrap();
                    if op.closed {
                        return Err(Status::Timeout);
                    }
                    if op.permitted {
                        return Err(Status::NotReady);
                    }
                    op.request = Some(life.ticket.clone());
                    op.source = Some(q64(q, 40));
                    op.source_len = q32(q, 48);
                    op.source_hash = Some(hash(&content));
                }
                data.frozen = true;
                data.writable = false;
                drop(data);
                (object, generation, Some(content), None)
            }
        };
        if let Some(prior) = prior {
            if let Some(receipt) = prior {
                return self.receipt_reply(peer, q, &receipt, life);
            }
            let _s = self.effect(peer, q, life)?;
            return Err(Status::NotReady);
        }
        let content = content.ok_or(Status::Internal)?;
        let operation = q64(q, 32);
        self.counters.dispatches.fetch_add(1, Ordering::SeqCst);
        if let Some(b) = barrier.filter(|b| b.point == PausePoint::BeforeCommitPermit) {
            b.enter(q64(q, 0), operation, false);
            b.wait_release();
            if life.closed.load(Ordering::SeqCst) {
                return Err(Status::Timeout);
            }
        }
        let permit = match VolatileBackend.admit(CommitClaim {
            object,
            clock: self.clock.clone(),
            counters: self.counters.clone(),
            instance: peer.instance,
            generation,
            operation,
            expected: q64(q, 24),
            source: q64(q, 40),
            source_len: q32(q, 48),
            ticket: life.ticket.clone(),
            bytes: content,
            deadline: life.deadline,
        }) {
            Ok(permit) => permit,
            Err(Status::Timeout) => {
                self.state
                    .lock()
                    .unwrap()
                    .retire_closed_source(peer.session, operation);
                return Err(Status::Timeout);
            }
            Err(error) => return Err(error),
        };
        if let Some(b) = barrier.filter(|b| b.point == PausePoint::AfterCommitPermit) {
            b.enter(q64(q, 0), operation, true);
            b.wait_release();
        }
        let receipt = match VolatileBackend.finish(permit) {
            Ok(receipt) => receipt,
            Err(_) => {
                let mut r = reply(q, peer.class, Status::Unknown);
                put64(&mut r.payload, 8, operation);
                return Ok(r);
            }
        };
        {
            let mut s = self.state.lock().unwrap();
            s.objects.retain(|_, o| {
                !(o.session == peer.session && o.kind == ObjectKind::SelectedText)
                    || o.base == receipt.result_revision
            });
            s.objects.remove(&q64(q, 40));
        }
        if self.authorize(peer, q, now).is_err() || life.closed.load(Ordering::SeqCst) {
            let mut r = reply(q, peer.class, Status::Unknown);
            put64(&mut r.payload, 8, operation);
            return Ok(r);
        }
        // Issued authority has already selected the original transition. A
        // publication failure cannot certify noncommit or clear client pending.
        match self.receipt_reply(peer, q, &receipt, life) {
            Ok(r) => Ok(r),
            Err(_) => {
                let mut r = reply(q, peer.class, Status::Unknown);
                put64(&mut r.payload, 8, operation);
                Ok(r)
            }
        }
    }
    fn receipt_reply(
        &self,
        peer: &PeerContext,
        q: &Envelope,
        receipt: &ReceiptSnapshot,
        life: &RequestLife,
    ) -> Result<Envelope, Status> {
        let mut bytes = vec![0; 176];
        put32(&mut bytes, 0, 1);
        put32(&mut bytes, 4, 176);
        put32(&mut bytes, 8, receipt.outcome as u32);
        for (n, value) in [
            receipt.owner_id,
            receipt.session_id,
            receipt.session_generation,
            receipt.original_instance_id,
            receipt.original_instance_generation,
            receipt.selected_object_id,
            receipt.selected_generation,
            receipt.operation_id,
            receipt.expected_revision,
            receipt.result_revision,
            receipt.backend_service_epoch,
            receipt.committed_at_ms,
        ]
        .into_iter()
        .enumerate()
        {
            put64(&mut bytes, 16 + n * 8, value);
        }
        bytes[112..144].copy_from_slice(&receipt.expected_content_hash);
        bytes[144..176].copy_from_slice(&receipt.source_content_hash);
        let mut s = self.effect(peer, q, life)?;
        let old = s.sessions[&peer.session].receipt_object;
        s.reserve_identities(1)?;
        if old == 0 && s.objects.len() >= s.shared_object_limit() {
            return Err(Status::Exhausted);
        }
        if old != 0 {
            s.objects.remove(&old);
        }
        let key = s.object(
            peer.session,
            receipt.original_instance_id,
            ObjectKind::Receipt,
            0,
            receipt.operation_id,
            Data::Immutable(Arc::new(bytes)),
        )?;
        s.sessions.get_mut(&peer.session).unwrap().receipt_object = key;
        let mut r = reply(q, peer.class, Status::Ok);
        put64(&mut r.payload, 8, receipt.operation_id);
        put64(&mut r.payload, 16, receipt.result_revision);
        put64(&mut r.payload, 24, key);
        put32(&mut r.payload, 36, 176);
        Ok(r)
    }
    fn save_status(
        &self,
        peer: &PeerContext,
        q: &Envelope,
        life: &RequestLife,
    ) -> Result<Envelope, Status> {
        let operation = q64(q, 24);
        let object = {
            let s = self.effect(peer, q, life)?;
            let e = s.auth(peer)?;
            if e.class == EndpointClass::RecoveryReceipt && e.resource != operation {
                return Err(Status::Denied);
            }
            s.sessions[&peer.session].object.clone()
        };
        let o = object.inner.lock().unwrap();
        let op = o.operations.get(&operation).ok_or(Status::Denied)?;
        if peer.class != EndpointClass::RecoveryReceipt && op.instance != peer.instance {
            return Err(Status::Denied);
        }
        let receipt = op.receipt.clone();
        let permitted = op.permitted;
        drop(o);
        if let Some(receipt) = receipt {
            self.receipt_reply(peer, q, &receipt, life)
        } else {
            let _s = self.effect(peer, q, life)?;
            let mut r = reply(
                q,
                peer.class,
                if permitted {
                    Status::Unknown
                } else {
                    Status::NotReady
                },
            );
            if permitted {
                put64(&mut r.payload, 8, operation);
            }
            Ok(r)
        }
    }
    fn lease_object(
        &self,
        peer: &PeerContext,
        d: &ObjectDescriptor,
        now: u64,
        write: bool,
        mapping: u64,
    ) -> Result<Object, Status> {
        if !Arc::ptr_eq(&self.state, &peer.registry) {
            return Err(Status::Denied);
        }
        self.advance(now);
        let allowed = if write {
            matches!(
                (peer.class, d.kind),
                (EndpointClass::Surface, ObjectKind::SurfaceBuffer)
                    | (EndpointClass::Artifact, ObjectKind::DraftSource)
            )
        } else {
            matches!(
                (peer.class, d.kind),
                (EndpointClass::Chrome, ObjectKind::Preview)
                    | (EndpointClass::SelfStatus, ObjectKind::Grants)
                    | (
                        EndpointClass::Artifact,
                        ObjectKind::SelectedText | ObjectKind::Receipt
                    )
                    | (EndpointClass::RecoveryReceipt, ObjectKind::Receipt)
                    | (
                        EndpointClass::Compositor,
                        ObjectKind::SurfaceBuffer | ObjectKind::ComposedFrame
                    )
            )
        };
        if !allowed {
            return Err(Status::Denied);
        }
        let mut s = self.state.lock().unwrap();
        s.expire(self.clock.load(Ordering::SeqCst));
        let e = s.auth(peer)?;
        let raw = d.handle.pack();
        if d.handle.kind != HandleKind::Shmem || Handle::unpack(raw) != d.handle {
            return Err(Status::Invalid);
        }
        let o = s.objects.get(&raw).ok_or(Status::Stale)?;
        if o.session != peer.session
            || peer.instance != 0 && o.instance != peer.instance
            || o.kind != d.kind
        {
            return Err(Status::Denied);
        }
        // Object ownership precedes own lifecycle disclosure: a foreign
        // preview descriptor must not report the caller's missing/expired pin.
        #[cfg(feature = "editor_native_preview_v0_dev")]
        self.admit_preview_lease_locked(&s, peer, d)?;
        if o.generation != d.object_generation {
            return Err(Status::Stale);
        }
        if peer.class == EndpointClass::RecoveryReceipt && o.operation != e.resource {
            return Err(Status::Denied);
        }
        if write
            && o.kind == ObjectKind::DraftSource
            && s.sessions[&peer.session]
                .object
                .inner
                .lock()
                .unwrap()
                .mutation_retired
        {
            return Err(Status::Stale);
        }
        if matches!(o.kind, ObjectKind::SelectedText | ObjectKind::DraftSource)
            && o.base
                != s.sessions[&peer.session]
                    .object
                    .inner
                    .lock()
                    .unwrap()
                    .revision
        {
            return Err(Status::Stale);
        }
        if o.kind == ObjectKind::Preview
            && s.sessions[&peer.session]
                .preview
                .as_ref()
                .is_none_or(|p| p.invalid || p.object != raw)
        {
            return Err(Status::Stale);
        }
        let len = match &o.data {
            Data::Immutable(data) => data.len(),
            Data::Mutable(data) => data.lock().unwrap().bytes.len(),
        };
        if len != d.byte_len as usize {
            return Err(Status::Invalid);
        }
        if write {
            let Data::Mutable(data) = &o.data else {
                return Err(Status::Denied);
            };
            let data = data.lock().unwrap();
            if o.kind == ObjectKind::DraftSource && mapping != 0 {
                return Err(Status::Invalid);
            }
            if o.kind == ObjectKind::SurfaceBuffer && (mapping == 0 || mapping != data.mapping) {
                return Err(Status::Stale);
            }
            if !data.writable || data.frozen {
                return Err(Status::Stale);
            }
        }
        let mut snapshot = o.clone();
        if !write && o.kind == ObjectKind::SurfaceBuffer {
            let i = s.instances.get(&o.instance).ok_or(Status::Stale)?;
            let (index, sequence, data) = i.surface.frozen.as_ref().ok_or(Status::NotReady)?;
            if i.surface.buffers[*index as usize] != raw {
                return Err(Status::Stale);
            }
            snapshot.data = Data::Immutable(data.clone());
            snapshot.operation = *sequence;
        }
        Ok(snapshot)
    }
    pub fn read_lease(
        &self,
        peer: &PeerContext,
        d: &ObjectDescriptor,
        now: u64,
    ) -> Result<ReadLease, Status> {
        let object = self.lease_object(peer, d, now, false, 0)?;
        Ok(ReadLease {
            host: self.clone(),
            peer: peer.clone(),
            descriptor: *d,
            frozen_sequence: (object.kind == ObjectKind::SurfaceBuffer).then_some(object.operation),
            #[cfg(feature = "editor_native_preview_v0_dev")]
            _preview_retention: self.retain_preview_lease(d.kind)?,
        })
    }
    pub fn write_lease(
        &self,
        peer: &PeerContext,
        d: &ObjectDescriptor,
        mapping: u64,
        now: u64,
    ) -> Result<WriteLease, Status> {
        self.lease_object(peer, d, now, true, mapping)?;
        Ok(WriteLease {
            host: self.clone(),
            peer: peer.clone(),
            descriptor: *d,
            mapping,
        })
    }
    pub fn maintenance(&self, now: u64) -> Result<(), Status> {
        self.advance(now);
        Ok(())
    }
    pub fn compose_next(&self, compositor: &Endpoint, now: u64) -> Result<ComposedFrame, Status> {
        #[cfg(feature = "editor_native_save_v0_dev")]
        if self.state.lock().map_err(|_| Status::Internal)?.save_mode {
            return self.compose_native_save(compositor, now);
        }
        #[cfg(feature = "editor_native_preview_v0_dev")]
        if self.preview_mode() {
            return self.compose_store_preview(compositor, now);
        }
        self.advance(now);
        if !Arc::ptr_eq(&self.state, &compositor.peer.registry)
            || compositor.peer.class != EndpointClass::Compositor
            || compositor.handle.pack() != compositor.peer.endpoint
        {
            return Err(Status::Denied);
        }
        let (
            sid,
            id,
            epoch,
            sequence,
            source,
            recovery,
            unknown,
            font,
            surface_id,
            surface_generation,
        ) = {
            let mut s = self.state.lock().unwrap();
            s.auth(&compositor.peer)?;
            let sid = compositor.peer.session;
            if s.sessions[&sid].compositor != compositor.peer.endpoint {
                return Err(Status::Denied);
            }
            if s.frame_limit_hit() {
                s.frame_exhausted = true;
                return Err(Status::Exhausted);
            }
            let active = s
                .instances
                .iter()
                .rev()
                .find(|(_, i)| i.session == sid && i.surface.frozen.is_some())
                .map(|(id, _)| *id);
            let id = active
                .or_else(|| {
                    s.instances
                        .iter()
                        .rev()
                        .find(|(_, i)| i.session == sid && i.recovery)
                        .map(|(id, _)| *id)
                })
                .ok_or(Status::NotReady)?;
            let i = &s.instances[&id];
            let recovery = active.is_none();
            let sequence = i
                .surface
                .frozen
                .as_ref()
                .map(|(_, seq, _)| *seq)
                .unwrap_or(0);
            let source = i.surface.frozen.as_ref().map(|(_, _, data)| data.clone());
            let epoch = if recovery { 0 } else { s.sessions[&sid].focus };
            let surface_id = i.surface.id;
            let surface_generation = i.surface.generation;
            let unknown = s.sessions[&sid]
                .object
                .inner
                .lock()
                .unwrap()
                .operations
                .values()
                .any(|op| op.instance == id && op.permitted && op.receipt.is_none());
            let font = s.config.font_rows;
            s.pending_frames += 1;
            (
                sid,
                id,
                epoch,
                sequence,
                source,
                recovery,
                unknown,
                font,
                surface_id,
                surface_generation,
            )
        };
        let mut reservation = FrameReservation {
            state: self.state.clone(),
            finished: false,
        };
        // Rasterization and the large immutable copy occur outside registry locks.
        let mut pixels = vec![224; 640 * 568 * 4];
        for pixel in pixels.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        pixels[48 * 2560..528 * 2560].fill(255);
        if let Some(source) = source {
            pixels[48 * 2560..528 * 2560].copy_from_slice(&source);
        }
        for y in 8..24 {
            for x in 8..24 {
                let offset = (y * 640 + x) * 4;
                pixels[offset..offset + 4].copy_from_slice(
                    if x == 8 || x == 23 || y == 8 || y == 23 {
                        &[0, 0, 0, 255]
                    } else {
                        &[0, 192, 0, 255]
                    },
                );
            }
        }
        draw_label(&mut pixels, &font, b"VOLATILE / IN-PROCESS", 8, 536);
        if recovery {
            draw_label(&mut pixels, &font, b"DRAFT LOST", 40, 16);
            if unknown {
                draw_label(&mut pixels, &font, b"SAVE UNKNOWN", 200, 16);
            }
        } else {
            let q = kernel_api::generated::desktop_surface_v1::Consume {
                request_id: sequence,
                surface_id,
                surface_generation,
                sequence,
            }
            .encode(compositor.handle)?;
            let r = self.dispatch(&compositor.peer, &q, now);
            let status = q32(&r, 16);
            if status != 0 {
                return Err(if status == 4 {
                    Status::Stale
                } else {
                    Status::NotReady
                });
            }
        }
        {
            let mut s = self.state.lock().unwrap();
            s.auth(&compositor.peer)?;
            if !recovery {
                let i = &s.instances[&id];
                if !matches!(i.state, InstanceState::Starting | InstanceState::Running)
                    || i.surface.destroyed
                    || s.sessions[&sid].focus != epoch
                {
                    return Err(Status::Stale);
                }
            }
            if recovery {
                if !s.instances[&id].recovery {
                    return Err(Status::Stale);
                }
                s.instances.get_mut(&id).unwrap().recovery = false;
            }
            s.pending_frames -= 1;
            s.composed_frames += 1;
            reservation.finished = true;
        }
        Ok(ComposedFrame {
            session_id: sid,
            instance_id: id,
            focus_epoch: epoch,
            sequence,
            bgra: pixels,
        })
    }
}
fn draw_label(pixels: &mut [u8], font: &[[u8; 16]; 96], label: &[u8], x: usize, y: usize) {
    for (n, c) in label.iter().enumerate() {
        for (row, bits) in font[(*c - 32) as usize].iter().enumerate() {
            for col in 0..8 {
                let offset = ((y + row) * 640 + x + n * 8 + col) * 4;
                let color = if bits & (128 >> col) != 0 {
                    [0, 0, 0, 255]
                } else {
                    [224, 224, 224, 255]
                };
                pixels[offset..offset + 4].copy_from_slice(&color);
            }
        }
    }
}
impl ReadLease {
    pub fn copy_into(&self, offset: u32, out: &mut [u8], now: u64) -> Result<(), Status> {
        let object = self
            .host
            .lease_object(&self.peer, &self.descriptor, now, false, 0)?;
        if self
            .frozen_sequence
            .is_some_and(|seq| seq != object.operation)
        {
            return Err(Status::Stale);
        }
        let Data::Immutable(data) = object.data else {
            return Err(Status::Denied);
        };
        let end = (offset as usize)
            .checked_add(out.len())
            .ok_or(Status::Invalid)?;
        if end > data.len() {
            return Err(Status::Invalid);
        }
        out.copy_from_slice(&data[offset as usize..end]);
        Ok(())
    }
}
impl WriteLease {
    pub fn copy_from(&self, offset: u32, input: &[u8], now: u64) -> Result<(), Status> {
        let object =
            self.host
                .lease_object(&self.peer, &self.descriptor, now, true, self.mapping)?;
        let Data::Mutable(data) = object.data else {
            return Err(Status::Denied);
        };
        let mut data = data.lock().unwrap();
        if !data.writable || data.frozen || data.mapping != self.mapping {
            return Err(Status::Stale);
        }
        let end = (offset as usize)
            .checked_add(input.len())
            .ok_or(Status::Invalid)?;
        if end > data.bytes.len() {
            return Err(Status::Invalid);
        }
        data.bytes[offset as usize..end].copy_from_slice(input);
        Ok(())
    }
}

impl FixtureController {
    pub fn register_session(
        &self,
        owner: u64,
        application: [u8; 32],
        manifest: [u8; 32],
        bytes: &[u8],
        now: u64,
    ) -> Result<SessionFixture, Status> {
        if owner == 0 || !ascii(bytes) {
            return Err(Status::Invalid);
        }
        let now = self.host.advance(now);
        let mut s = self.host.state.lock().unwrap();
        if s.sessions.len() >= 2 || s.endpoints.len() + 3 > 64 {
            return Err(Status::Exhausted);
        }
        now.checked_add(s.config.instance_ttl_ms)
            .ok_or(Status::Exhausted)?;
        s.reserve_identities(8)?;
        s.focus_counter.checked_add(1).ok_or(Status::Exhausted)?;
        let id = s.identity()?;
        let generation = s.identity()?;
        let selected = s.identity()?;
        let selected_generation = s.identity()?;
        let device = s.identity()?;
        let epoch = s.service_epoch;
        let chrome = s.endpoint(id, 0, EndpointClass::Chrome, id, epoch, 127)?;
        let input = s.endpoint(id, 0, EndpointClass::InputProducer, id, epoch, 1)?;
        let compositor = s.endpoint(id, 0, EndpointClass::Compositor, id, epoch, 1)?;
        let object = Arc::new(ObjectAdmissionState {
            inner: Mutex::new(Admission {
                owner,
                session: id,
                session_generation: generation,
                id: selected,
                generation: selected_generation,
                revision: 1,
                bytes: Arc::new(bytes.to_vec()),
                epoch: s.artifact_epoch,
                live: BTreeMap::new(),
                mutation_retired: false,
                operations: BTreeMap::new(),
            }),
        });
        s.sessions.insert(
            id,
            Session {
                owner,
                generation,
                application,
                manifest,
                policy: 1,
                chrome,
                input,
                compositor,
                device,
                queue_generation: 0,
                next_sequence: 1,
                attached: false,
                pressed: BTreeSet::new(),
                suppressed: BTreeSet::new(),
                modifiers: 0,
                keys: VecDeque::new(),
                reset_pending: false,
                focus: 0,
                focused: None,
                route: Route::Launcher,
                preview: None,
                pending_selector: None,
                last_plan: 0,
                object,
                receipt_object: 0,
            },
        );
        s.shift_focus(id, Route::Launcher, None)?;
        drop(s);
        Ok(SessionFixture {
            session_id: id,
            session_generation: generation,
            chrome: self.host.endpoint(chrome)?,
            input: self.host.endpoint(input)?,
            compositor: self.host.endpoint(compositor)?,
            device_generation: device,
        })
    }
    pub fn editor_bindings(&self, id: u64) -> Result<EditorBindings, Status> {
        #[cfg(feature = "editor_native_preview_v0_dev")]
        if self.host.preview_mode() {
            return Err(Status::Denied);
        }
        let s = self.host.state.lock().unwrap();
        let i = s.instances.get(&id).ok_or(Status::Stale)?;
        if !matches!(i.state, InstanceState::Starting | InstanceState::Running) {
            return Err(Status::Stale);
        }
        let session = &s.sessions[&i.session];
        let bootstrap = editor::InstanceBootstrap {
            session_id: i.session,
            session_generation: session.generation,
            instance_id: id,
            instance_generation: i.generation,
            grants_shm: i.grants,
            grants_len: 464,
            status: 0,
        }
        .encode(Handle::INVALID)?;
        let keys = i.bindings;
        drop(s);
        Ok(EditorBindings {
            bootstrap,
            self_status: self.host.endpoint(keys[0])?,
            focus_read: self.host.endpoint(keys[1])?,
            surface: self.host.endpoint(keys[2])?,
            artifact: self.host.endpoint(keys[3])?,
        })
    }
    pub fn set_policy_revision(&self, sid: u64, revision: u64) -> Result<(), Status> {
        if revision == 0 {
            return Err(Status::Invalid);
        }
        let mut s = self.host.state.lock().unwrap();
        let session = s.sessions.get_mut(&sid).ok_or(Status::Denied)?;
        session.policy = revision;
        if let Some(p) = &mut session.preview {
            p.invalid = true;
            p.approval = false;
        }
        #[cfg(feature = "editor_native_preview_v0_dev")]
        s.retire_preview_setup(sid);
        Ok(())
    }
    pub fn set_application_identity(
        &self,
        sid: u64,
        application: [u8; 32],
        manifest: [u8; 32],
    ) -> Result<(), Status> {
        let mut s = self.host.state.lock().unwrap();
        let session = s.sessions.get_mut(&sid).ok_or(Status::Denied)?;
        session.application = application;
        session.manifest = manifest;
        if let Some(p) = &mut session.preview {
            p.invalid = true;
            p.approval = false;
        }
        #[cfg(feature = "editor_native_preview_v0_dev")]
        s.retire_preview_setup(sid);
        Ok(())
    }
    pub fn set_selected_identity(
        &self,
        sid: u64,
        bytes: &[u8],
        revision: u64,
        now: u64,
    ) -> Result<(), Status> {
        if revision == 0 || !ascii(bytes) {
            return Err(Status::Invalid);
        }
        self.host.advance(now);
        let mut s = self.host.state.lock().unwrap();
        let session = s.sessions.get_mut(&sid).ok_or(Status::Denied)?;
        let mut o = session.object.inner.lock().unwrap();
        if o.operations
            .values()
            .any(|o| o.permitted && o.receipt.is_none())
        {
            return Err(Status::NotReady);
        }
        o.bytes = Arc::new(bytes.to_vec());
        o.revision = revision;
        if let Some(p) = &mut session.preview {
            p.invalid = true;
            p.approval = false;
        }
        Ok(())
    }
    pub fn detach_keyboard(&self, sid: u64, now: u64) -> Result<(), Status> {
        self.host.advance(now);
        let mut s = self.host.state.lock().unwrap();
        let session = s.sessions.get_mut(&sid).ok_or(Status::Denied)?;
        let key = session.input;
        session.attached = false;
        session.keys.clear();
        session.pressed.clear();
        session.suppressed.clear();
        session.modifiers = 0;
        session.reset_pending = false;
        if let Some(p) = &mut session.preview {
            p.invalid = true;
            p.approval = false;
        }
        s.endpoints.remove(&key);
        #[cfg(feature = "editor_native_preview_v0_dev")]
        s.retire_preview_setup(sid);
        Ok(())
    }
    pub fn reattach_keyboard(&self, sid: u64, now: u64) -> Result<(Endpoint, u64), Status> {
        self.host.advance(now);
        let mut s = self.host.state.lock().unwrap();
        if !s.sessions.contains_key(&sid) {
            return Err(Status::Denied);
        }
        s.reserve_identities(2)?;
        let old = s.sessions[&sid].input;
        s.endpoints.remove(&old);
        let device = s.identity()?;
        let epoch = s.service_epoch;
        let input = s.endpoint(sid, 0, EndpointClass::InputProducer, sid, epoch, 1)?;
        let session = s.sessions.get_mut(&sid).unwrap();
        session.input = input;
        session.device = device;
        session.attached = false;
        session.modifiers = 0;
        session.pressed.clear();
        session.suppressed.clear();
        session.keys.clear();
        session.reset_pending = false;
        drop(s);
        Ok((self.host.endpoint(input)?, device))
    }
    pub fn pause_next(&self, sid: u64, point: PausePoint) -> Result<PauseToken, Status> {
        let mut s = self.host.state.lock().unwrap();
        if !s.sessions.contains_key(&sid) {
            return Err(Status::Denied);
        }
        if s.pauses.len() >= 2 || s.pauses.iter().any(|p| p.session == sid) {
            return Err(Status::Exhausted);
        }
        let barrier = Arc::new(Barrier {
            point,
            session: sid,
            state: Mutex::new(BarrierState {
                snapshot: BarrierSnapshot {
                    request_id: 0,
                    operation_id: 0,
                    entered: false,
                    settled: false,
                    permit_issued: false,
                },
                released: false,
            }),
            changed: Condvar::new(),
        });
        s.pauses.push(barrier.clone());
        Ok(PauseToken {
            registry: self.host.state.clone(),
            barrier,
        })
    }
    pub fn release(&self, token: &PauseToken) -> Result<(), Status> {
        if !Arc::ptr_eq(&token.registry, &self.host.state) {
            return Err(Status::Denied);
        }
        let mut state = token.barrier.state.lock().unwrap();
        if state.released {
            return Err(Status::Stale);
        }
        state.released = true;
        token.barrier.changed.notify_all();
        Ok(())
    }
    pub fn fault_editor(&self, id: u64, now: u64) -> Result<(), Status> {
        self.host.advance(now);
        let mut s = self.host.state.lock().unwrap();
        if !s.instances.contains_key(&id) {
            return Err(Status::Stale);
        }
        s.retire(id, InstanceState::Faulted);
        Ok(())
    }
    pub fn restart_service(&self, service: ServiceKind, now: u64) -> Result<(), Status> {
        self.host.advance(now);
        let mut s = self.host.state.lock().unwrap();
        if matches!(service, ServiceKind::Artifact)
            && s.sessions.values().any(|session| {
                session
                    .object
                    .inner
                    .lock()
                    .unwrap()
                    .operations
                    .values()
                    .any(|o| o.permitted && o.receipt.is_none())
            })
        {
            return Err(Status::NotReady);
        }
        let epoch = match s.service_epoch.checked_add(1) {
            Some(epoch) => epoch,
            None => {
                match service {
                    ServiceKind::Focus => {
                        let ids: Vec<_> = s.sessions.keys().copied().collect();
                        for sid in ids {
                            s.retire_focus(sid);
                        }
                    }
                    ServiceKind::Compositor => {
                        let ids: Vec<_> = s.instances.keys().copied().collect();
                        for id in ids {
                            s.retire_surface(id);
                        }
                    }
                    ServiceKind::Artifact => {
                        #[cfg(feature = "editor_native_read_v0_dev")]
                        if let Some(native) = &s.native {
                            native.retire_all();
                        }
                        for session in s.sessions.values() {
                            session.object.inner.lock().unwrap().mutation_retired = true;
                        }
                        s.endpoints
                            .retain(|_, e| e.class != EndpointClass::Artifact);
                    }
                }
                return Err(Status::Exhausted);
            }
        };
        s.service_epoch = epoch;
        #[cfg(feature = "editor_native_preview_v0_dev")]
        if self.host.preview_mode() {
            if let Some(native) = &s.native {
                #[cfg(feature = "editor_native_save_v0_dev")]
                if s.save_mode && matches!(service, ServiceKind::Focus | ServiceKind::Compositor) {
                    native.retire_save_service();
                } else {
                    native.retire_all();
                }
                #[cfg(not(feature = "editor_native_save_v0_dev"))]
                native.retire_all();
            }
        }
        if matches!(service, ServiceKind::Artifact) {
            s.artifact_epoch = epoch;
            for session in s.sessions.values() {
                session.object.inner.lock().unwrap().epoch = epoch;
            }
        }
        let ids: Vec<_> = s.instances.keys().copied().collect();
        for id in ids {
            s.retire(id, InstanceState::Faulted);
        }
        let input_keys: Vec<_> = s.sessions.values().map(|session| session.input).collect();
        for key in input_keys {
            s.endpoints.remove(&key);
        }
        for session in s.sessions.values_mut() {
            session.attached = false;
            session.keys.clear();
            session.pressed.clear();
            session.suppressed.clear();
            session.modifiers = 0;
            session.reset_pending = false;
            if let Some(p) = &mut session.preview {
                p.invalid = true;
                p.approval = false;
            }
        }
        Ok(())
    }
    pub fn recovery_receipt(&self, sid: u64, operation: u64) -> Result<Endpoint, Status> {
        let mut s = self.host.state.lock().unwrap();
        let session = s.sessions.get(&sid).ok_or(Status::Denied)?;
        if !session
            .object
            .inner
            .lock()
            .unwrap()
            .operations
            .contains_key(&operation)
        {
            return Err(Status::Denied);
        }
        s.reserve_identities(1)?;
        let existing = s
            .endpoints
            .iter()
            .find(|(_, e)| e.session == sid && e.class == EndpointClass::RecoveryReceipt)
            .map(|(key, _)| *key);
        if let Some(key) = existing {
            s.endpoints.remove(&key);
        }
        let epoch = s.artifact_epoch;
        let key = s.endpoint(sid, 0, EndpointClass::RecoveryReceipt, operation, epoch, 4)?;
        drop(s);
        self.host.endpoint(key)
    }
    pub fn evidence(&self) -> Result<Evidence, Status> {
        let s = self.host.state.lock().unwrap();
        if s.trace_exhausted || s.frame_exhausted {
            return Err(Status::Exhausted);
        }
        let ledger = ResourceLedger {
            sessions: s.sessions.len() as u32,
            selected_objects: s.sessions.len() as u32,
            instances: s.instances.len() as u32,
            endpoints: s.endpoints.len() as u32,
            shared_objects: s.objects.len() as u32,
            queued_keys: s
                .sessions
                .values()
                .map(|session| session.keys.len() as u32)
                .sum(),
            operations_per_object: s
                .sessions
                .values()
                .map(|session| {
                    let o = session.object.inner.lock().unwrap();
                    (o.id, o.operations.len() as u32)
                })
                .collect(),
        };
        Ok(Evidence {
            exchanges: s.exchanges.clone(),
            ledger,
            permit_count: self.host.counters.permits.load(Ordering::SeqCst),
            transition_count: self.host.counters.transitions.load(Ordering::SeqCst),
            mutation_dispatch_count: self.host.counters.dispatches.load(Ordering::SeqCst),
            volatile_backend: {
                #[cfg(feature = "editor_native_save_v0_dev")]
                {
                    !s.save_mode
                }
                #[cfg(not(feature = "editor_native_save_v0_dev"))]
                {
                    true
                }
            },
        })
    }
    pub fn seed_exhaustion(&self, counter: CounterKind) -> Result<(), Status> {
        let mut s = self.host.state.lock().unwrap();
        match counter {
            CounterKind::Identity => s.next_identity = u64::MAX,
            CounterKind::FocusEpoch => s.focus_counter = u64::MAX,
            CounterKind::QueueSequence => s.queue_exhausted = true,
            CounterKind::MappingGeneration => s.mapping_counter = u64::MAX,
            CounterKind::SelectedRevision => {
                for session in s.sessions.values() {
                    session.object.inner.lock().unwrap().revision = u64::MAX;
                }
            }
            CounterKind::ServiceEpoch => s.service_epoch = u64::MAX,
            CounterKind::TimeOrigin => {
                self.host.clock.store(u64::MAX, Ordering::SeqCst);
                #[cfg(feature = "editor_native_read_v0_dev")]
                if s.native.is_some() {
                    // This supported fixture clock transition must retire actual
                    // instances and synchronize their Gate before State unlocks.
                    s.expire(u64::MAX);
                }
            }
        }
        Ok(())
    }
}

#[cfg(feature = "editor_native_read_v0_dev")]
impl HostDesktop {
    pub fn new_native_read(
        config: HostConfig,
    ) -> Result<(Self, FixtureController, super::RegistryWitness), Status> {
        let (mut host, _) = Self::new(config)?;
        let native = super::native_authority::NativeAuthority::new();
        let witness = native.witness();
        host.state.lock().map_err(|_| Status::Internal)?.native = Some(native.clone());
        host.native = Some(native);
        let controller = FixtureController { host: host.clone() };
        Ok((host, controller, witness))
    }

    fn native_authority(&self) -> Result<Arc<super::native_authority::NativeAuthority>, Status> {
        self.native.clone().ok_or(Status::Unsupported)
    }

    // Called only with the actual State held. No diagnostic view or caller time
    // can create the originating peer: endpoint/approval/actor checks use State.
    fn native_binding_view(
        &self,
        state: &State,
        peer: &PeerContext,
    ) -> Result<super::ReadBindingView, Status> {
        if !Arc::ptr_eq(&self.state, &peer.registry) || peer.class != EndpointClass::Artifact {
            return Err(Status::Denied);
        }
        let endpoint = state.auth(peer)?;
        let session = state.sessions.get(&peer.session).ok_or(Status::Denied)?;
        let instance = state.instances.get(&peer.instance).ok_or(Status::Stale)?;
        if instance.session != peer.session
            || endpoint.resource == 0
            || endpoint.rights != 7
            || instance.bindings[3] != peer.endpoint
            || instance.grants == 0
        {
            return Err(Status::Denied);
        }
        if endpoint.epoch != state.artifact_epoch
            || !matches!(
                instance.state,
                InstanceState::Starting | InstanceState::Running
            )
            || self.clock.load(Ordering::SeqCst) >= instance.expires
        {
            return Err(Status::Stale);
        }
        let selected = session.object.inner.lock().map_err(|_| Status::Internal)?;
        if selected.owner != session.owner
            || selected.session != peer.session
            || selected.session_generation != session.generation
            || selected.id != endpoint.resource
            || selected.live.get(&peer.instance) != Some(&(instance.generation, instance.expires))
        {
            return Err(Status::Denied);
        }
        Ok(super::ReadBindingView {
            actor: artifact_store_schema::editor_save::EditorActorV0 {
                owner_id: session.owner,
                session_id: peer.session,
                session_generation: session.generation,
                instance_id: peer.instance,
                instance_generation: instance.generation,
            },
            object_id: selected.id,
            object_generation: selected.generation,
            selected_revision: selected.revision,
            expires_at_ms: instance.expires,
            selected_hash: hash(&selected.bytes),
        })
    }

    pub fn approved_native_read(
        &self,
        peer: &PeerContext,
        now: u64,
    ) -> Result<super::ApprovedReadBinding, Status> {
        let mut state = self.state.lock().map_err(|_| Status::Internal)?;
        let current = self.clock.fetch_max(now, Ordering::SeqCst).max(now);
        state.expire(current);
        let native = state.native.as_ref().ok_or(Status::Unsupported)?;
        let view = self.native_binding_view(&state, peer)?;
        // The same Gate is updated by State::retire/expire before State unlocks.
        native.approve(peer.endpoint, view)
    }

    pub fn start_native_read(
        &self,
        peer: &PeerContext,
        request: &Envelope,
        now: u64,
    ) -> Result<super::NativeReadEntry, Status> {
        let entered = Instant::now();
        super::codec::validate_editor_envelope(request)?;
        if (request.protocol, request.msg_type) != (368, 1) {
            return Err(Status::Unsupported);
        }
        let (native, endpoint) = {
            let mut state = self.state.lock().map_err(|_| Status::Internal)?;
            let current = self.clock.fetch_max(now, Ordering::SeqCst).max(now);
            state.expire(current);
            let native = state.native.as_ref().ok_or(Status::Unsupported)?;
            let view = self.native_binding_view(&state, peer)?;
            // Canonical fields are a claimed tuple, not authority. Mismatched
            // tuples deny without revealing another retained actor's lifetime.
            if request.handle != Handle::unpack(peer.endpoint)
                || q64(request, 8) != view.actor.session_id
                || q64(request, 16) != view.actor.session_generation
                || q64(request, 24) != view.actor.instance_id
                || q64(request, 32) != view.actor.instance_generation
            {
                return Err(Status::Denied);
            }
            (native.clone(), peer.endpoint)
        };
        // Start rechecks the live row after State unlock; it never acquires State
        // from Gate and never holds an enforcing lock across either worker wait.
        native.start(endpoint, *request, entered)
    }

    pub fn native_producers(&self) -> Vec<super::ProducerId> {
        self.native_authority()
            .map(|native| native.producers())
            .unwrap_or_default()
    }
    pub fn join_native_finished(
        &self,
        producer: &super::ProducerId,
    ) -> Result<super::JoinedProducerProof, Status> {
        self.native_authority()?.witness().join_finished(producer)
    }
    pub fn native_producer_counts(&self) -> super::ProducerCounts {
        self.native_authority()
            .map(|native| native.witness().producer_counts())
            .unwrap_or(super::ProducerCounts {
                held: 0,
                io: 0,
                joining: 0,
                joined_total: 0,
                #[cfg(feature = "editor_native_save_v0_dev")]
                active_non_io: 0,
                #[cfg(feature = "editor_native_save_v0_dev")]
                active_io: 0,
            })
    }
}

#[cfg(feature = "editor_native_read_v0_dev")]
impl FixtureController {
    pub fn pause_next_native_read(&self, session: u64) -> Result<super::NativeReadPause, Status> {
        let state = self.host.state.lock().map_err(|_| Status::Internal)?;
        if !state.sessions.contains_key(&session) {
            return Err(Status::Denied);
        }
        state
            .native
            .as_ref()
            .ok_or(Status::Unsupported)?
            .pause(session)
    }
    pub fn release_native_read(&self, pause: &super::NativeReadPause) -> Result<(), Status> {
        self.host.native_authority()?.release(pause)
    }
    pub fn seed_native_counter(&self, counter: super::NativeCounter) -> Result<(), Status> {
        let state = self.host.state.lock().map_err(|_| Status::Internal)?;
        state
            .native
            .as_ref()
            .ok_or(Status::Unsupported)?
            .seed(counter)
    }
}

#[cfg(feature = "editor_native_preview_v0_dev")]
impl PeerContext {
    pub(super) fn matches_preview_owner(
        &self,
        owner: &super::PreviewOwner,
        chrome: u64,
        state_id: usize,
    ) -> bool {
        Arc::as_ptr(&self.registry) as usize == state_id
            && self.endpoint == chrome
            && self.class == EndpointClass::Chrome
            && self.instance == 0
            && self.session == owner.session_id
    }
}
#[cfg(feature = "editor_native_preview_v0_dev")]
impl HostDesktop {
    pub fn new_store_preview_read(
        config: HostConfig,
    ) -> Result<
        (
            Self,
            FixtureController,
            super::RegistryWitness,
            super::StoreSelectionEnrollment,
        ),
        Status,
    > {
        let (host, controller, witness) = Self::new_native_read(config)?;
        let authority = host.native_authority()?;
        authority.enable_preview()?;
        let enrollment = super::StoreSelectionEnrollment { authority };
        Ok((host, controller, witness, enrollment))
    }
    fn preview_mode(&self) -> bool {
        self.native.as_ref().is_some_and(|a| {
            a.gate
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .preview
                .is_some()
        })
    }
    fn preview_chrome_locked(&self, s: &State, peer: &PeerContext) -> Result<(), Status> {
        if !Arc::ptr_eq(&self.state, &peer.registry)
            || peer.class != EndpointClass::Chrome
            || peer.instance != 0
        {
            return Err(Status::Denied);
        }
        let e = s.auth(peer)?;
        let session = s.sessions.get(&peer.session).ok_or(Status::Denied)?;
        if e.class != EndpointClass::Chrome || session.chrome != peer.endpoint {
            return Err(Status::Denied);
        }
        Ok(())
    }
    pub fn preview_counts(&self) -> Result<super::PreviewCounts, Status> {
        let mut state = self.state.lock().map_err(|_| Status::Internal)?;
        state.settle_preview_retirements();
        self.native_authority()?.witness().preview_counts()
    }
    fn mint_preview_ticket_locked(
        &self,
        s: &mut State,
        sid: u64,
        q: &Envelope,
        now: u64,
        route: Route,
        request: &RequestLife,
    ) -> Result<(), Status> {
        use super::native_preview::*;
        let session = &s.sessions[&sid];
        let (kind, plan, revision) = if route == Route::Launcher {
            #[cfg(feature = "editor_native_save_v0_dev")]
            if s.save_mode
                && session
                    .pending_selector
                    .is_some_and(|(_, key, _)| key == 21)
                && session.pressed.contains(&21)
            {
                return Ok(());
            }
            (PreviewUiTicketKind::Selector, 0, 0)
        } else {
            let p = session.preview.as_ref().ok_or(Status::NotReady)?;
            if p.invalid || !p.release_seen {
                return Ok(());
            }
            (PreviewUiTicketKind::Approval, p.plan, p.revision)
        };
        // The accepted Produce's original entry remains authoritative through
        // validation and State acquisition. Ticket issuance cannot restart it.
        let entered = request.entered;
        let deadline = request.deadline;
        let authority = self.native_authority()?;
        let mut gate = lock(&authority.gate)?;
        let p = gate.preview.as_mut().ok_or(Status::Denied)?;
        let now = now.max(self.clock.load(Ordering::SeqCst)).max(p.now);
        if request.closed.load(Ordering::SeqCst) || Instant::now() >= deadline {
            return Err(Status::Timeout);
        }
        let expires = now.checked_add(1000).ok_or(Status::Exhausted)?;
        p.prune();
        if p.tickets.len() >= 16 {
            return Err(Status::Exhausted);
        }
        let id = p.next_ticket.checked_add(1).ok_or(Status::Exhausted)?;
        if kind == PreviewUiTicketKind::Selector {
            p.retire_setup(sid);
        }
        for r in p.tickets.values_mut() {
            if r.view.session_id == sid
                && matches!(r.state, TicketState::Available | TicketState::Taken)
            {
                r.state = TicketState::Closed;
                r.untaken = None;
            }
        }
        let request = if kind == PreviewUiTicketKind::Selector {
            editor::PrepareLaunch {
                request_id: q64(q, 24),
                session_id: sid,
                session_generation: session.generation,
                application_hash: session.application,
                requested_rights: 63,
                reserved: 0,
            }
            .encode(Handle::unpack(session.chrome))?
        } else {
            editor::ConfirmLaunch {
                request_id: q64(q, 24),
                session_id: sid,
                session_generation: session.generation,
                plan_id: plan,
                preview_revision: revision,
            }
            .encode(Handle::unpack(session.chrome))?
        };
        let view = PreviewUiTicketView {
            kind,
            ticket_id: id,
            ticket_generation: id,
            session_id: sid,
            session_generation: session.generation,
            device_id: session.device,
            device_generation: session.queue_generation,
            input_sequence: q64(q, 24),
            focus_epoch: session.focus,
            plan_id: plan,
            preview_revision: revision,
            expires_at_ms: expires,
            entered,
            deadline,
        };
        let life = Arc::new(TicketLife { id });
        p.next_ticket = id;
        p.tickets.insert(
            id,
            TicketRow {
                life: Arc::downgrade(&life),
                untaken: Some(life),
                view,
                request,
                state: TicketState::Available,
            },
        );
        #[cfg(feature = "editor_native_save_v0_dev")]
        if s.save_mode && kind == PreviewUiTicketKind::Selector {
            s.sessions.get_mut(&sid).unwrap().pending_selector = None;
        }
        Ok(())
    }
    pub fn take_preview_action(
        &self,
        chrome: &PeerContext,
    ) -> Result<super::PreviewUiTicket, Status> {
        use super::native_preview::*;
        let s = self.state.lock().map_err(|_| Status::Internal)?;
        self.preview_chrome_locked(&s, chrome)?;
        let authority = self.native_authority()?;
        let mut gate = lock(&authority.gate)?;
        let p = gate.preview.as_mut().ok_or(Status::Denied)?;
        p.prune();
        let id = p
            .tickets
            .iter()
            .rev()
            .find(|(_, r)| r.view.session_id == chrome.session && r.state == TicketState::Available)
            .map(|(&id, _)| id)
            .ok_or(Status::NotReady)?;
        let r = p.tickets.get_mut(&id).ok_or(Status::NotReady)?;
        if p.now >= r.view.expires_at_ms || Instant::now() >= r.view.deadline {
            r.state = TicketState::Closed;
            r.untaken = None;
            return Err(Status::Stale);
        }
        let life = r.untaken.take().ok_or(Status::NotReady)?;
        r.state = TicketState::Taken;
        Ok(PreviewUiTicket {
            authority: authority.clone(),
            life,
            view: r.view.clone(),
            request: r.request,
        })
    }
    fn preview_data(
        s: &State,
        sid: u64,
        pin: &super::PinView,
        plan: u64,
        revision: u64,
        expires: u64,
        active: Option<(u64, u64, u64, u64, [u64; 3], Handle)>,
    ) -> Result<Vec<u8>, Status> {
        use artifact_store_schema::editor_preview::*;
        let session = &s.sessions[&sid];
        let c = &pin.current;
        let (id, generation, surface, sg, handles, store) =
            active.unwrap_or((0, 0, 0, 0, [0; 3], Handle::INVALID));
        let classes = [
            EditorPreviewClassV0::SelfStatus,
            EditorPreviewClassV0::FocusRead,
            EditorPreviewClassV0::Surface,
            EditorPreviewClassV0::StoreRead,
        ];
        let mut records = [EditorPreviewGrantV0 {
            handle: 0,
            resource_id: 0,
            resource_generation: 0,
            service_epoch: s.service_epoch,
            expires_at_ms: expires,
            class: classes[0],
            rights: 1,
            protocol_version: 1,
            reserved: 0,
        }; 4];
        for n in 0..4 {
            records[n].class = classes[n];
            records[n].rights = if n == 2 { 15 } else { 1 };
            records[n].handle = if n < 3 { handles[n] } else { store.pack() };
            records[n].resource_id = match n {
                0 | 1 => id,
                2 => surface,
                _ => c.object_id,
            };
            records[n].resource_generation = match n {
                0 | 1 => generation,
                2 => sg,
                _ => c.object_generation,
            };
            if n == 3 {
                records[n].service_epoch = c.store_epoch;
            }
        }
        let data = EditorPreviewDataV0 {
            phase: if id == 0 {
                EditorPreviewPhaseV0::Preview
            } else {
                EditorPreviewPhaseV0::Active
            },
            schema_version: 2,
            total_len: 464,
            category_mask: 63,
            record_count: 4,
            session_id: sid,
            session_generation: session.generation,
            instance_id: id,
            instance_generation: generation,
            selected_object_id: c.object_id,
            selected_object_generation: c.object_generation,
            selected_revision: c.revision,
            policy_revision: session.policy,
            plan_id: plan,
            preview_revision: revision,
            expires_at_ms: expires,
            desktop_service_epoch: s.service_epoch,
            application_hash: session.application,
            manifest_hash: session.manifest,
            selected_content_hash: c.content_hash,
            max_text_len: 4096,
            surface_width: 640,
            surface_height: 480,
            surface_stride: 2560,
            surface_format: 1,
            input_capacity: 64,
            protocol_version: 1,
            reserved: 0,
            records,
        };
        #[cfg(feature = "editor_native_save_v0_dev")]
        if s.save_mode {
            use artifact_store_schema::editor_native_save::*;
            let records = core::array::from_fn(|n| EditorNativeSaveGrantV0 {
                handle: data.records[n].handle,
                resource_id: data.records[n].resource_id,
                resource_generation: data.records[n].resource_generation,
                service_epoch: data.records[n].service_epoch,
                expires_at_ms: data.records[n].expires_at_ms,
                class: [
                    EditorNativeSaveClassV0::SelfStatus,
                    EditorNativeSaveClassV0::FocusRead,
                    EditorNativeSaveClassV0::Surface,
                    EditorNativeSaveClassV0::StoreArtifact,
                ][n],
                rights: [1, 1, 15, 7][n],
                protocol_version: 1,
                reserved: 0,
            });
            let save = EditorNativeSaveDataV0 {
                phase: if id == 0 {
                    EditorNativeSavePhaseV0::Preview
                } else {
                    EditorNativeSavePhaseV0::Active
                },
                schema_version: 3,
                total_len: 464,
                category_mask: data.category_mask,
                record_count: 4,
                session_id: data.session_id,
                session_generation: data.session_generation,
                instance_id: data.instance_id,
                instance_generation: data.instance_generation,
                selected_object_id: data.selected_object_id,
                selected_object_generation: data.selected_object_generation,
                selected_revision: data.selected_revision,
                policy_revision: data.policy_revision,
                plan_id: data.plan_id,
                preview_revision: data.preview_revision,
                expires_at_ms: data.expires_at_ms,
                desktop_service_epoch: data.desktop_service_epoch,
                application_hash: data.application_hash,
                manifest_hash: data.manifest_hash,
                selected_content_hash: data.selected_content_hash,
                max_text_len: 4096,
                surface_width: 640,
                surface_height: 480,
                surface_stride: 2560,
                surface_format: 1,
                input_capacity: 64,
                protocol_version: 1,
                reserved: 0,
                records,
            };
            return Ok(save.encode_le().map_err(|_| Status::Invalid)?.to_vec());
        }
        Ok(data.encode_le().map_err(|_| Status::Invalid)?.to_vec())
    }
    fn preview_setup_time_locked(
        &self,
        state: &State,
        authority: &Arc<super::native_authority::NativeAuthority>,
        now: u64,
    ) -> Result<u64, Status> {
        #[cfg(feature = "editor_native_save_v0_dev")]
        if state.save_mode {
            let gate = super::native_preview::lock(&authority.gate)?;
            if gate.save.is_none() {
                return Err(Status::Denied);
            }
            let preview = gate.preview.as_ref().ok_or(Status::Denied)?;
            // Drop this Gate before State expiry can retire holders via Gate.
            return Ok(now.max(gate.now_ms).max(preview.now));
        }
        let _ = (state, authority);
        Ok(now)
    }
    pub fn prepare_store_preview(
        &self,
        chrome: &PeerContext,
        ticket: &mut super::PreviewUiTicket,
        pin: &super::StoreSelectionPin,
        now: u64,
    ) -> Result<Envelope, Status> {
        use super::native_preview::*;
        let mut s = self.state.lock().map_err(|_| Status::Internal)?;
        self.preview_chrome_locked(&s, chrome)?;
        let authority = self.native_authority()?;
        if !same(&authority, &ticket.authority)
            || !same(&authority, &pin.authority)
            || ticket.view.session_id != chrome.session
            || ticket.view.kind != PreviewUiTicketKind::Selector
        {
            return Err(Status::Denied);
        }
        let now = self.clock.fetch_max(now, Ordering::SeqCst).max(now);
        let now = self.preview_setup_time_locked(&s, &authority, now)?;
        self.clock.fetch_max(now, Ordering::SeqCst);
        s.expire(now);
        let mut gate = lock(&authority.gate)?;
        #[cfg(feature = "editor_native_save_v0_dev")]
        let now = if s.save_mode {
            if gate.save.is_none() {
                return Err(Status::Denied);
            }
            let now = now
                .max(gate.now_ms)
                .max(gate.preview.as_ref().ok_or(Status::Denied)?.now);
            gate.now_ms = now;
            gate.preview.as_mut().ok_or(Status::Denied)?.expire(now);
            self.clock.fetch_max(now, Ordering::SeqCst);
            now
        } else {
            now
        };
        let p = gate.preview.as_mut().ok_or(Status::Denied)?;
        let tr = p.tickets.get(&ticket.life.id).ok_or(Status::NotReady)?;
        if tr.state == TicketState::Consumed {
            return Err(Status::NotReady);
        }
        if tr.state != TicketState::Taken
            || now >= tr.view.expires_at_ms
            || Instant::now() >= tr.view.deadline
        {
            return Err(Status::Stale);
        }
        let pr = p.pins.get(&pin.life.id).ok_or(Status::Denied)?;
        if pr.view.current.owner.session_id != chrome.session {
            return Err(Status::Denied);
        }
        if pr.retired || now >= pr.view.expires_at_ms || Instant::now() >= pr.view.deadline {
            return Err(Status::Stale);
        }
        let pv = pr.view.clone();
        let sid = chrome.session;
        let session = &s.sessions[&sid];
        if ticket.view.session_generation != session.generation
            || ticket.view.device_generation != session.queue_generation
            || ticket.view.focus_epoch != session.focus
            || ticket.request.payload[24..56] != session.application
        {
            return Err(Status::Stale);
        }
        #[cfg(feature = "editor_native_save_v0_dev")]
        if s.save_mode && s.exchanges.len() >= s.trace_limit() {
            s.trace_exhausted = true;
            return Err(Status::Exhausted);
        }
        s.reserve_identities(3)?;
        s.focus_counter.checked_add(1).ok_or(Status::Exhausted)?;
        if s.objects.len() >= s.shared_object_limit() {
            return Err(Status::Exhausted);
        }
        let expires = now
            .checked_add(s.config.preview_ttl_ms)
            .ok_or(Status::Exhausted)?
            .min(pv.expires_at_ms);
        let plan = s.identity()?;
        let revision = s.identity()?;
        let bytes = Self::preview_data(&s, sid, &pv, plan, revision, expires, None)?;
        if let Some(old) = s.sessions[&sid].preview.as_ref().map(|p| p.object) {
            s.objects.remove(&old);
        }
        let descriptor = s.object(
            sid,
            0,
            ObjectKind::Preview,
            0,
            0,
            Data::Immutable(Arc::new(bytes)),
        )?;
        let session = s.sessions.get_mut(&sid).ok_or(Status::Denied)?;
        session.last_plan = plan;
        session.preview = Some(Preview {
            plan,
            revision,
            expires,
            policy: session.policy,
            application: session.application,
            manifest: session.manifest,
            selected_revision: pv.current.revision,
            selected_hash: pv.current.content_hash,
            object: descriptor,
            release_seen: false,
            selector: Some((40, false)),
            approval: false,
            invalid: false,
        });
        s.preview_pins.insert(sid, pin.life.clone());
        s.shift_focus(sid, Route::Preview, None)?;
        p.tickets
            .get_mut(&ticket.life.id)
            .ok_or(Status::NotReady)?
            .state = TicketState::Consumed;
        let mut r = reply(&ticket.request, chrome.class, Status::Ok);
        put64(&mut r.payload, 8, plan);
        put64(&mut r.payload, 16, revision);
        put64(&mut r.payload, 24, descriptor);
        put32(&mut r.payload, 32, 464);
        put32(&mut r.payload, 40, 63);
        #[cfg(feature = "editor_native_save_v0_dev")]
        if s.save_mode {
            s.exchanges.push((ticket.request, r));
        }
        Ok(r)
    }
    pub fn confirm_store_preview(
        &self,
        chrome: &PeerContext,
        ticket: &mut super::PreviewUiTicket,
        now: u64,
    ) -> Result<super::PendingStoreInstance, Status> {
        use super::native_preview::*;
        let mut s = self.state.lock().map_err(|_| Status::Internal)?;
        self.preview_chrome_locked(&s, chrome)?;
        let authority = self.native_authority()?;
        if !same(&authority, &ticket.authority)
            || ticket.view.session_id != chrome.session
            || ticket.view.kind != PreviewUiTicketKind::Approval
        {
            return Err(Status::Denied);
        }
        let now = self.clock.fetch_max(now, Ordering::SeqCst).max(now);
        let now = self.preview_setup_time_locked(&s, &authority, now)?;
        self.clock.fetch_max(now, Ordering::SeqCst);
        s.expire(now);
        #[cfg(feature = "editor_native_save_v0_dev")]
        if s.save_mode && s.save_pending.len() >= 16 {
            return Err(Status::Exhausted);
        }
        let mut gate = lock(&authority.gate)?;
        #[cfg(feature = "editor_native_save_v0_dev")]
        let now = if s.save_mode {
            if gate.save.is_none() {
                return Err(Status::Denied);
            }
            let now = now
                .max(gate.now_ms)
                .max(gate.preview.as_ref().ok_or(Status::Denied)?.now);
            gate.now_ms = now;
            gate.preview.as_mut().ok_or(Status::Denied)?.expire(now);
            self.clock.fetch_max(now, Ordering::SeqCst);
            now
        } else {
            now
        };
        let p = gate.preview.as_mut().ok_or(Status::Denied)?;
        p.prune();
        let tr = p.tickets.get(&ticket.life.id).ok_or(Status::NotReady)?;
        if tr.state == TicketState::Consumed {
            return Err(Status::NotReady);
        }
        if tr.state != TicketState::Taken
            || now >= tr.view.expires_at_ms
            || Instant::now() >= tr.view.deadline
        {
            return Err(Status::Stale);
        }
        let sid = chrome.session;
        let session = &s.sessions[&sid];
        let pv = session.preview.as_ref().ok_or(Status::Stale)?;
        let pin = s.preview_pins.get(&sid).ok_or(Status::Stale)?.clone();
        let pinrow = p.pins.get(&pin.id).ok_or(Status::Stale)?;
        if pv.invalid
            || !pv.release_seen
            || pv.plan != ticket.view.plan_id
            || pv.revision != ticket.view.preview_revision
            || pv.policy != session.policy
            || pv.application != session.application
            || pv.manifest != session.manifest
            || ticket.view.device_generation != session.queue_generation
            || ticket.view.focus_epoch != session.focus
            || now >= pv.expires
            || pinrow.retired
            || now >= pinrow.view.expires_at_ms
            || Instant::now() >= pinrow.view.deadline
        {
            return Err(Status::Stale);
        }
        if p.attachments.len() >= 16
            || s.instances.len() >= 16
            || s.endpoints.len() + 3 > 64
            || s.objects.len() + 1 > s.shared_object_limit()
        {
            return Err(Status::Exhausted);
        }
        let version = p.next_attachment.checked_add(1).ok_or(Status::Exhausted)?;
        let status = p.next_status.checked_add(1).ok_or(Status::Exhausted)?;
        let context = p.next_context.checked_add(1).ok_or(Status::Exhausted)?;
        let expires = ticket
            .view
            .expires_at_ms
            .checked_sub(1000)
            .ok_or(Status::Invalid)?
            .checked_add(s.config.instance_ttl_ms)
            .ok_or(Status::Exhausted)?;
        let plan = pv.plan;
        let revision = pv.revision;
        let setup_expires = pv
            .expires
            .min(pinrow.view.expires_at_ms)
            .min(ticket.view.expires_at_ms);
        let pinview = pinrow.view.clone();
        let old = pv.object;
        s.reserve_identities(9)?;
        let id = s.identity()?;
        let generation = s.identity()?;
        let surface = s.identity()?;
        let surface_generation = s.identity()?;
        let epoch = s.service_epoch;
        let endpoints = [
            s.endpoint(sid, id, EndpointClass::SelfStatus, id, epoch, 1)?,
            s.endpoint(sid, id, EndpointClass::FocusRead, id, epoch, 1)?,
            s.endpoint(sid, id, EndpointClass::Surface, surface, epoch, 15)?,
        ];
        let _context_identity = s.identity()?;
        let grants = s.object(
            sid,
            id,
            ObjectKind::Grants,
            0,
            0,
            Data::Immutable(Arc::new(Vec::new())),
        )?;
        let life = Arc::new(AttachmentLife {
            id,
            pin: pin.id,
            pin_identity: Arc::downgrade(&pin),
        });
        let setup = Arc::new(PendingSetup {
            _pin: pin.clone(),
            _ticket: ticket.life.clone(),
        });
        let actor = artifact_store_schema::editor_save::EditorActorV0 {
            owner_id: session_owner(&s, sid)?,
            session_id: sid,
            session_generation: s.sessions[&sid].generation,
            instance_id: id,
            instance_generation: generation,
        };
        p.next_attachment = version;
        p.next_status = status;
        p.next_context = context;
        p.attachments.insert(
            id,
            AttachmentRow {
                life: Arc::downgrade(&life),
                view: PendingView {
                    actor,
                    pin: pinview,
                    plan_id: plan,
                    preview_revision: revision,
                    instance_expires_at_ms: expires,
                },
                context,
                version,
                status_version: status,
                installation: None,
                attempt_started: false,
                active: false,
                delivered: false,
                retired: false,
                faulted: false,
                status_exhausted: false,
                logical_setup_expires: setup_expires,
                request_deadline: ticket.view.deadline,
                endpoints,
                grants,
                confirm_request: ticket.request,
            },
        );
        p.tickets
            .get_mut(&ticket.life.id)
            .ok_or(Status::NotReady)?
            .state = TicketState::Consumed;
        s.instances.insert(
            id,
            Instance {
                session: sid,
                generation,
                expires,
                state: InstanceState::Starting,
                bindings: [endpoints[0], endpoints[1], endpoints[2], 0],
                grants,
                surface: Surface {
                    id: surface,
                    generation: surface_generation,
                    buffers: [0; 2],
                    created: false,
                    destroyed: false,
                    sequence: 0,
                    frozen: None,
                    epoch,
                },
                recovery: false,
            },
        );
        s.preview_instances.insert(id, life.clone());
        s.objects.remove(&old);
        s.sessions
            .get_mut(&sid)
            .ok_or(Status::Denied)?
            .preview
            .as_mut()
            .ok_or(Status::Stale)?
            .invalid = true;
        let pending = PendingStoreInstance {
            authority: authority.clone(),
            life,
            setup,
        };
        // Save's terminal State owner is installed in the same State->Gate
        // confirmation transaction. Public caller ownership stays independent.
        #[cfg(feature = "editor_native_save_v0_dev")]
        if s.save_mode {
            s.save_pending.insert(
                id,
                Arc::new(PendingStoreInstance {
                    authority: pending.authority.clone(),
                    life: pending.life.clone(),
                    setup: pending.setup.clone(),
                }),
            );
        }
        Ok(pending)
    }
}
#[cfg(feature = "editor_native_preview_v0_dev")]
fn session_owner(s: &State, sid: u64) -> Result<u64, Status> {
    Ok(s.sessions.get(&sid).ok_or(Status::Denied)?.owner)
}

#[cfg(feature = "editor_native_preview_v0_dev")]
impl HostDesktop {
    fn endpoint_locked(&self, s: &State, key: u64) -> Result<Endpoint, Status> {
        let e = s.endpoints.get(&key).ok_or(Status::Stale)?;
        Ok(Endpoint {
            peer: PeerContext {
                registry: self.state.clone(),
                endpoint: key,
                session: e.session,
                instance: e.instance,
                class: e.class,
            },
            handle: Handle::unpack(key),
        })
    }
    pub fn take_store_launch(
        &self,
        chrome: &PeerContext,
        pending: &super::PendingStoreInstance,
        stamp: &super::StoreActivationStamp,
        now: u64,
    ) -> Result<super::StoreLaunch, super::NativePreviewDeliveryFailure> {
        use super::native_preview::*;
        let mut observation = DeliveryObservation {
            stage: PreviewDeliveryStage::Authorize,
            status: Status::Denied,
            authorized_attempt: false,
            active: false,
            retired: false,
        };
        let result = (|| {
            let mut s = self.state.lock().map_err(|_| Status::Internal)?;
            self.preview_chrome_locked(&s, chrome)?;
            let authority = self.native_authority()?;
            if !same(&authority, &pending.authority)
                || !same(&authority, &stamp.authority)
                || !Arc::ptr_eq(&pending.life, &stamp.life)
            {
                return Err(Status::Denied);
            }
            {
                let g = lock(&authority.gate)?;
                let p = g.preview.as_ref().ok_or(Status::Denied)?;
                let r = p.row(&pending.life)?;
                if r.view.actor.session_id != chrome.session || r.version != stamp.version {
                    return Err(Status::Denied);
                }
            }
            observation.authorized_attempt = true;
            observation.stage = PreviewDeliveryStage::Validate;
            let now = self.clock.fetch_max(now, Ordering::SeqCst).max(now);
            s.expire(now);
            let mut gate = lock(&authority.gate)?;
            let p = gate.preview.as_mut().ok_or(Status::Denied)?;
            let r = p.row(&pending.life)?;
            observation.active = r.active;
            let already_delivered = r.delivered;
            self.admit_preview_bootstrap_locked(&s, p, &pending.life)?;
            if already_delivered {
                return Err(Status::NotReady);
            }
            let r = p
                .attachments
                .get_mut(&pending.life.id)
                .ok_or(Status::Denied)?;
            if r.retired
                || now >= r.view.instance_expires_at_ms
                || !r.delivered
                    && (now >= r.logical_setup_expires || Instant::now() >= r.request_deadline)
            {
                r.retired = true;
                observation.retired = true;
                return Err(Status::Stale);
            }
            if !r.active {
                return Err(Status::NotReady);
            }
            if r.delivered {
                return Err(Status::NotReady);
            }
            #[cfg(feature = "editor_native_save_v0_dev")]
            if s.save_mode && s.exchanges.len() >= s.trace_limit() {
                s.trace_exhausted = true;
                return Err(Status::Exhausted);
            }
            let installation = r.installation.clone().ok_or(Status::NotReady)?;
            let id = pending.life.id;
            let i = s.instances.get(&id).ok_or(Status::Stale)?;
            let bytes = Self::preview_data(
                &s,
                chrome.session,
                &r.view.pin,
                r.view.plan_id,
                r.view.preview_revision,
                r.view.instance_expires_at_ms,
                Some((
                    id,
                    i.generation,
                    i.surface.id,
                    i.surface.generation,
                    r.endpoints,
                    installation.store_handle,
                )),
            )?;
            let grants = r.grants;
            let endpoints = r.endpoints;
            let generation = i.generation;
            let expires = r.view.instance_expires_at_ms;
            #[cfg(feature = "editor_native_save_v0_dev")]
            let confirm_request = r.confirm_request;
            let mut confirm = reply(&r.confirm_request, EndpointClass::Chrome, Status::Ok);
            put64(&mut confirm.payload, 8, r.view.actor.session_generation);
            put64(&mut confirm.payload, 16, id);
            put64(&mut confirm.payload, 24, generation);
            put64(&mut confirm.payload, 32, grants);
            put32(&mut confirm.payload, 40, 464);
            let bootstrap = editor::InstanceBootstrap {
                session_id: chrome.session,
                session_generation: r.view.actor.session_generation,
                instance_id: id,
                instance_generation: generation,
                grants_shm: grants,
                grants_len: 464,
                status: 0,
            }
            .encode(Handle::INVALID)?;
            let bindings = NativePreviewBindings {
                bootstrap,
                self_status: self.endpoint_locked(&s, endpoints[0])?,
                focus_read: self.endpoint_locked(&s, endpoints[1])?,
                surface: self.endpoint_locked(&s, endpoints[2])?,
                artifact_origin: PreviewReadOrigin {
                    authority: authority.clone(),
                    life: pending.life.clone(),
                    context: r.context,
                },
            };
            observation.stage = PreviewDeliveryStage::BuildReply;
            s.objects.get_mut(&grants).ok_or(Status::Stale)?.data =
                Data::Immutable(Arc::new(bytes));
            // All fallible construction is complete while the actual State/Gate
            // retain the same current admission. The visible carrier follows.
            r.delivered = true;
            if let Some(pin) = p.pins.get_mut(&pending.life.pin) {
                pin.consumed = true;
            }
            if s.preview_pins
                .get(&chrome.session)
                .is_some_and(|pin| pin.id == pending.life.pin)
            {
                s.preview_pins.remove(&chrome.session);
            }
            let session = s.sessions.get_mut(&chrome.session).ok_or(Status::Stale)?;
            session.route = Route::App;
            session.focused = None;
            session
                .object
                .inner
                .lock()
                .unwrap()
                .live
                .insert(id, (generation, expires));
            #[cfg(feature = "editor_native_save_v0_dev")]
            if s.save_mode {
                s.exchanges.push((confirm_request, confirm));
            }
            observation.stage = PreviewDeliveryStage::Expose;
            Ok(StoreLaunch {
                confirm_reply: confirm,
                bindings,
                stamp: StoreActivationStamp {
                    authority: authority.clone(),
                    life: pending.life.clone(),
                    version: stamp.version,
                },
            })
        })();
        result.map_err(|status| {
            if observation.authorized_attempt {
                if let Ok(mut s) = self.state.lock() {
                    if let Ok(authority) = self.native_authority() {
                        let retire = {
                            let mut gate = authority.gate.lock().unwrap_or_else(|p| p.into_inner());
                            if let Some(p) = gate.preview.as_mut() {
                                if let Some(row) = p.attachments.get_mut(&pending.life.id) {
                                    if !row.delivered {
                                        row.retired = true;
                                        true
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        };
                        if retire {
                            s.retire(pending.life.id, InstanceState::Revoked);
                            observation.retired = true;
                        }
                    }
                }
            }
            observation.status = status;
            NativePreviewDeliveryFailure { observation }
        })
    }
    pub fn begin_preview_read(
        &self,
        origin: &super::PreviewReadOrigin,
        q: &Envelope,
        now: u64,
    ) -> Result<super::PreviewReadEntry, Status> {
        use super::native_preview::*;
        let entered = Instant::now();
        let authority = self.native_authority()?;
        if !same(&authority, &origin.authority) {
            return Err(Status::Denied);
        }
        encode_envelope_wire(q).and_then(|b| decode_envelope_wire(&b))?;
        if q.protocol != 368 || !matches!(q.msg_type, 1 | 3 | 5 | 7) {
            return Err(Status::Unsupported);
        }
        let mut s = self.state.lock().map_err(|_| Status::Internal)?;
        let context = {
            let g = lock(&authority.gate)?;
            let p = g.preview.as_ref().ok_or(Status::Denied)?;
            let r = p.row(&origin.life)?;
            let installed = r.installation.as_ref().ok_or(Status::NotReady)?;
            if q.handle != installed.store_handle
                || q64(q, 8) != r.view.actor.session_id
                || q64(q, 16) != r.view.actor.session_generation
            {
                return Err(Status::Denied);
            }
            if q.msg_type == 1
                && (q64(q, 24) != r.view.actor.instance_id
                    || q64(q, 32) != r.view.actor.instance_generation)
            {
                return Err(Status::Denied);
            }
            if origin.context != r.context {
                return Err(Status::Denied);
            }
            r.context
        };
        if q.msg_type != 1 {
            return Err(Status::Unsupported);
        }
        let now = self.clock.fetch_max(now, Ordering::SeqCst).max(now);
        s.expire(now);
        {
            let g = lock(&authority.gate)?;
            let p = g.preview.as_ref().ok_or(Status::Denied)?;
            p.admit_instance(origin.life.id)?;
            let r = p.row(&origin.life)?;
            if !r.delivered {
                return Err(Status::NotReady);
            }
        }
        // The actual native entry executor repeats Gate admission, reserves the
        // real pair atomically, installs both handles and preserves its entry time.
        drop(s);
        let native = authority.start(context, *q, entered)?;
        Ok(PreviewReadEntry {
            native,
            authority,
            life: origin.life.clone(),
        })
    }
    pub fn dispatch_store_preview_chrome(
        &self,
        chrome: &PeerContext,
        q: &Envelope,
        now: u64,
    ) -> Envelope {
        use super::native_preview::*;
        let outcome = (|| {
            encode_envelope_wire(q).and_then(|b| decode_envelope_wire(&b))?;
            let mut s = self.state.lock().map_err(|_| Status::Internal)?;
            self.preview_chrome_locked(&s, chrome)?;
            if q.handle.pack() != chrome.endpoint
                || q.protocol != 352
                || q64(q, 8) != chrome.session
                || q64(q, 16) != s.sessions[&chrome.session].generation
            {
                return Err(Status::Denied);
            }
            let authority = self.native_authority()?;
            let now = self.clock.fetch_max(now, Ordering::SeqCst).max(now);
            s.expire(now);
            let mut g = lock(&authority.gate)?;
            let p = g.preview.as_mut().ok_or(Status::Denied)?;
            if q.msg_type == 3 {
                let session = &s.sessions[&chrome.session];
                let preview = session.preview.as_ref().ok_or(Status::Stale)?;
                // CancelPreview carries only plan_id; the current private
                // preview owns its revision. Reserved wire tail is not a revision.
                if preview.plan != q64(q, 24) {
                    return Err(Status::Denied);
                }
                if p.attachments.values().any(|r| {
                    r.view.actor.session_id == chrome.session
                        && r.view.plan_id == q64(q, 24)
                        && r.active
                        && !r.retired
                }) {
                    return Err(Status::NotReady);
                }
                p.retire_setup(chrome.session);
                s.preview_pins.remove(&chrome.session);
                let session = s.sessions.get_mut(&chrome.session).ok_or(Status::Denied)?;
                if let Some(preview) = &mut session.preview {
                    preview.invalid = true;
                    preview.approval = false;
                }
                drop(g);
                s.settle_preview_retirements();
                s.shift_focus(chrome.session, Route::Launcher, None)?;
                Ok(reply(q, chrome.class, Status::Ok))
            } else if matches!(q.msg_type, 9 | 11) {
                let id = q64(q, 24);
                let i = s.instances.get(&id).ok_or(Status::Stale)?;
                if i.session != chrome.session {
                    return Err(Status::Denied);
                }
                if i.generation != q64(q, 32) {
                    return Err(Status::Stale);
                }
                drop(g);
                s.retire(id, InstanceState::Revoked);
                s.shift_focus(chrome.session, Route::Launcher, None)?;
                Ok(reply(q, chrome.class, Status::Ok))
            } else {
                Err(Status::Unsupported)
            }
        })();
        outcome.unwrap_or_else(|status| reply(q, chrome.class, status))
    }
}
#[cfg(feature = "editor_native_preview_v0_dev")]
impl FixtureController {
    pub fn register_store_session(
        &self,
        owner: u64,
        application: [u8; 32],
        manifest: [u8; 32],
        object_id: u64,
        object_generation: u64,
        now: u64,
    ) -> Result<SessionFixture, Status> {
        use super::native_preview::*;
        if object_id == 0 || object_generation == 0 {
            return Err(Status::Invalid);
        }
        if !self.host.preview_mode() {
            return Err(Status::Unsupported);
        }
        {
            let authority = self.host.native_authority()?;
            let g = lock(&authority.gate)?;
            let p = g.preview.as_ref().ok_or(Status::Denied)?;
            if p.owners.values().any(|r| r.object == object_id) {
                return Err(Status::Denied);
            }
        }
        let fixture = self.register_session(owner, application, manifest, b"", now)?;
        let s = self.host.state.lock().map_err(|_| Status::Internal)?;
        let authority = self.host.native_authority()?;
        let mut g = lock(&authority.gate)?;
        let p = g.preview.as_mut().ok_or(Status::Denied)?;
        if p.owners.values().any(|r| r.object == object_id) {
            return Err(Status::Denied);
        }
        p.owners.insert(
            fixture.session_id,
            OwnerRow {
                owner: PreviewOwner {
                    owner_id: owner,
                    session_id: fixture.session_id,
                    session_generation: fixture.session_generation,
                },
                chrome: fixture.chrome.handle.pack(),
                state_id: Arc::as_ptr(&self.host.state) as usize,
                object: object_id,
                generation: object_generation,
            },
        );
        drop(s);
        Ok(fixture)
    }
    pub fn pause_next_store_read(&self, session: u64) -> Result<super::NativeReadPause, Status> {
        if !self.host.preview_mode() {
            return Err(Status::Unsupported);
        }
        self.pause_next_native_read(session)
    }
    pub fn release_store_read(&self, pause: &super::NativeReadPause) -> Result<(), Status> {
        self.release_native_read(pause)
    }
    pub fn seed_preview_counter(&self, counter: super::NativePreviewCounter) -> Result<(), Status> {
        use super::native_preview::*;
        let _s = self.host.state.lock().map_err(|_| Status::Internal)?;
        let a = self.host.native_authority()?;
        let mut g = lock(&a.gate)?;
        let p = g.preview.as_mut().ok_or(Status::Denied)?;
        match counter {
            NativePreviewCounter::Pin => p.next_pin = u64::MAX,
            NativePreviewCounter::AttachmentVersion => p.next_attachment = u64::MAX,
            NativePreviewCounter::StatusVersion => p.next_status = u64::MAX,
            NativePreviewCounter::UiTicket => p.next_ticket = u64::MAX,
            NativePreviewCounter::ReadContext => p.next_context = u64::MAX,
        }
        Ok(())
    }
    pub fn pause_next_store_preview(
        &self,
        session: u64,
        point: super::PreviewPausePoint,
    ) -> Result<super::PreviewPause, Status> {
        use super::native_preview::*;
        let s = self.host.state.lock().map_err(|_| Status::Internal)?;
        if !s.sessions.contains_key(&session) {
            return Err(Status::Denied);
        }
        let authority = self.host.native_authority()?;
        let mut g = lock(&authority.gate)?;
        let p = g.preview.as_mut().ok_or(Status::Denied)?;
        p.prune();
        if p.armed.is_some() {
            return Err(Status::NotReady);
        }
        if p.barriers.len() >= 33 {
            return Err(Status::Exhausted);
        }
        let barrier = PreviewBarrier::new(session, point);
        p.barriers.push(Arc::downgrade(&barrier));
        p.armed = Some((session, point, barrier.clone()));
        Ok(PreviewPause {
            authority: authority.clone(),
            barrier,
        })
    }
    pub fn release_store_preview(&self, pause: &super::PreviewPause) -> Result<(), Status> {
        let a = self.host.native_authority()?;
        if !super::native_preview::same(&a, &pause.authority) {
            return Err(Status::Denied);
        }
        pause.barrier.release()
    }
}

#[cfg(feature = "editor_native_preview_v0_dev")]
impl State {
    fn retire_preview_setup(&mut self, sid: u64) {
        if let Some(a) = &self.native {
            let mut g = a.gate.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(p) = &mut g.preview {
                p.retire_setup(sid);
            }
        }
        self.preview_pins.remove(&sid);
        self.settle_preview_retirements();
    }
    fn settle_preview_retirements(&mut self) {
        let retired = self
            .native
            .as_ref()
            .map(|authority| {
                let g = authority.gate.lock().unwrap_or_else(|p| p.into_inner());
                g.preview
                    .as_ref()
                    .map(|p| {
                        p.attachments
                            .iter()
                            .filter(|(_, r)| r.retired)
                            .map(|(&id, _)| id)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        for id in retired {
            // Gate used to select these actual terminal rows is already dropped.
            #[cfg(feature = "editor_native_save_v0_dev")]
            self.release_save_pending(id);
            if self.preview_instances.contains_key(&id) {
                self.retire(id, InstanceState::Revoked);
            }
        }
        // Only actual terminal State ownership is released. Opaque pending,
        // carriers, leases and held producers continue to keep their rows.
        if let Some(authority) = &self.native {
            let mut g = authority.gate.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(p) = &mut g.preview {
                self.preview_pins
                    .retain(|_, pin| p.pins.get(&pin.id).is_some_and(|r| !r.retired));
                p.prune();
                self.instances.retain(|id, i| {
                    matches!(i.state, InstanceState::Starting | InstanceState::Running)
                        || p.attachments.contains_key(id)
                });
            }
        }
    }
}
impl State {
    fn consume_frozen(&mut self, id: u64, sequence: u64) -> Result<(), Status> {
        let i = self.instances.get_mut(&id).ok_or(Status::Stale)?;
        if i.surface.frozen.as_ref().map(|(_, seq, _)| *seq) != Some(sequence) {
            return Err(Status::Stale);
        }
        i.surface.frozen = None;
        Ok(())
    }
}
#[cfg(feature = "editor_native_preview_v0_dev")]
impl HostDesktop {
    fn retain_preview_lease(&self, kind: ObjectKind) -> Result<Option<Arc<()>>, Status> {
        use super::native_preview::*;
        if !self.preview_mode() || !matches!(kind, ObjectKind::Preview | ObjectKind::Grants) {
            return Ok(None);
        }
        let a = self.native_authority()?;
        let mut g = lock(&a.gate)?;
        let p = g.preview.as_mut().ok_or(Status::Denied)?;
        p.prune();
        if p.leases.len() >= 16 {
            return Err(Status::Exhausted);
        }
        let life = Arc::new(());
        p.leases.push(Arc::downgrade(&life));
        Ok(Some(life))
    }
    fn admit_preview_lease_locked(
        &self,
        s: &State,
        peer: &PeerContext,
        d: &ObjectDescriptor,
    ) -> Result<(), Status> {
        use super::native_preview::*;
        if !self.preview_mode() {
            return Ok(());
        }
        let a = self.native_authority()?;
        let g = lock(&a.gate)?;
        let p = g.preview.as_ref().ok_or(Status::Denied)?;
        if peer.instance != 0 && s.preview_instances.contains_key(&peer.instance) {
            p.admit_instance(peer.instance)?;
        }
        if d.kind == ObjectKind::Preview {
            let life = s.preview_pins.get(&peer.session).ok_or(Status::Stale)?;
            let r = p.pins.get(&life.id).ok_or(Status::Stale)?;
            if r.retired || p.now >= r.view.expires_at_ms || Instant::now() >= r.view.deadline {
                return Err(Status::Stale);
            }
        }
        Ok(())
    }
    fn compose_store_preview(
        &self,
        compositor: &Endpoint,
        now: u64,
    ) -> Result<ComposedFrame, Status> {
        use super::native_preview::*;
        use artifact_store_schema::editor_preview::*;
        if !Arc::ptr_eq(&self.state, &compositor.peer.registry)
            || compositor.peer.class != EndpointClass::Compositor
            || compositor.handle.pack() != compositor.peer.endpoint
        {
            return Err(Status::Denied);
        }
        self.advance(now);
        let authority = self.native_authority()?;
        let (
            sid,
            id,
            focus,
            sequence,
            source,
            font,
            version,
            status,
            unavailable,
            record,
            setup,
            pause,
        ) = {
            let mut s = self.state.lock().map_err(|_| Status::Internal)?;
            s.auth(&compositor.peer)?;
            let sid = compositor.peer.session;
            if s.sessions[&sid].compositor != compositor.peer.endpoint {
                return Err(Status::Denied);
            }
            if s.frame_limit_hit() {
                return Err(Status::Exhausted);
            }
            let mut g = lock(&authority.gate)?;
            let p = g.preview.as_mut().ok_or(Status::Denied)?;
            let active = p
                .attachments
                .iter()
                .rev()
                .find(|(_, r)| r.view.actor.session_id == sid && r.delivered)
                .map(|(&id, _)| id);
            let (id, focus, sequence, source, version, status, unavailable, record, setup) =
                if let Some(id) = active {
                    let r = &p.attachments[&id];
                    if r.status_exhausted {
                        return Err(Status::Exhausted);
                    }
                    let unavailable = r.faulted;
                    if r.retired && !unavailable {
                        return Err(Status::Stale);
                    }
                    if unavailable && s.terminal_versions.get(&sid) == Some(&r.status_version) {
                        return Err(Status::NotReady);
                    }
                    let i = s.instances.get(&id).ok_or(Status::Stale)?;
                    let (focus, sequence, source) = if unavailable {
                        (0, 0, None)
                    } else {
                        p.admit_instance(id)?;
                        let (_, seq, data) = i.surface.frozen.as_ref().ok_or(Status::NotReady)?;
                        (s.sessions[&sid].focus, *seq, Some(data.clone()))
                    };
                    let c = &r.view.pin.current;
                    let encoded = EditorPreviewChromeV0 {
                        schema_version: CHROME_SCHEMA_VERSION,
                        total_len: 248,
                        state: if unavailable {
                            EditorPreviewChromeStateV0::Unavailable
                        } else {
                            EditorPreviewChromeStateV0::NoSave
                        },
                        reserved: 0,
                        actor: r.view.actor.clone(),
                        selected_object_id: c.object_id,
                        selected_object_generation: c.object_generation,
                        current_store_epoch: c.store_epoch,
                        original_backend_epoch: 0,
                        operation_id: 0,
                        source_handle: 0,
                        source_generation: 0,
                        expected_revision: 0,
                        result_revision: 0,
                        attachment_version: r.version,
                        status_version: r.status_version,
                        source_content_hash: [0; 32],
                        expected_content_hash: [0; 32],
                        receipt_sha256: [0; 32],
                        source_body_len: 0,
                        tail_reserved: 0,
                    }
                    .encode_le()
                    .map_err(|_| Status::Invalid)?;
                    (
                        id,
                        focus,
                        sequence,
                        source,
                        r.version,
                        r.status_version,
                        unavailable,
                        Some(encoded),
                        None,
                    )
                } else {
                    let preview = s.sessions[&sid].preview.as_ref().ok_or(Status::NotReady)?;
                    let pin = s.preview_pins.get(&sid).ok_or(Status::NotReady)?;
                    let row = p.pins.get(&pin.id).ok_or(Status::Stale)?;
                    if preview.invalid
                        || row.retired
                        || p.now >= preview.expires
                        || p.now >= row.view.expires_at_ms
                        || Instant::now() >= row.view.deadline
                        || !std::sync::Weak::ptr_eq(&row.life, &Arc::downgrade(pin))
                    {
                        return Err(Status::Stale);
                    }
                    let session = &s.sessions[&sid];
                    let setup = PreinstanceFrameSetup {
                        pin: pin.clone(),
                        pin_view: row.view.clone(),
                        plan: preview.plan,
                        revision: preview.revision,
                        object: preview.object,
                        expires: preview.expires,
                        policy: preview.policy,
                        session_generation: session.generation,
                        application: session.application,
                        manifest: session.manifest,
                    };
                    (0, 0, 0, None, 0, 0, false, None, Some(setup))
                };
            let pause = if p.armed.as_ref().is_some_and(|(session, point, _)| {
                *session == sid && *point == PreviewPausePoint::BeforeFramePublish
            }) {
                p.armed.take().map(|(_, _, b)| b)
            } else {
                None
            };
            s.pending_frames += 1;
            (
                sid,
                id,
                focus,
                sequence,
                source,
                s.config.font_rows,
                version,
                status,
                unavailable,
                record,
                setup,
                pause,
            )
        };
        let mut reservation = FrameReservation {
            state: self.state.clone(),
            finished: false,
        };
        let _settlement = PreviewSettlement(pause.clone());
        if let Some(b) = &pause {
            b.enter_wait(id, 0)?;
        }
        let mut pixels = vec![224; 640 * 568 * 4];
        for pixel in pixels.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        pixels[48 * 2560..528 * 2560].fill(255);
        if !unavailable {
            if let Some(source) = &source {
                pixels[48 * 2560..528 * 2560].copy_from_slice(source);
            }
        }
        for y in 8..24 {
            for x in 8..24 {
                let offset = (y * 640 + x) * 4;
                pixels[offset..offset + 4].copy_from_slice(
                    if x == 8 || x == 23 || y == 8 || y == 23 {
                        &[0, 0, 0, 255]
                    } else {
                        &[0, 192, 0, 255]
                    },
                );
            }
        }
        draw_label(&mut pixels, &font, b"HOST STORE", 40, 16);
        draw_label(&mut pixels, &font, b"READ ONLY", 144, 16);
        draw_label(&mut pixels, &font, b"NO SAVE", 8, 536);
        if unavailable {
            draw_label(&mut pixels, &font, b"UNAVAILABLE", 88, 536);
        }
        {
            let mut s = self.state.lock().map_err(|_| Status::Internal)?;
            s.expire(self.clock.load(Ordering::SeqCst));
            s.auth(&compositor.peer)?;
            let g = lock(&authority.gate)?;
            let p = g.preview.as_ref().ok_or(Status::Denied)?;
            if id != 0 {
                let r = p.attachments.get(&id).ok_or(Status::Stale)?;
                if r.version != version
                    || r.status_version != status
                    || r.faulted != unavailable
                    || r.status_exhausted
                {
                    return Err(Status::Stale);
                }
                if !unavailable {
                    p.admit_instance(id)?;
                    if s.sessions[&sid].focus != focus {
                        return Err(Status::Stale);
                    }
                    let i = &s.instances[&id];
                    if i.surface.frozen.as_ref().map(|(_, seq, _)| *seq) != Some(sequence)
                        || i.surface.destroyed
                    {
                        return Err(Status::Stale);
                    }
                    let q = kernel_api::generated::desktop_surface_v1::Consume {
                        request_id: sequence,
                        surface_id: i.surface.id,
                        surface_generation: i.surface.generation,
                        sequence,
                    }
                    .encode(compositor.handle)?;
                    if s.exchanges.len() >= s.trace_limit() {
                        return Err(Status::Exhausted);
                    }
                    s.consume_frozen(id, sequence)?;
                    let mut r = reply(&q, EndpointClass::Compositor, Status::Ok);
                    put64(&mut r.payload, 8, sequence);
                    s.exchanges.push((q, r));
                } else {
                    s.terminal_versions.insert(sid, status);
                }
                let bytes = record.ok_or(Status::Internal)?;
                s.chrome_records.insert(
                    sid,
                    PreviewChromeRecordObservation {
                        session_id: sid,
                        instance_id: id,
                        focus_epoch: focus,
                        sequence,
                        attachment_version: version,
                        status_version: status,
                        bytes,
                    },
                );
            } else {
                // Revalidate the SAME pre-instance snapshot under State->Gate
                // after rasterization/pause. A replacement pin or preview must
                // not authorize publication of the older captured frame.
                let setup = setup.as_ref().ok_or(Status::Internal)?;
                let session = s.sessions.get(&sid).ok_or(Status::Stale)?;
                let preview = session.preview.as_ref().ok_or(Status::Stale)?;
                let pin = s.preview_pins.get(&sid).ok_or(Status::Stale)?;
                let row = p.pins.get(&setup.pin.id).ok_or(Status::Stale)?;
                if !Arc::ptr_eq(pin, &setup.pin)
                    || !std::sync::Weak::ptr_eq(&row.life, &Arc::downgrade(&setup.pin))
                    || row.view.pin_id != setup.pin_view.pin_id
                    || row.view.pin_generation != setup.pin_view.pin_generation
                    || row.view.entered != setup.pin_view.entered
                    || row.view.deadline != setup.pin_view.deadline
                    || row.retired
                    || preview.invalid
                    || preview.plan != setup.plan
                    || preview.revision != setup.revision
                    || preview.object != setup.object
                    || preview.expires != setup.expires
                    || preview.policy != setup.policy
                    || session.policy != setup.policy
                    || session.generation != setup.session_generation
                    || session.application != setup.application
                    || session.manifest != setup.manifest
                    || p.now >= setup.expires
                    || p.now >= setup.pin_view.expires_at_ms
                    || Instant::now() >= setup.pin_view.deadline
                {
                    return Err(Status::Stale);
                }
            }
            s.pending_frames -= 1;
            s.composed_frames += 1;
            reservation.finished = true;
        }
        Ok(ComposedFrame {
            session_id: sid,
            instance_id: id,
            focus_epoch: focus,
            sequence,
            bgra: pixels,
        })
    }
}
#[cfg(feature = "editor_native_preview_v0_dev")]
impl FixtureController {
    pub fn preview_chrome_observation(
        &self,
        compositor: &Endpoint,
    ) -> Result<super::PreviewChromeRecordObservation, Status> {
        use super::native_preview::*;
        if !Arc::ptr_eq(&self.host.state, &compositor.peer.registry)
            || compositor.peer.class != EndpointClass::Compositor
            || compositor.handle.pack() != compositor.peer.endpoint
        {
            return Err(Status::Denied);
        }
        let s = self.host.state.lock().map_err(|_| Status::Internal)?;
        s.auth(&compositor.peer)?;
        let record = s
            .chrome_records
            .get(&compositor.peer.session)
            .ok_or(Status::NotReady)?;
        let a = self.host.native_authority()?;
        let g = lock(&a.gate)?;
        let p = g.preview.as_ref().ok_or(Status::Denied)?;
        let row = p
            .attachments
            .get(&record.instance_id)
            .ok_or(Status::Stale)?;
        if row.version != record.attachment_version
            || row.status_version != record.status_version
            || row.status_exhausted
            || row.retired && !row.faulted
        {
            return Err(Status::Stale);
        }
        let expected = if row.faulted { 4 } else { 0 };
        if u32::from_le_bytes(
            record.bytes[8..12]
                .try_into()
                .map_err(|_| Status::Internal)?,
        ) != expected
        {
            return Err(Status::Stale);
        }
        Ok(PreviewChromeRecordObservation {
            session_id: record.session_id,
            instance_id: record.instance_id,
            focus_epoch: record.focus_epoch,
            sequence: record.sequence,
            attachment_version: record.attachment_version,
            status_version: record.status_version,
            bytes: record.bytes,
        })
    }
}

#[cfg(feature = "editor_native_preview_v0_dev")]
impl FixtureController {
    pub fn probe_inactive_preview(
        &self,
        chrome: &PeerContext,
        pending: &super::PendingStoreInstance,
        pause: &super::PreviewPause,
        now: u64,
    ) -> Result<super::InactiveDesktopProbeObservation, Status> {
        use super::native_preview::*;
        use kernel_api::generated::{
            desktop_artifact_v1 as artifact, desktop_focus_v1 as focus,
            desktop_surface_v1 as surface,
        };
        let authority = self.host.native_authority()?;
        let (
            endpoints,
            origin,
            descriptor,
            bootstrap_status,
            grants_lease_status,
            actor,
            correlation,
            surface_id,
            surface_generation,
        ) = {
            let s = self.host.state.lock().map_err(|_| Status::Internal)?;
            self.host.preview_chrome_locked(&s, chrome)?;
            if !same(&authority, &pending.authority) || !same(&authority, &pause.authority) {
                return Err(Status::Denied);
            }
            let g = lock(&authority.gate)?;
            let p = g.preview.as_ref().ok_or(Status::Denied)?;
            let r = p.row(&pending.life)?;
            if r.view.actor.session_id != chrome.session {
                return Err(Status::Denied);
            }
            let installation = r.installation.as_ref().ok_or(Status::NotReady)?;
            let i = s.instances.get(&pending.life.id).ok_or(Status::Stale)?;
            let endpoints = [
                self.host.endpoint_locked(&s, r.endpoints[0])?,
                self.host.endpoint_locked(&s, r.endpoints[1])?,
                self.host.endpoint_locked(&s, r.endpoints[2])?,
            ];
            let descriptor = ObjectDescriptor {
                handle: Handle::unpack(r.grants),
                kind: ObjectKind::Grants,
                object_generation: Handle::unpack(r.grants).generation,
                byte_len: 464,
            };
            let origin = PreviewReadOrigin {
                authority: authority.clone(),
                life: pending.life.clone(),
                context: r.context,
            };
            let admission = p.admit_instance(pending.life.id);
            let bootstrap_admission =
                self.host
                    .admit_preview_bootstrap_locked(&s, p, &pending.life);
            let _actual_bootstrap_grants = (r.grants, r.endpoints);
            let _actual_installation = installation;
            (
                endpoints,
                origin,
                descriptor,
                bootstrap_admission,
                admission,
                r.view.actor.clone(),
                q64(&r.confirm_request, 0),
                i.surface.id,
                i.surface.generation,
            )
        };
        let _hold = pause.probe(pending)?;
        let requests = [
            editor::GetStatus {
                request_id: correlation,
                session_id: actor.session_id,
                session_generation: actor.session_generation,
                instance_id: actor.instance_id,
                instance_generation: actor.instance_generation,
            }
            .encode(endpoints[0].handle)?,
            focus::PollKeys {
                request_id: correlation,
                session_id: actor.session_id,
                session_generation: actor.session_generation,
                instance_id: actor.instance_id,
                instance_generation: actor.instance_generation,
                focus_epoch: 0,
            }
            .encode(endpoints[1].handle)?,
            surface::Create {
                request_id: correlation,
                session_id: actor.session_id,
                session_generation: actor.session_generation,
                instance_id: actor.instance_id,
                instance_generation: actor.instance_generation,
                width: 640,
                height: 480,
                format: 1,
                reserved: 0,
            }
            .encode(endpoints[2].handle)?,
        ];
        let replies =
            std::array::from_fn(|n| self.host.dispatch(&endpoints[n].peer, &requests[n], now));
        let handle = {
            let g = lock(&authority.gate)?;
            g.preview
                .as_ref()
                .ok_or(Status::Denied)?
                .row(&pending.life)?
                .installation
                .as_ref()
                .ok_or(Status::NotReady)?
                .store_handle
        };
        let request = artifact::ReadSelected {
            request_id: correlation,
            session_id: actor.session_id,
            session_generation: actor.session_generation,
            instance_id: actor.instance_id,
            instance_generation: actor.instance_generation,
        }
        .encode(handle)?;
        let origin_status = self
            .host
            .begin_preview_read(&origin, &request, now)
            .map(|_entry| ());
        let actual_lease = self
            .host
            .read_lease(&endpoints[0].peer, &descriptor, now)
            .map(|_lease| ());
        let _surface_identity = (surface_id, surface_generation);
        if actual_lease != grants_lease_status {
            return Err(Status::Internal);
        }
        Ok(InactiveDesktopProbeObservation {
            endpoint_requests: requests,
            endpoint_replies: replies,
            origin_status,
            bootstrap_status,
            grants_lease_status: actual_lease,
        })
    }
}

#[cfg(feature = "editor_native_preview_v0_dev")]
impl HostDesktop {
    fn admit_preview_bootstrap_locked(
        &self,
        s: &State,
        p: &super::native_preview::PreviewGate,
        life: &Arc<super::native_preview::AttachmentLife>,
    ) -> Result<(), Status> {
        let r = p.row(life)?;
        p.admit_instance(life.id)?;
        let i = s.instances.get(&life.id).ok_or(Status::Stale)?;
        if i.generation != r.view.actor.instance_generation
            || i.session != r.view.actor.session_id
            || i.grants != r.grants
            || i.bindings[..3] != r.endpoints
        {
            return Err(Status::Denied);
        }
        let grants = s.objects.get(&r.grants).ok_or(Status::Stale)?;
        if grants.kind != ObjectKind::Grants
            || grants.instance != life.id
            || grants.session != r.view.actor.session_id
        {
            return Err(Status::Denied);
        }
        Ok(())
    }
}

impl State {
    fn trace_limit(&self) -> usize {
        #[cfg(feature = "editor_native_save_v0_dev")]
        if self.save_mode {
            return 131072;
        }
        256
    }
    fn frame_limit_hit(&self) -> bool {
        #[cfg(feature = "editor_native_save_v0_dev")]
        if self.save_mode {
            return self.composed_frames >= 128 || self.pending_frames >= 16;
        }
        self.composed_frames + self.pending_frames >= 16
    }
}

#[cfg(feature = "editor_native_save_v0_dev")]
impl HostDesktop {
    fn save_authority(&self) -> Result<Arc<super::native_authority::NativeAuthority>, Status> {
        let authority = self.native_authority()?;
        if authority
            .gate
            .lock()
            .map_err(|_| Status::Internal)?
            .save
            .is_none()
        {
            return Err(Status::Unsupported);
        }
        Ok(authority)
    }
    pub fn new_store_save(
        config: HostConfig,
    ) -> Result<
        (
            Self,
            FixtureController,
            super::RegistryWitness,
            super::NativeSaveEnrollment,
        ),
        Status,
    > {
        let (host, controller, witness, enrollment) = Self::new_store_preview_read(config)?;
        let authority = host.native_authority()?;
        {
            let mut state = host.state.lock().map_err(|_| Status::Internal)?;
            let mut gate = authority.gate.lock().map_err(|_| Status::Internal)?;
            state.save_mode = true;
            gate.save = Some(super::native_save::SaveGate::new());
        }
        Ok((
            host,
            controller,
            witness,
            super::NativeSaveEnrollment {
                inner: enrollment,
                authority,
            },
        ))
    }
    pub fn native_save_profile(&self) -> Result<super::NativeSaveProfile, Status> {
        self.save_authority()?;
        Ok(super::NativeSaveProfile {
            selected_object_limit: 3,
            trace_capacity: 131072,
            composed_history_capacity: 128,
            pending_frame_capacity: 16,
        })
    }
    pub fn take_save_preview_action(
        &self,
        chrome: &PeerContext,
    ) -> Result<super::SaveUiTicket, Status> {
        self.save_authority()?;
        Ok(super::SaveUiTicket {
            inner: self.take_preview_action(chrome)?,
        })
    }
    pub fn prepare_save_preview(
        &self,
        chrome: &PeerContext,
        ticket: &mut super::SaveUiTicket,
        pin: &super::SaveSelectionPin,
        now: u64,
    ) -> Result<Envelope, Status> {
        self.save_authority()?;
        self.prepare_store_preview(chrome, &mut ticket.inner, &pin.inner, now)
    }
    pub fn confirm_save_preview(
        &self,
        chrome: &PeerContext,
        ticket: &mut super::SaveUiTicket,
        now: u64,
    ) -> Result<super::PendingSaveInstance, Status> {
        self.save_authority()?;
        let inner = Arc::new(self.confirm_store_preview(chrome, &mut ticket.inner, now)?);
        Ok(super::PendingSaveInstance { inner })
    }
    pub fn take_save_launch(
        &self,
        chrome: &PeerContext,
        held: &mut Option<super::DesktopSaveLaunch>,
        now: u64,
    ) -> Result<(), super::NativeSaveDeliveryFailure> {
        use super::native_save::*;
        let mut observation = NativeSaveDeliveryObservation {
            stage: NativeSaveDeliveryStage::Authorize,
            status: Status::Denied,
            authorized_attempt: false,
            active: false,
            retired: false,
            pending_retained: false,
            launch_retained: held.is_some(),
            bootstrap_delivered: false,
            confirm_delivered: false,
        };
        let mut retained = None;
        let result = (|| {
            let authority = self.save_authority()?;
            let (pending, stamp) = {
                let state = self.state.lock().map_err(|_| Status::Internal)?;
                self.preview_chrome_locked(&state, chrome)?;
                if held.is_some() {
                    return Err(Status::NotReady);
                }
                let gate = authority.gate.lock().map_err(|_| Status::Internal)?;
                let p = gate.preview.as_ref().ok_or(Status::Denied)?;
                let pending = state
                    .save_pending
                    .values()
                    .rev()
                    .find(|pending| {
                        p.attachments
                            .get(&pending.life.id)
                            .is_some_and(|r| r.view.actor.session_id == chrome.session)
                    })
                    .ok_or(Status::NotReady)?
                    .clone();
                let row = p.row(&pending.life)?;
                retained = Some(pending.clone());
                observation.authorized_attempt = true;
                observation.pending_retained = true;
                observation.active = row.active;
                let stamp = super::StoreActivationStamp {
                    authority: authority.clone(),
                    life: pending.life.clone(),
                    version: row.version,
                };
                (pending, stamp)
            };
            observation.stage = NativeSaveDeliveryStage::Validate;
            let launch = self
                .take_store_launch(chrome, &pending, &stamp, now)
                .map_err(|e| e.status())?;
            observation.stage = NativeSaveDeliveryStage::BuildReply;
            let b = launch.bindings;
            let origin = b.artifact_origin;
            let save = DesktopSaveLaunch {
                confirm_reply: launch.confirm_reply,
                bindings: NativeSaveBindings {
                    bootstrap: b.bootstrap,
                    self_status: b.self_status,
                    focus_read: b.focus_read,
                    surface: b.surface,
                    artifact_origin: SaveOrigin {
                        authority: origin.authority,
                        life: origin.life,
                        context: origin.context,
                    },
                },
                stamp: SaveActivationStamp {
                    inner: launch.stamp,
                },
            };
            self.state
                .lock()
                .map_err(|_| Status::Internal)?
                .save_pending
                .remove(&pending.life.id);
            *held = Some(save);
            observation.stage = NativeSaveDeliveryStage::Expose;
            observation.bootstrap_delivered = true;
            observation.confirm_delivered = true;
            Ok(())
        })();
        result.map_err(|status| {
            observation.status = status;
            if let Some(p) = &retained {
                if let Ok(a) = self.native_authority() {
                    let gate = a.gate.lock().unwrap_or_else(|p| p.into_inner());
                    observation.retired = gate
                        .preview
                        .as_ref()
                        .and_then(|g| g.attachments.get(&p.life.id))
                        .is_some_and(|r| r.retired);
                }
            }
            NativeSaveDeliveryFailure {
                status,
                observation,
                _pending: retained,
                _launch: None,
            }
        })
    }
    pub fn dispatch_store_save_chrome(
        &self,
        chrome: &PeerContext,
        q: &Envelope,
        now: u64,
    ) -> Envelope {
        if let Err(status) = self.save_authority() {
            return reply(q, chrome.class, status);
        }
        self.dispatch_store_preview_chrome(chrome, q, now)
    }
    pub fn native_join_observation(
        &self,
        proof: &super::JoinedProducerProof,
    ) -> Result<super::NativeJoinObservation, Status> {
        self.save_authority()?
            .witness()
            .save_join_observation(proof)
    }
}

#[cfg(feature = "editor_native_save_v0_dev")]
impl FixtureController {
    pub fn pause_next_native_save(
        &self,
        session: u64,
        point: super::NativeSavePausePoint,
    ) -> Result<super::NativeSavePause, Status> {
        use super::native_save::*;
        let authority = self.host.save_authority()?;
        if point == NativeSavePausePoint::BeforeActivation {
            return Ok(NativeSavePause {
                authority,
                inner: SavePause::Preview(self.pause_next_store_preview(
                    session,
                    super::PreviewPausePoint::BeforeActivation,
                )?),
            });
        }
        let state = self.host.state.lock().map_err(|_| Status::Internal)?;
        if !state.sessions.contains_key(&session) {
            return Err(Status::Denied);
        }
        let mut gate = authority.gate.lock().map_err(|_| Status::Internal)?;
        let save = gate.save.as_mut().ok_or(Status::Unsupported)?;
        save.barriers.retain(|b| b.strong_count() > 0);
        if save.armed.is_some() {
            return Err(Status::NotReady);
        }
        if save.barriers.len() >= 32 {
            return Err(Status::Exhausted);
        }
        let b = SaveBarrier::new(session, point);
        save.barriers.push(Arc::downgrade(&b));
        save.armed = Some((session, point, b.clone()));
        Ok(NativeSavePause {
            authority: authority.clone(),
            inner: SavePause::Save(b),
        })
    }
    pub fn release_native_save(&self, pause: &super::NativeSavePause) -> Result<(), Status> {
        use super::native_save::*;
        if !same(&self.host.save_authority()?, &pause.authority) {
            return Err(Status::Denied);
        }
        match &pause.inner {
            SavePause::Preview(p) => self.release_store_preview(p),
            SavePause::Save(b) => b.release(),
        }
    }
    pub fn probe_inactive_save(
        &self,
        chrome: &PeerContext,
        pending: &super::PendingSaveInstance,
        pause: &super::NativeSavePause,
        now: u64,
    ) -> Result<super::InactiveSaveDesktopObservation, Status> {
        use super::native_save::*;
        if !same(&self.host.save_authority()?, &pause.authority) {
            return Err(Status::Denied);
        }
        let SavePause::Preview(p) = &pause.inner else {
            return Err(Status::NotReady);
        };
        let v = self.probe_inactive_preview(chrome, &pending.inner, p, now)?;
        Ok(InactiveSaveDesktopObservation {
            endpoint_requests: v.endpoint_requests,
            endpoint_replies: v.endpoint_replies,
            origin_status: v.origin_status,
            bootstrap_status: v.bootstrap_status,
            grants_lease_status: v.grants_lease_status,
        })
    }
    pub fn seed_native_save_counter(
        &self,
        counter: super::NativeSaveCounter,
    ) -> Result<(), Status> {
        use super::NativeSaveCounter::*;
        let _state = self.host.state.lock().map_err(|_| Status::Internal)?;
        let a = self.host.save_authority()?;
        let mut g = a.gate.lock().map_err(|_| Status::Internal)?;
        #[cfg(feature = "editor_adapter_assertions_v0_dev")]
        if matches!(counter, TextGeneration | ViewVersion) {
            // Same State -> Gate -> roster lock order as actual admission/join.
            // Retained completed Read owners are allowed; every actual producer
            // must already have been removed by its genuine join operation.
            if a.witness().producer_counts().held != 0 {
                return Err(Status::NotReady);
            }
            return owner_private_assertions::seed_actual_counter(&_state, &mut g, counter);
        }
        if counter == StatusVersion {
            g.preview.as_mut().ok_or(Status::Denied)?.next_status = u64::MAX;
        }
        let s = g.save.as_mut().ok_or(Status::Denied)?;
        match counter {
            Intent => s.next_intent = u64::MAX,
            Document => s.next_document = u64::MAX,
            TextGeneration => s.text_exhausted = true,
            ViewVersion => s.view_exhausted = true,
            FrameJoin => s.next_frame = u64::MAX,
            StatusVersion => {}
            SourceBinding => s.next_source = u64::MAX,
            OriginalRecord => s.next_original = u64::MAX,
        }
        Ok(())
    }
    pub fn fault_native_editor(&self, instance_id: u64, now: u64) -> Result<(), Status> {
        let a = self.host.save_authority()?;
        self.host.advance(now);
        let mut state = self.host.state.lock().map_err(|_| Status::Internal)?;
        if !state.instances.contains_key(&instance_id) {
            return Err(Status::Stale);
        }
        state.retire(instance_id, InstanceState::Faulted);
        let mut gate = a.gate.lock().map_err(|_| Status::Internal)?;
        let p = gate.preview.as_mut().ok_or(Status::Denied)?;
        let next = p.next_status.checked_add(1);
        let row = p.attachments.get_mut(&instance_id).ok_or(Status::Stale)?;
        row.retired = true;
        row.faulted = true;
        match next {
            Some(n) => {
                row.status_version = n;
                p.next_status = n;
                Ok(())
            }
            None => {
                row.status_exhausted = true;
                Err(Status::Exhausted)
            }
        }
    }
}

#[cfg(feature = "editor_native_save_v0_dev")]
impl HostDesktop {
    pub fn begin_save_read(
        &self,
        origin: &super::SaveOrigin,
        q: &Envelope,
        now: u64,
    ) -> Result<super::NativeSaveEntry, Status> {
        self.begin_save_entry((Some(origin), None, None, None), q, now, 1)
    }
    pub fn begin_save_allocate(
        &self,
        origin: &super::SaveOrigin,
        intent: &super::NativeSaveIntent,
        q: &Envelope,
        now: u64,
    ) -> Result<super::NativeSaveEntry, Status> {
        self.begin_save_entry((Some(origin), Some(intent), None, None), q, now, 3)
    }
    pub fn begin_save_commit(
        &self,
        origin: &super::SaveOrigin,
        source: &super::BoundNativeSource,
        q: &Envelope,
        now: u64,
    ) -> Result<super::NativeSaveEntry, Status> {
        self.begin_save_entry((Some(origin), None, Some(source), None), q, now, 5)
    }
    pub fn begin_save_status(
        &self,
        origin: &super::SaveOrigin,
        q: &Envelope,
        now: u64,
    ) -> Result<super::NativeSaveEntry, Status> {
        self.begin_save_entry((Some(origin), None, None, None), q, now, 7)
    }
    pub fn begin_save_recovery(
        &self,
        origin: &super::OriginalRecoveryOrigin,
        q: &Envelope,
        now: u64,
    ) -> Result<super::NativeSaveEntry, Status> {
        if !matches!(q.msg_type, 7 | 9) {
            return Err(Status::Unsupported);
        }
        self.begin_save_entry((None, None, None, Some(origin)), q, now, q.msg_type)
    }
    fn begin_save_entry(
        &self,
        origins: (
            Option<&super::SaveOrigin>,
            Option<&super::NativeSaveIntent>,
            Option<&super::BoundNativeSource>,
            Option<&super::OriginalRecoveryOrigin>,
        ),
        q: &Envelope,
        now: u64,
        kind: u32,
    ) -> Result<super::NativeSaveEntry, Status> {
        use super::native_save::*;
        let (origin, intent, source, recovery) = origins;
        let entered = Instant::now();
        let authority = self.save_authority()?;
        if q.protocol != 368 || q.msg_type != kind {
            return Err(Status::Unsupported);
        }
        encode_envelope_wire(q).and_then(|b| decode_envelope_wire(&b))?;
        if origin.is_some_and(|o| !same(&authority, &o.authority))
            || intent.is_some_and(|i| !same(&authority, &i.authority))
            || source.is_some_and(|s| !same(&authority, &s.authority))
            || recovery.is_some_and(|r| !same(&authority, &r.authority))
        {
            return Err(Status::Denied);
        }
        let mut state = self.state.lock().map_err(|_| Status::Internal)?;
        let now = self.clock.fetch_max(now, Ordering::SeqCst).max(now);
        state.expire(now);
        let mut gate = authority.gate.lock().map_err(|_| Status::Internal)?;
        let (attachment, actor, object_id, object_generation, epoch) = if let Some(origin) = origin
        {
            let p = gate.preview.as_ref().ok_or(Status::Denied)?;
            let row = p.row(&origin.life)?;
            let installation = row.installation.as_ref().ok_or(Status::NotReady)?;
            if origin.context != row.context || q.handle != installation.store_handle {
                return Err(Status::Denied);
            }
            p.admit_instance(origin.life.id)?;
            if !row.delivered {
                return Err(Status::NotReady);
            }
            (
                origin.life.clone(),
                row.view.actor.clone(),
                installation.object_id,
                installation.object_generation,
                installation.store_epoch,
            )
        } else {
            let r = recovery.ok_or(Status::Denied)?;
            if q.handle != r.life.view.request.handle {
                return Err(Status::Denied);
            }
            if q.msg_type == 9 {
                if r.life.view.request.msg_type != 3 || q64(q, 24) != q64(&r.life.view.request, 0) {
                    return Err(Status::Denied);
                }
            } else {
                let op = r
                    .life
                    .source
                    .as_ref()
                    .map(|s| s.binding.allocation.operation_id)
                    .or_else(|| {
                        r.life
                            .state
                            .lock()
                            .ok()
                            .and_then(|s| s.allocation.as_ref().map(|w| q64(w, 8)))
                    })
                    .ok_or(Status::Denied)?;
                if q64(q, 24) != op {
                    return Err(Status::Denied);
                }
            }
            (
                r.life.attachment.clone(),
                r.life.view.actor.clone(),
                r.life.view.object_id,
                r.life.view.object_generation,
                r.life.view.store_epoch,
            )
        };
        if q64(q, 8) != actor.session_id
            || q64(q, 16) != actor.session_generation
            || kind == 1
                && (q64(q, 24) != actor.instance_id || q64(q, 32) != actor.instance_generation)
        {
            return Err(Status::Denied);
        }
        let actual_intent = intent
            .map(|i| i.life.clone())
            .or_else(|| source.map(|s| s.life.intent.clone()));
        if let Some(i) = &actual_intent {
            if !Arc::ptr_eq(&i.attachment, &attachment)
                || i.snapshot.data.view.actor != actor
                || q64(q, 24) != i.snapshot.data.view.expected_revision
            {
                return Err(Status::Denied);
            }
        }
        if let Some(source) = source {
            if q64(q, 32) != source.life.binding.allocation.operation_id
                || q64(q, 40) != source.life.binding.source.source_object_id
                || q32(q, 48)
                    != source
                        .life
                        .binding
                        .source
                        .byte_len
                        .checked_add(64)
                        .ok_or(Status::Exhausted)?
            {
                return Err(Status::Denied);
            }
        }
        let deadline = actual_intent.as_ref().map(|i| i.deadline).unwrap_or(
            entered
                .checked_add(Duration::from_millis(1000))
                .ok_or(Status::Exhausted)?,
        );
        let started = actual_intent.as_ref().map(|i| i.entered).unwrap_or(entered);
        if Instant::now() >= deadline {
            return Err(Status::Timeout);
        }
        if authority.prune_save_calls(&mut gate)? >= 64 {
            return Err(Status::Exhausted);
        }
        let id = gate.next_origin.checked_add(1).ok_or(Status::Exhausted)?;
        let scope = super::native_authority::NativeAuthority::save_origin(
            id,
            attachment.clone(),
            object_id,
        );
        let [dispatcher_reservation, supervisor_reservation] = authority.reserve_save_pair(
            &scope,
            [
                super::ProducerKind::DesktopDispatch,
                super::ProducerKind::DesktopSupervisor,
            ],
        )?;
        let producers = [
            dispatcher_reservation.save_id(),
            supervisor_reservation.save_id(),
        ];
        let life = Arc::new(CallLife {
            origin: scope,
            attachment,
            intent: actual_intent.clone(),
            source: source.map(|s| s.life.clone()),
            recovery: recovery.map(|r| r.life.clone()),
            producers,
            view: NativeSaveCallView {
                actor: actor.clone(),
                object_id,
                object_generation,
                store_epoch: epoch,
                origin_id: id,
                intent_id: actual_intent.as_ref().map(|i| i.id),
                entered: started,
                deadline,
                request: *q,
            },
        });
        gate.next_origin = id;
        let save = gate.save.as_mut().ok_or(Status::Denied)?;
        save.calls.insert(
            id,
            CallRow {
                life: Arc::downgrade(&life),
                claimed: false,
                closed: false,
            },
        );
        let barrier = if save.armed.as_ref().is_some_and(|(sid, p, _)| {
            *sid == actor.session_id && *p == NativeSavePausePoint::BeforeEntryAdmission
        }) {
            save.armed.take().map(|(_, _, b)| b)
        } else {
            None
        };
        drop(gate);
        drop(state);
        let result = Arc::new(SaveEntryResult {
            state: Mutex::new(SaveEntryState {
                started: false,
                completed: false,
                consumed: false,
                outcome: None,
            }),
            changed: Condvar::new(),
        });
        let worker_result = result.clone();
        let worker_authority = authority.clone();
        let worker_life = life.clone();
        let worker_barrier = barrier.clone();
        let dispatcher = std::thread::Builder::new()
            .name("desktop-native-save".into())
            .spawn(move || {
                let _settle = SaveSettlement(worker_barrier.clone());
                let mut ready = worker_result
                    .state
                    .lock()
                    .unwrap_or_else(|p| p.into_inner());
                while !ready.started && !ready.completed {
                    ready = worker_result
                        .changed
                        .wait(ready)
                        .unwrap_or_else(|p| p.into_inner());
                }
                let started = ready.started;
                drop(ready);
                if !started {
                    return;
                }
                if let Some(b) = &worker_barrier {
                    if let Err(e) =
                        b.enter_wait(worker_life.attachment.id, worker_life.attachment.pin)
                    {
                        worker_result.publish(Err(e));
                        return;
                    }
                }
                let call = NativeSaveCall {
                    authority: worker_authority.clone(),
                    life: worker_life,
                };
                let outcome = worker_authority
                    .gate
                    .lock()
                    .map_err(|_| Status::Internal)
                    .and_then(|g| g.save_call(&call, call.life.recovery.is_some()).map(|_| ()));
                worker_result.publish(outcome);
            });
        let dispatcher = match dispatcher {
            Ok(handle) => dispatcher_reservation.install(handle),
            Err(_) => {
                dispatcher_reservation.cancel_unspawned()?;
                supervisor_reservation.cancel_unspawned()?;
                if let Some(b) = barrier {
                    b.settle();
                }
                return Err(Status::Internal);
            }
        };
        let timer_result = result.clone();
        let timer_authority = authority.clone();
        let timer_life = life.clone();
        let supervisor = std::thread::Builder::new()
            .name("desktop-save-deadline".into())
            .spawn(move || {
                let mut ready = timer_result.state.lock().unwrap_or_else(|p| p.into_inner());
                while !ready.started && !ready.completed {
                    ready = timer_result
                        .changed
                        .wait(ready)
                        .unwrap_or_else(|p| p.into_inner());
                }
                while !ready.completed {
                    let now = Instant::now();
                    if now >= deadline {
                        drop(ready);
                        let mut gate = timer_authority
                            .gate
                            .lock()
                            .unwrap_or_else(|p| p.into_inner());
                        if let Some(row) = gate.save.as_mut().and_then(|s| s.calls.get_mut(&id)) {
                            row.closed = true;
                        }
                        drop(gate);
                        timer_result.publish(Err(Status::Timeout));
                        return;
                    }
                    ready = timer_result
                        .changed
                        .wait_timeout(ready, deadline - now)
                        .unwrap_or_else(|p| p.into_inner())
                        .0;
                }
                drop(timer_life);
            });
        let supervisor = match supervisor {
            Ok(handle) => supervisor_reservation.install(handle),
            Err(_) => {
                supervisor_reservation.cancel_unspawned()?;
                result.publish(Err(Status::Internal));
                return Err(Status::Internal);
            }
        };
        let mut ready = result.state.lock().unwrap_or_else(|p| p.into_inner());
        if !ready.completed {
            ready.started = true;
        }
        result.changed.notify_all();
        drop(ready);
        Ok(NativeSaveEntry {
            authority,
            life,
            result,
            producers: [dispatcher, supervisor],
        })
    }
}

#[cfg(feature = "editor_native_save_v0_dev")]
impl HostDesktop {
    pub fn editor_observation(
        &self,
        editor: &super::NativeEditor,
        now: u64,
    ) -> Result<super::NativeEditorObservation, Status> {
        use super::native_save::*;
        let authority = self.save_authority()?;
        if !same(&authority, &editor.authority) {
            return Err(Status::Denied);
        }
        let mut state = self.state.lock().map_err(|_| Status::Internal)?;
        let now = self.clock.fetch_max(now, Ordering::SeqCst).max(now);
        state.expire(now);
        let gate = authority.gate.lock().map_err(|_| Status::Internal)?;
        let row = gate
            .save
            .as_ref()
            .ok_or(Status::Denied)?
            .documents
            .get(&editor.life.id)
            .ok_or(Status::Stale)?;
        if !Arc::ptr_eq(&row.life, &editor.life) {
            return Err(Status::Denied);
        }
        gate.preview
            .as_ref()
            .ok_or(Status::Denied)?
            .admit_instance(row.attachment.id)?;
        Ok(row.view.clone())
    }
    pub fn take_save_intent(
        &self,
        editor: &super::NativeEditor,
    ) -> Result<super::NativeSaveIntent, Status> {
        use super::native_save::*;
        let a = self.save_authority()?;
        if !same(&a, &editor.authority) {
            return Err(Status::Denied);
        }
        let mut gate = a.gate.lock().map_err(|_| Status::Internal)?;
        let row = gate
            .save
            .as_mut()
            .ok_or(Status::Denied)?
            .documents
            .get_mut(&editor.life.id)
            .ok_or(Status::Stale)?;
        if !Arc::ptr_eq(&row.life, &editor.life) {
            return Err(Status::Denied);
        }
        let life = row.queued_intent.take().ok_or(Status::NotReady)?;
        life.exposed.store(true, Ordering::Release);
        Ok(NativeSaveIntent {
            authority: a.clone(),
            life,
        })
    }
    pub fn step_native_editor(
        &self,
        editor: &super::NativeEditor,
        now: u64,
    ) -> Result<super::NativeEditorStep, Status> {
        use super::native_save::*;
        use kernel_api::generated::desktop_focus_v1 as focus;
        let a = self.save_authority()?;
        if !same(&a, &editor.authority) {
            return Err(Status::Denied);
        }
        let (status_endpoint, focus_endpoint, actor, ids, queued) = {
            let mut state = self.state.lock().map_err(|_| Status::Internal)?;
            let now = self.clock.fetch_max(now, Ordering::SeqCst).max(now);
            state.expire(now);
            let mut gate = a.gate.lock().map_err(|_| Status::Internal)?;
            let save = gate.save.as_ref().ok_or(Status::Denied)?;
            let doc = save.documents.get(&editor.life.id).ok_or(Status::Stale)?;
            if !Arc::ptr_eq(&doc.life, &editor.life) {
                return Err(Status::Denied);
            }
            let actor = doc.view.actor.clone();
            let p = gate.preview.as_ref().ok_or(Status::Denied)?;
            p.admit_instance(actor.instance_id)?;
            let row = p.row(&doc.attachment)?;
            let endpoints = row.endpoints;
            let queued = state
                .sessions
                .get(&actor.session_id)
                .ok_or(Status::Stale)?
                .keys
                .front()
                .copied();
            let doc = gate
                .save
                .as_mut()
                .ok_or(Status::Denied)?
                .documents
                .get_mut(&editor.life.id)
                .ok_or(Status::Stale)?;
            let last = doc.request_id.checked_add(2).ok_or(Status::Exhausted)?;
            let ids = [last - 1, last];
            doc.request_id = last;
            (
                self.endpoint_locked(&state, endpoints[0])?,
                self.endpoint_locked(&state, endpoints[1])?,
                actor,
                ids,
                queued,
            )
        };
        let q = editor::ObserveFocus {
            request_id: ids[0],
            session_id: actor.session_id,
            session_generation: actor.session_generation,
            instance_id: actor.instance_id,
            instance_generation: actor.instance_generation,
        }
        .encode(status_endpoint.handle)?;
        let reply = self.dispatch(&status_endpoint.peer, &q, now);
        let observed = editor::ObserveFocusReply::decode(&reply)?;
        if observed.status == Status::NotReady as u32 {
            return Ok(NativeEditorStep {
                consumed_sequence: None,
                text_changed: false,
                save_intent_available: false,
                render_required: false,
            });
        }
        if observed.status != 0 {
            return Err(status_from_u32(observed.status));
        }
        let q = focus::PollKeys {
            request_id: ids[1],
            session_id: actor.session_id,
            session_generation: actor.session_generation,
            instance_id: actor.instance_id,
            instance_generation: actor.instance_generation,
            focus_epoch: observed.focus_epoch,
        }
        .encode(focus_endpoint.handle)?;
        let reply = self.dispatch(&focus_endpoint.peer, &q, now);
        let key = focus::PollKeysReply::decode(&reply)?;
        if key.status == Status::NotReady as u32 {
            return Ok(NativeEditorStep {
                consumed_sequence: None,
                text_changed: false,
                save_intent_available: false,
                render_required: false,
            });
        }
        if key.status != 0 {
            return Err(status_from_u32(key.status));
        }
        let queued = queued
            .filter(|q| q.sequence == key.sequence)
            .ok_or(Status::Stale)?;
        let barrier = {
            let mut gate = a.gate.lock().map_err(|_| Status::Internal)?;
            let save = gate.save.as_mut().ok_or(Status::Denied)?;
            if save.armed.as_ref().is_some_and(|(sid, p, _)| {
                *sid == actor.session_id && *p == NativeSavePausePoint::BeforeKeyDelivery
            }) {
                save.armed.take().map(|(_, _, b)| b)
            } else {
                None
            }
        };
        let _settle = SaveSettlement(barrier.clone());
        if let Some(b) = &barrier {
            b.enter_wait(actor.instance_id, 0)?;
        }
        let mut state = self.state.lock().map_err(|_| Status::Internal)?;
        let now = self.clock.fetch_max(now, Ordering::SeqCst).max(now);
        state.expire(now);
        let mut gate = a.gate.lock().map_err(|_| Status::Internal)?;
        gate.preview
            .as_ref()
            .ok_or(Status::Denied)?
            .admit_instance(actor.instance_id)?;
        let session = state.sessions.get(&actor.session_id).ok_or(Status::Stale)?;
        if session.focused != Some(actor.instance_id) || session.focus != key.focus_epoch {
            return Err(Status::Stale);
        }
        let save = gate.save.as_mut().ok_or(Status::Denied)?;
        let doc = save.documents.get(&editor.life.id).ok_or(Status::Stale)?;
        if !Arc::ptr_eq(&doc.life, &editor.life)
            || key.sequence <= doc.key_sequence && doc.focus_epoch == key.focus_epoch
        {
            return Err(Status::Stale);
        }
        let before = doc.view.clone();
        let anchor = doc.selection_anchor;
        let column = doc.preferred_column;
        let saving = key.phase == 1 && key.usage == 22 && key.modifiers & 1 != 0;
        let mut intent = None;
        let unresolved = save.unresolved_document(before.document_id)?;
        if saving
            && !unresolved
            && !matches!(
                before.phase,
                NativeEditorPhase::AllocationPending
                    | NativeEditorPhase::AllocationUnknown
                    | NativeEditorPhase::Saving
                    | NativeEditorPhase::Unknown
            )
        {
            save.intents.retain(|_, w| w.strong_count() > 0);
            let live = save.live_intents(&a)?;
            if live.len() >= 48
                || live
                    .iter()
                    .filter(|i| i.snapshot.data.view.object_id == doc.object_id)
                    .count()
                    >= 16
            {
                return Err(Status::Exhausted);
            }
            let id = save.next_intent.checked_add(1).ok_or(Status::Exhausted)?;
            let current = save.current.get(&doc.object_id).ok_or(Status::Denied)?;
            let deadline = queued
                .entered
                .checked_add(Duration::from_millis(1000))
                .ok_or(Status::Exhausted)?;
            let snapshot = NativeDraftSnapshot {
                data: Arc::new(SnapshotData {
                    body: before.bytes.clone(),
                    view: DraftSnapshotView {
                        actor: actor.clone(),
                        object_id: current.object_id,
                        object_generation: current.object_generation,
                        original_store_epoch: current.store_epoch,
                        document_id: before.document_id,
                        text_generation: before.text_generation,
                        expected_revision: before.confirmed_revision,
                        expected_content_hash: before.confirmed_hash,
                        byte_len: before.bytes.len() as u32,
                        source_content_hash: hash(&before.bytes),
                    },
                }),
            };
            intent = Some(Arc::new(IntentLife {
                id,
                exposed: std::sync::atomic::AtomicBool::new(false),
                snapshot,
                entered: queued.entered,
                deadline,
                attachment: doc.attachment.clone(),
            }));
        }
        #[cfg(feature = "editor_adapter_assertions_v0_dev")]
        let owner_checkpoint = (key.phase == 1
            && key.usage == 4
            && key.modifiers == 0
            && (before.text_generation == u64::MAX || before.view_version == u64::MAX))
            .then(|| owner_private_assertions::OwnedCheckpoint::capture(doc));
        let mut text = super::client::NativeTextEditor::from_native(
            &before,
            anchor,
            column,
            state.config.font_rows,
        )?;
        if key.phase == 1 && !saving {
            #[cfg(not(feature = "editor_adapter_assertions_v0_dev"))]
            let accepted_edit = text.press(key.usage, key.modifiers)?;
            #[cfg(feature = "editor_adapter_assertions_v0_dev")]
            let accepted_edit = match text.press(key.usage, key.modifiers) {
                Ok(accepted) => accepted,
                Err(status) => {
                    if status == Status::Exhausted {
                        if let Some(checkpoint) = &owner_checkpoint {
                            checkpoint.assert_max_rejection(doc);
                        }
                    }
                    return Err(status);
                }
            };
            if !accepted_edit {
                let doc = save
                    .documents
                    .get_mut(&editor.life.id)
                    .ok_or(Status::Stale)?;
                doc.key_sequence = key.sequence;
                doc.focus_epoch = key.focus_epoch;
                return Ok(NativeEditorStep {
                    consumed_sequence: Some(key.sequence),
                    text_changed: false,
                    save_intent_available: doc.queued_intent.is_some(),
                    render_required: false,
                });
            }
        }
        let changed = text.draft.bytes != before.bytes;
        let visible = changed
            || text.draft.cursor as u32 != before.cursor
            || text.draft.selection.map(|(a, b)| (a as u32, b as u32)) != before.selection
            || text.draft.first_visible_line as u32 != before.first_visible_line;
        if changed && save.text_exhausted || visible && save.view_exhausted {
            return Err(Status::Exhausted);
        }
        #[cfg(not(feature = "editor_adapter_assertions_v0_dev"))]
        let version = if visible {
            before
                .view_version
                .checked_add(1)
                .ok_or(Status::Exhausted)?
        } else {
            before.view_version
        };
        #[cfg(feature = "editor_adapter_assertions_v0_dev")]
        let version = if visible {
            match before.view_version.checked_add(1) {
                Some(version) => version,
                None => {
                    if let Some(checkpoint) = &owner_checkpoint {
                        checkpoint.assert_max_rejection(doc);
                    }
                    return Err(Status::Exhausted);
                }
            }
        } else {
            before.view_version
        };
        if let Some(i) = &intent {
            save.next_intent = i.id;
            save.intents.insert(i.id, Arc::downgrade(i));
        }
        let doc = save
            .documents
            .get_mut(&editor.life.id)
            .ok_or(Status::Stale)?;
        doc.view.bytes = text.draft.bytes;
        doc.view.cursor = text.draft.cursor as u32;
        doc.view.selection = text.draft.selection.map(|(a, b)| (a as u32, b as u32));
        doc.view.first_visible_line = text.draft.first_visible_line as u32;
        doc.view.text_generation = text.draft_generation;
        doc.view.view_version = version;
        doc.selection_anchor = text.selection_anchor;
        doc.preferred_column = text.preferred_column;
        doc.key_sequence = key.sequence;
        doc.focus_epoch = key.focus_epoch;
        if visible {
            doc.frame = None;
            if changed || doc.view.phase == NativeEditorPhase::Saved {
                doc.view.phase = NativeEditorPhase::Unsaved;
            }
        }
        if let Some(i) = intent {
            doc.last_intent_id = Some(i.id);
            doc.queued_intent = Some(i);
            doc.view.phase = NativeEditorPhase::AllocationPending;
        }
        Ok(NativeEditorStep {
            consumed_sequence: Some(key.sequence),
            text_changed: changed,
            save_intent_available: doc.queued_intent.is_some(),
            render_required: visible,
        })
    }
}

#[cfg(feature = "editor_native_save_v0_dev")]
fn status_from_u32(v: u32) -> Status {
    match v {
        0 => Status::Ok,
        1 => Status::Denied,
        2 => Status::Invalid,
        3 => Status::Unsupported,
        4 => Status::Stale,
        5 => Status::Exhausted,
        6 => Status::NotReady,
        7 => Status::Conflict,
        8 => Status::Timeout,
        9 => Status::Disconnected,
        10 => Status::Unknown,
        _ => Status::Internal,
    }
}

#[cfg(feature = "editor_native_save_v0_dev")]
impl HostDesktop {
    pub fn render_native_editor(
        &self,
        editor: &super::NativeEditor,
        now: u64,
    ) -> Result<super::NativeFrameObservation, Status> {
        use super::native_save::*;
        use kernel_api::generated::{desktop_focus_v1 as focus, desktop_surface_v1 as surface};
        let a = self.save_authority()?;
        if !same(&a, &editor.authority) {
            return Err(Status::Denied);
        }
        let (
            view,
            endpoint,
            focus_endpoint,
            created,
            surface_id,
            surface_gen,
            index,
            sequence,
            requests,
            pixels,
            initial_focus,
        ) = {
            let mut state = self.state.lock().map_err(|_| Status::Internal)?;
            let now = self.clock.fetch_max(now, Ordering::SeqCst).max(now);
            state.expire(now);
            if state.frame_limit_hit() {
                return Err(Status::Exhausted);
            }
            let mut gate = a.gate.lock().map_err(|_| Status::Internal)?;
            let d = gate
                .save
                .as_ref()
                .ok_or(Status::Denied)?
                .documents
                .get(&editor.life.id)
                .ok_or(Status::Stale)?;
            if !Arc::ptr_eq(&d.life, &editor.life) {
                return Err(Status::Denied);
            }
            let actor = &d.view.actor;
            let p = gate.preview.as_ref().ok_or(Status::Denied)?;
            p.admit_instance(actor.instance_id)?;
            let r = p.row(&d.attachment)?;
            let endpoints = r.endpoints;
            let i = state
                .instances
                .get(&actor.instance_id)
                .ok_or(Status::Stale)?;
            let (created, sid, sg, sequence) = (
                i.surface.created,
                i.surface.id,
                i.surface.generation,
                i.surface.sequence.checked_add(1).ok_or(Status::Exhausted)?,
            );
            if i.surface.frozen.is_some() {
                return Err(Status::NotReady);
            }
            let focused = state.sessions[&actor.session_id].focused == Some(actor.instance_id);
            let initial_focus = state.sessions[&actor.session_id].focus;
            let chrome_endpoint =
                self.endpoint_locked(&state, state.sessions[&actor.session_id].chrome)?;
            let view = d.view.clone();
            let text = super::client::NativeTextEditor::from_native(
                &view,
                d.selection_anchor,
                d.preferred_column,
                state.config.font_rows,
            )?;
            let pixels = text;
            let s = gate.save.as_mut().ok_or(Status::Denied)?;
            let frame = s.next_frame.checked_add(1).ok_or(Status::Exhausted)?;
            let d = s.documents.get_mut(&editor.life.id).ok_or(Status::Stale)?;
            let last = d.request_id.checked_add(4).ok_or(Status::Exhausted)?;
            d.request_id = last;
            d.frame = None;
            if d.view.phase == NativeEditorPhase::Saved {
                d.view.phase = NativeEditorPhase::Unsaved;
            }
            s.next_frame = frame;
            state.pending_frames += 1;
            (
                view,
                self.endpoint_locked(&state, endpoints[2])?,
                chrome_endpoint,
                created,
                sid,
                sg,
                ((sequence - 1) % 2) as u32,
                sequence,
                [last - 3, last - 2, last - 1, last],
                pixels,
                if focused { Some(initial_focus) } else { None },
            )
        };
        let mut reservation = FrameReservation {
            state: self.state.clone(),
            finished: false,
        };
        let pixels = pixels.raster()?;
        let actor = &view.actor;
        if !created {
            let q = surface::Create {
                request_id: requests[0],
                session_id: actor.session_id,
                session_generation: actor.session_generation,
                instance_id: actor.instance_id,
                instance_generation: actor.instance_generation,
                width: 640,
                height: 480,
                format: 1,
                reserved: 0,
            }
            .encode(endpoint.handle)?;
            let r = surface::CreateReply::decode(&self.dispatch(&endpoint.peer, &q, now))?;
            if r.status != 0 {
                return Err(status_from_u32(r.status));
            }
            if r.surface_id != surface_id || r.surface_generation != surface_gen {
                return Err(Status::Denied);
            }
        }
        let focus_epoch = if let Some(epoch) = initial_focus {
            epoch
        } else {
            let q = focus::Assign {
                request_id: requests[1],
                session_id: actor.session_id,
                session_generation: actor.session_generation,
                instance_id: actor.instance_id,
                instance_generation: actor.instance_generation,
                surface_id,
                surface_generation: surface_gen,
            }
            .encode(focus_endpoint.handle)?;
            let r = focus::AssignReply::decode(&self.dispatch(&focus_endpoint.peer, &q, now))?;
            if r.status != 0 {
                return Err(status_from_u32(r.status));
            }
            r.focus_epoch
        };
        let q = surface::Acquire {
            request_id: requests[2],
            surface_id,
            surface_generation: surface_gen,
            buffer_index: index,
            reserved: 0,
        }
        .encode(endpoint.handle)?;
        let acquired = surface::AcquireReply::decode(&self.dispatch(&endpoint.peer, &q, now))?;
        if acquired.status != 0 {
            return Err(status_from_u32(acquired.status));
        }
        if acquired.mapping_generation == 0 || acquired.byte_len != FRAME_BYTES as u32 {
            return Err(Status::Invalid);
        }
        let h = Handle::unpack(acquired.buffer_shm);
        let descriptor = ObjectDescriptor {
            handle: h,
            kind: ObjectKind::SurfaceBuffer,
            byte_len: acquired.byte_len,
            object_generation: h.generation,
        };
        let lease = self.write_lease(
            &endpoint.peer,
            &descriptor,
            acquired.mapping_generation,
            now,
        )?;
        lease.copy_from(0, &pixels, now)?;
        let barrier = self
            .take_save_frame_barrier(actor.session_id, NativeSavePausePoint::BeforePresentFreeze)?;
        let _settle = SaveSettlement(barrier.clone());
        if let Some(b) = &barrier {
            b.enter_wait(actor.instance_id, 0)?;
        }
        {
            let state = self.state.lock().map_err(|_| Status::Internal)?;
            let gate = a.gate.lock().map_err(|_| Status::Internal)?;
            gate.preview
                .as_ref()
                .ok_or(Status::Denied)?
                .admit_instance(actor.instance_id)?;
            if state.sessions[&actor.session_id].focus != focus_epoch {
                return Err(Status::Stale);
            }
        }
        let q = surface::Present {
            request_id: requests[3],
            surface_id,
            surface_generation: surface_gen,
            mapping_generation: acquired.mapping_generation,
            sequence,
            buffer_index: index,
            reserved: 0,
        }
        .encode(endpoint.handle)?;
        let r = surface::PresentReply::decode(&self.dispatch(&endpoint.peer, &q, now))?;
        if r.status != 0 {
            return Err(status_from_u32(r.status));
        }
        if r.sequence != sequence {
            return Err(Status::Invalid);
        }
        let barrier = self
            .take_save_frame_barrier(actor.session_id, NativeSavePausePoint::BeforeFramePublish)?;
        let _settle = SaveSettlement(barrier.clone());
        if let Some(b) = &barrier {
            b.enter_wait(actor.instance_id, 0)?;
        }
        let mut state = self.state.lock().map_err(|_| Status::Internal)?;
        let now = self.clock.fetch_max(now, Ordering::SeqCst).max(now);
        state.expire(now);
        let mut gate = a.gate.lock().map_err(|_| Status::Internal)?;
        gate.preview
            .as_ref()
            .ok_or(Status::Denied)?
            .admit_instance(actor.instance_id)?;
        let session = state.sessions.get(&actor.session_id).ok_or(Status::Stale)?;
        let frozen = state
            .instances
            .get(&actor.instance_id)
            .ok_or(Status::Stale)?
            .surface
            .frozen
            .as_ref()
            .ok_or(Status::Stale)?;
        if session.focus != focus_epoch
            || session.focused != Some(actor.instance_id)
            || frozen.1 != sequence
            || frozen.2.as_slice() != pixels.as_slice()
        {
            return Err(Status::Stale);
        }
        let save = gate.save.as_mut().ok_or(Status::Denied)?;
        let doc = save
            .documents
            .get_mut(&editor.life.id)
            .ok_or(Status::Stale)?;
        if !Arc::ptr_eq(&doc.life, &editor.life)
            || doc.view.actor != view.actor
            || doc.view.text_generation != view.text_generation
            || doc.view.view_version != view.view_version
            || doc.view.bytes != view.bytes
        {
            return Err(Status::Stale);
        }
        let observation = NativeFrameObservation {
            actor: view.actor,
            document_id: view.document_id,
            text_generation: view.text_generation,
            view_version: view.view_version,
            surface_id,
            surface_generation: surface_gen,
            mapping_generation: acquired.mapping_generation,
            focus_epoch,
            sequence,
            app_crop_sha256: hash(&pixels),
        };
        doc.frame = Some(observation.clone());
        state.pending_frames -= 1;
        reservation.finished = true;
        Ok(observation)
    }
    fn take_save_frame_barrier(
        &self,
        session: u64,
        point: super::NativeSavePausePoint,
    ) -> Result<Option<Arc<super::native_save::SaveBarrier>>, Status> {
        let a = self.save_authority()?;
        let mut g = a.gate.lock().map_err(|_| Status::Internal)?;
        let s = g.save.as_mut().ok_or(Status::Denied)?;
        if s.armed
            .as_ref()
            .is_some_and(|(sid, p, _)| *sid == session && *p == point)
        {
            Ok(s.armed.take().map(|(_, _, b)| b))
        } else {
            Ok(None)
        }
    }
}

#[cfg(feature = "editor_native_save_v0_dev")]
impl HostDesktop {
    pub fn compose_native_save(
        &self,
        compositor: &Endpoint,
        now: u64,
    ) -> Result<ComposedFrame, Status> {
        use super::native_save::*;
        if !Arc::ptr_eq(&self.state, &compositor.peer.registry)
            || compositor.peer.class != EndpointClass::Compositor
            || compositor.handle.pack() != compositor.peer.endpoint
        {
            return Err(Status::Denied);
        }
        let a = self.save_authority()?;
        let (sid, id, focus, sequence, source, font, observation, setup) = {
            let mut state = self.state.lock().map_err(|_| Status::Internal)?;
            let now = self.clock.fetch_max(now, Ordering::SeqCst).max(now);
            state.expire(now);
            state.auth(&compositor.peer)?;
            let sid = compositor.peer.session;
            if state.sessions[&sid].compositor != compositor.peer.endpoint {
                return Err(Status::Denied);
            }
            if state.frame_limit_hit() {
                return Err(Status::Exhausted);
            }
            let mut gate = a.gate.lock().map_err(|_| Status::Internal)?;
            let p = gate.preview.as_mut().ok_or(Status::Denied)?;
            let id = p
                .attachments
                .iter()
                .rev()
                .find(|(_, r)| r.view.actor.session_id == sid && r.delivered)
                .map(|(id, _)| *id)
                .unwrap_or(0);
            let (focus, sequence, source, observation, setup) = if id != 0 {
                let row = p.attachments.get(&id).ok_or(Status::Stale)?;
                if row.status_exhausted {
                    return Err(Status::Exhausted);
                }
                let terminal = row.retired || row.faulted;
                let (focus, sequence, source) = if terminal {
                    (0, 0, None)
                } else {
                    p.admit_instance(id)?;
                    let i = state.instances.get(&id).ok_or(Status::Stale)?;
                    let (_, sequence, source) =
                        i.surface.frozen.as_ref().ok_or(Status::NotReady)?;
                    (state.sessions[&sid].focus, *sequence, Some(source.clone()))
                };
                let next = p.next_status.checked_add(1).ok_or(Status::Exhausted)?;
                p.next_status = next;
                p.attachments
                    .get_mut(&id)
                    .ok_or(Status::Stale)?
                    .status_version = next;
                let observation = gate.save.as_ref().ok_or(Status::Denied)?.chrome(
                    gate.preview.as_ref().ok_or(Status::Denied)?,
                    id,
                    focus,
                    sequence,
                    next,
                )?;
                (focus, sequence, source, Some(observation), None)
            } else {
                let pin = state.preview_pins.get(&sid).ok_or(Status::NotReady)?;
                let row = p.pins.get(&pin.id).ok_or(Status::Stale)?;
                let preview = state.sessions[&sid]
                    .preview
                    .as_ref()
                    .ok_or(Status::NotReady)?;
                if row.retired
                    || preview.invalid
                    || p.now >= row.view.expires_at_ms
                    || Instant::now() >= row.view.deadline
                {
                    return Err(Status::Stale);
                }
                (
                    0,
                    0,
                    None,
                    None,
                    Some((
                        pin.clone(),
                        row.view.clone(),
                        preview.plan,
                        preview.revision,
                    )),
                )
            };
            state.pending_frames += 1;
            (
                sid,
                id,
                focus,
                sequence,
                source,
                state.config.font_rows,
                observation,
                setup,
            )
        };
        let mut reservation = FrameReservation {
            state: self.state.clone(),
            finished: false,
        };
        let mut pixels = vec![224; 640 * 568 * 4];
        for p in pixels.chunks_exact_mut(4) {
            p[3] = 255;
        }
        pixels[48 * 2560..528 * 2560].fill(255);
        let unavailable = observation
            .as_ref()
            .is_some_and(|o| o.label == NativeEditorPhase::Unavailable);
        if !unavailable {
            if let Some(source) = &source {
                pixels[48 * 2560..528 * 2560].copy_from_slice(source);
            }
        }
        for y in 8..24 {
            for x in 8..24 {
                let n = (y * 640 + x) * 4;
                pixels[n..n + 4].copy_from_slice(if x == 8 || x == 23 || y == 8 || y == 23 {
                    &[0, 0, 0, 255]
                } else {
                    &[0, 192, 0, 255]
                });
            }
        }
        draw_label(&mut pixels, &font, b"HOST STORE", 40, 8);
        let (label, banner) = if let Some(o) = &observation {
            let allocation = o
                .pending_originals
                .iter()
                .any(|o| o.phase == NativePendingOriginalPhase::AllocationUnknown);
            let commit = o
                .pending_originals
                .iter()
                .any(|o| o.phase == NativePendingOriginalPhase::CommitUnknown);
            let banner: &[u8] = match (commit, allocation) {
                (true, true) => b"SAVE UNKNOWN | ALLOCATION UNKNOWN",
                (true, false) => b"SAVE UNKNOWN",
                (false, true) => b"ALLOCATION UNKNOWN",
                _ => b"",
            };
            let label: &[u8] = match o.label {
                NativeEditorPhase::Loaded => b"LOADED",
                NativeEditorPhase::Unsaved => b"UNSAVED",
                NativeEditorPhase::AllocationPending | NativeEditorPhase::Saving => b"SAVING",
                NativeEditorPhase::AllocationUnknown | NativeEditorPhase::Unknown => b"UNSAVED",
                NativeEditorPhase::Saved => b"SAVED",
                NativeEditorPhase::Conflict => b"CONFLICT",
                NativeEditorPhase::Unavailable => b"UNAVAILABLE",
            };
            (label, banner)
        } else {
            (b"LOADED".as_slice(), b"".as_slice())
        };
        draw_label(&mut pixels, &font, banner, 8, 24);
        draw_label(&mut pixels, &font, label, 8, 536);
        {
            let mut state = self.state.lock().map_err(|_| Status::Internal)?;
            state.expire(self.clock.load(Ordering::SeqCst));
            state.auth(&compositor.peer)?;
            let mut gate = a.gate.lock().map_err(|_| Status::Internal)?;
            if let Some(captured) = observation {
                let actual = gate.save.as_ref().ok_or(Status::Denied)?.chrome(
                    gate.preview.as_ref().ok_or(Status::Denied)?,
                    id,
                    focus,
                    sequence,
                    captured.status_version,
                )?;
                if actual != captured {
                    return Err(Status::Stale);
                }
                let row = gate
                    .preview
                    .as_ref()
                    .ok_or(Status::Denied)?
                    .attachments
                    .get(&id)
                    .ok_or(Status::Stale)?;
                if row.status_version != captured.status_version || row.status_exhausted {
                    return Err(Status::Stale);
                }
                if !unavailable {
                    gate.preview
                        .as_ref()
                        .ok_or(Status::Denied)?
                        .admit_instance(id)?;
                    let i = state.instances.get(&id).ok_or(Status::Stale)?;
                    if state.sessions[&sid].focus != focus
                        || state.sessions[&sid].focused != Some(id)
                        || i.surface.frozen.as_ref().map(|(_, seq, _)| *seq) != Some(sequence)
                    {
                        return Err(Status::Stale);
                    }
                    let q = kernel_api::generated::desktop_surface_v1::Consume {
                        request_id: sequence,
                        surface_id: i.surface.id,
                        surface_generation: i.surface.generation,
                        sequence,
                    }
                    .encode(compositor.handle)?;
                    if state.exchanges.len() >= state.trace_limit() {
                        return Err(Status::Exhausted);
                    }
                    state.consume_frozen(id, sequence)?;
                    let mut r = reply(&q, EndpointClass::Compositor, Status::Ok);
                    put64(&mut r.payload, 8, sequence);
                    state.exchanges.push((q, r));
                }
                if let Some(docid) = captured.document_id {
                    if let Some(d) = gate
                        .save
                        .as_mut()
                        .ok_or(Status::Denied)?
                        .documents
                        .get_mut(&docid)
                    {
                        d.view.phase = captured.label.clone();
                    }
                }
                state.save_chrome.insert(sid, captured);
            } else {
                let (pin, view, plan, revision) = setup.as_ref().ok_or(Status::Internal)?;
                let current_pin = state.preview_pins.get(&sid).ok_or(Status::Stale)?;
                let row = gate
                    .preview
                    .as_ref()
                    .ok_or(Status::Denied)?
                    .pins
                    .get(&pin.id)
                    .ok_or(Status::Stale)?;
                let preview = state.sessions[&sid].preview.as_ref().ok_or(Status::Stale)?;
                if !Arc::ptr_eq(pin, current_pin)
                    || row.retired
                    || row.view.entered != view.entered
                    || row.view.deadline != view.deadline
                    || preview.invalid
                    || preview.plan != *plan
                    || preview.revision != *revision
                    || Instant::now() >= row.view.deadline
                {
                    return Err(Status::Stale);
                }
                state.save_chrome.remove(&sid);
            }
            state.pending_frames -= 1;
            state.composed_frames += 1;
            reservation.finished = true;
        }
        Ok(ComposedFrame {
            session_id: sid,
            instance_id: id,
            focus_epoch: focus,
            sequence,
            bgra: pixels,
        })
    }
}
#[cfg(feature = "editor_native_save_v0_dev")]
impl FixtureController {
    pub fn save_chrome_observation(
        &self,
        compositor: &Endpoint,
    ) -> Result<super::NativeSaveChromeObservation, Status> {
        if !Arc::ptr_eq(&self.host.state, &compositor.peer.registry)
            || compositor.peer.class != EndpointClass::Compositor
            || compositor.handle.pack() != compositor.peer.endpoint
        {
            return Err(Status::Denied);
        }
        let state = self.host.state.lock().map_err(|_| Status::Internal)?;
        state.auth(&compositor.peer)?;
        let stored = state
            .save_chrome
            .get(&compositor.peer.session)
            .ok_or(Status::NotReady)?;
        let a = self.host.save_authority()?;
        let gate = a.gate.lock().map_err(|_| Status::Internal)?;
        if gate.preview.as_ref().ok_or(Status::Denied)?.next_status == u64::MAX {
            return Err(Status::Exhausted);
        }
        let row = gate
            .preview
            .as_ref()
            .ok_or(Status::Denied)?
            .attachments
            .get(&stored.current_frame_actor.instance_id)
            .ok_or(Status::Stale)?;
        if row.status_exhausted
            || row.version != stored.attachment_version
            || row.status_version != stored.status_version
        {
            return Err(Status::Stale);
        }
        if row.retired && stored.label != super::NativeEditorPhase::Unavailable {
            return Err(Status::Stale);
        }
        let actual = gate.save.as_ref().ok_or(Status::Denied)?.chrome(
            gate.preview.as_ref().ok_or(Status::Denied)?,
            row.view.actor.instance_id,
            stored.focus_epoch,
            stored.frame_sequence,
            stored.status_version,
        )?;
        if actual != *stored {
            return Err(Status::Stale);
        }
        Ok(stored.clone())
    }
    pub fn save_chrome_record(&self, compositor: &Endpoint) -> Result<[u8; 248], Status> {
        self.save_chrome_observation(compositor)?
            .record248
            .ok_or(Status::NotReady)
    }
    pub fn save_original_chrome_observation(
        &self,
        compositor: &Endpoint,
        original: &super::OriginalRecoveryOrigin,
    ) -> Result<super::NativeSaveChromeObservation, Status> {
        use super::native_save::*;
        let mut out = self.save_chrome_observation(compositor)?;
        let a = self.host.save_authority()?;
        if !same(&a, &original.authority)
            || original.life.view.actor.session_id != compositor.peer.session
        {
            return Err(Status::Denied);
        }
        let gate = a.gate.lock().map_err(|_| Status::Internal)?;
        let save = gate.save.as_ref().ok_or(Status::Denied)?;
        if save
            .originals
            .get(&original.life.view.origin_id)
            .is_none_or(|w| !std::sync::Weak::ptr_eq(w, &Arc::downgrade(&original.life)))
        {
            return Err(Status::Denied);
        }
        // The current frame identity is retained; only the requested genuine
        // original outcome is substituted, never a caller-selected operation.
        let state = super::native_save::lock(&original.life.state)?;
        out.original_actor = Some(original.life.view.actor.clone());
        if let Some(source) = &original.life.source {
            use artifact_store_schema::editor_native_save::*;
            let b = &source.binding;
            let current = save
                .current
                .get(&original.life.view.object_id)
                .ok_or(Status::Stale)?;
            let record = EditorNativeSaveChromeV0 {
                schema_version: 2,
                total_len: 248,
                state: if let Some(r) = &state.receipt {
                    if r.outcome
                        == artifact_store_schema::editor_save::EditorReceiptOutcomeV0::Committed
                    {
                        EditorNativeSaveChromeStateV0::Committed
                    } else {
                        EditorNativeSaveChromeStateV0::DefinitiveNoncommit
                    }
                } else {
                    EditorNativeSaveChromeStateV0::Unknown
                },
                reserved: 0,
                actor: b.allocation.actor.clone(),
                selected_object_id: b.allocation.selected_object_id,
                selected_object_generation: b.allocation.selected_generation,
                current_store_epoch: current.store_epoch,
                original_backend_epoch: source.epoch,
                operation_id: b.allocation.operation_id,
                source_handle: b.source.source_object_id,
                source_generation: b.source.source_generation,
                expected_revision: b.allocation.expected_revision,
                result_revision: state
                    .receipt
                    .as_ref()
                    .map(|r| r.result_revision)
                    .unwrap_or(0),
                attachment_version: out.attachment_version,
                status_version: out.status_version,
                source_content_hash: b.source.content_hash,
                expected_content_hash: b.allocation.expected_content_hash,
                receipt_sha256: state
                    .receipt_bytes
                    .as_ref()
                    .map(|b| hash(b))
                    .unwrap_or([0; 32]),
                source_body_len: b.source.byte_len,
                tail_reserved: 0,
            };
            out.record248 = Some(record.encode_le().map_err(|_| Status::Invalid)?);
        } else {
            out.record248 = None;
        }
        Ok(out)
    }
}
