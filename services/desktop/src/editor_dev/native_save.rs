//! Default-off trusted host Save admission. Values returned by this module do not
//! establish process containment, durable storage, or authority from serialized data.
use super::EditorMessage;
use super::native_authority::{GateState, NativeAuthority};
use super::native_preview as preview;
use super::{
    Endpoint, JoinOutcome, JoinedProducerProof, PeerContext, PinView, PreviewOwner, ProducerId,
    ProducerKind, ProducerReservation, RegistryWitness, StoreCurrentSelection,
    StoreObjectEnrollment,
};
use crate::dev::Status;
use artifact_store_schema::editor_save::{
    EditorActorV0, EditorReceiptOutcomeV0, EditorSaveBindingV0, EditorSaveReceiptV0,
    EditorTextHeaderV0,
};
use kernel_api::cap::Handle;
use kernel_api::ipc::Envelope;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak};
use std::time::{Duration, Instant};

pub(super) fn lock<T>(m: &Mutex<T>) -> Result<MutexGuard<'_, T>, Status> {
    m.lock().map_err(|_| Status::Internal)
}
pub(super) fn same(a: &Arc<NativeAuthority>, b: &Arc<NativeAuthority>) -> bool {
    Arc::ptr_eq(a, b)
}
fn digest(b: &[u8]) -> [u8; 32] {
    Sha256::digest(b).into()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreSaveInstallation {
    pub store_handle: Handle,
    pub object_id: u64,
    pub object_generation: u64,
    pub store_epoch: u64,
}

#[derive(Clone, Debug)]
pub struct SavePendingView {
    pub actor: EditorActorV0,
    pub pin: PinView,
    pub plan_id: u64,
    pub preview_revision: u64,
    pub instance_expires_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveActivationView {
    pub actor: EditorActorV0,
    pub store_handle: Handle,
    pub object_id: u64,
    pub object_generation: u64,
    pub store_epoch: u64,
    pub context_id: u64,
    pub attachment_version: u64,
    pub instance_expires_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftSnapshotView {
    pub actor: EditorActorV0,
    pub object_id: u64,
    pub object_generation: u64,
    pub original_store_epoch: u64,
    pub document_id: u64,
    pub text_generation: u64,
    pub expected_revision: u64,
    pub expected_content_hash: [u8; 32],
    pub byte_len: u32,
    pub source_content_hash: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveIntentView {
    pub intent_id: u64,
    pub snapshot: DraftSnapshotView,
    pub entered: Instant,
    pub deadline: Instant,
}

#[derive(Clone)]
pub struct NativeSaveCallView {
    pub actor: EditorActorV0,
    pub object_id: u64,
    pub object_generation: u64,
    pub store_epoch: u64,
    pub origin_id: u64,
    pub intent_id: Option<u64>,
    pub entered: Instant,
    pub deadline: Instant,
    pub request: Envelope,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeEditorStep {
    pub consumed_sequence: Option<u64>,
    pub text_changed: bool,
    pub save_intent_available: bool,
    pub render_required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeEditorObservation {
    pub actor: EditorActorV0,
    pub document_id: u64,
    pub bytes: Vec<u8>,
    pub cursor: u32,
    pub selection: Option<(u32, u32)>,
    pub first_visible_line: u32,
    pub text_generation: u64,
    pub view_version: u64,
    pub confirmed_revision: u64,
    pub confirmed_hash: [u8; 32],
    pub phase: NativeEditorPhase,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeEditorPhase {
    Loaded,
    Unsaved,
    AllocationPending,
    AllocationUnknown,
    Saving,
    Unknown,
    Saved,
    Conflict,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeFrameObservation {
    pub actor: EditorActorV0,
    pub document_id: u64,
    pub text_generation: u64,
    pub view_version: u64,
    pub surface_id: u64,
    pub surface_generation: u64,
    pub mapping_generation: u64,
    pub focus_epoch: u64,
    pub sequence: u64,
    pub app_crop_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeSaveChromeObservation {
    pub current_frame_actor: EditorActorV0,
    pub original_actor: Option<EditorActorV0>,
    pub document_id: Option<u64>,
    pub text_generation: Option<u64>,
    pub view_version: Option<u64>,
    pub frame_sequence: u64,
    pub focus_epoch: u64,
    pub attachment_version: u64,
    pub status_version: u64,
    pub label: NativeEditorPhase,
    pub current_document_phase: NativeEditorPhase,
    pub outstanding_original_count: u32,
    pub pending_originals: Vec<NativePendingOriginalObservation>,
    pub record248: Option<[u8; 248]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeCloseOutcome {
    QueryClosed,
    UnpermittedSourceRetired,
    OriginalPermittedUnknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeOriginalState {
    Registered,
    AllocationPending,
    AllocationSettled,
    Submitted,
    Permitted,
    Committed,
    DefinitiveNoncommit,
    ClosedUnpermitted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeSaveProfile {
    pub selected_object_limit: u32,
    pub trace_capacity: u32,
    pub composed_history_capacity: u32,
    pub pending_frame_capacity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeSaveDeliveryStage {
    Authorize,
    Validate,
    BuildReply,
    Expose,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeSaveDeliveryObservation {
    pub stage: NativeSaveDeliveryStage,
    pub status: Status,
    pub authorized_attempt: bool,
    pub active: bool,
    pub retired: bool,
    pub pending_retained: bool,
    pub launch_retained: bool,
    pub bootstrap_delivered: bool,
    pub confirm_delivered: bool,
}

#[derive(Clone)]
pub struct InactiveSaveDesktopObservation {
    pub endpoint_requests: [Envelope; 3],
    pub endpoint_replies: [Envelope; 3],
    pub origin_status: Result<(), Status>,
    pub bootstrap_status: Result<(), Status>,
    pub grants_lease_status: Result<(), Status>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeJoinObservation {
    pub producer_id: u64,
    pub producer_generation: u64,
    pub origin_id: u64,
    pub kind: ProducerKind,
    pub native_pid: u32,
    pub rust_thread_id: String,
    pub outcome: JoinOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativePendingOriginalPhase {
    AllocationUnknown,
    CommitUnknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativePendingOriginalObservation {
    pub original_actor: EditorActorV0,
    pub object_id: u64,
    pub object_generation: u64,
    pub intent_id: u64,
    pub operation_id: Option<u64>,
    pub original_backend_epoch: u64,
    pub phase: NativePendingOriginalPhase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeSaveCounter {
    Intent,
    Document,
    TextGeneration,
    ViewVersion,
    FrameJoin,
    StatusVersion,
    SourceBinding,
    OriginalRecord,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeSavePausePoint {
    BeforeActivation,
    BeforeKeyDelivery,
    BeforePresentFreeze,
    BeforeFramePublish,
    BeforeEntryAdmission,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeSaveBarrierView {
    pub entered: bool,
    pub settled: bool,
    pub session: u64,
    pub instance: u64,
    pub pin: u64,
    pub point: NativeSavePausePoint,
}

pub struct NativeSaveEnrollment {
    pub(super) inner: preview::StoreSelectionEnrollment,
    pub(super) authority: Arc<NativeAuthority>,
}
pub struct NativeSaveEnrollmentFailure {
    _inner: preview::EnrollmentFailure,
    status: Status,
}
pub struct SaveIssuer {
    pub(super) inner: preview::SelectionIssuer,
    pub(super) authority: Arc<NativeAuthority>,
}
pub struct SaveGateGuard<'a> {
    pub(super) issuer: &'a SaveIssuer,
    pub(super) inner: preview::SelectionGuard<'a>,
}
pub struct SaveSelectionPin {
    pub(super) inner: preview::StoreSelectionPin,
}
pub struct SaveUiTicket {
    pub(super) inner: preview::PreviewUiTicket,
}
pub struct PendingSaveInstance {
    pub(super) inner: Arc<preview::PendingStoreInstance>,
}
pub struct SaveActivationStamp {
    pub(super) inner: preview::StoreActivationStamp,
}
pub struct SavePendingRetention {
    _inner: preview::PreviewPendingRetention,
}
pub struct SaveActivationAttempt {
    inner: preview::PreviewActivationAttempt,
}
pub struct InactiveSaveProbeReservation {
    _inner: preview::InactiveReadProbeReservation,
}
pub struct NativeSaveDeliveryFailure {
    pub(super) status: Status,
    pub(super) observation: NativeSaveDeliveryObservation,
    pub(super) _pending: Option<Arc<preview::PendingStoreInstance>>,
    pub(super) _launch: Option<Box<DesktopSaveLaunch>>,
}
pub struct DesktopSaveLaunch {
    pub(super) confirm_reply: Envelope,
    pub(super) bindings: NativeSaveBindings,
    pub(super) stamp: SaveActivationStamp,
}
pub struct NativeSaveBindings {
    pub(super) bootstrap: Envelope,
    pub(super) self_status: Endpoint,
    pub(super) focus_read: Endpoint,
    pub(super) surface: Endpoint,
    pub(super) artifact_origin: SaveOrigin,
}
pub struct SaveOrigin {
    pub(super) authority: Arc<NativeAuthority>,
    pub(super) life: Arc<preview::AttachmentLife>,
    pub(super) context: u64,
}

pub(super) struct DocumentLife {
    pub(super) id: u64,
}
pub struct NativeEditor {
    pub(super) authority: Arc<NativeAuthority>,
    pub(super) life: Arc<DocumentLife>,
}
pub(super) struct SnapshotData {
    pub(super) view: DraftSnapshotView,
    pub(super) body: Vec<u8>,
}
pub struct NativeDraftSnapshot {
    pub(super) data: Arc<SnapshotData>,
}
pub(super) struct IntentLife {
    pub(super) id: u64,
    pub(super) exposed: std::sync::atomic::AtomicBool,
    pub(super) snapshot: NativeDraftSnapshot,
    pub(super) entered: Instant,
    pub(super) deadline: Instant,
    pub(super) attachment: Arc<preview::AttachmentLife>,
}
pub struct NativeSaveIntent {
    pub(super) authority: Arc<NativeAuthority>,
    pub(super) life: Arc<IntentLife>,
}
pub struct BoundNativeSource {
    pub(super) authority: Arc<NativeAuthority>,
    pub(super) life: Arc<SourceLife>,
}

impl NativeSaveEnrollment {
    pub fn claim_for_store_constructor(self) -> Result<SaveIssuer, NativeSaveEnrollmentFailure> {
        match self.inner.claim_for_store_constructor() {
            Ok(inner) => Ok(SaveIssuer {
                inner,
                authority: self.authority,
            }),
            Err(inner) => {
                let status = inner.status();
                Err(NativeSaveEnrollmentFailure {
                    _inner: inner,
                    status,
                })
            }
        }
    }
}
impl NativeSaveEnrollmentFailure {
    pub fn status(&self) -> Status {
        self.status
    }
}
impl SaveIssuer {
    pub fn validate_witness(&self, w: &RegistryWitness) -> Result<(), Status> {
        if !same(&self.authority, &w.authority) {
            return Err(Status::Denied);
        }
        self.inner.validate_witness(w)?;
        if lock(&self.authority.gate)?.save.is_none() {
            return Err(Status::Denied);
        }
        Ok(())
    }
    pub fn lock(&self) -> Result<SaveGateGuard<'_>, Status> {
        let inner = self.inner.lock()?;
        if inner.gate.save.is_none() {
            return Err(Status::Denied);
        }
        Ok(SaveGateGuard {
            issuer: self,
            inner,
        })
    }
    pub fn retain_pending_for_error(
        &self,
        p: &PendingSaveInstance,
    ) -> Result<SavePendingRetention, Status> {
        Ok(SavePendingRetention {
            _inner: self.inner.retain_pending_for_error(&p.inner)?,
        })
    }
    pub fn begin_save_activation_attempt(
        &self,
        p: &PendingSaveInstance,
    ) -> Result<SaveActivationAttempt, Status> {
        Ok(SaveActivationAttempt {
            inner: self.inner.begin_read_activation_attempt(&p.inner)?,
        })
    }
}
impl SaveActivationAttempt {
    pub fn wait_before_activation(&self) -> Result<(), Status> {
        self.inner.wait_before_activation()
    }
    pub fn finish(self) {
        self.inner.finish();
    }
}
impl SaveGateGuard<'_> {
    pub fn observe_core_time(&mut self, observed_ms: u64) -> Result<u64, Status> {
        if self.inner.gate.save.is_none() {
            return Err(Status::Denied);
        }
        let preview = self.inner.gate.preview.as_ref().ok_or(Status::Denied)?;
        if !preview.claimed {
            return Err(Status::Denied);
        }
        if preview.retired {
            return Err(Status::Stale);
        }
        let effective = observed_ms.max(self.inner.gate.now_ms).max(preview.now);
        self.inner.gate.now_ms = effective;
        self.inner
            .gate
            .preview
            .as_mut()
            .ok_or(Status::Denied)?
            .expire(effective);
        Ok(effective)
    }
    pub fn enroll_objects(&mut self, o: &[StoreObjectEnrollment]) -> Result<(), Status> {
        self.inner.enroll_objects(o)
    }
    pub fn preview_owner(&self, c: &PeerContext) -> Result<PreviewOwner, Status> {
        self.inner.preview_owner(c)
    }
    pub fn view_pending(&self, p: &PendingSaveInstance) -> Result<SavePendingView, Status> {
        let v = self.inner.view_pending(&p.inner)?;
        Ok(SavePendingView {
            actor: v.actor,
            pin: v.pin,
            plan_id: v.plan_id,
            preview_revision: v.preview_revision,
            instance_expires_at_ms: v.instance_expires_at_ms,
        })
    }
    pub fn stage_installation(
        &mut self,
        p: &PendingSaveInstance,
        i: StoreSaveInstallation,
    ) -> Result<(), Status> {
        self.inner.stage_read_installation(
            &p.inner,
            preview::StoreReadInstallation {
                store_handle: i.store_handle,
                object_id: i.object_id,
                object_generation: i.object_generation,
                store_epoch: i.store_epoch,
            },
        )
    }
    pub fn admit_installation(
        &self,
        p: &PendingSaveInstance,
        i: &StoreSaveInstallation,
    ) -> Result<(), Status> {
        self.inner.admit_read_installation(
            &p.inner,
            preview::StoreReadInstallation {
                store_handle: i.store_handle,
                object_id: i.object_id,
                object_generation: i.object_generation,
                store_epoch: i.store_epoch,
            },
        )
    }
    pub fn activate(
        &mut self,
        p: &PendingSaveInstance,
        i: StoreSaveInstallation,
    ) -> Result<SaveActivationStamp, Status> {
        let inner = self.inner.activate_read(
            &p.inner,
            preview::StoreReadInstallation {
                store_handle: i.store_handle,
                object_id: i.object_id,
                object_generation: i.object_generation,
                store_epoch: i.store_epoch,
            },
        )?;
        Ok(SaveActivationStamp { inner })
    }
    pub fn match_activation(
        &self,
        l: &SaveActivationStamp,
        r: &SaveActivationStamp,
    ) -> Result<SaveActivationView, Status> {
        let v = self.inner.match_read_activation(&l.inner, &r.inner)?;
        Ok(SaveActivationView {
            actor: v.actor,
            store_handle: v.store_handle,
            object_id: v.object_id,
            object_generation: v.object_generation,
            store_epoch: v.store_epoch,
            context_id: v.read_context_id,
            attachment_version: v.attachment_version,
            instance_expires_at_ms: v.instance_expires_at_ms,
        })
    }
}
impl SaveUiTicket {
    pub fn request(&self) -> &Envelope {
        self.inner.request()
    }
    pub fn view(&self) -> preview::PreviewUiTicketView {
        self.inner.view()
    }
}
impl NativeSaveDeliveryFailure {
    pub fn status(&self) -> Status {
        self.status
    }
    pub fn observation(&self) -> NativeSaveDeliveryObservation {
        self.observation.clone()
    }
}
impl DesktopSaveLaunch {
    pub fn confirm_reply(&self) -> &Envelope {
        &self.confirm_reply
    }
    pub fn bindings(&self) -> &NativeSaveBindings {
        &self.bindings
    }
    pub fn stamp(&self) -> &SaveActivationStamp {
        &self.stamp
    }
}
impl NativeSaveBindings {
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
    pub fn artifact_origin(&self) -> &SaveOrigin {
        &self.artifact_origin
    }
}
impl NativeDraftSnapshot {
    pub fn body(&self) -> &[u8] {
        &self.data.body
    }
    pub fn view(&self) -> DraftSnapshotView {
        self.data.view.clone()
    }
}
impl NativeSaveIntent {
    pub fn snapshot(&self) -> &NativeDraftSnapshot {
        &self.life.snapshot
    }
    pub fn entered(&self) -> Instant {
        self.life.entered
    }
    pub fn deadline(&self) -> Instant {
        self.life.deadline
    }
    pub fn view(&self) -> SaveIntentView {
        SaveIntentView {
            intent_id: self.life.id,
            snapshot: self.snapshot().view(),
            entered: self.entered(),
            deadline: self.deadline(),
        }
    }
}
impl BoundNativeSource {
    pub fn binding(&self) -> &EditorSaveBindingV0 {
        &self.life.binding
    }
    pub fn snapshot(&self) -> &NativeDraftSnapshot {
        &self.life.intent.snapshot
    }
}

pub(super) struct SaveGate {
    pub(super) current: BTreeMap<u64, StoreCurrentSelection>,
    pub(super) fenced: BTreeMap<u64, (StoreObjectEnrollment, Vec<u64>)>,
    pub(super) calls: BTreeMap<u64, CallRow>,
    pub(super) allocations: BTreeMap<u64, u64>,
    pub(super) sources: BTreeMap<u64, Weak<SourceLife>>,
    pub(super) originals: BTreeMap<u64, Weak<ClaimLife>>,

    pub(super) next_original: u64,
    pub(super) text_exhausted: bool,
    pub(super) view_exhausted: bool,
    pub(super) armed: Option<(u64, NativeSavePausePoint, Arc<SaveBarrier>)>,
    pub(super) barriers: Vec<Weak<SaveBarrier>>,

    pub(super) documents: BTreeMap<u64, DocumentRow>,
    pub(super) intents: BTreeMap<u64, Weak<IntentLife>>,
    pub(super) next_document: u64,
    pub(super) next_intent: u64,
    pub(super) next_source: u64,
    pub(super) next_frame: u64,
}
pub(super) struct DocumentRow {
    pub(super) life: Arc<DocumentLife>,
    pub(super) attachment: Arc<preview::AttachmentLife>,
    pub(super) object_id: u64,
    pub(super) view: NativeEditorObservation,
    pub(super) queued_intent: Option<Arc<IntentLife>>,
    // Correlation with this document’s genuinely accepted latest Ctrl+S; no authority.
    pub(super) last_intent_id: Option<u64>,
    pub(super) selection_anchor: Option<usize>,
    pub(super) preferred_column: Option<usize>,
    pub(super) request_id: u64,
    pub(super) key_sequence: u64,
    pub(super) focus_epoch: u64,
    pub(super) frame: Option<NativeFrameObservation>,
}
impl SaveGate {
    pub(super) fn new() -> Self {
        Self {
            current: BTreeMap::new(),
            fenced: BTreeMap::new(),
            calls: BTreeMap::new(),
            allocations: BTreeMap::new(),
            sources: BTreeMap::new(),
            originals: BTreeMap::new(),
            next_original: 0,
            text_exhausted: false,
            view_exhausted: false,
            armed: None,
            barriers: Vec::new(),
            documents: BTreeMap::new(),
            intents: BTreeMap::new(),
            next_document: 0,
            next_intent: 0,
            next_source: 0,
            next_frame: 0,
        }
    }
}

impl SaveGateGuard<'_> {
    pub fn reserve_pin_started(
        &mut self,
        owner: &PreviewOwner,
        current: StoreCurrentSelection,
        ttl_ms: u64,
        now_ms: u64,
        entered: Instant,
    ) -> Result<SaveSelectionPin, Status> {
        Ok(SaveSelectionPin {
            inner: self
                .inner
                .reserve_owned_pin(owner, current, ttl_ms, now_ms, entered)?,
        })
    }
}

pub(super) struct CallLife {
    pub(super) origin: Arc<super::native_authority::OriginLifetime>,
    pub(super) attachment: Arc<preview::AttachmentLife>,
    pub(super) view: NativeSaveCallView,
    pub(super) intent: Option<Arc<IntentLife>>,
    pub(super) source: Option<Arc<SourceLife>>,
    pub(super) recovery: Option<Arc<ClaimLife>>,
    pub(super) producers: [u64; 2],
}
pub(super) struct CallRow {
    pub(super) life: Weak<CallLife>,
    pub(super) claimed: bool,
    pub(super) closed: bool,
}
pub(super) struct SourceLife {
    pub(super) intent: Arc<IntentLife>,
    pub(super) binding: EditorSaveBindingV0,
    pub(super) epoch: u64,
    pub(super) _id: u64,
    // Sticky consumption is set only by actual first Commit admission.
    consumed: std::sync::atomic::AtomicBool,
}
pub(super) struct ClaimLife {
    pub(super) view: NativeSaveCallView,
    pub(super) scope: Arc<super::native_authority::OriginLifetime>,
    pub(super) attachment: Arc<preview::AttachmentLife>,
    pub(super) intent: Option<Arc<IntentLife>>,
    pub(super) source: Option<Arc<SourceLife>>,
    pub(super) state: Mutex<ClaimState>,
}
pub(super) struct ClaimState {
    pub(super) phase: NativeOriginalState,
    pub(super) allocation: Option<Envelope>,
    pub(super) receipt: Option<EditorSaveReceiptV0>,
    pub(super) receipt_bytes: Option<[u8; 176]>,
    pub(super) uncertain: bool,
    pub(super) closed: bool,
}
pub struct NativeSaveCall {
    pub(super) authority: Arc<NativeAuthority>,
    pub(super) life: Arc<CallLife>,
}
pub struct NativeExecutionClaim {
    pub(super) authority: Arc<NativeAuthority>,
    pub(super) life: Arc<ClaimLife>,
}
pub struct OriginalRecoveryOrigin {
    pub(super) authority: Arc<NativeAuthority>,
    pub(super) life: Arc<ClaimLife>,
}
pub struct NativeCommitAdmission {
    pub(super) authority: Arc<NativeAuthority>,
    pub(super) claim: Arc<ClaimLife>,
    pub(super) source: Arc<SourceLife>,
}
pub struct NativeSaveGuard<'a> {
    call: &'a NativeSaveCall,
    gate: MutexGuard<'a, GateState>,
}
pub struct NativeSaveOriginalGuard<'a> {
    call: &'a NativeSaveCall,
    gate: MutexGuard<'a, GateState>,
}
pub struct NativeSaveEntry {
    pub(super) authority: Arc<NativeAuthority>,
    pub(super) life: Arc<CallLife>,
    pub(super) result: Arc<SaveEntryResult>,
    pub(super) producers: [ProducerId; 2],
}
pub(super) struct SaveEntryResult {
    pub(super) state: Mutex<SaveEntryState>,
    pub(super) changed: Condvar,
}
pub(super) struct SaveEntryState {
    pub(super) started: bool,
    pub(super) completed: bool,
    pub(super) consumed: bool,
    pub(super) outcome: Option<Result<(), Status>>,
}
impl SaveEntryResult {
    pub(super) fn publish(&self, r: Result<(), Status>) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if !state.completed {
            state.completed = true;
            state.outcome = Some(r);
            self.changed.notify_all();
        }
    }
}
impl NativeSaveEntry {
    pub fn wait_once(&self) -> Result<NativeSaveCall, Status> {
        let mut s = lock(&self.result.state)?;
        if s.consumed {
            return Err(Status::NotReady);
        }
        while !s.completed {
            s = self.result.changed.wait(s).map_err(|_| Status::Internal)?;
        }
        s.consumed = true;
        s.outcome.take().ok_or(Status::Internal)??;
        Ok(NativeSaveCall {
            authority: self.authority.clone(),
            life: self.life.clone(),
        })
    }
    pub fn producer_ids(&self) -> [&ProducerId; 2] {
        [&self.producers[0], &self.producers[1]]
    }
    pub fn origin_id(&self) -> u64 {
        self.life.view.origin_id
    }
}
impl GateState {
    pub(super) fn save_call(
        &self,
        c: &NativeSaveCall,
        original_scope: bool,
    ) -> Result<NativeSaveCallView, Status> {
        let save = self.save.as_ref().ok_or(Status::Denied)?;
        let row = save
            .calls
            .get(&c.life.view.origin_id)
            .ok_or(Status::Denied)?;
        if !Weak::ptr_eq(&row.life, &Arc::downgrade(&c.life)) {
            return Err(Status::Denied);
        }
        if let Some(original) = &c.life.recovery {
            if !original.view.actor.eq(&c.life.view.actor)
                || original.view.object_id != c.life.view.object_id
            {
                return Err(Status::Denied);
            }
            if !original_scope {
                return Err(Status::Denied);
            }
        } else if !original_scope {
            self.preview
                .as_ref()
                .ok_or(Status::Denied)?
                .admit_instance(c.life.attachment.id)?;
        }
        if row.closed || Instant::now() >= c.life.view.deadline {
            return Err(Status::Timeout);
        }
        Ok(c.life.view.clone())
    }
}
impl NativeSaveCall {
    pub fn request(&self) -> &Envelope {
        &self.life.view.request
    }
    pub fn entered(&self) -> Instant {
        self.life.view.entered
    }
    pub fn deadline(&self) -> Instant {
        self.life.view.deadline
    }
    pub fn admit<'a>(&'a self, w: &RegistryWitness) -> Result<NativeSaveGuard<'a>, Status> {
        if !same(&self.authority, &w.authority) {
            return Err(Status::Denied);
        }
        let gate = lock(&self.authority.gate)?;
        gate.save_call(self, false)?;
        Ok(NativeSaveGuard { call: self, gate })
    }
    pub fn admit_original<'a>(
        &'a self,
        w: &RegistryWitness,
    ) -> Result<NativeSaveOriginalGuard<'a>, Status> {
        if !same(&self.authority, &w.authority) {
            return Err(Status::Denied);
        }
        let gate = lock(&self.authority.gate)?;
        gate.save_call(self, true)?;
        Ok(NativeSaveOriginalGuard { call: self, gate })
    }
    pub fn entry_producers(&self, w: &RegistryWitness) -> Result<[ProducerId; 2], Status> {
        if !same(&self.authority, &w.authority) {
            return Err(Status::Denied);
        }
        self.authority
            .save_ids(&self.life.origin, self.life.producers)
    }
    pub fn reserve_store_pair(
        &self,
        w: &RegistryWitness,
    ) -> Result<[ProducerReservation; 2], Status> {
        if !same(&self.authority, &w.authority) {
            return Err(Status::Denied);
        }
        let gate = lock(&self.authority.gate)?;
        gate.save_call(self, self.life.recovery.is_some())?;
        self.authority.reserve_save_pair(
            &self.life.origin,
            [ProducerKind::StoreDispatch, ProducerKind::StoreSupervisor],
        )
    }
}
impl NativeSaveGuard<'_> {
    pub fn view(&self) -> NativeSaveCallView {
        let _held = &self.gate;
        self.call.life.view.clone()
    }
}
impl NativeSaveOriginalGuard<'_> {
    pub fn view(&self) -> NativeSaveCallView {
        self.call.life.view.clone()
    }
    pub fn matches_call(&self, c: &NativeSaveCall) -> bool {
        same(&self.call.authority, &c.authority) && Arc::ptr_eq(&self.call.life, &c.life)
    }
    pub fn close_query(&mut self) -> Result<(), Status> {
        if Instant::now() < self.call.deadline() {
            return Err(Status::NotReady);
        }
        self.gate
            .save
            .as_mut()
            .ok_or(Status::Denied)?
            .calls
            .get_mut(&self.call.life.view.origin_id)
            .ok_or(Status::Denied)?
            .closed = true;
        Ok(())
    }
}
impl NativeCommitAdmission {
    pub fn binding(&self) -> &EditorSaveBindingV0 {
        &self.source.binding
    }
    pub fn original_epoch(&self) -> u64 {
        self.source.epoch
    }
}
impl SaveGateGuard<'_> {
    pub fn admit_call(&self, c: &NativeSaveCall) -> Result<NativeSaveCallView, Status> {
        if !same(&self.issuer.authority, &c.authority) {
            return Err(Status::Denied);
        }
        self.inner.gate.save_call(c, false)
    }
    pub fn admit_original_call(&self, c: &NativeSaveCall) -> Result<NativeSaveCallView, Status> {
        if !same(&self.issuer.authority, &c.authority) {
            return Err(Status::Denied);
        }
        self.inner.gate.save_call(c, true)
    }
    pub fn claim_execution(&mut self, c: &NativeSaveCall) -> Result<NativeExecutionClaim, Status> {
        let view = if c.life.recovery.is_some() {
            self.admit_original_call(c)?
        } else {
            self.admit_call(c)?
        };
        let save = self.inner.gate.save.as_mut().ok_or(Status::Denied)?;
        let next = if matches!(view.request.msg_type, 3 | 5) {
            Some(save.next_original.checked_add(1).ok_or(Status::Exhausted)?)
        } else {
            None
        };
        let row = save.calls.get_mut(&view.origin_id).ok_or(Status::Denied)?;
        if row.claimed {
            return Err(Status::NotReady);
        }
        let life = Arc::new(ClaimLife {
            view,
            scope: c.life.origin.clone(),
            attachment: c.life.attachment.clone(),
            intent: c.life.intent.clone(),
            source: c.life.source.clone(),
            state: Mutex::new(ClaimState {
                phase: if c.life.view.request.msg_type == 3 {
                    NativeOriginalState::AllocationPending
                } else {
                    NativeOriginalState::Registered
                },
                allocation: None,
                receipt: None,
                receipt_bytes: None,
                uncertain: false,
                closed: false,
            }),
        });
        row.claimed = true;
        if let Some(next) = next {
            save.next_original = next;
            save.originals
                .insert(life.view.origin_id, Arc::downgrade(&life));
        }
        Ok(NativeExecutionClaim {
            authority: self.issuer.authority.clone(),
            life,
        })
    }
    pub fn original_recovery(
        &self,
        c: &NativeExecutionClaim,
    ) -> Result<OriginalRecoveryOrigin, Status> {
        if !same(&self.issuer.authority, &c.authority) {
            return Err(Status::Denied);
        }
        Ok(OriginalRecoveryOrigin {
            authority: self.issuer.authority.clone(),
            life: c.life.clone(),
        })
    }
    pub fn classify_original(
        &self,
        c: &NativeExecutionClaim,
    ) -> Result<NativeOriginalState, Status> {
        if !same(&self.issuer.authority, &c.authority) {
            return Err(Status::Denied);
        }
        Ok(lock(&c.life.state)?.phase.clone())
    }
    pub fn close_deadline(
        &mut self,
        c: &NativeExecutionClaim,
    ) -> Result<NativeCloseOutcome, Status> {
        if !same(&self.issuer.authority, &c.authority) {
            return Err(Status::Denied);
        }
        if Instant::now() < c.life.view.deadline {
            return Err(Status::NotReady);
        }
        let mut state = lock(&c.life.state)?;
        if state.receipt.is_some() {
            return Ok(if state.phase == NativeOriginalState::Committed {
                NativeCloseOutcome::OriginalPermittedUnknown
            } else {
                NativeCloseOutcome::QueryClosed
            });
        }
        state.closed = true;
        state.uncertain = true;
        let permitted = matches!(
            state.phase,
            NativeOriginalState::Permitted | NativeOriginalState::Committed
        );
        let outcome = if permitted {
            NativeCloseOutcome::OriginalPermittedUnknown
        } else if c.life.view.request.msg_type == 5 {
            state.phase = NativeOriginalState::ClosedUnpermitted;
            NativeCloseOutcome::UnpermittedSourceRetired
        } else {
            NativeCloseOutcome::QueryClosed
        };
        if let Some(intent) = &c.life.intent {
            if let Some(d) = self
                .inner
                .gate
                .save
                .as_mut()
                .ok_or(Status::Denied)?
                .documents
                .get_mut(&intent.snapshot.data.view.document_id)
            {
                d.view.phase = if c.life.view.request.msg_type == 3 {
                    NativeEditorPhase::AllocationUnknown
                } else if permitted {
                    NativeEditorPhase::Unknown
                } else {
                    NativeEditorPhase::Unsaved
                };
            }
        }
        Ok(outcome)
    }
    pub fn close_original(&mut self, c: &NativeSaveCall) -> Result<(), Status> {
        if !same(&self.issuer.authority, &c.authority) {
            return Err(Status::Denied);
        }
        let row = self
            .inner
            .gate
            .save
            .as_mut()
            .ok_or(Status::Denied)?
            .calls
            .get_mut(&c.life.view.origin_id)
            .ok_or(Status::Denied)?;
        if !Weak::ptr_eq(&row.life, &Arc::downgrade(&c.life)) {
            return Err(Status::Denied);
        }
        row.closed = true;
        let save = self.inner.gate.save.as_mut().ok_or(Status::Denied)?;
        if let Some(original) = save
            .originals
            .get(&c.life.view.origin_id)
            .and_then(Weak::upgrade)
        {
            let mut state = lock(&original.state)?;
            state.closed = true;
            // A permit remains original-only uncertain until its authentic receipt.
            // An owner-known rejection before permit cannot leave the editor Saving.
            let phase = if state.receipt.is_some() {
                None
            } else if state.phase == NativeOriginalState::Permitted {
                state.uncertain = true;
                Some(NativeEditorPhase::Unknown)
            } else {
                state.uncertain = false;
                if state.phase != NativeOriginalState::AllocationSettled {
                    state.phase = NativeOriginalState::ClosedUnpermitted;
                }
                Some(NativeEditorPhase::Unsaved)
            };
            if let (Some(phase), Some(intent)) = (phase, original.intent.as_ref()) {
                if let Some(d) = save
                    .documents
                    .get_mut(&intent.snapshot.data.view.document_id)
                {
                    if d.view.phase != NativeEditorPhase::Unavailable {
                        d.view.phase = phase;
                    }
                }
            }
        }
        Ok(())
    }
    pub fn reserve_io(&mut self, c: &NativeSaveCall) -> Result<ProducerReservation, Status> {
        if c.life.recovery.is_some() && matches!(c.request().msg_type, 7 | 9) {
            self.admit_original_call(c)?;
        } else {
            self.admit_call(c)?;
        }
        self.issuer
            .authority
            .reserve_save_one(&c.life.origin, ProducerKind::StoreIo)
    }
}
impl SaveIssuer {
    pub fn reserve_store_dispatch(
        &self,
        c: &NativeSaveCall,
        kind: ProducerKind,
    ) -> Result<ProducerReservation, Status> {
        if !matches!(
            kind,
            ProducerKind::StoreDispatch | ProducerKind::StoreSupervisor
        ) {
            return Err(Status::Unsupported);
        }
        let guard = self.lock()?;
        guard.admit_call(c)?;
        self.authority.reserve_save_one(&c.life.origin, kind)
    }
    pub fn reserve_original_observation(
        &self,
        o: &OriginalRecoveryOrigin,
        kind: ProducerKind,
    ) -> Result<ProducerReservation, Status> {
        if !same(&self.authority, &o.authority) {
            return Err(Status::Denied);
        }
        if !matches!(
            kind,
            ProducerKind::StoreDispatch | ProducerKind::StoreSupervisor
        ) {
            return Err(Status::Unsupported);
        }
        self.authority.reserve_save_one(&o.life.scope, kind)
    }
    pub fn join_finished(&self, id: &ProducerId) -> Result<JoinedProducerProof, Status> {
        self.authority.witness().join_finished(id)
    }
}

pub struct NativeSavePause {
    pub(super) authority: Arc<NativeAuthority>,
    pub(super) inner: SavePause,
}
pub(super) enum SavePause {
    Preview(preview::PreviewPause),
    Save(Arc<SaveBarrier>),
}
pub(super) struct SaveBarrier {
    state: Mutex<SaveBarrierState>,
    changed: Condvar,
}
struct SaveBarrierState {
    released: bool,
    view: NativeSaveBarrierView,
}
pub(super) struct SaveSettlement(pub(super) Option<Arc<SaveBarrier>>);
impl Drop for SaveSettlement {
    fn drop(&mut self) {
        if let Some(b) = &self.0 {
            b.settle();
        }
    }
}
impl SaveBarrier {
    pub(super) fn new(session: u64, point: NativeSavePausePoint) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(SaveBarrierState {
                released: false,
                view: NativeSaveBarrierView {
                    entered: false,
                    settled: false,
                    session,
                    instance: 0,
                    pin: 0,
                    point,
                },
            }),
            changed: Condvar::new(),
        })
    }
    pub(super) fn enter_wait(&self, instance: u64, pin: u64) -> Result<(), Status> {
        let mut s = lock(&self.state)?;
        if s.view.entered || s.view.settled {
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
        if s.released {
            return Err(Status::NotReady);
        }
        s.released = true;
        self.changed.notify_all();
        Ok(())
    }
    fn wait(&self, timeout_ms: u64, settled: bool) -> Result<NativeSaveBarrierView, Status> {
        if !(1..=5000).contains(&timeout_ms) {
            return Err(Status::Invalid);
        }
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(timeout_ms))
            .ok_or(Status::Exhausted)?;
        let mut s = lock(&self.state)?;
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
                .changed
                .wait_timeout(s, deadline - now)
                .map_err(|_| Status::Internal)?
                .0;
        }
        Ok(s.view)
    }
}
impl NativeSavePause {
    fn wait(&self, ms: u64, settled: bool) -> Result<NativeSaveBarrierView, Status> {
        match &self.inner {
            SavePause::Save(b) => b.wait(ms, settled),
            SavePause::Preview(p) => {
                let v = if settled {
                    p.wait_until_settled(ms)?
                } else {
                    p.wait_until_entered(ms)?
                };
                Ok(NativeSaveBarrierView {
                    entered: v.entered,
                    settled: v.settled,
                    session: v.session,
                    instance: v.instance,
                    pin: v.pin,
                    point: NativeSavePausePoint::BeforeActivation,
                })
            }
        }
    }
    pub fn wait_until_entered(&self, ms: u64) -> Result<NativeSaveBarrierView, Status> {
        self.wait(ms, false)
    }
    pub fn wait_until_settled(&self, ms: u64) -> Result<NativeSaveBarrierView, Status> {
        self.wait(ms, true)
    }
}
impl SaveIssuer {
    pub fn claim_inactive_save_probe(
        &self,
        chrome: &PeerContext,
        pending: &PendingSaveInstance,
        pause: &NativeSavePause,
    ) -> Result<InactiveSaveProbeReservation, Status> {
        if !same(&self.authority, &pause.authority) {
            return Err(Status::Denied);
        }
        let SavePause::Preview(p) = &pause.inner else {
            return Err(Status::NotReady);
        };
        Ok(InactiveSaveProbeReservation {
            _inner: self
                .inner
                .claim_inactive_read_probe(chrome, &pending.inner, p)?,
        })
    }
}

impl SaveGateGuard<'_> {
    pub fn initialize_editor(
        &mut self,
        c: &NativeSaveCall,
        bytes: &[u8],
    ) -> Result<NativeEditor, Status> {
        let view = self.admit_call(c)?;
        if view.request.msg_type != 1 || !(64..=4160).contains(&bytes.len()) {
            return Err(Status::Invalid);
        }
        let h = EditorTextHeaderV0::decode_le(&bytes[..64]).map_err(|_| Status::Invalid)?;
        h.validate_bytes(&bytes[64..])
            .map_err(|_| Status::Invalid)?;
        let p = self.inner.gate.preview.as_ref().ok_or(Status::Denied)?;
        let row = p.row(&c.life.attachment)?;
        let current = self
            .inner
            .gate
            .save
            .as_ref()
            .ok_or(Status::Denied)?
            .current
            .get(&view.object_id)
            .cloned()
            .unwrap_or_else(|| row.view.pin.current.clone());
        if (h.selected_object_id, h.revision, h.content_hash)
            != (current.object_id, current.revision, current.content_hash)
        {
            return Err(Status::Denied);
        }
        self.issuer
            .authority
            .prune_save_calls(&mut self.inner.gate)?;
        let s = self.inner.gate.save.as_mut().ok_or(Status::Denied)?;
        if let Some(row) = s
            .documents
            .values()
            .find(|r| Arc::ptr_eq(&r.attachment, &c.life.attachment))
        {
            // Querying an existing editor never resets its privately edited bytes.
            return Ok(NativeEditor {
                authority: self.issuer.authority.clone(),
                life: row.life.clone(),
            });
        }
        if s.documents.len() >= 16 {
            return Err(Status::Exhausted);
        }
        let id = s.next_document.checked_add(1).ok_or(Status::Exhausted)?;
        let life = Arc::new(DocumentLife { id });
        let observation = NativeEditorObservation {
            actor: view.actor,
            document_id: id,
            bytes: bytes[64..].to_vec(),
            cursor: (bytes.len() - 64) as u32,
            selection: None,
            first_visible_line: 0,
            text_generation: 1,
            view_version: 1,
            confirmed_revision: h.revision,
            confirmed_hash: h.content_hash,
            phase: NativeEditorPhase::Loaded,
        };
        s.next_document = id;
        s.current.insert(current.object_id, current.clone());
        s.documents.insert(
            id,
            DocumentRow {
                life: life.clone(),
                attachment: c.life.attachment.clone(),
                object_id: current.object_id,
                view: observation,
                queued_intent: None,
                last_intent_id: None,
                selection_anchor: None,
                preferred_column: None,
                request_id: 0,
                key_sequence: 0,
                focus_epoch: 0,
                frame: None,
            },
        );
        Ok(NativeEditor {
            authority: self.issuer.authority.clone(),
            life,
        })
    }
    pub fn snapshot_for_intent<'a>(
        &self,
        i: &'a NativeSaveIntent,
    ) -> Result<&'a NativeDraftSnapshot, Status> {
        if !same(&self.issuer.authority, &i.authority) {
            return Err(Status::Denied);
        }
        let s = self.inner.gate.save.as_ref().ok_or(Status::Denied)?;
        if !s
            .intents
            .get(&i.life.id)
            .is_some_and(|r| Weak::ptr_eq(r, &Arc::downgrade(&i.life)))
        {
            return Err(Status::Denied);
        }
        self.inner
            .gate
            .preview
            .as_ref()
            .ok_or(Status::Denied)?
            .admit_instance(i.life.attachment.id)?;
        if Instant::now() >= i.deadline() {
            return Err(Status::Timeout);
        }
        for original in s.originals.values().filter_map(Weak::upgrade) {
            if original
                .intent
                .as_ref()
                .is_some_and(|intent| Arc::ptr_eq(intent, &i.life))
                && lock(&original.state)?.closed
            {
                return Err(Status::Stale);
            }
        }
        Ok(i.snapshot())
    }
    pub fn bind_source(
        &mut self,
        i: &NativeSaveIntent,
        b: &EditorSaveBindingV0,
        epoch: u64,
    ) -> Result<BoundNativeSource, Status> {
        let snapshot = self.snapshot_for_intent(i)?.view();
        self.inner
            .gate
            .preview
            .as_ref()
            .ok_or(Status::Denied)?
            .admit_instance(i.life.attachment.id)?;
        if Instant::now() >= i.deadline() {
            return Err(Status::Timeout);
        }
        b.validate().map_err(|_| Status::Invalid)?;
        let a = &b.allocation;
        let source = &b.source;
        let handle = Handle::unpack(source.source_object_id);
        if handle.kind != kernel_api::cap::HandleKind::Shmem
            || handle.index == 0
            || handle.generation == 0
            || handle.pack() != source.source_object_id
            || handle.generation != source.source_generation
            || epoch != snapshot.original_store_epoch
            || a.actor != snapshot.actor
            || a.selected_object_id != snapshot.object_id
            || a.selected_generation != snapshot.object_generation
            || a.expected_revision != snapshot.expected_revision
            || a.expected_content_hash != snapshot.expected_content_hash
            || source.byte_len != snapshot.byte_len
            || source.content_hash != snapshot.source_content_hash
        {
            return Err(Status::Denied);
        }
        let s = self.inner.gate.save.as_mut().ok_or(Status::Denied)?;
        if s.allocations.get(&i.life.id) != Some(&a.operation_id) {
            return Err(Status::Denied);
        }
        for original in s.originals.values().filter_map(Weak::upgrade) {
            if original
                .intent
                .as_ref()
                .is_some_and(|intent| Arc::ptr_eq(intent, &i.life))
                && lock(&original.state)?.closed
            {
                // A real terminal original cannot regain source rights through
                // its retained positive allocation or a fresh lookup result.
                return Err(Status::Stale);
            }
        }
        let live_ids = s
            .live_intents(&self.issuer.authority)?
            .into_iter()
            .map(|i| i.id)
            .collect::<std::collections::BTreeSet<_>>();
        s.sources
            .retain(|_, r| r.upgrade().is_some_and(|r| live_ids.contains(&r.intent.id)));
        if s.sources.len() >= 48 {
            return Err(Status::Exhausted);
        }
        if s.sources
            .values()
            .filter_map(Weak::upgrade)
            .any(|r| Arc::ptr_eq(&r.intent, &i.life))
        {
            return Err(Status::NotReady);
        }
        let id = s.next_source.checked_add(1).ok_or(Status::Exhausted)?;
        let life = Arc::new(SourceLife {
            intent: i.life.clone(),
            binding: b.clone(),
            epoch,
            _id: id,
            consumed: std::sync::atomic::AtomicBool::new(false),
        });
        s.next_source = id;
        s.sources.insert(id, Arc::downgrade(&life));
        Ok(BoundNativeSource {
            authority: self.issuer.authority.clone(),
            life,
        })
    }
    pub fn publish_allocation(
        &mut self,
        c: &NativeExecutionClaim,
        wire: &Envelope,
    ) -> Result<(), Status> {
        if !same(&self.issuer.authority, &c.authority) {
            return Err(Status::Denied);
        }
        let request = &c.life.view.request;
        let canonical =
            super::encode_envelope_wire(wire).and_then(|b| super::decode_envelope_wire(&b))?;
        if request.msg_type != 3
            || canonical.protocol != 368
            || canonical.msg_type != 4
            || canonical.handle != Handle::INVALID
            || canonical.payload[..8] != request.payload[..8]
        {
            return Err(Status::Denied);
        }
        let reply =
            kernel_api::generated::desktop_artifact_v1::AllocateSaveIdReply::decode(&canonical)?;
        if reply.status != 0 || reply.operation_id == 0 {
            return Err(Status::Invalid);
        }
        let intent = c.life.intent.as_ref().ok_or(Status::Denied)?;
        let mut state = lock(&c.life.state)?;
        if let Some(old) = &state.allocation {
            if super::encode_envelope_wire(old)? != super::encode_envelope_wire(&canonical)? {
                return Err(Status::Denied);
            }
            state.uncertain = false;
            state.closed |= Instant::now() >= intent.deadline;
            if let Some(d) = self
                .inner
                .gate
                .save
                .as_mut()
                .ok_or(Status::Denied)?
                .documents
                .get_mut(&intent.snapshot.data.view.document_id)
            {
                if d.view.phase != NativeEditorPhase::Unavailable {
                    d.view.phase = if state.closed {
                        NativeEditorPhase::Unsaved
                    } else {
                        NativeEditorPhase::Saving
                    };
                }
            }
            return Ok(());
        }
        let s = self.inner.gate.save.as_mut().ok_or(Status::Denied)?;
        if s.allocations.len() >= 48 && !s.allocations.contains_key(&intent.id) {
            return Err(Status::Exhausted);
        }
        if s.allocations.contains_key(&intent.id) {
            return Err(Status::NotReady);
        }
        s.allocations.insert(intent.id, reply.operation_id);
        state.uncertain = false;
        state.allocation = Some(canonical);
        state.phase = NativeOriginalState::AllocationSettled;
        state.closed |= Instant::now() >= intent.deadline;
        if let Some(doc) = s.documents.get_mut(&intent.snapshot.data.view.document_id) {
            if doc.view.phase != NativeEditorPhase::Unavailable {
                doc.view.phase = if state.closed {
                    NativeEditorPhase::Unsaved
                } else {
                    NativeEditorPhase::Saving
                };
            }
        }
        Ok(())
    }
    pub fn admit_bound_source(&self, source: &BoundNativeSource) -> Result<(), Status> {
        if !same(&self.issuer.authority, &source.authority) {
            return Err(Status::Denied);
        }
        let save = self.inner.gate.save.as_ref().ok_or(Status::Denied)?;
        let life = &source.life;
        let i = &life.intent;
        if save
            .sources
            .get(&life._id)
            .is_none_or(|w| !Weak::ptr_eq(w, &Arc::downgrade(life)))
            || save
                .intents
                .get(&i.id)
                .is_none_or(|w| !Weak::ptr_eq(w, &Arc::downgrade(i)))
            || save.allocations.get(&i.id) != Some(&life.binding.allocation.operation_id)
        {
            return Err(Status::Denied);
        }
        self.inner
            .gate
            .preview
            .as_ref()
            .ok_or(Status::Denied)?
            .admit_instance(i.attachment.id)?;
        if Instant::now() >= i.deadline {
            return Err(Status::Timeout);
        }
        if life.consumed.load(std::sync::atomic::Ordering::Acquire) {
            return Err(Status::Stale);
        }
        let snapshot = &i.snapshot.data.view;
        let current = save.current.get(&snapshot.object_id).ok_or(Status::Stale)?;
        if current.object_generation != snapshot.object_generation
            || current.store_epoch != life.epoch
            || life.epoch != snapshot.original_store_epoch
            || life.binding.allocation.actor != snapshot.actor
            || life.binding.allocation.selected_object_id != snapshot.object_id
            || life.binding.allocation.selected_generation != snapshot.object_generation
        {
            return Err(Status::Stale);
        }
        for c in save.originals.values().filter_map(Weak::upgrade) {
            if c.intent.as_ref().is_some_and(|a| Arc::ptr_eq(a, i)) {
                let state = lock(&c.state)?;
                if state.closed
                    || matches!(
                        state.phase,
                        NativeOriginalState::Permitted
                            | NativeOriginalState::Committed
                            | NativeOriginalState::DefinitiveNoncommit
                            | NativeOriginalState::ClosedUnpermitted
                    )
                {
                    return Err(Status::Stale);
                }
            }
        }
        Ok(())
    }
    pub fn issue_commit_permit(
        &mut self,
        c: &NativeExecutionClaim,
        b: &BoundNativeSource,
    ) -> Result<NativeCommitAdmission, Status> {
        if !same(&self.issuer.authority, &c.authority)
            || !same(&self.issuer.authority, &b.authority)
            || c.life
                .source
                .as_ref()
                .is_none_or(|s| !Arc::ptr_eq(s, &b.life))
        {
            return Err(Status::Denied);
        }
        self.inner
            .gate
            .preview
            .as_ref()
            .ok_or(Status::Denied)?
            .admit_instance(c.life.attachment.id)?;
        if Instant::now() >= c.life.view.deadline || Instant::now() >= b.life.intent.deadline {
            return Err(Status::Timeout);
        }
        let q = &c.life.view.request;
        let binding = &b.life.binding;
        if q.msg_type != 5
            || u64::from_le_bytes(q.payload[24..32].try_into().map_err(|_| Status::Invalid)?)
                != binding.allocation.expected_revision
            || u64::from_le_bytes(q.payload[32..40].try_into().map_err(|_| Status::Invalid)?)
                != binding.allocation.operation_id
            || u64::from_le_bytes(q.payload[40..48].try_into().map_err(|_| Status::Invalid)?)
                != binding.source.source_object_id
            || u32::from_le_bytes(q.payload[48..52].try_into().map_err(|_| Status::Invalid)?)
                != binding
                    .source
                    .byte_len
                    .checked_add(64)
                    .ok_or(Status::Exhausted)?
        {
            return Err(Status::Denied);
        }
        let mut state = lock(&c.life.state)?;
        if !matches!(
            state.phase,
            NativeOriginalState::Registered | NativeOriginalState::Submitted
        ) {
            return Err(Status::NotReady);
        }
        if b.life
            .consumed
            .swap(true, std::sync::atomic::Ordering::AcqRel)
        {
            return Err(Status::NotReady);
        }
        state.phase = NativeOriginalState::Permitted;
        Ok(NativeCommitAdmission {
            authority: self.issuer.authority.clone(),
            claim: c.life.clone(),
            source: b.life.clone(),
        })
    }
    pub fn publish_permitted(&mut self, p: &NativeCommitAdmission) -> Result<(), Status> {
        if !same(&self.issuer.authority, &p.authority)
            || lock(&p.claim.state)?.phase != NativeOriginalState::Permitted
        {
            return Err(Status::Denied);
        }
        if let Some(doc) = self
            .inner
            .gate
            .save
            .as_mut()
            .ok_or(Status::Denied)?
            .documents
            .get_mut(&p.source.intent.snapshot.data.view.document_id)
        {
            doc.view.phase = NativeEditorPhase::Saving;
        }
        Ok(())
    }
    pub fn publish_current_selection(
        &mut self,
        current: &StoreCurrentSelection,
    ) -> Result<(), Status> {
        let p = self.inner.gate.preview.as_ref().ok_or(Status::Denied)?;
        let object = p.objects.get(&current.object_id).ok_or(Status::Denied)?;
        if (
            object.owner_id,
            object.object_generation,
            object.store_epoch,
        ) != (
            current.owner.owner_id,
            current.object_generation,
            current.store_epoch,
        ) || !p.owners.values().any(|r| r.owner == current.owner)
            || current.revision == 0
        {
            return Err(Status::Denied);
        }
        let p = self.inner.gate.preview.as_mut().ok_or(Status::Denied)?;
        for pin in p
            .pins
            .values_mut()
            .filter(|r| r.view.current.object_id == current.object_id && !r.consumed)
        {
            if pin.view.current.revision != current.revision
                || pin.view.current.content_hash != current.content_hash
                || pin.view.current.store_epoch != current.store_epoch
            {
                pin.retired = true;
            }
        }
        self.inner
            .gate
            .save
            .as_mut()
            .ok_or(Status::Denied)?
            .current
            .insert(current.object_id, current.clone());
        Ok(())
    }
    pub fn publish_receipt(
        &mut self,
        c: &NativeExecutionClaim,
        source: &BoundNativeSource,
        bytes: &[u8],
        current: &StoreCurrentSelection,
    ) -> Result<(), Status> {
        if !same(&self.issuer.authority, &c.authority)
            || !same(&self.issuer.authority, &source.authority)
            || c.life
                .source
                .as_ref()
                .is_none_or(|s| !Arc::ptr_eq(s, &source.life))
        {
            return Err(Status::Denied);
        }
        let receipt = EditorSaveReceiptV0::decode_le(bytes).map_err(|_| Status::Invalid)?;
        receipt
            .validate_binding(source.binding())
            .map_err(|_| Status::Denied)?;
        if receipt.backend_service_epoch != source.life.epoch
            || current.object_id != source.life.binding.allocation.selected_object_id
            || current.object_generation != source.life.binding.allocation.selected_generation
        {
            return Err(Status::Denied);
        }
        let fixed: [u8; 176] = bytes.try_into().map_err(|_| Status::Invalid)?;
        let mut state = lock(&c.life.state)?;
        if let Some(old) = &state.receipt_bytes {
            if old != &fixed {
                return Err(Status::Denied);
            }
            self.publish_current_selection(current)?;
            return Ok(());
        }
        if receipt.outcome == EditorReceiptOutcomeV0::Committed
            && state.phase != NativeOriginalState::Permitted
        {
            return Err(Status::Denied);
        }
        // Validate every receipt/phase/object relation before publication effects.
        self.publish_current_selection(current)?;
        state.phase = match receipt.outcome {
            EditorReceiptOutcomeV0::Committed => NativeOriginalState::Committed,
            EditorReceiptOutcomeV0::DefinitiveNoncommit => NativeOriginalState::DefinitiveNoncommit,
        };
        state.receipt = Some(receipt.clone());
        state.receipt_bytes = Some(fixed);
        state.uncertain = false;
        let s = self.inner.gate.save.as_mut().ok_or(Status::Denied)?;
        let snapshot = &source.life.intent.snapshot.data;
        if let Some(doc) = s.documents.get_mut(&snapshot.view.document_id) {
            let exact_current = receipt.outcome == EditorReceiptOutcomeV0::Committed
                && current.revision == receipt.result_revision
                && current.content_hash == receipt.source_content_hash;
            if exact_current {
                doc.view.confirmed_revision = current.revision;
                doc.view.confirmed_hash = current.content_hash;
            }
            doc.view.phase = if receipt.outcome == EditorReceiptOutcomeV0::DefinitiveNoncommit {
                NativeEditorPhase::Conflict
            } else {
                // Receipt verification establishes the candidate only. A fresh
                // private frame and protected compose must publish Saved.
                NativeEditorPhase::Unsaved
            };
            doc.frame = None;
        }
        Ok(())
    }
}

impl SaveIssuer {
    pub fn retire_objects_for_fence(&self, objects: &[u64]) -> Result<(), Status> {
        if objects.is_empty() || objects.len() > 3 || objects.windows(2).any(|w| w[0] >= w[1]) {
            return Err(Status::Denied);
        }
        let mut gate = self.lock()?;
        let previous = {
            let p = gate.inner.gate.preview.as_ref().ok_or(Status::Denied)?;
            objects
                .iter()
                .map(|o| p.objects.get(o).cloned().ok_or(Status::Denied))
                .collect::<Result<Vec<_>, _>>()?
        };
        let held = self.authority.held_save_objects(objects)?;
        for old in previous {
            gate.inner
                .gate
                .save
                .as_mut()
                .ok_or(Status::Denied)?
                .fenced
                .entry(old.object_id)
                .or_insert_with(|| (old, held.clone()));
        }
        let mut result = Ok(());
        for object in objects {
            // This concrete helper retires the actual pins/attachments before
            // returning Exhausted solely for the display status increment.
            // Fencing must still join their owners after that retirement; keep
            // ordinary display callers' exhaustion behavior unchanged.
            match gate.inner.retire_source_object(*object) {
                Ok(()) | Err(Status::Exhausted) => {}
                Err(status) => {
                    if result.is_ok() {
                        result = Err(status);
                    }
                }
            }
        }
        if let Some(s) = &mut gate.inner.gate.save {
            for d in s
                .documents
                .values_mut()
                .filter(|d| objects.contains(&d.object_id))
            {
                d.view.phase = NativeEditorPhase::Unavailable;
                d.frame = None;
                d.queued_intent = None;
            }
        }
        result
    }
}
impl SaveGateGuard<'_> {
    pub fn validate_fenced_reenrollment(
        &self,
        objects: &[StoreObjectEnrollment],
        proofs: &[JoinedProducerProof],
    ) -> Result<(), Status> {
        if objects.is_empty()
            || objects.len() > 3
            || objects.windows(2).any(|w| w[0].object_id >= w[1].object_id)
            || proofs.len() > 64
        {
            return Err(Status::Denied);
        }
        let s = self.inner.gate.save.as_ref().ok_or(Status::Denied)?;
        let ids = objects.iter().map(|o| o.object_id).collect::<Vec<_>>();
        if !self.issuer.authority.held_save_objects(&ids)?.is_empty() {
            return Err(Status::NotReady);
        }
        let witness = self.issuer.authority.witness();
        let mut actual = BTreeMap::new();
        for proof in proofs {
            let v = witness.save_join_observation(proof)?;
            if actual
                .insert(v.producer_id, (proof.save_object, v))
                .is_some()
            {
                return Err(Status::Denied);
            }
        }
        for o in objects {
            let (old, held) = s.fenced.get(&o.object_id).ok_or(Status::Denied)?;
            if old.owner_id != o.owner_id
                || old.object_generation != o.object_generation
                || o.store_epoch != old.store_epoch.checked_add(1).ok_or(Status::Exhausted)?
            {
                return Err(Status::Denied);
            }
            for id in held {
                let (object, proof) = actual.get(id).ok_or(Status::NotReady)?;
                if object.is_none_or(|id| !ids.contains(&id)) || proof.producer_generation != 1 {
                    return Err(Status::Denied);
                }
            }
        }
        Ok(())
    }
    pub fn fenced_enrolled_owners(
        &self,
        objects: &[StoreObjectEnrollment],
        proofs: &[JoinedProducerProof],
    ) -> Result<Vec<PreviewOwner>, Status> {
        self.validate_fenced_reenrollment(objects, proofs)?;
        let preview = self.inner.gate.preview.as_ref().ok_or(Status::Denied)?;
        let mut owners = Vec::with_capacity(objects.len());
        for object in objects {
            let mut matches = preview
                .owners
                .values()
                .filter(|row| row.owner.owner_id == object.owner_id);
            let owner = matches.next().ok_or(Status::Denied)?;
            if matches.next().is_some() {
                return Err(Status::Denied);
            }
            owners.push(owner.owner.clone());
        }
        Ok(owners)
    }
    pub fn reenroll_fenced_objects(
        &mut self,
        objects: &[StoreObjectEnrollment],
        current: &[StoreCurrentSelection],
        proofs: &[JoinedProducerProof],
    ) -> Result<(), Status> {
        self.validate_fenced_reenrollment(objects, proofs)?;
        if current.len() != objects.len() {
            return Err(Status::Denied);
        }
        for (o, c) in objects.iter().zip(current) {
            if (o.object_id, o.object_generation, o.store_epoch, o.owner_id)
                != (
                    c.object_id,
                    c.object_generation,
                    c.store_epoch,
                    c.owner.owner_id,
                )
                || c.revision == 0
            {
                return Err(Status::Denied);
            }
            let p = self.inner.gate.preview.as_ref().ok_or(Status::Denied)?;
            if !p.owners.values().any(|r| r.owner == c.owner) {
                return Err(Status::Denied);
            }
        }
        let p = self.inner.gate.preview.as_mut().ok_or(Status::Denied)?;
        for o in objects {
            p.objects.insert(o.object_id, o.clone());
        }
        let s = self.inner.gate.save.as_mut().ok_or(Status::Denied)?;
        for c in current {
            s.current.insert(c.object_id, c.clone());
            s.fenced.remove(&c.object_id);
        }
        Ok(())
    }
}

impl SaveGate {
    // Gate is held. This derives observations exclusively from retained genuine
    // execution owners, the Core publication and the exact current frame join.
    pub(super) fn chrome(
        &self,
        p: &preview::PreviewGate,
        instance: u64,
        focus: u64,
        sequence: u64,
        status: u64,
    ) -> Result<NativeSaveChromeObservation, Status> {
        use artifact_store_schema::editor_native_save::{
            EditorNativeSaveChromeStateV0 as State, EditorNativeSaveChromeV0 as Chrome,
        };
        let row = p.attachments.get(&instance).ok_or(Status::Stale)?;
        let actor = &row.view.actor;
        let object = row.view.pin.current.object_id;
        let current = self.current.get(&object).unwrap_or(&row.view.pin.current);
        let doc = self
            .documents
            .values()
            .find(|d| d.attachment.id == instance);
        let current_intent = doc.and_then(|d| d.last_intent_id);
        let complete_current_binding = self.originals.values().filter_map(Weak::upgrade).any(|c| {
            c.view.request.msg_type == 5
                && c.source.is_some()
                && c.intent.as_ref().is_some_and(|i| {
                    Some(i.id) == current_intent
                        && doc
                            .is_some_and(|d| i.snapshot.data.view.document_id == d.view.document_id)
                })
        });
        let unavailable = row.retired || row.faulted;
        let mut pending = Vec::new();
        let mut chosen = None;
        for original in self.originals.values().filter_map(Weak::upgrade) {
            if original.view.actor.session_id != actor.session_id {
                continue;
            }
            let state = lock(&original.state)?;
            if state.uncertain
                && state.receipt.is_none()
                && matches!(original.view.request.msg_type, 3 | 5)
            {
                if pending.len() >= 48 {
                    return Err(Status::Exhausted);
                }
                pending.push(NativePendingOriginalObservation {
                    original_actor: original.view.actor.clone(),
                    object_id: original.view.object_id,
                    object_generation: original.view.object_generation,
                    intent_id: original.view.intent_id.ok_or(Status::Internal)?,
                    operation_id: original
                        .source
                        .as_ref()
                        .map(|s| s.binding.allocation.operation_id),
                    original_backend_epoch: original.view.store_epoch,
                    phase: if original.view.request.msg_type == 3 {
                        NativePendingOriginalPhase::AllocationUnknown
                    } else {
                        NativePendingOriginalPhase::CommitUnknown
                    },
                });
            }
            if original.view.object_id == object
                && current_intent
                    .is_none_or(|id| original.intent.as_ref().is_some_and(|i| i.id == id))
                && original.source.is_some()
                && (state.receipt.is_some() || state.uncertain)
            {
                chosen = Some(original.clone());
            }
        }
        pending.sort_by_key(|p| (p.object_id, p.original_backend_epoch, p.intent_id));
        let frame = doc.and_then(|d| d.frame.as_ref()).filter(|f| {
            f.sequence == sequence
                && f.focus_epoch == focus
                && doc.is_some_and(|d| {
                    f.actor == d.view.actor
                        && f.text_generation == d.view.text_generation
                        && f.view_version == d.view.view_version
                })
        });
        let saved = if let (Some(d), Some(original), Some(_)) = (doc, chosen.as_ref(), frame) {
            let state = lock(&original.state)?;
            if let (Some(receipt), Some(source)) = (&state.receipt, &original.source) {
                receipt.outcome == EditorReceiptOutcomeV0::Committed
                    && current.revision == receipt.result_revision
                    && current.content_hash == receipt.source_content_hash
                    && digest(&d.view.bytes) == receipt.source_content_hash
                    && d.view.confirmed_revision == current.revision
                    && d.view.confirmed_hash == current.content_hash
                    && (d.view.document_id == source.intent.snapshot.data.view.document_id
                        && d.view.text_generation
                            == source.intent.snapshot.data.view.text_generation
                        || d.view.text_generation == 1
                            && d.view.confirmed_revision == receipt.result_revision)
            } else {
                false
            }
        } else {
            false
        };
        let label = if unavailable {
            NativeEditorPhase::Unavailable
        } else if saved {
            NativeEditorPhase::Saved
        } else if let Some(d) = doc {
            if d.view.phase == NativeEditorPhase::Saved {
                NativeEditorPhase::Unsaved
            } else {
                d.view.phase.clone()
            }
        } else {
            NativeEditorPhase::Loaded
        };
        let mut record = Chrome {
            schema_version: 2,
            total_len: 248,
            state: if unavailable {
                State::Unavailable
            } else {
                State::NoSave
            },
            reserved: 0,
            actor: actor.clone(),
            selected_object_id: object,
            selected_object_generation: current.object_generation,
            current_store_epoch: current.store_epoch,
            original_backend_epoch: 0,
            operation_id: 0,
            source_handle: 0,
            source_generation: 0,
            expected_revision: 0,
            result_revision: 0,
            attachment_version: row.version,
            status_version: status,
            source_content_hash: [0; 32],
            expected_content_hash: [0; 32],
            receipt_sha256: [0; 32],
            source_body_len: 0,
            tail_reserved: 0,
        };
        if !unavailable {
            if let Some(original) = &chosen {
                let state = lock(&original.state)?;
                if let Some(source) = &original.source {
                    let binding = &source.binding;
                    record.actor = binding.allocation.actor.clone();
                    record.original_backend_epoch = source.epoch;
                    record.operation_id = binding.allocation.operation_id;
                    record.source_handle = binding.source.source_object_id;
                    record.source_generation = binding.source.source_generation;
                    record.expected_revision = binding.allocation.expected_revision;
                    record.expected_content_hash = binding.allocation.expected_content_hash;
                    record.source_content_hash = binding.source.content_hash;
                    record.source_body_len = binding.source.byte_len;
                    if let Some(receipt) = &state.receipt {
                        record.state = if receipt.outcome == EditorReceiptOutcomeV0::Committed {
                            State::Committed
                        } else {
                            State::DefinitiveNoncommit
                        };
                        record.result_revision = receipt.result_revision;
                        record.receipt_sha256 =
                            digest(state.receipt_bytes.as_ref().ok_or(Status::Internal)?);
                        record
                            .validate_receipt(
                                binding,
                                source.epoch,
                                state.receipt_bytes.as_ref().ok_or(Status::Internal)?,
                            )
                            .map_err(|_| Status::Denied)?;
                    } else {
                        record.state = State::Unknown;
                        record
                            .validate_binding(binding, source.epoch)
                            .map_err(|_| Status::Denied)?;
                    }
                }
            }
        }
        let record248 = if !unavailable && current_intent.is_some() && !complete_current_binding {
            // Allocation-only states have no schema2 representation. An older
            // Commit from this object, or another document's banner, cannot fill it.
            None
        } else {
            Some(record.encode_le().map_err(|_| Status::Invalid)?)
        };
        Ok(NativeSaveChromeObservation {
            current_frame_actor: actor.clone(),
            original_actor: chosen.as_ref().map(|c| c.view.actor.clone()),
            document_id: doc.map(|d| d.view.document_id),
            text_generation: doc.map(|d| d.view.text_generation),
            view_version: doc.map(|d| d.view.view_version),
            frame_sequence: sequence,
            focus_epoch: focus,
            attachment_version: row.version,
            status_version: status,
            label: label.clone(),
            current_document_phase: label,
            outstanding_original_count: pending.len() as u32,
            pending_originals: pending,
            record248,
        })
    }
}

impl Drop for NativeSaveIntent {
    fn drop(&mut self) {
        self.life
            .exposed
            .store(false, std::sync::atomic::Ordering::Release);
    }
}
impl SaveGate {
    fn intent_unresolved(&self, intent: &Arc<IntentLife>) -> Result<bool, Status> {
        let mut unresolved_commit = false;
        let mut terminal_commit = false;
        let mut unresolved_allocation = false;
        for original in self.originals.values().filter_map(Weak::upgrade) {
            if !original
                .intent
                .as_ref()
                .is_some_and(|i| Arc::ptr_eq(i, intent))
            {
                continue;
            }
            let state = lock(&original.state)?;
            let terminal = matches!(
                state.phase,
                NativeOriginalState::Committed
                    | NativeOriginalState::DefinitiveNoncommit
                    | NativeOriginalState::ClosedUnpermitted
            );
            match original.view.request.msg_type {
                5 => {
                    terminal_commit |= terminal;
                    unresolved_commit |= !terminal;
                }
                3 => {
                    unresolved_allocation |= !(terminal
                        || state.phase == NativeOriginalState::AllocationSettled
                            && state.closed
                            && !state.uncertain);
                }
                _ => {}
            }
        }
        // A terminal Commit resolves its preceding Allocate. A terminal Allocate
        // cannot resolve any still-permitted or uncertain Commit of this intent.
        Ok(unresolved_commit || !terminal_commit && unresolved_allocation)
    }
    pub(super) fn unresolved_document(&self, document: u64) -> Result<bool, Status> {
        for intent in self.intents.values().filter_map(Weak::upgrade) {
            if intent.snapshot.data.view.document_id == document
                && self.intent_unresolved(&intent)?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub(super) fn live_intents(
        &self,
        authority: &NativeAuthority,
    ) -> Result<Vec<Arc<IntentLife>>, Status> {
        use std::sync::atomic::Ordering;
        let mut live = Vec::new();
        for i in self.intents.values().filter_map(Weak::upgrade) {
            let call = self
                .calls
                .values()
                .filter_map(|c| c.life.upgrade())
                .any(|c| {
                    c.intent.as_ref().is_some_and(|a| Arc::ptr_eq(a, &i))
                        || c.recovery.as_ref().is_some_and(|original| {
                            original.intent.as_ref().is_some_and(|a| Arc::ptr_eq(a, &i))
                        })
                });
            let queued = self
                .documents
                .values()
                .any(|d| d.queued_intent.as_ref().is_some_and(|a| Arc::ptr_eq(a, &i)));
            let mut producer = false;
            for original in self.originals.values().filter_map(Weak::upgrade) {
                if original.intent.as_ref().is_some_and(|a| Arc::ptr_eq(a, &i))
                    && authority.has_save_origin(original.scope.id)?
                {
                    producer = true;
                    break;
                }
            }
            if i.exposed.load(Ordering::Acquire)
                || call
                || queued
                || producer
                || self.intent_unresolved(&i)?
            {
                live.push(i);
            }
        }
        Ok(live)
    }
}

pub enum NativeSaveDeliveryOutcome<'a> {
    ReplyLost,
    AllocateRejected { wire: &'a Envelope },
    AllocatedSourceRejected,
    NoAllocation { joined: &'a [JoinedProducerProof] },
}
impl SaveGateGuard<'_> {
    pub fn publish_original_delivery(
        &mut self,
        c: &NativeExecutionClaim,
        outcome: NativeSaveDeliveryOutcome<'_>,
    ) -> Result<(), Status> {
        if !same(&self.issuer.authority, &c.authority)
            || !matches!(c.life.view.request.msg_type, 3 | 5)
        {
            return Err(Status::Denied);
        }
        let save = self.inner.gate.save.as_ref().ok_or(Status::Denied)?;
        if save
            .originals
            .get(&c.life.view.origin_id)
            .is_none_or(|w| !Weak::ptr_eq(w, &Arc::downgrade(&c.life)))
        {
            return Err(Status::Denied);
        }
        let intent = c.life.intent.as_ref().ok_or(Status::Denied)?;
        if c.life.view.actor != intent.snapshot.data.view.actor
            || c.life.view.object_id != intent.snapshot.data.view.object_id
            || c.life.view.object_generation != intent.snapshot.data.view.object_generation
            || c.life.view.store_epoch != intent.snapshot.data.view.original_store_epoch
        {
            return Err(Status::Denied);
        }
        let definite = !matches!(outcome, NativeSaveDeliveryOutcome::ReplyLost);
        let mut state = lock(&c.life.state)?;
        let phase = match outcome {
            NativeSaveDeliveryOutcome::ReplyLost => {
                if state.receipt.is_some() {
                    return Ok(());
                }
                state.uncertain = true;
                if c.life.view.request.msg_type == 3 {
                    NativeEditorPhase::AllocationUnknown
                } else {
                    NativeEditorPhase::Unknown
                }
            }
            NativeSaveDeliveryOutcome::AllocateRejected { wire } => {
                if c.life.view.request.msg_type != 3
                    || !state.closed
                    || state.allocation.is_some()
                    || state.receipt.is_some()
                    || state.phase == NativeOriginalState::Permitted
                {
                    return Err(Status::Denied);
                }
                let canonical = super::decode_envelope_wire(&super::encode_envelope_wire(wire)?)?;
                if canonical.protocol != 368
                    || canonical.msg_type != 4
                    || canonical.handle != Handle::INVALID
                    || canonical.payload[..8] != c.life.view.request.payload[..8]
                {
                    return Err(Status::Denied);
                }
                let reply =
                    kernel_api::generated::desktop_artifact_v1::AllocateSaveIdReply::decode(
                        &canonical,
                    )?;
                if reply.operation_id != 0 || !matches!(reply.status, 1 | 2 | 3 | 4 | 5 | 7 | 8) {
                    return Err(Status::Denied);
                }
                state.uncertain = false;
                state.phase = NativeOriginalState::ClosedUnpermitted;
                if reply.status == 7 {
                    NativeEditorPhase::Conflict
                } else {
                    NativeEditorPhase::Unsaved
                }
            }
            NativeSaveDeliveryOutcome::AllocatedSourceRejected => {
                if c.life.view.request.msg_type != 3
                    || c.life.source.is_some()
                    || state.receipt.is_some()
                    || state.phase != NativeOriginalState::AllocationSettled
                {
                    return Err(Status::Denied);
                }
                let allocation = state.allocation.as_ref().ok_or(Status::Denied)?;
                let canonical =
                    super::decode_envelope_wire(&super::encode_envelope_wire(allocation)?)?;
                if canonical.protocol != 368
                    || canonical.msg_type != 4
                    || canonical.handle != Handle::INVALID
                    || canonical.payload[..8] != c.life.view.request.payload[..8]
                {
                    return Err(Status::Denied);
                }
                let reply =
                    kernel_api::generated::desktop_artifact_v1::AllocateSaveIdReply::decode(
                        &canonical,
                    )?;
                if reply.status != 0
                    || reply.operation_id == 0
                    || save.allocations.get(&intent.id) != Some(&reply.operation_id)
                    || save
                        .originals
                        .values()
                        .filter_map(Weak::upgrade)
                        .any(|original| {
                            original.view.request.msg_type == 5
                                && original
                                    .intent
                                    .as_ref()
                                    .is_some_and(|i| Arc::ptr_eq(i, intent))
                        })
                    || save
                        .calls
                        .values()
                        .filter_map(|row| row.life.upgrade())
                        .any(|call| {
                            call.view.request.msg_type == 5
                                && call.intent.as_ref().is_some_and(|i| Arc::ptr_eq(i, intent))
                        })
                    || save
                        .sources
                        .values()
                        .filter_map(Weak::upgrade)
                        .any(|source| Arc::ptr_eq(&source.intent, intent))
                {
                    return Err(Status::Denied);
                }
                // Known positive allocation is retained. This owner-only result
                // closes source/first-Commit continuation, never proves absence.
                state.closed = true;
                state.uncertain = false;
                NativeEditorPhase::Unsaved
            }
            NativeSaveDeliveryOutcome::NoAllocation { joined } => {
                if c.life.view.request.msg_type != 3
                    || state.allocation.is_some()
                    || state.receipt.is_some()
                    || state.phase == NativeOriginalState::Permitted
                {
                    return Err(Status::Denied);
                }
                if self.issuer.authority.has_save_origin(c.life.scope.id)? {
                    return Err(Status::NotReady);
                }
                let installed = lock(&c.life.scope.save_installed)?.clone();
                if joined.len() > 64 || joined.len() != installed.len() {
                    return Err(Status::Denied);
                }
                let witness = self.issuer.authority.witness();
                let mut seen = std::collections::BTreeSet::new();
                for proof in joined {
                    let v = witness.save_join_observation(proof)?;
                    if !Weak::ptr_eq(&proof.origin, &Arc::downgrade(&c.life.scope))
                        || !seen.insert(v.producer_id)
                        || installed.get(&v.producer_id) != Some(&(v.kind, v.producer_generation))
                    {
                        return Err(Status::Denied);
                    }
                }
                state.uncertain = false;
                state.closed = true;
                state.phase = NativeOriginalState::ClosedUnpermitted;
                NativeEditorPhase::Unsaved
            }
        };
        let save = self.inner.gate.save.as_mut().ok_or(Status::Denied)?;
        if definite {
            if let Some(row) = save.calls.get_mut(&c.life.view.origin_id) {
                row.closed = true;
            }
        }
        if let Some(d) = save
            .documents
            .get_mut(&intent.snapshot.data.view.document_id)
        {
            if d.view.phase != NativeEditorPhase::Unavailable {
                d.view.phase = phase;
            }
        }
        Ok(())
    }
}
