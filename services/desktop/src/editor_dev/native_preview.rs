//! Private, default-off native Store preview authority. Views never mint admission.
use super::native_authority::{GateState, NativeAuthority};
use super::{
    Endpoint, NativeReadCall, NativeReadEntry, PeerContext, ProducerId, ProducerKind,
    ProducerReservation, ReadDeadlineGuard, ReadGuard, RegistryWitness,
};
use crate::dev::Status;
use artifact_store_schema::editor_save::EditorActorV0;
use kernel_api::cap::{Handle, HandleKind};
use kernel_api::ipc::Envelope;
use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak};
use std::time::{Duration, Instant};

pub(super) fn lock<T>(m: &Mutex<T>) -> Result<MutexGuard<'_, T>, Status> {
    m.lock().map_err(|_| Status::Internal)
}
pub(super) fn same(a: &Arc<NativeAuthority>, b: &Arc<NativeAuthority>) -> bool {
    Arc::ptr_eq(a, b)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewOwner {
    pub owner_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreObjectEnrollment {
    pub owner_id: u64,
    pub object_id: u64,
    pub object_generation: u64,
    pub store_epoch: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreCurrentSelection {
    pub owner: PreviewOwner,
    pub object_id: u64,
    pub object_generation: u64,
    pub store_epoch: u64,
    pub revision: u64,
    pub content_hash: [u8; 32],
}
#[derive(Clone, Debug)]
pub struct PinView {
    pub current: StoreCurrentSelection,
    pub pin_id: u64,
    pub pin_generation: u64,
    pub expires_at_ms: u64,
    pub entered: Instant,
    pub deadline: Instant,
}
#[derive(Clone, Debug)]
pub struct PendingView {
    pub actor: EditorActorV0,
    pub pin: PinView,
    pub plan_id: u64,
    pub preview_revision: u64,
    pub instance_expires_at_ms: u64,
}
#[derive(Clone)]
pub struct PreviewReadCallView {
    pub actor: EditorActorV0,
    pub object_id: u64,
    pub object_generation: u64,
    pub store_epoch: u64,
    pub origin_id: u64,
    pub entered: Instant,
    pub deadline: Instant,
    pub request: Envelope,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewUiTicketKind {
    Selector = 1,
    Approval = 2,
}
#[derive(Clone, Debug)]
pub struct PreviewUiTicketView {
    pub kind: PreviewUiTicketKind,
    pub ticket_id: u64,
    pub ticket_generation: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub device_id: u64,
    pub device_generation: u64,
    pub input_sequence: u64,
    pub focus_epoch: u64,
    pub plan_id: u64,
    pub preview_revision: u64,
    pub expires_at_ms: u64,
    pub entered: Instant,
    pub deadline: Instant,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewPausePoint {
    BeforeActivation = 1,
    BeforeFramePublish = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativePreviewCounter {
    Pin = 1,
    AttachmentVersion = 2,
    StatusVersion = 3,
    UiTicket = 4,
    ReadContext = 5,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewBarrierView {
    pub point: PreviewPausePoint,
    pub entered: bool,
    pub settled: bool,
    pub session: u64,
    pub instance: u64,
    pub pin: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewCounts {
    pub live_pins: u32,
    pub pin_rows: u32,
    pub inactive: u32,
    pub active: u32,
    pub retired: u32,
    pub retained_view_leases: u32,
    pub ticket_rows: u32,
    pub live_tickets: u32,
    pub read_contexts: u32,
    pub query_slots: u32,
}
#[derive(Clone, Copy, Debug)]
pub enum PreviewDeliveryStage {
    Authorize,
    Validate,
    BuildReply,
    Expose,
}
#[derive(Clone, Copy, Debug)]
pub struct DeliveryObservation {
    pub stage: PreviewDeliveryStage,
    pub status: Status,
    pub authorized_attempt: bool,
    pub active: bool,
    pub retired: bool,
}
pub struct NativePreviewDeliveryFailure {
    pub(super) observation: DeliveryObservation,
}
impl NativePreviewDeliveryFailure {
    pub fn status(&self) -> Status {
        self.observation.status
    }
    pub fn observation(&self) -> DeliveryObservation {
        self.observation
    }
}
pub struct InactiveDesktopProbeObservation {
    pub endpoint_requests: [Envelope; 3],
    pub endpoint_replies: [Envelope; 3],
    pub origin_status: Result<(), Status>,
    pub bootstrap_status: Result<(), Status>,
    pub grants_lease_status: Result<(), Status>,
}
pub struct PreviewChromeRecordObservation {
    pub session_id: u64,
    pub instance_id: u64,
    pub focus_epoch: u64,
    pub sequence: u64,
    pub attachment_version: u64,
    pub status_version: u64,
    pub bytes: [u8; 248],
}

pub struct StoreSelectionEnrollment {
    pub(super) authority: Arc<NativeAuthority>,
}
pub struct EnrollmentFailure {
    authority: Arc<NativeAuthority>,
    status: Status,
}
impl EnrollmentFailure {
    pub fn status(&self) -> Status {
        let _held = &self.authority;
        self.status
    }
}
pub struct SelectionIssuer {
    authority: Arc<NativeAuthority>,
}
pub struct SelectionGuard<'a> {
    issuer: &'a SelectionIssuer,
    pub(super) gate: MutexGuard<'a, GateState>,
}
pub struct StoreSelectionPin {
    pub(super) authority: Arc<NativeAuthority>,
    pub(super) life: Arc<PinLife>,
}
pub(super) struct PinLife {
    pub(super) id: u64,
}
pub struct PreviewUiTicket {
    pub(super) authority: Arc<NativeAuthority>,
    pub(super) life: Arc<TicketLife>,
    pub(super) view: PreviewUiTicketView,
    pub(super) request: Envelope,
}
pub(super) struct TicketLife {
    pub(super) id: u64,
}
pub struct PendingStoreInstance {
    pub(super) authority: Arc<NativeAuthority>,
    pub(super) life: Arc<AttachmentLife>,
    pub(super) setup: Arc<PendingSetup>,
}
// A genuine pending owner charges its setup aliases independently of the
// delivered attachment/context. Active carriers never inherit these owners.
pub(super) struct PendingSetup {
    pub(super) _pin: Arc<PinLife>,
    pub(super) _ticket: Arc<TicketLife>,
}
pub(super) struct AttachmentLife {
    pub(super) id: u64,
    pub(super) pin: u64,
    pub(super) pin_identity: Weak<PinLife>,
}
pub struct StoreActivationStamp {
    pub(super) authority: Arc<NativeAuthority>,
    pub(super) life: Arc<AttachmentLife>,
    pub(super) version: u64,
}
pub struct PreviewReadOrigin {
    pub(super) authority: Arc<NativeAuthority>,
    pub(super) life: Arc<AttachmentLife>,
    pub(super) context: u64,
}
pub struct NativePreviewBindings {
    pub(super) bootstrap: Envelope,
    pub(super) self_status: Endpoint,
    pub(super) focus_read: Endpoint,
    pub(super) surface: Endpoint,
    pub(super) artifact_origin: PreviewReadOrigin,
}
pub struct StoreLaunch {
    pub(super) confirm_reply: Envelope,
    pub(super) bindings: NativePreviewBindings,
    pub(super) stamp: StoreActivationStamp,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreReadInstallation {
    pub store_handle: Handle,
    pub object_id: u64,
    pub object_generation: u64,
    pub store_epoch: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreReadActivationView {
    pub actor: EditorActorV0,
    pub store_handle: Handle,
    pub object_id: u64,
    pub object_generation: u64,
    pub store_epoch: u64,
    pub read_context_id: u64,
    pub attachment_version: u64,
    pub instance_expires_at_ms: u64,
}
pub struct PreviewReadEntry {
    pub(super) native: NativeReadEntry,
    pub(super) authority: Arc<NativeAuthority>,
    pub(super) life: Arc<AttachmentLife>,
}
pub struct PreviewReadCall {
    native: NativeReadCall,
    authority: Arc<NativeAuthority>,
    life: Arc<AttachmentLife>,
}
pub struct PreviewReadGuard<'a> {
    call: &'a PreviewReadCall,
    native: ReadGuard<'a>,
}
pub struct PreviewReadDeadlineGuard<'a> {
    native: ReadDeadlineGuard<'a>,
}
pub struct PreviewPendingRetention {
    _life: Arc<AttachmentLife>,
    _setup: Arc<PendingSetup>,
}
pub struct PreviewActivationAttempt {
    life: Arc<AttachmentLife>,
    _setup: Arc<PendingSetup>,
    barrier: Option<Arc<PreviewBarrier>>,
    waited: Mutex<bool>,
}
pub struct InactiveReadProbeReservation {
    _hold: ProbeHold,
    _life: Arc<AttachmentLife>,
}
pub struct PreviewPause {
    pub(super) authority: Arc<NativeAuthority>,
    pub(super) barrier: Arc<PreviewBarrier>,
}
pub(super) struct PreviewBarrier {
    state: Mutex<PreviewBarrierState>,
    changed: Condvar,
}
struct PreviewBarrierState {
    released: bool,
    probing: bool,
    view: PreviewBarrierView,
}
pub(super) struct ProbeHold {
    barrier: Arc<PreviewBarrier>,
}
impl Drop for ProbeHold {
    fn drop(&mut self) {
        let mut s = self.barrier.state.lock().unwrap_or_else(|p| p.into_inner());
        s.probing = false;
        self.barrier.changed.notify_all();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TicketState {
    Available,
    Taken,
    Consumed,
    Closed,
}
pub(super) struct TicketRow {
    pub(super) life: Weak<TicketLife>,
    pub(super) untaken: Option<Arc<TicketLife>>,
    pub(super) view: PreviewUiTicketView,
    pub(super) request: Envelope,
    pub(super) state: TicketState,
}
pub(super) struct OwnerRow {
    pub(super) owner: PreviewOwner,
    pub(super) chrome: u64,
    pub(super) state_id: usize,
    pub(super) object: u64,
    pub(super) generation: u64,
}
pub(super) struct PinRow {
    pub(super) life: Weak<PinLife>,
    pub(super) view: PinView,
    pub(super) retired: bool,
    pub(super) consumed: bool,
}
pub(super) struct AttachmentRow {
    pub(super) life: Weak<AttachmentLife>,
    pub(super) view: PendingView,
    pub(super) context: u64,
    pub(super) version: u64,
    pub(super) status_version: u64,
    pub(super) installation: Option<StoreReadInstallation>,
    pub(super) attempt_started: bool,
    pub(super) active: bool,
    pub(super) delivered: bool,
    pub(super) retired: bool,
    pub(super) faulted: bool,
    pub(super) status_exhausted: bool,
    pub(super) logical_setup_expires: u64,
    pub(super) request_deadline: Instant,
    pub(super) endpoints: [u64; 3],
    pub(super) grants: u64,
    pub(super) confirm_request: Envelope,
}
pub(super) struct PreviewGate {
    pub(super) claimed: bool,
    pub(super) retired: bool,
    pub(super) owners: BTreeMap<u64, OwnerRow>,
    pub(super) objects: BTreeMap<u64, StoreObjectEnrollment>,
    pub(super) pins: BTreeMap<u64, PinRow>,
    pub(super) tickets: BTreeMap<u64, TicketRow>,
    pub(super) attachments: BTreeMap<u64, AttachmentRow>,
    pub(super) next_pin: u64,
    pub(super) next_ticket: u64,
    pub(super) next_attachment: u64,
    pub(super) next_status: u64,
    pub(super) next_context: u64,
    pub(super) now: u64,
    pub(super) armed: Option<(u64, PreviewPausePoint, Arc<PreviewBarrier>)>,
    pub(super) barriers: Vec<Weak<PreviewBarrier>>,
    pub(super) leases: Vec<Weak<()>>,
}
impl PreviewGate {
    pub(super) fn new() -> Self {
        Self {
            claimed: false,
            retired: false,
            owners: BTreeMap::new(),
            objects: BTreeMap::new(),
            pins: BTreeMap::new(),
            tickets: BTreeMap::new(),
            attachments: BTreeMap::new(),
            next_pin: 0,
            next_ticket: 0,
            next_attachment: 0,
            next_status: 0,
            next_context: 0,
            now: 0,
            armed: None,
            barriers: Vec::new(),
            leases: Vec::new(),
        }
    }
    pub(super) fn prune(&mut self) {
        self.pins.retain(|_, r| r.life.strong_count() > 0);
        self.tickets.retain(|_, r| r.life.strong_count() > 0);
        self.attachments.retain(|_, r| r.life.strong_count() > 0);
        self.leases.retain(|r| r.strong_count() > 0);
        self.barriers.retain(|r| r.strong_count() > 0);
    }
    pub(super) fn counts(&self, query_slots: u32) -> PreviewCounts {
        PreviewCounts {
            live_pins: self
                .pins
                .values()
                .filter(|r| !r.retired && !r.consumed)
                .count() as u32,
            pin_rows: self.pins.len() as u32,
            inactive: self
                .attachments
                .values()
                .filter(|r| !r.active && !r.retired)
                .count() as u32,
            active: self
                .attachments
                .values()
                .filter(|r| r.active && !r.retired)
                .count() as u32,
            retired: self.attachments.values().filter(|r| r.retired).count() as u32,
            retained_view_leases: self.leases.len() as u32,
            ticket_rows: self.tickets.len() as u32,
            live_tickets: self
                .tickets
                .values()
                .filter(|r| matches!(r.state, TicketState::Available | TicketState::Taken))
                .count() as u32,
            read_contexts: self.attachments.len() as u32,
            query_slots,
        }
    }
    pub(super) fn expire(&mut self, now: u64) {
        self.now = self.now.max(now);
        for r in self.pins.values_mut() {
            if self.now >= r.view.expires_at_ms || Instant::now() >= r.view.deadline {
                r.retired = true;
            }
        }
        for r in self.tickets.values_mut() {
            if now >= r.view.expires_at_ms && r.state != TicketState::Consumed {
                r.state = TicketState::Closed;
            }
        }
        for r in self.attachments.values_mut() {
            if now >= r.view.instance_expires_at_ms
                || !r.delivered
                    && (self.now >= r.logical_setup_expires
                        || Instant::now() >= r.request_deadline
                        || self
                            .pins
                            .get(&r.view.pin.pin_id)
                            .is_none_or(|pin| pin.retired))
            {
                r.retired = true;
            }
        }
    }
    pub(super) fn retire_instance(&mut self, id: u64) {
        if let Some(r) = self.attachments.get_mut(&id) {
            r.retired = true;
        }
    }
    pub(super) fn retire_all(&mut self) {
        self.retired = true;
        for r in self.attachments.values_mut() {
            r.retired = true;
        }
        for r in self.pins.values_mut() {
            r.retired = true;
        }
        for r in self.tickets.values_mut() {
            r.state = TicketState::Closed;
        }
    }
    pub(super) fn retire_setup(&mut self, session: u64) {
        for r in self.attachments.values_mut() {
            if r.view.actor.session_id == session && !r.active {
                r.retired = true;
            }
        }
        for r in self.pins.values_mut() {
            if r.view.current.owner.session_id == session && !r.consumed {
                r.retired = true;
            }
        }
        for r in self.tickets.values_mut() {
            if r.view.session_id == session && r.state != TicketState::Consumed {
                r.state = TicketState::Closed;
            }
        }
    }
    fn live_attachment(&self, r: &AttachmentRow) -> Result<(), Status> {
        if r.retired
            || self.now >= r.view.instance_expires_at_ms
            || !r.delivered
                && (self.now >= r.logical_setup_expires
                    || Instant::now() >= r.request_deadline
                    || self.pins.get(&r.view.pin.pin_id).is_none_or(|pin| {
                        pin.retired
                            || self.now >= pin.view.expires_at_ms
                            || Instant::now() >= pin.view.deadline
                            || pin.view.pin_generation != r.view.pin.pin_generation
                            || pin.view.entered != r.view.pin.entered
                            || pin.view.deadline != r.view.pin.deadline
                            || r.life
                                .upgrade()
                                .is_none_or(|life| !Weak::ptr_eq(&pin.life, &life.pin_identity))
                    }))
        {
            return Err(Status::Stale);
        }
        Ok(())
    }
    pub(super) fn admit_instance(&self, id: u64) -> Result<(), Status> {
        let r = self.attachments.get(&id).ok_or(Status::Denied)?;
        self.live_attachment(r)?;
        if !r.active {
            return Err(Status::NotReady);
        }
        Ok(())
    }
    pub(super) fn row(&self, life: &Arc<AttachmentLife>) -> Result<&AttachmentRow, Status> {
        let r = self.attachments.get(&life.id).ok_or(Status::Denied)?;
        if !Weak::ptr_eq(&r.life, &Arc::downgrade(life)) {
            return Err(Status::Denied);
        }
        Ok(r)
    }
}
impl StoreSelectionEnrollment {
    pub fn claim_for_store_constructor(self) -> Result<SelectionIssuer, EnrollmentFailure> {
        let outcome = (|| {
            let mut g = lock(&self.authority.gate)?;
            let p = g.preview.as_mut().ok_or(Status::Denied)?;
            if p.claimed {
                return Err(Status::NotReady);
            }
            p.claimed = true;
            Ok(())
        })();
        match outcome {
            Ok(()) => Ok(SelectionIssuer {
                authority: self.authority,
            }),
            Err(status) => Err(EnrollmentFailure {
                authority: self.authority,
                status,
            }),
        }
    }
}
impl SelectionIssuer {
    pub fn retain_pending_for_error(
        &self,
        pending: &PendingStoreInstance,
    ) -> Result<PreviewPendingRetention, Status> {
        // Both components originate only from actual Desktop State. Retaining
        // them is not admission and deliberately does not acquire Gate: an
        // authentic closed/poisoned lifecycle still needs its error owner.
        if !same(&self.authority, &pending.authority) {
            return Err(Status::Denied);
        }
        Ok(PreviewPendingRetention {
            _life: pending.life.clone(),
            _setup: pending.setup.clone(),
        })
    }
    pub fn validate_witness(&self, witness: &RegistryWitness) -> Result<(), Status> {
        if !same(&self.authority, &witness.authority) {
            return Err(Status::Denied);
        }
        let gate = lock(&self.authority.gate)?;
        let p = gate.preview.as_ref().ok_or(Status::Denied)?;
        if !p.claimed {
            return Err(Status::Denied);
        }
        if p.retired {
            return Err(Status::Stale);
        }
        Ok(())
    }
    pub fn begin_read_activation_attempt(
        &self,
        pending: &PendingStoreInstance,
    ) -> Result<PreviewActivationAttempt, Status> {
        let mut guard = self.lock()?;
        let view = guard.view_pending(pending)?;
        let p = guard.gate.preview.as_mut().ok_or(Status::Denied)?;
        let row = p
            .attachments
            .get_mut(&pending.life.id)
            .ok_or(Status::Denied)?;
        if row.installation.is_none() || row.attempt_started || row.active {
            return Err(Status::NotReady);
        }
        row.attempt_started = true;
        let barrier = if p.armed.as_ref().is_some_and(|(session, point, _)| {
            *session == view.actor.session_id && *point == PreviewPausePoint::BeforeActivation
        }) {
            p.armed.take().map(|(_, _, barrier)| barrier)
        } else {
            None
        };
        Ok(PreviewActivationAttempt {
            life: pending.life.clone(),
            _setup: pending.setup.clone(),
            barrier,
            waited: Mutex::new(false),
        })
    }
    pub fn claim_inactive_read_probe(
        &self,
        chrome: &PeerContext,
        pending: &PendingStoreInstance,
        pause: &PreviewPause,
    ) -> Result<InactiveReadProbeReservation, Status> {
        {
            let guard = self.lock()?;
            let owner = guard.preview_owner(chrome)?;
            if !same(&self.authority, &pause.authority) {
                return Err(Status::Denied);
            }
            let view = guard.view_pending(pending)?;
            if view.actor.session_id != owner.session_id || view.actor.owner_id != owner.owner_id {
                return Err(Status::Denied);
            }
            let row = guard.preview()?.row(&pending.life)?;
            if row.active || row.installation.is_none() {
                return Err(Status::NotReady);
            }
        }
        // The interlock is claimed with no Gate/Registry/Object guard held.
        let hold = pause.probe(pending)?;
        Ok(InactiveReadProbeReservation {
            _hold: hold,
            _life: pending.life.clone(),
        })
    }
    pub fn lock(&self) -> Result<SelectionGuard<'_>, Status> {
        let gate = lock(&self.authority.gate)?;
        if !gate.preview.as_ref().is_some_and(|p| p.claimed) {
            return Err(Status::Denied);
        }
        Ok(SelectionGuard { issuer: self, gate })
    }
}
impl SelectionGuard<'_> {
    fn preview(&self) -> Result<&PreviewGate, Status> {
        self.gate.preview.as_ref().ok_or(Status::Denied)
    }
    pub fn enroll_objects(&mut self, objects: &[StoreObjectEnrollment]) -> Result<(), Status> {
        if objects.is_empty() || objects.len() > 2 {
            return Err(Status::Invalid);
        }
        let p = self.gate.preview.as_mut().ok_or(Status::Denied)?;
        if !p.objects.is_empty() {
            return Err(Status::NotReady);
        }
        let mut checked = BTreeMap::new();
        for o in objects {
            if o.owner_id == 0
                || o.object_id == 0
                || o.object_generation == 0
                || o.store_epoch == 0
                || !p.owners.values().any(|r| {
                    r.owner.owner_id == o.owner_id
                        && r.object == o.object_id
                        && r.generation == o.object_generation
                })
                || checked.insert(o.object_id, o.clone()).is_some()
            {
                return Err(Status::Denied);
            }
        }
        p.objects = checked;
        Ok(())
    }
    pub fn preview_owner(&self, chrome: &PeerContext) -> Result<PreviewOwner, Status> {
        let p = self.preview()?;
        let owner = p
            .owners
            .values()
            .find(|r| chrome.matches_preview_owner(&r.owner, r.chrome, r.state_id))
            .ok_or(Status::Denied)?;
        if p.retired || p.now == u64::MAX {
            return Err(Status::Stale);
        }
        Ok(owner.owner.clone())
    }
    pub fn reserve_pin(
        &mut self,
        chrome: &PeerContext,
        current: StoreCurrentSelection,
        ttl_ms: u64,
    ) -> Result<StoreSelectionPin, Status> {
        self.reserve_pin_started(chrome, current, ttl_ms, Instant::now())
    }
    pub fn reserve_pin_started(
        &mut self,
        chrome: &PeerContext,
        current: StoreCurrentSelection,
        ttl_ms: u64,
        entered: Instant,
    ) -> Result<StoreSelectionPin, Status> {
        let owner = self.preview_owner(chrome)?;
        if current.owner != owner {
            return Err(Status::Denied);
        }
        if !(1..=30000).contains(&ttl_ms) {
            return Err(Status::Invalid);
        }
        let deadline = entered
            .checked_add(Duration::from_millis(ttl_ms))
            .ok_or(Status::Exhausted)?;
        if Instant::now() >= deadline {
            return Err(Status::Stale);
        }
        let p = self.gate.preview.as_mut().ok_or(Status::Denied)?;
        p.prune();
        let o = p.objects.get(&current.object_id).ok_or(Status::Denied)?;
        if (o.owner_id, o.object_generation, o.store_epoch)
            != (
                owner.owner_id,
                current.object_generation,
                current.store_epoch,
            )
            || current.revision == 0
        {
            return Err(Status::Denied);
        }
        if p.pins
            .values()
            .any(|r| r.view.current.object_id == current.object_id && !r.retired && !r.consumed)
        {
            return Err(Status::NotReady);
        }
        if p.pins.len() >= 16
            || p.pins
                .values()
                .filter(|r| !r.retired && !r.consumed)
                .count()
                >= 2
        {
            return Err(Status::Exhausted);
        }
        let id = p.next_pin.checked_add(1).ok_or(Status::Exhausted)?;
        let expires = p.now.checked_add(ttl_ms).ok_or(Status::Exhausted)?;
        let life = Arc::new(PinLife { id });
        p.next_pin = id;
        p.pins.insert(
            id,
            PinRow {
                life: Arc::downgrade(&life),
                view: PinView {
                    current,
                    pin_id: id,
                    pin_generation: id,
                    expires_at_ms: expires,
                    entered,
                    deadline,
                },
                retired: false,
                consumed: false,
            },
        );
        Ok(StoreSelectionPin {
            authority: self.issuer.authority.clone(),
            life,
        })
    }
    pub fn view_pin(&self, pin: &StoreSelectionPin) -> Result<PinView, Status> {
        if !same(&self.issuer.authority, &pin.authority) {
            return Err(Status::Denied);
        }
        let p = self.preview()?;
        let r = p.pins.get(&pin.life.id).ok_or(Status::Denied)?;
        if !Weak::ptr_eq(&r.life, &Arc::downgrade(&pin.life)) {
            return Err(Status::Denied);
        }
        if r.retired || p.now >= r.view.expires_at_ms || Instant::now() >= r.view.deadline {
            return Err(Status::Stale);
        }
        Ok(r.view.clone())
    }
    pub fn view_pending(&self, pending: &PendingStoreInstance) -> Result<PendingView, Status> {
        if !same(&self.issuer.authority, &pending.authority) {
            return Err(Status::Denied);
        }
        let p = self.preview()?;
        let r = p.row(&pending.life)?;
        p.live_attachment(r)?;
        Ok(r.view.clone())
    }
    pub fn stage_read_installation(
        &mut self,
        pending: &PendingStoreInstance,
        installation: StoreReadInstallation,
    ) -> Result<(), Status> {
        self.view_pending(pending)?;
        let p = self.gate.preview.as_mut().ok_or(Status::Denied)?;
        let r = p
            .attachments
            .get_mut(&pending.life.id)
            .ok_or(Status::Denied)?;
        let c = &r.view.pin.current;
        if installation.object_id != c.object_id
            || installation.object_generation != c.object_generation
            || installation.store_epoch != c.store_epoch
            || installation.store_handle.kind != HandleKind::Ipc
            || !(1..=65535).contains(&installation.store_handle.index)
            || !(1..=u32::MAX as u64).contains(&installation.store_handle.generation)
        {
            return Err(Status::Denied);
        }
        if let Some(old) = &r.installation {
            if old != &installation {
                return Err(Status::Denied);
            }
            return Ok(());
        }
        let view = super::ReadBindingView {
            actor: r.view.actor.clone(),
            object_id: installation.object_id,
            object_generation: installation.object_generation,
            selected_revision: c.revision,
            expires_at_ms: r.view.instance_expires_at_ms,
            selected_hash: c.content_hash,
        };
        let context = r.context;
        self.gate.install_preview_binding(context, view)?;
        self.gate
            .preview
            .as_mut()
            .ok_or(Status::Denied)?
            .attachments
            .get_mut(&pending.life.id)
            .ok_or(Status::Denied)?
            .installation = Some(installation);
        Ok(())
    }
    pub fn admit_read_installation(
        &self,
        pending: &PendingStoreInstance,
        installation: StoreReadInstallation,
    ) -> Result<(), Status> {
        if !same(&self.issuer.authority, &pending.authority) {
            return Err(Status::Denied);
        }
        let p = self.preview()?;
        let row = p.row(&pending.life)?;
        if row.installation.as_ref() != Some(&installation) {
            return Err(Status::Denied);
        }
        self.view_pending(pending)?;
        p.admit_instance(pending.life.id)
    }
    pub fn activate_read(
        &mut self,
        pending: &PendingStoreInstance,
        installation: StoreReadInstallation,
    ) -> Result<StoreActivationStamp, Status> {
        self.view_pending(pending)?;
        let p = self.gate.preview.as_mut().ok_or(Status::Denied)?;
        let r = p
            .attachments
            .get_mut(&pending.life.id)
            .ok_or(Status::Denied)?;
        if r.installation.as_ref() != Some(&installation) {
            return Err(Status::Denied);
        }
        if r.active {
            return Err(Status::NotReady);
        }
        // Same lifetime enforcer as live/probe admission, immediately before
        // the one-way Active flag. No fallible work follows that flag.
        p.live_attachment(p.row(&pending.life)?)?;
        let r = p
            .attachments
            .get_mut(&pending.life.id)
            .ok_or(Status::Denied)?;
        r.active = true;
        Ok(StoreActivationStamp {
            authority: self.issuer.authority.clone(),
            life: pending.life.clone(),
            version: r.version,
        })
    }
    pub fn match_read_activation(
        &self,
        left: &StoreActivationStamp,
        right: &StoreActivationStamp,
    ) -> Result<StoreReadActivationView, Status> {
        if !same(&self.issuer.authority, &left.authority)
            || !same(&self.issuer.authority, &right.authority)
            || !Arc::ptr_eq(&left.life, &right.life)
            || left.version != right.version
        {
            return Err(Status::Denied);
        }
        let p = self.preview()?;
        p.admit_instance(left.life.id)?;
        let r = p.row(&left.life)?;
        if r.version != left.version {
            return Err(Status::Denied);
        }
        let i = r.installation.as_ref().ok_or(Status::NotReady)?;
        Ok(StoreReadActivationView {
            actor: r.view.actor.clone(),
            store_handle: i.store_handle,
            object_id: i.object_id,
            object_generation: i.object_generation,
            store_epoch: i.store_epoch,
            read_context_id: r.context,
            attachment_version: r.version,
            instance_expires_at_ms: r.view.instance_expires_at_ms,
        })
    }
    pub fn retire_source_object(&mut self, object_id: u64) -> Result<(), Status> {
        let p = self.gate.preview.as_mut().ok_or(Status::Denied)?;
        if !p.objects.contains_key(&object_id) {
            return Err(Status::Denied);
        }
        for r in p.pins.values_mut() {
            if r.view.current.object_id == object_id {
                r.retired = true;
            }
        }
        for r in p.attachments.values_mut() {
            if r.view.pin.current.object_id == object_id {
                r.retired = true;
                r.faulted = true;
            }
        }
        let next = p.next_status.checked_add(1);
        for r in p.attachments.values_mut() {
            if r.view.pin.current.object_id == object_id {
                match next {
                    Some(v) => r.status_version = v,
                    None => r.status_exhausted = true,
                }
            }
        }
        if let Some(v) = next {
            p.next_status = v;
            Ok(())
        } else {
            Err(Status::Exhausted)
        }
    }
}
impl PreviewUiTicket {
    pub fn view(&self) -> PreviewUiTicketView {
        self.view.clone()
    }
    pub fn request(&self) -> &Envelope {
        &self.request
    }
}
impl StoreLaunch {
    pub fn confirm_reply(&self) -> &Envelope {
        &self.confirm_reply
    }
    pub fn bindings(&self) -> &NativePreviewBindings {
        &self.bindings
    }
    pub fn stamp(&self) -> &StoreActivationStamp {
        &self.stamp
    }
}
impl NativePreviewBindings {
    pub fn bootstrap(&self) -> &Envelope {
        &self.bootstrap
    }
    pub fn self_status(&self) -> &Endpoint {
        &self.self_status
    }
    pub fn focus_read(&self) -> &Endpoint {
        &self.focus_read
    }
    pub fn surface(&self) -> &Endpoint {
        &self.surface
    }
    pub fn artifact_origin(&self) -> &PreviewReadOrigin {
        &self.artifact_origin
    }
}
impl Drop for StoreSelectionPin {
    fn drop(&mut self) {
        let mut g = self
            .authority
            .gate
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if let Some(p) = &mut g.preview {
            if let Some(r) = p.pins.get_mut(&self.life.id) {
                if !r.consumed {
                    r.retired = true;
                }
            }
        }
    }
}
impl Drop for PreviewUiTicket {
    fn drop(&mut self) {
        let mut g = self
            .authority
            .gate
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if let Some(p) = &mut g.preview {
            if let Some(r) = p.tickets.get_mut(&self.life.id) {
                if r.state != TicketState::Consumed {
                    r.state = TicketState::Closed;
                }
            }
        }
    }
}
impl PreviewReadEntry {
    pub fn wait(&self) -> Result<PreviewReadCall, Status> {
        Ok(PreviewReadCall {
            native: self.native.wait()?,
            authority: self.authority.clone(),
            life: self.life.clone(),
        })
    }
    pub fn producer_ids(&self) -> [&ProducerId; 2] {
        self.native.producer_ids()
    }
    pub fn origin_id(&self) -> u64 {
        self.native.query_origin_id()
    }
}
impl PreviewReadCall {
    pub fn request(&self) -> Envelope {
        self.native.request()
    }
    pub fn entered(&self) -> Instant {
        self.native.entered()
    }
    pub fn deadline(&self) -> Instant {
        self.native.deadline()
    }
    pub fn admit<'a>(&'a self, witness: &RegistryWitness) -> Result<PreviewReadGuard<'a>, Status> {
        if !same(&self.authority, &witness.authority) {
            return Err(Status::Denied);
        }
        let native = self.native.admit(witness)?;
        native
            .gate
            .preview
            .as_ref()
            .ok_or(Status::Denied)?
            .admit_instance(self.life.id)?;
        Ok(PreviewReadGuard { call: self, native })
    }
    pub fn deadline_guard<'a>(
        &'a self,
        witness: &RegistryWitness,
    ) -> Result<PreviewReadDeadlineGuard<'a>, Status> {
        if !same(&self.authority, &witness.authority) {
            return Err(Status::Denied);
        }
        Ok(PreviewReadDeadlineGuard {
            native: self.native.deadline_guard(witness)?,
        })
    }
    pub fn reserve_store_producer(
        &self,
        kind: ProducerKind,
    ) -> Result<ProducerReservation, Status> {
        self.native.reserve_store_producer(kind)
    }
}
impl PreviewReadGuard<'_> {
    pub fn view(&self) -> PreviewReadCallView {
        let r = self
            .native
            .gate
            .preview
            .as_ref()
            .expect("admitted preview gate")
            .row(&self.call.life)
            .expect("admitted row");
        let c = &r.view.pin.current;
        PreviewReadCallView {
            actor: r.view.actor.clone(),
            object_id: c.object_id,
            object_generation: c.object_generation,
            store_epoch: c.store_epoch,
            origin_id: self.native.view().origin_id,
            entered: self.call.entered(),
            deadline: self.call.deadline(),
            request: self.call.request(),
        }
    }
}
impl PreviewReadDeadlineGuard<'_> {
    pub fn close_query(&mut self) -> Result<(), Status> {
        self.native.close_query()
    }
}
impl PreviewPause {
    fn wait(&self, timeout_ms: u64, settled: bool) -> Result<PreviewBarrierView, Status> {
        if !(1..=5000).contains(&timeout_ms) {
            return Err(Status::Invalid);
        }
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let mut s = lock(&self.barrier.state)?;
        while if settled {
            !s.view.settled
        } else {
            !s.view.entered
        } {
            let now = Instant::now();
            if now >= deadline {
                return Err(Status::NotReady);
            }
            s = self
                .barrier
                .changed
                .wait_timeout(s, deadline - now)
                .map_err(|_| Status::Internal)?
                .0;
        }
        Ok(s.view)
    }
    pub fn wait_until_entered(&self, timeout_ms: u64) -> Result<PreviewBarrierView, Status> {
        self.wait(timeout_ms, false)
    }
    pub fn wait_until_settled(&self, timeout_ms: u64) -> Result<PreviewBarrierView, Status> {
        self.wait(timeout_ms, true)
    }
    pub(super) fn probe(&self, pending: &PendingStoreInstance) -> Result<ProbeHold, Status> {
        if !same(&self.authority, &pending.authority) {
            return Err(Status::Denied);
        }
        let mut s = lock(&self.barrier.state)?;
        if !s.view.entered
            || s.view.settled
            || s.released
            || s.probing
            || s.view.point != PreviewPausePoint::BeforeActivation
            || s.view.instance != pending.life.id
            || s.view.pin != pending.life.pin
        {
            return Err(Status::NotReady);
        }
        s.probing = true;
        Ok(ProbeHold {
            barrier: self.barrier.clone(),
        })
    }
}

impl PreviewBarrier {
    pub(super) fn new(session: u64, point: PreviewPausePoint) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(PreviewBarrierState {
                released: false,
                probing: false,
                view: PreviewBarrierView {
                    point,
                    entered: false,
                    settled: false,
                    session,
                    instance: 0,
                    pin: 0,
                },
            }),
            changed: Condvar::new(),
        })
    }
    pub(super) fn enter_wait(&self, instance: u64, pin: u64) -> Result<(), Status> {
        let mut s = lock(&self.state)?;
        if s.view.entered {
            return Err(Status::NotReady);
        }
        s.view.entered = true;
        s.view.instance = instance;
        s.view.pin = pin;
        self.changed.notify_all();
        while !s.released {
            s = self.changed.wait(s).map_err(|_| Status::Internal)?;
        }
        Ok(())
    }
    pub(super) fn settle(&self) {
        let mut s = self.state.lock().unwrap_or_else(|p| p.into_inner());
        s.view.settled = true;
        self.changed.notify_all();
    }
    pub(super) fn release(&self) -> Result<(), Status> {
        let mut s = lock(&self.state)?;
        if s.probing || s.released {
            return Err(Status::NotReady);
        }
        s.released = true;
        self.changed.notify_all();
        Ok(())
    }
}
pub(super) struct PreviewSettlement(pub(super) Option<Arc<PreviewBarrier>>);
impl Drop for PreviewSettlement {
    fn drop(&mut self) {
        if let Some(b) = &self.0 {
            b.settle();
        }
    }
}

impl PreviewActivationAttempt {
    pub fn wait_before_activation(&self) -> Result<(), Status> {
        {
            let mut waited = lock(&self.waited)?;
            if *waited {
                return Err(Status::NotReady);
            }
            *waited = true;
        }
        if let Some(barrier) = &self.barrier {
            barrier.enter_wait(self.life.id, self.life.pin)?;
        }
        Ok(())
    }
    pub fn finish(self) {}
}
impl Drop for PreviewActivationAttempt {
    fn drop(&mut self) {
        if let Some(barrier) = &self.barrier {
            barrier.settle();
        }
    }
}
