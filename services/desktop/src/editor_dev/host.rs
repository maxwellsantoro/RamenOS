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
        if self.objects.len() >= 32 {
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
    fn retire(&mut self, id: u64, state: InstanceState) {
        #[cfg(feature = "editor_native_read_v0_dev")]
        if let Some(native) = &self.native {
            native.retire(id);
        }
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
                #[cfg(feature = "editor_native_read_v0_dev")]
                native: None,
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
        let life = Arc::new(RequestLife {
            ticket: Arc::new(()),
            deadline: Instant::now() + Duration::from_millis(1000),
            closed: std::sync::atomic::AtomicBool::new(false),
        });
        let checked = encode_envelope_wire(q).and_then(|b| decode_envelope_wire(&b));
        let mut response = reply(q, peer.class, Status::NotReady);
        let failure = match checked {
            Err(e) => Some(e),
            Ok(_) if q.msg_type.is_multiple_of(2) || q.msg_type == 17 => Some(Status::Unsupported),
            Ok(_) => self.authorize(peer, q, now).err(),
        };
        if let Some(error) = failure {
            response = reply(q, peer.class, error);
        }
        let slot = {
            let mut s = self.state.lock().unwrap();
            if s.exchanges.len() >= 256 {
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
        if s.objects.len() >= 32 {
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
        if s.instances.len() >= 16 || s.endpoints.len() + 4 > 64 || s.objects.len() + 5 > 32 {
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
            if session.route != Route::App {
                session.pending_selector = None;
                session.reset_pending = false;
                s.shift_focus(sid, Route::Launcher, None)?;
            }
            return Ok(reply(q, peer.class, Status::Ok));
        }
        if !matches!(usage,4..=49|51..=56|74|77|79..=82|224|225) {
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
                sequence: q64(q, 24),
                usage: 0,
                phase: 3,
                modifiers: 0,
            });
            if let Some(p) = &mut session.preview {
                p.invalid = true;
                p.approval = false;
            }
            if session.route != Route::App {
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
                action = if usage == 20 {
                    Action::Close(id, generation)
                } else {
                    Action::Restart(id, generation)
                };
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
                if s.objects.len() + 2 > 32 {
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
                let mut data = data.lock().unwrap();
                if !data.writable || data.mapping != q64(q, 24) {
                    return Err(Status::Stale);
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
                let i = s.instances.get_mut(&id).unwrap();
                if i.surface.frozen.as_ref().map(|(_, seq, _)| *seq) != Some(q64(q, 24)) {
                    return Err(Status::Stale);
                }
                i.surface.frozen = None;
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
        if old.is_none() && s.objects.len() >= 32 {
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
        if old == 0 && s.objects.len() >= 32 {
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
        self.advance(now);
        if !Arc::ptr_eq(&self.state, &peer.registry) {
            return Err(Status::Denied);
        }
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
            if s.composed_frames + s.pending_frames >= 16 {
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
            volatile_backend: true,
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
