//! Default-off, trusted in-process Store-owned transaction fixture.
//! This is host CAS evidence, not a deployed editor bridge or containment boundary.
mod codec;
#[cfg(feature = "editor_native_preview_v0_dev")]
mod native_preview;
#[cfg(feature = "editor_native_read_v0_dev")]
mod native_read;
mod storage;
mod types;
use artifact_store_schema::editor_save::*;
pub use codec::{ArtifactMessage, decode_envelope_wire, encode_envelope_wire};
use kernel_api::{
    cap::{Handle, HandleKind},
    generated::desktop_artifact_v1 as a,
    ipc::Envelope,
};
#[cfg(feature = "editor_native_preview_v0_dev")]
pub use native_preview::*;
#[cfg(feature = "editor_native_read_v0_dev")]
pub use native_read::*;
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::DirBuilderExt,
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
pub use types::*;

pub struct StoreFixture;
#[derive(Clone)]
pub struct StoreHost {
    core: Arc<Core>,
}
pub struct FixtureController {
    core: Arc<Core>,
}
#[derive(Clone)]
pub struct EditorPeer {
    core: Arc<Core>,
    grant: Arc<Grant>,
}
pub struct EditorEndpoint {
    pub peer: EditorPeer,
    pub handle: Handle,
}
#[derive(Clone)]
pub struct ReadLease {
    peer: EditorPeer,
    data: Arc<Data>,
}
#[derive(Clone)]
pub struct WriteLease {
    peer: EditorPeer,
    data: Arc<Data>,
}
pub struct PauseToken {
    core: Arc<Core>,
    barrier: Arc<Barrier>,
}
pub struct QuiescedStoreOwner {
    core: Arc<Core>,
    snapshot: Box<FenceSnapshot>,
}
pub struct ReopenFailure {
    pub error: FixtureError,
    pub owner: QuiescedStoreOwner,
}
impl QuiescedStoreOwner {
    pub fn fence_snapshot(&self) -> FenceSnapshot {
        self.snapshot.as_ref().clone()
    }
}
struct Grant {
    #[cfg(feature = "editor_native_read_v0_dev")]
    native_read: Option<Arc<native_read::NativeReadGrant>>,
    handle: Handle,
    actor: EditorActorV0,
    object: u64,
    rights: u32,
    class: EndpointClass,
    recovery_op: Option<u64>,
    epoch: u64,
    expires: u64,
    alive: AtomicBool,
    invocation: AtomicBool,
    drop_reply: AtomicBool,
}
struct Data {
    handle: Handle,
    actor: EditorActorV0,
    object: u64,
    operation: Option<u64>,
    revision: u64,
    kind: SharedObjectKind,
    alive: AtomicBool,
    state: Mutex<DataState>,
}
struct DataState {
    bytes: Vec<u8>,
    writable: bool,
    frozen: bool,
}
struct Registry {
    next_identity: u64,
    grants: BTreeMap<u64, Arc<Grant>>,
    data: BTreeMap<u64, Arc<Data>>,
    sessions: BTreeMap<(u64, u64), u64>,
    instances: BTreeMap<(u64, u64, u64, u64), u64>,
}
struct Core {
    native_only: bool,
    #[cfg(feature = "editor_native_read_v0_dev")]
    native_read: Mutex<Option<Arc<native_read::NativeCore>>>,
    #[cfg(feature = "editor_native_preview_v0_dev")]
    native_preview: Mutex<Option<Arc<native_preview::PreviewCore>>>,
    root: PathBuf,
    profile: FixtureProfile,
    profile_hash: Hash32,
    namespace: u64,
    clock: AtomicU64,
    closed: AtomicBool,
    setup_active: AtomicBool,
    fence_issued: AtomicBool,
    quiescing: AtomicBool,
    registry: Mutex<Registry>,
    objects: BTreeMap<u64, Arc<Object>>,
    diagnostics: Mutex<Diagnostics>,
    owned: Mutex<Owned>,
    owned_changed: Condvar,
    barriers: Mutex<Vec<Arc<Barrier>>>,
}
struct Diagnostics {
    exchanges: Vec<Exchange>,
    io: Vec<IoEvent>,
    io_sequence: u64,
    reserved: usize,
    exchange_reservations: usize,
    exhausted: bool,
}
struct Owned {
    next: u64,
    handles: BTreeMap<u64, Producer>,
}
struct Producer {
    kind: ProducerKind,
    object: u64,
    request: u64,
    operation: u64,
    handle: Option<JoinHandle<()>>,
    joining: bool,
    terminal_join: Option<ProducerJoinOutcome>,
    thread_id: String,
    barrier: Option<Arc<Barrier>>,
}
struct Object {
    id: u64,
    root: PathBuf,
    state: Mutex<ObjectState>,
}
struct ObjectState {
    #[cfg(feature = "editor_native_preview_v0_dev")]
    preview_faulted: bool,
    journal: EditorSelectionJournalV0,
    durable: EditorSelectionJournalV0,
    prepared: Option<EditorSelectionJournalV0>,
    synced: Option<EditorSelectionJournalV0>,
    digest: Hash32,
    bytes: Vec<u8>,
    owner_hash: Hash32,
    next_op: u64,
    next_writer: u64,
    force_op_range: bool,
    force_writer_range: bool,
    admission_epoch: u64,
    allocation_fenced: bool,
    mutation_fenced: bool,
    io_worker: Option<u64>,
    runtime: BTreeMap<u64, RuntimeOperation>,
    dispatches: u32,
    permits: u32,
    transitions: u32,
    quarantine: bool,
    fault: Option<IoPoint>,
    reopen: Option<ReopenWitness>,
}
struct RuntimeOperation {
    original: Option<Arc<Request>>,
    source: Option<Arc<Data>>,
    permitted: Option<EditorCommitPermitV0>,
    dispatched: bool,
}
// One-shot, private genesis authority. Only create_inner builds this after full
// trusted configuration/header/signature validation. It is consumed by initial
// publication and has no Clone, serde or caller-facing constructor.
struct ProvisioningBinding {
    owner: u64,
    object: u64,
    generation: u64,
    initial_revision: u64,
    blob_hash: Hash32,
    byte_len: u32,
    manifest_hash: Hash32,
    profile_hash: Hash32,
}
impl ProvisioningBinding {
    fn check(
        &self,
        selected: &SelectedArtifactV0,
        profile_hash: Hash32,
        bytes: &[u8],
        manifest: &artifact_store_schema::Manifest,
    ) -> Result<(), StoreStatus> {
        if self.owner != selected.owner_id
            || self.object != selected.selected_object_id
            || self.generation != selected.selected_generation
            || self.initial_revision != selected.revision
            || self.blob_hash != selected.content_hash
            || self.byte_len != selected.byte_len
            || self.manifest_hash != selected.manifest_hash
            || self.profile_hash != profile_hash
            || storage::hash(bytes) != self.blob_hash
            || bytes.len() != self.byte_len as usize
            || storage::hash(&serde_json::to_vec(manifest).map_err(|_| StoreStatus::Internal)?)
                != self.manifest_hash
        {
            return Err(StoreStatus::Denied);
        }
        schema(EditorTextHeaderV0::try_new(
            self.object,
            self.initial_revision,
            bytes,
        ))?;
        Ok(())
    }
}
// A pure journal permit is evidence only. This non-Clone private value binds
// the actual original ticket, frozen source and exclusively held IO reservation.
struct StoreCommitPermit {
    evidence: EditorCommitPermitV0,
    ticket: Arc<Request>,
    source: Arc<Data>,
    object: Arc<Object>,
    worker: u64,
    bytes: Vec<u8>,
}
impl StoreCommitPermit {
    fn check_reservation(&self, context: &Context) -> Result<(), StoreStatus> {
        let s = lock(&self.object.state);
        if !Arc::ptr_eq(&self.object, &context.object)
            || s.io_worker != Some(self.worker)
            || context
                .request
                .as_ref()
                .is_none_or(|r| !Arc::ptr_eq(r, &self.ticket))
            || self.source.handle.pack() != self.evidence.binding.source.source_object_id
            || !lock(&self.source.state).frozen
        {
            return Err(StoreStatus::Internal);
        }
        Ok(())
    }
}
struct Request {
    producer: u64,
    grant: Arc<Grant>,
    wire: Envelope,
    deadline: Instant,
    submitted_source: Option<Arc<Data>>,
    closed: AtomicBool,
}
struct Barrier {
    object: u64,
    endpoint: u64,
    point: PausePoint,
    state: Mutex<BarrierState>,
    changed: Condvar,
}
struct BarrierState {
    snapshot: BarrierSnapshot,
    stop: bool,
}
struct Context {
    core: Arc<Core>,
    object: Arc<Object>,
    request: Option<Arc<Request>>,
    operation: u64,
    writer: u64,
    selection: bool,
}
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}
fn fixture<T>(r: Result<T, StoreStatus>) -> Result<T, FixtureError> {
    r.map_err(FixtureError::new)
}
fn schema<T>(r: Result<T, EditorSaveError>) -> Result<T, StoreStatus> {
    r.map_err(|_| StoreStatus::Invalid)
}
fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}
fn op_of(env: &Envelope) -> u64 {
    match env.msg_type {
        5 => u64_at(&env.payload, 32),
        7 => u64_at(&env.payload, 24),
        _ => 0,
    }
}
fn instance_key(a: &EditorActorV0) -> (u64, u64, u64, u64) {
    (
        a.session_id,
        a.session_generation,
        a.instance_id,
        a.instance_generation,
    )
}
fn empty_reply(request: &Envelope, status: StoreStatus) -> Envelope {
    let mut reply = Envelope::empty(368, request.msg_type + 1);
    reply.handle = request.handle;
    reply.payload[..8].copy_from_slice(&request.payload[..8]);
    let (len, offset) = match request.msg_type {
        1 => (40, 36),
        3 => (24, 16),
        5 | 7 => (40, 32),
        _ => (0, 0),
    };
    reply.payload_len = len;
    reply.payload[offset..offset + 4].copy_from_slice(&(status as u32).to_le_bytes());
    reply
}
fn unknown(request: &Envelope, operation: u64) -> Envelope {
    let mut r = empty_reply(request, StoreStatus::Unknown);
    r.payload[8..16].copy_from_slice(&operation.to_le_bytes());
    r
}
fn auth(core: &Arc<Core>, grant: &Arc<Grant>, state: &ObjectState) -> Result<(), StoreStatus> {
    if !grant.alive.load(Ordering::Acquire)
        || core.closed.load(Ordering::Acquire)
        || grant.epoch != state.journal.service_epoch
        || core.clock.load(Ordering::Acquire) >= grant.expires
    {
        return Err(StoreStatus::Stale);
    }
    Ok(())
}
fn validate_actor(grant: &Grant, request: &Envelope) -> Result<(), StoreStatus> {
    if u64_at(&request.payload, 8) != grant.actor.session_id
        || u64_at(&request.payload, 16) != grant.actor.session_generation
    {
        return Err(StoreStatus::Denied);
    }
    if request.msg_type == 1
        && (u64_at(&request.payload, 24) != grant.actor.instance_id
            || u64_at(&request.payload, 32) != grant.actor.instance_generation)
    {
        return Err(StoreStatus::Denied);
    }
    let right = match request.msg_type {
        1 => 1,
        3 | 5 => 2,
        7 => 4,
        _ => return Err(StoreStatus::Unsupported),
    };
    if grant.rights & right == 0
        || (grant.class == EndpointClass::RecoveryReceipt
            && (request.msg_type != 7 || grant.recovery_op != Some(op_of(request))))
    {
        return Err(StoreStatus::Denied);
    }
    Ok(())
}
fn record_index(s: &ObjectState, operation: u64) -> Result<usize, StoreStatus> {
    s.journal
        .operations
        .as_slice()
        .iter()
        .position(|r| r.allocation.operation_id == operation)
        .ok_or(StoreStatus::Denied)
}
fn records(
    s: &mut ObjectState,
    edit: impl FnOnce(&mut Vec<EditorOperationRecordV0>),
) -> Result<(), StoreStatus> {
    let mut rows = s.journal.operations.as_slice().to_vec();
    edit(&mut rows);
    s.journal.operations = schema(BoundedVec::try_from_vec(rows))?;
    Ok(())
}
fn active(s: &ObjectState) -> bool {
    s.journal.operations.as_slice().iter().any(|r| {
        (r.state == EditorOperationStateV0::Submitted && r.closure == EditorClosureV0::None)
            || r.state == EditorOperationStateV0::Permitted
    })
}
fn next_journal(s: &ObjectState) -> Result<EditorSelectionJournalV0, StoreStatus> {
    let mut j = s.journal.clone();
    j.transition_sequence = s
        .durable
        .transition_sequence
        .checked_add(1)
        .ok_or(StoreStatus::Exhausted)?;
    j.prior_journal_hash = s.digest;
    Ok(j)
}
fn settle_unpermitted(s: &mut ObjectState) -> Result<(), StoreStatus> {
    records(s, |rows| {
        for row in rows {
            if row.permit.is_none()
                && row.closure != EditorClosureV0::None
                && row.state != EditorOperationStateV0::Committed
            {
                row.state = EditorOperationStateV0::Noncommit;
                if let Some(binding) = &row.binding {
                    row.receipt = EditorSaveReceiptV0::noncommit(
                        binding,
                        row.submitted_service_epoch.unwrap(),
                    )
                    .ok();
                }
            }
        }
    })
}
fn close_operation(s: &mut ObjectState, operation: u64, closure: EditorClosureV0) {
    let Ok(index) = record_index(s, operation) else {
        return;
    };
    let rt = s.runtime.get(&operation);
    if let Some(original) = rt.and_then(|rt| rt.original.as_ref()) {
        original.closed.store(true, Ordering::Release);
    }
    if rt.is_some_and(|rt| rt.permitted.is_none()) {
        if let Some(source) = rt.and_then(|rt| rt.source.as_ref()) {
            source.alive.store(false, Ordering::Release);
        }
    }
    let _ = records(s, |rows| {
        let row = &mut rows[index];
        if row.closure == EditorClosureV0::None {
            row.closure = closure;
        }
        if row.permit.is_none() {
            row.state = EditorOperationStateV0::Noncommit;
            if let Some(binding) = row.binding.as_ref() {
                row.receipt =
                    EditorSaveReceiptV0::noncommit(binding, row.submitted_service_epoch.unwrap())
                        .ok();
            }
        }
    });
}
impl storage::Observer for Context {
    fn journal_prepared(&self, journal: &EditorSelectionJournalV0) -> Result<(), StoreStatus> {
        schema(journal.validate())?;
        lock(&self.object.state).prepared = Some(journal.clone());
        Ok(())
    }

    fn before(&self, point: IoPoint, artifact: Option<Hash32>) -> Result<(), StoreStatus> {
        let simulated = {
            let mut s = lock(&self.object.state);
            if s.fault == Some(point) {
                s.fault = None;
                true
            } else {
                false
            }
        };
        let mut d = lock(&self.core.diagnostics);
        let needed = if simulated { 1 } else { 2 };
        if d.exhausted || d.io.len() + d.reserved + needed > 1024 {
            d.exhausted = true;
            return Err(StoreStatus::Exhausted);
        }
        d.io_sequence = d.io_sequence.checked_add(1).ok_or(StoreStatus::Exhausted)?;
        let sequence = d.io_sequence;
        d.io.push(IoEvent {
            selected_object_id: self.object.id,
            operation_id: self.operation,
            writer_id: self.writer,
            sequence,
            point,
            outcome: if simulated {
                IoOutcome::SimulatedFailure
            } else {
                IoOutcome::Began
            },
            artifact_hash: artifact,
        });
        if simulated {
            return Err(StoreStatus::Unknown);
        }
        d.reserved += 1;
        Ok(())
    }
    fn after(&self, point: IoPoint, artifact: Option<Hash32>, success: bool) {
        if success && self.selection && point == IoPoint::JournalRename {
            lock(&self.object.state).transitions += 1;
        }
        if success && point == IoPoint::JournalDirectorySync {
            let mut s = lock(&self.object.state);
            if let Some(prepared) = &s.prepared {
                if prepared.digest().ok() == artifact {
                    s.synced = Some(prepared.clone());
                }
            }
        }
        let mut d = lock(&self.core.diagnostics);
        d.reserved -= 1;
        d.io_sequence += 1;
        let sequence = d.io_sequence;
        d.io.push(IoEvent {
            selected_object_id: self.object.id,
            operation_id: self.operation,
            writer_id: self.writer,
            sequence,
            point,
            outcome: if success {
                IoOutcome::Completed
            } else {
                IoOutcome::ActualFailure
            },
            artifact_hash: artifact,
        });
    }
    fn pause(&self, point: PausePoint) -> Result<(), StoreStatus> {
        let endpoint = self
            .request
            .as_ref()
            .map(|r| r.grant.handle.pack())
            .unwrap_or(0);
        let barrier = {
            let bars = lock(&self.core.barriers);
            bars.iter()
                .find(|b| {
                    b.object == self.object.id
                        && b.endpoint == endpoint
                        && b.point == point
                        && !lock(&b.state).snapshot.entered
                })
                .cloned()
        };
        let Some(barrier) = barrier else {
            return Ok(());
        };
        {
            let mut state = lock(&barrier.state);
            state.snapshot.request_id = self
                .request
                .as_ref()
                .map(|r| u64_at(&r.wire.payload, 0))
                .unwrap_or(0);
            state.snapshot.operation_id = self.operation;
            state.snapshot.permit_issued = self.writer != 0;
            state.snapshot.entered = true;
            barrier.changed.notify_all();
        }
        if let Some(request) = &self.request {
            if let Some(producer) = lock(&self.core.owned).handles.get_mut(&request.producer) {
                producer.barrier = Some(barrier.clone());
            }
            let worker = lock(&self.object.state).io_worker;
            if let Some(worker) = worker {
                if let Some(producer) = lock(&self.core.owned).handles.get_mut(&worker) {
                    producer.barrier = Some(barrier.clone());
                }
            }
        }
        let mut state = lock(&barrier.state);
        while !state.snapshot.released && !state.stop {
            state = barrier.changed.wait(state).unwrap();
        }
        let stop = state.stop;
        barrier.changed.notify_all();
        if stop {
            Err(StoreStatus::Unknown)
        } else {
            Ok(())
        }
    }
}
// Genesis-only administrative IO cannot overlap grant exposure or quiescence.
// Callers do not wait on this flag; they receive NotReady. It is never authority
// supplied by a client, and is cleared only when the setup call actually returns.
struct SetupGuard<'a>(&'a AtomicBool);
impl Drop for SetupGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
struct WorkerExit {
    core: Arc<Core>,
    object: Option<Arc<Object>>,
    producer: u64,
}
impl Drop for WorkerExit {
    fn drop(&mut self) {
        if let Some(object) = &self.object {
            let mut s = lock(&object.state);
            if s.io_worker == Some(self.producer) {
                s.io_worker = None;
            }
            if thread::panicking() {
                s.quarantine = true;
            }
        }
        self.core.finished(self.producer);
    }
}
struct ExchangeReservation<'a> {
    core: &'a Core,
}
impl Drop for ExchangeReservation<'_> {
    fn drop(&mut self) {
        lock(&self.core.diagnostics).exchange_reservations -= 1;
    }
}
impl Core {
    fn reserve_exchange(&self) -> Result<ExchangeReservation<'_>, StoreStatus> {
        let mut d = lock(&self.diagnostics);
        if d.exhausted || d.exchanges.len() + d.exchange_reservations >= 256 {
            return Err(StoreStatus::Exhausted);
        }
        d.exchange_reservations += 1;
        Ok(ExchangeReservation { core: self })
    }
    fn object(&self, id: u64) -> Result<Arc<Object>, StoreStatus> {
        self.objects.get(&id).cloned().ok_or(StoreStatus::Denied)
    }
    fn new_handle(registry: &mut Registry, kind: HandleKind) -> Result<Handle, StoreStatus> {
        let id = registry.next_identity;
        if id == 0 || id > 65535 {
            return Err(StoreStatus::Exhausted);
        }
        registry.next_identity = id.checked_add(1).ok_or(StoreStatus::Exhausted)?;
        Ok(Handle {
            kind,
            index: id as u32,
            generation: 1,
        })
    }
    fn reserve_producer(
        &self,
        kind: ProducerKind,
        object: u64,
        request: u64,
        operation: u64,
    ) -> Result<u64, StoreStatus> {
        let mut owned = lock(&self.owned);
        if self.closed.load(Ordering::Acquire) {
            return Err(StoreStatus::Stale);
        }
        let is_io = matches!(kind, ProducerKind::ObjectIoWorker);
        let held = owned
            .handles
            .values()
            .filter(|p| matches!(p.kind, ProducerKind::ObjectIoWorker) == is_io)
            .count();
        if held >= if is_io { 2 } else { 64 } {
            return Err(StoreStatus::Exhausted);
        }
        let id = owned.next;
        owned.next = id.checked_add(1).ok_or(StoreStatus::Exhausted)?;
        owned.handles.insert(
            id,
            Producer {
                kind,
                object,
                request,
                operation,
                handle: None,
                joining: false,
                terminal_join: None,
                thread_id: String::new(),
                barrier: None,
            },
        );
        Ok(id)
    }
    fn register_handle(&self, id: u64, handle: JoinHandle<()>) {
        let mut owned = lock(&self.owned);
        let entry = owned.handles.get_mut(&id).expect("reserved producer");
        entry.thread_id = format!("{:?}", handle.thread().id());
        entry.handle = Some(handle);
        self.owned_changed.notify_all();
    }
    fn finished(&self, id: u64) {
        let barrier = lock(&self.owned)
            .handles
            .get(&id)
            .and_then(|p| p.barrier.clone());
        if let Some(barrier) = barrier {
            let mut b = lock(&barrier.state);
            b.snapshot.settled = true;
            barrier.changed.notify_all();
        }
        self.owned_changed.notify_all();
    }
    fn join_one(&self, id: u64) {
        let handle = {
            let mut owned = lock(&self.owned);
            if self.closed.load(Ordering::Acquire) {
                return;
            }
            let Some(p) = owned.handles.get_mut(&id) else {
                return;
            };
            if p.joining {
                return;
            }
            p.joining = true;
            p.handle.take()
        };
        if let Some(handle) = handle {
            let outcome = if handle.join().is_ok() {
                ProducerJoinOutcome::Returned
            } else {
                ProducerJoinOutcome::Panicked
            };
            let mut owned = lock(&self.owned);
            if self.closed.load(Ordering::Acquire) {
                if let Some(p) = owned.handles.get_mut(&id) {
                    p.joining = false;
                    p.terminal_join = Some(outcome);
                }
            } else {
                owned.handles.remove(&id);
            }
            self.owned_changed.notify_all();
        }
    }

    fn reap(&self) {
        if self.closed.load(Ordering::Acquire) {
            return;
        }
        let ids = {
            let owned = lock(&self.owned);
            owned
                .handles
                .iter()
                .filter(|(_, p)| p.handle.as_ref().is_some_and(JoinHandle::is_finished))
                .map(|(id, _)| *id)
                .collect::<Vec<_>>()
        };
        for id in ids {
            self.join_one(id)
        }
    }
    fn run_io<T: Send + 'static>(
        core: &Arc<Self>,
        object: &Arc<Object>,
        request: &Arc<Request>,
        operation: u64,
        writer: u64,
        job: impl FnOnce(Context) -> Result<T, StoreStatus> + Send + 'static,
    ) -> Result<T, StoreStatus> {
        let producer = {
            let mut s = lock(&object.state);
            if s.io_worker.is_some() {
                return Err(StoreStatus::NotReady);
            }
            let id = core.reserve_producer(
                ProducerKind::ObjectIoWorker,
                object.id,
                u64_at(&request.wire.payload, 0),
                operation,
            )?;
            s.io_worker = Some(id);
            id
        };
        let (start_tx, start_rx) = mpsc::channel();
        let (tx, rx) = mpsc::channel();
        let c = core.clone();
        let o = object.clone();
        let r = request.clone();
        let handle = thread::Builder::new().spawn(move || {
            let _ = start_rx.recv();
            let _exit = WorkerExit {
                core: c.clone(),
                object: Some(o.clone()),
                producer,
            };
            let context = Context {
                core: c.clone(),
                object: o.clone(),
                request: Some(r),
                operation,
                writer,
                selection: false,
            };
            let result = job(context);
            // The finalizer drops before publishing completion to the dispatcher.
            drop(_exit);
            let _ = tx.send(result);
        });
        let handle = match handle {
            Ok(handle) => handle,
            Err(_) => {
                lock(&object.state).io_worker = None;
                lock(&core.owned).handles.remove(&producer);
                core.owned_changed.notify_all();
                return Err(StoreStatus::Internal);
            }
        };
        core.register_handle(producer, handle);
        let _ = start_tx.send(());
        let result = rx.recv().map_err(|_| StoreStatus::Internal)?;
        core.join_one(producer);
        result
    }
    fn append_exchange(
        &self,
        grant: &Grant,
        request: &Envelope,
        reply: &Envelope,
        suppressed: bool,
        start: Instant,
    ) -> Result<(), StoreStatus> {
        let mut d = lock(&self.diagnostics);
        if d.exhausted || d.exchanges.len() >= 256 {
            d.exhausted = true;
            return Err(StoreStatus::Exhausted);
        }
        d.exchanges.push(Exchange {
            actor: grant.actor.clone(),
            endpoint_class: grant.class,
            selected_object_id: grant.object,
            request_wire: encode_envelope_wire(request)?,
            actual_reply_wire: Some(encode_envelope_wire(reply)?),
            reply_suppressed: suppressed,
            operation_id: op_of(request),
            elapsed_us: start.elapsed().as_micros().min(u64::MAX as u128) as u64,
            observed_ms: self.clock.load(Ordering::Acquire),
        });
        Ok(())
    }
    fn data(
        core: &Arc<Self>,
        grant: &Arc<Grant>,
        kind: SharedObjectKind,
        operation: Option<u64>,
        revision: u64,
        bytes: Vec<u8>,
        life: Option<&Request>,
    ) -> Result<ObjectDescriptor, StoreStatus> {
        let object = core.object(grant.object)?;
        let mut reg = lock(&core.registry);
        let s = lock(&object.state);
        auth(core, grant, &s)?;
        if life.is_some_and(|r| r.closed.load(Ordering::Acquire) || Instant::now() >= r.deadline) {
            return Err(StoreStatus::Timeout);
        }
        if let Some(existing) = reg.data.values().find(|data| {
            data.alive.load(Ordering::Acquire)
                && data.actor == grant.actor
                && data.object == grant.object
                && data.kind == kind
                && data.operation == operation
                && data.revision == revision
        }) {
            return Ok(ObjectDescriptor {
                handle: existing.handle,
                kind,
                object_generation: existing.handle.generation,
                byte_len: lock(&existing.state).bytes.len() as u32,
            });
        }
        if reg.data.len() >= 32 {
            return Err(StoreStatus::Exhausted);
        }
        let handle = Self::new_handle(&mut reg, HandleKind::Shmem)?;
        let len = bytes.len() as u32;
        reg.data.insert(
            handle.pack(),
            Arc::new(Data {
                handle,
                actor: grant.actor.clone(),
                object: grant.object,
                operation,
                revision,
                kind,
                alive: AtomicBool::new(true),
                state: Mutex::new(DataState {
                    bytes,
                    writable: kind == SharedObjectKind::DraftSource,
                    frozen: false,
                }),
            }),
        );
        Ok(ObjectDescriptor {
            handle,
            kind,
            object_generation: handle.generation,
            byte_len: len,
        })
    }
}
fn install_journal(s: &mut ObjectState, journal: EditorSelectionJournalV0, digest: Hash32) {
    let old = s.journal.operations.as_slice().to_vec();
    s.durable = journal.clone();
    s.synced = Some(journal.clone());
    s.digest = digest;
    s.journal = journal;
    let _ = records(s, |rows| {
        for row in rows {
            if let Some(prior) = old
                .iter()
                .find(|r| r.allocation.operation_id == row.allocation.operation_id)
            {
                if row.closure == EditorClosureV0::None {
                    row.closure = prior.closure;
                }
            }
        }
    });
}
fn persist(context: &Context, selection: bool) -> Result<(), StoreStatus> {
    let j = {
        let mut s = lock(&context.object.state);
        settle_unpermitted(&mut s)?;
        next_journal(&s)?
    };
    if selection {
        return Err(StoreStatus::Internal);
    }
    let digest = storage::publish_journal(&context.object.root, &j, context, false)?;
    let mut s = lock(&context.object.state);
    install_journal(&mut s, j, digest);
    Ok(())
}
fn descriptor(data: &Arc<Data>) -> ObjectDescriptor {
    ObjectDescriptor {
        handle: data.handle,
        kind: data.kind,
        object_generation: data.handle.generation,
        byte_len: lock(&data.state).bytes.len() as u32,
    }
}
impl StoreHost {
    fn peer(&self, peer: &EditorPeer, request: &Envelope) -> Result<Arc<Grant>, StoreStatus> {
        if !Arc::ptr_eq(&self.core, &peer.core) || peer.grant.handle != request.handle {
            return Err(StoreStatus::Denied);
        }
        validate_actor(&peer.grant, request)?;
        Ok(peer.grant.clone())
    }
    pub fn draft_source(
        &self,
        peer: &EditorPeer,
        expected_revision: u64,
        bytes: &[u8],
    ) -> Result<ObjectDescriptor, StoreStatus> {
        if self.core.native_only || !Arc::ptr_eq(&self.core, &peer.core) {
            return Err(StoreStatus::Denied);
        }
        let grant = &peer.grant;
        if grant.class != EndpointClass::Artifact || grant.rights & 2 == 0 {
            return Err(StoreStatus::Denied);
        }
        let object = self.core.object(grant.object)?;
        let mut reg = lock(&self.core.registry);
        let s = lock(&object.state);
        auth(&self.core, grant, &s)?;
        if s.mutation_fenced {
            return Err(StoreStatus::Exhausted);
        }
        if expected_revision != s.journal.current.revision {
            return Err(StoreStatus::Conflict);
        }
        let header = schema(EditorTextHeaderV0::try_new(
            grant.object,
            expected_revision,
            bytes,
        ))?;
        let mut encoded = schema(header.encode_le())?.to_vec();
        encoded.extend_from_slice(bytes);
        let previous = reg
            .data
            .values()
            .filter(|d| {
                d.actor == grant.actor
                    && d.object == grant.object
                    && d.kind == SharedObjectKind::DraftSource
            })
            .cloned()
            .collect::<Vec<_>>();
        for data in &previous {
            if data.alive.load(Ordering::Acquire)
                && lock(&data.state).frozen
                && !s.journal.operations.as_slice().iter().any(|row| {
                    row.receipt.is_some()
                        && s.runtime
                            .get(&row.allocation.operation_id)
                            .and_then(|rt| rt.source.as_ref())
                            .is_some_and(|source| Arc::ptr_eq(source, data))
                })
            {
                return Err(StoreStatus::NotReady);
            }
        }
        if reg.data.len() - previous.len() >= 32 {
            return Err(StoreStatus::Exhausted);
        }
        let handle = Core::new_handle(&mut reg, HandleKind::Shmem)?;
        for data in previous {
            data.alive.store(false, Ordering::Release);
            lock(&data.state).writable = false;
            reg.data.remove(&data.handle.pack());
        }
        let data = Arc::new(Data {
            handle,
            actor: grant.actor.clone(),
            object: grant.object,
            operation: None,
            revision: expected_revision,
            kind: SharedObjectKind::DraftSource,
            alive: AtomicBool::new(true),
            state: Mutex::new(DataState {
                bytes: encoded,
                writable: true,
                frozen: false,
            }),
        });
        let out = descriptor(&data);
        reg.data.insert(handle.pack(), data);
        Ok(out)
    }
    fn lease_data(
        &self,
        peer: &EditorPeer,
        descriptor: &ObjectDescriptor,
        write: bool,
    ) -> Result<Arc<Data>, StoreStatus> {
        if self.core.native_only || !Arc::ptr_eq(&self.core, &peer.core) {
            return Err(StoreStatus::Denied);
        }
        // Typed descriptors must be canonical before packing: pack() truncates
        // the index/generation to their wire widths and is not validation.
        if descriptor.handle.kind != HandleKind::Shmem
            || !(1..=u16::MAX as u32).contains(&descriptor.handle.index)
            || !(1..=u32::MAX as u64).contains(&descriptor.handle.generation)
            || descriptor.object_generation != descriptor.handle.generation
        {
            return Err(StoreStatus::Denied);
        }
        let data = lock(&self.core.registry)
            .data
            .get(&descriptor.handle.pack())
            .cloned()
            .ok_or(StoreStatus::Stale)?;
        let object = self.core.object(peer.grant.object)?;
        let s = lock(&object.state);
        auth(&self.core, &peer.grant, &s)?;
        if descriptor.handle != data.handle
            || descriptor.object_generation != data.handle.generation
            || descriptor.kind != data.kind
            || descriptor.byte_len as usize != lock(&data.state).bytes.len()
            || data.actor != peer.grant.actor
            || data.object != peer.grant.object
        {
            return Err(StoreStatus::Denied);
        }
        if !data.alive.load(Ordering::Acquire) {
            return Err(StoreStatus::Stale);
        }
        if write {
            if data.kind != SharedObjectKind::DraftSource
                || peer.grant.class != EndpointClass::Artifact
                || peer.grant.rights & 2 == 0
            {
                return Err(StoreStatus::Denied);
            }
        } else {
            if data.kind == SharedObjectKind::DraftSource {
                return Err(StoreStatus::Denied);
            }
            let right = if data.kind == SharedObjectKind::SelectedText {
                1
            } else {
                4
            };
            if peer.grant.rights & right == 0
                || peer.grant.class == EndpointClass::RecoveryReceipt
                    && (data.kind != SharedObjectKind::Receipt
                        || peer.grant.recovery_op != data.operation)
            {
                return Err(StoreStatus::Denied);
            }
        }
        Ok(data)
    }
    pub fn read_lease(
        &self,
        peer: &EditorPeer,
        descriptor: &ObjectDescriptor,
    ) -> Result<ReadLease, StoreStatus> {
        Ok(ReadLease {
            peer: peer.clone(),
            data: self.lease_data(peer, descriptor, false)?,
        })
    }
    pub fn write_lease(
        &self,
        peer: &EditorPeer,
        descriptor: &ObjectDescriptor,
    ) -> Result<WriteLease, StoreStatus> {
        Ok(WriteLease {
            peer: peer.clone(),
            data: self.lease_data(peer, descriptor, true)?,
        })
    }
}
impl ReadLease {
    pub fn copy_into(&self, offset: u32, out: &mut [u8]) -> Result<(), StoreStatus> {
        if self.peer.core.native_only {
            return Err(StoreStatus::Denied);
        }
        let object = self.peer.core.object(self.peer.grant.object)?;
        let s = lock(&object.state);
        auth(&self.peer.core, &self.peer.grant, &s)?;
        if !self.data.alive.load(Ordering::Acquire) {
            return Err(StoreStatus::Stale);
        }
        let state = lock(&self.data.state);
        let end = (offset as usize)
            .checked_add(out.len())
            .ok_or(StoreStatus::Invalid)?;
        let input = state
            .bytes
            .get(offset as usize..end)
            .ok_or(StoreStatus::Invalid)?;
        out.copy_from_slice(input);
        Ok(())
    }
}
impl WriteLease {
    pub fn copy_from(&self, offset: u32, input: &[u8]) -> Result<(), StoreStatus> {
        if self.peer.core.native_only {
            return Err(StoreStatus::Denied);
        }
        let object = self.peer.core.object(self.peer.grant.object)?;
        let s = lock(&object.state);
        auth(&self.peer.core, &self.peer.grant, &s)?;
        if !self.data.alive.load(Ordering::Acquire) {
            return Err(StoreStatus::Stale);
        }
        let mut state = lock(&self.data.state);
        if !state.writable || state.frozen {
            return Err(StoreStatus::Denied);
        }
        let end = (offset as usize)
            .checked_add(input.len())
            .ok_or(StoreStatus::Invalid)?;
        let out = state
            .bytes
            .get_mut(offset as usize..end)
            .ok_or(StoreStatus::Invalid)?;
        out.copy_from_slice(input);
        Ok(())
    }
}
impl StoreHost {
    fn fast(
        &self,
        grant: &Arc<Grant>,
        request: &Envelope,
    ) -> Result<Option<Envelope>, StoreStatus> {
        let object = self.core.object(grant.object)?;
        let mut s = lock(&object.state);
        auth(&self.core, grant, &s)?;
        match request.msg_type {
            3 => {
                if s.allocation_fenced || s.mutation_fenced || s.journal.operations.len() >= 16 {
                    return Err(StoreStatus::Exhausted);
                }
                if s.durable.transition_sequence == u64::MAX {
                    s.allocation_fenced = true;
                    s.mutation_fenced = true;
                    return Err(StoreStatus::Exhausted);
                }
                if active(&s) || s.io_worker.is_some() || s.quarantine {
                    return Err(StoreStatus::NotReady);
                }
                if u64_at(&request.payload, 24) != s.journal.current.revision {
                    return Err(StoreStatus::Conflict);
                }
            }
            5 => {
                let operation = op_of(request);
                let index = record_index(&s, operation)?;
                let record = &s.journal.operations.as_slice()[index];
                if record.allocation.actor != grant.actor
                    || record.allocation.expected_revision != u64_at(&request.payload, 24)
                {
                    return Err(StoreStatus::Denied);
                }
                if let Some(binding) = &record.binding {
                    if binding.source.source_object_id != u64_at(&request.payload, 40)
                        || u64::from(u32::from_le_bytes(
                            request.payload[48..52].try_into().unwrap(),
                        )) != 64 + u64::from(binding.source.byte_len)
                    {
                        return Err(StoreStatus::Denied);
                    }
                }
                if s.mutation_fenced {
                    return Err(StoreStatus::Exhausted);
                }
                if let Some(receipt) = &record.receipt {
                    let receipt = receipt.clone();
                    drop(s);
                    return self.receipt_reply(grant, request, &receipt, None).map(Some);
                }
                if s.runtime
                    .get(&operation)
                    .is_some_and(|rt| rt.original.is_some())
                    || active(&s)
                    || s.quarantine
                    || s.io_worker.is_some()
                {
                    return Err(StoreStatus::NotReady);
                }
                if record.allocation.expected_revision != s.journal.current.revision {
                    let mut reply = empty_reply(request, StoreStatus::Conflict);
                    reply.payload[16..24]
                        .copy_from_slice(&s.journal.current.revision.to_le_bytes());
                    return Ok(Some(reply));
                }
            }
            7 => {
                let index = record_index(&s, op_of(request))?;
                let record = &s.journal.operations.as_slice()[index];
                if record.allocation.actor != grant.actor {
                    return Err(StoreStatus::Denied);
                }
                if let Some(receipt) = &record.receipt {
                    let receipt = receipt.clone();
                    drop(s);
                    return self.receipt_reply(grant, request, &receipt, None).map(Some);
                }
                if record.binding.is_some() {
                    return Ok(Some(unknown(request, op_of(request))));
                }
                return Err(StoreStatus::NotReady);
            }
            _ => {}
        }
        Ok(None)
    }
    fn receipt_reply(
        &self,
        grant: &Arc<Grant>,
        request: &Envelope,
        receipt: &EditorSaveReceiptV0,
        life: Option<&Request>,
    ) -> Result<Envelope, StoreStatus> {
        let bytes = schema(receipt.encode_le())?.to_vec();
        let desc = Core::data(
            &self.core,
            grant,
            SharedObjectKind::Receipt,
            Some(receipt.operation_id),
            receipt.result_revision,
            bytes,
            life,
        )?;
        let mut reply = empty_reply(request, StoreStatus::Ok);
        reply.payload[8..16].copy_from_slice(&receipt.operation_id.to_le_bytes());
        reply.payload[16..24].copy_from_slice(&receipt.result_revision.to_le_bytes());
        reply.payload[24..32].copy_from_slice(&desc.handle.pack().to_le_bytes());
        reply.payload[36..40].copy_from_slice(&176u32.to_le_bytes());
        Ok(reply)
    }
    pub fn dispatch(&self, peer: &EditorPeer, request: &Envelope) -> Result<Envelope, StoreStatus> {
        if self.core.native_only {
            return Err(StoreStatus::Denied);
        }
        let start = Instant::now();
        codec::validate(request)?;
        if !matches!(request.msg_type, 1 | 3 | 5 | 7) {
            return Err(StoreStatus::Unsupported);
        }
        let _trace = self.core.reserve_exchange()?;
        self.core.reap();
        let grant = self.peer(peer, request);
        let result = match grant {
            Err(status) => empty_reply(request, status),
            Ok(grant) => match self.fast(&grant, request) {
                Err(status) => empty_reply(request, status),
                Ok(Some(reply)) => reply,
                Ok(None) => {
                    let object = self.core.object(grant.object)?;
                    {
                        let s = lock(&object.state);
                        if let Err(status) = auth(&self.core, &grant, &s) {
                            let reply = empty_reply(request, status);
                            self.core
                                .append_exchange(&grant, request, &reply, false, start)?;
                            return Ok(reply);
                        }
                        if grant.invocation.swap(true, Ordering::AcqRel) {
                            let reply = empty_reply(request, StoreStatus::NotReady);
                            self.core
                                .append_exchange(&grant, request, &reply, false, start)?;
                            return Ok(reply);
                        }
                    }
                    let id = match self.core.reserve_producer(
                        ProducerKind::Dispatcher,
                        object.id,
                        u64_at(&request.payload, 0),
                        op_of(request),
                    ) {
                        Ok(id) => id,
                        Err(status) => {
                            grant.invocation.store(false, Ordering::Release);
                            let reply = empty_reply(request, status);
                            self.core
                                .append_exchange(&grant, request, &reply, false, start)?;
                            return Ok(reply);
                        }
                    };
                    let ticket = Arc::new(Request {
                        producer: id,
                        grant: grant.clone(),
                        wire: *request,
                        deadline: start + Duration::from_millis(1000),
                        submitted_source: if request.msg_type == 5 {
                            lock(&self.core.registry)
                                .data
                                .get(&u64_at(&request.payload, 40))
                                .cloned()
                        } else {
                            None
                        },
                        closed: AtomicBool::new(false),
                    });
                    let (tx, rx) = mpsc::channel();
                    let host = self.clone();
                    let worker_ticket = ticket.clone();
                    let core = self.core.clone();
                    let handle = thread::Builder::new().spawn(move || {
                        let _exit = WorkerExit {
                            core: core.clone(),
                            object: None,
                            producer: id,
                        };
                        let reply = host.execute(&worker_ticket);
                        drop(_exit);
                        let _ = tx.send(reply);
                    });
                    let handle = match handle {
                        Ok(handle) => handle,
                        Err(_) => {
                            grant.invocation.store(false, Ordering::Release);
                            lock(&self.core.owned).handles.remove(&id);
                            self.core.owned_changed.notify_all();
                            let reply = empty_reply(request, StoreStatus::Internal);
                            self.core
                                .append_exchange(&grant, request, &reply, false, start)?;
                            return Ok(reply);
                        }
                    };
                    self.core.register_handle(id, handle);
                    let result = match rx
                        .recv_timeout(ticket.deadline.saturating_duration_since(Instant::now()))
                    {
                        Ok(Ok(reply)) => reply,
                        Ok(Err(error)) => {
                            let s = lock(&object.state);
                            if request.msg_type == 5
                                && s.runtime.get(&op_of(request)).is_some_and(|rt| {
                                    rt.original.as_ref().is_some_and(|r| r.producer == id)
                                        && rt.permitted.is_some()
                                })
                            {
                                unknown(request, op_of(request))
                            } else {
                                // No permit exists: fence the ticket, with no uncertain
                                // mutation capability. Unknown is reserved for an issued
                                // original transition or authenticated receipt lookup.
                                empty_reply(
                                    request,
                                    if error == StoreStatus::Unknown {
                                        StoreStatus::Internal
                                    } else {
                                        error
                                    },
                                )
                            }
                        }
                        Err(_) => self.deadline(&ticket),
                    };
                    grant.invocation.store(false, Ordering::Release);
                    result
                }
            },
        };
        let suppressed = peer.grant.drop_reply.swap(false, Ordering::AcqRel);
        self.core
            .append_exchange(&peer.grant, request, &result, suppressed, start)?;
        if suppressed {
            Ok(empty_reply(request, StoreStatus::Disconnected))
        } else {
            Ok(result)
        }
    }
    fn deadline(&self, request: &Arc<Request>) -> Envelope {
        let operation = op_of(&request.wire);
        let Ok(object) = self.core.object(request.grant.object) else {
            return empty_reply(&request.wire, StoreStatus::Denied);
        };
        let mut s = lock(&object.state);
        request.closed.store(true, Ordering::Release);
        if request.wire.msg_type != 5 {
            return empty_reply(&request.wire, StoreStatus::Timeout);
        }
        let Ok(index) = record_index(&s, operation) else {
            return empty_reply(&request.wire, StoreStatus::Denied);
        };
        let record = &s.journal.operations.as_slice()[index];
        if record.allocation.actor != request.grant.actor
            || record.allocation.expected_revision != u64_at(&request.wire.payload, 24)
            || record.binding.as_ref().is_some_and(|b| {
                b.source.source_object_id != u64_at(&request.wire.payload, 40)
                    || b.source.byte_len + 64
                        != u32::from_le_bytes(request.wire.payload[48..52].try_into().unwrap())
            })
        {
            return empty_reply(&request.wire, StoreStatus::Denied);
        }
        if record.binding.is_none()
            && !request.submitted_source.as_ref().is_some_and(|source| {
                source.actor == request.grant.actor
                    && source.object == object.id
                    && source.kind == SharedObjectKind::DraftSource
                    && source.handle.pack() == u64_at(&request.wire.payload, 40)
                    && source.alive.load(Ordering::Acquire)
                    && lock(&source.state).bytes.len()
                        == u32::from_le_bytes(request.wire.payload[48..52].try_into().unwrap())
                            as usize
            })
        {
            return empty_reply(&request.wire, StoreStatus::Denied);
        }
        let original = s
            .runtime
            .get(&operation)
            .and_then(|rt| rt.original.as_ref());
        if original.is_some_and(|r| r.producer != request.producer) {
            return empty_reply(
                &request.wire,
                auth(&self.core, &request.grant, &s)
                    .err()
                    .unwrap_or(StoreStatus::NotReady),
            );
        }
        if s.runtime
            .get(&operation)
            .is_some_and(|rt| rt.permitted.is_some())
        {
            return unknown(&request.wire, operation);
        }
        if let Err(status) = auth(&self.core, &request.grant, &s) {
            return empty_reply(&request.wire, status);
        }
        close_operation(&mut s, operation, EditorClosureV0::Deadline);
        empty_reply(&request.wire, StoreStatus::Timeout)
    }
    fn execute(&self, request: &Arc<Request>) -> Result<Envelope, StoreStatus> {
        match request.wire.msg_type {
            1 => self.selected_reply(request),
            3 => self.allocate(request),
            5 => self.commit(request),
            _ => Err(StoreStatus::Unsupported),
        }
    }
    fn selected_reply(&self, request: &Arc<Request>) -> Result<Envelope, StoreStatus> {
        let object = self.core.object(request.grant.object)?;
        let (selected, bytes) = {
            let s = lock(&object.state);
            auth(&self.core, &request.grant, &s)?;
            (s.journal.current.clone(), s.bytes.clone())
        };
        let header = schema(EditorTextHeaderV0::try_new(
            selected.selected_object_id,
            selected.revision,
            &bytes,
        ))?;
        if header.content_hash != selected.content_hash {
            return Err(StoreStatus::Internal);
        }
        let mut encoded = schema(header.encode_le())?.to_vec();
        encoded.extend_from_slice(&bytes);
        let context = Context {
            core: self.core.clone(),
            object,
            request: Some(request.clone()),
            operation: 0,
            writer: 0,
            selection: false,
        };
        storage::Observer::pause(&context, PausePoint::BeforeReadReply)?;
        if request.closed.load(Ordering::Acquire) || Instant::now() >= request.deadline {
            return Err(StoreStatus::Timeout);
        }
        let data = Core::data(
            &self.core,
            &request.grant,
            SharedObjectKind::SelectedText,
            None,
            selected.revision,
            encoded,
            Some(request),
        )?;
        a::ReadSelectedReply {
            request_id: u64_at(&request.wire.payload, 0),
            revision: selected.revision,
            data_shm: data.handle.pack(),
            object_generation: data.object_generation,
            byte_len: data.byte_len,
            status: 0,
        }
        .encode(request.grant.handle)
    }
    fn allocate(&self, request: &Arc<Request>) -> Result<Envelope, StoreStatus> {
        let object = self.core.object(request.grant.object)?;
        let core = self.core.clone();
        let r = request.clone();
        let op = Core::run_io(&self.core, &object, request, 0, 0, move |context| {
            storage::Observer::pause(&context, PausePoint::BeforeAllocationJournal)?;
            let operation = {
                let mut s = lock(&context.object.state);
                auth(&core, &r.grant, &s)?;
                if r.closed.load(Ordering::Acquire) || Instant::now() >= r.deadline {
                    return Err(StoreStatus::Timeout);
                }
                if s.allocation_fenced || s.mutation_fenced || s.journal.operations.len() >= 16 {
                    return Err(StoreStatus::Exhausted);
                }
                if active(&s) || s.quarantine {
                    return Err(StoreStatus::NotReady);
                }
                if s.journal.current.revision != u64_at(&r.wire.payload, 24) {
                    return Err(StoreStatus::Conflict);
                }
                settle_unpermitted(&mut s)?;
                if s.next_op > s.journal.operation_high_water || s.force_op_range {
                    s.journal.operation_high_water =
                        match s.journal.operation_high_water.checked_add(16) {
                            Some(value) => value,
                            None => {
                                s.allocation_fenced = true;
                                return Err(StoreStatus::Exhausted);
                            }
                        };
                    s.next_op = s.journal.operation_high_water - 15;
                    s.force_op_range = false;
                }
                let operation = s.next_op;
                s.next_op = s.next_op.checked_add(1).ok_or(StoreStatus::Exhausted)?;
                let allocation = schema(EditorSaveAllocationV0::try_new(
                    r.grant.actor.clone(),
                    &s.journal.current,
                    operation,
                ))?;
                let epoch = s.journal.service_epoch;
                records(&mut s, |rows| {
                    rows.push(EditorOperationRecordV0 {
                        allocation,
                        allocated_service_epoch: epoch,
                        binding: None,
                        submitted_service_epoch: None,
                        permit: None,
                        closure: EditorClosureV0::None,
                        state: EditorOperationStateV0::Allocated,
                        successor: None,
                        receipt: None,
                    })
                })?;
                s.runtime.insert(
                    operation,
                    RuntimeOperation {
                        original: None,
                        source: None,
                        permitted: None,
                        dispatched: false,
                    },
                );
                operation
            };
            let context = Context {
                operation,
                ..context
            };
            persist(&context, false)?;
            Ok(operation)
        })?;
        a::AllocateSaveIdReply {
            request_id: u64_at(&request.wire.payload, 0),
            operation_id: op,
            status: 0,
            reserved: 0,
        }
        .encode(request.grant.handle)
    }
    fn commit(&self, request: &Arc<Request>) -> Result<Envelope, StoreStatus> {
        let operation = op_of(&request.wire);
        let object = self.core.object(request.grant.object)?;
        let source = lock(&self.core.registry)
            .data
            .get(&u64_at(&request.wire.payload, 40))
            .cloned()
            .ok_or(StoreStatus::Denied)?;
        let bytes = {
            let mut s = lock(&object.state);
            auth(&self.core, &request.grant, &s)?;
            if s.mutation_fenced {
                return Err(StoreStatus::Exhausted);
            }
            let index = record_index(&s, operation)?;
            if s.journal.operations.as_slice()[index].allocation.actor != request.grant.actor {
                return Err(StoreStatus::Denied);
            }
            if s.runtime[&operation].original.is_some() {
                return Err(StoreStatus::NotReady);
            }
            if active(&s) || s.quarantine {
                return Err(StoreStatus::NotReady);
            }
            if !source.alive.load(Ordering::Acquire)
                || source.actor != request.grant.actor
                || source.object != object.id
                || source.kind != SharedObjectKind::DraftSource
            {
                return Err(StoreStatus::Denied);
            }
            let rt = s.runtime.get_mut(&operation).unwrap();
            if !rt.dispatched {
                rt.dispatched = true;
                s.dispatches += 1;
            }
            let mut data = lock(&source.state);
            if data.frozen || !data.writable {
                return Err(StoreStatus::Denied);
            }
            if data.bytes.len() < 64
                || data.bytes.len()
                    != u32::from_le_bytes(request.wire.payload[48..52].try_into().unwrap()) as usize
            {
                return Err(StoreStatus::Invalid);
            }
            let header = schema(EditorTextHeaderV0::decode_le(&data.bytes[..64]))?;
            schema(header.validate_bytes(&data.bytes[64..]))?;
            if header.selected_object_id != object.id
                || header.revision != u64_at(&request.wire.payload, 24)
            {
                return Err(StoreStatus::Invalid);
            }
            let binding = schema(EditorSaveBindingV0::try_new(
                s.journal.operations.as_slice()[index].allocation.clone(),
                schema(EditorSourceBindingV0::try_new(
                    source.handle.pack(),
                    source.handle.generation,
                    header.byte_len,
                    header.content_hash,
                ))?,
            ))?;
            if request.closed.load(Ordering::Acquire) || Instant::now() >= request.deadline {
                return Err(StoreStatus::Timeout);
            }
            data.frozen = true;
            data.writable = false;
            let bytes = data.bytes[64..].to_vec();
            let epoch = s.journal.service_epoch;
            records(&mut s, |rows| {
                let row = &mut rows[index];
                row.binding = Some(binding);
                row.submitted_service_epoch = Some(epoch);
                row.state = EditorOperationStateV0::Submitted;
            })?;
            let rt = s.runtime.get_mut(&operation).unwrap();
            rt.original = Some(request.clone());
            rt.source = Some(source.clone());
            bytes
        };
        let submitted = Core::run_io(&self.core, &object, request, operation, 0, |context| {
            storage::Observer::pause(&context, PausePoint::BeforeSubmissionJournal)?;
            persist(&context, false)
        });
        if let Err(error) = submitted {
            let mut s = lock(&object.state);
            close_operation(&mut s, operation, EditorClosureV0::Deadline);
            return Err(error);
        }
        let context = Context {
            core: self.core.clone(),
            object: object.clone(),
            request: Some(request.clone()),
            operation,
            writer: 0,
            selection: false,
        };
        storage::Observer::pause(&context, PausePoint::BeforeCommitPermit)?;
        {
            let mut s = lock(&object.state);
            auth(&self.core, &request.grant, &s)?;
            if request.closed.load(Ordering::Acquire) || Instant::now() >= request.deadline {
                close_operation(&mut s, operation, EditorClosureV0::Deadline);
                return Err(StoreStatus::Timeout);
            }
        }
        let core = self.core.clone();
        let r = request.clone();
        let outcome = Core::run_io(
            &self.core,
            &object,
            request,
            operation,
            0,
            move |mut context| {
                let permit = {
                    let mut s = lock(&context.object.state);
                    auth(&core, &r.grant, &s)?;
                    if r.closed.load(Ordering::Acquire) || Instant::now() >= r.deadline {
                        close_operation(&mut s, operation, EditorClosureV0::Deadline);
                        return Err(StoreStatus::Timeout);
                    }
                    let index = record_index(&s, operation)?;
                    let row = &s.journal.operations.as_slice()[index];
                    if row.closure != EditorClosureV0::None
                        || s.runtime[&operation].original.as_ref().unwrap().producer != r.producer
                    {
                        return Err(StoreStatus::Timeout);
                    }
                    if row.allocation.expected_revision != s.journal.current.revision {
                        return Err(StoreStatus::Conflict);
                    }
                    let binding = row.binding.as_ref().unwrap().clone();
                    if s.next_writer > s.journal.writer_high_water || s.force_writer_range {
                        s.journal.writer_high_water =
                            match s.journal.writer_high_water.checked_add(16) {
                                Some(v) => v,
                                None => {
                                    s.mutation_fenced = true;
                                    close_operation(&mut s, operation, EditorClosureV0::Deadline);
                                    return Err(StoreStatus::Exhausted);
                                }
                            };
                        s.next_writer = s.journal.writer_high_water - 15;
                        s.force_writer_range = false;
                    }
                    let writer = s.next_writer;
                    s.next_writer = s.next_writer.checked_add(1).ok_or(StoreStatus::Exhausted)?;
                    let admission = s
                        .admission_epoch
                        .checked_add(1)
                        .ok_or(StoreStatus::Exhausted)?;
                    let permit = match EditorCommitPermitV0::try_new(
                        binding,
                        s.journal.service_epoch,
                        writer,
                        admission,
                    ) {
                        Ok(p) => p,
                        Err(_) => {
                            s.mutation_fenced = true;
                            close_operation(&mut s, operation, EditorClosureV0::Deadline);
                            return Err(StoreStatus::Exhausted);
                        }
                    };
                    s.admission_epoch = admission;
                    s.permits += 1;
                    s.quarantine = true;
                    s.runtime.get_mut(&operation).unwrap().permitted = Some(permit.clone());
                    records(&mut s, |rows| {
                        rows[index].permit = Some(permit.clone());
                        rows[index].state = EditorOperationStateV0::Permitted;
                    })?;
                    StoreCommitPermit {
                        evidence: permit,
                        ticket: r.clone(),
                        source: source.clone(),
                        object: context.object.clone(),
                        worker: s.io_worker.ok_or(StoreStatus::Internal)?,
                        bytes,
                    }
                };
                permit.check_reservation(&context)?;
                context.writer = permit.evidence.writer_id;
                persist(&context, false)?;
                storage::Observer::pause(&context, PausePoint::AfterCommitPermit)?;
                storage::Observer::pause(&context, PausePoint::BeforeCandidateWrite)?;
                let journal = lock(&context.object.state).journal.clone();
                let consumed = storage::publish(
                    &context.object.root.join("cas"),
                    &core.profile,
                    &journal,
                    &permit.bytes,
                    core.profile_hash,
                    None,
                    &context,
                )?;
                storage::Observer::pause(&context, PausePoint::AfterCandidatePublication)?;
                let selected = schema(SelectedArtifactV0::try_new(
                    permit.evidence.binding.allocation.actor.owner_id,
                    context.object.id,
                    permit.evidence.binding.allocation.selected_generation,
                    permit.evidence.successor_revision,
                    permit.evidence.binding.source.content_hash,
                    permit.evidence.binding.source.byte_len,
                    consumed.manifest_hash,
                ))?;
                let receipt = schema(EditorSaveReceiptV0::committed(
                    &permit.evidence,
                    core.clock.load(Ordering::Acquire),
                ))?;
                let mut successor = {
                    let s = lock(&context.object.state);
                    next_journal(&s)?
                };
                let mut rows = successor.operations.as_slice().to_vec();
                let index = rows
                    .iter()
                    .position(|row| row.allocation.operation_id == operation)
                    .unwrap();
                rows[index].state = EditorOperationStateV0::Committed;
                rows[index].successor = Some(selected.clone());
                rows[index].receipt = Some(receipt.clone());
                successor.current = selected;
                successor.operations = schema(BoundedVec::try_from_vec(rows))?;
                context.selection = true;
                let digest =
                    storage::publish_journal(&context.object.root, &successor, &context, true)?;
                {
                    let mut s = lock(&context.object.state);
                    install_journal(&mut s, successor, digest);
                    s.bytes = consumed.bytes;
                    s.owner_hash = consumed.owner_hash;
                    s.quarantine = false;
                }
                storage::Observer::pause(&context, PausePoint::AfterAcknowledgement)?;
                Ok(receipt)
            },
        );
        match outcome {
            Ok(receipt) => {
                match self.receipt_reply(&request.grant, &request.wire, &receipt, Some(request)) {
                    Ok(reply) => Ok(reply),
                    Err(_) => Ok(unknown(&request.wire, operation)),
                }
            }
            Err(error) => {
                let mut s = lock(&object.state);
                if s.runtime[&operation].permitted.is_some() {
                    s.quarantine = true;
                    Ok(unknown(&request.wire, operation))
                } else {
                    close_operation(&mut s, operation, EditorClosureV0::Deadline);
                    Err(error)
                }
            }
        }
    }
}
impl StoreFixture {
    pub fn create(
        root: &std::path::Path,
        profile: FixtureProfile,
    ) -> Result<(StoreHost, FixtureController), FixtureError> {
        fixture(Self::create_inner(root, profile))
    }
    fn create_inner(
        root: &std::path::Path,
        profile: FixtureProfile,
    ) -> Result<(StoreHost, FixtureController), StoreStatus> {
        let core = Self::prepare_core(root, profile, false)?;
        Self::provision_core(&core)?;
        Ok((StoreHost { core: core.clone() }, FixtureController { core }))
    }
    // Pure preparation for the native path: it creates the actual owner before
    // any filesystem effect. Legacy preparation retains its existing preflight.
    fn prepare_core(
        root: &std::path::Path,
        profile: FixtureProfile,
        native_only: bool,
    ) -> Result<Arc<Core>, StoreStatus> {
        if !root.is_absolute()
            || root.as_os_str().len() > 4096
            || profile.origin_ms == 0
            || profile.initial_objects.is_empty()
            || profile.initial_objects.len() > 2
            || profile.fixture_key_id.is_empty()
            || profile.fixture_key_id.len() > 64
            || !profile
                .fixture_key_id
                .bytes()
                .all(|b| (32..=126).contains(&b))
        {
            return Err(StoreStatus::Invalid);
        }
        let mut initial = BTreeMap::new();
        for object in &profile.initial_objects {
            schema(EditorActorV0::try_new(object.owner_id, 1, 1, 1, 1))?;
            schema(EditorTextHeaderV0::try_new(
                object.object_id,
                object.revision,
                &object.bytes,
            ))?;
            if object.generation == 0 || initial.contains_key(&object.object_id) {
                return Err(StoreStatus::Invalid);
            }
            let manifest = storage::make_manifest(&profile, &object.bytes)?;
            let selected = schema(SelectedArtifactV0::try_new(
                object.owner_id,
                object.object_id,
                object.generation,
                object.revision,
                storage::hash(&object.bytes),
                object.bytes.len() as u32,
                storage::hash(&serde_json::to_vec(&manifest).map_err(|_| StoreStatus::Internal)?),
            ))?;
            initial.insert(object.object_id, selected);
        }
        let root = if native_only {
            root.to_path_buf()
        } else {
            if root.try_exists().map_err(|_| StoreStatus::Invalid)? {
                return Err(StoreStatus::Invalid);
            }
            let parent = root
                .parent()
                .ok_or(StoreStatus::Invalid)?
                .canonicalize()
                .map_err(|_| StoreStatus::Invalid)?;
            parent.join(root.file_name().ok_or(StoreStatus::Invalid)?)
        };
        let profile_hash = storage::hash(
            &serde_json::to_vec(&(
                &initial,
                profile.origin_ms,
                &profile.fixture_key_id,
                storage::hash(&profile.fixture_signing_seed),
            ))
            .map_err(|_| StoreStatus::Internal)?,
        );
        let policy_hash = storage::hash(
            &serde_json::to_vec(&(
                1u32,
                "RequireSignature",
                "editor-selected-private",
                "dev-host-editor",
                &profile.fixture_key_id,
                ed25519_dalek::SigningKey::from_bytes(&profile.fixture_signing_seed)
                    .verifying_key()
                    .to_bytes(),
            ))
            .map_err(|_| StoreStatus::Internal)?,
        );
        let mut objects = BTreeMap::new();
        for (id, selected) in initial {
            let journal = schema(EditorSelectionJournalV0::try_new(selected, policy_hash))?;
            let digest = schema(journal.digest())?;
            objects.insert(
                id,
                Arc::new(Object {
                    id,
                    root: storage::fixed_root(&root, id),
                    state: Mutex::new(ObjectState {
                        #[cfg(feature = "editor_native_preview_v0_dev")]
                        preview_faulted: false,
                        journal: journal.clone(),
                        durable: journal,
                        prepared: None,
                        synced: None,
                        digest,
                        bytes: vec![],
                        owner_hash: [0; 32],
                        next_op: 1,
                        next_writer: 1,
                        force_op_range: false,
                        force_writer_range: false,
                        admission_epoch: 1,
                        allocation_fenced: false,
                        mutation_fenced: false,
                        io_worker: None,
                        runtime: BTreeMap::new(),
                        dispatches: 0,
                        permits: 0,
                        transitions: 0,
                        quarantine: false,
                        fault: None,
                        reopen: None,
                    }),
                }),
            );
        }
        let core = Arc::new(Core {
            native_only,
            #[cfg(feature = "editor_native_read_v0_dev")]
            native_read: Mutex::new(None),
            #[cfg(feature = "editor_native_preview_v0_dev")]
            native_preview: Mutex::new(None),
            root,
            profile,
            profile_hash,
            namespace: rand::random::<u64>().max(1),
            clock: AtomicU64::new(1),
            closed: AtomicBool::new(false),
            setup_active: AtomicBool::new(false),
            fence_issued: AtomicBool::new(false),
            quiescing: AtomicBool::new(false),
            registry: Mutex::new(Registry {
                next_identity: 1,
                grants: BTreeMap::new(),
                data: BTreeMap::new(),
                sessions: BTreeMap::new(),
                instances: BTreeMap::new(),
            }),
            objects,
            diagnostics: Mutex::new(Diagnostics {
                exchanges: vec![],
                io: vec![],
                io_sequence: 0,
                reserved: 0,
                exchange_reservations: 0,
                exhausted: false,
            }),
            owned: Mutex::new(Owned {
                next: 1,
                handles: BTreeMap::new(),
            }),
            owned_changed: Condvar::new(),
            barriers: Mutex::new(vec![]),
        });
        core.clock.store(core.profile.origin_ms, Ordering::Release);
        Ok(core)
    }
    fn provision_core(core: &Arc<Core>) -> Result<(), StoreStatus> {
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&core.root)
            .map_err(|_| StoreStatus::Unknown)?;
        Self::provision_objects(core)
    }
    fn provision_objects(core: &Arc<Core>) -> Result<(), StoreStatus> {
        for object in core.objects.values() {
            fs::DirBuilder::new()
                .mode(0o700)
                .create(&object.root)
                .map_err(|_| StoreStatus::Unknown)?;
            fs::DirBuilder::new()
                .mode(0o700)
                .create(object.root.join("cas"))
                .map_err(|_| StoreStatus::Unknown)?;
            let context = Context {
                core: core.clone(),
                object: object.clone(),
                request: None,
                operation: 0,
                writer: 0,
                selection: false,
            };
            let raw = core
                .profile
                .initial_objects
                .iter()
                .find(|o| o.object_id == object.id)
                .unwrap();
            let journal = lock(&object.state).journal.clone();
            let selected = &journal.initial;
            let provisioning = ProvisioningBinding {
                owner: selected.owner_id,
                object: selected.selected_object_id,
                generation: selected.selected_generation,
                initial_revision: selected.revision,
                blob_hash: selected.content_hash,
                byte_len: selected.byte_len,
                manifest_hash: selected.manifest_hash,
                profile_hash: core.profile_hash,
            };
            let consumed = storage::publish(
                &object.root.join("cas"),
                &core.profile,
                &journal,
                &raw.bytes,
                core.profile_hash,
                Some(provisioning),
                &context,
            )?;
            let digest = storage::publish_journal(&object.root, &journal, &context, false)?;
            {
                let mut s = lock(&object.state);
                s.bytes = consumed.bytes;
                s.owner_hash = consumed.owner_hash;
                install_journal(&mut s, journal, digest);
                s.journal.operation_high_water = 16;
                s.journal.writer_high_water = 16;
            }
            persist(&context, false)?;
        }
        Ok(())
    }
    pub fn reopen(
        owner: QuiescedStoreOwner,
    ) -> Result<(StoreHost, FixtureController), ReopenFailure> {
        match Self::reopen_inner(&owner) {
            Ok(pair) => Ok(pair),
            Err(status) => Err(ReopenFailure {
                error: FixtureError::new(status),
                owner,
            }),
        }
    }
    fn reopen_inner(
        owner: &QuiescedStoreOwner,
    ) -> Result<(StoreHost, FixtureController), StoreStatus> {
        let old = &owner.core;
        if old.native_only {
            return Err(StoreStatus::Denied);
        }
        if !old.closed.load(Ordering::Acquire) || !lock(&old.owned).handles.is_empty() {
            return Err(StoreStatus::NotReady);
        }
        let identity = owner
            .snapshot
            .identity_counter_min
            .checked_add(1)
            .ok_or(StoreStatus::Exhausted)?;
        let fence_hash =
            storage::hash(&serde_json::to_vec(&owner.snapshot).map_err(|_| StoreStatus::Internal)?);
        let mut validated = BTreeMap::new();
        for object in old.objects.values() {
            let s = lock(&object.state);
            let runtime = s.journal.clone();
            let durable = s
                .synced
                .as_ref()
                .filter(|j| j.transition_sequence >= s.durable.transition_sequence)
                .unwrap_or(&s.durable)
                .clone();
            let expected = schema(durable.digest())?;
            let permits = s
                .runtime
                .values()
                .filter_map(|r| r.permitted.clone())
                .collect::<Vec<_>>();
            drop(s);
            let metadata = fs::symlink_metadata(&object.root).map_err(|_| StoreStatus::Unknown)?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(StoreStatus::Unknown);
            }
            let journal = storage::read_journal(&object.root)?;
            if journal.owner_id != durable.owner_id
                || journal.selected_object_id != object.id
                || journal.selected_generation != durable.selected_generation
                || journal.initial != durable.initial
                || journal.signing_policy_hash != durable.signing_policy_hash
                || journal.service_epoch < durable.service_epoch
                || journal.operation_high_water < runtime.operation_high_water
                || journal.writer_high_water < runtime.writer_high_water
                || journal.transition_sequence < durable.transition_sequence
                || (journal.transition_sequence == durable.transition_sequence
                    && schema(journal.digest())? != expected)
            {
                return Err(StoreStatus::Unknown);
            }
            for known in durable.operations.as_slice() {
                let got = journal
                    .operations
                    .as_slice()
                    .iter()
                    .find(|r| r.allocation.operation_id == known.allocation.operation_id)
                    .ok_or(StoreStatus::Unknown)?;
                if got.allocation != known.allocation
                    || got.allocated_service_epoch != known.allocated_service_epoch
                    || known
                        .binding
                        .as_ref()
                        .is_some_and(|b| got.binding.as_ref() != Some(b))
                    || known.submitted_service_epoch.is_some()
                        && got.submitted_service_epoch != known.submitted_service_epoch
                    || known
                        .permit
                        .as_ref()
                        .is_some_and(|p| got.permit.as_ref() != Some(p))
                    || known
                        .receipt
                        .as_ref()
                        .is_some_and(|r| got.receipt.as_ref() != Some(r))
                    || known
                        .successor
                        .as_ref()
                        .is_some_and(|r| got.successor.as_ref() != Some(r))
                {
                    return Err(StoreStatus::Unknown);
                }
            }
            for row in journal.operations.as_slice() {
                let known = runtime
                    .operations
                    .as_slice()
                    .iter()
                    .find(|r| r.allocation.operation_id == row.allocation.operation_id)
                    .ok_or(StoreStatus::Unknown)?;
                if row.allocation != known.allocation
                    || row
                        .binding
                        .as_ref()
                        .is_some_and(|b| known.binding.as_ref() != Some(b))
                    || row.permit.as_ref().is_some_and(|p| !permits.contains(p))
                {
                    return Err(StoreStatus::Unknown);
                }
            }
            storage::preflight(&object.root.join("cas"), &old.profile, &journal, &permits)?;
            let consumed =
                storage::consume(&object.root.join("cas"), &old.profile, &journal.current)?;
            let epoch = owner
                .snapshot
                .objects
                .iter()
                .find(|o| o.selected_object_id == object.id)
                .unwrap()
                .reserved_successor_epoch
                .ok_or(StoreStatus::Exhausted)?;
            if journal.service_epoch > epoch {
                return Err(StoreStatus::Unknown);
            }
            let op_high = journal
                .operation_high_water
                .checked_add(16)
                .ok_or(StoreStatus::Exhausted)?;
            let writer_high = journal
                .writer_high_water
                .checked_add(16)
                .ok_or(StoreStatus::Exhausted)?;
            journal
                .transition_sequence
                .checked_add(1)
                .ok_or(StoreStatus::Exhausted)?;
            validated.insert(
                object.id,
                (journal, runtime, consumed, epoch, op_high, writer_high),
            );
        }
        let mut objects = BTreeMap::new();
        for (id, (mut journal, runtime, consumed, epoch, op_high, writer_high)) in validated {
            let old_object = &old.objects[&id];
            let old_state = lock(&old_object.state);
            let durable = journal.clone();
            let digest = schema(journal.digest())?;
            let mut rows = journal.operations.as_slice().to_vec();
            for row in &mut rows {
                if let Some(known) = runtime
                    .operations
                    .as_slice()
                    .iter()
                    .find(|r| r.allocation.operation_id == row.allocation.operation_id)
                {
                    if row.binding.is_none() && known.binding.is_some() {
                        row.binding = known.binding.clone();
                        row.submitted_service_epoch = known.submitted_service_epoch;
                        row.state = EditorOperationStateV0::Submitted;
                    }
                    if row.permit.is_none() && known.permit.is_some() {
                        row.permit = known.permit.clone();
                        row.state = EditorOperationStateV0::Permitted;
                    }
                    if known.closure != EditorClosureV0::None {
                        row.closure = known.closure;
                    }
                }
                if matches!(
                    row.state,
                    EditorOperationStateV0::Submitted | EditorOperationStateV0::Permitted
                ) {
                    row.state = EditorOperationStateV0::Noncommit;
                    row.closure = if row.closure == EditorClosureV0::None {
                        EditorClosureV0::ServiceRetired
                    } else {
                        row.closure
                    };
                    row.receipt = Some(schema(EditorSaveReceiptV0::noncommit(
                        row.binding.as_ref().unwrap(),
                        row.submitted_service_epoch.unwrap(),
                    ))?);
                    row.successor = None;
                }
            }
            journal.operations = schema(BoundedVec::try_from_vec(rows))?;
            journal.service_epoch = epoch;
            journal.operation_high_water = op_high;
            journal.writer_high_water = writer_high;
            let runtime_ops = journal
                .operations
                .as_slice()
                .iter()
                .map(|r| {
                    (
                        r.allocation.operation_id,
                        RuntimeOperation {
                            original: None,
                            source: None,
                            permitted: r.permit.clone(),
                            dispatched: old_state
                                .runtime
                                .get(&r.allocation.operation_id)
                                .is_some_and(|r| r.dispatched),
                        },
                    )
                })
                .collect();
            objects.insert(
                id,
                Arc::new(Object {
                    id,
                    root: old_object.root.clone(),
                    state: Mutex::new(ObjectState {
                        #[cfg(feature = "editor_native_preview_v0_dev")]
                        preview_faulted: false,
                        journal,
                        durable,
                        prepared: None,
                        synced: None,
                        digest,
                        bytes: consumed.bytes,
                        owner_hash: consumed.owner_hash,
                        next_op: op_high - 15,
                        next_writer: writer_high - 15,
                        force_op_range: false,
                        force_writer_range: false,
                        admission_epoch: old_state.admission_epoch,
                        allocation_fenced: old_state.allocation_fenced,
                        mutation_fenced: old_state.mutation_fenced,
                        io_worker: None,
                        runtime: runtime_ops,
                        dispatches: old_state.dispatches,
                        permits: old_state.permits,
                        transitions: old_state.transitions,
                        quarantine: false,
                        fault: None,
                        reopen: None,
                    }),
                }),
            );
        }
        let old_registry = lock(&old.registry);
        let core = Arc::new(Core {
            native_only: false,
            #[cfg(feature = "editor_native_read_v0_dev")]
            native_read: Mutex::new(None),
            #[cfg(feature = "editor_native_preview_v0_dev")]
            native_preview: Mutex::new(None),
            root: old.root.clone(),
            profile: old.profile.clone(),
            profile_hash: old.profile_hash,
            namespace: old.namespace,
            clock: AtomicU64::new(old.clock.load(Ordering::Acquire)),
            closed: AtomicBool::new(false),
            setup_active: AtomicBool::new(false),
            fence_issued: AtomicBool::new(false),
            quiescing: AtomicBool::new(false),
            registry: Mutex::new(Registry {
                next_identity: identity,
                grants: BTreeMap::new(),
                data: BTreeMap::new(),
                sessions: old_registry.sessions.clone(),
                instances: old_registry.instances.clone(),
            }),
            objects,
            diagnostics: Mutex::new(Diagnostics {
                exchanges: vec![],
                io: vec![],
                io_sequence: 0,
                reserved: 0,
                exchange_reservations: 0,
                exhausted: false,
            }),
            owned: Mutex::new(Owned {
                next: lock(&old.owned).next,
                handles: BTreeMap::new(),
            }),
            owned_changed: Condvar::new(),
            barriers: Mutex::new(vec![]),
        });
        drop(old_registry);
        // All roots and metadata passed preflight before ANY auto-recovery helper below.
        for object in core.objects.values() {
            crate::domain_visibility::DomainArtifactRegistry::new(&object.root.join("cas"))
                .map_err(|_| StoreStatus::Unknown)?;
            let context = Context {
                core: core.clone(),
                object: object.clone(),
                request: None,
                operation: 0,
                writer: 0,
                selection: false,
            };
            persist(&context, false)?;
            let mut s = lock(&object.state);
            let prior = owner
                .snapshot
                .objects
                .iter()
                .find(|o| o.selected_object_id == object.id)
                .unwrap()
                .service_epoch_at_fence;
            s.reopen = Some(ReopenWitness {
                fence_id: owner.snapshot.fence_id,
                fence_snapshot_hash: fence_hash,
                selected_object_id: object.id,
                selected_generation: s.journal.selected_generation,
                prior_service_epoch: prior,
                actual_service_epoch: s.journal.service_epoch,
                reopened_journal_sequence: s.durable.transition_sequence,
                reopened_journal_hash: s.digest,
            });
        }
        Ok((StoreHost { core: core.clone() }, FixtureController { core }))
    }
}
impl FixtureController {
    pub fn issue_editor(&self, spec: GrantSpec) -> Result<EditorEndpoint, FixtureError> {
        if self.core.native_only {
            return Err(FixtureError::new(StoreStatus::Denied));
        }
        fixture(self.issue(spec, EndpointClass::Artifact, None))
    }
    fn issue(
        &self,
        spec: GrantSpec,
        class: EndpointClass,
        recovery_op: Option<u64>,
    ) -> Result<EditorEndpoint, StoreStatus> {
        if self.core.native_only {
            return Err(StoreStatus::Denied);
        }
        schema(spec.actor.validate())?;
        if spec.rights == 0 || spec.rights & !7 != 0 || spec.ttl_ms == 0 || spec.ttl_ms > 600000 {
            return Err(StoreStatus::Invalid);
        }
        let object = self.core.object(spec.selected_object_id)?;
        let mut reg = lock(&self.core.registry);
        let s = lock(&object.state);
        if self.core.closed.load(Ordering::Acquire) {
            return Err(StoreStatus::Stale);
        }
        if self.core.setup_active.load(Ordering::Acquire) {
            return Err(StoreStatus::NotReady);
        }
        if spec.actor.owner_id != s.journal.owner_id
            || spec.selected_generation != s.journal.selected_generation
        {
            return Err(StoreStatus::Denied);
        }
        let session = (spec.actor.session_id, spec.actor.session_generation);
        let key = instance_key(&spec.actor);
        if reg
            .sessions
            .get(&session)
            .is_some_and(|owner| *owner != spec.actor.owner_id)
            || reg
                .instances
                .get(&key)
                .is_some_and(|owner| *owner != spec.actor.owner_id)
        {
            return Err(StoreStatus::Denied);
        }
        if !reg.sessions.contains_key(&session) && reg.sessions.len() >= 2
            || !reg.instances.contains_key(&key) && reg.instances.len() >= 16
            || reg.grants.len() >= 64
        {
            return Err(StoreStatus::Exhausted);
        }
        let expires = self
            .core
            .clock
            .load(Ordering::Acquire)
            .checked_add(spec.ttl_ms)
            .ok_or(StoreStatus::Exhausted)?;
        let handle = Core::new_handle(&mut reg, HandleKind::Ipc)?;
        let grant = Arc::new(Grant {
            #[cfg(feature = "editor_native_read_v0_dev")]
            native_read: None,
            handle,
            actor: spec.actor.clone(),
            object: object.id,
            rights: spec.rights,
            class,
            recovery_op,
            epoch: s.journal.service_epoch,
            expires,
            alive: AtomicBool::new(true),
            invocation: AtomicBool::new(false),
            drop_reply: AtomicBool::new(false),
        });
        reg.sessions.insert(session, spec.actor.owner_id);
        reg.instances.insert(key, spec.actor.owner_id);
        reg.grants.insert(handle.pack(), grant.clone());
        Ok(EditorEndpoint {
            peer: EditorPeer {
                core: self.core.clone(),
                grant,
            },
            handle,
        })
    }
    pub fn issue_recovery(
        &self,
        actor: EditorActorV0,
        object_id: u64,
        operation_id: u64,
    ) -> Result<EditorEndpoint, FixtureError> {
        if self.core.native_only {
            return Err(FixtureError::new(StoreStatus::Denied));
        }
        let object = fixture(self.core.object(object_id))?;
        let generation = {
            let s = lock(&object.state);
            let index = fixture(record_index(&s, operation_id))?;
            if s.journal.operations.as_slice()[index].allocation.actor != actor {
                return Err(FixtureError::new(StoreStatus::Denied));
            }
            s.journal.selected_generation
        };
        fixture(self.issue(
            GrantSpec {
                actor,
                selected_object_id: object_id,
                selected_generation: generation,
                rights: 4,
                ttl_ms: 600000,
            },
            EndpointClass::RecoveryReceipt,
            Some(operation_id),
        ))
    }
    pub fn advance_time(&self, observed_ms: u64) -> Result<u64, FixtureError> {
        if self.core.closed.load(Ordering::Acquire) {
            return Err(FixtureError::new(StoreStatus::Stale));
        }
        let objects = self.core.objects.values().collect::<Vec<_>>();
        let mut states = objects.iter().map(|o| lock(&o.state)).collect::<Vec<_>>();
        let now = self.core.clock.load(Ordering::Acquire).max(observed_ms);
        self.core.clock.store(now, Ordering::Release);
        for s in &mut states {
            let expired = s
                .runtime
                .iter()
                .filter(|(_, r)| r.original.as_ref().is_some_and(|r| now >= r.grant.expires))
                .map(|(op, _)| *op)
                .collect::<Vec<_>>();
            for op in expired {
                close_operation(s, op, EditorClosureV0::Expired);
            }
        }
        Ok(now)
    }
    pub fn retire(
        &self,
        endpoint: &EditorEndpoint,
        reason: RetirementReason,
    ) -> Result<(), FixtureError> {
        if !Arc::ptr_eq(&self.core, &endpoint.peer.core) {
            return Err(FixtureError::new(StoreStatus::Denied));
        }
        let object = fixture(self.core.object(endpoint.peer.grant.object))?;
        let mut s = lock(&object.state);
        endpoint.peer.grant.alive.store(false, Ordering::Release);
        let operations = s
            .runtime
            .iter()
            .filter(|(_, rt)| {
                rt.original
                    .as_ref()
                    .is_some_and(|r| r.grant.handle == endpoint.handle)
            })
            .map(|(op, _)| *op)
            .collect::<Vec<_>>();
        for op in operations {
            close_operation(
                &mut s,
                op,
                match reason {
                    RetirementReason::Revoked => EditorClosureV0::Revoked,
                    RetirementReason::ServiceRetired => EditorClosureV0::ServiceRetired,
                },
            );
        }
        let allocated = s
            .journal
            .operations
            .as_slice()
            .iter()
            .filter(|r| {
                r.allocation.actor == endpoint.peer.grant.actor
                    && r.state == EditorOperationStateV0::Allocated
            })
            .map(|r| r.allocation.operation_id)
            .collect::<Vec<_>>();
        for op in allocated {
            close_operation(&mut s, op, EditorClosureV0::Revoked);
        }
        Ok(())
    }
    pub fn pause_next(
        &self,
        endpoint: &EditorEndpoint,
        point: PausePoint,
    ) -> Result<PauseToken, FixtureError> {
        if !Arc::ptr_eq(&self.core, &endpoint.peer.core) {
            return Err(FixtureError::new(StoreStatus::Denied));
        }
        if self.core.native_only && point != PausePoint::BeforeReadReply {
            return Err(FixtureError::new(StoreStatus::Unsupported));
        }
        let object = fixture(self.core.object(endpoint.peer.grant.object))?;
        {
            let s = lock(&object.state);
            fixture(auth(&self.core, &endpoint.peer.grant, &s))?;
        }
        let mut barriers = lock(&self.core.barriers);
        if barriers.len() >= 64 {
            return Err(FixtureError::new(StoreStatus::Exhausted));
        }
        if barriers
            .iter()
            .any(|b| b.object == object.id && !lock(&b.state).snapshot.settled)
        {
            return Err(FixtureError::new(StoreStatus::NotReady));
        }
        let barrier = Arc::new(Barrier {
            object: object.id,
            endpoint: endpoint.handle.pack(),
            point,
            state: Mutex::new(BarrierState {
                snapshot: BarrierSnapshot {
                    request_id: 0,
                    operation_id: 0,
                    entered: false,
                    released: false,
                    settled: false,
                    permit_issued: false,
                },
                stop: false,
            }),
            changed: Condvar::new(),
        });
        barriers.push(barrier.clone());
        Ok(PauseToken {
            core: self.core.clone(),
            barrier,
        })
    }
    pub fn release(&self, token: &PauseToken) -> Result<(), FixtureError> {
        self.signal(token, false)
    }
    pub fn stop_at_phase(&self, token: &PauseToken) -> Result<(), FixtureError> {
        self.signal(token, true)
    }
    fn signal(&self, token: &PauseToken, stop: bool) -> Result<(), FixtureError> {
        if !Arc::ptr_eq(&self.core, &token.core) {
            return Err(FixtureError::new(StoreStatus::Denied));
        }
        let mut s = lock(&token.barrier.state);
        if !s.snapshot.entered || s.snapshot.settled {
            return Err(FixtureError::new(StoreStatus::Stale));
        }
        s.snapshot.released = true;
        s.stop = stop;
        token.barrier.changed.notify_all();
        Ok(())
    }
    pub fn drop_next_reply(&self, endpoint: &EditorEndpoint) -> Result<(), FixtureError> {
        if self.core.closed.load(Ordering::Acquire) {
            return Err(FixtureError::new(StoreStatus::Stale));
        }
        if !Arc::ptr_eq(&self.core, &endpoint.peer.core) {
            return Err(FixtureError::new(StoreStatus::Denied));
        }
        endpoint
            .peer
            .grant
            .drop_reply
            .store(true, Ordering::Release);
        Ok(())
    }
    pub fn fail_next_io(
        &self,
        endpoint: &EditorEndpoint,
        point: IoPoint,
    ) -> Result<(), FixtureError> {
        if !Arc::ptr_eq(&self.core, &endpoint.peer.core) {
            return Err(FixtureError::new(StoreStatus::Denied));
        }
        let object = fixture(self.core.object(endpoint.peer.grant.object))?;
        let mut s = lock(&object.state);
        if self.core.closed.load(Ordering::Acquire) {
            return Err(FixtureError::new(StoreStatus::Stale));
        }
        if s.fault.is_some() {
            return Err(FixtureError::new(StoreStatus::NotReady));
        }
        s.fault = Some(point);
        Ok(())
    }
    pub fn seed_counter(&self, object_id: u64, counter: CounterKind) -> Result<(), FixtureError> {
        fixture(self.seed_inner(object_id, counter))
    }
    fn seed_inner(&self, object_id: u64, counter: CounterKind) -> Result<(), StoreStatus> {
        let object = self.core.object(object_id)?;
        let mut reg = lock(&self.core.registry);
        let mut s = lock(&object.state);
        if self.core.closed.load(Ordering::Acquire) {
            return Err(StoreStatus::Stale);
        }
        if self.core.setup_active.swap(true, Ordering::AcqRel) {
            return Err(StoreStatus::NotReady);
        }
        let _setup = SetupGuard(&self.core.setup_active);
        if !reg.grants.is_empty()
            || self.core.objects.values().any(|o| {
                o.id != object_id && !lock(&o.state).journal.operations.as_slice().is_empty()
            })
            || !s.journal.operations.is_empty()
            || s.io_worker.is_some()
        {
            return Err(StoreStatus::NotReady);
        }
        match counter {
            CounterKind::OperationHighWater => {
                s.journal.operation_high_water = u64::MAX;
                s.force_op_range = true;
            }
            CounterKind::WriterHighWater => {
                s.journal.writer_high_water = u64::MAX;
                s.force_writer_range = true;
            }
            CounterKind::ServiceEpoch => s.journal.service_epoch = u64::MAX,
            CounterKind::JournalSequence => s.journal.transition_sequence = u64::MAX,
            CounterKind::Identity => reg.next_identity = u64::MAX,
            CounterKind::TimeOrigin => self.core.clock.store(u64::MAX, Ordering::Release),
        }
        let mut journal = s.journal.clone();
        if counter != CounterKind::JournalSequence {
            journal.transition_sequence = s
                .durable
                .transition_sequence
                .checked_add(1)
                .ok_or(StoreStatus::Exhausted)?;
        }
        journal.prior_journal_hash = s.digest;
        drop(s);
        drop(reg);
        let context = Context {
            core: self.core.clone(),
            object: object.clone(),
            request: None,
            operation: 0,
            writer: 0,
            selection: false,
        };
        let digest = storage::publish_journal(&object.root, &journal, &context, false)?;
        install_journal(&mut lock(&object.state), journal, digest);
        Ok(())
    }
    pub fn evidence(&self) -> Result<StoreEvidence, FixtureError> {
        // Bounded per-component live observation, not an atomic filesystem/state
        // snapshot. After joined quiescence these components are stable. Copy
        // admission/registry diagnostics briefly; NEVER hold those locks for IO.
        let (sessions, instances, endpoints, shared_objects) = {
            let reg = lock(&self.core.registry);
            (
                reg.sessions.len() as u32,
                reg.instances.len() as u32,
                reg.grants.len() as u32,
                reg.data.len() as u32,
            )
        };
        let (held, active) = {
            let owned = lock(&self.core.owned);
            (
                owned
                    .handles
                    .values()
                    .filter(|p| !matches!(p.kind, ProducerKind::ObjectIoWorker))
                    .count() as u32,
                owned
                    .handles
                    .values()
                    .filter(|p| {
                        !matches!(p.kind, ProducerKind::ObjectIoWorker)
                            && p.handle.as_ref().is_none_or(|h| !h.is_finished())
                    })
                    .count() as u32,
            )
        };
        let mut objects = Vec::new();
        let mut operation_counts = Vec::new();
        let mut active_io = 0;
        for object in self.core.objects.values() {
            let s = lock(&object.state);
            operation_counts.push((object.id, s.journal.operations.len() as u32));
            active_io += u32::from(s.io_worker.is_some());
            objects.push(ObjectEvidence {
                selected: s.journal.current.clone(),
                operations: s.journal.operations.as_slice().to_vec(),
                mutation_dispatch_count: s.dispatches,
                permit_count: s.permits,
                transition_count: s.transitions,
                journal_hash: s.digest,
                journal_sequence: s.durable.transition_sequence,
                blob_hash: s.journal.current.content_hash,
                manifest_hash: s.journal.current.manifest_hash,
                ownership_hash: s.owner_hash,
                mutation_quarantined: s.quarantine,
                pending_writer_id: if s.io_worker.is_some() {
                    s.runtime
                        .values()
                        .filter_map(|r| r.permitted.as_ref())
                        .map(|p| p.writer_id)
                        .max()
                } else {
                    None
                },
                service_epoch: s.journal.service_epoch,
                reopen_witness: s.reopen.clone(),
            });
        }
        let (exchanges, io_events) = {
            let d = lock(&self.core.diagnostics);
            (d.exchanges.clone(), d.io.clone())
        };
        let barriers = lock(&self.core.barriers)
            .iter()
            .map(|b| lock(&b.state).snapshot.clone())
            .collect();
        let mut entry_counts = Vec::new();
        let mut byte_counts = Vec::new();
        for object in self.core.objects.values() {
            let (count, bytes) = fixture(storage::diagnostic_inventory(&object.root.join("cas")))?;
            entry_counts.push((object.id, count));
            byte_counts.push((object.id, bytes));
        }
        let ledger = StoreLedger {
            sessions,
            selected_objects: objects.len() as u32,
            original_instances: instances,
            endpoints,
            shared_objects,
            active_io_workers: active_io,
            active_supervisors: active,
            held_service_producers: held,
            operations_per_object: operation_counts,
            cas_entries_per_object: entry_counts,
            bytes_per_object: byte_counts,
        };
        Ok(StoreEvidence {
            exchanges,
            objects,
            barriers,
            io_events,
            ledger,
        })
    }
    pub fn quiesce(&self, timeout_ms: u64) -> Result<QuiescedStoreOwner, FixtureError> {
        // Native handles live in the shared Desktop roster, not legacy Owned.
        // This fixture cannot certify their quiescence with a legacy fence.
        if self.core.native_only {
            return Err(FixtureError::new(StoreStatus::Denied));
        }
        if !(1..=2000).contains(&timeout_ms) {
            return Err(FixtureError::new(StoreStatus::Invalid));
        }
        // This owner can issue exactly one successful fence. A timeout keeps
        // its handles charged and permits a later retry, but not a concurrent
        // second issuer. Failed reopen returns this same token; it never clears
        // the old owner's sticky issued latch.
        if self.core.fence_issued.load(Ordering::Acquire)
            || self
                .core
                .quiescing
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return Err(FixtureError::new(StoreStatus::NotReady));
        }
        let _quiescing = SetupGuard(&self.core.quiescing);
        if self.core.fence_issued.load(Ordering::Acquire) {
            return Err(FixtureError::new(StoreStatus::NotReady));
        }
        let until = Instant::now() + Duration::from_millis(timeout_ms);
        {
            let objects = self.core.objects.values().collect::<Vec<_>>();
            let mut states = objects.iter().map(|o| lock(&o.state)).collect::<Vec<_>>();
            if self.core.setup_active.load(Ordering::Acquire) {
                return Err(FixtureError::new(StoreStatus::NotReady));
            }
            self.core.closed.store(true, Ordering::Release);
            for s in &mut states {
                let operations = s.runtime.keys().copied().collect::<Vec<_>>();
                for operation in operations {
                    close_operation(s, operation, EditorClosureV0::ServiceRetired);
                }
            }
        }
        let mut owned = lock(&self.core.owned);
        loop {
            if owned.handles.values().all(|p| {
                !p.joining
                    && (p.terminal_join.is_some()
                        || p.handle.as_ref().is_some_and(JoinHandle::is_finished))
            }) {
                break;
            }
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(FixtureError::new(StoreStatus::NotReady));
            }
            let (guard, _) = self
                .core
                .owned_changed
                .wait_timeout(owned, left.min(Duration::from_millis(5)))
                .unwrap();
            owned = guard;
        }
        let fence_id = owned.next;
        owned.next = owned
            .next
            .checked_add(1)
            .ok_or_else(|| FixtureError::new(StoreStatus::Exhausted))?;
        let handles = std::mem::take(&mut owned.handles);
        drop(owned);
        let mut service_producers = Vec::new();
        let mut object_workers = Vec::new();
        for (id, mut p) in handles {
            let outcome = if let Some(outcome) = p.terminal_join.take() {
                outcome
            } else {
                let handle = p.handle.take().expect("finished registered handle");
                if handle.join().is_ok() {
                    ProducerJoinOutcome::Returned
                } else {
                    ProducerJoinOutcome::Panicked
                }
            };
            let settled = p.barrier.as_ref().map(|b| {
                let mut state = lock(&b.state);
                state.snapshot.settled = true;
                b.changed.notify_all();
                state.snapshot.clone()
            });
            let record = ProducerFence {
                producer_id: id,
                producer_generation: 1,
                kind: p.kind.clone(),
                native_pid: std::process::id(),
                rust_thread_id: p.thread_id,
                selected_object_id: Some(p.object),
                request_id: Some(p.request),
                operation_id: Some(p.operation),
                completed_join: true,
                join_outcome: outcome,
                settled_barrier: settled,
                live_ownership_after_join: false,
            };
            if matches!(p.kind, ProducerKind::ObjectIoWorker) {
                object_workers.push(record)
            } else {
                service_producers.push(record)
            }
        }
        let mut objects = Vec::new();
        for object in self.core.objects.values() {
            let s = lock(&object.state);
            if s.io_worker.is_some() {
                return Err(FixtureError::new(StoreStatus::NotReady));
            }
            let original_closures = s
                .journal
                .operations
                .as_slice()
                .iter()
                .filter_map(|row| {
                    s.runtime
                        .get(&row.allocation.operation_id)
                        .and_then(|r| r.original.as_ref())
                        .map(|r| OperationClosureWitness {
                            allocation: row.allocation.clone(),
                            submitted_binding: row.binding.clone(),
                            original_request_id: u64_at(&r.wire.payload, 0),
                            original_producer_id: r.producer,
                            original_producer_generation: 1,
                            closure: row.closure,
                        })
                })
                .collect();
            let durable = s
                .synced
                .as_ref()
                .filter(|j| j.transition_sequence >= s.durable.transition_sequence)
                .unwrap_or(&s.durable);
            objects.push(ObjectFenceWitness {
                owner_id: s.journal.owner_id,
                selected_object_id: object.id,
                selected_generation: s.journal.selected_generation,
                service_epoch_at_fence: s.journal.service_epoch,
                reserved_successor_epoch: s.journal.service_epoch.checked_add(1),
                durable_journal_sequence: durable.transition_sequence,
                durable_journal_hash: fixture(durable.digest().map_err(|_| StoreStatus::Internal))?,
                operation_high_water_min: s.journal.operation_high_water,
                writer_high_water_min: s.journal.writer_high_water,
                admission_epoch_min: s.admission_epoch,
                known_durable_operations: durable.operations.as_slice().to_vec(),
                issued_runtime_permits: s
                    .runtime
                    .values()
                    .filter_map(|r| r.permitted.clone())
                    .collect(),
                original_closures,
            });
        }
        let snapshot = FenceSnapshot {
            schema_version: 1,
            namespace_id: self.core.namespace,
            fence_id,
            profile_hash: self.core.profile_hash,
            semantic_clock_min: self.core.clock.load(Ordering::Acquire),
            identity_counter_min: lock(&self.core.registry).next_identity,
            service_producers,
            object_workers,
            objects,
            live_ownership_after_join: 0,
        };
        self.core.fence_issued.store(true, Ordering::Release);
        Ok(QuiescedStoreOwner {
            core: self.core.clone(),
            snapshot: Box::new(snapshot),
        })
    }
}
impl PauseToken {
    fn wait(&self, timeout_ms: u64, settled: bool) -> Result<BarrierSnapshot, FixtureError> {
        if !(1..=2000).contains(&timeout_ms) {
            return Err(FixtureError::new(StoreStatus::Invalid));
        }
        let until = Instant::now() + Duration::from_millis(timeout_ms);
        let mut state = lock(&self.barrier.state);
        while if settled {
            !state.snapshot.settled
        } else {
            !state.snapshot.entered
        } {
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(FixtureError::new(StoreStatus::Timeout));
            }
            let (s, _) = self.barrier.changed.wait_timeout(state, left).unwrap();
            state = s;
        }
        Ok(state.snapshot.clone())
    }
    pub fn wait_until_entered(&self, timeout_ms: u64) -> Result<BarrierSnapshot, FixtureError> {
        self.wait(timeout_ms, false)
    }
    pub fn wait_until_settled(&self, timeout_ms: u64) -> Result<BarrierSnapshot, FixtureError> {
        self.wait(timeout_ms, true)
    }
}
impl StoreFixture {
    pub fn corrupt_fenced(
        token: &mut QuiescedStoreOwner,
        object_id: u64,
        fault: StorageFault,
    ) -> Result<(), FixtureError> {
        fixture(Self::corrupt_inner(token, object_id, fault))
    }
    fn corrupt_inner(
        token: &mut QuiescedStoreOwner,
        object_id: u64,
        fault: StorageFault,
    ) -> Result<(), StoreStatus> {
        if !token.core.closed.load(Ordering::Acquire) || !lock(&token.core.owned).handles.is_empty()
        {
            return Err(StoreStatus::NotReady);
        }
        let object = token.core.object(object_id)?;
        let state = lock(&object.state);
        let selected = &state.durable.current;
        let id = schema(selected.content_id())?;
        let journal_path = object.root.join("selection.json");
        let cas = object.root.join("cas");
        let manifest_path = cas.join(format!("{}.manifest.json", id.hash_hex()));
        let blob_path = cas.join(format!("{}.blob", id.hash_hex()));
        let bytes = storage::read(&journal_path, 131072)?;
        let write = |path: PathBuf, bytes: Vec<u8>| {
            fs::write(path, bytes).map_err(|_| StoreStatus::Unknown)
        };
        match fault {
            StorageFault::MissingJournal => {
                fs::remove_file(journal_path).map_err(|_| StoreStatus::Unknown)?
            }
            StorageFault::TruncatedJournal => {
                write(journal_path, bytes[..bytes.len() / 2].to_vec())?
            }
            StorageFault::DuplicateJournalField => {
                let mut output = b"{\"schema_version\":1,".to_vec();
                output.extend_from_slice(&bytes[1..]);
                write(journal_path, output)?;
            }
            StorageFault::UnknownJournalVersion
            | StorageFault::ExtraJournalField
            | StorageFault::WrongJournalBinding
            | StorageFault::BrokenJournalChain => {
                let mut value: serde_json::Value =
                    serde_json::from_slice(&bytes).map_err(|_| StoreStatus::Unknown)?;
                match fault {
                    StorageFault::UnknownJournalVersion => {
                        value["schema_version"] = serde_json::json!(999)
                    }
                    StorageFault::ExtraJournalField => {
                        value["unexpected"] = serde_json::json!(true)
                    }
                    StorageFault::WrongJournalBinding => {
                        value["owner_id"] = serde_json::json!(selected.owner_id + 1)
                    }
                    StorageFault::BrokenJournalChain => {
                        value["prior_journal_hash"] = serde_json::json!([0u8; 32].as_slice())
                    }
                    _ => unreachable!(),
                }
                write(
                    journal_path,
                    serde_json::to_vec(&value).map_err(|_| StoreStatus::Unknown)?,
                )?;
            }
            StorageFault::EarlierAllocatedOnlyJournal => {
                let mut earlier = state.durable.clone();
                earlier.current = earlier.initial.clone();
                let mut rows = earlier.operations.as_slice().to_vec();
                for row in &mut rows {
                    row.state = EditorOperationStateV0::Allocated;
                    row.closure = EditorClosureV0::None;
                    row.binding = None;
                    row.submitted_service_epoch = None;
                    row.permit = None;
                    row.receipt = None;
                    row.successor = None;
                }
                earlier.operations = schema(BoundedVec::try_from_vec(rows))?;
                write(journal_path, schema(earlier.canonical_bytes())?)?;
            }
            StorageFault::CorruptBlob => write(blob_path, b"corrupt".to_vec())?,
            StorageFault::UnsignedManifest => {
                let mut manifest: artifact_store_schema::Manifest =
                    serde_json::from_slice(&storage::read(&manifest_path, 2048)?)
                        .map_err(|_| StoreStatus::Unknown)?;
                manifest.signatures.clear();
                write(
                    manifest_path,
                    serde_json::to_vec(&manifest).map_err(|_| StoreStatus::Unknown)?,
                )?;
            }
            StorageFault::WrongSigningKey => {
                let mut profile = token.core.profile.clone();
                profile.fixture_signing_seed = [23; 32];
                let manifest = storage::make_manifest(&profile, &state.bytes)?;
                write(
                    manifest_path,
                    serde_json::to_vec(&manifest).map_err(|_| StoreStatus::Unknown)?,
                )?;
            }
            StorageFault::UnboundPublicationIntent | StorageFault::UnsignedPublicationIntent => {
                let body = b"unbound-intent\n";
                let mut manifest = storage::make_manifest(&token.core.profile, body)?;
                if matches!(fault, StorageFault::UnsignedPublicationIntent) {
                    manifest.signatures.clear();
                }
                let id = artifact_store_schema::ContentId::parse(&manifest.content_id)
                    .map_err(|_| StoreStatus::Unknown)?;
                let value = serde_json::json!({"owner":crate::domain_visibility::ArtifactOwner{content_id:id.clone(),domain_id:selected.owner_id,is_global:false,ingested_at:0},"manifest":manifest});
                write(
                    cas.join(format!("{}.publication.json", id.hash_hex())),
                    serde_json::to_vec(&value).map_err(|_| StoreStatus::Unknown)?,
                )?;
            }
            StorageFault::SymlinkEntry => {
                std::os::unix::fs::symlink("selection.json", cas.join("symlink-entry"))
                    .map_err(|_| StoreStatus::Unknown)?
            }
        }
        Ok(())
    }
}
