//! Default-off native READ adapter. Concrete Desktop origins carry the original
//! deadline; Store owns object/grant/data enforcement and actual service work.
use super::*;
use desktop_service::dev::Status as DesktopStatus;
use desktop_service::editor_dev::{
    ApprovedReadBinding, JoinedProducerProof, NativeReadCall, ProducerCounts, ProducerId,
    ReadBindingView, ReadCallView, RegistryWitness,
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
        DesktopStatus::Conflict => StoreStatus::Conflict,
        DesktopStatus::Timeout => StoreStatus::Timeout,
        DesktopStatus::Disconnected => StoreStatus::Disconnected,
        DesktopStatus::Unknown => StoreStatus::Unknown,
        DesktopStatus::Internal => StoreStatus::Internal,
    }
}
fn native<T>(result: Result<T, DesktopStatus>) -> Result<T, StoreStatus> {
    result.map_err(desktop_status)
}
fn checked_lock<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, StoreStatus> {
    mutex.lock().map_err(|_| StoreStatus::Internal)
}

pub(super) struct NativeCore {
    witness: Arc<RegistryWitness>,
    bindings: Vec<Arc<NativeReadGrant>>,
    // Only IDs returned from this adapter's actual private spawned handles.
    // References to joined rows are removed before the bounded roster grows.
    producers: Mutex<Vec<ProducerId>>,
}
pub(super) struct NativeReadGrant {
    witness: Arc<RegistryWitness>,
    binding: ApprovedReadBinding,
}
pub struct NativeReadFixture {
    host: StoreHost,
    controller: FixtureController,
    endpoints: Vec<EditorEndpoint>,
}
impl NativeReadFixture {
    pub fn host(&self) -> StoreHost {
        self.host.clone()
    }
    pub fn controller(&self) -> &FixtureController {
        &self.controller
    }
    pub fn endpoint(&self, index: usize) -> Result<&EditorEndpoint, StoreStatus> {
        self.endpoints.get(index).ok_or(StoreStatus::Denied)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadInitStage {
    Validate,
    PrepareCore,
    CreateRoot,
    Provision,
    Expose,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadRootObservation {
    NotAttempted,
    AttemptUncertain,
    ReportedCreated,
}
pub struct ReadInitObservation {
    pub stage: ReadInitStage,
    pub status: StoreStatus,
    pub panicked: bool,
    pub root: ReadRootObservation,
    pub root_path: Option<PathBuf>,
    pub core_retained: bool,
}
struct ReadInitOwner {
    witness: Option<RegistryWitness>,
    bindings: Vec<ApprovedReadBinding>,
    core: Option<Arc<Core>>,
    stage: ReadInitStage,
    root: ReadRootObservation,
    root_path: Option<PathBuf>,
    panicked: bool,
}
pub struct NativeReadInitFailure {
    status: StoreStatus,
    held: ReadInitOwner,
}
impl NativeReadInitFailure {
    pub fn observation(&self) -> ReadInitObservation {
        ReadInitObservation {
            stage: self.held.stage,
            status: self.status,
            panicked: self.held.panicked,
            root: self.held.root,
            root_path: self.held.root_path.clone(),
            core_retained: self.held.core.is_some(),
        }
    }
}

fn profile_binding(profile: &FixtureProfile, view: &ReadBindingView) -> Result<(), StoreStatus> {
    let initial = profile
        .initial_objects
        .iter()
        .find(|initial| initial.object_id == view.object_id)
        .ok_or(StoreStatus::Denied)?;
    if initial.owner_id != view.actor.owner_id
        || initial.generation != view.object_generation
        || initial.revision != view.selected_revision
        || storage::hash(&initial.bytes) != view.selected_hash
    {
        return Err(StoreStatus::Denied);
    }
    Ok(())
}
fn validate_bindings(owner: &ReadInitOwner, profile: &FixtureProfile) -> Result<(), StoreStatus> {
    if !(1..=2).contains(&owner.bindings.len())
        || owner.bindings.len() != profile.initial_objects.len()
    {
        return Err(StoreStatus::Denied);
    }
    let witness = owner.witness.as_ref().ok_or(StoreStatus::Internal)?;
    let guard = native(owner.bindings[0].admit(witness))?;
    let mut actors = vec![];
    let mut objects = vec![];
    for binding in &owner.bindings {
        let view = native(guard.view_binding(binding))?;
        profile_binding(profile, &view)?;
        if actors.contains(&view.actor) || objects.contains(&view.object_id) {
            return Err(StoreStatus::Denied);
        }
        actors.push(view.actor);
        objects.push(view.object_id);
    }
    Ok(())
}
impl StoreFixture {
    pub fn create_native_read(
        root: &Path,
        profile: FixtureProfile,
        witness: RegistryWitness,
        bindings: Vec<ApprovedReadBinding>,
    ) -> Result<NativeReadFixture, NativeReadInitFailure> {
        // The retained holder exists outside the unwind boundary. No constructor
        // cleanup deletes a partial/preexisting root or reopens another owner.
        let mut held = ReadInitOwner {
            witness: Some(witness),
            bindings,
            core: None,
            stage: ReadInitStage::Validate,
            root: ReadRootObservation::NotAttempted,
            root_path: if root.as_os_str().len() <= 4096 {
                Some(root.to_path_buf())
            } else {
                None
            },
            panicked: false,
        };
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            validate_bindings(&held, &profile)?;
            held.stage = ReadInitStage::PrepareCore;
            let core = Self::prepare_core(root, profile, true)?;
            held.core = Some(core.clone());
            let witness = Arc::new(held.witness.take().ok_or(StoreStatus::Internal)?);
            let bindings = held
                .bindings
                .drain(..)
                .map(|binding| {
                    Arc::new(NativeReadGrant {
                        witness: witness.clone(),
                        binding,
                    })
                })
                .collect::<Vec<_>>();
            let attachment = Arc::new(NativeCore {
                witness,
                bindings,
                producers: Mutex::new(vec![]),
            });
            *checked_lock(&core.native_read)? = Some(attachment.clone());
            // The concrete Core, witness and opaque binding rows are now all
            // retained before the first filesystem action, including panic.
            held.stage = ReadInitStage::CreateRoot;
            held.root = ReadRootObservation::AttemptUncertain;
            fs::DirBuilder::new()
                .mode(0o700)
                .create(&core.root)
                .map_err(|_| StoreStatus::Internal)?;
            held.root = ReadRootObservation::ReportedCreated;
            held.stage = ReadInitStage::Provision;
            Self::provision_objects(&core)?;
            held.stage = ReadInitStage::Expose;
            let endpoints = expose_native_grants(&core, &attachment)?;
            Ok(NativeReadFixture {
                host: StoreHost { core: core.clone() },
                controller: FixtureController { core },
                endpoints,
            })
        }));
        match outcome {
            Ok(Ok(fixture)) => Ok(fixture),
            Ok(Err(status)) => Err(NativeReadInitFailure {
                status: read_error(status),
                held,
            }),
            Err(_) => {
                held.panicked = true;
                Err(NativeReadInitFailure {
                    status: StoreStatus::Internal,
                    held,
                })
            }
        }
    }
}
fn expose_native_grants(
    core: &Arc<Core>,
    attachment: &Arc<NativeCore>,
) -> Result<Vec<EditorEndpoint>, StoreStatus> {
    let mut registry = checked_lock(&core.registry)?;
    let objects = core.objects.values().collect::<Vec<_>>();
    let states = objects
        .iter()
        .map(|object| checked_lock(&object.state))
        .collect::<Result<Vec<_>, _>>()?;
    // Fresh unexposed owner: Registry -> sorted Objects -> SAME common Gate.
    // No Gate -> Object inversion and no IO while this final guard is held.
    let guard = native(attachment.bindings[0].binding.admit(&attachment.witness))?;
    let mut pending = vec![];
    for binding in &attachment.bindings {
        let view = native(guard.view_binding(&binding.binding))?;
        profile_binding(&core.profile, &view)?;
        let index = objects
            .iter()
            .position(|object| object.id == view.object_id)
            .ok_or(StoreStatus::Denied)?;
        let state = &states[index];
        if state.journal.owner_id != view.actor.owner_id
            || state.journal.selected_generation != view.object_generation
            || state.journal.current.revision != view.selected_revision
            || state.journal.current.content_hash != view.selected_hash
        {
            return Err(StoreStatus::Denied);
        }
        let expires = core
            .clock
            .load(Ordering::Acquire)
            .checked_add(600_000)
            .ok_or(StoreStatus::Exhausted)?;
        let handle = Core::new_handle(&mut registry, HandleKind::Ipc)?;
        pending.push(Arc::new(Grant {
            native_read: Some(binding.clone()),
            handle,
            actor: view.actor,
            object: view.object_id,
            rights: 1,
            class: EndpointClass::Artifact,
            recovery_op: None,
            epoch: state.journal.service_epoch,
            expires,
            alive: AtomicBool::new(true),
            invocation: AtomicBool::new(false),
            drop_reply: AtomicBool::new(false),
        }));
    }
    let mut endpoints = vec![];
    for grant in pending {
        registry.sessions.insert(
            (grant.actor.session_id, grant.actor.session_generation),
            grant.actor.owner_id,
        );
        registry
            .instances
            .insert(instance_key(&grant.actor), grant.actor.owner_id);
        registry.grants.insert(grant.handle.pack(), grant.clone());
        endpoints.push(EditorEndpoint {
            handle: grant.handle,
            peer: EditorPeer {
                core: core.clone(),
                grant,
            },
        });
    }
    Ok(endpoints)
}

struct ReadOriginal {
    core: Arc<Core>,
    native: NativeReadCall,
    grant: Arc<Grant>,
    binding: Arc<NativeReadGrant>,
    claimed: AtomicBool,
}
pub struct RegisteredRead {
    inner: Arc<ReadOriginal>,
}
pub struct NativeReadReply {
    wire: Envelope,
    selected: Option<NativeSelectedRead>,
    _original: Arc<ReadOriginal>,
    producers: Vec<ProducerId>,
}
impl NativeReadReply {
    pub fn wire(&self) -> Envelope {
        self.wire
    }
    pub fn selected(&self) -> Option<&NativeSelectedRead> {
        self.selected.as_ref()
    }
    pub fn producer_ids(&self) -> &[ProducerId] {
        &self.producers
    }
}
pub(super) struct NativeReadSelectedRead {
    original: Arc<ReadOriginal>,
    data: Arc<Data>,
    descriptor: ObjectDescriptor,
}
fn check_binding(
    original: &ReadOriginal,
    state: &ObjectState,
    view: &ReadCallView,
) -> Result<(), StoreStatus> {
    auth(&original.core, &original.grant, state)?;
    if original.grant.rights != 1
        || original.grant.class != EndpointClass::Artifact
        || original.grant.actor != view.binding.actor
        || original.grant.object != view.binding.object_id
        || state.journal.owner_id != view.binding.actor.owner_id
        || state.journal.selected_generation != view.binding.object_generation
        || state.journal.current.revision != view.binding.selected_revision
        || state.journal.current.content_hash != view.binding.selected_hash
    {
        return Err(StoreStatus::Denied);
    }
    Ok(())
}
impl StoreHost {
    fn native_attachment(&self) -> Result<Arc<NativeCore>, StoreStatus> {
        if !self.core.native_only {
            return Err(StoreStatus::Denied);
        }
        checked_lock(&self.core.native_read)?
            .as_ref()
            .cloned()
            .ok_or(StoreStatus::Denied)
    }
    pub fn register_native_read(
        &self,
        peer: &EditorPeer,
        call: NativeReadCall,
    ) -> Result<RegisteredRead, StoreStatus> {
        let attachment = self.native_attachment()?;
        if !Arc::ptr_eq(&self.core, &peer.core) {
            return Err(StoreStatus::Denied);
        }
        let binding = peer
            .grant
            .native_read
            .as_ref()
            .ok_or(StoreStatus::Denied)?
            .clone();
        if !Arc::ptr_eq(&binding.witness, &attachment.witness)
            || !attachment.bindings.iter().any(|b| Arc::ptr_eq(b, &binding))
            || !checked_lock(&self.core.registry)?
                .grants
                .get(&peer.grant.handle.pack())
                .is_some_and(|grant| Arc::ptr_eq(grant, &peer.grant))
        {
            return Err(StoreStatus::Denied);
        }
        let mut request = call.request();
        codec::validate(&request)?;
        if request.msg_type != 1 {
            return Err(StoreStatus::Unsupported);
        }
        request.handle = peer.grant.handle;
        validate_actor(&peer.grant, &request)?;
        let original = Arc::new(ReadOriginal {
            core: self.core.clone(),
            native: call,
            grant: peer.grant.clone(),
            binding,
            claimed: AtomicBool::new(false),
        });
        let object = self.core.object(original.grant.object)?;
        let state = checked_lock(&object.state)?;
        let guard = native(original.native.admit(&attachment.witness))?;
        check_binding(&original, &state, &guard.view())?;
        drop(guard);
        drop(state);
        Ok(RegisteredRead { inner: original })
    }
    pub fn close_native_read_deadline(
        &self,
        registered: &RegisteredRead,
    ) -> Result<(), StoreStatus> {
        if !Arc::ptr_eq(&self.core, &registered.inner.core) {
            return Err(StoreStatus::Denied);
        }
        let object = self.core.object(registered.inner.grant.object)?;
        let state = checked_lock(&object.state)?;
        auth(&self.core, &registered.inner.grant, &state)?;
        let mut guard = native(
            registered
                .inner
                .native
                .deadline_guard(&registered.inner.binding.witness),
        )?;
        native(guard.close_query())
    }
}
impl NativeReadSelectedRead {
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
        let guard = native(self.original.native.admit(&self.original.binding.witness))?;
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
fn native_barrier(original: &ReadOriginal) -> Result<Option<Arc<Barrier>>, StoreStatus> {
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
fn pause_read(original: &ReadOriginal, barrier: &Option<Arc<Barrier>>) -> Result<(), StoreStatus> {
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
fn live_original(original: &ReadOriginal) -> Result<(), StoreStatus> {
    let object = original.core.object(original.grant.object)?;
    let state = checked_lock(&object.state)?;
    let guard = native(original.native.admit(&original.binding.witness))?;
    check_binding(original, &state, &guard.view())
}
fn read_selected(
    original: &Arc<ReadOriginal>,
) -> Result<(Envelope, NativeSelectedRead), StoreStatus> {
    let object = original.core.object(original.grant.object)?;
    let mut registry = checked_lock(&original.core.registry)?;
    let state = checked_lock(&object.state)?;
    auth(&original.core, &original.grant, &state)?;
    let guard = native(original.native.admit(&original.binding.witness))?;
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
    .encode(original.grant.handle)?;
    Ok((
        wire,
        NativeSelectedRead::from_read(NativeReadSelectedRead {
            original: original.clone(),
            data,
            descriptor,
        }),
    ))
}
fn query_timeout(original: &ReadOriginal) -> StoreStatus {
    // This guard closes this one original query, never an instance, another
    // query, or any historical Store operation. The Instant is not reset.
    let result = original
        .native
        .deadline_guard(&original.binding.witness)
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
fn request_for(original: &ReadOriginal) -> Envelope {
    let mut request = original.native.request();
    request.handle = original.grant.handle;
    request
}
fn error_reply(
    original: &Arc<ReadOriginal>,
    status: StoreStatus,
    producers: Vec<ProducerId>,
) -> NativeReadReply {
    NativeReadReply {
        wire: empty_reply(&request_for(original), read_error(status)),
        selected: None,
        _original: original.clone(),
        producers,
    }
}

// A returned duplicate is a reference to this SAME installed row, not a new
// charge or an invented ID. On alias failure retain the original actual token.
fn retain_installed(attachment: &NativeCore, id: ProducerId) -> Result<ProducerId, StoreStatus> {
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
fn reference_room(attachment: &NativeCore) -> Result<(), StoreStatus> {
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
    pub fn execute_native_read(
        &self,
        registered: &RegisteredRead,
    ) -> Result<NativeReadReply, StoreStatus> {
        let original = &registered.inner;
        if !Arc::ptr_eq(&self.core, &original.core) {
            return Err(StoreStatus::Denied);
        }
        let attachment = self.native_attachment()?;
        if original.claimed.swap(true, Ordering::AcqRel) {
            return Ok(error_reply(original, StoreStatus::NotReady, vec![]));
        }
        let _trace = match self.core.reserve_exchange() {
            Ok(reservation) => reservation,
            Err(status) => return Ok(error_reply(original, status, vec![])),
        };
        let start = original.native.entered();
        let mut reply = match live_original(original).and_then(|()| reference_room(&attachment)) {
            Ok(()) => self.run_native_read(original, &attachment),
            Err(status) => error_reply(original, status, vec![]),
        };
        if let Err(status) = self.core.append_exchange(
            &original.grant,
            &request_for(original),
            &reply.wire,
            false,
            start,
        ) {
            reply.wire = empty_reply(&request_for(original), read_error(status));
            reply.selected = None;
        }
        Ok(reply)
    }
    fn run_native_read(
        &self,
        original: &Arc<ReadOriginal>,
        attachment: &Arc<NativeCore>,
    ) -> NativeReadReply {
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
                Ok(()) => NativeReadReply {
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
    pub fn native_read_producer_counts(&self) -> ProducerCounts {
        self.native_attachment()
            .map(|a| a.witness.producer_counts())
            .unwrap_or(ProducerCounts {
                #[cfg(feature = "editor_native_save_v0_dev")]
                active_non_io: 0,
                #[cfg(feature = "editor_native_save_v0_dev")]
                active_io: 0,
                held: 0,
                io: 0,
                joining: 0,
                joined_total: 0,
            })
    }
    pub fn join_native_read_finished(
        &self,
        id: &ProducerId,
    ) -> Result<JoinedProducerProof, StoreStatus> {
        let attachment = self.native_attachment()?;
        // This is shared domain cleanup authority. It does not identify which
        // service spawned a row; that provenance comes only from actual private
        // reservations/installations and our own retained-token getter.
        native(attachment.witness.join_finished(id))
    }
    pub fn native_read_producers(&self) -> Vec<ProducerId> {
        let Ok(attachment) = self.native_attachment() else {
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
