//! Actual Desktop authority for the default-off native Read prerequisite.
//! Diagnostic views and numeric IDs never replace the private Gate or held handles.
use crate::dev::Status;
use artifact_store_schema::editor_save::EditorActorV0;
use kernel_api::ipc::Envelope;
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const BINDINGS: usize = 16;
const ORIGINS: usize = 64;
const PRODUCERS: usize = 64;
const BARRIERS: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadBindingView {
    pub actor: EditorActorV0,
    pub object_id: u64,
    pub object_generation: u64,
    pub selected_revision: u64,
    pub expires_at_ms: u64,
    pub selected_hash: [u8; 32],
}
#[derive(Clone)]
pub struct ReadCallView {
    pub binding: ReadBindingView,
    pub origin_id: u64,
    pub entered: Instant,
    pub deadline: Instant,
    pub request: Envelope,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProducerCounts {
    pub held: u32,
    pub io: u32,
    pub joining: u32,
    pub joined_total: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadBarrierView {
    pub entered: bool,
    pub settled: bool,
    pub origin_id: u64,
    pub dispatcher: u64,
    pub supervisor: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum JoinOutcome {
    Returned = 1,
    Panicked = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ProducerKind {
    DesktopDispatch = 1,
    DesktopSupervisor = 2,
    StoreDispatch = 3,
    StoreSupervisor = 4,
    StoreIo = 5,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum NativeCounter {
    Binding = 1,
    Origin = 2,
    Producer = 3,
}

pub struct RegistryWitness {
    authority: Arc<NativeAuthority>,
}
pub struct ApprovedReadBinding {
    authority: Arc<NativeAuthority>,
    row: u64,
}
pub struct NativeReadCall {
    authority: Arc<NativeAuthority>,
    origin: Arc<OriginLifetime>,
    entered: Instant,
    deadline: Instant,
    request: Envelope,
}
pub struct ApprovedReadGuard<'a> {
    binding: &'a ApprovedReadBinding,
    gate: MutexGuard<'a, GateState>,
}
pub struct ReadGuard<'a> {
    call: &'a NativeReadCall,
    gate: MutexGuard<'a, GateState>,
}
pub struct ReadDeadlineGuard<'a> {
    call: &'a NativeReadCall,
    gate: MutexGuard<'a, GateState>,
}
pub struct NativeReadEntry {
    authority: Arc<NativeAuthority>,
    origin: Arc<OriginLifetime>,
    result: Arc<EntryResult>,
    producers: [ProducerId; 2],
}
pub struct NativeReadPause {
    authority: Arc<NativeAuthority>,
    barrier: Arc<EntryBarrier>,
}
pub struct ProducerReservation {
    authority: Arc<NativeAuthority>,
    id: u64,
    pending: bool,
}
pub struct ProducerId {
    authority: Arc<NativeAuthority>,
    id: u64,
    // A reference is not an actual producer holder. Retired references must not
    // retain origin capacity after the original and actual row have gone away.
    origin: Weak<OriginLifetime>,
    kind: ProducerKind,
}
pub struct JoinedProducerProof {
    authority: Arc<NativeAuthority>,
    id: u64,
    outcome: JoinOutcome,
}

// There is deliberately no State back-reference. State alone issues bindings and
// synchronizes its lifetime transitions into this Gate before unlocking State.
pub(super) struct NativeAuthority {
    identity: u64,
    gate: Mutex<GateState>,
    owned: Mutex<ProducerRoster>,
}
struct BindingRow {
    view: ReadBindingView,
    endpoint: u64,
    retired: bool,
}
struct OriginLifetime {
    id: u64,
}
struct OriginRow {
    lifetime: Weak<OriginLifetime>,
    binding: u64,
    deadline: Instant,
    closed: bool,
}
struct GateState {
    now_ms: u64,
    next_binding: u64,
    next_origin: u64,
    bindings: BTreeMap<u64, BindingRow>,
    origins: BTreeMap<u64, OriginRow>,
    armed: Option<(u64, Arc<EntryBarrier>)>,
    barriers: Vec<Weak<EntryBarrier>>,
}
struct ProducerRow {
    kind: ProducerKind,
    origin: Arc<OriginLifetime>,
    handle: Option<JoinHandle<()>>,
    joining: bool,
}
struct ProducerRoster {
    next: u64,
    rows: BTreeMap<u64, ProducerRow>,
    joined_total: u64,
    recent: VecDeque<(u64, JoinOutcome)>,
}
struct EntryResult {
    state: Mutex<ResultState>,
    changed: Condvar,
}
struct ResultState {
    outcome: Option<Result<NativeReadCall, Status>>,
    completed: bool,
    started: bool,
    consumed: bool,
}
struct EntryBarrier {
    state: Mutex<BarrierState>,
    changed: Condvar,
}
struct BarrierState {
    released: bool,
    view: ReadBarrierView,
}
fn locked<T>(m: &Mutex<T>) -> Result<MutexGuard<'_, T>, Status> {
    m.lock().map_err(|_| Status::Internal)
}
fn same(a: &Arc<NativeAuthority>, b: &Arc<NativeAuthority>) -> bool {
    Arc::ptr_eq(a, b) && a.identity == b.identity
}
impl GateState {
    fn live_binding(&self, id: u64) -> Result<&BindingRow, Status> {
        let row = self.bindings.get(&id).ok_or(Status::Denied)?;
        if row.retired || self.now_ms >= row.view.expires_at_ms {
            return Err(Status::Stale);
        }
        Ok(row)
    }
    fn live_call(&self, call: &NativeReadCall) -> Result<&BindingRow, Status> {
        let origin = self.origins.get(&call.origin.id).ok_or(Status::Denied)?;
        if !origin
            .lifetime
            .upgrade()
            .is_some_and(|token| Arc::ptr_eq(&token, &call.origin))
            || origin.deadline != call.deadline
        {
            return Err(Status::Denied);
        }
        let row = self.live_binding(origin.binding)?;
        if origin.closed || Instant::now() >= origin.deadline {
            return Err(Status::Timeout);
        }
        Ok(row)
    }
    fn call_view(&self, call: &NativeReadCall) -> ReadCallView {
        let origin = &self.origins[&call.origin.id];
        ReadCallView {
            binding: self.bindings[&origin.binding].view.clone(),
            origin_id: call.origin.id,
            entered: call.entered,
            deadline: call.deadline,
            request: call.request,
        }
    }
    fn prune_origins(&mut self) {
        self.origins
            .retain(|_, row| row.lifetime.strong_count() > 0);
    }
}
impl NativeAuthority {
    pub(super) fn new() -> Arc<Self> {
        Arc::new_cyclic(|identity| Self {
            // Local allocation identity is private and checked together with
            // Arc pointer identity; numeric values cannot construct authority.
            identity: identity.as_ptr() as usize as u64,
            gate: Mutex::new(GateState {
                now_ms: 0,
                next_binding: 0,
                next_origin: 0,
                bindings: BTreeMap::new(),
                origins: BTreeMap::new(),
                armed: None,
                barriers: Vec::new(),
            }),
            owned: Mutex::new(ProducerRoster {
                next: 0,
                rows: BTreeMap::new(),
                joined_total: 0,
                recent: VecDeque::new(),
            }),
        })
    }
    pub(super) fn witness(self: &Arc<Self>) -> RegistryWitness {
        RegistryWitness {
            authority: self.clone(),
        }
    }
    // These hooks run only while actual Desktop State is locked. Poison fails
    // subsequent admission closed; hooks still retain the authoritative retirement.
    pub(super) fn advance(&self, now: u64) {
        let mut gate = self.gate.lock().unwrap_or_else(|p| p.into_inner());
        gate.now_ms = gate.now_ms.max(now);
    }
    pub(super) fn retire(&self, instance: u64) {
        let mut gate = self.gate.lock().unwrap_or_else(|p| p.into_inner());
        for row in gate.bindings.values_mut() {
            if row.view.actor.instance_id == instance {
                row.retired = true;
            }
        }
    }
    pub(super) fn retire_all(&self) {
        let mut gate = self.gate.lock().unwrap_or_else(|p| p.into_inner());
        for row in gate.bindings.values_mut() {
            row.retired = true;
        }
    }
    pub(super) fn approve(
        self: &Arc<Self>,
        endpoint: u64,
        view: ReadBindingView,
    ) -> Result<ApprovedReadBinding, Status> {
        let mut gate = locked(&self.gate)?;
        if let Some((&id, row)) = gate.bindings.iter().find(|(_, r)| r.endpoint == endpoint) {
            gate.live_binding(id)?;
            if row.view != view {
                return Err(Status::Denied);
            }
            return Ok(ApprovedReadBinding {
                authority: self.clone(),
                row: id,
            });
        }
        if gate.now_ms >= view.expires_at_ms {
            return Err(Status::Stale);
        }
        let id = gate.next_binding.checked_add(1).ok_or(Status::Exhausted)?;
        if gate.bindings.len() >= BINDINGS {
            return Err(Status::Exhausted);
        }
        let mut objects = gate
            .bindings
            .values()
            .map(|r| r.view.object_id)
            .collect::<Vec<_>>();
        objects.sort_unstable();
        objects.dedup();
        if objects.len() >= 2 && !objects.contains(&view.object_id) {
            return Err(Status::Exhausted);
        }
        gate.next_binding = id;
        gate.bindings.insert(
            id,
            BindingRow {
                view,
                endpoint,
                retired: false,
            },
        );
        Ok(ApprovedReadBinding {
            authority: self.clone(),
            row: id,
        })
    }
    pub(super) fn start(
        self: &Arc<Self>,
        endpoint: u64,
        request: Envelope,
        entered: Instant,
    ) -> Result<NativeReadEntry, Status> {
        let deadline = entered
            .checked_add(Duration::from_millis(1000))
            .ok_or(Status::Exhausted)?;
        let mut gate = locked(&self.gate)?;
        let binding = gate
            .bindings
            .iter()
            .find(|(_, r)| r.endpoint == endpoint)
            .map(|(&id, _)| id)
            .ok_or(Status::Denied)?;
        let session = gate.live_binding(binding)?.view.actor.session_id;
        if Instant::now() >= deadline {
            return Err(Status::Timeout);
        }
        gate.barriers.retain(|barrier| barrier.strong_count() > 0);
        if gate.armed.as_ref().is_some_and(|(sid, _)| *sid == session)
            && gate.barriers.len() > BARRIERS
        {
            return Err(Status::Exhausted);
        }
        gate.prune_origins();
        let id = gate.next_origin.checked_add(1).ok_or(Status::Exhausted)?;
        if gate.origins.len() >= ORIGINS {
            return Err(Status::Exhausted);
        }
        // Check the pair atomically before any spawn or partial reservation.
        let mut roster = locked(&self.owned)?;
        let last = roster.next.checked_add(2).ok_or(Status::Exhausted)?;
        if roster.rows.len() > PRODUCERS - 2 {
            return Err(Status::Exhausted);
        }
        let origin = Arc::new(OriginLifetime { id });
        let first = last - 1;
        roster.next = last;
        for (pid, kind) in [
            (first, ProducerKind::DesktopDispatch),
            (last, ProducerKind::DesktopSupervisor),
        ] {
            roster.rows.insert(
                pid,
                ProducerRow {
                    kind,
                    origin: origin.clone(),
                    handle: None,
                    joining: false,
                },
            );
        }
        drop(roster);
        gate.next_origin = id;
        gate.origins.insert(
            id,
            OriginRow {
                lifetime: Arc::downgrade(&origin),
                binding,
                deadline,
                closed: false,
            },
        );
        let pause = if gate.armed.as_ref().is_some_and(|(sid, _)| *sid == session) {
            gate.armed.take().map(|(_, b)| b)
        } else {
            None
        };
        drop(gate);
        let mut reservations = [
            ProducerReservation {
                authority: self.clone(),
                id: first,
                pending: true,
            },
            ProducerReservation {
                authority: self.clone(),
                id: last,
                pending: true,
            },
        ];
        let supervisor_reservation = reservations[1].take_pending();
        let dispatcher_reservation = reservations[0].take_pending();
        let result = Arc::new(EntryResult {
            state: Mutex::new(ResultState {
                outcome: None,
                completed: false,
                started: false,
                consumed: false,
            }),
            changed: Condvar::new(),
        });
        let worker_authority = self.clone();
        let worker_origin = origin.clone();
        let worker_result = result.clone();
        let worker_pause = pause.clone();
        let dispatch = std::thread::Builder::new()
            .name("desktop-native-read".into())
            .spawn(move || {
                // Both actual handles must be installed before this worker can
                // publish a call or an entered-barrier observation. A partial
                // spawn failure releases this latch with an Internal outcome.
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
                    if let Some(barrier) = &worker_pause {
                        let mut state = barrier.state.lock().unwrap_or_else(|p| p.into_inner());
                        state.view.settled = true;
                        barrier.changed.notify_all();
                    }
                    return;
                }
                if let Some(barrier) = &worker_pause {
                    let mut state = barrier.state.lock().unwrap_or_else(|p| p.into_inner());
                    state.view = ReadBarrierView {
                        entered: true,
                        settled: false,
                        origin_id: id,
                        dispatcher: first,
                        supervisor: last,
                    };
                    barrier.changed.notify_all();
                    while !state.released {
                        state = barrier
                            .changed
                            .wait(state)
                            .unwrap_or_else(|p| p.into_inner());
                    }
                }
                let call = NativeReadCall {
                    authority: worker_authority.clone(),
                    origin: worker_origin,
                    entered,
                    deadline,
                    request,
                };
                let outcome = locked(&worker_authority.gate)
                    .and_then(|gate| {
                        gate.live_call(&call)?;
                        Ok(())
                    })
                    .map(|()| call);
                worker_result.publish(outcome);
                if let Some(barrier) = worker_pause {
                    let mut state = barrier.state.lock().unwrap_or_else(|p| p.into_inner());
                    state.view.settled = true;
                    barrier.changed.notify_all();
                }
            });
        let dispatcher = match dispatch {
            Ok(handle) => dispatcher_reservation.install(handle),
            Err(_) => {
                dispatcher_reservation.cancel_unspawned()?;
                supervisor_reservation.cancel_unspawned()?;
                self.close_origin(id);
                return Err(Status::Internal);
            }
        };
        let timer_authority = self.clone();
        let timer_origin = origin.clone();
        let timer_result = result.clone();
        let supervisor = std::thread::Builder::new()
            .name("desktop-native-deadline".into())
            .spawn(move || {
                let mut state = timer_result.state.lock().unwrap_or_else(|p| p.into_inner());
                while !state.started && !state.completed {
                    state = timer_result
                        .changed
                        .wait(state)
                        .unwrap_or_else(|p| p.into_inner());
                }
                while !state.completed {
                    let now = Instant::now();
                    if now >= deadline {
                        // Close under admission before making Timeout observable.
                        drop(state);
                        timer_authority.close_origin(timer_origin.id);
                        timer_result.publish(Err(Status::Timeout));
                        return;
                    }
                    let (next, _) = timer_result
                        .changed
                        .wait_timeout(state, deadline.duration_since(now))
                        .unwrap_or_else(|p| p.into_inner());
                    state = next;
                }
            });
        let supervisor = match supervisor {
            Ok(handle) => supervisor_reservation.install(handle),
            Err(_) => {
                supervisor_reservation.cancel_unspawned()?;
                self.close_origin(id);
                result.publish(Err(Status::Internal));
                // Dispatcher is already installed in the roster and remains
                // observable/charged even if still paused after this failure.
                return Err(Status::Internal);
            }
        };
        {
            let mut ready = result.state.lock().unwrap_or_else(|p| p.into_inner());
            if !ready.completed {
                ready.started = true;
            }
            result.changed.notify_all();
        }
        Ok(NativeReadEntry {
            authority: self.clone(),
            origin,
            result,
            producers: [dispatcher, supervisor],
        })
    }
    fn close_origin(&self, id: u64) {
        let mut gate = self.gate.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(row) = gate.origins.get_mut(&id) {
            row.closed = true;
        }
    }
    pub(super) fn pause(self: &Arc<Self>, session: u64) -> Result<NativeReadPause, Status> {
        let mut gate = locked(&self.gate)?;
        gate.barriers.retain(|b| b.strong_count() > 0);
        if gate.armed.is_some() {
            return Err(Status::NotReady);
        }
        if gate.barriers.len() > BARRIERS {
            return Err(Status::Exhausted);
        }
        let barrier = Arc::new(EntryBarrier {
            state: Mutex::new(BarrierState {
                released: false,
                view: ReadBarrierView {
                    entered: false,
                    settled: false,
                    origin_id: 0,
                    dispatcher: 0,
                    supervisor: 0,
                },
            }),
            changed: Condvar::new(),
        });
        gate.barriers.push(Arc::downgrade(&barrier));
        gate.armed = Some((session, barrier.clone()));
        Ok(NativeReadPause {
            authority: self.clone(),
            barrier,
        })
    }
    pub(super) fn release(self: &Arc<Self>, pause: &NativeReadPause) -> Result<(), Status> {
        if !same(self, &pause.authority) {
            return Err(Status::Denied);
        }
        let mut state = locked(&pause.barrier.state)?;
        if state.released {
            return Err(Status::NotReady);
        }
        state.released = true;
        pause.barrier.changed.notify_all();
        Ok(())
    }
    pub(super) fn seed(&self, counter: NativeCounter) -> Result<(), Status> {
        let mut gate = locked(&self.gate)?;
        match counter {
            NativeCounter::Binding => gate.next_binding = u64::MAX,
            NativeCounter::Origin => gate.next_origin = u64::MAX,
            NativeCounter::Producer => locked(&self.owned)?.next = u64::MAX,
        }
        Ok(())
    }
    pub(super) fn producers(self: &Arc<Self>) -> Vec<ProducerId> {
        let roster = self.owned.lock().unwrap_or_else(|p| p.into_inner());
        roster
            .rows
            .iter()
            .filter(|(_, row)| row.handle.is_some() || row.joining)
            .map(|(&id, row)| ProducerId {
                authority: self.clone(),
                id,
                origin: Arc::downgrade(&row.origin),
                kind: row.kind,
            })
            .collect()
    }
}
impl EntryResult {
    fn publish(&self, outcome: Result<NativeReadCall, Status>) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if !state.completed {
            state.completed = true;
            state.outcome = Some(outcome);
            self.changed.notify_all();
        }
    }
}
impl RegistryWitness {
    pub fn matches_binding(&self, binding: &ApprovedReadBinding) -> bool {
        same(&self.authority, &binding.authority)
    }
    pub fn matches_call(&self, call: &NativeReadCall) -> bool {
        same(&self.authority, &call.authority)
    }
    pub fn duplicate_producer(&self, id: &ProducerId) -> Result<ProducerId, Status> {
        if !same(&self.authority, &id.authority) {
            return Err(Status::Denied);
        }
        let roster = locked(&self.authority.owned)?;
        let row = roster.rows.get(&id.id).ok_or(Status::NotReady)?;
        if row.kind != id.kind || !Weak::ptr_eq(&Arc::downgrade(&row.origin), &id.origin) {
            return Err(Status::Denied);
        }
        if row.handle.is_none() && !row.joining {
            return Err(Status::NotReady);
        }
        // Recreate a reference to this actual installed row only. The row's
        // charge, identity and handle do not change; this is not service proof.
        Ok(ProducerId {
            authority: self.authority.clone(),
            id: id.id,
            origin: Arc::downgrade(&row.origin),
            kind: row.kind,
        })
    }
    pub fn producer_counts(&self) -> ProducerCounts {
        let roster = self
            .authority
            .owned
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        ProducerCounts {
            held: roster.rows.len() as u32,
            io: roster
                .rows
                .values()
                .filter(|r| r.kind == ProducerKind::StoreIo)
                .count() as u32,
            joining: roster.rows.values().filter(|r| r.joining).count() as u32,
            joined_total: roster.joined_total,
        }
    }
    pub fn join_finished(&self, id: &ProducerId) -> Result<JoinedProducerProof, Status> {
        if !same(&self.authority, &id.authority) {
            return Err(Status::Denied);
        }
        let handle = {
            let mut roster = locked(&self.authority.owned)?;
            let row = roster.rows.get_mut(&id.id).ok_or(Status::NotReady)?;
            if row.kind != id.kind || !Weak::ptr_eq(&Arc::downgrade(&row.origin), &id.origin) {
                return Err(Status::Denied);
            }
            if row.joining || row.handle.as_ref().is_none_or(|h| !h.is_finished()) {
                return Err(Status::NotReady);
            }
            row.joining = true;
            row.handle.take().ok_or(Status::Internal)?
        };
        // Only the actual held handle can produce a join proof. No enforcement
        // lock spans join; a concurrent caller observes Joining/NotReady.
        let outcome = if handle.join().is_ok() {
            JoinOutcome::Returned
        } else {
            JoinOutcome::Panicked
        };
        let mut gate = self
            .authority
            .gate
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let mut roster = self
            .authority
            .owned
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        roster.rows.remove(&id.id);
        roster.joined_total = roster.joined_total.saturating_add(1);
        if roster.recent.len() == 16 {
            roster.recent.pop_front();
        }
        roster.recent.push_back((id.id, outcome));
        drop(roster);
        gate.prune_origins();
        Ok(JoinedProducerProof {
            authority: self.authority.clone(),
            id: id.id,
            outcome,
        })
    }
}
impl ApprovedReadBinding {
    pub fn admit<'a>(&'a self, witness: &RegistryWitness) -> Result<ApprovedReadGuard<'a>, Status> {
        if !same(&self.authority, &witness.authority) {
            return Err(Status::Denied);
        }
        let gate = locked(&self.authority.gate)?;
        gate.live_binding(self.row)?;
        Ok(ApprovedReadGuard {
            binding: self,
            gate,
        })
    }
}
impl ApprovedReadGuard<'_> {
    pub fn view(&self) -> ReadBindingView {
        self.gate.bindings[&self.binding.row].view.clone()
    }
    pub fn view_binding(&self, binding: &ApprovedReadBinding) -> Result<ReadBindingView, Status> {
        if !same(&self.binding.authority, &binding.authority) {
            return Err(Status::Denied);
        }
        Ok(self.gate.live_binding(binding.row)?.view.clone())
    }
}
impl NativeReadCall {
    pub fn request(&self) -> Envelope {
        self.request
    }
    pub fn entered(&self) -> Instant {
        self.entered
    }
    pub fn deadline(&self) -> Instant {
        self.deadline
    }
    pub fn admit<'a>(&'a self, witness: &RegistryWitness) -> Result<ReadGuard<'a>, Status> {
        if !same(&self.authority, &witness.authority) {
            return Err(Status::Denied);
        }
        let gate = locked(&self.authority.gate)?;
        gate.live_call(self)?;
        Ok(ReadGuard { call: self, gate })
    }
    pub fn deadline_guard<'a>(
        &'a self,
        witness: &RegistryWitness,
    ) -> Result<ReadDeadlineGuard<'a>, Status> {
        if !same(&self.authority, &witness.authority) {
            return Err(Status::Denied);
        }
        let gate = locked(&self.authority.gate)?;
        let origin = gate.origins.get(&self.origin.id).ok_or(Status::Denied)?;
        if !origin
            .lifetime
            .upgrade()
            .is_some_and(|o| Arc::ptr_eq(&o, &self.origin))
        {
            return Err(Status::Denied);
        }
        gate.live_binding(origin.binding)?;
        Ok(ReadDeadlineGuard { call: self, gate })
    }
    pub fn reserve_store_producer(
        &self,
        kind: ProducerKind,
    ) -> Result<ProducerReservation, Status> {
        if !matches!(
            kind,
            ProducerKind::StoreDispatch | ProducerKind::StoreSupervisor
        ) {
            return Err(Status::Unsupported);
        }
        let gate = locked(&self.authority.gate)?;
        gate.live_call(self)?;
        let mut roster = locked(&self.authority.owned)?;
        let id = roster.next.checked_add(1).ok_or(Status::Exhausted)?;
        if roster.rows.len() >= PRODUCERS {
            return Err(Status::Exhausted);
        }
        roster.next = id;
        roster.rows.insert(
            id,
            ProducerRow {
                kind,
                origin: self.origin.clone(),
                handle: None,
                joining: false,
            },
        );
        Ok(ProducerReservation {
            authority: self.authority.clone(),
            id,
            pending: true,
        })
    }
}
impl ReadGuard<'_> {
    pub fn view(&self) -> ReadCallView {
        self.gate.call_view(self.call)
    }
}
impl ReadDeadlineGuard<'_> {
    pub fn view(&self) -> ReadCallView {
        self.gate.call_view(self.call)
    }
    pub fn close_query(&mut self) -> Result<(), Status> {
        let row = self
            .gate
            .origins
            .get_mut(&self.call.origin.id)
            .ok_or(Status::Denied)?;
        if Instant::now() < row.deadline {
            return Err(Status::NotReady);
        }
        row.closed = true;
        Ok(())
    }
}
impl NativeReadEntry {
    pub fn producer_ids(&self) -> [&ProducerId; 2] {
        [&self.producers[0], &self.producers[1]]
    }
    pub fn wait(&self) -> Result<NativeReadCall, Status> {
        // The entry itself keeps its original row even after wait consumes the
        // call. The installed producers independently retain the same origin.
        let _owner = (&self.authority, &self.origin);
        let mut state = locked(&self.result.state)?;
        if state.consumed {
            return Err(Status::NotReady);
        }
        while !state.completed {
            state = self
                .result
                .changed
                .wait(state)
                .map_err(|_| Status::Internal)?;
        }
        state.consumed = true;
        state.outcome.take().ok_or(Status::Internal)?
    }
}
impl ProducerReservation {
    fn take_pending(&mut self) -> Self {
        self.pending = false;
        Self {
            authority: self.authority.clone(),
            id: self.id,
            pending: true,
        }
    }
    pub fn install(mut self, handle: JoinHandle<()>) -> ProducerId {
        let mut roster = self
            .authority
            .owned
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        // No other API can retire a reserved row: join requires an installed
        // finished handle, and cancellation consumes this sole reservation.
        let row = roster
            .rows
            .get_mut(&self.id)
            .expect("private reserved producer row");
        row.handle = Some(handle);
        self.pending = false;
        ProducerId {
            authority: self.authority.clone(),
            id: self.id,
            origin: Arc::downgrade(&row.origin),
            kind: row.kind,
        }
    }
    pub fn cancel_unspawned(mut self) -> Result<(), Status> {
        let mut roster = locked(&self.authority.owned)?;
        let row = roster.rows.get(&self.id).ok_or(Status::NotReady)?;
        if row.handle.is_some() || row.joining {
            return Err(Status::NotReady);
        }
        roster.rows.remove(&self.id);
        self.pending = false;
        Ok(())
    }
}
impl Drop for ProducerReservation {
    fn drop(&mut self) {
        if self.pending {
            let mut roster = self
                .authority
                .owned
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            if roster
                .rows
                .get(&self.id)
                .is_some_and(|r| r.handle.is_none() && !r.joining)
            {
                roster.rows.remove(&self.id);
            }
        }
    }
}
impl JoinedProducerProof {
    pub fn producer_id(&self) -> u64 {
        let _issuer = &self.authority;
        self.id
    }
    pub fn outcome(&self) -> JoinOutcome {
        self.outcome
    }
}
impl NativeReadPause {
    fn wait(&self, timeout_ms: u64, settled: bool) -> Result<ReadBarrierView, Status> {
        if !(1..=5000).contains(&timeout_ms) {
            return Err(Status::Invalid);
        }
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(timeout_ms))
            .ok_or(Status::Exhausted)?;
        let mut state = locked(&self.barrier.state)?;
        while if settled {
            !state.view.settled
        } else {
            !state.view.entered
        } {
            let now = Instant::now();
            if now >= deadline {
                return Err(Status::NotReady);
            }
            let (next, _) = self
                .barrier
                .changed
                .wait_timeout(state, deadline.duration_since(now))
                .map_err(|_| Status::Internal)?;
            state = next;
        }
        Ok(state.view)
    }
    pub fn wait_until_entered(&self, timeout_ms: u64) -> Result<ReadBarrierView, Status> {
        self.wait(timeout_ms, false)
    }
    pub fn wait_until_settled(&self, timeout_ms: u64) -> Result<ReadBarrierView, Status> {
        self.wait(timeout_ms, true)
    }
}
