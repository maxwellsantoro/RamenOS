//! Default-off Store-owned native Save adapter.
//! Opaque Desktop rows carry admission; Store owns the selected object and bytes.
//! Diagnostic records do not grant current authority or establish containment.
use super::native_preview::{
    ExposeObservation, InitObservation, PreviewExposeStage, PreviewInitStage,
};
use super::*;
use desktop_service::dev::Status as DesktopStatus;
use desktop_service::editor_dev::{
    DesktopSaveLaunch, JoinedProducerProof, NativeSaveCall, NativeSaveCallView,
    NativeSaveEnrollment, NativeSaveEnrollmentFailure, NativeSavePause, PeerContext,
    PendingSaveInstance, ProducerId, RegistryWitness, SaveActivationStamp, SaveActivationView,
    SaveIssuer, SavePendingRetention, SaveSelectionPin, StoreCurrentSelection,
    StoreObjectEnrollment, StoreSaveInstallation,
};
use std::collections::BTreeSet;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

fn desktop_status(status: DesktopStatus) -> StoreStatus {
    match status {
        DesktopStatus::Ok => StoreStatus::Ok,
        DesktopStatus::Denied => StoreStatus::Denied,
        DesktopStatus::Invalid => StoreStatus::Invalid,
        DesktopStatus::Unsupported => StoreStatus::Unsupported,
        DesktopStatus::Stale => StoreStatus::Stale,
        DesktopStatus::Exhausted => StoreStatus::Exhausted,
        DesktopStatus::NotReady => StoreStatus::NotReady,
        DesktopStatus::Timeout => StoreStatus::Timeout,
        DesktopStatus::Conflict
        | DesktopStatus::Disconnected
        | DesktopStatus::Unknown
        | DesktopStatus::Internal => StoreStatus::Internal,
    }
}
fn native<T>(result: Result<T, DesktopStatus>) -> Result<T, StoreStatus> {
    result.map_err(desktop_status)
}
fn checked_lock<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, StoreStatus> {
    mutex.lock().map_err(|_| StoreStatus::Internal)
}

pub(super) struct SaveCore {
    pub(super) witness: Arc<RegistryWitness>,
    issuer: Arc<SaveIssuer>,
    tags: Mutex<BTreeMap<u64, Arc<SaveGrant>>>,
    producers: Mutex<Vec<ProducerId>>,
    originals: Mutex<Vec<Arc<SaveOriginal>>>,
    captures: Mutex<CaptureLedger>,
    sources: Mutex<BTreeMap<u64, std::sync::Weak<SourceOwner>>>,
    fence_joins: Mutex<FenceProofStore>,
    delivered_fence_joins: Mutex<BTreeSet<u64>>,
}
#[derive(Default)]
struct FenceProofStore {
    proofs: Vec<(ProducerFence, JoinedProducerProof)>,
    joining: BTreeSet<u64>,
}
struct SaveGrant {
    grant: Arc<Grant>,
    object_generation: u64,
}
pub struct PreparedNativeSaveStore {
    host: StoreHost,
    controller: FixtureController,
    attachment: Arc<SaveCore>,
}
pub type NativeSaveInitObservation = InitObservation;
pub type NativeSaveExposeObservation = ExposeObservation;
struct InitOwner {
    witness: Option<RegistryWitness>,
    issuer: Option<SaveIssuer>,
    _enrollment_failure: Option<NativeSaveEnrollmentFailure>,
    core: Option<Arc<Core>>,
    stage: PreviewInitStage,
    root: ReadRootObservation,
    root_path: Option<PathBuf>,
    panicked: bool,
}
pub struct NativeSaveInitFailure {
    status: StoreStatus,
    held: InitOwner,
}
impl NativeSaveInitFailure {
    pub fn status(&self) -> StoreStatus {
        self.status
    }
    pub fn observation(&self) -> InitObservation {
        InitObservation {
            stage: self.held.stage,
            status: self.status,
            panicked: self.held.panicked,
            root: self.held.root,
            root_path: self.held.root_path.clone(),
            core_retained: self.held.core.is_some(),
            issuer_retained: self.held.issuer.is_some()
                || self
                    .held
                    .core
                    .as_ref()
                    .is_some_and(|core| lock(&core.native_save).is_some()),
        }
    }
}
pub struct NativeSaveExposeFailure {
    status: StoreStatus,
    stage: PreviewExposeStage,
    // These are real retained owners; observation flags cannot create them.
    _core: Arc<Core>,
    pending: Option<SavePendingRetention>,
    tag: Option<Arc<SaveGrant>>,
}
impl NativeSaveExposeFailure {
    pub fn status(&self) -> StoreStatus {
        self.status
    }
    pub fn observation(&self) -> ExposeObservation {
        ExposeObservation {
            stage: self.stage,
            status: self.status,
            pending_retained: self.pending.is_some(),
            inactive_store_grant_retained: self.tag.is_some(),
            // Preparation failures have no successful activation stamp;
            // delivery denials disclose no foreign activation observation.
            active: false,
        }
    }
}
pub struct NativeSaveActivation {
    core: Arc<Core>,
    attachment: Arc<SaveCore>,
    tag: Arc<SaveGrant>,
    stamp: SaveActivationStamp,
    endpoint: EditorEndpoint,
}
pub struct PairedNativeSaveLaunch {
    desktop: DesktopSaveLaunch,
    endpoint: EditorEndpoint,
    _tag: Arc<SaveGrant>,
}
impl PairedNativeSaveLaunch {
    pub fn desktop(&self) -> &DesktopSaveLaunch {
        &self.desktop
    }
    pub fn endpoint(&self) -> &EditorEndpoint {
        &self.endpoint
    }
}

impl StoreFixture {
    pub fn prepare_native_save(
        root: &Path,
        profile: NativeSaveFixtureProfile,
        witness: RegistryWitness,
        enrollment: NativeSaveEnrollment,
    ) -> Result<PreparedNativeSaveStore, NativeSaveInitFailure> {
        // Nothing inside the unwind boundary can silently drop the real owner.
        let mut held = InitOwner {
            witness: Some(witness),
            issuer: None,
            _enrollment_failure: None,
            core: None,
            stage: PreviewInitStage::Validate,
            root: ReadRootObservation::NotAttempted,
            root_path: (root.as_os_str().len() <= 4096).then(|| root.to_path_buf()),
            panicked: false,
        };
        // Claim is pure and precedes the narrow IO/unwind boundary. Both
        // successful issuer and ordinary opaque failure remain in the actual
        // outside owner; retaining only a status would drop enrollment ownership.
        match enrollment.claim_for_store_constructor() {
            Ok(issuer) => held.issuer = Some(issuer),
            Err(error) => {
                let status = desktop_status(error.status());
                held._enrollment_failure = Some(error);
                return Err(NativeSaveInitFailure { status, held });
            }
        }
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let issuer = held.issuer.as_ref().ok_or(StoreStatus::Internal)?;
            native(issuer.validate_witness(held.witness.as_ref().ok_or(StoreStatus::Internal)?))?;
            // Actual owner/session/object matching occurs in the opaque Gate,
            // before IO. These supplied records cannot mint a Desktop owner.
            if !(1..=3).contains(&profile.initial_objects.len()) {
                return Err(StoreStatus::Invalid);
            }
            let objects = profile
                .initial_objects
                .iter()
                .map(|object| StoreObjectEnrollment {
                    owner_id: object.owner_id,
                    object_id: object.object_id,
                    object_generation: object.generation,
                    store_epoch: 1,
                })
                .collect::<Vec<_>>();
            native(native(issuer.lock())?.enroll_objects(&objects))?;
            held.stage = PreviewInitStage::PrepareCore;
            let profile = FixtureProfile {
                origin_ms: profile.origin_ms,
                initial_objects: profile.initial_objects,
                fixture_signing_seed: profile.fixture_signing_seed,
                fixture_key_id: profile.fixture_key_id,
            };
            let core = Self::prepare_core_bounded(root, profile, true, 3)?;
            held.core = Some(core.clone());
            let attachment = Arc::new(SaveCore {
                witness: Arc::new(held.witness.take().ok_or(StoreStatus::Internal)?),
                issuer: Arc::new(held.issuer.take().ok_or(StoreStatus::Internal)?),
                tags: Mutex::new(BTreeMap::new()),
                producers: Mutex::new(vec![]),
                originals: Mutex::new(vec![]),
                captures: Mutex::new(CaptureLedger {
                    next: 1,
                    reserved: 0,
                    rows: vec![],
                }),
                sources: Mutex::new(BTreeMap::new()),
                fence_joins: Mutex::new(FenceProofStore::default()),
                delivered_fence_joins: Mutex::new(BTreeSet::new()),
            });
            *checked_lock(&core.native_save)? = Some(attachment.clone());
            held.stage = PreviewInitStage::CreateRoot;
            held.root = ReadRootObservation::AttemptUncertain;
            fs::DirBuilder::new()
                .mode(0o700)
                .create(&core.root)
                .map_err(|_| StoreStatus::Internal)?;
            held.root = ReadRootObservation::ReportedCreated;
            held.stage = PreviewInitStage::Provision;
            Self::provision_objects(&core)?;
            held.stage = PreviewInitStage::Enroll;
            for object in core.objects.values() {
                let state = checked_lock(&object.state)?;
                core.validate_save_selection(object, &state)?;
            }
            Ok(PreparedNativeSaveStore {
                host: StoreHost { core: core.clone() },
                controller: FixtureController { core },
                attachment,
            })
        }));
        match outcome {
            Ok(Ok(prepared)) => Ok(prepared),
            Ok(Err(status)) => Err(NativeSaveInitFailure { status, held }),
            Err(_) => {
                held.panicked = true;
                Err(NativeSaveInitFailure {
                    status: StoreStatus::Internal,
                    held,
                })
            }
        }
    }
}
impl Core {
    fn validate_save_selection(
        &self,
        object: &Object,
        state: &ObjectState,
    ) -> Result<(), StoreStatus> {
        if !self.native_only || self.closed.load(Ordering::Acquire) || state.preview_faulted {
            return Err(StoreStatus::Stale);
        }
        schema(state.journal.validate())?;
        let initial = self
            .profile
            .initial_objects
            .iter()
            .find(|row| row.object_id == object.id)
            .ok_or(StoreStatus::Denied)?;
        let selected = &state.journal.current;
        if state.journal.owner_id != initial.owner_id
            || state.journal.selected_object_id != object.id
            || state.journal.selected_generation != initial.generation
            || selected.content_hash != storage::hash(&state.bytes)
        {
            return Err(StoreStatus::Denied);
        }
        Ok(())
    }
}
fn current_selection(
    core: &Core,
    object: &Object,
    state: &ObjectState,
    owner: desktop_service::editor_dev::PreviewOwner,
) -> Result<StoreCurrentSelection, StoreStatus> {
    core.validate_save_selection(object, state)?;
    if state.journal.owner_id != owner.owner_id {
        return Err(StoreStatus::Denied);
    }
    Ok(StoreCurrentSelection {
        owner,
        object_id: object.id,
        object_generation: state.journal.selected_generation,
        store_epoch: state.journal.service_epoch,
        revision: state.journal.current.revision,
        content_hash: state.journal.current.content_hash,
    })
}
impl PreparedNativeSaveStore {
    pub fn host(&self) -> &StoreHost {
        &self.host
    }
    pub fn controller(&self) -> &FixtureController {
        &self.controller
    }
    pub fn pin_selection(
        &self,
        chrome: &PeerContext,
        object_id: u64,
        ttl_ms: u64,
    ) -> Result<SaveSelectionPin, StoreStatus> {
        let entered = Instant::now();
        // Authenticate before disclosing selected ownership, then release Gate
        // before acquiring Store enforcing locks in their forward order.
        let owner = native(native(self.attachment.issuer.lock())?.preview_owner(chrome))?;
        if !(1..=30000).contains(&ttl_ms) {
            return Err(StoreStatus::Invalid);
        }
        let object = self
            .host
            .core
            .objects
            .values()
            .find(|object| {
                object.id == object_id
                    && self
                        .host
                        .core
                        .profile
                        .initial_objects
                        .iter()
                        .any(|initial| {
                            initial.object_id == object.id && initial.owner_id == owner.owner_id
                        })
            })
            .cloned()
            .ok_or(StoreStatus::Denied)?;
        let _registry = checked_lock(&self.host.core.registry)?;
        let state = checked_lock(&object.state)?;
        let mut guard = native(self.attachment.issuer.lock())?;
        let checked_owner = native(guard.preview_owner(chrome))?;
        if owner.owner_id != checked_owner.owner_id
            || owner.session_id != checked_owner.session_id
            || owner.session_generation != checked_owner.session_generation
        {
            return Err(StoreStatus::Denied);
        }
        native(guard.reserve_pin_started(
            &checked_owner,
            current_selection(&self.host.core, &object, &state, checked_owner.clone())?,
            ttl_ms,
            self.host.core.clock.load(Ordering::Acquire),
            entered,
        ))
    }
    pub fn activate_save(
        &self,
        pending: &PendingSaveInstance,
    ) -> Result<NativeSaveActivation, NativeSaveExposeFailure> {
        let core = &self.host.core;
        let mut stage = PreviewExposeStage::Validate;
        let mut retained = None;
        let mut attempt = None;
        // Retain the genuine complete owner before any fallible own-lifecycle
        // validation or Store lock. This same-Authority private holder grants
        // no admission and remains meaningful even when its row is retired.
        let pending_owner = native(self.attachment.issuer.retain_pending_for_error(pending))
            .map_err(|status| NativeSaveExposeFailure {
                status,
                stage,
                _core: core.clone(),
                pending: None,
                tag: None,
            })?;
        let outcome = (|| {
            let view = native(native(self.attachment.issuer.lock())?.view_pending(pending))?;
            let object = core.object(view.pin.current.object_id)?;
            let mut registry = checked_lock(&core.registry)?;
            let state = checked_lock(&object.state)?;
            let mut guard = native(self.attachment.issuer.lock())?;
            let view = native(guard.view_pending(pending))?;
            core.validate_save_selection(&object, &state)?;
            check_pending(&view, &object, &state)?;
            stage = PreviewExposeStage::ReserveStoreGrant;
            // These keys come only from the genuine same-Gate pending actor.
            // Reserve the actual bounded Store registry before staging; failed
            // activation retains the pending/grant/Core owners and their rows.
            schema(view.actor.validate())?;
            let session = (view.actor.session_id, view.actor.session_generation);
            let instance = instance_key(&view.actor);
            if registry
                .sessions
                .get(&session)
                .is_some_and(|owner| *owner != view.actor.owner_id)
                || registry
                    .instances
                    .get(&instance)
                    .is_some_and(|owner| *owner != view.actor.owner_id)
            {
                return Err(StoreStatus::Denied);
            }
            if (!registry.sessions.contains_key(&session) && registry.sessions.len() >= 2)
                || (!registry.instances.contains_key(&instance) && registry.instances.len() >= 16)
                || registry.grants.len() >= 64
                || checked_lock(&self.attachment.tags)?.len() >= 64
            {
                return Err(StoreStatus::Exhausted);
            }
            if registry
                .grants
                .values()
                .any(|grant| grant.actor == view.actor)
            {
                return Err(StoreStatus::NotReady);
            }
            let handle = Core::new_handle(&mut registry, HandleKind::Ipc)?;
            let grant = Arc::new(Grant {
                #[cfg(feature = "editor_native_read_v0_dev")]
                native_read: None,
                handle,
                actor: view.actor.clone(),
                object: object.id,
                rights: 7,
                class: EndpointClass::Artifact,
                recovery_op: None,
                epoch: state.journal.service_epoch,
                expires: view.instance_expires_at_ms,
                alive: AtomicBool::new(true),
                invocation: AtomicBool::new(false),
                drop_reply: AtomicBool::new(false),
            });
            let tag = Arc::new(SaveGrant {
                grant: grant.clone(),
                object_generation: state.journal.selected_generation,
            });
            registry.sessions.insert(session, view.actor.owner_id);
            registry.instances.insert(instance, view.actor.owner_id);
            registry.grants.insert(handle.pack(), grant.clone());
            checked_lock(&self.attachment.tags)?.insert(handle.pack(), tag.clone());
            retained = Some(tag.clone());
            stage = PreviewExposeStage::PrepareRecord;
            native(guard.stage_installation(pending, installation(&tag)))?;
            // No enforcing lock is retained while the supported setup actor
            // waits. The final transition repeats all Store/Gate checks.
            drop(guard);
            drop(state);
            drop(registry);
            attempt = Some(native(
                self.attachment
                    .issuer
                    .begin_save_activation_attempt(pending),
            )?);
            native(
                attempt
                    .as_ref()
                    .ok_or(StoreStatus::Internal)?
                    .wait_before_activation(),
            )?;
            stage = PreviewExposeStage::Activate;
            let registry = checked_lock(&core.registry)?;
            let state = checked_lock(&object.state)?;
            let mut guard = native(self.attachment.issuer.lock())?;
            let view = native(guard.view_pending(pending))?;
            core.validate_save_selection(&object, &state)?;
            check_pending(&view, &object, &state)?;
            if registry.sessions.get(&session) != Some(&view.actor.owner_id)
                || registry.instances.get(&instance) != Some(&view.actor.owner_id)
                || grant.actor != view.actor
                || !registry
                    .grants
                    .get(&handle.pack())
                    .is_some_and(|row| Arc::ptr_eq(row, &grant))
            {
                return Err(StoreStatus::Denied);
            }
            // All fallible preparation and allocation has preceded this flag.
            let endpoint = EditorEndpoint {
                peer: EditorPeer {
                    core: core.clone(),
                    grant,
                },
                handle,
            };
            let stamp = native(guard.activate(pending, installation(&tag)))?;
            Ok(NativeSaveActivation {
                core: core.clone(),
                attachment: self.attachment.clone(),
                tag,
                stamp,
                endpoint,
            })
        })();
        let result = outcome.map_err(|status| NativeSaveExposeFailure {
            status,
            stage,
            _core: core.clone(),
            pending: Some(pending_owner),
            tag: retained,
        });
        // The attempt remains owned through the terminal outcome. Enforcing
        // guards are gone before this unit observational settlement; unwind
        // uses the same opaque owner's Drop fallback.
        if let Some(attempt) = attempt {
            attempt.finish();
        }
        result
    }
}
fn installation(tag: &SaveGrant) -> StoreSaveInstallation {
    StoreSaveInstallation {
        store_handle: tag.grant.handle,
        object_id: tag.grant.object,
        object_generation: tag.object_generation,
        store_epoch: tag.grant.epoch,
    }
}
fn check_pending(
    view: &desktop_service::editor_dev::SavePendingView,
    object: &Object,
    state: &ObjectState,
) -> Result<(), StoreStatus> {
    if state.preview_faulted {
        return Err(StoreStatus::Stale);
    }
    if view.actor.owner_id != state.journal.owner_id
        || view.pin.current.owner.owner_id != view.actor.owner_id
        || view.pin.current.object_id != object.id
        || view.pin.current.object_generation != state.journal.selected_generation
        || view.pin.current.store_epoch != state.journal.service_epoch
        || view.pin.current.revision != state.journal.current.revision
        || view.pin.current.content_hash != state.journal.current.content_hash
    {
        return Err(StoreStatus::Denied);
    }
    Ok(())
}
fn check_activation(
    tag: &SaveGrant,
    state: &ObjectState,
    view: &SaveActivationView,
) -> Result<(), StoreStatus> {
    if state.preview_faulted || !tag.grant.alive.load(Ordering::Acquire) {
        return Err(StoreStatus::Stale);
    }
    if tag.grant.actor != view.actor
        || tag.grant.handle != view.store_handle
        || tag.grant.object != view.object_id
        || tag.object_generation != view.object_generation
        || tag.grant.epoch != view.store_epoch
        || tag.grant.expires != view.instance_expires_at_ms
        || state.journal.owner_id != view.actor.owner_id
        || state.journal.selected_generation != view.object_generation
        || state.journal.service_epoch != view.store_epoch
    {
        return Err(StoreStatus::Denied);
    }
    Ok(())
}
impl NativeSaveActivation {
    pub fn pair_launch(
        &self,
        launch: &mut Option<DesktopSaveLaunch>,
    ) -> Result<PairedNativeSaveLaunch, NativeSaveExposeFailure> {
        let outcome = (|| {
            let held = launch.as_ref().ok_or(StoreStatus::NotReady)?;
            let registry = checked_lock(&self.core.registry)?;
            let object = self.core.object(self.tag.grant.object)?;
            let state = checked_lock(&object.state)?;
            let guard = native(self.attachment.issuer.lock())?;
            let view = native(guard.match_activation(&self.stamp, held.stamp()))?;
            check_activation(&self.tag, &state, &view)?;
            if !registry
                .grants
                .get(&self.tag.grant.handle.pack())
                .is_some_and(|grant| Arc::ptr_eq(grant, &self.tag.grant))
            {
                return Err(StoreStatus::Denied);
            }
            // Move only after all foreign/lifetime checks, keeping Some intact
            // on denial. No actual Store peer is reconstructed from view data.
            Ok(PairedNativeSaveLaunch {
                desktop: launch.take().ok_or(StoreStatus::NotReady)?,
                endpoint: EditorEndpoint {
                    peer: self.endpoint.peer.clone(),
                    handle: self.endpoint.handle,
                },
                _tag: self.tag.clone(),
            })
        })();
        outcome.map_err(|status| NativeSaveExposeFailure {
            status,
            stage: PreviewExposeStage::Deliver,
            _core: self.core.clone(),
            pending: None,
            tag: None,
        })
    }
}

pub struct NativeSaveFixtureProfile {
    pub origin_ms: u64,
    pub initial_objects: Vec<InitialObject>,
    pub fixture_signing_seed: [u8; 32],
    pub fixture_key_id: String,
}

pub use desktop_service::editor_dev::NativeJoinObservation;
use desktop_service::editor_dev::{
    BoundNativeSource, NativeCommitAdmission, NativeEditor, NativeExecutionClaim, NativeSaveIntent,
    OriginalRecoveryOrigin, ProducerKind as NativeProducerKind,
};

struct SaveOriginal {
    core: std::sync::Weak<Core>,
    attachment: std::sync::Weak<SaveCore>,
    grant: Arc<Grant>,
    tag: Arc<SaveGrant>,
    claim: NativeExecutionClaim,
    recovery: OriginalRecoveryOrigin,
    request: Envelope,
    view: NativeSaveCallView,
    claimed: AtomicBool,
    state: Mutex<OriginalState>,
}
struct OriginalState {
    wire: Option<Envelope>,
    allocation: Option<EditorSaveAllocationV0>,
    binding: Option<EditorSaveBindingV0>,
    bound: Option<Arc<BoundNativeSource>>,
    completion: OriginalTicketCompletion,
    closed: bool,
    commit_dispatched: bool,
    allocation_delivered: bool,
    dispatcher_id: Option<u64>,
    installed: Vec<u64>,
    joined: Vec<JoinedProducerProof>,
    joined_tokens: Vec<ProducerId>,
    no_allocation: bool,
}
pub struct OriginalNativeSave {
    inner: Arc<SaveOriginal>,
    core: Arc<Core>,
}
pub struct RegisteredNativeSave {
    original: OriginalNativeSave,
    call: Arc<NativeSaveCall>,
    observed: Option<Arc<SaveOriginal>>,
    original_scope: bool,
}
pub struct NativeSaveRegistrationFailure {
    status: StoreStatus,
    call: Arc<NativeSaveCall>,
    original: Option<OriginalNativeSave>,
    producers: Vec<ProducerId>,
}
impl NativeSaveRegistrationFailure {
    pub fn status(&self) -> StoreStatus {
        self.status
    }
    pub fn call(&self) -> &NativeSaveCall {
        &self.call
    }
    pub fn original(&self) -> Option<&OriginalNativeSave> {
        self.original.as_ref()
    }
    pub fn producers(&self) -> &[ProducerId] {
        &self.producers
    }
}
impl RegisteredNativeSave {
    pub fn original(&self) -> &OriginalNativeSave {
        &self.original
    }
}
impl OriginalNativeSave {
    pub fn observation_origin(&self) -> &OriginalRecoveryOrigin {
        &self.inner.recovery
    }
}
pub struct NativeSaveRecovery {
    original: Arc<SaveOriginal>,
    _core: Arc<Core>,
}
impl NativeSaveRecovery {
    pub fn origin(&self) -> &OriginalRecoveryOrigin {
        &self.original.recovery
    }
}
pub struct NativeAllocatedOperation {
    core: Arc<Core>,
    call: Arc<NativeSaveCall>,
    original: Arc<SaveOriginal>,
    allocation: EditorSaveAllocationV0,
}
impl NativeAllocatedOperation {
    pub fn observation(&self) -> EditorSaveAllocationV0 {
        self.allocation.clone()
    }
}
pub struct NativeDraftSource {
    _owner: Arc<SourceOwner>,
    core: Arc<Core>,
    call: Arc<NativeSaveCall>,
    original: Arc<SaveOriginal>,
    data: Arc<Data>,
    bound: Arc<BoundNativeSource>,
    descriptor: ObjectDescriptor,
}
pub struct NativeDraftWriteLease {
    core: Arc<Core>,
    _call: Arc<NativeSaveCall>,
    original: Arc<SaveOriginal>,
    data: Arc<Data>,
    bound: Arc<BoundNativeSource>,
}
pub(super) struct NativeSaveSelectedRead {
    core: Arc<Core>,
    original: Arc<SaveOriginal>,
    call: Arc<NativeSaveCall>,
    data: Arc<Data>,
    descriptor: ObjectDescriptor,
}
pub struct NativeReceiptRead {
    capture_reply: Option<Envelope>,
    recovery: bool,
    core: Arc<Core>,
    original: Arc<SaveOriginal>,
    call: Arc<NativeSaveCall>,
    data: Arc<Data>,
    descriptor: ObjectDescriptor,
}
pub struct NativeSaveReply {
    wire: Envelope,
    original: OriginalNativeSave,
    allocation: Option<NativeAllocatedOperation>,
    selected: Option<NativeSelectedRead>,
    receipt: Option<NativeReceiptRead>,
    editor: Option<NativeEditor>,
    producers: Vec<ProducerId>,
}
pub struct NativeSaveReplyParts {
    pub wire: Envelope,
    pub original: OriginalNativeSave,
    pub allocation: Option<NativeAllocatedOperation>,
    pub selected: Option<NativeSelectedRead>,
    pub receipt: Option<NativeReceiptRead>,
    pub editor: Option<NativeEditor>,
    pub producers: Vec<ProducerId>,
}
impl NativeSaveReply {
    pub fn wire(&self) -> &Envelope {
        &self.wire
    }
    pub fn producers(&self) -> &[ProducerId] {
        &self.producers
    }
    pub fn selected(&self) -> Option<&NativeSelectedRead> {
        self.selected.as_ref()
    }
    pub fn receipt(&self) -> Option<&NativeReceiptRead> {
        self.receipt.as_ref()
    }
    pub fn allocation(&self) -> Option<&NativeAllocatedOperation> {
        self.allocation.as_ref()
    }
    pub fn original(&self) -> &OriginalNativeSave {
        &self.original
    }
    pub fn take_editor(&mut self) -> Result<NativeEditor, StoreStatus> {
        self.editor.take().ok_or(StoreStatus::NotReady)
    }
    pub fn into_parts(self) -> NativeSaveReplyParts {
        NativeSaveReplyParts {
            wire: self.wire,
            original: self.original,
            allocation: self.allocation,
            selected: self.selected,
            receipt: self.receipt,
            editor: self.editor,
            producers: self.producers,
        }
    }
}
pub enum NativeOriginalObservation {
    AllocationPending,
    NoAllocation,
    AllocationRetained {
        allocation: EditorSaveAllocationV0,
        commit_dispatched: bool,
    },
    CommitPending {
        binding: EditorSaveBindingV0,
    },
    Receipt {
        binding: EditorSaveBindingV0,
        receipt: NativeReceiptRead,
    },
}
pub struct NativeOriginalReply {
    wire: Envelope,
    original_wire: Option<Envelope>,
    allocation: Option<NativeAllocatedOperation>,
    observation: NativeOriginalObservation,
    producers: Vec<ProducerId>,
}
impl NativeOriginalReply {
    pub fn wire(&self) -> &Envelope {
        &self.wire
    }
    pub fn original_wire(&self) -> Option<&Envelope> {
        self.original_wire.as_ref()
    }
    pub fn allocation(&self) -> Option<&NativeAllocatedOperation> {
        self.allocation.as_ref()
    }
    pub fn take_allocation(&mut self) -> Option<NativeAllocatedOperation> {
        self.allocation.take()
    }
    pub fn observation(&self) -> &NativeOriginalObservation {
        &self.observation
    }
    pub fn producers(&self) -> &[ProducerId] {
        &self.producers
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalTicketKind {
    Allocate,
    Commit,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalTicketCompletion {
    Pending,
    JoinedReturned,
    JoinedPanicked,
}
#[derive(Clone)]
pub struct OriginalTicketObservation {
    pub original_request: [u8; 88],
    pub origin_id: u64,
    pub intent_id: u64,
    pub kind: OriginalTicketKind,
    pub actor: EditorActorV0,
    pub object_id: u64,
    pub original_epoch: u64,
    pub allocation: Option<EditorSaveAllocationV0>,
    pub binding: Option<EditorSaveBindingV0>,
    pub closed: bool,
    pub commit_dispatched: bool,
    pub completion: OriginalTicketCompletion,
}
pub struct NativeSaveFenceSnapshot {
    pub schema_version: u32,
    pub base: FenceSnapshot,
    pub original_tickets: Vec<OriginalTicketObservation>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeSaveArtifactKind {
    SelectedTextCopy,
    DraftSourceFreeze,
    ReceiptCopy,
}
#[derive(Clone)]
pub struct NativeSaveArtifactCapture {
    pub capture_id: u64,
    pub kind: NativeSaveArtifactKind,
    pub actor: EditorActorV0,
    pub object_id: u64,
    pub object_generation: u64,
    pub original_backend_epoch: u64,
    pub original_request: [u8; 88],
    pub original_reply: Option<[u8; 88]>,
    pub capture_request: [u8; 88],
    pub capture_reply: Option<[u8; 88]>,
    pub operation_id: Option<u64>,
    pub intent_id: Option<u64>,
    pub descriptor: ObjectDescriptor,
    pub bytes: Vec<u8>,
}
struct CaptureLedger {
    next: u64,
    reserved: usize,
    rows: Vec<NativeSaveArtifactCapture>,
}
struct CaptureReservation {
    attachment: Arc<SaveCore>,
    active: bool,
}
impl CaptureReservation {
    fn reserve(attachment: &Arc<SaveCore>) -> Result<Self, StoreStatus> {
        let mut ledger = checked_lock(&attachment.captures)?;
        if ledger.rows.len() + ledger.reserved >= 256 {
            return Err(StoreStatus::Exhausted);
        }
        ledger.reserved += 1;
        Ok(Self {
            attachment: attachment.clone(),
            active: true,
        })
    }
    fn publish(mut self, mut capture: NativeSaveArtifactCapture) -> Result<(), StoreStatus> {
        let mut ledger = checked_lock(&self.attachment.captures)?;
        capture.capture_id = ledger.next;
        ledger.next = ledger.next.checked_add(1).ok_or(StoreStatus::Exhausted)?;
        ledger.rows.push(capture);
        ledger.reserved -= 1;
        self.active = false;
        Ok(())
    }
}
impl Drop for CaptureReservation {
    fn drop(&mut self) {
        if self.active {
            lock(&self.attachment.captures).reserved -= 1;
        }
    }
}
impl FixtureController {
    pub fn native_save_artifacts(&self) -> Result<Vec<NativeSaveArtifactCapture>, FixtureError> {
        fixture((|| {
            let attachment = self.core.save_attachment()?;
            let rows = checked_lock(&attachment.captures)?;
            Ok(rows.rows.clone())
        })())
    }
}
impl Core {
    // Called only while the trusted controller holds every actual Object state.
    // An absent Save attachment preserves the legacy clock path; a failed
    // attachment/Gate admission must never fall back to an unsynchronized clock.
    pub(super) fn observe_native_save_time(&self, observed_ms: u64) -> Result<u64, StoreStatus> {
        let attachment = checked_lock(&self.native_save)?.clone();
        let Some(attachment) = attachment else {
            return Ok(observed_ms);
        };
        if self.closed.load(Ordering::Acquire) {
            return Err(StoreStatus::Stale);
        }
        let mut gate = native(attachment.issuer.lock())?;
        native(gate.observe_core_time(observed_ms))
    }
    fn save_attachment(&self) -> Result<Arc<SaveCore>, StoreStatus> {
        checked_lock(&self.native_save)?
            .clone()
            .ok_or(StoreStatus::Denied)
    }
}
fn save_reply(request: &Envelope, status: StoreStatus) -> Envelope {
    let mut reply = empty_reply(request, status);
    reply.handle = Handle::INVALID;
    reply
}

fn check_call(
    original: &SaveOriginal,
    call: &NativeSaveCall,
    state: &ObjectState,
) -> Result<(), StoreStatus> {
    let core = original.core.upgrade().ok_or(StoreStatus::Stale)?;
    auth(&core, &original.grant, state)?;
    let attachment = original.attachment.upgrade().ok_or(StoreStatus::Stale)?;
    let guard = native(call.admit(&attachment.witness))?;
    let view = guard.view();
    if view.actor != original.grant.actor
        || view.object_id != original.grant.object
        || view.object_generation != original.tag.object_generation
        || view.store_epoch != original.grant.epoch
        || state.journal.owner_id != view.actor.owner_id
        || state.journal.selected_generation != view.object_generation
        || state.journal.service_epoch != view.store_epoch
    {
        return Err(StoreStatus::Denied);
    }
    if !original.grant.alive.load(Ordering::Acquire) {
        return Err(StoreStatus::Stale);
    }
    Ok(())
}
impl StoreHost {
    pub fn register_native_save(
        &self,
        endpoint: &EditorEndpoint,
        call: NativeSaveCall,
    ) -> Result<RegisteredNativeSave, NativeSaveRegistrationFailure> {
        let call = Arc::new(call);
        let mut producers = vec![];
        let mut exhausted_entry_context = None;
        let result = (|| {
            let attachment = self.core.save_attachment()?;
            // Concrete call admission establishes actual native identity before
            // any artifact IDs or Store state are inspected.
            let view = native(call.admit(&attachment.witness))?.view();
            if !Arc::ptr_eq(&self.core, &endpoint.peer.core)
                || endpoint.handle != endpoint.peer.grant.handle
            {
                return Err(StoreStatus::Denied);
            }
            let grant = &endpoint.peer.grant;
            let registry = checked_lock(&self.core.registry)?;
            let tag = checked_lock(&attachment.tags)?
                .get(&endpoint.handle.pack())
                .cloned()
                .ok_or(StoreStatus::Denied)?;
            if !registry
                .grants
                .get(&endpoint.handle.pack())
                .is_some_and(|g| Arc::ptr_eq(g, grant))
                || !Arc::ptr_eq(&tag.grant, grant)
            {
                return Err(StoreStatus::Denied);
            }
            let object = self.core.object(grant.object)?;
            let state = checked_lock(&object.state)?;
            let mut gate = native(attachment.issuer.lock())?;
            let current = native(gate.admit_call(&call))?;
            if current.actor != grant.actor
                || current.object_id != object.id
                || current.object_generation != tag.object_generation
                || current.store_epoch != grant.epoch
                || current.request.handle != grant.handle
                || state.journal.service_epoch != grant.epoch
                || state.journal.owner_id != current.actor.owner_id
                || view.origin_id != current.origin_id
            {
                return Err(StoreStatus::Denied);
            }
            codec::validate(call.request())?;
            if !matches!(call.request().msg_type, 1 | 3 | 5 | 7) {
                return Err(StoreStatus::Unsupported);
            }
            if u64_at(&call.request().payload, 8) != grant.actor.session_id
                || u64_at(&call.request().payload, 16) != grant.actor.session_generation
            {
                return Err(StoreStatus::Denied);
            }
            if call.request().msg_type == 1
                && (u64_at(&call.request().payload, 24) != grant.actor.instance_id
                    || u64_at(&call.request().payload, 32) != grant.actor.instance_generation)
            {
                return Err(StoreStatus::Denied);
            }
            if matches!(call.request().msg_type, 3 | 5)
                && checked_lock(&attachment.originals)?.len() >= 96
            {
                exhausted_entry_context = Some((attachment.clone(), object.id));
                return Err(StoreStatus::Exhausted);
            }
            let claim = match native(gate.claim_execution(&call)) {
                Ok(claim) => claim,
                Err(StoreStatus::Exhausted) => {
                    exhausted_entry_context = Some((attachment.clone(), object.id));
                    return Err(StoreStatus::Exhausted);
                }
                Err(status) => return Err(status),
            };
            let recovery = native(gate.original_recovery(&claim))?;
            let original = Arc::new(SaveOriginal {
                core: Arc::downgrade(&self.core),
                attachment: Arc::downgrade(&attachment),
                grant: grant.clone(),
                tag,
                claim,
                recovery,
                request: *call.request(),
                view: current,
                claimed: AtomicBool::new(false),
                state: Mutex::new(OriginalState {
                    wire: None,
                    allocation: None,
                    binding: None,
                    bound: None,
                    completion: OriginalTicketCompletion::Pending,
                    closed: false,
                    commit_dispatched: false,
                    allocation_delivered: false,
                    dispatcher_id: None,
                    installed: vec![],
                    joined: vec![],
                    joined_tokens: vec![],
                    no_allocation: false,
                }),
            });
            if matches!(call.request().msg_type, 3 | 5) {
                checked_lock(&attachment.originals)?.push(original.clone());
            }
            drop(gate);
            drop(state);
            drop(registry);
            producers = retain_entry_producers(&self.core, &attachment, &call, &original)?;
            Ok(original)
        })();
        match result {
            Ok(inner) => {
                let observed = if call.request().msg_type == 7 {
                    let attachment = self.core.save_attachment().map_err(|status| {
                        NativeSaveRegistrationFailure {
                            status,
                            call: call.clone(),
                            original: None,
                            producers: vec![],
                        }
                    })?;
                    let matches = lock(&attachment.originals)
                        .iter()
                        .filter(|o| {
                            o.request.msg_type == 5
                                && o.grant.actor == inner.grant.actor
                                && o.grant.object == inner.grant.object
                                && op_of(&o.request) == op_of(call.request())
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    if matches.len() != 1 {
                        return Err(NativeSaveRegistrationFailure {
                            status: StoreStatus::Denied,
                            call,
                            original: Some(OriginalNativeSave {
                                inner,
                                core: self.core.clone(),
                            }),
                            producers,
                        });
                    }
                    Some(matches[0].clone())
                } else {
                    None
                };
                Ok(RegisteredNativeSave {
                    original: OriginalNativeSave {
                        inner,
                        core: self.core.clone(),
                    },
                    call,
                    observed,
                    original_scope: false,
                })
            }
            Err(mut status) => {
                // The context exists only after complete own-Core/current-call
                // authentication and the exact original-capacity/claim failure.
                // All enforcing guards are gone before retrieving real tokens.
                if let Some((attachment, object)) = exhausted_entry_context {
                    if let Err(retention_status) = retain_failed_entry_producers(
                        &self.core,
                        &attachment,
                        &call,
                        object,
                        &mut producers,
                    ) {
                        status = retention_status;
                    }
                }
                Err(NativeSaveRegistrationFailure {
                    status,
                    call,
                    original: None,
                    producers,
                })
            }
        }
    }
    pub fn source_for_intent(
        &self,
        endpoint: &EditorEndpoint,
        allocation: &NativeAllocatedOperation,
        intent: &NativeSaveIntent,
    ) -> Result<NativeDraftSource, StoreStatus> {
        if !Arc::ptr_eq(&self.core, &allocation.core)
            || !Arc::ptr_eq(&self.core, &endpoint.peer.core)
            || !Arc::ptr_eq(&endpoint.peer.grant, &allocation.original.grant)
        {
            return Err(StoreStatus::Denied);
        }
        let attachment = self.core.save_attachment()?;
        let object = self.core.object(endpoint.peer.grant.object)?;
        let mut registry = checked_lock(&self.core.registry)?;
        let mut state = checked_lock(&object.state)?;
        auth(&self.core, &allocation.original.grant, &state)?;
        let mut gate = native(attachment.issuer.lock())?;
        if checked_lock(&allocation.original.state)?.closed {
            return Err(StoreStatus::Stale);
        }
        if allocation.original.view.intent_id != Some(intent.view().intent_id) {
            return Err(StoreStatus::Denied);
        }
        let snapshot = match native(gate.snapshot_for_intent(intent)) {
            Ok(snapshot) => snapshot,
            Err(StoreStatus::Timeout) => {
                // This exact Timeout follows private Authority/intent-row and
                // current-instance authentication inside snapshot_for_intent.
                // Never close an allocation on a caller's numeric intent alone.
                let no_source = !checked_lock(&attachment.sources)?
                    .values()
                    .filter_map(std::sync::Weak::upgrade)
                    .any(|source| Arc::ptr_eq(&source.allocation, &allocation.original));
                let no_commit = !checked_lock(&attachment.originals)?.iter().any(|original| {
                    original.request.msg_type == 5
                        && original.view.actor == allocation.original.view.actor
                        && original.view.intent_id == allocation.original.view.intent_id
                });
                if no_source && no_commit {
                    close_late_allocation(&mut state, &allocation.original, &mut gate)?;
                    let row = &state.journal.operations.as_slice()
                        [record_index(&state, allocation.allocation.operation_id)?];
                    if checked_lock(&allocation.original.state)?.closed
                        && row.allocation == allocation.allocation
                        && row.state == EditorOperationStateV0::Noncommit
                        && row.closure == EditorClosureV0::Deadline
                        && row.binding.is_none()
                        && row.permit.is_none()
                        && row.receipt.is_none()
                        && row.successor.is_none()
                    {
                        return Err(StoreStatus::Stale);
                    }
                }
                return Err(StoreStatus::Timeout);
            }
            Err(error) => return Err(error),
        };
        let view = snapshot.view();
        if view.actor != allocation.allocation.actor
            || view.object_id != object.id
            || view.object_generation != state.journal.selected_generation
            || view.original_store_epoch != state.journal.service_epoch
            || view.expected_revision != allocation.allocation.expected_revision
        {
            return Err(StoreStatus::Denied);
        }
        let outcome = (|| {
            if state.mutation_fenced {
                return Err(StoreStatus::Exhausted);
            }
            let row = &state.journal.operations.as_slice()
                [record_index(&state, allocation.allocation.operation_id)?];
            if row.allocation != allocation.allocation
                || row.state != EditorOperationStateV0::Allocated
                || row.closure != EditorClosureV0::None
            {
                return Err(StoreStatus::NotReady);
            }
            registry.data.retain(|_, data| Arc::strong_count(data) > 1);
            let body = snapshot.body();
            if body.len() > 4096 || !body.is_ascii() {
                return Err(StoreStatus::Invalid);
            }
            if registry.data.len() >= 32 {
                return Err(StoreStatus::Exhausted);
            }
            // Predict without advancing the registry; actual same-Gate source
            // authorization/binding must succeed before allocation consumes it.
            let handle = self.core.next_data_handle(&registry)?;
            let binding = schema(EditorSaveBindingV0::try_new(
                allocation.allocation.clone(),
                schema(EditorSourceBindingV0::try_new(
                    handle.pack(),
                    handle.generation,
                    body.len() as u32,
                    storage::hash(body),
                ))?,
            ))?;
            let bound = Arc::new(native(gate.bind_source(
                intent,
                &binding,
                state.journal.service_epoch,
            ))?);
            if self.core.new_data_handle(&mut registry)? != handle {
                return Err(StoreStatus::Internal);
            }
            let header = schema(EditorTextHeaderV0::try_new(
                object.id,
                binding.allocation.expected_revision,
                body,
            ))?;
            let mut bytes = schema(header.encode_le())?.to_vec();
            bytes.extend_from_slice(body);
            let descriptor = ObjectDescriptor {
                handle,
                kind: SharedObjectKind::DraftSource,
                object_generation: handle.generation,
                byte_len: header.total_len,
            };
            let data = Arc::new(Data {
                handle,
                actor: allocation.allocation.actor.clone(),
                object: object.id,
                operation: Some(binding.allocation.operation_id),
                revision: binding.allocation.expected_revision,
                kind: SharedObjectKind::DraftSource,
                alive: AtomicBool::new(true),
                state: Mutex::new(DataState {
                    bytes,
                    writable: true,
                    frozen: false,
                }),
            });
            let source = Arc::new(SourceOwner {
                allocation: allocation.original.clone(),
                bound: bound.clone(),
                data: data.clone(),
            });
            checked_lock(&attachment.sources)?.insert(handle.pack(), Arc::downgrade(&source));
            registry.data.insert(handle.pack(), data.clone());
            Ok(NativeDraftSource {
                _owner: source,
                core: self.core.clone(),
                original: allocation.original.clone(),
                call: allocation.call.clone(),
                data,
                bound,
                descriptor,
            })
        })();
        if outcome.is_err() {
            let index = record_index(&state, allocation.allocation.operation_id)?;
            let row = &state.journal.operations.as_slice()[index];
            let no_commit = !checked_lock(&attachment.originals)?.iter().any(|original| {
                original.request.msg_type == 5
                    && original.view.actor == view.actor
                    && original.view.intent_id == allocation.original.view.intent_id
            });
            let no_source = !checked_lock(&attachment.sources)?
                .values()
                .filter_map(std::sync::Weak::upgrade)
                .any(|source| Arc::ptr_eq(&source.allocation, &allocation.original));
            let mut actual = checked_lock(&allocation.original.state)?;
            if row.allocation == allocation.allocation
                && row.state == EditorOperationStateV0::Allocated
                && row.closure == EditorClosureV0::None
                && row.binding.is_none()
                && row.permit.is_none()
                && row.receipt.is_none()
                && row.successor.is_none()
                && !actual.commit_dispatched
                && no_commit
                && no_source
                && state.io_worker.is_none()
                && !state.io_reserving
                && state
                    .runtime
                    .get(&row.allocation.operation_id)
                    .is_some_and(|runtime| {
                        !runtime.dispatched
                            && runtime.permitted.is_none()
                            && runtime.source.is_none()
                    })
            {
                native(gate.publish_original_delivery(
                    &allocation.original.claim,
                    desktop_service::editor_dev::NativeSaveDeliveryOutcome::AllocatedSourceRejected,
                ))?;
                actual.closed = true;
            }
        }
        outcome
    }
}
struct SourceOwner {
    allocation: Arc<SaveOriginal>,
    bound: Arc<BoundNativeSource>,
    data: Arc<Data>,
}
impl NativeDraftSource {
    pub fn descriptor(&self) -> ObjectDescriptor {
        self.descriptor.clone()
    }
    pub fn bound(&self) -> &BoundNativeSource {
        &self.bound
    }
    pub fn write_lease(&self) -> Result<NativeDraftWriteLease, StoreStatus> {
        let object = self.core.object(self.data.object)?;
        let state = checked_lock(&object.state)?;
        auth(&self.core, &self.original.grant, &state)?;
        let attachment = self.core.save_attachment()?;
        let gate = native(attachment.issuer.lock())?;
        native(gate.admit_bound_source(&self.bound))?;
        let bytes = checked_lock(&self.data.state)?;
        if !self.data.alive.load(Ordering::Acquire) || !bytes.writable || bytes.frozen {
            return Err(StoreStatus::Stale);
        }
        Ok(NativeDraftWriteLease {
            core: self.core.clone(),
            original: self.original.clone(),
            _call: self.call.clone(),
            data: self.data.clone(),
            bound: self.bound.clone(),
        })
    }
}
impl NativeDraftWriteLease {
    pub fn copy_from(&self, offset: u32, input: &[u8]) -> Result<(), StoreStatus> {
        let object = self.core.object(self.data.object)?;
        let state = checked_lock(&object.state)?;
        auth(&self.core, &self.original.grant, &state)?;
        let attachment = self.core.save_attachment()?;
        let gate = native(attachment.issuer.lock())?;
        native(gate.admit_bound_source(&self.bound))?;
        let mut bytes = checked_lock(&self.data.state)?;
        if !self.data.alive.load(Ordering::Acquire) || !bytes.writable || bytes.frozen {
            return Err(StoreStatus::Stale);
        }
        let start = offset as usize;
        let end = start.checked_add(input.len()).ok_or(StoreStatus::Invalid)?;
        if end > bytes.bytes.len() {
            return Err(StoreStatus::Invalid);
        }
        bytes.bytes[start..end].copy_from_slice(input);
        Ok(())
    }
}

pub(super) struct NativeRequest {
    owner: Arc<SaveCore>,
    observed: Option<Arc<SaveOriginal>>,
    original_scope: bool,
    original: Arc<SaveOriginal>,
    call: std::sync::Weak<NativeSaveCall>,
    source: Mutex<Option<Arc<SourceOwner>>>,
    admission: Mutex<Option<NativeCommitAdmission>>,
}
impl NativeRequest {
    pub(super) fn authorize(&self, state: &ObjectState) -> Result<(), StoreStatus> {
        let call = self.call.upgrade().ok_or(StoreStatus::Stale)?;
        if let Some(target) = &self.observed {
            if !self.original_scope {
                check_call(&self.original, &call, state)?;
            }
            let gate = self.gate()?;
            let view = if self.original_scope {
                native(gate.admit_original_call(&call))?
            } else {
                native(gate.admit_call(&call))?
            };
            if view.actor != target.grant.actor
                || view.object_id != target.grant.object
                || state.journal.selected_generation != target.tag.object_generation
            {
                return Err(StoreStatus::Denied);
            }
            if self
                .owner
                .originals
                .lock()
                .map_err(|_| StoreStatus::Internal)?
                .iter()
                .all(|r| !Arc::ptr_eq(r, target))
            {
                return Err(StoreStatus::Denied);
            }
            return Ok(());
        }
        check_call(&self.original, &call, state)
    }
    pub(super) fn gate(
        &self,
    ) -> Result<desktop_service::editor_dev::SaveGateGuard<'_>, StoreStatus> {
        // Original external/request owners retain this actual attachment while
        // executing. The issuer is never reconstructed from a diagnostic ID.
        native(self.original_attachment().issuer.lock())
    }
    fn original_attachment(&self) -> &SaveCore {
        &self.owner
    }
    pub(super) fn admit_held(
        &self,
        gate: &mut desktop_service::editor_dev::SaveGateGuard<'_>,
    ) -> Result<(), StoreStatus> {
        let call = self.call.upgrade().ok_or(StoreStatus::Stale)?;
        if self.original_scope {
            native(gate.admit_original_call(&call))?;
        } else {
            native(gate.admit_call(&call))?;
        }
        Ok(())
    }
    pub(super) fn mark_dispatch(&self) -> Result<(), StoreStatus> {
        lock(&self.original.state).commit_dispatched = true;
        if let Some(source) = lock(&self.source).as_ref() {
            lock(&source.allocation.state).commit_dispatched = true;
        }
        Ok(())
    }
    pub(super) fn seal(
        &self,
        gate: &mut desktop_service::editor_dev::SaveGateGuard<'_>,
        binding: &EditorSaveBindingV0,
        bytes: &[u8],
        descriptor: ObjectDescriptor,
    ) -> Result<(), StoreStatus> {
        self.admit_held(gate)?;
        let held_source = checked_lock(&self.source)?;
        let source = held_source.as_ref().ok_or(StoreStatus::Denied)?;
        if source.bound.binding() != binding
            || source.bound.snapshot().body() != &bytes[64..]
            || source.data.handle != descriptor.handle
        {
            return Err(StoreStatus::Denied);
        }
        let reserve = CaptureReservation::reserve(&self.owner)?;
        let capture = NativeSaveArtifactCapture {
            capture_id: 0,
            kind: NativeSaveArtifactKind::DraftSourceFreeze,
            actor: self.original.grant.actor.clone(),
            object_id: self.original.grant.object,
            object_generation: self.original.tag.object_generation,
            original_backend_epoch: self.original.grant.epoch,
            original_request: encode_envelope_wire(&self.original.request)?,
            original_reply: None,
            capture_request: encode_envelope_wire(&self.original.request)?,
            capture_reply: None,
            operation_id: Some(binding.allocation.operation_id),
            intent_id: self.original.view.intent_id,
            descriptor,
            bytes: bytes.to_vec(),
        };
        reserve.publish(capture)?;
        let mut state = checked_lock(&self.original.state)?;
        state.binding = Some(binding.clone());
        state.bound = Some(source.bound.clone());
        state.commit_dispatched = true;
        Ok(())
    }
    pub(super) fn issue_permit(
        &self,
        gate: &mut desktop_service::editor_dev::SaveGateGuard<'_>,
        binding: &EditorSaveBindingV0,
    ) -> Result<(), StoreStatus> {
        self.admit_held(gate)?;
        let held_source = checked_lock(&self.source)?;
        let source = held_source.as_ref().ok_or(StoreStatus::Denied)?;
        if source.bound.binding() != binding {
            return Err(StoreStatus::Denied);
        }
        let admission = native(gate.issue_commit_permit(&self.original.claim, &source.bound))?;
        *checked_lock(&self.admission)? = Some(admission);
        Ok(())
    }
    pub(super) fn publish_permitted(
        &self,
        gate: &mut desktop_service::editor_dev::SaveGateGuard<'_>,
    ) -> Result<(), StoreStatus> {
        let admission = checked_lock(&self.admission)?;
        native(gate.publish_permitted(admission.as_ref().ok_or(StoreStatus::Internal)?))
    }
    pub(super) fn reserve_io(
        &self,
        gate: &mut desktop_service::editor_dev::SaveGateGuard<'_>,
    ) -> Result<desktop_service::editor_dev::ProducerReservation, StoreStatus> {
        let call = self.call.upgrade().ok_or(StoreStatus::Stale)?;
        native(gate.reserve_io(&call))
    }
}

fn retain_producer(attachment: &Arc<SaveCore>, id: ProducerId) -> Result<ProducerId, StoreStatus> {
    let alias = native(attachment.witness.duplicate_producer(&id));
    let mut retained = checked_lock(&attachment.producers)?;
    retained.retain(|old| {
        !matches!(
            attachment.witness.duplicate_producer(old),
            Err(DesktopStatus::NotReady)
        )
    });
    if retained.len() >= 64 {
        return Err(StoreStatus::Exhausted);
    }
    match alias {
        Ok(alias) => {
            retained.push(alias);
            Ok(id)
        }
        Err(status) => {
            retained.push(id);
            Err(status)
        }
    }
}
impl NativeRequest {
    pub(super) fn run_io<T: Send + 'static>(
        self: &Arc<Self>,
        core: &Arc<Core>,
        object: &Arc<Object>,
        request: &Arc<Request>,
        operation: u64,
        writer: u64,
        job: impl FnOnce(Context) -> Result<T, StoreStatus> + Send + 'static,
    ) -> Result<T, StoreStatus> {
        let reservation = {
            let mut state = checked_lock(&object.state)?;
            if state.io_worker.is_some() || state.io_reserving {
                return Err(StoreStatus::NotReady);
            }
            let mut gate = self.gate()?;
            self.admit_held(&mut gate)?;
            let reservation = self.reserve_io(&mut gate)?;
            state.io_reserving = true;
            reservation
        };
        let (start_tx, start_rx) = mpsc::channel::<u64>();
        let (tx, rx) = mpsc::channel();
        let worker_core = core.clone();
        let worker_object = object.clone();
        let worker_request = request.clone();
        let handle = thread::Builder::new()
            .name("store-native-save-io".into())
            .spawn(move || {
                let Ok(producer) = start_rx.recv() else {
                    return;
                };
                let exit = WorkerExit {
                    core: worker_core.clone(),
                    object: Some(worker_object.clone()),
                    producer,
                };
                let context = Context {
                    core: worker_core,
                    object: worker_object,
                    request: Some(worker_request),
                    operation,
                    writer,
                    selection: false,
                };
                let result = job(context);
                drop(exit);
                let _ = tx.send(result);
            });
        let handle = match handle {
            Ok(handle) => handle,
            Err(_) => {
                checked_lock(&object.state)?.io_reserving = false;
                let _ = reservation.cancel_unspawned();
                return Err(StoreStatus::Internal);
            }
        };
        let thread_id = format!("{:?}", handle.thread().id());
        let id = reservation.install(handle);
        let number = id.producer_id();
        {
            let mut original = checked_lock(&self.original.state)?;
            if !original.installed.contains(&number) {
                original.installed.push(number);
            }
        }
        {
            let mut state = checked_lock(&object.state)?;
            state.io_worker = Some(number);
            state.io_reserving = false;
            checked_lock(&core.owned)?.handles.insert(
                number,
                Producer {
                    kind: ProducerKind::ObjectIoWorker,
                    object: object.id,
                    request: u64_at(&request.wire.payload, 0),
                    operation,
                    handle: None,
                    joining: false,
                    terminal_join: None,
                    thread_id,
                    barrier: None,
                },
            );
        }
        let retain = retain_producer(&self.owner, id);
        // Installation is infallible; even retention diagnostics failure must
        // release the actual start latch so the owned worker can be joined.
        let _ = start_tx.send(number);
        let retained = retain?;
        let result = rx.recv().map_err(|_| StoreStatus::Internal)?;
        let until = Instant::now() + Duration::from_millis(50);
        while Instant::now() < until {
            match (StoreHost { core: core.clone() }).join_native_save_finished(&retained) {
                Ok(_) => break,
                Err(StoreStatus::NotReady) => thread::yield_now(),
                Err(_) => break,
            }
        }
        result
    }
}

enum ExecutionReady {
    Reply(NativeSaveReply),
    InitializeEditorExhausted(NativeSaveReply),
}
struct ExecutionState {
    request: Mutex<Option<Arc<Request>>>,
    ready: Mutex<Option<ExecutionReady>>,
    taken: AtomicBool,
    responded: AtomicBool,
    finished: Mutex<bool>,
    ready_changed: Condvar,
    finished_changed: Condvar,
    producers: Mutex<Option<Vec<ProducerId>>>,
    observed: Option<Arc<SaveOriginal>>,
    original_scope: bool,
}
pub struct NativeSaveExecution {
    state: Arc<ExecutionState>,
    _original: Arc<SaveOriginal>,
    _call: Arc<NativeSaveCall>,
}
impl NativeSaveExecution {
    pub fn poll_once(&self) -> Result<Option<NativeSaveReply>, StoreStatus> {
        if self.state.taken.load(Ordering::Acquire) {
            return Err(StoreStatus::NotReady);
        }
        let mut ready = checked_lock(&self.state.ready)?;
        if self.state.taken.load(Ordering::Acquire) {
            return Err(StoreStatus::NotReady);
        }
        match ready.take() {
            Some(ExecutionReady::Reply(reply)) => {
                self.state.taken.store(true, Ordering::Release);
                Ok(Some(reply))
            }
            Some(ExecutionReady::InitializeEditorExhausted(reply)) => {
                // The error is delivered once, while its real canonical reply,
                // original/Core and producer references remain actually owned.
                *ready = Some(ExecutionReady::InitializeEditorExhausted(reply));
                self.state.taken.store(true, Ordering::Release);
                Err(StoreStatus::Exhausted)
            }
            None => Ok(None),
        }
    }
}
fn store_current(original: &SaveOriginal, state: &ObjectState) -> StoreCurrentSelection {
    StoreCurrentSelection {
        owner: desktop_service::editor_dev::PreviewOwner {
            owner_id: original.grant.actor.owner_id,
            session_id: original.grant.actor.session_id,
            session_generation: original.grant.actor.session_generation,
        },
        object_id: original.grant.object,
        object_generation: state.journal.selected_generation,
        store_epoch: state.journal.service_epoch,
        revision: state.journal.current.revision,
        content_hash: state.journal.current.content_hash,
    }
}
fn data_descriptor(
    core: &Core,
    handle: u64,
    kind: SharedObjectKind,
) -> Result<(Arc<Data>, ObjectDescriptor), StoreStatus> {
    let registry = checked_lock(&core.registry)?;
    let data = registry
        .data
        .get(&handle)
        .cloned()
        .ok_or(StoreStatus::Denied)?;
    if data.handle.pack() != handle || data.kind != kind {
        return Err(StoreStatus::Denied);
    }
    let len = checked_lock(&data.state)?.bytes.len() as u32;
    Ok((
        data.clone(),
        ObjectDescriptor {
            handle: data.handle,
            kind,
            object_generation: data.handle.generation,
            byte_len: len,
        },
    ))
}
enum ReplyBuildFailure {
    Ordinary(StoreStatus),
    InitializeEditorExhausted,
}
impl From<StoreStatus> for ReplyBuildFailure {
    fn from(status: StoreStatus) -> Self {
        Self::Ordinary(status)
    }
}
fn build_reply(
    core: &Arc<Core>,
    original: &Arc<SaveOriginal>,
    call: &Arc<NativeSaveCall>,
    observed: Option<&Arc<SaveOriginal>>,
    original_scope: bool,
    mut wire: Envelope,
    producers: Vec<ProducerId>,
) -> Result<NativeSaveReply, ReplyBuildFailure> {
    wire.handle = Handle::INVALID;
    codec::validate(&wire)?;
    let mut selected = None;
    let mut receipt = None;
    let mut editor = None;
    let mut allocation = None;
    let status_offset = match wire.msg_type {
        2 => 36,
        4 => 16,
        6 | 8 => 32,
        10 => 24,
        _ => return Err(StoreStatus::Invalid.into()),
    };
    let status = u32::from_le_bytes(
        wire.payload[status_offset..status_offset + 4]
            .try_into()
            .unwrap(),
    );
    if status == StoreStatus::Ok as u32 {
        match wire.msg_type {
            2 => {
                let (data, descriptor) = data_descriptor(
                    core,
                    u64_at(&wire.payload, 16),
                    SharedObjectKind::SelectedText,
                )?;
                let object = core.object(original.grant.object)?;
                let state = checked_lock(&object.state)?;
                check_call(original, call, &state)?;
                let attachment = original.attachment.upgrade().ok_or(StoreStatus::Stale)?;
                let mut gate = native(attachment.issuer.lock())?;
                native(gate.admit_call(call))?;
                let bytes = checked_lock(&data.state)?;
                match native(gate.initialize_editor(call, &bytes.bytes)) {
                    Ok(value) => editor = Some(value),
                    Err(StoreStatus::NotReady) => {}
                    Err(StoreStatus::Exhausted) => {
                        return Err(ReplyBuildFailure::InitializeEditorExhausted);
                    }
                    Err(status) => return Err(status.into()),
                }
                selected = Some(NativeSelectedRead::from_save(NativeSaveSelectedRead {
                    core: core.clone(),
                    original: original.clone(),
                    call: call.clone(),
                    data: data.clone(),
                    descriptor,
                }));
            }
            4 => {
                let object = core.object(original.grant.object)?;
                let state = checked_lock(&object.state)?;
                let record = &state.journal.operations.as_slice()
                    [record_index(&state, u64_at(&wire.payload, 8))?];
                if record.allocation.actor != original.grant.actor {
                    return Err(StoreStatus::Denied.into());
                }
                let actual = record.allocation.clone();
                let mut inner = checked_lock(&original.state)?;
                inner.allocation = Some(actual.clone());
                if !inner.allocation_delivered {
                    inner.allocation_delivered = true;
                    allocation = Some(NativeAllocatedOperation {
                        core: core.clone(),
                        call: call.clone(),
                        original: original.clone(),
                        allocation: actual,
                    });
                }
            }
            6 | 8 => {
                let (data, descriptor) =
                    data_descriptor(core, u64_at(&wire.payload, 24), SharedObjectKind::Receipt)?;
                let mut receipt_owner = observed.unwrap_or(original).clone();
                if wire.msg_type == 6 && observed.is_none() {
                    let object = core.object(original.grant.object)?;
                    let state = checked_lock(&object.state)?;
                    let operation = u64_at(&wire.payload, 8);
                    let row =
                        &state.journal.operations.as_slice()[record_index(&state, operation)?];
                    let submitted = state
                        .runtime
                        .get(&operation)
                        .and_then(|runtime| runtime.original.as_ref())
                        .and_then(|request| request.native_save.as_ref())
                        .map(|native| &native.original)
                        .ok_or(StoreStatus::Denied)?;
                    if !Arc::ptr_eq(submitted, original) {
                        // A terminal duplicate control owns a fresh call, not a
                        // submitted source. Retain the actual runtime original
                        // for receipt validation/publication, never copy a view
                        // into the fresh control or re-enter mutation work.
                        let terminal = row.receipt.as_ref().ok_or(StoreStatus::Denied)?;
                        let binding = row.binding.as_ref().ok_or(StoreStatus::Denied)?;
                        let held = checked_lock(&submitted.state)?;
                        let bound = held.bound.as_ref().ok_or(StoreStatus::Denied)?;
                        if submitted
                            .core
                            .upgrade()
                            .is_none_or(|actual| !Arc::ptr_eq(&actual, core))
                            || submitted.grant.actor != original.grant.actor
                            || submitted.grant.object != object.id
                            || submitted.grant.epoch != state.journal.service_epoch
                            || submitted.tag.object_generation != state.journal.selected_generation
                            || submitted.request.msg_type != 5
                            || submitted.request.handle != original.request.handle
                            || submitted.request.payload[8..52] != original.request.payload[8..52]
                            || held.binding.as_ref() != Some(binding)
                            || bound.binding() != binding
                            || row.submitted_service_epoch != Some(submitted.grant.epoch)
                            || data.actor != submitted.grant.actor
                            || data.object != object.id
                            || data.operation != Some(operation)
                            || descriptor.byte_len != 176
                        {
                            return Err(StoreStatus::Denied.into());
                        }
                        let bytes = checked_lock(&data.state)?;
                        let decoded = schema(EditorSaveReceiptV0::decode_le(&bytes.bytes))?;
                        schema(decoded.validate_binding(binding))?;
                        if &decoded != terminal
                            || decoded.backend_service_epoch != submitted.grant.epoch
                        {
                            return Err(StoreStatus::Denied.into());
                        }
                        drop(bytes);
                        drop(held);
                        receipt_owner = submitted.clone();
                    }
                }
                receipt = Some(NativeReceiptRead {
                    recovery: original_scope,
                    capture_reply: Some(wire),
                    core: core.clone(),
                    original: receipt_owner,
                    call: call.clone(),
                    data,
                    descriptor,
                });
            }
            10 => {}
            _ => {}
        }
    }
    Ok(NativeSaveReply {
        wire,
        original: OriginalNativeSave {
            inner: original.clone(),
            core: core.clone(),
        },
        allocation,
        selected,
        receipt,
        editor,
        producers,
    })
}
fn respond(
    core: &Arc<Core>,
    original: &Arc<SaveOriginal>,
    call: &Arc<NativeSaveCall>,
    execution: &Arc<ExecutionState>,
    wire: Envelope,
) {
    if execution.responded.swap(true, Ordering::AcqRel) {
        return;
    }
    let producers = lock(&execution.producers).take().unwrap_or_default();
    let fallback_producers = core
        .save_attachment()
        .ok()
        .map(|attachment| {
            producers
                .iter()
                .filter_map(|id| attachment.witness.duplicate_producer(id).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let canonical = build_reply(
        core,
        original,
        call,
        execution.observed.as_ref(),
        execution.original_scope,
        wire,
        producers,
    );
    let mut initialization_exhausted = false;
    let mut reply = match canonical {
        Ok(reply) => reply,
        Err(failure) => {
            let status = match failure {
                ReplyBuildFailure::Ordinary(status) => status,
                ReplyBuildFailure::InitializeEditorExhausted => {
                    initialization_exhausted = true;
                    StoreStatus::Exhausted
                }
            };
            NativeSaveReply {
                wire: save_reply(&original.request, status),
                original: OriginalNativeSave {
                    inner: original.clone(),
                    core: core.clone(),
                },
                allocation: None,
                selected: None,
                receipt: None,
                editor: None,
                producers: fallback_producers,
            }
        }
    };
    let suppressed = original.grant.drop_reply.swap(false, Ordering::AcqRel);
    {
        let mut state = lock(&original.state);
        state.wire = Some(reply.wire);
        if reply.wire.msg_type == 6
            && u32::from_le_bytes(reply.wire.payload[32..36].try_into().unwrap())
                != StoreStatus::Unknown as u32
        {
            state.joined.clear();
            state.joined_tokens.clear();
        }
    }
    if core
        .append_exchange(
            &original.grant,
            &original.request,
            &reply.wire,
            suppressed,
            call.entered(),
        )
        .is_err()
    {
        reply.wire = save_reply(&original.request, StoreStatus::Exhausted);
        reply.allocation = None;
        reply.selected = None;
        reply.receipt = None;
        reply.editor = None;
    }
    let unknown_result = matches!(reply.wire.msg_type, 4 | 6)
        && u32::from_le_bytes(
            reply.wire.payload[if reply.wire.msg_type == 4 {
                16..20
            } else {
                32..36
            }]
            .try_into()
            .unwrap(),
        ) == StoreStatus::Unknown as u32;
    if (suppressed || unknown_result) && matches!(original.request.msg_type, 3 | 5) {
        if let Ok(object) = core.object(original.grant.object) {
            let _state = lock(&object.state);
            if let Ok(attachment) = core.save_attachment() {
                if let Ok(mut gate) = attachment.issuer.lock() {
                    if let Err(status) = gate.publish_original_delivery(
                        &original.claim,
                        desktop_service::editor_dev::NativeSaveDeliveryOutcome::ReplyLost,
                    ) {
                        reply.wire = save_reply(&original.request, desktop_status(status));
                        reply.allocation = None;
                        reply.selected = None;
                        reply.receipt = None;
                        reply.editor = None;
                    }
                }
            }
        }
    }
    if suppressed {
        reply.wire = save_reply(&original.request, StoreStatus::Disconnected);
        reply.allocation = None;
        reply.selected = None;
        reply.receipt = None;
        reply.editor = None;
        lock(&original.state).allocation_delivered = false;
    }
    *lock(&execution.ready) = Some(if initialization_exhausted && !suppressed {
        ExecutionReady::InitializeEditorExhausted(reply)
    } else {
        ExecutionReady::Reply(reply)
    });
    execution.ready_changed.notify_all();
}
fn timeout_request(core: &Arc<Core>, request: &Arc<Request>) -> Envelope {
    request.closed.store(true, Ordering::Release);
    if request.wire.msg_type != 5 {
        if let Some(native) = &request.native_save {
            if let Ok(mut gate) = native.gate() {
                let _ = gate.close_deadline(&native.original.claim);
            }
            lock(&native.original.state).closed = true;
        }
    }
    let Ok(object) = core.object(request.grant.object) else {
        return save_reply(&request.wire, StoreStatus::Denied);
    };
    let mut state = lock(&object.state);
    if request.wire.msg_type == 3 {
        if let Some(original) = &request.native_save {
            if let Ok(mut gate) = original.gate() {
                let _ = close_late_allocation(&mut state, &original.original, &mut gate);
            }
        }
        return save_reply(&request.wire, StoreStatus::Timeout);
    }
    if request.wire.msg_type != 5 {
        return save_reply(&request.wire, StoreStatus::Timeout);
    }
    let operation = op_of(&request.wire);
    let Ok(index) = record_index(&state, operation) else {
        return save_reply(&request.wire, StoreStatus::Denied);
    };
    let row = &state.journal.operations.as_slice()[index];
    if row.allocation.actor != request.grant.actor
        || row.allocation.expected_revision != u64_at(&request.wire.payload, 24)
    {
        return save_reply(&request.wire, StoreStatus::Denied);
    }
    let existing_closure = row.closure;
    if state
        .runtime
        .get(&operation)
        .is_some_and(|r| r.permitted.is_some())
    {
        // Issued writer authority survives ordinary retirement and deadline.
        if let Some(native) = &request.native_save {
            if let Ok(mut gate) = native.gate() {
                let _ = gate.close_deadline(&native.original.claim);
            }
            lock(&native.original.state).closed = true;
        }
        return unknown(&request.wire, operation);
    }
    let mut status = StoreStatus::Timeout;
    let mut cause = EditorClosureV0::Deadline;
    if let Some(native) = &request.native_save {
        let Ok(mut gate) = native.gate() else {
            return save_reply(&request.wire, StoreStatus::Internal);
        };
        // Actual Object -> same Gate. Observe only the real Core high-water;
        // the returned time also includes the existing Gate/Preview high-water.
        let effective = gate
            .observe_core_time(core.clock.load(Ordering::Acquire))
            .map_err(desktop_status);
        let admitted = match effective {
            Ok(_) => native.admit_held(&mut gate),
            Err(error) => Err(error),
        };
        match admitted {
            Ok(()) | Err(StoreStatus::Timeout) => {}
            Err(StoreStatus::Stale) => {
                status = StoreStatus::Stale;
                cause = if existing_closure != EditorClosureV0::None {
                    existing_closure
                } else if core.closed.load(Ordering::Acquire) {
                    EditorClosureV0::ServiceRetired
                } else if effective.is_ok_and(|now| now >= request.grant.expires) {
                    EditorClosureV0::Expired
                } else {
                    EditorClosureV0::Revoked
                };
            }
            Err(error) => return save_reply(&request.wire, error),
        }
        let _ = gate.close_deadline(&native.original.claim);
        lock(&native.original.state).closed = true;
    }
    close_unpermitted(core, &object, &mut state, operation, cause);
    save_reply(&request.wire, status)
}

fn close_late_allocation(
    state: &mut ObjectState,
    original: &SaveOriginal,
    gate: &mut desktop_service::editor_dev::SaveGateGuard<'_>,
) -> Result<(), StoreStatus> {
    if original.request.msg_type != 3 || Instant::now() < original.view.deadline {
        return Ok(());
    }
    let mut actual = checked_lock(&original.state)?;
    let Some(allocation) = &actual.allocation else {
        return Ok(());
    };
    if actual.commit_dispatched {
        return Ok(());
    }
    let operation = allocation.operation_id;
    let index = record_index(state, operation)?;
    let row = &state.journal.operations.as_slice()[index];
    if row.allocation != *allocation {
        return Err(StoreStatus::Denied);
    }
    if row.binding.is_some()
        || row.permit.is_some()
        || row.receipt.is_some()
        || row.successor.is_some()
        || state
            .runtime
            .get(&operation)
            .is_some_and(|runtime| runtime.dispatched || runtime.permitted.is_some())
    {
        return Ok(());
    }
    native(gate.close_deadline(&original.claim))?;
    close_operation(state, operation, EditorClosureV0::Deadline);
    actual.closed = true;
    Ok(())
}
pub(super) fn close_unpermitted(
    core: &Core,
    object: &Object,
    state: &mut ObjectState,
    operation: u64,
    cause: EditorClosureV0,
) {
    let held_io = lock(&core.owned).handles.values().any(|p| {
        p.object == object.id
            && p.operation == operation
            && matches!(p.kind, ProducerKind::ObjectIoWorker)
            && p.terminal_join.is_none()
    });
    if held_io {
        let _ = records(state, |rows| {
            if let Some(row) = rows
                .iter_mut()
                .find(|r| r.allocation.operation_id == operation)
            {
                if row.closure == EditorClosureV0::None {
                    row.closure = cause;
                }
            }
        });
    } else {
        close_operation(state, operation, cause);
    }
}
fn install_metadata(
    core: &Core,
    id: &ProducerId,
    kind: ProducerKind,
    original: &SaveOriginal,
    thread_id: String,
) {
    let mut state = lock(&original.state);
    if !state.installed.contains(&id.producer_id()) {
        state.installed.push(id.producer_id());
    }
    drop(state);
    lock(&core.owned).handles.insert(
        id.producer_id(),
        Producer {
            kind,
            object: original.grant.object,
            request: u64_at(&original.request.payload, 0),
            operation: op_of(&original.request),
            handle: None,
            joining: false,
            terminal_join: None,
            thread_id,
            barrier: None,
        },
    );
}
impl StoreHost {
    pub fn start_native_save(
        &self,
        registered: &RegisteredNativeSave,
    ) -> Result<NativeSaveExecution, StoreStatus> {
        let original = &registered.original.inner;
        let call = &registered.call;
        if !Arc::ptr_eq(&self.core, &registered.original.core) {
            return Err(StoreStatus::Denied);
        }
        let attachment = self.core.save_attachment()?;
        {
            let object = self.core.object(original.grant.object)?;
            let state = checked_lock(&object.state)?;
            if let Some(target) = registered
                .observed
                .as_ref()
                .filter(|_| registered.original_scope)
            {
                let gate = native(attachment.issuer.lock())?;
                let view = native(gate.admit_original_call(call))?;
                if view.actor != target.grant.actor
                    || view.object_id != object.id
                    || state.journal.selected_generation != target.tag.object_generation
                {
                    return Err(StoreStatus::Denied);
                }
            } else {
                check_call(original, call, &state)?;
            }
        }
        if original.claimed.swap(true, Ordering::AcqRel) {
            return Err(StoreStatus::NotReady);
        }
        let trace = self.core.reserve_exchange()?;
        let [dispatch, supervise] = native(call.reserve_store_pair(&attachment.witness))?;
        let source = if original.request.msg_type == 5 {
            checked_lock(&attachment.sources)?
                .get(&u64_at(&original.request.payload, 40))
                .and_then(std::sync::Weak::upgrade)
        } else {
            None
        };
        let native_request = Arc::new(NativeRequest {
            observed: registered.observed.clone(),
            original_scope: registered.original_scope,
            owner: attachment.clone(),
            original: original.clone(),
            call: Arc::downgrade(call),
            source: Mutex::new(source.clone()),
            admission: Mutex::new(None),
        });
        let execution = Arc::new(ExecutionState {
            request: Mutex::new(None),
            ready: Mutex::new(None),
            taken: AtomicBool::new(false),
            responded: AtomicBool::new(false),
            finished: Mutex::new(false),
            ready_changed: Condvar::new(),
            finished_changed: Condvar::new(),
            producers: Mutex::new(None),
            observed: registered.observed.clone(),
            original_scope: registered.original_scope,
        });
        let (start_tx, start_rx) = mpsc::channel::<u64>();
        let (watch_tx, watch_rx) = mpsc::channel::<Arc<Request>>();
        let worker_core = self.core.clone();
        let worker_original = original.clone();
        let worker_call = call.clone();
        let worker_execution = execution.clone();
        let context = native_request.clone();
        let handle=thread::Builder::new().name("store-native-save-dispatch".into()).spawn(move || {
            let _trace=trace;
            let Ok(producer)=start_rx.recv() else {return};
            let _exit=WorkerExit {core:worker_core.clone(),object:None,producer};
            let request=Arc::new(Request {native_save:Some(context),producer,grant:worker_original.grant.clone(),wire:worker_original.request,deadline:worker_call.deadline(),submitted_source:None,closed:AtomicBool::new(false)});
            *lock(&worker_execution.request)=Some(request.clone());
            let _=watch_tx.send(request.clone());
            let host=StoreHost {core:worker_core.clone()};
            let result=(|| {
                {let object=worker_core.object(request.grant.object)?;let state=checked_lock(&object.state)?;request_auth(&worker_core,&request,&state)?;}
                if let Some(target)=request.native_save.as_ref().and_then(|n|n.observed.as_ref()) {return lookup_wire(&worker_core,target,&worker_call,&request)}
                match host.fast(&request.grant,&request.wire)? {
                    Some(reply)=>{
                        if request.wire.msg_type==5&&u32::from_le_bytes(reply.payload[32..36].try_into().unwrap())==StoreStatus::Conflict as u32 {
                            let object=worker_core.object(request.grant.object)?;
                            let mut state=checked_lock(&object.state)?;
                            request_auth(&worker_core,&request,&state)?;
                            let runtime=state.runtime.get_mut(&op_of(&request.wire)).ok_or(StoreStatus::Denied)?;
                            if !runtime.dispatched {
                                runtime.dispatched=true;
                                state.dispatches=state.dispatches.checked_add(1).ok_or(StoreStatus::Exhausted)?;
                                if let Some(native)=&request.native_save {native.mark_dispatch()?;}
                            }
                        }
                        Ok(reply)
                    },None=>match request.wire.msg_type {1=>host.selected_reply(&request),3=>host.allocate(&request),5=>host.commit(&request),_=>Err(StoreStatus::Unsupported)}
                }
            })();
            let wire=match result {Ok(wire)=>wire,Err(status)=>{
                let object=worker_core.object(request.grant.object);
                let permitted=object.as_ref().is_ok_and(|o|lock(&o.state).runtime.get(&op_of(&request.wire)).is_some_and(|r|r.permitted.is_some()));
                if permitted {unknown(&request.wire,op_of(&request.wire))} else {save_reply(&request.wire,if status==StoreStatus::Unknown {StoreStatus::Internal} else {status})}
            }};
            if matches!(request.wire.msg_type,3|5) {
                let status_offset=if request.wire.msg_type==3 {16} else {32};
                let status=u32::from_le_bytes(wire.payload[status_offset..status_offset+4].try_into().unwrap());
                if status!=0&&status!=StoreStatus::Unknown as u32 {
                    if let Ok(object)=worker_core.object(request.grant.object) {
                        let state=lock(&object.state);
                        let no_permit=state.runtime.get(&op_of(&request.wire)).is_none_or(|r|r.permitted.is_none());
                        if no_permit {
                            if let Some(native)=&request.native_save {
                                if let Ok(mut gate)=native.gate() {
                                    if gate.close_original(&worker_call).is_ok() {
                                        let mut original=lock(&worker_original.state);original.closed=true;
                                        if request.wire.msg_type==3&&original.allocation.is_none()&&state.io_worker.is_none()&&!state.io_reserving&&matches!(status,1|2|5|7|8) {
                                            let _=gate.publish_original_delivery(&worker_original.claim,desktop_service::editor_dev::NativeSaveDeliveryOutcome::AllocateRejected {wire:&Envelope {handle:Handle::INVALID,..wire}});
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            respond(&worker_core,&worker_original,&worker_call,&worker_execution,wire);
            if let Some(native)=&request.native_save {
                lock(&native.source).take();
                if let Ok(object)=worker_core.object(request.grant.object) {
                    if let Some(runtime)=lock(&object.state).runtime.get_mut(&op_of(&request.wire)) {runtime.source=None;}
                }
            }
            worker_core.finished(producer);
            *lock(&worker_execution.finished)=true;worker_execution.finished_changed.notify_all();
        });
        let handle = match handle {
            Ok(handle) => handle,
            Err(_) => {
                let _ = dispatch.cancel_unspawned();
                let _ = supervise.cancel_unspawned();
                return Err(StoreStatus::Internal);
            }
        };
        let thread_id = format!("{:?}", handle.thread().id());
        let dispatch = dispatch.install(handle);
        let number = dispatch.producer_id();
        install_metadata(
            &self.core,
            &dispatch,
            ProducerKind::Dispatcher,
            original,
            thread_id,
        );
        checked_lock(&original.state)?.dispatcher_id = Some(number);
        let dispatch = retain_producer(&attachment, dispatch)?;
        let watch_core = self.core.clone();
        let watch_original = original.clone();
        let watch_call = call.clone();
        let watch_execution = execution.clone();
        let (watch_start_tx, watch_start_rx) = mpsc::channel::<()>();
        let handle = thread::Builder::new()
            .name("store-native-save-supervisor".into())
            .spawn(move || {
                if watch_start_rx.recv().is_err() {
                    return;
                }
                let request = match watch_rx.recv() {
                    Ok(request) => request,
                    Err(_) => {
                        respond(
                            &watch_core,
                            &watch_original,
                            &watch_call,
                            &watch_execution,
                            save_reply(&watch_original.request, StoreStatus::Internal),
                        );
                        return;
                    }
                };
                let mut finished = lock(&watch_execution.finished);
                while !*finished {
                    let left = watch_call
                        .deadline()
                        .saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        drop(finished);
                        let wire = timeout_request(&watch_core, &request);
                        respond(
                            &watch_core,
                            &watch_original,
                            &watch_call,
                            &watch_execution,
                            wire,
                        );
                        return;
                    }
                    finished = watch_execution
                        .finished_changed
                        .wait_timeout(finished, left)
                        .unwrap_or_else(|p| p.into_inner())
                        .0;
                }
            });
        let handle = match handle {
            Ok(handle) => handle,
            Err(_) => {
                let _ = supervise.cancel_unspawned();
                drop(start_tx);
                return Err(StoreStatus::Internal);
            }
        };
        let thread_id = format!("{:?}", handle.thread().id());
        let supervise = supervise.install(handle);
        install_metadata(
            &self.core,
            &supervise,
            ProducerKind::Supervisor,
            original,
            thread_id,
        );
        let supervise = retain_producer(&attachment, supervise)?;
        *checked_lock(&execution.producers)? = Some(vec![dispatch, supervise]);
        for barrier in lock(&self.core.barriers).iter().filter(|b| {
            let request_kind = match b.point {
                PausePoint::BeforeReadReply => 1,
                PausePoint::BeforeAllocationJournal => 3,
                PausePoint::BeforeSubmissionJournal
                | PausePoint::BeforeCommitPermit
                | PausePoint::AfterCommitPermit
                | PausePoint::BeforeCandidateWrite
                | PausePoint::AfterCandidatePublication
                | PausePoint::BeforeSelectionRename
                | PausePoint::AfterSelectionRename
                | PausePoint::AfterAcknowledgement => 5,
            };
            b.endpoint == original.grant.handle.pack()
                && b.object == original.grant.object
                && original.request.msg_type == request_kind
        }) {
            let mut state = lock(&barrier.state);
            if !state.snapshot.settled && state.snapshot.request_id == 0 {
                state.snapshot.request_id = u64_at(&original.request.payload, 0);
                state.snapshot.operation_id = op_of(&original.request);
            }
        }
        let _ = watch_start_tx.send(());
        let _ = start_tx.send(number);
        Ok(NativeSaveExecution {
            state: execution,
            _original: original.clone(),
            _call: call.clone(),
        })
    }
    pub fn execute_native_save(
        &self,
        registered: &RegisteredNativeSave,
    ) -> Result<NativeSaveReply, StoreStatus> {
        let execution = self.start_native_save(registered)?;
        loop {
            if let Some(reply) = execution.poll_once()? {
                return Ok(reply);
            }
            let ready = checked_lock(&execution.state.ready)?;
            if ready.is_some() {
                drop(ready);
                continue;
            }
            let left = registered
                .call
                .deadline()
                .saturating_duration_since(Instant::now());
            if left.is_zero() {
                drop(ready);
                let request = lock(&execution.state.request).clone();
                let wire = if let Some(request) = request {
                    timeout_request(&self.core, &request)
                } else {
                    // The installed dispatcher has not passed its start latch;
                    // no Object/IO/permit work has been admitted.
                    if let Ok(attachment) = self.core.save_attachment() {
                        if let Ok(mut gate) = attachment.issuer.lock() {
                            let _ = gate.close_deadline(&registered.original.inner.claim);
                        }
                    }
                    save_reply(&registered.original.inner.request, StoreStatus::Timeout)
                };
                respond(
                    &self.core,
                    &registered.original.inner,
                    &registered.call,
                    &execution.state,
                    wire,
                );
                continue;
            }
            drop(
                execution
                    .state
                    .ready_changed
                    .wait_timeout(ready, left)
                    .map_err(|_| StoreStatus::Internal)?
                    .0,
            );
        }
    }
}

fn checked_descriptor(descriptor: &ObjectDescriptor, data: &Data) -> Result<(), StoreStatus> {
    let h = descriptor.handle;
    if h.kind != HandleKind::Shmem
        || !(1..=65535).contains(&h.index)
        || h.generation == 0
        || h.generation > u32::MAX as u64
        || descriptor.object_generation != h.generation
        || data.handle != h
        || data.kind != descriptor.kind
    {
        return Err(StoreStatus::Denied);
    }
    Ok(())
}
struct NativeCopy<'a> {
    core: &'a Arc<Core>,
    original: &'a Arc<SaveOriginal>,
    call: &'a Arc<NativeSaveCall>,
    data: &'a Arc<Data>,
    descriptor: &'a ObjectDescriptor,
    receipt: bool,
    recovery: bool,
    captured_reply: Option<Envelope>,
}
fn copy_read(copy: NativeCopy<'_>, offset: u32, out: &mut [u8]) -> Result<(), StoreStatus> {
    let NativeCopy {
        core,
        original,
        call,
        data,
        descriptor,
        receipt,
        recovery,
        captured_reply,
    } = copy;
    let object = core.object(original.grant.object)?;
    let state = checked_lock(&object.state)?;
    if !recovery {
        auth(core, &original.grant, &state)?;
    }
    let attachment = core.save_attachment()?;
    let mut gate = native(attachment.issuer.lock())?;
    let view = if recovery {
        native(gate.admit_original_call(call))?
    } else {
        native(gate.admit_call(call))?
    };
    if view.actor != original.grant.actor
        || view.object_id != object.id
        || view.object_generation != original.tag.object_generation
        || view
            .intent_id
            .is_some_and(|intent| Some(intent) != original.view.intent_id)
    {
        return Err(StoreStatus::Denied);
    }
    if (core.closed.load(Ordering::Acquire) && !(recovery && receipt))
        || state.journal.selected_generation != original.tag.object_generation
    {
        return Err(StoreStatus::Stale);
    }
    checked_descriptor(descriptor, data)?;
    if data.actor != original.grant.actor || data.object != object.id {
        return Err(StoreStatus::Denied);
    }
    if !data.alive.load(Ordering::Acquire) {
        return Err(StoreStatus::Stale);
    }
    let bytes = checked_lock(&data.state)?;
    if descriptor.byte_len as usize != bytes.bytes.len() {
        return Err(StoreStatus::Denied);
    }
    if receipt {
        let decoded = schema(EditorSaveReceiptV0::decode_le(&bytes.bytes))?;
        let original_state = checked_lock(&original.state)?;
        let binding = original_state.binding.as_ref().ok_or(StoreStatus::Denied)?;
        schema(decoded.validate_binding(binding))?;
        if decoded.backend_service_epoch != original.grant.epoch {
            return Err(StoreStatus::Denied);
        }
    } else {
        if bytes.bytes.len() < 64 {
            return Err(StoreStatus::Invalid);
        }
        let header = schema(EditorTextHeaderV0::decode_le(&bytes.bytes[..64]))?;
        schema(header.validate_bytes(&bytes.bytes[64..]))?;
        if header.selected_object_id != object.id
            || header.revision != state.journal.current.revision
            || header.content_hash != state.journal.current.content_hash
        {
            return Err(StoreStatus::Stale);
        }
    }
    let start = offset as usize;
    let end = start.checked_add(out.len()).ok_or(StoreStatus::Invalid)?;
    let source = bytes.bytes.get(start..end).ok_or(StoreStatus::Invalid)?;
    let full = start == 0 && end == bytes.bytes.len();
    let capture = if full {
        Some(CaptureReservation::reserve(&attachment)?)
    } else {
        None
    };
    if let Some(reservation) = capture {
        if receipt {
            let original_state = checked_lock(&original.state)?;
            let bound = original_state.bound.as_ref().ok_or(StoreStatus::Denied)?;
            native(gate.publish_receipt(
                &original.claim,
                bound,
                &bytes.bytes,
                &store_current(original, &state),
            ))?;
        }
        let reply = checked_lock(&original.state)?
            .wire
            .map(|wire| encode_envelope_wire(&wire))
            .transpose()?;
        let capture_reply = captured_reply
            .map(|wire| encode_envelope_wire(&wire))
            .transpose()?
            .or(reply);
        reservation.publish(NativeSaveArtifactCapture {
            capture_id: 0,
            kind: if receipt {
                NativeSaveArtifactKind::ReceiptCopy
            } else {
                NativeSaveArtifactKind::SelectedTextCopy
            },
            actor: original.grant.actor.clone(),
            object_id: object.id,
            object_generation: state.journal.selected_generation,
            original_backend_epoch: original.grant.epoch,
            original_request: encode_envelope_wire(&original.request)?,
            original_reply: reply,
            capture_request: encode_envelope_wire(call.request())?,
            capture_reply,
            operation_id: data.operation,
            intent_id: original.view.intent_id,
            descriptor: descriptor.clone(),
            bytes: bytes.bytes.clone(),
        })?;
    }
    out.copy_from_slice(source);
    Ok(())
}
impl NativeSaveSelectedRead {
    pub fn descriptor(&self) -> ObjectDescriptor {
        self.descriptor.clone()
    }
    pub fn copy_into(&self, offset: u32, out: &mut [u8]) -> Result<(), StoreStatus> {
        self.copy_checked(&self.descriptor, offset, out)
    }
    pub fn copy_checked(
        &self,
        descriptor: &ObjectDescriptor,
        offset: u32,
        out: &mut [u8],
    ) -> Result<(), StoreStatus> {
        checked_descriptor(descriptor, &self.data)?;
        if descriptor.byte_len != self.descriptor.byte_len {
            return Err(StoreStatus::Denied);
        }
        copy_read(
            NativeCopy {
                core: &self.core,
                original: &self.original,
                call: &self.call,
                data: &self.data,
                descriptor,
                receipt: false,
                recovery: false,
                captured_reply: None,
            },
            offset,
            out,
        )
    }
}
impl NativeReceiptRead {
    pub fn descriptor(&self) -> ObjectDescriptor {
        self.descriptor.clone()
    }
    pub fn copy_into(&self, offset: u32, out: &mut [u8]) -> Result<(), StoreStatus> {
        copy_read(
            NativeCopy {
                core: &self.core,
                original: &self.original,
                call: &self.call,
                data: &self.data,
                descriptor: &self.descriptor,
                receipt: true,
                recovery: self.recovery,
                captured_reply: self.capture_reply,
            },
            offset,
            out,
        )
    }
}
impl StoreHost {
    pub fn selected_lease(
        &self,
        query: &RegisteredNativeSave,
        descriptor: &ObjectDescriptor,
    ) -> Result<NativeSelectedRead, StoreStatus> {
        if !Arc::ptr_eq(&self.core, &query.original.core) || query.call.request().msg_type != 1 {
            return Err(StoreStatus::Denied);
        }
        let wire = checked_lock(&query.original.inner.state)?
            .wire
            .ok_or(StoreStatus::NotReady)?;
        if u64_at(&wire.payload, 16) != descriptor.handle.pack()
            || u64_at(&wire.payload, 24) != descriptor.object_generation
            || u32::from_le_bytes(wire.payload[32..36].try_into().unwrap()) != descriptor.byte_len
        {
            return Err(StoreStatus::Denied);
        }
        let (data, _) = data_descriptor(
            &self.core,
            descriptor.handle.pack(),
            SharedObjectKind::SelectedText,
        )?;
        checked_descriptor(descriptor, &data)?;
        Ok(NativeSelectedRead::from_save(NativeSaveSelectedRead {
            core: self.core.clone(),
            original: query.original.inner.clone(),
            call: query.call.clone(),
            data,
            descriptor: descriptor.clone(),
        }))
    }
    pub fn receipt_lease(
        &self,
        query: &RegisteredNativeSave,
        descriptor: &ObjectDescriptor,
    ) -> Result<NativeReceiptRead, StoreStatus> {
        if !Arc::ptr_eq(&self.core, &query.original.core)
            || !matches!(query.call.request().msg_type, 5 | 7)
        {
            return Err(StoreStatus::Denied);
        }
        let wire = checked_lock(&query.original.inner.state)?
            .wire
            .ok_or(StoreStatus::NotReady)?;
        if u64_at(&wire.payload, 24) != descriptor.handle.pack()
            || u32::from_le_bytes(wire.payload[36..40].try_into().unwrap()) != descriptor.byte_len
        {
            return Err(StoreStatus::Denied);
        }
        let (data, _) = data_descriptor(
            &self.core,
            descriptor.handle.pack(),
            SharedObjectKind::Receipt,
        )?;
        checked_descriptor(descriptor, &data)?;
        Ok(NativeReceiptRead {
            core: self.core.clone(),
            original: query
                .observed
                .as_ref()
                .unwrap_or(&query.original.inner)
                .clone(),
            call: query.call.clone(),
            data,
            descriptor: descriptor.clone(),
            capture_reply: Some(wire),
            recovery: query.original_scope,
        })
    }
    pub fn native_save_producers(&self) -> Result<Vec<ProducerId>, StoreStatus> {
        let attachment = self.core.save_attachment()?;
        let mut retained = checked_lock(&attachment.producers)?;
        let mut out = vec![];
        let mut failed = None;
        retained.retain(|id| match attachment.witness.duplicate_producer(id) {
            Ok(alias) => {
                out.push(alias);
                true
            }
            Err(DesktopStatus::NotReady) => false,
            Err(status) => {
                failed = Some(desktop_status(status));
                true
            }
        });
        if let Some(status) = failed {
            return Err(status);
        };
        Ok(out)
    }
    pub fn join_native_save_finished(
        &self,
        id: &ProducerId,
    ) -> Result<JoinedProducerProof, StoreStatus> {
        self.join_native_save_owned(id, true)
    }
    fn join_native_save_owned(
        &self,
        id: &ProducerId,
        deliver_retained: bool,
    ) -> Result<JoinedProducerProof, StoreStatus> {
        let attachment = self.core.save_attachment()?;
        // Authenticate the opaque token before any evidence lookup/reservation,
        // including when the bounded retained table is full or another join runs.
        let mut live_reference = match native(attachment.witness.duplicate_producer(id)) {
            Ok(reference) => Some(reference),
            Err(StoreStatus::NotReady) => None,
            Err(status) => return Err(status),
        };
        let live = live_reference.is_some();
        // Reserve evidence capacity without any enforcing lock. The real join
        // below holds neither this mutex nor Registry/Object/Gate guards.
        let mut retained = checked_lock(&attachment.fence_joins)?;
        let closed = self.core.closed.load(Ordering::Acquire);
        if retained.joining.contains(&id.producer_id()) {
            return Err(StoreStatus::NotReady);
        }
        if closed {
            if let Some((_, proof)) = retained
                .proofs
                .iter()
                .find(|(_, proof)| attachment.witness.verify_join_for(id, proof).is_ok())
            {
                if !deliver_retained {
                    return Err(StoreStatus::NotReady);
                }
                // The authentic completed proof stays owned for the final fence.
                // Returning its private alias does not perform a second join.
                let mut delivered = checked_lock(&attachment.delivered_fence_joins)?;
                if delivered.contains(&id.producer_id()) {
                    return Err(StoreStatus::NotReady);
                }
                if delivered.len() >= 64 {
                    return Err(StoreStatus::Exhausted);
                }
                let alias = native(attachment.issuer.retain_joined_proof(proof))?;
                delivered.insert(id.producer_id());
                return Ok(alias);
            }
        }
        // An absent live row is not authority. Only the exact authentic retained
        // proof above permits once-only transfer of an already completed join.
        if !live {
            return Err(StoreStatus::NotReady);
        }
        // Reserve even while open: closure may occur during the real join.
        // Overflow preserves the actual held handle and its genuine token.
        let reserved_without_proof = retained
            .joining
            .iter()
            .filter(|id| {
                retained
                    .proofs
                    .iter()
                    .all(|(record, _)| record.producer_id != **id)
            })
            .count();
        if retained.proofs.len() + reserved_without_proof >= 64 {
            return Err(StoreStatus::Exhausted);
        }
        retained.joining.insert(id.producer_id());
        drop(retained);
        let proof = match native(attachment.witness.join_finished(id)) {
            Ok(proof) => proof,
            Err(status) => {
                checked_lock(&attachment.fence_joins)?
                    .joining
                    .remove(&id.producer_id());
                return Err(status);
            }
        };
        let observation = native(attachment.witness.verify_join_for(id, &proof))?;
        // The same evidence mutex serializes this completed join with the Core
        // closed transition. A join recorded before closure is already complete.
        let mut retained = checked_lock(&attachment.fence_joins)?;
        let closed = self.core.closed.load(Ordering::Acquire);
        let mut owned = checked_lock(&self.core.owned)?;
        let metadata = owned.handles.get(&id.producer_id()).map(|p| {
            (
                p.kind.clone(),
                p.object,
                p.request,
                p.operation,
                p.barrier.clone(),
            )
        });
        if closed {
            if let Some(producer) = owned.handles.get_mut(&id.producer_id()) {
                producer.terminal_join = Some(match observation.outcome {
                    desktop_service::editor_dev::JoinOutcome::Returned => {
                        ProducerJoinOutcome::Returned
                    }
                    desktop_service::editor_dev::JoinOutcome::Panicked => {
                        ProducerJoinOutcome::Panicked
                    }
                });
            }
        } else {
            owned.handles.remove(&id.producer_id());
        }
        drop(owned);
        if closed {
            let (kind, object, request, operation, barrier) = metadata.unwrap_or((
                match observation.kind {
                    NativeProducerKind::StoreIo => ProducerKind::ObjectIoWorker,
                    NativeProducerKind::DesktopSupervisor | NativeProducerKind::StoreSupervisor => {
                        ProducerKind::Supervisor
                    }
                    _ => ProducerKind::Dispatcher,
                },
                0,
                0,
                0,
                None,
            ));
            let record = ProducerFence {
                producer_id: observation.producer_id,
                producer_generation: observation.producer_generation,
                kind,
                native_pid: observation.native_pid,
                rust_thread_id: observation.rust_thread_id,
                selected_object_id: (object != 0).then_some(object),
                request_id: (request != 0).then_some(request),
                operation_id: (operation != 0).then_some(operation),
                completed_join: true,
                join_outcome: match observation.outcome {
                    desktop_service::editor_dev::JoinOutcome::Returned => {
                        ProducerJoinOutcome::Returned
                    }
                    desktop_service::editor_dev::JoinOutcome::Panicked => {
                        ProducerJoinOutcome::Panicked
                    }
                },
                settled_barrier: barrier.map(|b| lock(&b.state).snapshot.clone()),
                live_ownership_after_join: false,
            };
            let alias = native(attachment.issuer.retain_joined_proof(&proof))?;
            retained.proofs.push((record, alias));
            if deliver_retained {
                checked_lock(&attachment.delivered_fence_joins)?.insert(id.producer_id());
            }
        }
        // The reservation remains through genuine post-join bookkeeping so
        // quiesce cannot publish a fence before original completion settles.
        // In particular, never carry the evidence mutex into Object cleanup.
        drop(retained);
        if observation.kind == NativeProducerKind::StoreIo {
            for object in self.core.objects.values() {
                let mut state = lock(&object.state);
                let pending = lock(&self.core.owned)
                    .handles
                    .values()
                    .filter(|p| {
                        p.object == object.id
                            && matches!(p.kind, ProducerKind::ObjectIoWorker)
                            && p.terminal_join.is_none()
                    })
                    .map(|p| p.operation)
                    .collect::<Vec<_>>();
                let closed = state
                    .journal
                    .operations
                    .as_slice()
                    .iter()
                    .filter(|r| {
                        r.closure != EditorClosureV0::None
                            && r.permit.is_none()
                            && !pending.contains(&r.allocation.operation_id)
                    })
                    .map(|r| (r.allocation.operation_id, r.closure))
                    .collect::<Vec<_>>();
                for (operation, cause) in closed {
                    close_operation(&mut state, operation, cause);
                }
            }
        }
        // Compact completion comes from this actual owned dispatcher join,
        // never from a child-set bool, returned reply or an elapsed deadline.
        for original in checked_lock(&attachment.originals)?.iter() {
            let mut state = checked_lock(&original.state)?;
            if state.installed.contains(&id.producer_id())
                && original.request.msg_type == 3
                && state.allocation.is_none()
            {
                if state.joined.len() >= 64 {
                    return Err(StoreStatus::Internal);
                }
                state
                    .joined
                    .push(native(attachment.issuer.retain_joined_proof(&proof))?);
            }
            let terminal_reply = state.wire.as_ref().is_some_and(|wire| {
                wire.msg_type == 6
                    && u32::from_le_bytes(wire.payload[32..36].try_into().unwrap())
                        != StoreStatus::Unknown as u32
            });
            if original.request.msg_type == 5
                && state.installed.contains(&id.producer_id())
                && !terminal_reply
            {
                if observation.origin_id != original.view.origin_id
                    || observation.producer_generation != 1
                    || state.joined.len() >= 64
                    || state
                        .joined_tokens
                        .iter()
                        .any(|token| token.producer_id() == id.producer_id())
                {
                    return Err(StoreStatus::Internal);
                }
                let reference = live_reference.take().ok_or(StoreStatus::Internal)?;
                native(attachment.witness.verify_join_for(&reference, &proof))?;
                state
                    .joined
                    .push(native(attachment.issuer.retain_joined_proof(&proof))?);
                state.joined_tokens.push(reference);
            }
            if state.dispatcher_id == Some(id.producer_id()) {
                state.completion = match observation.outcome {
                    desktop_service::editor_dev::JoinOutcome::Returned => {
                        OriginalTicketCompletion::JoinedReturned
                    }
                    desktop_service::editor_dev::JoinOutcome::Panicked => {
                        OriginalTicketCompletion::JoinedPanicked
                    }
                };
            }
        }
        checked_lock(&attachment.fence_joins)?
            .joining
            .remove(&id.producer_id());
        Ok(proof)
    }
}

// Every observation below is derived from a retained authentic join proof and
// its private pre-join token, never from an ID or an elapsed deadline alone.
fn original_commit_joins(
    core: &Core,
    attachment: &SaveCore,
    target: &SaveOriginal,
) -> Result<Vec<NativeJoinObservation>, StoreStatus> {
    let actual = checked_lock(&target.state)?;
    if actual.completion != OriginalTicketCompletion::JoinedReturned
        || actual.installed.is_empty()
        || actual.installed.len() > 64
        || actual.joined.len() != actual.installed.len()
        || actual.joined_tokens.len() != actual.installed.len()
    {
        return Err(StoreStatus::NotReady);
    }
    let mut observations = BTreeMap::new();
    for (token, proof) in actual.joined_tokens.iter().zip(&actual.joined) {
        let view = native(attachment.witness.verify_join_for(token, proof))?;
        if view.origin_id != target.view.origin_id
            || view.producer_generation != 1
            || view.outcome != desktop_service::editor_dev::JoinOutcome::Returned
            || !actual.installed.contains(&view.producer_id)
            || observations.insert(view.producer_id, view).is_some()
        {
            return Err(StoreStatus::Denied);
        }
    }
    if actual
        .installed
        .iter()
        .any(|id| !observations.contains_key(id))
    {
        return Err(StoreStatus::NotReady);
    }
    // A first permitted Commit genuinely installed the original Desktop entry
    // pair, Store execution pair, and IO worker. Missing entry ownership cannot
    // be replaced by an otherwise complete subset of numeric installed IDs.
    for kind in [
        NativeProducerKind::DesktopDispatch,
        NativeProducerKind::DesktopSupervisor,
        NativeProducerKind::StoreDispatch,
        NativeProducerKind::StoreSupervisor,
    ] {
        if observations
            .values()
            .filter(|view| view.kind == kind)
            .count()
            != 1
        {
            return Err(StoreStatus::NotReady);
        }
    }
    if !observations
        .values()
        .any(|view| view.kind == NativeProducerKind::StoreIo)
    {
        return Err(StoreStatus::NotReady);
    }
    let joining = checked_lock(&attachment.fence_joins)?;
    let owned = checked_lock(&core.owned)?;
    if actual.installed.iter().any(|id| {
        joining.joining.contains(id)
            || owned
                .handles
                .get(id)
                .is_some_and(|row| row.terminal_join.is_none() || row.joining)
    }) {
        return Err(StoreStatus::NotReady);
    }
    Ok(observations.into_values().collect())
}

struct ReadbackRuntime {
    operation: u64,
    original: Option<Arc<Request>>,
    source: Option<Arc<Data>>,
    permitted: Option<EditorCommitPermitV0>,
    dispatched: bool,
}
struct OriginalReadbackBaseline {
    journal: EditorSelectionJournalV0,
    durable: EditorSelectionJournalV0,
    synced: Option<EditorSelectionJournalV0>,
    prepared: Option<EditorSelectionJournalV0>,
    digest: Hash32,
    bytes: Vec<u8>,
    owner_hash: Hash32,
    next_op: u64,
    next_writer: u64,
    admission_epoch: u64,
    allocation_fenced: bool,
    mutation_fenced: bool,
    quarantine: bool,
    preview_faulted: bool,
    fault: Option<IoPoint>,
    force_op_range: bool,
    force_writer_range: bool,
    dispatches: u32,
    permits: u32,
    transitions: u32,
    closed: bool,
    runtime: Vec<ReadbackRuntime>,
    joins: Vec<NativeJoinObservation>,
}
impl OriginalReadbackBaseline {
    fn capture(core: &Core, state: &ObjectState, joins: Vec<NativeJoinObservation>) -> Self {
        Self {
            journal: state.journal.clone(),
            durable: state.durable.clone(),
            synced: state.synced.clone(),
            prepared: state.prepared.clone(),
            digest: state.digest,
            bytes: state.bytes.clone(),
            owner_hash: state.owner_hash,
            next_op: state.next_op,
            next_writer: state.next_writer,
            admission_epoch: state.admission_epoch,
            allocation_fenced: state.allocation_fenced,
            mutation_fenced: state.mutation_fenced,
            quarantine: state.quarantine,
            preview_faulted: state.preview_faulted,
            fault: state.fault,
            force_op_range: state.force_op_range,
            force_writer_range: state.force_writer_range,
            dispatches: state.dispatches,
            permits: state.permits,
            transitions: state.transitions,
            closed: core.closed.load(Ordering::Acquire),
            runtime: state
                .runtime
                .iter()
                .map(|(id, row)| ReadbackRuntime {
                    operation: *id,
                    original: row.original.clone(),
                    source: row.source.clone(),
                    permitted: row.permitted.clone(),
                    dispatched: row.dispatched,
                })
                .collect(),
            joins,
        }
    }
    fn matches(&self, core: &Core, state: &ObjectState) -> bool {
        self.journal == state.journal
            && self.durable == state.durable
            && self.synced == state.synced
            && self.prepared == state.prepared
            && self.digest == state.digest
            && self.bytes == state.bytes
            && self.owner_hash == state.owner_hash
            && self.next_op == state.next_op
            && self.next_writer == state.next_writer
            && self.admission_epoch == state.admission_epoch
            && self.allocation_fenced == state.allocation_fenced
            && self.mutation_fenced == state.mutation_fenced
            && self.quarantine == state.quarantine
            && self.preview_faulted == state.preview_faulted
            && self.fault == state.fault
            && self.force_op_range == state.force_op_range
            && self.force_writer_range == state.force_writer_range
            && self.dispatches == state.dispatches
            && self.permits == state.permits
            && self.transitions == state.transitions
            && self.closed == core.closed.load(Ordering::Acquire)
            && self.runtime.len() == state.runtime.len()
            && self.runtime.iter().all(|old| {
                state.runtime.get(&old.operation).is_some_and(|now| {
                    old.dispatched == now.dispatched
                        && old.permitted == now.permitted
                        && match (&old.original, &now.original) {
                            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                            (None, None) => true,
                            _ => false,
                        }
                        && match (&old.source, &now.source) {
                            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                            (None, None) => true,
                            _ => false,
                        }
                })
            })
    }
}

fn reconcile_original_receipt(
    core: &Arc<Core>,
    target: &Arc<SaveOriginal>,
    call: &Arc<NativeSaveCall>,
    request: &Arc<Request>,
) -> Result<bool, StoreStatus> {
    let attachment = core.save_attachment()?;
    let object = core.object(target.grant.object)?;
    let operation = op_of(&target.request);
    let (baseline, binding, permit) = {
        let state = checked_lock(&object.state)?;
        let gate = native(attachment.issuer.lock())?;
        let view = native(gate.admit_original_call(call))?;
        if request.native_save.as_ref().is_none_or(|n| {
            !n.original_scope || n.observed.as_ref().is_none_or(|r| !Arc::ptr_eq(r, target))
        }) || call.request().msg_type != 7
            || target.request.msg_type != 5
            || view.actor != target.grant.actor
            || view.object_id != object.id
            || view.object_generation != target.tag.object_generation
            || view
                .intent_id
                .is_some_and(|id| Some(id) != target.view.intent_id)
            || u64_at(&call.request().payload, 24) != operation
        {
            return Err(StoreStatus::Denied);
        }
        if state.io_worker.is_some() || state.io_reserving {
            return Ok(false);
        }
        let row = &state.journal.operations.as_slice()[record_index(&state, operation)?];
        if row.receipt.is_some() {
            return Ok(true);
        }
        let runtime = state.runtime.get(&operation).ok_or(StoreStatus::Denied)?;
        let binding = row.binding.as_ref().ok_or(StoreStatus::NotReady)?.clone();
        let permit = runtime
            .permitted
            .as_ref()
            .ok_or(StoreStatus::NotReady)?
            .clone();
        if row.permit.as_ref() != Some(&permit)
            || permit.binding != binding
            || permit.service_epoch != target.grant.epoch
            || row.submitted_service_epoch != Some(target.grant.epoch)
            || binding.allocation.actor != target.grant.actor
            || binding.allocation.selected_object_id != object.id
            || binding.allocation.selected_generation != target.tag.object_generation
            || binding.allocation.operation_id != operation
            || binding.allocation.expected_revision != u64_at(&target.request.payload, 24)
            || binding.source.source_object_id != u64_at(&target.request.payload, 40)
            || runtime
                .original
                .as_ref()
                .and_then(|r| r.native_save.as_ref())
                .is_none_or(|n| !Arc::ptr_eq(&n.original, target))
        {
            return Err(StoreStatus::Denied);
        }
        {
            let actual = checked_lock(&target.state)?;
            if actual.binding.as_ref() != Some(&binding)
                || actual
                    .bound
                    .as_ref()
                    .is_none_or(|bound| bound.binding() != &binding)
            {
                return Err(StoreStatus::Denied);
            }
        }
        let joins = match original_commit_joins(core, &attachment, target) {
            Ok(joins) => joins,
            Err(StoreStatus::NotReady) => return Ok(false),
            Err(error) => return Err(error),
        };
        (
            OriginalReadbackBaseline::capture(core, &state, joins),
            binding,
            permit,
        )
    };
    let runtime = baseline.journal.clone();
    let durable = baseline
        .synced
        .as_ref()
        .filter(|j| j.transition_sequence >= baseline.durable.transition_sequence)
        .unwrap_or(&baseline.durable)
        .clone();
    let permits = baseline
        .runtime
        .iter()
        .filter_map(|r| r.permitted.clone())
        .collect::<Vec<_>>();
    let profile = core.profile.clone();
    // Only the fresh original-observation call owns this IO reservation/thread.
    // Registry, Object, Gate and proof guards above have all been released.
    let read = Core::run_io(core, &object, request, operation, 0, move |context| {
        storage::Observer::before(&context, IoPoint::JournalReadback, None)?;
        let result =
            read_validated_selection(&context.object, &profile, &runtime, &durable, &permits);
        let hash = result
            .as_ref()
            .ok()
            .and_then(|(journal, _)| journal.digest().ok());
        storage::Observer::after(&context, IoPoint::JournalReadback, hash, result.is_ok());
        result
    });
    let (disk, consumed) = match read {
        Ok(value) => value,
        Err(StoreStatus::Unknown | StoreStatus::Invalid | StoreStatus::NotReady) => {
            return Ok(false);
        }
        Err(error) => return Err(error),
    };
    if disk.service_epoch != baseline.journal.service_epoch {
        return Ok(false);
    }
    let Some(row) = disk
        .operations
        .as_slice()
        .iter()
        .find(|row| row.allocation.operation_id == operation)
    else {
        return Ok(false);
    };
    let Some(receipt) = row.receipt.as_ref() else {
        return Ok(false);
    };
    schema(receipt.validate_binding(&binding))?;
    if row.binding.as_ref() != Some(&binding)
        || row.permit.as_ref() != Some(&permit)
        || row.submitted_service_epoch != Some(target.grant.epoch)
        || receipt.backend_service_epoch != target.grant.epoch
        || !matches!(
            row.state,
            EditorOperationStateV0::Committed | EditorOperationStateV0::Noncommit
        )
        || receipt.outcome == artifact_store_schema::editor_save::EditorReceiptOutcomeV0::Committed
            && row.successor.as_ref() != Some(&disk.current)
    {
        return Ok(false);
    }
    let digest = schema(disk.digest())?;
    let mut state = checked_lock(&object.state)?;
    let mut gate = native(attachment.issuer.lock())?;
    let view = native(gate.admit_original_call(call))?;
    if view.actor != target.grant.actor
        || view.object_id != object.id
        || view.object_generation != target.tag.object_generation
        || view
            .intent_id
            .is_some_and(|id| Some(id) != target.view.intent_id)
    {
        return Err(StoreStatus::Denied);
    }
    if !baseline.matches(core, &state)
        || state.io_worker.is_some()
        || state.io_reserving
        || original_commit_joins(core, &attachment, target)? != baseline.joins
    {
        return Ok(false);
    }
    let current = StoreCurrentSelection {
        owner: desktop_service::editor_dev::PreviewOwner {
            owner_id: target.grant.actor.owner_id,
            session_id: target.grant.actor.session_id,
            session_generation: target.grant.actor.session_generation,
        },
        object_id: object.id,
        object_generation: disk.selected_generation,
        store_epoch: disk.service_epoch,
        revision: disk.current.revision,
        content_hash: disk.current.content_hash,
    };
    native(gate.publish_current_selection(&current))?;
    install_journal(&mut state, disk, digest);
    state.bytes = consumed.bytes;
    state.owner_hash = consumed.owner_hash;
    state.quarantine = false;
    // The real terminal row/selection is installed; protected receipt publication
    // still happens only at the genuine successful full176 receipt copy.
    let mut actual = checked_lock(&target.state)?;
    actual.joined.clear();
    actual.joined_tokens.clear();
    Ok(true)
}

fn lookup_wire(
    core: &Arc<Core>,
    target: &Arc<SaveOriginal>,
    call: &Arc<NativeSaveCall>,
    request: &Arc<Request>,
) -> Result<Envelope, StoreStatus> {
    lookup_wire_once(core, target, call, request, true)
}
fn lookup_wire_once(
    core: &Arc<Core>,
    target: &Arc<SaveOriginal>,
    call: &Arc<NativeSaveCall>,
    request: &Arc<Request>,
    allow_readback: bool,
) -> Result<Envelope, StoreStatus> {
    let attachment = core.save_attachment()?;
    let mut registry = checked_lock(&core.registry)?;
    let object = core.object(target.grant.object)?;
    let state = checked_lock(&object.state)?;
    let gate = native(attachment.issuer.lock())?;
    let view = if request
        .native_save
        .as_ref()
        .is_some_and(|n| n.original_scope)
    {
        native(gate.admit_original_call(call))?
    } else {
        auth(core, &target.grant, &state)?;
        native(gate.admit_call(call))?
    };
    if view.actor != target.grant.actor
        || view.object_id != object.id
        || view.object_generation != target.tag.object_generation
        || view
            .intent_id
            .is_some_and(|v| Some(v) != target.view.intent_id)
    {
        return Err(StoreStatus::Denied);
    }
    let q = call.request();
    if q.msg_type == 9 {
        if target.request.msg_type != 3
            || u64_at(&q.payload, 24) != u64_at(&target.request.payload, 0)
        {
            return Err(StoreStatus::Denied);
        }
        drop(gate);
        drop(state);
        drop(registry);
        let disk = Core::run_io(core, &object, request, 0, 0, move |context| {
            storage::Observer::before(&context, IoPoint::JournalReadback, None)?;
            let raw = storage::read(&context.object.root.join("selection.json"), 131072);
            let hash = raw.as_ref().ok().map(|bytes| storage::hash(bytes));
            let parsed =
                raw.and_then(|bytes| schema(EditorSelectionJournalV0::decode_canonical(&bytes)));
            storage::Observer::after(&context, IoPoint::JournalReadback, hash, parsed.is_ok());
            parsed
        })?;
        let mut state = checked_lock(&object.state)?;
        let mut gate = native(attachment.issuer.lock())?;
        native(gate.admit_original_call(call))?;
        if disk.owner_id != state.journal.owner_id
            || disk.selected_object_id != object.id
            || disk.selected_generation != state.journal.selected_generation
            || disk.service_epoch != state.journal.service_epoch
            || disk.initial != state.journal.initial
            || disk.signing_policy_hash != state.journal.signing_policy_hash
        {
            return Err(StoreStatus::Unknown);
        }
        close_late_allocation(&mut state, target, &mut gate)?;
        let mut actual = checked_lock(&target.state)?;
        let allocation = actual.allocation.as_ref().and_then(|a| {
            disk.operations
                .as_slice()
                .iter()
                .find(|row| row.allocation == *a)
        });
        let operation = if let Some(row) = allocation {
            let canonical = a::AllocateSaveIdReply {
                request_id: u64_at(&target.request.payload, 0),
                operation_id: row.allocation.operation_id,
                status: 0,
                reserved: 0,
            }
            .encode(Handle::INVALID)?;
            native(gate.publish_allocation(&target.claim, &canonical))?;
            row.allocation.operation_id
        } else if actual.no_allocation && actual.allocation.is_none() {
            0
        } else if actual.allocation.is_none()
            && actual.closed
            && actual.completion == OriginalTicketCompletion::JoinedReturned
            && actual.joined.len() == actual.installed.len()
        {
            native(gate.publish_original_delivery(
                &target.claim,
                desktop_service::editor_dev::NativeSaveDeliveryOutcome::NoAllocation {
                    joined: &actual.joined,
                },
            ))?;
            actual.joined.clear();
            actual.no_allocation = true;
            0
        } else {
            return Ok(save_reply(q, StoreStatus::Unknown));
        };
        return a::ObserveAllocationReply {
            request_id: u64_at(&q.payload, 0),
            original_allocate_request_id: u64_at(&target.request.payload, 0),
            operation_id: operation,
            status: 0,
            reserved: 0,
        }
        .encode(Handle::INVALID);
    }
    if q.msg_type != 7
        || target.request.msg_type != 5
        || u64_at(&q.payload, 24) != op_of(&target.request)
    {
        return Err(StoreStatus::Denied);
    }
    let index = record_index(&state, op_of(&target.request))?;
    let row = &state.journal.operations.as_slice()[index];
    if row.allocation.actor != target.grant.actor
        || row.allocation.expected_revision != u64_at(&target.request.payload, 24)
    {
        return Err(StoreStatus::Denied);
    }
    let Some(binding) = row.binding.as_ref() else {
        return Ok(save_reply(q, StoreStatus::NotReady));
    };
    if binding.source.source_object_id != u64_at(&target.request.payload, 40) {
        return Err(StoreStatus::Denied);
    }
    let Some(receipt) = row.receipt.as_ref() else {
        let operation = row.allocation.operation_id;
        if !allow_readback
            || request
                .native_save
                .as_ref()
                .is_none_or(|n| !n.original_scope)
        {
            return Ok(unknown(q, operation));
        }
        // The genuine lookup owns its own IO. No Registry/Object/Gate guard
        // spans that read or any real join performed by its IO finalizer.
        drop(gate);
        drop(state);
        drop(registry);
        if reconcile_original_receipt(core, target, call, request)? {
            return lookup_wire_once(core, target, call, request, false);
        }
        return Ok(unknown(q, operation));
    };
    schema(receipt.validate_binding(binding))?;
    if receipt.backend_service_epoch != target.grant.epoch {
        return Err(StoreStatus::Denied);
    }
    let bytes = schema(receipt.encode_le())?.to_vec();
    let data = if let Some(data) = registry.data.values().find(|d| {
        d.alive.load(Ordering::Acquire)
            && d.actor == target.grant.actor
            && d.object == object.id
            && d.kind == SharedObjectKind::Receipt
            && d.operation == Some(binding.allocation.operation_id)
    }) {
        data.clone()
    } else {
        registry.data.retain(|_, row| Arc::strong_count(row) > 1);
        if registry.data.len() >= 32 {
            return Err(StoreStatus::Exhausted);
        }
        let handle = core.new_data_handle(&mut registry)?;
        let data = Arc::new(Data {
            handle,
            actor: target.grant.actor.clone(),
            object: object.id,
            operation: Some(binding.allocation.operation_id),
            revision: receipt.result_revision,
            kind: SharedObjectKind::Receipt,
            alive: AtomicBool::new(true),
            state: Mutex::new(DataState {
                bytes,
                writable: false,
                frozen: true,
            }),
        });
        registry.data.insert(handle.pack(), data.clone());
        data
    };
    {
        let mut original = checked_lock(&target.state)?;
        original.binding = Some(binding.clone());
        original.joined.clear();
        original.joined_tokens.clear();
    }
    a::SaveStatusReply {
        request_id: u64_at(&q.payload, 0),
        operation_id: binding.allocation.operation_id,
        revision: receipt.result_revision,
        receipt_shm: data.handle.pack(),
        status: 0,
        receipt_len: 176,
    }
    .encode(Handle::INVALID)
}
impl StoreHost {
    fn register_original_lookup(
        &self,
        call: NativeSaveCall,
        target: &Arc<SaveOriginal>,
    ) -> Result<RegisteredNativeSave, NativeSaveRegistrationFailure> {
        let call = Arc::new(call);
        let mut producers = vec![];
        let outcome = (|| {
            let attachment = self.core.save_attachment()?;
            // The actual transferred compact table is the only cross-epoch
            // lineage route; an artifact's numeric actor/op cannot enroll it.
            if !checked_lock(&attachment.originals)?
                .iter()
                .any(|r| Arc::ptr_eq(r, target))
            {
                return Err(StoreStatus::Denied);
            }
            let view = native(call.admit_original(&attachment.witness))?.view();
            if view.actor != target.grant.actor
                || view.object_id != target.grant.object
                || view.object_generation != target.tag.object_generation
                || view
                    .intent_id
                    .is_some_and(|v| Some(v) != target.view.intent_id)
            {
                return Err(StoreStatus::Denied);
            }
            codec::validate(call.request())?;
            let q = call.request();
            if (q.msg_type == 9
                && (target.request.msg_type != 3
                    || u64_at(&q.payload, 24) != u64_at(&target.request.payload, 0)))
                || (q.msg_type == 7
                    && (target.request.msg_type != 5
                        || u64_at(&q.payload, 24) != op_of(&target.request)))
                || !matches!(q.msg_type, 7 | 9)
            {
                return Err(StoreStatus::Denied);
            }
            if q.handle != target.grant.handle
                || u64_at(&q.payload, 8) != target.grant.actor.session_id
                || u64_at(&q.payload, 16) != target.grant.actor.session_generation
            {
                return Err(StoreStatus::Denied);
            }
            let mut gate = native(attachment.issuer.lock())?;
            let claim = native(gate.claim_execution(&call))?;
            let recovery = native(gate.original_recovery(&claim))?;
            let inner = Arc::new(SaveOriginal {
                core: Arc::downgrade(&self.core),
                attachment: Arc::downgrade(&attachment),
                grant: target.grant.clone(),
                tag: target.tag.clone(),
                claim,
                recovery,
                request: *q,
                view,
                claimed: AtomicBool::new(false),
                state: Mutex::new(OriginalState {
                    wire: None,
                    allocation: None,
                    binding: None,
                    bound: None,
                    completion: OriginalTicketCompletion::Pending,
                    closed: false,
                    commit_dispatched: false,
                    allocation_delivered: false,
                    dispatcher_id: None,
                    installed: vec![],
                    joined: vec![],
                    joined_tokens: vec![],
                    no_allocation: false,
                }),
            });
            drop(gate);
            producers = retain_entry_producers(&self.core, &attachment, &call, &inner)?;
            Ok(inner)
        })();
        match outcome {
            Ok(inner) => Ok(RegisteredNativeSave {
                original: OriginalNativeSave {
                    inner,
                    core: self.core.clone(),
                },
                call,
                observed: Some(target.clone()),
                original_scope: true,
            }),
            Err(status) => Err(NativeSaveRegistrationFailure {
                status,
                call,
                original: None,
                producers,
            }),
        }
    }
    pub fn observe_original(
        &self,
        lookup: NativeSaveCall,
        original: &OriginalNativeSave,
    ) -> Result<NativeOriginalReply, NativeSaveRegistrationFailure> {
        let registered = self.register_original_lookup(lookup, &original.inner)?;
        self.observe_registered(&registered, &original.inner)
    }
    pub fn recover_original(
        &self,
        lookup: NativeSaveCall,
        owner: &NativeSaveRecovery,
    ) -> Result<NativeOriginalReply, NativeSaveRegistrationFailure> {
        let registered = self.register_original_lookup(lookup, &owner.original)?;
        self.observe_registered(&registered, &owner.original)
    }
    fn observe_registered(
        &self,
        registered: &RegisteredNativeSave,
        target: &Arc<SaveOriginal>,
    ) -> Result<NativeOriginalReply, NativeSaveRegistrationFailure> {
        // No second dispatch or mutation route exists here: start claims this
        // newly registered lookup once, and lookup_wire only reads the target.
        let reply = self.execute_native_save(registered);
        let reply = match reply {
            Ok(reply) => reply,
            Err(status) => {
                // The public failure retains its actual call. A private duplicate
                // of an opaque call is not available; registration already owns it.
                return Err(self.lookup_failure(registered, status));
            }
        };
        let NativeSaveReplyParts {
            wire,
            allocation: _,
            selected: _,
            receipt,
            editor: _,
            original: _,
            producers,
        } = reply.into_parts();
        let actual = lock(&target.state);
        let original_wire = actual.wire;
        let (observation, allocation) = if let Some(receipt) = receipt {
            let binding = actual
                .binding
                .clone()
                .ok_or_else(|| self.lookup_failure(registered, StoreStatus::Internal))?;
            (
                NativeOriginalObservation::Receipt { binding, receipt },
                None,
            )
        } else if target.request.msg_type == 3 {
            let successful = wire.msg_type == 10
                && u32::from_le_bytes(wire.payload[24..28].try_into().unwrap()) == 0;
            match &actual.allocation {
                Some(a) if successful && u64_at(&wire.payload, 16) == a.operation_id => {
                    let allocation =
                        if !actual.allocation_delivered && Instant::now() < target.view.deadline {
                            Some(NativeAllocatedOperation {
                                core: self.core.clone(),
                                call: registered.call.clone(),
                                original: target.clone(),
                                allocation: a.clone(),
                            })
                        } else {
                            None
                        };
                    (
                        NativeOriginalObservation::AllocationRetained {
                            allocation: a.clone(),
                            commit_dispatched: actual.commit_dispatched,
                        },
                        allocation,
                    )
                }
                None if successful && u64_at(&wire.payload, 16) == 0 => {
                    (NativeOriginalObservation::NoAllocation, None)
                }
                _ => (NativeOriginalObservation::AllocationPending, None),
            }
        } else if let Some(binding) = &actual.binding {
            (
                NativeOriginalObservation::CommitPending {
                    binding: binding.clone(),
                },
                None,
            )
        } else {
            (NativeOriginalObservation::AllocationPending, None)
        };
        drop(actual);
        if allocation.is_some() {
            lock(&target.state).allocation_delivered = true;
        }
        Ok(NativeOriginalReply {
            wire,
            original_wire,
            allocation,
            observation,
            producers,
        })
    }
}
impl StoreHost {
    fn lookup_failure(
        &self,
        registered: &RegisteredNativeSave,
        status: StoreStatus,
    ) -> NativeSaveRegistrationFailure {
        let producers = self
            .core
            .save_attachment()
            .ok()
            .and_then(|a| registered.call.entry_producers(&a.witness).ok())
            .map(|p| p.into_iter().collect())
            .unwrap_or_default();
        NativeSaveRegistrationFailure {
            status,
            call: registered.call.clone(),
            original: Some(OriginalNativeSave {
                inner: registered.original.inner.clone(),
                core: self.core.clone(),
            }),
            producers,
        }
    }
}

pub struct QuiescedNativeSaveOwner {
    core: Arc<Core>,
    attachment: Arc<SaveCore>,
    base: Box<FenceSnapshot>,
    tickets: Vec<OriginalTicketObservation>,
    proofs: Vec<JoinedProducerProof>,
    attempt: Option<Arc<Core>>,
}
pub struct NativeSaveReopenFailure {
    pub error: FixtureError,
    pub owner: QuiescedNativeSaveOwner,
}
impl QuiescedNativeSaveOwner {
    pub fn fence_snapshot(&self) -> NativeSaveFenceSnapshot {
        NativeSaveFenceSnapshot {
            schema_version: 1,
            base: (*self.base).clone(),
            original_tickets: self.tickets.clone(),
        }
    }
}
impl FixtureController {
    pub fn quiesce_native_save(
        &self,
        timeout_ms: u64,
    ) -> Result<QuiescedNativeSaveOwner, FixtureError> {
        let attachment = fixture(self.core.save_attachment())?;
        if !(1..=2000).contains(&timeout_ms) {
            return Err(FixtureError::new(StoreStatus::Invalid));
        }
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
        let until = Instant::now() + Duration::from_millis(timeout_ms);
        {
            let _registry = lock(&self.core.registry);
            let objects = self.core.objects.values().collect::<Vec<_>>();
            let mut states = objects.iter().map(|o| lock(&o.state)).collect::<Vec<_>>();
            if self.core.setup_active.load(Ordering::Acquire) {
                return Err(FixtureError::new(StoreStatus::NotReady));
            }
            // Serialize only the closed transition with proof evidence, not
            // actual joins. Drop this mutex before any Gate admission/retirement.
            {
                let _proofs = checked_lock(&attachment.fence_joins).map_err(FixtureError::new)?;
                self.core.closed.store(true, Ordering::Release);
            }
            fixture(native(attachment.issuer.retire_objects_for_fence(
                &objects.iter().map(|o| o.id).collect::<Vec<_>>(),
            )))?;
            for state in &mut states {
                for operation in state.runtime.keys().copied().collect::<Vec<_>>() {
                    close_operation(state, operation, EditorClosureV0::ServiceRetired);
                }
            }
            for original in lock(&attachment.originals).iter() {
                lock(&original.state).closed = true;
            }
        }
        let host = StoreHost {
            core: self.core.clone(),
        };
        let mut service_producers = vec![];
        let mut object_workers = vec![];
        loop {
            for id in fixture(host.native_save_producers())? {
                match host.join_native_save_owned(&id, false) {
                    Ok(_) => {}
                    Err(StoreStatus::NotReady) => {}
                    Err(status) => return Err(FixtureError::new(status)),
                }
            }
            if attachment.witness.producer_counts().held == 0
                && checked_lock(&attachment.fence_joins)
                    .map_err(FixtureError::new)?
                    .joining
                    .is_empty()
            {
                break;
            }
            if Instant::now() >= until {
                return Err(FixtureError::new(StoreStatus::NotReady));
            }
            thread::yield_now();
        }
        for (record, _) in lock(&attachment.fence_joins).proofs.iter() {
            if matches!(record.kind, ProducerKind::ObjectIoWorker) {
                object_workers.push(record.clone())
            } else {
                service_producers.push(record.clone())
            }
        }
        service_producers.sort_by_key(|p| p.producer_id);
        object_workers.sort_by_key(|p| p.producer_id);
        let fence_id = {
            let mut owned = lock(&self.core.owned);
            let id = owned.next;
            owned.next = owned
                .next
                .checked_add(1)
                .ok_or_else(|| FixtureError::new(StoreStatus::Exhausted))?;
            owned.handles.clear();
            id
        };
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

        let tickets = fixture(ticket_observations(&attachment))?;
        self.core.fence_issued.store(true, Ordering::Release);
        let proofs = std::mem::take(&mut lock(&attachment.fence_joins).proofs)
            .into_iter()
            .map(|(_, proof)| proof)
            .collect();
        Ok(QuiescedNativeSaveOwner {
            core: self.core.clone(),
            attachment,
            base: Box::new(snapshot),
            tickets,
            proofs,
            attempt: None,
        })
    }
}
fn ticket_observations(
    attachment: &SaveCore,
) -> Result<Vec<OriginalTicketObservation>, StoreStatus> {
    let originals = checked_lock(&attachment.originals)?;
    if originals.len() > 96 {
        return Err(StoreStatus::Internal);
    }
    originals
        .iter()
        .map(|o| {
            let state = checked_lock(&o.state)?;
            Ok(OriginalTicketObservation {
                original_request: encode_envelope_wire(&o.request)?,
                origin_id: o.view.origin_id,
                intent_id: o.view.intent_id.ok_or(StoreStatus::Internal)?,
                kind: if o.request.msg_type == 3 {
                    OriginalTicketKind::Allocate
                } else {
                    OriginalTicketKind::Commit
                },
                actor: o.grant.actor.clone(),
                object_id: o.grant.object,
                original_epoch: o.grant.epoch,
                allocation: state.allocation.clone(),
                binding: state.binding.clone(),
                closed: state.closed,
                commit_dispatched: state.commit_dispatched,
                completion: state.completion,
            })
        })
        .collect()
}
impl NativeRequest {
    pub(super) fn publish_allocation(
        &self,
        state: &mut ObjectState,
        operation: u64,
        wire: &Envelope,
    ) -> Result<(), StoreStatus> {
        let row = &state.journal.operations.as_slice()[record_index(state, operation)?];
        if row.allocation.actor != self.original.grant.actor {
            return Err(StoreStatus::Denied);
        }
        checked_lock(&self.original.state)?.allocation = Some(row.allocation.clone());
        let mut gate = self.gate()?;
        close_late_allocation(state, &self.original, &mut gate)?;
        let mut canonical = *wire;
        canonical.handle = Handle::INVALID;
        native(gate.publish_allocation(&self.original.claim, &canonical))
    }
    pub(super) fn publish_current(&self, state: &ObjectState) -> Result<(), StoreStatus> {
        let mut gate = self.gate()?;
        native(gate.publish_current_selection(&store_current(&self.original, state)))
    }
}

impl FixtureController {
    pub fn recovery_for_original(
        &self,
        chrome: &PeerContext,
        original: &OriginalNativeSave,
    ) -> Result<NativeSaveRecovery, StoreStatus> {
        let attachment = self.core.save_attachment()?;
        let owner = native(native(attachment.issuer.lock())?.preview_owner(chrome))?;
        let actor = &original.inner.grant.actor;
        if owner.owner_id != actor.owner_id
            || owner.session_id != actor.session_id
            || owner.session_generation != actor.session_generation
            || !lock(&attachment.originals)
                .iter()
                .any(|r| Arc::ptr_eq(r, &original.inner))
        {
            return Err(StoreStatus::Denied);
        }
        Ok(NativeSaveRecovery {
            original: original.inner.clone(),
            _core: self.core.clone(),
        })
    }
    pub fn pause_native_save(
        &self,
        endpoint: &EditorEndpoint,
        point: PausePoint,
    ) -> Result<PauseToken, FixtureError> {
        fixture(self.core.save_attachment())?;
        if !Arc::ptr_eq(&self.core, &endpoint.peer.core) {
            return Err(FixtureError::new(StoreStatus::Denied));
        }
        let object = fixture(self.core.object(endpoint.peer.grant.object))?;
        fixture(auth(&self.core, &endpoint.peer.grant, &lock(&object.state)))?;
        let mut barriers = lock(&self.core.barriers);
        if barriers.len() >= 64 {
            return Err(FixtureError::new(StoreStatus::Exhausted));
        }
        let unsettled = barriers
            .iter()
            .filter(|b| b.object == object.id && !lock(&b.state).snapshot.settled)
            .collect::<Vec<_>>();
        let pair = unsettled.len() == 1
            && point == PausePoint::AfterSelectionRename
            && unsettled[0].point == PausePoint::BeforeSelectionRename
            && unsettled[0].endpoint == endpoint.handle.pack()
            && !lock(&unsettled[0].state).snapshot.entered;
        if !unsettled.is_empty() && !pair {
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
    pub fn drop_next_native_reply(&self, endpoint: &EditorEndpoint) -> Result<(), FixtureError> {
        fixture(self.core.save_attachment())?;
        self.drop_next_reply(endpoint)
    }
    pub fn probe_inactive_native_save(
        &self,
        chrome: &PeerContext,
        pending: &PendingSaveInstance,
        pause: &NativeSavePause,
    ) -> Result<InactiveSaveStoreObservation, StoreStatus> {
        let attachment = self.core.save_attachment()?;
        let reservation = native(
            attachment
                .issuer
                .claim_inactive_save_probe(chrome, pending, pause),
        )?;
        let view = native(native(attachment.issuer.lock())?.view_pending(pending))?;
        let tag = checked_lock(&attachment.tags)?
            .values()
            .find(|tag| tag.grant.actor == view.actor)
            .cloned()
            .ok_or(StoreStatus::Denied)?;
        let object = self.core.object(tag.grant.object)?;
        let registry = checked_lock(&self.core.registry)?;
        let state = checked_lock(&object.state)?;
        let gate = native(attachment.issuer.lock())?;
        check_pending(&native(gate.view_pending(pending))?, &object, &state)?;
        if !registry
            .grants
            .get(&tag.grant.handle.pack())
            .is_some_and(|g| Arc::ptr_eq(g, &tag.grant))
        {
            return Err(StoreStatus::Denied);
        }
        let grant_status = native(gate.admit_installation(pending, &installation(&tag)));
        drop(gate);
        drop(state);
        drop(registry);
        drop(reservation);
        Ok(InactiveSaveStoreObservation { grant_status })
    }
}
pub struct InactiveSaveStoreObservation {
    pub grant_status: Result<(), StoreStatus>,
}
impl StoreFixture {
    pub fn corrupt_native_fenced(
        owner: &mut QuiescedNativeSaveOwner,
        object: u64,
        fault: StorageFault,
    ) -> Result<(), FixtureError> {
        let mut base = QuiescedStoreOwner {
            core: owner.core.clone(),
            snapshot: owner.base.clone(),
        };
        Self::corrupt_fenced(&mut base, object, fault)
    }
    pub fn reopen_native_save(
        mut owner: QuiescedNativeSaveOwner,
    ) -> Result<PreparedNativeSaveStore, NativeSaveReopenFailure> {
        let result = (|| {
            // Pure roster/epoch proof precedes every automatic metadata effect.
            let objects = owner
                .core
                .objects
                .values()
                .map(|object| {
                    let s = lock(&object.state);
                    Ok(StoreObjectEnrollment {
                        owner_id: s.journal.owner_id,
                        object_id: object.id,
                        object_generation: s.journal.selected_generation,
                        store_epoch: s
                            .journal
                            .service_epoch
                            .checked_add(1)
                            .ok_or(StoreStatus::Exhausted)?,
                    })
                })
                .collect::<Result<Vec<_>, StoreStatus>>()?;
            let enrolled_owners = native(
                native(owner.attachment.issuer.lock())?
                    .fenced_enrolled_owners(&objects, &owner.proofs),
            )?;
            if enrolled_owners.len() != objects.len() {
                return Err(StoreStatus::Denied);
            }
            let (host, controller) = if let Some(core) = &owner.attempt {
                (
                    StoreHost { core: core.clone() },
                    FixtureController { core: core.clone() },
                )
            } else {
                let base = QuiescedStoreOwner {
                    core: owner.core.clone(),
                    snapshot: owner.base.clone(),
                };
                let pair = Self::reopen_inner_mode(&base, true)?;
                owner.attempt = Some(pair.0.core.clone());
                pair
            };
            let current = host
                .core
                .objects
                .values()
                .zip(enrolled_owners)
                .map(|(object, enrolled_owner)| {
                    let state = lock(&object.state);
                    if enrolled_owner.owner_id != state.journal.owner_id {
                        return Err(StoreStatus::Denied);
                    }
                    current_selection(&host.core, object, &state, enrolled_owner)
                })
                .collect::<Result<Vec<_>, StoreStatus>>()?;
            native(
                native(owner.attachment.issuer.lock())?.reenroll_fenced_objects(
                    &objects,
                    &current,
                    &owner.proofs,
                ),
            )?;
            let attachment = Arc::new(SaveCore {
                witness: owner.attachment.witness.clone(),
                issuer: owner.attachment.issuer.clone(),
                tags: Mutex::new(BTreeMap::new()),
                producers: Mutex::new(vec![]),
                originals: Mutex::new(lock(&owner.attachment.originals).clone()),
                captures: Mutex::new(CaptureLedger {
                    next: 1,
                    reserved: 0,
                    rows: vec![],
                }),
                sources: Mutex::new(BTreeMap::new()),
                fence_joins: Mutex::new(FenceProofStore::default()),
                delivered_fence_joins: Mutex::new(BTreeSet::new()),
            });
            *lock(&host.core.native_save) = Some(attachment.clone());
            Ok(PreparedNativeSaveStore {
                host,
                controller,
                attachment,
            })
        })();
        match result {
            Ok(store) => Ok(store),
            Err(status) => Err(NativeSaveReopenFailure {
                error: FixtureError::new(status),
                owner,
            }),
        }
    }
}

// Retain only the genuine Desktop pair of an authenticated but unclaimed
// registration. No original ticket, Store dispatch or IO is manufactured.
fn retain_failed_entry_producers(
    core: &Arc<Core>,
    attachment: &Arc<SaveCore>,
    call: &Arc<NativeSaveCall>,
    object: u64,
    out: &mut Vec<ProducerId>,
) -> Result<(), StoreStatus> {
    let ids = native(call.entry_producers(&attachment.witness))?;
    let aliases = [
        native(attachment.witness.duplicate_producer(&ids[0]))?,
        native(attachment.witness.duplicate_producer(&ids[1]))?,
    ];
    let mut retained = checked_lock(&attachment.producers)?;
    retained.retain(|old| {
        !matches!(
            attachment.witness.duplicate_producer(old),
            Err(DesktopStatus::NotReady)
        )
    });
    let additional = ids
        .iter()
        .filter(|id| {
            !retained
                .iter()
                .any(|old| old.producer_id() == id.producer_id())
        })
        .count();
    if retained.len() + additional > 64 {
        return Err(StoreStatus::Exhausted);
    }
    let mut owned = checked_lock(&core.owned)?;
    let request = u64_at(&call.request().payload, 0);
    let operation = op_of(call.request());
    for (index, id) in ids.iter().enumerate() {
        if let Some(existing) = owned.handles.get(&id.producer_id()) {
            let same_kind = matches!(
                (index, &existing.kind),
                (0, ProducerKind::Dispatcher) | (1, ProducerKind::Supervisor)
            );
            if !same_kind
                || existing.object != object
                || existing.request != request
                || existing.operation != operation
            {
                return Err(StoreStatus::Denied);
            }
        }
    }
    // Publish the authentic scope metadata before making these aliases visible
    // to the Core fence enumerator. Neither private mutex spans an actual join.
    for (index, id) in ids.iter().enumerate() {
        owned
            .handles
            .entry(id.producer_id())
            .or_insert_with(|| Producer {
                kind: if index == 0 {
                    ProducerKind::Dispatcher
                } else {
                    ProducerKind::Supervisor
                },
                object,
                request,
                operation,
                handle: None,
                joining: false,
                terminal_join: None,
                thread_id: String::new(),
                barrier: None,
            });
    }
    for alias in aliases {
        if !retained
            .iter()
            .any(|old| old.producer_id() == alias.producer_id())
        {
            retained.push(alias);
        }
    }
    drop(owned);
    drop(retained);
    out.extend(ids);
    Ok(())
}

fn retain_entry_producers(
    core: &Arc<Core>,
    attachment: &Arc<SaveCore>,
    call: &Arc<NativeSaveCall>,
    original: &Arc<SaveOriginal>,
) -> Result<Vec<ProducerId>, StoreStatus> {
    let ids = match call.entry_producers(&attachment.witness) {
        Ok(ids) => ids,
        Err(DesktopStatus::NotReady) => return Ok(vec![]),
        Err(status) => return Err(desktop_status(status)),
    };
    let mut out = vec![];
    for (index, id) in ids.into_iter().enumerate() {
        install_metadata(
            core,
            &id,
            if index == 0 {
                ProducerKind::Dispatcher
            } else {
                ProducerKind::Supervisor
            },
            original,
            String::new(),
        );
        out.push(retain_producer(attachment, id)?);
    }
    Ok(out)
}

impl NativeRequest {
    pub(super) fn record_allocation(
        &self,
        state: &ObjectState,
        operation: u64,
    ) -> Result<(), StoreStatus> {
        let row = &state.journal.operations.as_slice()[record_index(state, operation)?];
        if row.allocation.actor != self.original.grant.actor {
            return Err(StoreStatus::Denied);
        }
        lock(&self.original.state).allocation = Some(row.allocation.clone());
        Ok(())
    }
}
