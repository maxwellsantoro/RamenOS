//! Concrete default-off consumer. All authority and publication stay in services.
use artifact_store_schema::editor_save::{
    EditorSaveBindingV0, EditorSaveReceiptV0, EditorTextHeaderV0,
};
use desktop_service::dev::Status;
use desktop_service::editor_dev::{self as desktop, EditorMessage};
use kernel_api::generated::{
    desktop_artifact_v1 as artifact, desktop_editor_session_v1 as session,
};
use kernel_api::ipc::Envelope;
use std::time::Instant;
use store_service::editor_store::{self as store, StoreStatus};

const ORIGINALS_PER_DOCUMENT: usize = 16;

#[derive(Debug, PartialEq, Eq)]
pub enum NativeEditorError {
    Desktop(Status),
    Store(StoreStatus),
    InvalidReceipt,
    Exhausted,
}
impl From<Status> for NativeEditorError {
    fn from(value: Status) -> Self {
        Self::Desktop(value)
    }
}
impl From<StoreStatus> for NativeEditorError {
    fn from(value: StoreStatus) -> Self {
        Self::Store(value)
    }
}

pub struct NativeArtifactEditor {
    desktop: desktop::HostDesktop,
    store: store::StoreHost,
    launch: store::PairedNativeSaveLaunch,
    editor: desktop::NativeEditor,
    next_request: u64,
    pending: Vec<SaveChain>,
    // Registration failure owns the consumed call and any genuine installed producers.
    registration_failure: Option<store::NativeSaveRegistrationFailure>,
}

struct Flight {
    _entry: desktop::NativeSaveEntry,
    registered: store::RegisteredNativeSave,
    execution: Option<store::NativeSaveExecution>,
    request: Envelope,
}
#[derive(Clone, Copy)]
enum Stage {
    Allocate,
    Commit,
    ObserveAllocation,
    ObserveCommit,
}
struct SaveChain {
    intent: desktop::NativeSaveIntent,
    allocation_request: Envelope,
    original: Option<store::OriginalNativeSave>,
    flight: Option<Flight>,
    source: Option<store::NativeDraftSource>,
    binding: Option<EditorSaveBindingV0>,
    stage: Stage,
    continuation_allowed: bool,
}

impl NativeArtifactEditor {
    pub fn open(
        desktop: desktop::HostDesktop,
        store: store::StoreHost,
        launch: store::PairedNativeSaveLaunch,
        now: u64,
    ) -> Result<Self, NativeEditorError> {
        let boot = session::InstanceBootstrap::decode(launch.desktop().bindings().bootstrap())?;
        let request = canonical(
            artifact::ReadSelected {
                request_id: 1,
                session_id: boot.session_id,
                session_generation: boot.session_generation,
                instance_id: boot.instance_id,
                instance_generation: boot.instance_generation,
            }
            .encode(launch.endpoint().handle)?,
        )?;
        let entry = desktop.begin_save_read(
            launch.desktop().bindings().artifact_origin(),
            &request,
            now,
        )?;
        let registered = store
            .register_native_save(launch.endpoint(), entry.wait_once()?)
            .map_err(|error| NativeEditorError::Store(error.status()))?;
        let mut reply = store.execute_native_save(&registered)?;
        let response = artifact::ReadSelectedReply::decode(reply.wire())?;
        if response.request_id != 1 {
            return Err(NativeEditorError::InvalidReceipt);
        }
        successful(response.status)?;
        let lease = reply.selected().ok_or(NativeEditorError::InvalidReceipt)?;
        let descriptor = lease.descriptor();
        if descriptor.byte_len != response.byte_len
            || descriptor.handle.pack() != response.data_shm
            || descriptor.object_generation != response.object_generation
            || !(64..=4160).contains(&descriptor.byte_len)
        {
            return Err(NativeEditorError::InvalidReceipt);
        }
        // This is a genuine full lease copy, never document initialization authority.
        let mut bytes = vec![0; descriptor.byte_len as usize];
        lease.copy_into(0, &mut bytes)?;
        let header = EditorTextHeaderV0::decode_le(&bytes[..64])
            .map_err(|_| NativeEditorError::InvalidReceipt)?;
        header
            .validate_bytes(&bytes[64..])
            .map_err(|_| NativeEditorError::InvalidReceipt)?;
        if header.revision != response.revision {
            return Err(NativeEditorError::InvalidReceipt);
        }
        let editor = reply.take_editor()?;
        // Only the actual Core created this opaque editor; copied bytes confer no authority.
        drop((bytes, reply, registered, entry));
        Ok(Self {
            desktop,
            store,
            launch,
            editor,
            next_request: 2,
            pending: Vec::new(),
            registration_failure: None,
        })
    }

    pub fn step(&mut self, now: u64) -> Result<desktop::NativeEditorStep, NativeEditorError> {
        // Input is processed first; older filesystem work never blocks current edits.
        let step = self.desktop.step_native_editor(&self.editor, now)?;
        let mut index = 0;
        while index < self.pending.len() {
            let mut chain = self.pending.remove(index);
            match self.progress(&mut chain, now) {
                Ok(true) => {
                    self.pending.insert(index, chain);
                    index += 1;
                }
                Ok(false) => {}
                Err(error) => {
                    self.pending.insert(index, chain);
                    return Err(error);
                }
            }
        }
        if step.save_intent_available {
            if self.pending.len() >= ORIGINALS_PER_DOCUMENT {
                return Err(NativeEditorError::Exhausted);
            }
            let intent = self.desktop.take_save_intent(&self.editor)?;
            let view = intent.snapshot().view();
            let request = canonical(
                artifact::AllocateSaveId {
                    request_id: self.request_id()?,
                    session_id: view.actor.session_id,
                    session_generation: view.actor.session_generation,
                    expected_revision: view.expected_revision,
                }
                .encode(self.launch.endpoint().handle)?,
            )?;
            let mut chain = SaveChain {
                intent,
                allocation_request: request,
                original: None,
                flight: None,
                source: None,
                binding: None,
                stage: Stage::Allocate,
                continuation_allowed: true,
            };
            let start = self.start_allocate(&mut chain, now);
            self.pending.push(chain);
            start?;
        }
        Ok(step)
    }

    pub fn render(
        &mut self,
        now: u64,
    ) -> Result<desktop::NativeFrameObservation, NativeEditorError> {
        self.desktop
            .render_native_editor(&self.editor, now)
            .map_err(Into::into)
    }
    pub fn observe(&self, now: u64) -> Result<desktop::NativeEditorObservation, NativeEditorError> {
        self.desktop
            .editor_observation(&self.editor, now)
            .map_err(Into::into)
    }

    pub fn reconcile(&mut self, now: u64) -> Result<(), NativeEditorError> {
        let mut index = 0;
        while index < self.pending.len() {
            let mut chain = self.pending.remove(index);
            let outcome = match self.progress(&mut chain, now) {
                Ok(false) => Ok(false),
                Ok(true) if chain.flight.as_ref().is_some_and(|f| f.execution.is_some()) => {
                    Ok(true)
                }
                Ok(true) => self.observe_original(&mut chain, now),
                Err(error) => Err(error),
            };
            match outcome {
                Ok(true) => {
                    self.pending.insert(index, chain);
                    index += 1;
                }
                Ok(false) => {}
                Err(error) => {
                    self.pending.insert(index, chain);
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    fn request_id(&mut self) -> Result<u64, NativeEditorError> {
        let next = self
            .next_request
            .checked_add(1)
            .ok_or(NativeEditorError::Exhausted)?;
        let id = self.next_request;
        self.next_request = next;
        Ok(id)
    }
    fn register(
        &mut self,
        entry: desktop::NativeSaveEntry,
        request: Envelope,
    ) -> Result<Flight, NativeEditorError> {
        let call = entry.wait_once()?;
        let registered = match self
            .store
            .register_native_save(self.launch.endpoint(), call)
        {
            Ok(value) => value,
            Err(error) => {
                let status = error.status();
                self.registration_failure = Some(error);
                return Err(NativeEditorError::Store(status));
            }
        };
        Ok(Flight {
            _entry: entry,
            registered,
            execution: None,
            request,
        })
    }
    fn start_allocate(&mut self, chain: &mut SaveChain, now: u64) -> Result<(), NativeEditorError> {
        let entry = self.desktop.begin_save_allocate(
            self.launch.desktop().bindings().artifact_origin(),
            &chain.intent,
            &chain.allocation_request,
            now,
        )?;
        self.start_flight(chain, entry, chain.allocation_request)
    }
    fn start_flight(
        &mut self,
        chain: &mut SaveChain,
        entry: desktop::NativeSaveEntry,
        request: Envelope,
    ) -> Result<(), NativeEditorError> {
        chain.flight = Some(self.register(entry, request)?);
        let flight = chain.flight.as_mut().expect("actual registered flight");
        match self.store.start_native_save(&flight.registered) {
            Ok(execution) => {
                flight.execution = Some(execution);
                Ok(())
            }
            Err(error) => {
                chain.stage = match chain.stage {
                    Stage::Allocate => Stage::ObserveAllocation,
                    _ => Stage::ObserveCommit,
                };
                Err(error.into())
            }
        }
    }
    fn start_commit(
        &mut self,
        chain: &mut SaveChain,
        allocation: &store::NativeAllocatedOperation,
        now: u64,
    ) -> Result<(), NativeEditorError> {
        // An observation never renews the original explicit CtrlS deadline.
        if !chain.continuation_allowed || Instant::now() >= chain.intent.deadline() {
            chain.continuation_allowed = false;
            chain.stage = Stage::ObserveAllocation;
            return Ok(());
        }
        // Even a source-construction error must never cause an automatic retry.
        chain.continuation_allowed = false;
        let source =
            self.store
                .source_for_intent(self.launch.endpoint(), allocation, &chain.intent)?;
        let view = chain.intent.snapshot().view();
        let body = chain.intent.snapshot().body();
        let header = EditorTextHeaderV0 {
            schema_version: 1,
            total_len: 64 + view.byte_len,
            selected_object_id: view.object_id,
            revision: view.expected_revision,
            content_hash: view.source_content_hash,
            byte_len: view.byte_len,
            reserved: 0,
        };
        let mut bytes = header
            .encode_le()
            .map_err(|_| NativeEditorError::InvalidReceipt)?
            .to_vec();
        bytes.extend_from_slice(body);
        let descriptor = source.descriptor();
        if bytes.len() != descriptor.byte_len as usize {
            return Err(NativeEditorError::InvalidReceipt);
        }
        source.write_lease()?.copy_from(0, &bytes)?;
        let request = canonical(
            artifact::Commit {
                request_id: self.request_id()?,
                session_id: view.actor.session_id,
                session_generation: view.actor.session_generation,
                expected_revision: view.expected_revision,
                operation_id: allocation.observation().operation_id,
                source_shm: descriptor.handle.pack(),
                byte_len: descriptor.byte_len,
                reserved: 0,
            }
            .encode(self.launch.endpoint().handle)?,
        )?;
        let entry = self.desktop.begin_save_commit(
            self.launch.desktop().bindings().artifact_origin(),
            source.bound(),
            &request,
            now,
        )?;
        chain.binding = Some(source.bound().binding().clone());
        chain.source = Some(source);
        chain.stage = Stage::Commit;
        self.start_flight(chain, entry, request)
    }

    /// Returns false only after an actual terminal response or original observation.
    fn progress(&mut self, chain: &mut SaveChain, now: u64) -> Result<bool, NativeEditorError> {
        let reply = match chain.flight.as_ref() {
            Some(flight) => match &flight.execution {
                Some(execution) => match execution.poll_once()? {
                    Some(reply) => reply,
                    None => return Ok(true),
                },
                None => return Ok(true),
            },
            None => return Ok(true),
        };
        let flight = chain.flight.take().expect("actual ready flight");
        let request = match chain.stage {
            Stage::Allocate => {
                artifact::AllocateSaveId::decode(&chain.allocation_request)?.request_id
            }
            Stage::Commit => artifact::Commit::decode(&flight.request)?.request_id,
            _ => return Err(NativeEditorError::InvalidReceipt),
        };
        let mut parts = reply.into_parts();
        chain.original = Some(parts.original);
        match chain.stage {
            Stage::Allocate => {
                let response = artifact::AllocateSaveIdReply::decode(&parts.wire)?;
                if response.request_id != request {
                    return Err(NativeEditorError::InvalidReceipt);
                }
                chain.stage = Stage::ObserveAllocation;
                if response.status == StoreStatus::Ok as u32 {
                    let allocation = parts
                        .allocation
                        .take()
                        .ok_or(NativeEditorError::InvalidReceipt)?;
                    if allocation.observation().operation_id != response.operation_id {
                        return Err(NativeEditorError::InvalidReceipt);
                    }
                    self.start_commit(chain, &allocation, now)?;
                } else if !uncertain(response.status)? {
                    return Ok(false);
                }
            }
            Stage::Commit => {
                let response = artifact::CommitReply::decode(&parts.wire)?;
                if response.request_id != request {
                    return Err(NativeEditorError::InvalidReceipt);
                }
                chain.stage = Stage::ObserveCommit;
                if response.status == StoreStatus::Ok as u32 {
                    let lease = parts
                        .receipt
                        .as_ref()
                        .ok_or(NativeEditorError::InvalidReceipt)?;
                    self.validate_receipt(
                        chain,
                        lease,
                        response.operation_id,
                        response.revision,
                        response.receipt_shm,
                        response.receipt_len,
                    )?;
                    return Ok(false);
                }
                if !uncertain(response.status)? {
                    return Ok(false);
                }
            }
            _ => return Err(NativeEditorError::InvalidReceipt),
        }
        // Caller aliases are released; genuine installed handles remain in Core until actual join.
        drop((
            flight,
            parts.selected,
            parts.receipt,
            parts.editor,
            parts.producers,
        ));
        // Allocate processing may have just started Commit: retain its genuine data
        // owner until that Commit produces its own caller response.
        if matches!(chain.stage, Stage::ObserveCommit) {
            chain.source.take();
        }
        Ok(true)
    }

    fn validate_receipt(
        &self,
        chain: &SaveChain,
        lease: &store::NativeReceiptRead,
        operation: u64,
        revision: u64,
        shm: u64,
        len: u32,
    ) -> Result<(), NativeEditorError> {
        let descriptor = lease.descriptor();
        if len != 176 || descriptor.byte_len != 176 || descriptor.handle.pack() != shm {
            return Err(NativeEditorError::InvalidReceipt);
        }
        let mut bytes = [0; 176];
        lease.copy_into(0, &mut bytes)?;
        let receipt = EditorSaveReceiptV0::decode_le(&bytes)
            .map_err(|_| NativeEditorError::InvalidReceipt)?;
        let binding = chain
            .binding
            .as_ref()
            .ok_or(NativeEditorError::InvalidReceipt)?;
        receipt
            .validate_binding(binding)
            .map_err(|_| NativeEditorError::InvalidReceipt)?;
        if receipt.operation_id != operation
            || receipt.result_revision != revision
            || receipt.backend_service_epoch != chain.intent.snapshot().view().original_store_epoch
        {
            return Err(NativeEditorError::InvalidReceipt);
        }
        // Receipt is an observation. Only service-private original/current-frame joins admit Saved.
        Ok(())
    }

    fn observe_original(
        &mut self,
        chain: &mut SaveChain,
        now: u64,
    ) -> Result<bool, NativeEditorError> {
        let original = match &chain.flight {
            Some(flight) => flight.registered.original(),
            None => match &chain.original {
                Some(value) => value,
                None => return Ok(false),
            },
        };
        let snapshot = chain.intent.snapshot().view();
        let id = self.request_id()?;
        let request = match chain.stage {
            Stage::ObserveAllocation => artifact::ObserveAllocation {
                request_id: id,
                session_id: snapshot.actor.session_id,
                session_generation: snapshot.actor.session_generation,
                original_allocate_request_id: artifact::AllocateSaveId::decode(
                    &chain.allocation_request,
                )?
                .request_id,
            }
            .encode(self.launch.endpoint().handle)?,
            Stage::ObserveCommit => artifact::SaveStatus {
                request_id: id,
                session_id: snapshot.actor.session_id,
                session_generation: snapshot.actor.session_generation,
                operation_id: chain
                    .binding
                    .as_ref()
                    .ok_or(NativeEditorError::InvalidReceipt)?
                    .allocation
                    .operation_id,
            }
            .encode(self.launch.endpoint().handle)?,
            _ => return Ok(true),
        };
        let request = canonical(request)?;
        let entry =
            self.desktop
                .begin_save_recovery(original.observation_origin(), &request, now)?;
        let mut reply = self
            .store
            .observe_original(entry.wait_once()?, original)
            .map_err(|error| NativeEditorError::Store(error.status()))?;
        match reply.observation() {
            store::NativeOriginalObservation::AllocationPending
            | store::NativeOriginalObservation::CommitPending { .. } => Ok(true),
            store::NativeOriginalObservation::NoAllocation => Ok(false),
            store::NativeOriginalObservation::AllocationRetained {
                commit_dispatched, ..
            } => {
                if !commit_dispatched
                    && chain.continuation_allowed
                    && Instant::now() < chain.intent.deadline()
                {
                    if let Some(allocation) = reply.take_allocation() {
                        self.start_commit(chain, &allocation, now)?;
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            store::NativeOriginalObservation::Receipt { binding, receipt } => {
                if chain.binding.as_ref() != Some(binding) {
                    return Err(NativeEditorError::InvalidReceipt);
                }
                let response = artifact::SaveStatusReply::decode(reply.wire())?;
                if response.request_id != id || response.status != 0 {
                    return Err(NativeEditorError::InvalidReceipt);
                }
                self.validate_receipt(
                    chain,
                    receipt,
                    response.operation_id,
                    response.revision,
                    response.receipt_shm,
                    response.receipt_len,
                )?;
                Ok(false)
            }
        }
    }
}

fn canonical(request: Envelope) -> Result<Envelope, NativeEditorError> {
    Ok(desktop::decode_envelope_wire(
        &desktop::encode_envelope_wire(&request)?,
    )?)
}
fn successful(value: u32) -> Result<(), NativeEditorError> {
    if value == 0 {
        Ok(())
    } else {
        Err(NativeEditorError::Store(status(value)?))
    }
}
fn uncertain(value: u32) -> Result<bool, NativeEditorError> {
    Ok(matches!(
        status(value)?,
        StoreStatus::Unknown
            | StoreStatus::Timeout
            | StoreStatus::Disconnected
            | StoreStatus::NotReady
    ))
}
fn status(value: u32) -> Result<StoreStatus, NativeEditorError> {
    Ok(match value {
        0 => StoreStatus::Ok,
        1 => StoreStatus::Denied,
        2 => StoreStatus::Invalid,
        3 => StoreStatus::Unsupported,
        4 => StoreStatus::Stale,
        5 => StoreStatus::Exhausted,
        6 => StoreStatus::NotReady,
        7 => StoreStatus::Conflict,
        8 => StoreStatus::Timeout,
        9 => StoreStatus::Disconnected,
        10 => StoreStatus::Unknown,
        11 => StoreStatus::Internal,
        _ => return Err(NativeEditorError::InvalidReceipt),
    })
}
