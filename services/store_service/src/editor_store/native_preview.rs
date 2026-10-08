//! Default-off Store-owned native preview Read1 adapter.
//! Opaque Desktop rows carry admission; Store owns the selected object and bytes.
//! Diagnostic records do not grant current authority or establish containment.
use super::*;
use desktop_service::dev::Status as DesktopStatus;
use desktop_service::editor_dev::{
    EnrollmentFailure, JoinedProducerProof, PeerContext, PendingStoreInstance, PreviewPause,
    PreviewPendingRetention, PreviewReadCall, PreviewReadCallView, ProducerId, RegistryWitness,
    SelectionIssuer, StoreActivationStamp, StoreCurrentSelection, StoreLaunch,
    StoreObjectEnrollment, StoreReadActivationView, StoreReadInstallation,
    StoreSelectionEnrollment, StoreSelectionPin,
};
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

pub(super) struct PreviewCore {
    witness: Arc<RegistryWitness>,
    issuer: SelectionIssuer,
    tags: Mutex<BTreeMap<u64, Arc<PreviewGrant>>>,
    producers: Mutex<Vec<ProducerId>>,
}
struct PreviewGrant {
    grant: Arc<Grant>,
    object_generation: u64,
    revision: u64,
    content_hash: Hash32,
}
pub struct PreparedPreviewStore {
    host: StoreHost,
    controller: FixtureController,
    attachment: Arc<PreviewCore>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewInitStage {
    Validate,
    PrepareCore,
    CreateRoot,
    Provision,
    Enroll,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewExposeStage {
    Validate,
    ReserveStoreGrant,
    PrepareRecord,
    Activate,
    Deliver,
}
pub struct InitObservation {
    pub stage: PreviewInitStage,
    pub status: StoreStatus,
    pub panicked: bool,
    pub root: ReadRootObservation,
    pub root_path: Option<PathBuf>,
    pub core_retained: bool,
    pub issuer_retained: bool,
}
pub struct ExposeObservation {
    pub stage: PreviewExposeStage,
    pub status: StoreStatus,
    pub pending_retained: bool,
    pub inactive_store_grant_retained: bool,
    pub active: bool,
}
struct InitOwner {
    witness: Option<RegistryWitness>,
    issuer: Option<SelectionIssuer>,
    _enrollment_failure: Option<EnrollmentFailure>,
    core: Option<Arc<Core>>,
    stage: PreviewInitStage,
    root: ReadRootObservation,
    root_path: Option<PathBuf>,
    panicked: bool,
}
pub struct NativePreviewInitFailure {
    status: StoreStatus,
    held: InitOwner,
}
impl NativePreviewInitFailure {
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
                    .is_some_and(|core| lock(&core.native_preview).is_some()),
        }
    }
}
pub struct NativePreviewExposeFailure {
    status: StoreStatus,
    stage: PreviewExposeStage,
    // These are real retained owners; observation flags cannot create them.
    _core: Arc<Core>,
    pending: Option<PreviewPendingRetention>,
    tag: Option<Arc<PreviewGrant>>,
}
impl NativePreviewExposeFailure {
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
pub struct NativePreviewActivation {
    core: Arc<Core>,
    attachment: Arc<PreviewCore>,
    tag: Arc<PreviewGrant>,
    stamp: StoreActivationStamp,
    endpoint: EditorEndpoint,
}
pub struct PairedStoreLaunch {
    desktop: StoreLaunch,
    endpoint: EditorEndpoint,
    _tag: Arc<PreviewGrant>,
}
impl PairedStoreLaunch {
    pub fn desktop(&self) -> &StoreLaunch {
        &self.desktop
    }
    pub fn endpoint(&self) -> &EditorEndpoint {
        &self.endpoint
    }
}

impl StoreFixture {
    pub fn prepare_preview_read(
        root: &Path,
        profile: FixtureProfile,
        witness: RegistryWitness,
        enrollment: StoreSelectionEnrollment,
    ) -> Result<PreparedPreviewStore, NativePreviewInitFailure> {
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
                let status = read_error(desktop_status(error.status()));
                held._enrollment_failure = Some(error);
                return Err(NativePreviewInitFailure { status, held });
            }
        }
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let issuer = held.issuer.as_ref().ok_or(StoreStatus::Internal)?;
            native(issuer.validate_witness(held.witness.as_ref().ok_or(StoreStatus::Internal)?))?;
            // Actual owner/session/object matching occurs in the opaque Gate,
            // before IO. These supplied records cannot mint a Desktop owner.
            if !(1..=2).contains(&profile.initial_objects.len()) {
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
            let core = Self::prepare_core(root, profile, true)?;
            held.core = Some(core.clone());
            let attachment = Arc::new(PreviewCore {
                witness: Arc::new(held.witness.take().ok_or(StoreStatus::Internal)?),
                issuer: held.issuer.take().ok_or(StoreStatus::Internal)?,
                tags: Mutex::new(BTreeMap::new()),
                producers: Mutex::new(vec![]),
            });
            *checked_lock(&core.native_preview)? = Some(attachment.clone());
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
                core.publish_preview_selection(object, &state)?;
            }
            Ok(PreparedPreviewStore {
                host: StoreHost { core: core.clone() },
                controller: FixtureController { core },
                attachment,
            })
        }));
        match outcome {
            Ok(Ok(prepared)) => Ok(prepared),
            Ok(Err(status)) => Err(NativePreviewInitFailure {
                status: read_error(status),
                held,
            }),
            Err(_) => {
                held.panicked = true;
                Err(NativePreviewInitFailure {
                    status: StoreStatus::Internal,
                    held,
                })
            }
        }
    }
}
impl Core {
    fn publish_preview_selection(
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
            || selected.revision != initial.revision
            || selected.content_hash != storage::hash(&state.bytes)
            || selected.content_hash != storage::hash(&initial.bytes)
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
    core.publish_preview_selection(object, state)?;
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
impl PreparedPreviewStore {
    pub fn host(&self) -> StoreHost {
        self.host.clone()
    }
    pub fn controller(&self) -> &FixtureController {
        &self.controller
    }
    pub fn pin_selection(
        &self,
        chrome: &PeerContext,
        ttl_ms: u64,
    ) -> Result<StoreSelectionPin, StoreStatus> {
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
                self.host
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
            chrome,
            current_selection(&self.host.core, &object, &state, checked_owner)?,
            ttl_ms,
            entered,
        ))
    }
    pub fn activate_read(
        &self,
        pending: &PendingStoreInstance,
    ) -> Result<NativePreviewActivation, NativePreviewExposeFailure> {
        let core = &self.host.core;
        let mut stage = PreviewExposeStage::Validate;
        let mut retained = None;
        let mut attempt = None;
        // Retain the genuine complete owner before any fallible own-lifecycle
        // validation or Store lock. This same-Authority private holder grants
        // no admission and remains meaningful even when its row is retired.
        let pending_owner = native(self.attachment.issuer.retain_pending_for_error(pending))
            .map_err(|status| NativePreviewExposeFailure {
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
            core.publish_preview_selection(&object, &state)?;
            check_pending(&view, &object, &state)?;
            stage = PreviewExposeStage::ReserveStoreGrant;
            if registry.grants.len() >= 64 || checked_lock(&self.attachment.tags)?.len() >= 64 {
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
                rights: 1,
                class: EndpointClass::Artifact,
                recovery_op: None,
                epoch: state.journal.service_epoch,
                expires: view.instance_expires_at_ms,
                alive: AtomicBool::new(true),
                invocation: AtomicBool::new(false),
                drop_reply: AtomicBool::new(false),
            });
            let tag = Arc::new(PreviewGrant {
                grant: grant.clone(),
                object_generation: state.journal.selected_generation,
                revision: state.journal.current.revision,
                content_hash: state.journal.current.content_hash,
            });
            registry.grants.insert(handle.pack(), grant.clone());
            checked_lock(&self.attachment.tags)?.insert(handle.pack(), tag.clone());
            retained = Some(tag.clone());
            stage = PreviewExposeStage::PrepareRecord;
            native(guard.stage_read_installation(pending, installation(&tag)))?;
            // No enforcing lock is retained while the supported setup actor
            // waits. The final transition repeats all Store/Gate checks.
            drop(guard);
            drop(state);
            drop(registry);
            attempt = Some(native(
                self.attachment
                    .issuer
                    .begin_read_activation_attempt(pending),
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
            core.publish_preview_selection(&object, &state)?;
            check_pending(&view, &object, &state)?;
            if !registry
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
            let stamp = native(guard.activate_read(pending, installation(&tag)))?;
            Ok(NativePreviewActivation {
                core: core.clone(),
                attachment: self.attachment.clone(),
                tag,
                stamp,
                endpoint,
            })
        })();
        let result = outcome.map_err(|status| NativePreviewExposeFailure {
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
fn installation(tag: &PreviewGrant) -> StoreReadInstallation {
    StoreReadInstallation {
        store_handle: tag.grant.handle,
        object_id: tag.grant.object,
        object_generation: tag.object_generation,
        store_epoch: tag.grant.epoch,
    }
}
fn check_pending(
    view: &desktop_service::editor_dev::PendingView,
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
    tag: &PreviewGrant,
    state: &ObjectState,
    view: &StoreReadActivationView,
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
        || state.journal.current.revision != tag.revision
        || state.journal.current.content_hash != tag.content_hash
    {
        return Err(StoreStatus::Denied);
    }
    Ok(())
}
impl NativePreviewActivation {
    pub fn stamp(&self) -> &StoreActivationStamp {
        &self.stamp
    }
    pub fn endpoint(&self) -> &EditorEndpoint {
        &self.endpoint
    }
    pub fn pair_launch(
        &self,
        launch: &mut Option<StoreLaunch>,
    ) -> Result<PairedStoreLaunch, NativePreviewExposeFailure> {
        let outcome = (|| {
            let held = launch.as_ref().ok_or(StoreStatus::NotReady)?;
            let registry = checked_lock(&self.core.registry)?;
            let object = self.core.object(self.tag.grant.object)?;
            let state = checked_lock(&object.state)?;
            let guard = native(self.attachment.issuer.lock())?;
            let view = native(guard.match_read_activation(&self.stamp, held.stamp()))?;
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
            Ok(PairedStoreLaunch {
                desktop: launch.take().ok_or(StoreStatus::NotReady)?,
                endpoint: EditorEndpoint {
                    peer: self.endpoint.peer.clone(),
                    handle: self.endpoint.handle,
                },
                _tag: self.tag.clone(),
            })
        })();
        outcome.map_err(|status| NativePreviewExposeFailure {
            status,
            stage: PreviewExposeStage::Deliver,
            _core: self.core.clone(),
            pending: None,
            tag: None,
        })
    }
}

struct PreviewOriginal {
    core: Arc<Core>,
    native: PreviewReadCall,
    grant: Arc<Grant>,
    tag: Arc<PreviewGrant>,
    attachment: Arc<PreviewCore>,
    claimed: AtomicBool,
}
pub struct RegisteredPreviewRead {
    inner: Arc<PreviewOriginal>,
}
pub struct PreviewReadReply {
    wire: Envelope,
    selected: Option<PreviewSelectedRead>,
    _original: Arc<PreviewOriginal>,
    producers: Vec<ProducerId>,
}
impl PreviewReadReply {
    pub fn wire(&self) -> Envelope {
        self.wire
    }
    pub fn selected(&self) -> Option<&PreviewSelectedRead> {
        self.selected.as_ref()
    }
    pub fn producer_ids(&self) -> &[ProducerId] {
        &self.producers
    }
}
pub struct PreviewSelectedRead {
    original: Arc<PreviewOriginal>,
    data: Arc<Data>,
    descriptor: ObjectDescriptor,
}
fn check_binding(
    original: &PreviewOriginal,
    state: &ObjectState,
    view: &PreviewReadCallView,
) -> Result<(), StoreStatus> {
    auth(&original.core, &original.grant, state)?;
    if state.preview_faulted {
        return Err(StoreStatus::Stale);
    }
    if !Arc::ptr_eq(&original.tag.grant, &original.grant)
        || original.grant.class != EndpointClass::Artifact
        || original.grant.rights != 1
        || original.grant.actor != view.actor
        || original.grant.object != view.object_id
        || original.grant.epoch != view.store_epoch
        || original.tag.object_generation != view.object_generation
        || state.journal.owner_id != view.actor.owner_id
        || state.journal.selected_generation != view.object_generation
        || state.journal.service_epoch != view.store_epoch
        || state.journal.current.revision != original.tag.revision
        || state.journal.current.content_hash != original.tag.content_hash
        || view.request.handle != original.grant.handle
        || encode_envelope_wire(&view.request)? != encode_envelope_wire(&original.native.request())?
        || view.entered != original.native.entered()
        || view.deadline != original.native.deadline()
    {
        return Err(StoreStatus::Denied);
    }
    Ok(())
}

pub struct InactiveStoreProbeObservation {
    pub grant_status: Result<(), StoreStatus>,
}
impl FixtureController {
    pub fn probe_inactive_preview_read(
        &self,
        chrome: &PeerContext,
        pending: &PendingStoreInstance,
        pause: &PreviewPause,
    ) -> Result<InactiveStoreProbeObservation, StoreStatus> {
        let attachment = StoreHost {
            core: self.core.clone(),
        }
        .preview_attachment()?;
        // The interlock is claimed without Store locks. It authenticates this
        // actual issuer, Chrome, pending and entered original setup pause.
        let reservation = native(
            attachment
                .issuer
                .claim_inactive_read_probe(chrome, pending, pause),
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
        let guard = native(attachment.issuer.lock())?;
        check_pending(&native(guard.view_pending(pending))?, &object, &state)?;
        if !registry
            .grants
            .get(&tag.grant.handle.pack())
            .is_some_and(|row| Arc::ptr_eq(row, &tag.grant))
        {
            return Err(StoreStatus::Denied);
        }
        check_tag(&self.core, &tag, &state)?;
        // This calls the same live private attachment admission enforcer used
        // by actual origin/read consumption, not a comparison with an expected
        // inactive flag or a fabricated PreviewReadCall/activation stamp.
        let grant_status = native(guard.admit_read_installation(pending, installation(&tag)));
        drop(guard);
        drop(state);
        drop(registry);
        drop(reservation);
        Ok(InactiveStoreProbeObservation { grant_status })
    }
    pub fn fault_preview_object(&self, object_id: u64) -> Result<(), FixtureError> {
        let attachment = fixture(
            StoreHost {
                core: self.core.clone(),
            }
            .preview_attachment(),
        )?;
        let object = fixture(self.core.object(object_id))?;
        let registry = fixture(checked_lock(&self.core.registry))?;
        let mut state = fixture(checked_lock(&object.state))?;
        if self.core.closed.load(Ordering::Acquire) || state.preview_faulted {
            return Err(FixtureError::new(StoreStatus::Stale));
        }
        // This irreversible real Object transition precedes all version work.
        // Exhausted publication cannot restore the object or its old aliases.
        state.preview_faulted = true;
        for grant in registry
            .grants
            .values()
            .filter(|grant| grant.object == object_id)
        {
            grant.alive.store(false, Ordering::Release);
        }
        for data in registry
            .data
            .values()
            .filter(|data| data.object == object_id)
        {
            data.alive.store(false, Ordering::Release);
        }
        let mut guard = fixture(native(attachment.issuer.lock()))?;
        fixture(native(guard.retire_source_object(object_id)))
    }
}
fn check_tag(core: &Arc<Core>, tag: &PreviewGrant, state: &ObjectState) -> Result<(), StoreStatus> {
    auth(core, &tag.grant, state)?;
    if state.preview_faulted {
        return Err(StoreStatus::Stale);
    }
    if tag.grant.class != EndpointClass::Artifact
        || tag.grant.rights != 1
        || tag.grant.actor.owner_id != state.journal.owner_id
        || tag.object_generation != state.journal.selected_generation
        || tag.grant.epoch != state.journal.service_epoch
        || tag.revision != state.journal.current.revision
        || tag.content_hash != state.journal.current.content_hash
    {
        return Err(StoreStatus::Denied);
    }
    Ok(())
}
impl StoreHost {
    fn preview_attachment(&self) -> Result<Arc<PreviewCore>, StoreStatus> {
        if !self.core.native_only {
            return Err(StoreStatus::Denied);
        }
        checked_lock(&self.core.native_preview)?
            .as_ref()
            .cloned()
            .ok_or(StoreStatus::Denied)
    }
    pub fn register_preview_read(
        &self,
        peer: &EditorPeer,
        call: PreviewReadCall,
    ) -> Result<RegisteredPreviewRead, StoreStatus> {
        let attachment = self.preview_attachment()?;
        if !Arc::ptr_eq(&self.core, &peer.core) {
            return Err(StoreStatus::Denied);
        }
        let tag = checked_lock(&attachment.tags)?
            .get(&peer.grant.handle.pack())
            .cloned()
            .ok_or(StoreStatus::Denied)?;
        if !Arc::ptr_eq(&tag.grant, &peer.grant) {
            return Err(StoreStatus::Denied);
        }
        let request = call.request();
        codec::validate(&request)?;
        if request.msg_type != 1 {
            return Err(StoreStatus::Unsupported);
        }
        if request.handle != peer.grant.handle {
            return Err(StoreStatus::Denied);
        }
        validate_actor(&peer.grant, &request)?;
        let registry = checked_lock(&self.core.registry)?;
        if !registry
            .grants
            .get(&peer.grant.handle.pack())
            .is_some_and(|grant| Arc::ptr_eq(grant, &peer.grant))
        {
            return Err(StoreStatus::Denied);
        }
        let original = Arc::new(PreviewOriginal {
            core: self.core.clone(),
            native: call,
            grant: peer.grant.clone(),
            tag,
            attachment,
            claimed: AtomicBool::new(false),
        });
        let object = self.core.object(original.grant.object)?;
        let state = checked_lock(&object.state)?;
        let guard = native(original.native.admit(&original.attachment.witness))?;
        check_binding(&original, &state, &guard.view())?;
        drop(guard);
        drop(state);
        drop(registry);
        Ok(RegisteredPreviewRead { inner: original })
    }
    pub fn close_preview_read_deadline(
        &self,
        registered: &RegisteredPreviewRead,
    ) -> Result<(), StoreStatus> {
        if !Arc::ptr_eq(&self.core, &registered.inner.core) {
            return Err(StoreStatus::Denied);
        }
        let object = self.core.object(registered.inner.grant.object)?;
        let state = checked_lock(&object.state)?;
        auth(&self.core, &registered.inner.grant, &state)?;
        if state.preview_faulted {
            return Err(StoreStatus::Stale);
        }
        let mut guard = native(
            registered
                .inner
                .native
                .deadline_guard(&registered.inner.attachment.witness),
        )?;
        native(guard.close_query())
    }
}

impl PreviewSelectedRead {
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
        let object = self.original.core.object(self.original.grant.object)?;
        let state = checked_lock(&object.state)?;
        auth(&self.original.core, &self.original.grant, &state)?;
        let guard = native(
            self.original
                .native
                .admit(&self.original.attachment.witness),
        )?;
        check_binding(&self.original, &state, &guard.view())?;
        if descriptor.kind != SharedObjectKind::SelectedText
            || descriptor.handle.kind != HandleKind::Shmem
            || !(1..=u16::MAX as u32).contains(&descriptor.handle.index)
            || !(1..=u32::MAX as u64).contains(&descriptor.handle.generation)
            || descriptor.object_generation != descriptor.handle.generation
            || descriptor.handle != self.data.handle
            || descriptor.object_generation != self.data.handle.generation
            || descriptor.byte_len != self.descriptor.byte_len
            || self.data.actor != self.original.grant.actor
            || self.data.object != object.id
            || self.data.kind != SharedObjectKind::SelectedText
        {
            return Err(StoreStatus::Denied);
        }
        if !self.data.alive.load(Ordering::Acquire) {
            return Err(StoreStatus::Stale);
        }
        let data = checked_lock(&self.data.state)?;
        if descriptor.byte_len as usize != data.bytes.len() {
            return Err(StoreStatus::Denied);
        }
        let header = schema(EditorTextHeaderV0::decode_le(
            data.bytes
                .get(..TEXT_HEADER_LEN)
                .ok_or(StoreStatus::Invalid)?,
        ))?;
        schema(header.validate_bytes(&data.bytes[TEXT_HEADER_LEN..]))?;
        if header.selected_object_id != object.id
            || header.revision != state.journal.current.revision
            || header.content_hash != state.journal.current.content_hash
        {
            return Err(StoreStatus::Denied);
        }
        let end = (offset as usize)
            .checked_add(out.len())
            .ok_or(StoreStatus::Invalid)?;
        let bytes = data
            .bytes
            .get(offset as usize..end)
            .ok_or(StoreStatus::Invalid)?;
        if Instant::now() >= self.original.native.deadline() {
            return Err(StoreStatus::Timeout);
        }
        out.copy_from_slice(bytes);
        Ok(())
    }
}

struct Completion {
    done: Mutex<bool>,
    changed: Condvar,
}
impl Completion {
    fn finish(&self) {
        *lock(&self.done) = true;
        self.changed.notify_all();
    }
}
struct NativeWorkerExit {
    completion: Arc<Completion>,
    barrier: Option<Arc<Barrier>>,
}
impl Drop for NativeWorkerExit {
    fn drop(&mut self) {
        if let Some(barrier) = &self.barrier {
            lock(&barrier.state).snapshot.settled = true;
            barrier.changed.notify_all();
        }
        self.completion.finish();
    }
}
fn native_barrier(original: &PreviewOriginal) -> Result<Option<Arc<Barrier>>, StoreStatus> {
    let barriers = checked_lock(&original.core.barriers)?;
    Ok(barriers
        .iter()
        .find(|barrier| {
            barrier.object == original.grant.object
                && barrier.endpoint == original.grant.handle.pack()
                && barrier.point == PausePoint::BeforeReadReply
                && !lock(&barrier.state).snapshot.entered
        })
        .cloned())
}
fn pause_read(
    original: &PreviewOriginal,
    barrier: &Option<Arc<Barrier>>,
) -> Result<(), StoreStatus> {
    if let Some(barrier) = barrier {
        let mut state = checked_lock(&barrier.state)?;
        if state.snapshot.entered {
            return Err(StoreStatus::NotReady);
        }
        state.snapshot.request_id = u64_at(&original.native.request().payload, 0);
        state.snapshot.operation_id = 0;
        state.snapshot.permit_issued = false;
        state.snapshot.entered = true;
        barrier.changed.notify_all();
        while !state.snapshot.released && !state.stop {
            state = barrier
                .changed
                .wait(state)
                .map_err(|_| StoreStatus::Internal)?;
        }
        if state.stop {
            return Err(StoreStatus::Internal);
        }
    }
    Ok(())
}
fn live_original(original: &PreviewOriginal) -> Result<(), StoreStatus> {
    let object = original.core.object(original.grant.object)?;
    let state = checked_lock(&object.state)?;
    let guard = native(original.native.admit(&original.attachment.witness))?;
    check_binding(original, &state, &guard.view())
}
fn read_selected(
    original: &Arc<PreviewOriginal>,
) -> Result<(Envelope, PreviewSelectedRead), StoreStatus> {
    let object = original.core.object(original.grant.object)?;
    let mut registry = checked_lock(&original.core.registry)?;
    let state = checked_lock(&object.state)?;
    auth(&original.core, &original.grant, &state)?;
    let guard = native(original.native.admit(&original.attachment.witness))?;
    check_binding(original, &state, &guard.view())?;
    let selected = &state.journal.current;
    let header = schema(EditorTextHeaderV0::try_new(
        object.id,
        selected.revision,
        &state.bytes,
    ))?;
    if header.content_hash != selected.content_hash {
        return Err(StoreStatus::Internal);
    }
    let mut encoded = schema(header.encode_le())?.to_vec();
    encoded.extend_from_slice(&state.bytes);
    let cached = registry
        .data
        .values()
        .find(|data| {
            data.actor == original.grant.actor
                && data.object == object.id
                && data.revision == selected.revision
                && data.kind == SharedObjectKind::SelectedText
                && data.alive.load(Ordering::Acquire)
        })
        .cloned();
    if Instant::now() >= original.native.deadline() {
        return Err(StoreStatus::Timeout);
    }
    let data = if let Some(data) = cached {
        if checked_lock(&data.state)?.bytes != encoded {
            return Err(StoreStatus::Internal);
        }
        data
    } else {
        if registry.data.len() >= 32 {
            return Err(StoreStatus::Exhausted);
        }
        let handle = Core::new_handle(&mut registry, HandleKind::Shmem)?;
        let data = Arc::new(Data {
            handle,
            actor: original.grant.actor.clone(),
            object: object.id,
            operation: None,
            revision: selected.revision,
            kind: SharedObjectKind::SelectedText,
            alive: AtomicBool::new(true),
            state: Mutex::new(DataState {
                bytes: encoded,
                writable: false,
                frozen: true,
            }),
        });
        registry.data.insert(handle.pack(), data.clone());
        data
    };
    let descriptor = ObjectDescriptor {
        handle: data.handle,
        kind: SharedObjectKind::SelectedText,
        object_generation: data.handle.generation,
        byte_len: header.total_len,
    };
    let wire = a::ReadSelectedReply {
        request_id: u64_at(&original.native.request().payload, 0),
        revision: selected.revision,
        data_shm: descriptor.handle.pack(),
        object_generation: descriptor.object_generation,
        byte_len: descriptor.byte_len,
        status: 0,
    }
    .encode(Handle::INVALID)?;
    Ok((
        wire,
        PreviewSelectedRead {
            original: original.clone(),
            data,
            descriptor,
        },
    ))
}
fn query_timeout(original: &PreviewOriginal) -> StoreStatus {
    // This guard closes this one original query, never an instance, another
    // query, or any historical Store operation. The Instant is not reset.
    let result = original
        .native
        .deadline_guard(&original.attachment.witness)
        .and_then(|mut guard| guard.close_query());
    match result {
        Ok(()) => StoreStatus::Timeout,
        Err(error) => desktop_status(error),
    }
}
fn read_error(status: StoreStatus) -> StoreStatus {
    match status {
        StoreStatus::Unknown | StoreStatus::Conflict | StoreStatus::Disconnected => {
            StoreStatus::Internal
        }
        other => other,
    }
}
fn request_for(original: &PreviewOriginal) -> Envelope {
    // Registration binds this original request to the actual Store endpoint;
    // response construction never substitutes a handle from another service.
    original.native.request()
}
fn native_redacted_reply(original: &PreviewOriginal, status: StoreStatus) -> Envelope {
    // Native replies carry no endpoint authority. The legacy builder remains
    // unchanged for its existing consumers; this Read1 reply retains only the
    // original request ID and mapped status in an otherwise empty payload.
    let mut reply = empty_reply(&request_for(original), read_error(status));
    reply.handle = Handle::INVALID;
    reply
}
fn error_reply(
    original: &Arc<PreviewOriginal>,
    status: StoreStatus,
    producers: Vec<ProducerId>,
) -> PreviewReadReply {
    PreviewReadReply {
        wire: native_redacted_reply(original, status),
        selected: None,
        _original: original.clone(),
        producers,
    }
}

// A returned duplicate is a reference to this SAME installed row, not a new
// charge or an invented ID. On alias failure retain the original actual token.
fn retain_installed(attachment: &PreviewCore, id: ProducerId) -> Result<ProducerId, StoreStatus> {
    let duplicate = native(attachment.witness.duplicate_producer(&id));
    let mut retained = lock(&attachment.producers);
    // This collection never gains more rows than the shared64 actual roster.
    // Prune only actual retired NotReady; Internal must retain ownership.
    retained.retain(|id| {
        !matches!(
            attachment.witness.duplicate_producer(id),
            Err(DesktopStatus::NotReady)
        )
    });
    match duplicate {
        Ok(reference) => {
            retained.push(reference);
            Ok(id)
        }
        Err(status) => {
            retained.push(id);
            Err(status)
        }
    }
}
fn reference_room(attachment: &PreviewCore) -> Result<(), StoreStatus> {
    let mut retained = checked_lock(&attachment.producers)?;
    let mut failed = None;
    retained.retain(|id| match attachment.witness.duplicate_producer(id) {
        Ok(_) => true,
        Err(DesktopStatus::NotReady) => false,
        Err(status) => {
            failed = Some(desktop_status(status));
            true
        }
    });
    if let Some(status) = failed {
        return Err(status);
    }
    if retained.len() > 62 {
        return Err(StoreStatus::Exhausted);
    }
    Ok(())
}
impl StoreHost {
    pub fn execute_preview_read(
        &self,
        registered: &RegisteredPreviewRead,
    ) -> Result<PreviewReadReply, StoreStatus> {
        let original = &registered.inner;
        if !Arc::ptr_eq(&self.core, &original.core) {
            return Err(StoreStatus::Denied);
        }
        let attachment = self.preview_attachment()?;
        if original.claimed.swap(true, Ordering::AcqRel) {
            return Ok(error_reply(original, StoreStatus::NotReady, vec![]));
        }
        let _trace = match self.core.reserve_exchange() {
            Ok(reservation) => reservation,
            Err(status) => return Ok(error_reply(original, status, vec![])),
        };
        let start = original.native.entered();
        let mut reply = match live_original(original).and_then(|()| reference_room(&attachment)) {
            Ok(()) => self.run_preview_read(original, &attachment),
            Err(status) => error_reply(original, status, vec![]),
        };
        if let Err(status) = self.core.append_exchange(
            &original.grant,
            &request_for(original),
            &reply.wire,
            false,
            start,
        ) {
            reply.wire = native_redacted_reply(original, status);
            reply.selected = None;
        }
        Ok(reply)
    }
    fn run_preview_read(
        &self,
        original: &Arc<PreviewOriginal>,
        attachment: &Arc<PreviewCore>,
    ) -> PreviewReadReply {
        use desktop_service::editor_dev::ProducerKind as NativeKind;
        // No Object/Gate guard survives into delegation/reservation or waits.
        let dispatcher = match native(
            original
                .native
                .reserve_store_producer(NativeKind::StoreDispatch),
        ) {
            Ok(reservation) => reservation,
            Err(status) => return error_reply(original, status, vec![]),
        };
        let supervisor = match native(
            original
                .native
                .reserve_store_producer(NativeKind::StoreSupervisor),
        ) {
            Ok(reservation) => reservation,
            Err(status) => {
                let _ = dispatcher.cancel_unspawned();
                return error_reply(original, status, vec![]);
            }
        };
        let barrier = match native_barrier(original) {
            Ok(barrier) => barrier,
            Err(status) => {
                let _ = dispatcher.cancel_unspawned();
                let _ = supervisor.cancel_unspawned();
                return error_reply(original, status, vec![]);
            }
        };
        let completion = Arc::new(Completion {
            done: Mutex::new(false),
            changed: Condvar::new(),
        });
        let abort = Arc::new(AtomicBool::new(false));
        let (start_tx, start_rx) = mpsc::channel();
        let (reply_tx, reply_rx) = mpsc::channel();
        let worker_original = original.clone();
        let worker_completion = completion.clone();
        let worker_abort = abort.clone();
        let handle = thread::Builder::new()
            .name("store-native-read".into())
            .spawn(move || {
                let _exit = NativeWorkerExit {
                    completion: worker_completion,
                    barrier: barrier.clone(),
                };
                if start_rx.recv().is_err() || worker_abort.load(Ordering::Acquire) {
                    return;
                }
                let result = pause_read(&worker_original, &barrier)
                    .and_then(|()| read_selected(&worker_original));
                let _ = reply_tx.send(result);
            });
        let handle = match handle {
            Ok(handle) => handle,
            Err(_) => {
                let _ = dispatcher.cancel_unspawned();
                let _ = supervisor.cancel_unspawned();
                return error_reply(original, StoreStatus::Internal, vec![]);
            }
        };
        let dispatcher = dispatcher.install(handle);
        let dispatcher = match retain_installed(attachment, dispatcher) {
            Ok(id) => id,
            Err(status) => {
                let _ = supervisor.cancel_unspawned();
                abort.store(true, Ordering::Release);
                let _ = start_tx.send(());
                return error_reply(original, status, vec![]);
            }
        };
        let watchdog_original = original.clone();
        let watchdog_completion = completion.clone();
        let (watch_start_tx, watch_start_rx) = mpsc::channel();
        let handle = thread::Builder::new()
            .name("store-native-read-deadline".into())
            .spawn(move || {
                if watch_start_rx.recv().is_err() {
                    return;
                }
                let mut done = lock(&watchdog_completion.done);
                while !*done {
                    let left = watchdog_original
                        .native
                        .deadline()
                        .saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        drop(done);
                        let _ = query_timeout(&watchdog_original);
                        return;
                    }
                    let (state, _) = watchdog_completion
                        .changed
                        .wait_timeout(done, left)
                        .unwrap_or_else(|p| p.into_inner());
                    done = state;
                }
            });
        let handle = match handle {
            Ok(handle) => handle,
            Err(_) => {
                let _ = supervisor.cancel_unspawned();
                abort.store(true, Ordering::Release);
                let _ = start_tx.send(());
                return error_reply(original, StoreStatus::Internal, vec![dispatcher]);
            }
        };
        let supervisor = supervisor.install(handle);
        let supervisor = match retain_installed(attachment, supervisor) {
            Ok(id) => id,
            Err(status) => {
                abort.store(true, Ordering::Release);
                let _ = watch_start_tx.send(());
                let _ = start_tx.send(());
                return error_reply(original, status, vec![dispatcher]);
            }
        };
        let producers = vec![dispatcher, supervisor];
        // Both actual JoinHandles are installed/retained before barriers, data
        // allocation/copy work, or any deadline wait can begin.
        let _ = watch_start_tx.send(());
        let _ = start_tx.send(());
        let result = reply_rx.recv_timeout(
            original
                .native
                .deadline()
                .saturating_duration_since(Instant::now()),
        );
        match result {
            Ok(Ok((wire, selected))) => match live_original(original) {
                Ok(()) => PreviewReadReply {
                    wire,
                    selected: Some(selected),
                    _original: original.clone(),
                    producers,
                },
                Err(status) => error_reply(original, status, producers),
            },
            Ok(Err(status)) => error_reply(original, status, producers),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                error_reply(original, query_timeout(original), producers)
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                error_reply(original, StoreStatus::Internal, producers)
            }
        }
    }
    pub fn join_preview_read_finished(
        &self,
        id: &ProducerId,
    ) -> Result<JoinedProducerProof, StoreStatus> {
        let attachment = self.preview_attachment()?;
        // This is shared domain cleanup authority. It does not identify which
        // service spawned a row; that provenance comes only from actual private
        // reservations/installations and our own retained-token getter.
        native(attachment.witness.join_finished(id))
    }
    pub fn preview_read_producers(&self) -> Vec<ProducerId> {
        let Ok(attachment) = self.preview_attachment() else {
            return vec![];
        };
        let mut retained = lock(&attachment.producers);
        let mut result = vec![];
        retained.retain(|id| match attachment.witness.duplicate_producer(id) {
            Ok(alias) => {
                result.push(alias);
                true
            }
            Err(DesktopStatus::NotReady) => false,
            Err(_) => true,
        });
        result
    }
}
