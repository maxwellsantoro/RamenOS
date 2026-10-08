//! Bounded Rust ASCII editor consumer of the default-off host contracts.
//! Local draft copies carry no authority; every effect uses an issued endpoint.
use super::{
    EditorBindings, EditorMessage, Endpoint, HostDesktop, ObjectDescriptor, ObjectKind,
    decode_envelope_wire, encode_envelope_wire,
};
use crate::dev::Status;
use desktop_editor_core::{
    EditError, EditorHint, EditorModel, EditorSnapshot, FontRows, InitialView, ResetPolicy,
};
use kernel_api::{
    cap::{Handle, HandleKind},
    generated::{
        desktop_artifact_v1 as artifact, desktop_editor_session_v1 as editor,
        desktop_focus_v1 as focus, desktop_surface_v1 as surface,
    },
};
use sha2::{Digest, Sha256};

const WIDTH: usize = desktop_editor_core::WIDTH;
const HEIGHT: usize = desktop_editor_core::HEIGHT;
const STRIDE: usize = desktop_editor_core::STRIDE;
const BUFFER_LEN: usize = desktop_editor_core::FRAME_BYTES;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveState {
    Unsaved,
    Saving { operation_id: u64 },
    Saved { revision: u64 },
    Conflict,
    Unknown { operation_id: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftSnapshot {
    pub bytes: Vec<u8>,
    pub cursor: usize,
    pub selection: Option<(usize, usize)>,
    pub first_visible_line: usize,
    pub confirmed_revision: u64,
    pub save: SaveState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepObservation {
    pub consumed_sequence: Option<u64>,
    pub draft_changed: bool,
    pub frame_sequence: Option<u64>,
    pub save: SaveState,
}

struct PendingSave {
    operation_id: u64,
    expected_revision: u64,
    expected_hash: [u8; 32],
    source_hash: [u8; 32],
    draft_generation: u64,
}

struct SaveReply {
    operation_id: u64,
    revision: u64,
    receipt_shm: u64,
    receipt_len: u32,
    status: u32,
}

pub struct EditorClient {
    host: HostDesktop,
    bindings: EditorBindings,
    bootstrap: editor::InstanceBootstrap,
    font: [[u8; 16]; 96],
    draft: DraftSnapshot,
    selection_anchor: Option<usize>,
    preferred_column: Option<usize>,
    draft_generation: u64,
    confirmed_hash: [u8; 32],
    selected_object: u64,
    selected_generation: u64,
    artifact_epoch: u64,
    focus_service_epoch: u64,
    surface_id: u64,
    surface_generation: u64,
    buffers: [u64; 2],
    next_buffer: u32,
    frame_sequence: u64,
    next_request: u64,
    focus_epoch: u64,
    key_sequence: u64,
    pending: Option<PendingSave>,
}

// Pure model errors never authorize a host effect or relax an owner counter.
fn edit_status(error: EditError) -> Status {
    match error {
        EditError::UnsupportedKey => Status::Unsupported,
        EditError::CounterExhausted => Status::Exhausted,
        EditError::BodyTooLong
        | EditError::InvalidAscii
        | EditError::InvalidSnapshot
        | EditError::InvalidModifiers
        | EditError::InvalidBufferLength => Status::Invalid,
    }
}

fn draft_model(
    draft: &DraftSnapshot,
    selection_anchor: Option<usize>,
    preferred_column: Option<usize>,
    text_generation: u64,
) -> Result<EditorModel, Status> {
    EditorModel::from_snapshot(EditorSnapshot {
        bytes: draft.bytes.clone(),
        cursor: draft.cursor,
        selection: draft.selection,
        first_visible_line: draft.first_visible_line,
        selection_anchor,
        preferred_column,
        text_generation,
    })
    .map_err(edit_status)
}

fn accept_model(
    draft: &mut DraftSnapshot,
    selection_anchor: &mut Option<usize>,
    preferred_column: &mut Option<usize>,
    text_generation: &mut u64,
    snapshot: EditorSnapshot,
) {
    draft.bytes = snapshot.bytes;
    draft.cursor = snapshot.cursor;
    draft.selection = snapshot.selection;
    draft.first_visible_line = snapshot.first_visible_line;
    *selection_anchor = snapshot.selection_anchor;
    *preferred_column = snapshot.preferred_column;
    *text_generation = snapshot.text_generation;
}

fn raster_model(model: EditorModel, font: &FontRows) -> Result<Vec<u8>, Status> {
    let mut pixels = vec![0; BUFFER_LEN];
    model
        .rasterize_bgra(font, &mut pixels)
        .map_err(edit_status)?;
    Ok(pixels)
}

fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("checked schema"),
    )
}
fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("checked schema"),
    )
}
fn schema_hash(bytes: &[u8], offset: usize) -> [u8; 32] {
    bytes[offset..offset + 32]
        .try_into()
        .expect("checked schema")
}
fn status(raw: u32) -> Result<(), Status> {
    Err(match raw {
        0 => return Ok(()),
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
        11 => Status::Internal,
        _ => Status::Invalid,
    })
}
fn descriptor(raw: u64, kind: ObjectKind, byte_len: u32) -> Result<ObjectDescriptor, Status> {
    let handle = Handle::unpack(raw);
    if raw == 0
        || handle.pack() != raw
        || handle.kind != HandleKind::Shmem
        || handle.index == 0
        || handle.generation == 0
        || byte_len == 0
    {
        return Err(Status::Invalid);
    }
    Ok(ObjectDescriptor {
        object_generation: handle.generation,
        handle,
        kind,
        byte_len,
    })
}
fn read(
    host: &HostDesktop,
    endpoint: &Endpoint,
    object: &ObjectDescriptor,
    now: u64,
) -> Result<Vec<u8>, Status> {
    // Each caller has already bounded the declared allocation before allocating.
    let lease = host.read_lease(&endpoint.peer, object, now)?;
    let mut bytes = vec![0; object.byte_len as usize];
    lease.copy_into(0, &mut bytes, now)?;
    Ok(bytes)
}
fn call<Q: EditorMessage, R: EditorMessage>(
    host: &HostDesktop,
    endpoint: &Endpoint,
    request: Q,
    now: u64,
) -> Result<R, Status> {
    let request = request.encode(endpoint.handle)?;
    let request = decode_envelope_wire(&encode_envelope_wire(&request)?)?;
    let reply = host.dispatch(&endpoint.peer, &request, now);
    let reply = decode_envelope_wire(&encode_envelope_wire(&reply)?)?;
    if request.payload[..8] != reply.payload[..8] {
        return Err(Status::Invalid);
    }
    R::decode(&reply)
}

impl EditorClient {
    pub fn new(host: HostDesktop, bindings: EditorBindings, now_ms: u64) -> Result<Self, Status> {
        let boot = editor::InstanceBootstrap::decode(&bindings.bootstrap)?;
        status(boot.status)?;
        if boot.session_id == 0
            || boot.session_generation == 0
            || boot.instance_id == 0
            || boot.instance_generation == 0
            || boot.grants_len != 464
        {
            return Err(Status::Invalid);
        }
        let grant_object = descriptor(boot.grants_shm, ObjectKind::Grants, boot.grants_len)?;
        let grants = read(&host, &bindings.self_status, &grant_object, now_ms)?;
        if u32_at(&grants, 0) != 1
            || u32_at(&grants, 4) != 464
            || u32_at(&grants, 8) != 63
            || u32_at(&grants, 12) != 4
            || u64_at(&grants, 16) != boot.session_id
            || u64_at(&grants, 24) != boot.session_generation
            || u64_at(&grants, 32) != boot.instance_id
            || u64_at(&grants, 40) != boot.instance_generation
            || u64_at(&grants, 48) == 0
            || u64_at(&grants, 56) == 0
            || u64_at(&grants, 64) == 0
            || u64_at(&grants, 72) == 0
            || u64_at(&grants, 80) == 0
            || u64_at(&grants, 88) == 0
            || u64_at(&grants, 96) <= now_ms
            || u64_at(&grants, 104) == 0
            || [
                u32_at(&grants, 208),
                u32_at(&grants, 212),
                u32_at(&grants, 216),
                u32_at(&grants, 220),
                u32_at(&grants, 224),
                u32_at(&grants, 228),
                u32_at(&grants, 232),
                u32_at(&grants, 236),
            ] != [4096, 640, 480, 2560, 1, 64, 1, 0]
        {
            return Err(Status::Invalid);
        }
        for (i, (endpoint, rights)) in [
            (&bindings.self_status, 1),
            (&bindings.focus_read, 1),
            (&bindings.surface, 15),
            (&bindings.artifact, 7),
        ]
        .into_iter()
        .enumerate()
        {
            let o = 240 + i * 56;
            if u64_at(&grants, o) != endpoint.handle.pack()
                || u64_at(&grants, o + 8) == 0
                || u64_at(&grants, o + 16) == 0
                || u64_at(&grants, o + 24) == 0
                || u64_at(&grants, o + 32) != u64_at(&grants, 96)
                || u32_at(&grants, o + 40) != i as u32 + 1
                || u32_at(&grants, o + 44) != rights
                || u32_at(&grants, o + 48) != 1
                || u32_at(&grants, o + 52) != 0
            {
                return Err(Status::Invalid);
            }
        }
        let selected_object = u64_at(&grants, 48);
        let selected_generation = u64_at(&grants, 56);
        if u64_at(&grants, 416) != selected_object || u64_at(&grants, 424) != selected_generation {
            return Err(Status::Invalid);
        }
        let initial_revision = u64_at(&grants, 64);
        let selected: artifact::ReadSelectedReply = call(
            &host,
            &bindings.artifact,
            artifact::ReadSelected {
                request_id: 1,
                session_id: boot.session_id,
                session_generation: boot.session_generation,
                instance_id: boot.instance_id,
                instance_generation: boot.instance_generation,
            },
            now_ms,
        )?;
        status(selected.status)?;
        if selected.revision != initial_revision || !(64..=4160).contains(&selected.byte_len) {
            return Err(Status::Invalid);
        }
        let text_object = descriptor(
            selected.data_shm,
            ObjectKind::SelectedText,
            selected.byte_len,
        )?;
        if selected.object_generation != text_object.object_generation {
            return Err(Status::Invalid);
        }
        let text = read(&host, &bindings.artifact, &text_object, now_ms)?;
        if u32_at(&text, 0) != 1
            || u32_at(&text, 4) as usize != text.len()
            || u64_at(&text, 8) != selected_object
            || u64_at(&text, 16) != initial_revision
            || u32_at(&text, 56) as usize != text.len() - 64
            || u32_at(&text, 60) != 0
            || !text[64..].iter().all(|b| matches!(*b, 9 | 10 | 32..=126))
            || hash(&text[64..]) != schema_hash(&text, 24)
            || schema_hash(&text, 24) != schema_hash(&grants, 176)
        {
            return Err(Status::Invalid);
        }
        let created: surface::CreateReply = call(
            &host,
            &bindings.surface,
            surface::Create {
                request_id: 2,
                session_id: boot.session_id,
                session_generation: boot.session_generation,
                instance_id: boot.instance_id,
                instance_generation: boot.instance_generation,
                width: WIDTH as u32,
                height: HEIGHT as u32,
                format: 1,
                reserved: 0,
            },
            now_ms,
        )?;
        status(created.status)?;
        if created.surface_id != u64_at(&grants, 360)
            || created.surface_generation != u64_at(&grants, 368)
            || created.stride != STRIDE as u32
            || created.format != 1
            || created.buffer0_shm == created.buffer1_shm
        {
            return Err(Status::Invalid);
        }
        descriptor(
            created.buffer0_shm,
            ObjectKind::SurfaceBuffer,
            BUFFER_LEN as u32,
        )?;
        descriptor(
            created.buffer1_shm,
            ObjectKind::SurfaceBuffer,
            BUFFER_LEN as u32,
        )?;
        // The authenticated legacy Read starts at generation zero with its
        // cursor visible. Save confirmation remains owned by this adapter.
        let model = EditorModel::from_selected_ascii(&text[64..], 0, InitialView::CursorVisible)
            .map_err(edit_status)?
            .snapshot();
        let font = host.font_rows();
        let client = Self {
            host,
            bindings,
            bootstrap: boot,
            font,
            draft: DraftSnapshot {
                bytes: model.bytes,
                cursor: model.cursor,
                selection: model.selection,
                first_visible_line: model.first_visible_line,
                confirmed_revision: initial_revision,
                save: SaveState::Saved {
                    revision: initial_revision,
                },
            },
            selection_anchor: model.selection_anchor,
            preferred_column: model.preferred_column,
            draft_generation: model.text_generation,
            confirmed_hash: schema_hash(&text, 24),
            selected_object,
            selected_generation,
            artifact_epoch: u64_at(&grants, 432),
            focus_service_epoch: u64_at(&grants, 320),
            surface_id: created.surface_id,
            surface_generation: created.surface_generation,
            buffers: [created.buffer0_shm, created.buffer1_shm],
            next_buffer: 0,
            frame_sequence: 0,
            next_request: 3,
            focus_epoch: 0,
            key_sequence: 0,
            pending: None,
        };
        Ok(client)
    }

    fn request_id(&mut self) -> Result<u64, Status> {
        let id = self.next_request;
        self.next_request = id.checked_add(1).ok_or(Status::Exhausted)?;
        Ok(id)
    }

    pub fn snapshot(&self) -> DraftSnapshot {
        self.draft.clone()
    }

    pub fn step(&mut self, now_ms: u64) -> Result<StepObservation, Status> {
        // Reconciliation reads the original receipt only. It never replays Commit.
        if self.pending.is_some() {
            self.reconcile(now_ms)?;
        }
        let request_id = self.request_id()?;
        let observed: editor::ObserveFocusReply = call(
            &self.host,
            &self.bindings.self_status,
            editor::ObserveFocus {
                request_id,
                session_id: self.bootstrap.session_id,
                session_generation: self.bootstrap.session_generation,
                instance_id: self.bootstrap.instance_id,
                instance_generation: self.bootstrap.instance_generation,
            },
            now_ms,
        )?;
        status(observed.status)?;
        if observed.focus_epoch == 0 || observed.service_epoch != self.focus_service_epoch {
            return Err(Status::Invalid);
        }
        if self.focus_epoch != observed.focus_epoch {
            self.focus_epoch = observed.focus_epoch;
            self.key_sequence = 0;
            self.reset_navigation()?;
        }
        let request_id = self.request_id()?;
        let key: focus::PollKeysReply = call(
            &self.host,
            &self.bindings.focus_read,
            focus::PollKeys {
                request_id,
                session_id: self.bootstrap.session_id,
                session_generation: self.bootstrap.session_generation,
                instance_id: self.bootstrap.instance_id,
                instance_generation: self.bootstrap.instance_generation,
                focus_epoch: self.focus_epoch,
            },
            now_ms,
        )?;
        status(key.status)?;
        if key.sequence == 0
            || key.sequence <= self.key_sequence
            || key.focus_epoch != self.focus_epoch
            || key.modifiers & !3 != 0
            || !(1..=3).contains(&key.phase)
        {
            return Err(Status::Invalid);
        }
        if key.phase == 3 && (key.usage != 0 || key.modifiers != 0) {
            return Err(Status::Invalid);
        }
        if key.phase != 3 && !matches!(key.usage, 4..=49 | 51..=56 | 74 | 77 | 79..=82 | 224 | 225)
        {
            return Err(Status::Unsupported);
        }
        self.key_sequence = key.sequence;
        let generation = self.draft_generation;
        if key.phase == 3 {
            self.reset_navigation()?;
        } else if key.phase == 1 {
            self.press(key.usage, key.modifiers, now_ms)?;
        }
        Ok(StepObservation {
            consumed_sequence: Some(key.sequence),
            draft_changed: generation != self.draft_generation,
            frame_sequence: None,
            save: self.draft.save.clone(),
        })
    }

    fn model(&self) -> Result<EditorModel, Status> {
        draft_model(
            &self.draft,
            self.selection_anchor,
            self.preferred_column,
            self.draft_generation,
        )
    }

    fn accept_model(&mut self, snapshot: EditorSnapshot) {
        accept_model(
            &mut self.draft,
            &mut self.selection_anchor,
            &mut self.preferred_column,
            &mut self.draft_generation,
            snapshot,
        );
    }

    fn reset_navigation(&mut self) -> Result<(), Status> {
        let mut model = self.model()?;
        model.reset_navigation(ResetPolicy::ClearPreferredColumn);
        self.accept_model(model.snapshot());
        Ok(())
    }

    fn press(&mut self, usage: u32, modifiers: u32, now: u64) -> Result<(), Status> {
        let mut model = self.model()?;
        let effect = model.apply_key(usage, modifiers).map_err(edit_status)?;
        if effect.overflow_rejected {
            return Err(Status::Exhausted);
        }
        if effect.hint == EditorHint::SaveRequested {
            // A data-only hint enters the original authenticated Save path.
            return self.save(now);
        }
        self.accept_model(model.snapshot());
        if effect.text_changed && self.pending.is_none() {
            self.draft.save = SaveState::Unsaved;
        }
        Ok(())
    }

    fn raster(&self) -> Result<Vec<u8>, Status> {
        raster_model(self.model()?, &self.font)
    }

    pub fn render(&mut self, now_ms: u64) -> Result<u64, Status> {
        let sequence = self
            .frame_sequence
            .checked_add(1)
            .ok_or(Status::Exhausted)?;
        let request_id = self.request_id()?;
        let acquired: surface::AcquireReply = call(
            &self.host,
            &self.bindings.surface,
            surface::Acquire {
                request_id,
                surface_id: self.surface_id,
                surface_generation: self.surface_generation,
                buffer_index: self.next_buffer,
                reserved: 0,
            },
            now_ms,
        )?;
        status(acquired.status)?;
        if acquired.mapping_generation == 0
            || acquired.buffer_shm != self.buffers[self.next_buffer as usize]
            || acquired.byte_len != BUFFER_LEN as u32
        {
            return Err(Status::Invalid);
        }
        let object = descriptor(
            acquired.buffer_shm,
            ObjectKind::SurfaceBuffer,
            acquired.byte_len,
        )?;
        let lease = self.host.write_lease(
            &self.bindings.surface.peer,
            &object,
            acquired.mapping_generation,
            now_ms,
        )?;
        lease.copy_from(0, &self.raster()?, now_ms)?;
        let request_id = self.request_id()?;
        let presented: surface::PresentReply = call(
            &self.host,
            &self.bindings.surface,
            surface::Present {
                request_id,
                surface_id: self.surface_id,
                surface_generation: self.surface_generation,
                mapping_generation: acquired.mapping_generation,
                sequence,
                buffer_index: self.next_buffer,
                reserved: 0,
            },
            now_ms,
        )?;
        status(presented.status)?;
        if presented.sequence != sequence {
            return Err(Status::Invalid);
        }
        self.frame_sequence = sequence;
        self.next_buffer ^= 1;
        Ok(sequence)
    }

    fn save(&mut self, now: u64) -> Result<(), Status> {
        if self.pending.is_some() {
            return Ok(());
        }
        let expected_revision = self.draft.confirmed_revision;
        let source_hash = hash(&self.draft.bytes);
        let request_id = self.request_id()?;
        let allocated: artifact::AllocateSaveIdReply = call(
            &self.host,
            &self.bindings.artifact,
            artifact::AllocateSaveId {
                request_id,
                session_id: self.bootstrap.session_id,
                session_generation: self.bootstrap.session_generation,
                expected_revision,
            },
            now,
        )?;
        if allocated.status == 7 {
            self.draft.save = SaveState::Conflict;
            return Ok(());
        }
        status(allocated.status)?;
        if allocated.operation_id == 0 {
            return Err(Status::Invalid);
        }
        let source = self.host.draft_source(
            &self.bindings.artifact.peer,
            expected_revision,
            &self.draft.bytes,
            now,
        )?;
        if source.kind != ObjectKind::DraftSource
            || source.byte_len as usize != self.draft.bytes.len() + 64
            || source.object_generation != source.handle.generation
        {
            return Err(Status::Invalid);
        }
        descriptor(
            source.handle.pack(),
            ObjectKind::DraftSource,
            source.byte_len,
        )?;
        let request_id = self.request_id()?;
        let pending = PendingSave {
            operation_id: allocated.operation_id,
            expected_revision,
            expected_hash: self.confirmed_hash,
            source_hash,
            draft_generation: self.draft_generation,
        };
        self.draft.save = SaveState::Saving {
            operation_id: pending.operation_id,
        };
        self.pending = Some(pending);
        // Once dispatched, a missing/malformed reply is uncertainty, never permission to retry.
        let reply: Result<artifact::CommitReply, Status> = call(
            &self.host,
            &self.bindings.artifact,
            artifact::Commit {
                request_id,
                session_id: self.bootstrap.session_id,
                session_generation: self.bootstrap.session_generation,
                expected_revision,
                operation_id: allocated.operation_id,
                source_shm: source.handle.pack(),
                byte_len: source.byte_len,
                reserved: 0,
            },
            now,
        );
        match reply {
            Ok(r) => self.accept_save_reply(
                SaveReply {
                    operation_id: r.operation_id,
                    revision: r.revision,
                    receipt_shm: r.receipt_shm,
                    receipt_len: r.receipt_len,
                    status: r.status,
                },
                true,
                now,
            ),
            Err(_) => {
                self.draft.save = SaveState::Unknown {
                    operation_id: allocated.operation_id,
                };
                Ok(())
            }
        }
    }

    fn reconcile(&mut self, now: u64) -> Result<(), Status> {
        let operation_id = self.pending.as_ref().expect("pending save").operation_id;
        let request_id = self.request_id()?;
        let reply: artifact::SaveStatusReply = call(
            &self.host,
            &self.bindings.artifact,
            artifact::SaveStatus {
                request_id,
                session_id: self.bootstrap.session_id,
                session_generation: self.bootstrap.session_generation,
                operation_id,
            },
            now,
        )?;
        self.accept_save_reply(
            SaveReply {
                operation_id: reply.operation_id,
                revision: reply.revision,
                receipt_shm: reply.receipt_shm,
                receipt_len: reply.receipt_len,
                status: reply.status,
            },
            false,
            now,
        )
    }

    fn accept_save_reply(
        &mut self,
        reply: SaveReply,
        original: bool,
        now: u64,
    ) -> Result<(), Status> {
        let SaveReply {
            operation_id,
            revision,
            receipt_shm,
            receipt_len,
            status: raw_status,
        } = reply;
        let pending = self.pending.as_ref().ok_or(Status::Invalid)?;
        let original_id = pending.operation_id;
        if raw_status == 10 {
            self.draft.save = SaveState::Unknown {
                operation_id: original_id,
            };
            if operation_id != original_id || revision != 0 || receipt_shm != 0 || receipt_len != 0
            {
                return Err(Status::Invalid);
            }
            return Ok(());
        }
        if raw_status != 0 {
            // An original Timeout is definitive: the host fenced admission before a
            // permit. A timed-out receipt lookup still leaves the operation Unknown.
            if original && matches!(raw_status, 1..=8) {
                self.pending = None;
                self.draft.save = if raw_status == 7 {
                    SaveState::Conflict
                } else {
                    SaveState::Unsaved
                };
                if raw_status == 7 {
                    return Ok(());
                }
                return status(raw_status);
            }
            self.draft.save = SaveState::Unknown {
                operation_id: original_id,
            };
            // A retired receipt-read endpoint cannot prove the original save failed.
            // Preserve its operation while refusing further use of that endpoint.
            if !original && matches!(raw_status, 1..=5) {
                return status(raw_status);
            }
            return Ok(());
        }
        // Until every original receipt field is verified, the draft remains Unknown.
        self.draft.save = SaveState::Unknown {
            operation_id: original_id,
        };
        if operation_id != original_id || receipt_len != 176 {
            return Err(Status::Invalid);
        }
        let object = descriptor(receipt_shm, ObjectKind::Receipt, receipt_len)?;
        let bytes = read(&self.host, &self.bindings.artifact, &object, now)?;
        if u32_at(&bytes, 0) != 1
            || u32_at(&bytes, 4) != 176
            || u32_at(&bytes, 12) != 0
            || u64_at(&bytes, 16) == 0
            || u64_at(&bytes, 24) != self.bootstrap.session_id
            || u64_at(&bytes, 32) != self.bootstrap.session_generation
            || u64_at(&bytes, 40) != self.bootstrap.instance_id
            || u64_at(&bytes, 48) != self.bootstrap.instance_generation
            || u64_at(&bytes, 56) != self.selected_object
            || u64_at(&bytes, 64) != self.selected_generation
            || u64_at(&bytes, 72) != original_id
            || u64_at(&bytes, 80) != pending.expected_revision
            || u64_at(&bytes, 96) != self.artifact_epoch
            || schema_hash(&bytes, 112) != pending.expected_hash
            || schema_hash(&bytes, 144) != pending.source_hash
        {
            return Err(Status::Invalid);
        }
        match u32_at(&bytes, 8) {
            0 => {
                let successor = pending
                    .expected_revision
                    .checked_add(1)
                    .ok_or(Status::Exhausted)?;
                if revision != successor || u64_at(&bytes, 88) != successor {
                    return Err(Status::Invalid);
                }
                self.draft.confirmed_revision = successor;
                self.confirmed_hash = pending.source_hash;
                self.draft.save = if self.draft_generation == pending.draft_generation
                    && hash(&self.draft.bytes) == pending.source_hash
                {
                    SaveState::Saved {
                        revision: successor,
                    }
                } else {
                    SaveState::Unsaved
                };
            }
            1 => {
                // Commit Ok certifies a committed transition. Only a later
                // authenticated SaveStatus read can settle definitive noncommit.
                if original || revision != 0 || u64_at(&bytes, 88) != 0 || u64_at(&bytes, 104) != 0
                {
                    return Err(Status::Invalid);
                }
                self.draft.save = SaveState::Unsaved;
            }
            _ => return Err(Status::Invalid),
        }
        self.pending = None;
        Ok(())
    }
}

#[cfg(feature = "editor_native_save_v0_dev")]
pub(super) struct NativeTextEditor {
    pub(super) draft: DraftSnapshot,
    pub(super) selection_anchor: Option<usize>,
    pub(super) preferred_column: Option<usize>,
    pub(super) draft_generation: u64,
    font: [[u8; 16]; 96],
}
#[cfg(feature = "editor_native_save_v0_dev")]
impl NativeTextEditor {
    pub(super) fn from_native(
        v: &super::NativeEditorObservation,
        anchor: Option<usize>,
        column: Option<usize>,
        font: [[u8; 16]; 96],
    ) -> Result<Self, Status> {
        let text = Self {
            draft: DraftSnapshot {
                bytes: v.bytes.clone(),
                cursor: v.cursor as usize,
                selection: v.selection.map(|(a, b)| (a as usize, b as usize)),
                first_visible_line: v.first_visible_line as usize,
                confirmed_revision: v.confirmed_revision,
                save: SaveState::Unsaved,
            },
            selection_anchor: anchor,
            preferred_column: column,
            draft_generation: v.text_generation,
            font,
        };
        // Preserve all seven fields of the genuine owner snapshot. Native Read
        // initialization (generation one, top view) remains in its real owner;
        // continuing edits never reconstruct or reset that state from the body.
        text.model()?;
        Ok(text)
    }

    fn model(&self) -> Result<EditorModel, Status> {
        draft_model(
            &self.draft,
            self.selection_anchor,
            self.preferred_column,
            self.draft_generation,
        )
    }

    fn accept_model(&mut self, snapshot: EditorSnapshot) {
        accept_model(
            &mut self.draft,
            &mut self.selection_anchor,
            &mut self.preferred_column,
            &mut self.draft_generation,
            snapshot,
        );
    }

    // False is an accepted key whose insertion exceeds the text length bound;
    // it leaves the detached scratch untouched. Counter exhaustion is an error.
    pub(super) fn press(&mut self, usage: u32, modifiers: u32) -> Result<bool, Status> {
        let mut model = self.model()?;
        let effect = model.apply_key(usage, modifiers).map_err(edit_status)?;
        if effect.hint == EditorHint::SaveRequested {
            // Only the genuine host CtrlS branch can mint a NativeSaveIntent.
            return Err(Status::Unsupported);
        }
        if effect.overflow_rejected {
            return Ok(false);
        }
        self.accept_model(model.snapshot());
        if effect.text_changed {
            self.draft.save = SaveState::Unsaved;
        }
        Ok(true)
    }

    pub(super) fn raster(&self) -> Result<Vec<u8>, Status> {
        raster_model(self.model()?, &self.font)
    }
}
