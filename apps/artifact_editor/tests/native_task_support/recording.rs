//! Test-private complete actual observations; records confer no service authority.
//! Export admission and source scenario inventory remain separate from native enforcement.
#![allow(dead_code)]
#![allow(
    clippy::needless_borrow,
    clippy::map_identity,
    clippy::too_many_arguments
)]
use artifact_store_schema::editor_save as pure;
use desktop_service::editor_dev as desktop;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
use store_service::editor_store as store;

#[derive(Debug)]
pub struct RecordingError(pub &'static str);
type Result<T> = std::result::Result<T, RecordingError>;
fn require(v: bool, why: &'static str) -> Result<()> {
    if v { Ok(()) } else { Err(RecordingError(why)) }
}
fn observed<T>(r: Result<T>) -> T {
    r.expect("bounded actual recording")
}
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 15) as usize] as char);
    }
    s
}
fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
fn dto_desktop_producer_kind(r: &desktop::ProducerKind) -> Value {
    json!(match r {
        desktop::ProducerKind::DesktopDispatch => "DesktopDispatch",
        desktop::ProducerKind::DesktopSupervisor => "DesktopSupervisor",
        desktop::ProducerKind::StoreDispatch => "StoreDispatch",
        desktop::ProducerKind::StoreSupervisor => "StoreSupervisor",
        desktop::ProducerKind::StoreIo => "StoreIo",
    })
}
fn dto_desktop_join_outcome(r: &desktop::JoinOutcome) -> Value {
    json!(match r {
        desktop::JoinOutcome::Returned => "Returned",
        desktop::JoinOutcome::Panicked => "Panicked",
    })
}
fn dto_store_producer_kind(r: &store::ProducerKind) -> Value {
    json!(match r {
        store::ProducerKind::Dispatcher => "Dispatcher",
        store::ProducerKind::Supervisor => "Supervisor",
        store::ProducerKind::ObjectIoWorker => "ObjectIoWorker",
    })
}
fn dto_store_producer_join_outcome(r: &store::ProducerJoinOutcome) -> Value {
    json!(match r {
        store::ProducerJoinOutcome::Returned => "Returned",
        store::ProducerJoinOutcome::Panicked => "Panicked",
    })
}
fn dto_original_ticket_kind(r: &store::OriginalTicketKind) -> Value {
    json!(match r {
        store::OriginalTicketKind::Allocate => "Allocate",
        store::OriginalTicketKind::Commit => "Commit",
    })
}
fn dto_original_ticket_completion(r: &store::OriginalTicketCompletion) -> Value {
    json!(match r {
        store::OriginalTicketCompletion::Pending => "Pending",
        store::OriginalTicketCompletion::JoinedReturned => "JoinedReturned",
        store::OriginalTicketCompletion::JoinedPanicked => "JoinedPanicked",
    })
}
fn dto_native_pending_original_phase(r: &desktop::NativePendingOriginalPhase) -> Value {
    json!(match r {
        desktop::NativePendingOriginalPhase::AllocationUnknown => "AllocationUnknown",
        desktop::NativePendingOriginalPhase::CommitUnknown => "CommitUnknown",
    })
}
fn dto_native_editor_phase(r: &desktop::NativeEditorPhase) -> Value {
    json!(match r {
        desktop::NativeEditorPhase::Loaded => "Loaded",
        desktop::NativeEditorPhase::Unsaved => "Unsaved",
        desktop::NativeEditorPhase::AllocationPending => "AllocationPending",
        desktop::NativeEditorPhase::AllocationUnknown => "AllocationUnknown",
        desktop::NativeEditorPhase::Saving => "Saving",
        desktop::NativeEditorPhase::Unknown => "Unknown",
        desktop::NativeEditorPhase::Saved => "Saved",
        desktop::NativeEditorPhase::Conflict => "Conflict",
        desktop::NativeEditorPhase::Unavailable => "Unavailable",
    })
}
fn dto_store_endpoint_class(r: &store::EndpointClass) -> Value {
    json!(match r {
        store::EndpointClass::Artifact => "Artifact",
        store::EndpointClass::RecoveryReceipt => "RecoveryReceipt",
    })
}
fn dto_store_fixture_error_kind(r: &store::FixtureErrorKind) -> Value {
    json!(match r {
        store::FixtureErrorKind::InvalidProfile => "InvalidProfile",
        store::FixtureErrorKind::Io => "Io",
        store::FixtureErrorKind::Corrupt => "Corrupt",
        store::FixtureErrorKind::Unfenced => "Unfenced",
        store::FixtureErrorKind::Capacity => "Capacity",
        store::FixtureErrorKind::Exhausted => "Exhausted",
        store::FixtureErrorKind::Timeout => "Timeout",
    })
}
fn dto_store_io_point(r: &store::IoPoint) -> Value {
    json!(match r {
        store::IoPoint::CandidateIntent => "CandidateIntent",
        store::IoPoint::CandidateBlobWrite => "CandidateBlobWrite",
        store::IoPoint::CandidateBlobSync => "CandidateBlobSync",
        store::IoPoint::CandidateManifest => "CandidateManifest",
        store::IoPoint::CandidateOwner => "CandidateOwner",
        store::IoPoint::CandidateDirectorySync => "CandidateDirectorySync",
        store::IoPoint::CandidateReadback => "CandidateReadback",
        store::IoPoint::JournalWrite => "JournalWrite",
        store::IoPoint::JournalSync => "JournalSync",
        store::IoPoint::JournalRename => "JournalRename",
        store::IoPoint::JournalDirectorySync => "JournalDirectorySync",
        store::IoPoint::JournalReadback => "JournalReadback",
    })
}
fn dto_store_io_outcome(r: &store::IoOutcome) -> Value {
    json!(match r {
        store::IoOutcome::Began => "Began",
        store::IoOutcome::Completed => "Completed",
        store::IoOutcome::SimulatedFailure => "SimulatedFailure",
        store::IoOutcome::ActualFailure => "ActualFailure",
    })
}
pub fn dto_editor_actor_v0(r: &pure::EditorActorV0) -> Value {
    json!({"owner_id":r.owner_id,
"session_id":r.session_id,
"session_generation":r.session_generation,
"instance_id":r.instance_id,
"instance_generation":r.instance_generation})
}
pub fn dto_selected_artifact_v0(r: &pure::SelectedArtifactV0) -> Value {
    json!({"schema_version":r.schema_version,
"owner_id":r.owner_id,
"selected_object_id":r.selected_object_id,
"selected_generation":r.selected_generation,
"revision":r.revision,
"content_hash":hex(&r.content_hash),
"byte_len":r.byte_len,
"manifest_hash":hex(&r.manifest_hash)})
}
pub fn dto_editor_save_allocation_v0(r: &pure::EditorSaveAllocationV0) -> Value {
    json!({"schema_version":r.schema_version,
"actor":dto_editor_actor_v0(&r.actor),
"selected_object_id":r.selected_object_id,
"selected_generation":r.selected_generation,
"operation_id":r.operation_id,
"expected_revision":r.expected_revision,
"expected_content_hash":hex(&r.expected_content_hash)})
}
pub fn dto_editor_source_binding_v0(r: &pure::EditorSourceBindingV0) -> Value {
    json!({"source_object_id":r.source_object_id,
"source_generation":r.source_generation,
"byte_len":r.byte_len,
"content_hash":hex(&r.content_hash)})
}
pub fn dto_editor_save_binding_v0(r: &pure::EditorSaveBindingV0) -> Value {
    json!({"schema_version":r.schema_version,
"allocation":dto_editor_save_allocation_v0(&r.allocation),
"source":dto_editor_source_binding_v0(&r.source)})
}
pub fn dto_editor_commit_permit_v0(r: &pure::EditorCommitPermitV0) -> Value {
    json!({"schema_version":r.schema_version,
"binding":dto_editor_save_binding_v0(&r.binding),
"successor_revision":r.successor_revision,
"service_epoch":r.service_epoch,
"writer_id":r.writer_id,
"admission_epoch":r.admission_epoch})
}
pub fn dto_editor_save_receipt_v0(r: &pure::EditorSaveReceiptV0) -> Value {
    json!({"schema_version":r.schema_version,
"total_len":r.total_len,
"outcome":r.outcome,
"reserved":r.reserved,
"owner_id":r.owner_id,
"session_id":r.session_id,
"session_generation":r.session_generation,
"original_instance_id":r.original_instance_id,
"original_instance_generation":r.original_instance_generation,
"selected_object_id":r.selected_object_id,
"selected_generation":r.selected_generation,
"operation_id":r.operation_id,
"expected_revision":r.expected_revision,
"result_revision":r.result_revision,
"backend_service_epoch":r.backend_service_epoch,
"committed_at_ms":r.committed_at_ms,
"expected_content_hash":hex(&r.expected_content_hash),
"source_content_hash":hex(&r.source_content_hash)})
}
pub fn dto_editor_operation_record_v0(r: &pure::EditorOperationRecordV0) -> Value {
    json!({"allocation":dto_editor_save_allocation_v0(&r.allocation),
"allocated_service_epoch":r.allocated_service_epoch,
"binding":r.binding.as_ref().map(|v|dto_editor_save_binding_v0(&v)),
"submitted_service_epoch":r.submitted_service_epoch.as_ref().map(|v|v),
"permit":r.permit.as_ref().map(|v|dto_editor_commit_permit_v0(&v)),
"closure":r.closure,
"state":r.state,
"successor":r.successor.as_ref().map(|v|dto_selected_artifact_v0(&v)),
"receipt":r.receipt.as_ref().map(|v|dto_editor_save_receipt_v0(&v))})
}
pub fn dto_editor_selection_journal_v0(r: &pure::EditorSelectionJournalV0) -> Value {
    observed(require(r.operations.len() <= 16, "nested vector cap"));
    json!({"schema_version":r.schema_version,
"owner_id":r.owner_id,
"selected_object_id":r.selected_object_id,
"selected_generation":r.selected_generation,
"signing_policy_hash":hex(&r.signing_policy_hash),
"initial":dto_selected_artifact_v0(&r.initial),
"current":dto_selected_artifact_v0(&r.current),
"service_epoch":r.service_epoch,
"operation_high_water":r.operation_high_water,
"writer_high_water":r.writer_high_water,
"transition_sequence":r.transition_sequence,
"prior_journal_hash":hex(&r.prior_journal_hash),
"operations":r.operations.as_slice().iter().map(|v|dto_editor_operation_record_v0(&v)).collect::<Vec<_>>()})
}
pub fn dto_barrier_snapshot(r: &store::BarrierSnapshot) -> Value {
    json!({"request_id":r.request_id,
"operation_id":r.operation_id,
"entered":r.entered,
"released":r.released,
"settled":r.settled,
"permit_issued":r.permit_issued})
}
pub fn dto_store_ledger(r: &store::StoreLedger) -> Value {
    observed(require(
        r.operations_per_object.len() <= 3,
        "nested vector cap",
    ));
    observed(require(
        r.cas_entries_per_object.len() <= 3,
        "nested vector cap",
    ));
    observed(require(r.bytes_per_object.len() <= 3, "nested vector cap"));
    json!({"sessions":r.sessions,
"selected_objects":r.selected_objects,
"original_instances":r.original_instances,
"endpoints":r.endpoints,
"shared_objects":r.shared_objects,
"active_io_workers":r.active_io_workers,
"active_supervisors":r.active_supervisors,
"held_service_producers":r.held_service_producers,
"operations_per_object":r.operations_per_object,
"cas_entries_per_object":r.cas_entries_per_object,
"bytes_per_object":r.bytes_per_object})
}
pub fn dto_exchange(r: &store::Exchange) -> Value {
    json!({"actor":dto_editor_actor_v0(&r.actor),
"endpoint_class":dto_store_endpoint_class(&r.endpoint_class),
"selected_object_id":r.selected_object_id,
"request_wire":hex(&r.request_wire),
"actual_reply_wire":r.actual_reply_wire.as_ref().map(|v|hex(v)),
"reply_suppressed":r.reply_suppressed,
"operation_id":r.operation_id,
"elapsed_us":r.elapsed_us,
"observed_ms":r.observed_ms})
}
pub fn dto_io_event(r: &store::IoEvent) -> Value {
    json!({"selected_object_id":r.selected_object_id,
"operation_id":r.operation_id,
"writer_id":r.writer_id,
"sequence":r.sequence,
"point":dto_store_io_point(&r.point),
"outcome":dto_store_io_outcome(&r.outcome),
"artifact_hash":r.artifact_hash.as_ref().map(|v|hex(v))})
}
pub fn dto_object_evidence(r: &store::ObjectEvidence) -> Value {
    observed(require(r.operations.len() <= 16, "nested vector cap"));
    json!({"selected":dto_selected_artifact_v0(&r.selected),
"operations":r.operations.iter().map(|v|dto_editor_operation_record_v0(&v)).collect::<Vec<_>>(),
"mutation_dispatch_count":r.mutation_dispatch_count,
"permit_count":r.permit_count,
"transition_count":r.transition_count,
"journal_hash":hex(&r.journal_hash),
"journal_sequence":r.journal_sequence,
"blob_hash":hex(&r.blob_hash),
"manifest_hash":hex(&r.manifest_hash),
"ownership_hash":hex(&r.ownership_hash),
"mutation_quarantined":r.mutation_quarantined,
"pending_writer_id":r.pending_writer_id.as_ref().map(|v|v),
"service_epoch":r.service_epoch,
"reopen_witness":r.reopen_witness.as_ref().map(|v|dto_reopen_witness(&v))})
}
pub fn dto_reopen_witness(r: &store::ReopenWitness) -> Value {
    json!({"fence_id":r.fence_id,
"fence_snapshot_hash":hex(&r.fence_snapshot_hash),
"selected_object_id":r.selected_object_id,
"selected_generation":r.selected_generation,
"prior_service_epoch":r.prior_service_epoch,
"actual_service_epoch":r.actual_service_epoch,
"reopened_journal_sequence":r.reopened_journal_sequence,
"reopened_journal_hash":hex(&r.reopened_journal_hash)})
}
pub fn dto_producer_fence(r: &store::ProducerFence) -> Value {
    observed(require(
        !r.rust_thread_id.is_empty() && r.rust_thread_id.len() <= 64,
        "actual Rust thread id cap",
    ));
    json!({"producer_id":r.producer_id,
"producer_generation":r.producer_generation,
"kind":dto_store_producer_kind(&r.kind),
"native_pid":r.native_pid,
"rust_thread_id":r.rust_thread_id,
"selected_object_id":r.selected_object_id.as_ref().map(|v|v),
"request_id":r.request_id.as_ref().map(|v|v),
"operation_id":r.operation_id.as_ref().map(|v|v),
"completed_join":r.completed_join,
"join_outcome":dto_store_producer_join_outcome(&r.join_outcome),
"settled_barrier":r.settled_barrier.as_ref().map(|v|dto_barrier_snapshot(&v)),
"live_ownership_after_join":r.live_ownership_after_join})
}
pub fn dto_operation_closure_witness(r: &store::OperationClosureWitness) -> Value {
    json!({"allocation":dto_editor_save_allocation_v0(&r.allocation),
"submitted_binding":r.submitted_binding.as_ref().map(|v|dto_editor_save_binding_v0(&v)),
"original_request_id":r.original_request_id,
"original_producer_id":r.original_producer_id,
"original_producer_generation":r.original_producer_generation,
"closure":r.closure})
}
pub fn dto_object_fence_witness(r: &store::ObjectFenceWitness) -> Value {
    observed(require(
        r.known_durable_operations.len() <= 16,
        "nested vector cap",
    ));
    observed(require(
        r.issued_runtime_permits.len() <= 16,
        "nested vector cap",
    ));
    observed(require(
        r.original_closures.len() <= 16,
        "nested vector cap",
    ));
    json!({"owner_id":r.owner_id,
"selected_object_id":r.selected_object_id,
"selected_generation":r.selected_generation,
"service_epoch_at_fence":r.service_epoch_at_fence,
"reserved_successor_epoch":r.reserved_successor_epoch.as_ref().map(|v|v),
"durable_journal_sequence":r.durable_journal_sequence,
"durable_journal_hash":hex(&r.durable_journal_hash),
"operation_high_water_min":r.operation_high_water_min,
"writer_high_water_min":r.writer_high_water_min,
"admission_epoch_min":r.admission_epoch_min,
"known_durable_operations":r.known_durable_operations.iter().map(|v|dto_editor_operation_record_v0(&v)).collect::<Vec<_>>(),
"issued_runtime_permits":r.issued_runtime_permits.iter().map(|v|dto_editor_commit_permit_v0(&v)).collect::<Vec<_>>(),
"original_closures":r.original_closures.iter().map(|v|dto_operation_closure_witness(&v)).collect::<Vec<_>>()})
}
pub fn dto_fence_snapshot(r: &store::FenceSnapshot) -> Value {
    observed(require(
        r.service_producers.len() <= 64,
        "nested vector cap",
    ));
    observed(require(r.object_workers.len() <= 2, "nested vector cap"));
    observed(require(r.objects.len() <= 3, "nested vector cap"));
    observed(require(
        r.service_producers.len() + r.object_workers.len() <= 64,
        "combined shared64",
    ));
    json!({"schema_version":r.schema_version,
"namespace_id":r.namespace_id,
"fence_id":r.fence_id,
"profile_hash":hex(&r.profile_hash),
"semantic_clock_min":r.semantic_clock_min,
"identity_counter_min":r.identity_counter_min,
"service_producers":r.service_producers.iter().map(|v|dto_producer_fence(&v)).collect::<Vec<_>>(),
"object_workers":r.object_workers.iter().map(|v|dto_producer_fence(&v)).collect::<Vec<_>>(),
"objects":r.objects.iter().map(|v|dto_object_fence_witness(&v)).collect::<Vec<_>>(),
"live_ownership_after_join":r.live_ownership_after_join})
}
pub fn dto_native_editor_observation(r: &desktop::NativeEditorObservation) -> Value {
    observed(require(r.bytes.len() <= 4096, "body cap"));
    json!({"actor":dto_editor_actor_v0(&r.actor),
"document_id":r.document_id,
"bytes":hex(&r.bytes),
"cursor":r.cursor,
"selection":r.selection.as_ref().map(|v|v),
"first_visible_line":r.first_visible_line,
"text_generation":r.text_generation,
"view_version":r.view_version,
"confirmed_revision":r.confirmed_revision,
"confirmed_hash":hex(&r.confirmed_hash),
"phase":dto_native_editor_phase(&r.phase)})
}
pub fn dto_native_frame_observation(r: &desktop::NativeFrameObservation) -> Value {
    json!({"actor":dto_editor_actor_v0(&r.actor),
"document_id":r.document_id,
"text_generation":r.text_generation,
"view_version":r.view_version,
"surface_id":r.surface_id,
"surface_generation":r.surface_generation,
"mapping_generation":r.mapping_generation,
"focus_epoch":r.focus_epoch,
"sequence":r.sequence,
"app_crop_sha256":hex(&r.app_crop_sha256)})
}
pub fn dto_native_save_chrome_observation(r: &desktop::NativeSaveChromeObservation) -> Value {
    observed(require(
        r.pending_originals.len() <= 48,
        "nested vector cap",
    ));
    json!({"current_frame_actor":dto_editor_actor_v0(&r.current_frame_actor),
"original_actor":r.original_actor.as_ref().map(|v|dto_editor_actor_v0(&v)),
"document_id":r.document_id.as_ref().map(|v|v),
"text_generation":r.text_generation.as_ref().map(|v|v),
"view_version":r.view_version.as_ref().map(|v|v),
"frame_sequence":r.frame_sequence,
"focus_epoch":r.focus_epoch,
"attachment_version":r.attachment_version,
"status_version":r.status_version,
"label":dto_native_editor_phase(&r.label),
"current_document_phase":dto_native_editor_phase(&r.current_document_phase),
"outstanding_original_count":r.outstanding_original_count,
"pending_originals":r.pending_originals.iter().map(|v|dto_native_pending_original_observation(&v)).collect::<Vec<_>>(),
"record248":r.record248.as_ref().map(|v|hex(v))})
}
pub fn dto_native_pending_original_observation(
    r: &desktop::NativePendingOriginalObservation,
) -> Value {
    json!({"original_actor":dto_editor_actor_v0(&r.original_actor),
"object_id":r.object_id,
"object_generation":r.object_generation,
"intent_id":r.intent_id,
"operation_id":r.operation_id.as_ref().map(|v|v),
"original_backend_epoch":r.original_backend_epoch,
"phase":dto_native_pending_original_phase(&r.phase)})
}
pub fn dto_native_join_observation(r: &desktop::NativeJoinObservation) -> Value {
    observed(require(
        !r.rust_thread_id.is_empty() && r.rust_thread_id.len() <= 64,
        "actual Rust thread id cap",
    ));
    json!({"producer_id":r.producer_id,
"producer_generation":r.producer_generation,
"origin_id":r.origin_id,
"kind":dto_desktop_producer_kind(&r.kind),
"native_pid":r.native_pid,
"rust_thread_id":r.rust_thread_id,
"outcome":dto_desktop_join_outcome(&r.outcome)})
}
pub fn dto_native_save_fence_snapshot(r: &store::NativeSaveFenceSnapshot) -> Value {
    observed(require(r.original_tickets.len() <= 96, "nested vector cap"));
    json!({"schema_version":r.schema_version,
"base":dto_fence_snapshot(&r.base),
"original_tickets":r.original_tickets.iter().map(|v|dto_original_ticket_observation(&v)).collect::<Vec<_>>()})
}
pub fn dto_original_ticket_observation(r: &store::OriginalTicketObservation) -> Value {
    json!({"original_request":hex(&r.original_request),
"origin_id":r.origin_id,
"intent_id":r.intent_id,
"kind":dto_original_ticket_kind(&r.kind),
"actor":dto_editor_actor_v0(&r.actor),
"object_id":r.object_id,
"original_epoch":r.original_epoch,
"allocation":r.allocation.as_ref().map(|v|dto_editor_save_allocation_v0(&v)),
"binding":r.binding.as_ref().map(|v|dto_editor_save_binding_v0(&v)),
"closed":r.closed,
"commit_dispatched":r.commit_dispatched,
"completion":dto_original_ticket_completion(&r.completion)})
}
pub fn dto_resource_ledger(r: &desktop::ResourceLedger) -> Value {
    observed(require(
        r.operations_per_object.len() <= 3,
        "nested vector cap",
    ));
    json!({"sessions":r.sessions,
"selected_objects":r.selected_objects,
"instances":r.instances,
"endpoints":r.endpoints,
"shared_objects":r.shared_objects,
"queued_keys":r.queued_keys,
"operations_per_object":r.operations_per_object})
}
pub fn dto_producer_counts(r: &desktop::ProducerCounts) -> Value {
    json!({"held":r.held,
"io":r.io,
"joining":r.joining,
"joined_total":r.joined_total})
}

const MAX_ALL: u64 = 2 * 1024 * 1024 * 1024;
const MAX_JSON: u64 = 512 * 1024 * 1024;
const MAX_CASE_JSON: u64 = 128 * 1024 * 1024;
const MAX_FILE: u64 = 32 * 1024 * 1024;
const MAX_FILES: usize = 40960;
const MAX_ENTRIES: usize = 41115;
#[cfg(target_os = "linux")]
const NOFOLLOW: i32 = 0x20000;
#[cfg(target_os = "macos")]
const NOFOLLOW: i32 = 0x100;
#[derive(Clone, Copy)]
pub enum ScenarioScope {
    Composition,
    DirectNative,
    VolatileControl,
    NativeReadControl,
}
impl ScenarioScope {
    fn name(self) -> &'static str {
        match self {
            Self::Composition => "composition",
            Self::DirectNative => "direct-native",
            Self::VolatileControl => "volatile-control",
            Self::NativeReadControl => "native-read-control",
        }
    }
}
pub struct ScenarioSpec {
    pub case: &'static str,
    pub scenario: &'static str,
    pub leg: u32,
    pub scope: ScenarioScope,
    pub checkpoints: &'static [&'static str],
    pub requirements: &'static [&'static str],
}
pub struct CaseRecorder {
    root: PathBuf,
    held_root: File,
    case: &'static str,
    nonce: String,
}
pub struct ScenarioRecorder {
    case: Arc<CaseRecorder>,
    spec: ScenarioSpec,
    path: PathBuf,
    held: File,
    raw: Vec<Value>,
    records: Vec<String>,
    snapshots: Vec<Value>,
    epochs: Vec<u32>,
    frames: Vec<Value>,
    drafts: Vec<Value>,
    grants: Vec<Value>,
    counts: Vec<Value>,
    controls: Vec<Value>,
    negative: Vec<Value>,
    native_joins: Vec<Value>,
    external_joins: Vec<Value>,
    ticket_batches: Vec<Value>,
    capture_batches: Vec<Value>,
    known_files: BTreeSet<String>,
    applied_storage_faults: BTreeMap<String, store::StorageFault>,
}
#[derive(Default)]
struct Totals {
    all: u64,
    json: u64,
    case_json: u64,
    files: usize,
    entries: usize,
}
fn name(v: &str) -> Result<()> {
    require(
        !v.is_empty()
            && v.len() <= 96
            && v.bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_' || c == b'-'),
        "closed source name",
    )
}
fn basename(v: &str) -> Result<()> {
    require(
        !v.is_empty()
            && v.len() <= 128
            && v.bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-' || c == b'.')
            && !v.contains(".."),
        "recorder basename",
    )
}
fn identity(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    (a.dev(), a.ino(), a.mode(), a.uid(), a.gid()) == (b.dev(), b.ino(), b.mode(), b.uid(), b.gid())
}
fn held_directory(path: &Path) -> Result<File> {
    require(path.is_absolute(), "absolute owned recording root")?;
    let mut current = PathBuf::from("/");
    for component in path.components().skip(1) {
        current.push(component);
        let m = fs::symlink_metadata(&current).map_err(|_| RecordingError("directory metadata"))?;
        require(
            m.is_dir() && !m.file_type().is_symlink(),
            "nonsymlink ancestor",
        )?;
    }
    let before =
        fs::symlink_metadata(path).map_err(|_| RecordingError("owned directory metadata"))?;
    require(before.mode() & 0o022 == 0, "private owned output directory")?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(NOFOLLOW)
        .open(path)
        .map_err(|_| RecordingError("held output directory"))?;
    require(
        identity(
            &before,
            &file
                .metadata()
                .map_err(|_| RecordingError("held directory stat"))?,
        ),
        "stable directory identity",
    )?;
    Ok(file)
}
fn check_directory(path: &Path, held: &File) -> Result<()> {
    let now = fs::symlink_metadata(path).map_err(|_| RecordingError("output directory changed"))?;
    require(
        now.is_dir()
            && !now.file_type().is_symlink()
            && identity(
                &now,
                &held
                    .metadata()
                    .map_err(|_| RecordingError("held directory changed"))?,
            ),
        "output directory identity",
    )
}
fn scan(path: &Path, case: &Path, depth: usize, t: &mut Totals) -> Result<()> {
    require(depth <= 8, "export depth")?;
    let entries = fs::read_dir(path).map_err(|_| RecordingError("owned inventory scan"))?;
    for row in entries {
        let row = row.map_err(|_| RecordingError("inventory entry"))?;
        t.entries = t
            .entries
            .checked_add(1)
            .ok_or(RecordingError("entry overflow"))?;
        require(t.entries <= MAX_ENTRIES, "export entry cap")?;
        let p = row.path();
        let m = fs::symlink_metadata(&p).map_err(|_| RecordingError("inventory metadata"))?;
        require(!m.file_type().is_symlink(), "export symlink")?;
        if m.is_dir() {
            scan(&p, case, depth + 1, t)?;
        } else {
            require(
                m.is_file() && m.nlink() == 1,
                "export regular single-link file",
            )?;
            t.files = t
                .files
                .checked_add(1)
                .ok_or(RecordingError("file count overflow"))?;
            require(t.files <= MAX_FILES, "export file cap")?;
            require(m.len() <= MAX_FILE, "export individual cap")?;
            t.all = t
                .all
                .checked_add(m.len())
                .ok_or(RecordingError("export byte overflow"))?;
            if p.extension().is_some_and(|v| v == "json") {
                t.json = t
                    .json
                    .checked_add(m.len())
                    .ok_or(RecordingError("JSON byte overflow"))?;
                if p.starts_with(case) {
                    t.case_json = t
                        .case_json
                        .checked_add(m.len())
                        .ok_or(RecordingError("case JSON overflow"))?;
                }
            }
            require(
                t.all <= MAX_ALL && t.json <= MAX_JSON && t.case_json <= MAX_CASE_JSON,
                "aggregate export cap",
            )?;
        }
    }
    Ok(())
}
struct CountWriter {
    bytes: u64,
}
impl Write for CountWriter {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        let next = self
            .bytes
            .checked_add(b.len() as u64)
            .ok_or_else(|| io::Error::other("record length overflow"))?;
        if next > MAX_FILE {
            return Err(io::Error::other("record32MiB"));
        }
        self.bytes = next;
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
struct ReservedWriter {
    file: File,
    left: u64,
    hash: Sha256,
}
impl Write for ReservedWriter {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if b.len() as u64 > self.left {
            return Err(io::Error::other("pre-growth reservation exceeded"));
        }
        let n = self.file.write(b)?;
        self.left -= n as u64;
        self.hash.update(&b[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
fn fresh_directory(root: &Path, held: &File, path: &Path) -> Result<()> {
    check_directory(root, held)?;
    require(
        path.starts_with(root) && path != root,
        "owned child directory",
    )?;
    let marker = root.join(".recording-reservation");
    let reservation = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(NOFOLLOW)
        .open(&marker)
        .map_err(|_| RecordingError("exclusive directory reservation"))?;
    let mut t = Totals::default();
    scan(root, path, 0, &mut t)?;
    require(t.entries < MAX_ENTRIES, "directory entry cap before growth")?;
    fs::DirBuilder::new()
        .mode(0o700)
        .create(path)
        .map_err(|_| RecordingError("fresh owned directory"))?;
    let actual = held_directory(path)?;
    check_directory(path, &actual)?;
    drop(actual);
    drop(reservation);
    fs::remove_file(marker).map_err(|_| RecordingError("settled directory reservation"))?;
    Ok(())
}
impl CaseRecorder {
    pub fn open(case: &'static str) -> Result<Arc<Self>> {
        name(case)?;
        let root = PathBuf::from(
            std::env::var_os("RAMEN_NATIVE_TASK_EVIDENCE")
                .ok_or(RecordingError("missing actual evidence destination"))?,
        );
        let held_root = held_directory(&root)?;
        let nonce = std::env::var("RAMEN_NATIVE_TASK_RUN_NONCE")
            .map_err(|_| RecordingError("missing actual run nonce"))?;
        require(
            nonce.len() == 32
                && nonce
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "root selected nonce",
        )?;
        let p = root.join(case);
        fresh_directory(&root, &held_root, &p)?;
        Ok(Arc::new(Self {
            root,
            held_root,
            case,
            nonce,
        }))
    }
    pub fn scenario(self: &Arc<Self>, spec: ScenarioSpec) -> Result<ScenarioRecorder> {
        require(spec.case == self.case, "source case match")?;
        name(spec.scenario)?;
        require(spec.checkpoints.len() <= 128, "source checkpoint cap")?;
        for v in spec.checkpoints {
            name(v)?;
        }
        check_directory(&self.root, &self.held_root)?;
        let parent = self.root.join(self.case).join(spec.scenario);
        if !parent.exists() {
            fresh_directory(&self.root, &self.held_root, &parent)?;
        }
        let path = parent.join(spec.leg.to_string());
        require(
            path.strip_prefix(&self.root)
                .map_err(|_| RecordingError("relative output"))?
                .as_os_str()
                .len()
                <= 256,
            "relative output bytes",
        )?;
        fresh_directory(&self.root, &self.held_root, &path)?;
        let held = held_directory(&path)?;
        Ok(ScenarioRecorder {
            case: self.clone(),
            spec,
            path,
            held,
            raw: vec![],
            records: vec![],
            snapshots: vec![],
            epochs: vec![],
            frames: vec![],
            drafts: vec![],
            grants: vec![],
            counts: vec![],
            controls: vec![],
            negative: vec![],
            native_joins: vec![],
            external_joins: vec![],
            ticket_batches: vec![],
            capture_batches: vec![],
            known_files: BTreeSet::new(),
            applied_storage_faults: BTreeMap::new(),
        })
    }
}
impl ScenarioRecorder {
    pub fn unit_control(
        &mut self,
        purpose: &'static str,
        call: &'static str,
        result: &'static str,
        space: &'static str,
        status: u32,
        request: Option<&kernel_api::ipc::Envelope>,
    ) -> Result<()> {
        self.purpose(purpose)?;
        require(
            self.controls.len() < 1024
                && call.len() <= 96
                && matches!(result, "Ok" | "Err")
                && matches!(space, "DesktopStatus" | "StoreStatus")
                && ((result == "Ok" && status == 0)
                    || (result == "Err" && status > 0 && status <= 11)),
            "closed actual unit result",
        )?;
        let request = request
            .map(|q| desktop::encode_envelope_wire(q).map(|wire| hex(&wire)))
            .transpose()
            .map_err(|_| RecordingError("actual control canonical request"))?;
        let n = u32::try_from(self.controls.len())
            .map_err(|_| RecordingError("checked local observation ordinal"))?;
        self.controls.push(json!({"observation_index":n,"purpose":purpose,"call":call,"kind":"unit_result","actual_status":{"result":result,"status_space":space,"status":status},"request":request,"init":Value::Null,"expose":Value::Null,"delivery":Value::Null,"inactive_desktop":Value::Null,"inactive_store":Value::Null,"registration":Value::Null}));
        Ok(())
    }
    fn typed_control(
        &mut self,
        purpose: &'static str,
        call: &'static str,
        kind: &'static str,
        space: &'static str,
        status: u32,
        payload: Value,
        request: Option<&kernel_api::ipc::Envelope>,
    ) -> Result<()> {
        self.purpose(purpose)?;
        require(
            self.controls.len() < 1024
                && call.len() <= 96
                && matches!(
                    kind,
                    "native_save_expose"
                        | "inactive_save_desktop"
                        | "inactive_save_store"
                        | "native_save_registration"
                )
                && matches!(space, "DesktopStatus" | "StoreStatus")
                && status <= 11,
            "closed typed control",
        )?;
        let request = request
            .map(|q| desktop::encode_envelope_wire(q).map(|wire| hex(&wire)))
            .transpose()
            .map_err(|_| RecordingError("actual control request"))?;
        let n = u32::try_from(self.controls.len())
            .map_err(|_| RecordingError("checked local observation ordinal"))?;
        let mut row = json!({"observation_index":n,"purpose":purpose,"call":call,"kind":kind,
            "actual_status":{"result":if status == 0 {"Ok"} else {"Err"},"status_space":space,"status":status},
            "request":request,"init":Value::Null,"expose":Value::Null,"delivery":Value::Null,
            "inactive_desktop":Value::Null,"inactive_store":Value::Null,"registration":Value::Null});
        let slot = match kind {
            "native_save_expose" => "expose",
            "inactive_save_desktop" => "inactive_desktop",
            "inactive_save_store" => "inactive_store",
            "native_save_registration" => "registration",
            _ => unreachable!(),
        };
        row[slot] = payload;
        self.controls.push(row);
        Ok(())
    }
    pub fn inactive_desktop(
        &mut self,
        purpose: &'static str,
        r: &desktop::InactiveSaveDesktopObservation,
    ) -> Result<()> {
        fn unit(r: &std::result::Result<(), desktop_service::dev::Status>) -> Value {
            let status = match r {
                Ok(()) => 0,
                Err(s) => *s as u32,
            };
            json!({"result":if status==0 {"Ok"} else {"Err"},"status_space":"DesktopStatus","status":status})
        }
        let requests = r
            .endpoint_requests
            .iter()
            .map(|q| desktop::encode_envelope_wire(q).map(|w| hex(&w)))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|_| RecordingError("actual inactive requests"))?;
        let replies = r
            .endpoint_replies
            .iter()
            .map(|q| desktop::encode_envelope_wire(q).map(|w| hex(&w)))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|_| RecordingError("actual inactive replies"))?;
        self.typed_control(
            purpose,
            "probe_inactive_save",
            "inactive_save_desktop",
            "DesktopStatus",
            0,
            json!({"endpoint_requests":requests,"endpoint_replies":replies,
                "origin_status":unit(&r.origin_status),"bootstrap_status":unit(&r.bootstrap_status),
                "grants_lease_status":unit(&r.grants_lease_status)}),
            None,
        )
    }
    pub fn inactive_store(
        &mut self,
        purpose: &'static str,
        r: &store::InactiveSaveStoreObservation,
    ) -> Result<()> {
        let status = match &r.grant_status {
            Ok(()) => 0,
            Err(s) => *s as u32,
        };
        self.typed_control(purpose,"probe_inactive_native_save","inactive_save_store","StoreStatus",0,
            json!({"grant_status":{"result":if status==0 {"Ok"} else {"Err"},"status_space":"StoreStatus","status":status}}),None)
    }
    pub fn expose_failure(
        &mut self,
        purpose: &'static str,
        call: &'static str,
        r: &store::NativeSaveExposeFailure,
    ) -> Result<()> {
        let o = r.observation();
        let stage = match o.stage {
            store::PreviewExposeStage::Validate => "Validate",
            store::PreviewExposeStage::ReserveStoreGrant => "ReserveStoreGrant",
            store::PreviewExposeStage::PrepareRecord => "PrepareRecord",
            store::PreviewExposeStage::Activate => "Activate",
            store::PreviewExposeStage::Deliver => "Deliver",
        };
        require(o.status == r.status(), "actual expose status identity")?;
        self.typed_control(
            purpose,
            call,
            "native_save_expose",
            "StoreStatus",
            r.status() as u32,
            json!({"stage":stage,"status":o.status as u32,"pending_retained":o.pending_retained,
                "inactive_store_grant_retained":o.inactive_store_grant_retained,"active":o.active}),
            None,
        )
    }
    pub fn registration_failure(
        &mut self,
        purpose: &'static str,
        r: &store::NativeSaveRegistrationFailure,
        joined_ids: &[u64],
    ) -> Result<()> {
        require(
            r.producers().len() <= 2
                && joined_ids.len() == r.producers().len()
                && joined_ids.iter().all(|id| *id > 0)
                && joined_ids
                    .iter()
                    .enumerate()
                    .all(|(i, id)| !joined_ids[..i].contains(id)),
            "actual registration joined producer bound",
        )?;
        let request = r.call().request();
        let wire = desktop::encode_envelope_wire(request)
            .map_err(|_| RecordingError("actual registration request"))?;
        self.typed_control(
            purpose,
            "register_native_save",
            "native_save_registration",
            "StoreStatus",
            r.status() as u32,
            json!({"original_retained":r.original().is_some(),"producer_ids":joined_ids,
                "request":hex(&wire)}),
            Some(request),
        )
    }
    pub fn finish_volatile(mut self, ledger: &desktop::ResourceLedger) -> Result<()> {
        require(
            matches!(self.spec.scope, ScenarioScope::VolatileControl),
            "actual volatile cleanup scope",
        )?;
        let cleanup = json!({"binding":self.binding(0),"actual_controller_final_ledger":dto_resource_ledger(ledger),"external_callers":self.external_joins,"consumer_holders_dropped":true,"root_identity":Value::Null,"store_fence":Value::Null,"native_process_cleanup_claim":false});
        self.final_records(cleanup, 0, false)
    }
    pub fn finish_native_read(
        mut self,
        root: tempfile::TempDir,
        held: u32,
        joins: &[Value],
    ) -> Result<()> {
        require(
            matches!(self.spec.scope, ScenarioScope::NativeReadControl)
                && held == 0
                && joins.len() <= 64,
            "actual old NativeRead cleanup scope",
        )?;
        for r in joins {
            require(
                r.as_object().is_some_and(|o| {
                    o.len() == 2 && o.contains_key("id") && o.contains_key("outcome")
                }) && r["id"].as_u64().is_some_and(|v| v > 0)
                    && r["outcome"].as_u64() == Some(1),
                "actual old Rig join diagnostic shape",
            )?;
        }
        let identity = root_identity(root.path())?;
        let path = root.path().to_owned();
        root.close()
            .map_err(|_| RecordingError("actual NativeRead TempDir close"))?;
        require(
            fs::symlink_metadata(&path).is_err_and(|e| e.kind() == io::ErrorKind::NotFound),
            "actual NativeRead root absence",
        )?;
        let cleanup = json!({"binding":self.binding(0),"root_identity":identity,"owner_relative":"owner","held_after_join":held,"external_callers":self.external_joins,"root_close_result":"Ok","root_absence_result":"NotFound","store_fence":Value::Null,"actual_join_records":joins});
        self.final_records(cleanup, held, false)
    }
    fn binding(&self, epoch: u32) -> Value {
        json!({"schema_version":1,"run_nonce":self.case.nonce,"case":self.case.case,"scenario":format!("{}/{}",self.spec.scenario,self.spec.leg),"scope":self.spec.scope.name(),"epoch_index":epoch})
    }
    fn purpose(&self, v: &str) -> Result<()> {
        require(
            self.spec.checkpoints.contains(&v),
            "source-declared observation purpose",
        )
    }
    fn write<F>(&mut self, file: &str, len: u64, is_json: bool, f: F) -> Result<String>
    where
        F: FnOnce(&mut ReservedWriter) -> io::Result<()>,
    {
        basename(file)?;
        require(len <= MAX_FILE, "individual output cap")?;
        require(
            self.records.len() + self.raw.len() < 128 + 2048,
            "scenario output inventory cap",
        )?;
        check_directory(&self.case.root, &self.case.held_root)?;
        check_directory(&self.path, &self.held)?;
        let marker = self.case.root.join(".recording-reservation");
        let mut reservation = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(NOFOLLOW)
            .open(&marker)
            .map_err(|_| RecordingError("exclusive export reservation or abandoned writer"))?;
        reservation
            .write_all(len.to_string().as_bytes())
            .map_err(|_| RecordingError("reservation write"))?;
        let mut totals = Totals::default();
        scan(
            &self.case.root,
            &self.case.root.join(self.case.case),
            0,
            &mut totals,
        )?;
        require(
            totals.files < MAX_FILES && totals.entries < MAX_ENTRIES,
            "next export file count",
        )?;
        require(
            totals.all.checked_add(len).is_some_and(|v| v <= MAX_ALL),
            "next aggregate bytes",
        )?;
        if is_json {
            require(
                totals.json.checked_add(len).is_some_and(|v| v <= MAX_JSON)
                    && totals
                        .case_json
                        .checked_add(len)
                        .is_some_and(|v| v <= MAX_CASE_JSON),
                "next JSON bytes",
            )?;
        }
        let p = self.path.join(file);
        let opened = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(NOFOLLOW)
            .open(&p)
            .map_err(|_| RecordingError("fresh export file"))?;
        let before = opened
            .metadata()
            .map_err(|_| RecordingError("opened export stat"))?;
        require(
            before.is_file() && before.nlink() == 1,
            "owned regular export",
        )?;
        let mut writer = ReservedWriter {
            file: opened,
            left: len,
            hash: Sha256::new(),
        };
        f(&mut writer).map_err(|_| RecordingError("bounded export write"))?;
        writer.flush().map_err(|_| RecordingError("export flush"))?;
        require(writer.left == 0, "complete reserved export")?;
        let after = writer
            .file
            .metadata()
            .map_err(|_| RecordingError("export final stat"))?;
        require(
            identity(&before, &after) && after.len() == len,
            "stable complete export",
        )?;
        let actual_hash = hex(&writer.hash.finalize());
        drop(writer.file);
        check_directory(&self.path, &self.held)?;
        let final_stat =
            fs::symlink_metadata(&p).map_err(|_| RecordingError("final export path stat"))?;
        require(
            identity(&after, &final_stat) && final_stat.len() == len,
            "final path identity",
        )?;
        drop(reservation);
        fs::remove_file(marker).map_err(|_| RecordingError("settled reservation removal"))?;
        Ok(actual_hash)
    }
    // Fixed frame plus optional chrome outputs share admission, not completion.
    fn frame_raw_pair(
        &mut self,
        first: (&str, &str, &[u8]),
        second: Option<(&str, &str, &[u8])>,
    ) -> Result<Value> {
        require(
            first.1 == "composed_frame" && first.2.len() == 1454080,
            "fixed composed frame output",
        )?;
        basename(first.0)?;
        if let Some((file, kind, bytes)) = second {
            basename(file)?;
            require(
                kind == "chrome248" && bytes.len() == 248 && file != first.0,
                "fixed distinct optional chrome output",
            )?;
        }
        let count = 1 + usize::from(second.is_some());
        require(
            self.raw.len().checked_add(count).is_some_and(|v| v <= 2048)
                && self
                    .records
                    .len()
                    .checked_add(self.raw.len())
                    .and_then(|v| v.checked_add(count))
                    .is_some_and(|v| v <= 128 + 2048),
            "combined frame raw inventory cap",
        )?;
        let len = (first.2.len() as u64)
            .checked_add(second.map_or(0, |v| v.2.len() as u64))
            .ok_or(RecordingError("frame reservation length overflow"))?;
        check_directory(&self.case.root, &self.case.held_root)?;
        check_directory(&self.path, &self.held)?;
        let marker = self.case.root.join(".recording-reservation");
        let mut reservation = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(NOFOLLOW)
            .open(&marker)
            .map_err(|_| RecordingError("exclusive frame reservation or abandoned writer"))?;
        reservation
            .write_all(len.to_string().as_bytes())
            .map_err(|_| RecordingError("frame reservation write"))?;
        for (file, _, _) in [Some(first), second].into_iter().flatten() {
            match fs::symlink_metadata(self.path.join(file)) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => (),
                _ => return Err(RecordingError("fresh frame target preflight")),
            }
        }
        let mut totals = Totals::default();
        scan(
            &self.case.root,
            &self.case.root.join(self.case.case),
            0,
            &mut totals,
        )?;
        require(
            totals
                .files
                .checked_add(count)
                .is_some_and(|v| v <= MAX_FILES)
                && totals
                    .entries
                    .checked_add(count)
                    .is_some_and(|v| v <= MAX_ENTRIES)
                && totals.all.checked_add(len).is_some_and(|v| v <= MAX_ALL),
            "combined frame pre-growth reservation",
        )?;
        let mut first_record = None;
        for (file, kind, bytes) in [Some(first), second].into_iter().flatten() {
            check_directory(&self.case.root, &self.case.held_root)?;
            check_directory(&self.path, &self.held)?;
            let p = self.path.join(file);
            let opened = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(NOFOLLOW)
                .open(&p)
                .map_err(|_| RecordingError("fresh frame export file"))?;
            let before = opened
                .metadata()
                .map_err(|_| RecordingError("opened frame export stat"))?;
            require(
                before.is_file() && before.nlink() == 1,
                "owned regular frame export",
            )?;
            let mut writer = ReservedWriter {
                file: opened,
                left: bytes.len() as u64,
                hash: Sha256::new(),
            };
            writer
                .write_all(bytes)
                .map_err(|_| RecordingError("bounded frame export write"))?;
            writer
                .flush()
                .map_err(|_| RecordingError("frame export flush"))?;
            require(writer.left == 0, "complete reserved frame export")?;
            let after = writer
                .file
                .metadata()
                .map_err(|_| RecordingError("frame export final stat"))?;
            require(
                after.is_file()
                    && after.nlink() == 1
                    && identity(&before, &after)
                    && after.len() == bytes.len() as u64,
                "stable single-link complete frame export",
            )?;
            let actual_hash = hex(&writer.hash.finalize());
            drop(writer.file);
            check_directory(&self.case.root, &self.case.held_root)?;
            check_directory(&self.path, &self.held)?;
            let final_stat = fs::symlink_metadata(&p)
                .map_err(|_| RecordingError("final frame export path stat"))?;
            require(
                final_stat.is_file()
                    && final_stat.nlink() == 1
                    && identity(&after, &final_stat)
                    && final_stat.len() == bytes.len() as u64,
                "final single-link frame path identity",
            )?;
            let record =
                json!({"file":file,"kind":kind,"byte_len":bytes.len(),"sha256":actual_hash});
            self.raw.push(record.clone());
            if first_record.is_none() {
                first_record = Some(record);
            }
        }
        check_directory(&self.case.root, &self.case.held_root)?;
        check_directory(&self.path, &self.held)?;
        drop(reservation);
        fs::remove_file(marker).map_err(|_| RecordingError("settled frame reservation removal"))?;
        first_record.ok_or(RecordingError("completed frame output"))
    }
    pub fn raw_bytes(&mut self, file: &str, kind: &str, bytes: &[u8]) -> Result<Value> {
        let (lo, hi) = match kind {
            "text64" | "text64_negative" => (64, 4160),
            "receipt176" => (176, 176),
            "wire88" | "wire88_negative" => (88, 88),
            "grant464" => (464, 464),
            "journal" => (1, 131072),
            "corrupt_journal" | "fixture_file" => (0, 131072),
            "cas_blob" => (0, 4096),
            "app_frame" => (1228800, 1228800),
            "composed_frame" => (1454080, 1454080),
            "chrome248" => (248, 248),
            _ => return Err(RecordingError("closed raw kind")),
        };
        require((lo..=hi).contains(&bytes.len()), "narrow raw kind bound")?;
        require(self.raw.len() < 2048, "raw inventory cap")?;
        let hash = self.write(file, bytes.len() as u64, false, |w| w.write_all(bytes))?;
        let record = json!({"file":file,"kind":kind,"byte_len":bytes.len(),"sha256":hash});
        self.raw.push(record.clone());
        Ok(record)
    }
    fn record(&mut self, file: &str, v: &Value) -> Result<()> {
        require(self.records.len() < 128, "record inventory cap")?;
        let mut count = CountWriter { bytes: 0 };
        serde_json::to_writer(&mut count, v)
            .map_err(|_| RecordingError("bounded JSON measurement"))?;
        self.write(file, count.bytes, true, |w| {
            serde_json::to_writer(w, v).map_err(io::Error::other)
        })?;
        self.records.push(file.to_owned());
        Ok(())
    }
}

impl ScenarioRecorder {
    pub fn desktop(&mut self, e: &desktop::Evidence) -> Result<()> {
        require(e.exchanges.len() <= 131072, "Desktop complete trace cap")?;
        let mut rows = Vec::with_capacity(e.exchanges.len());
        let malformed_case = self.spec.case == "ui1_1_ascii_bounds_preserve_prior_draft"
            && self.spec.scenario == "primary"
            && self.spec.leg == 0;
        let mut malformed_count = 0;
        for (q, r) in &e.exchanges {
            let wire = match desktop::encode_envelope_wire(q) {
                Ok(wire) => wire,
                Err(desktop_service::dev::Status::Invalid)
                    if malformed_case
                        && q.protocol == 802
                        && q.msg_type == 3
                        && q.payload_len == 48
                        && q.payload[44..48] == 1_u32.to_le_bytes() =>
                {
                    let mut canonical = *q;
                    canonical.payload[44..48].fill(0);
                    let mut wire = desktop::encode_envelope_wire(&canonical)
                        .map_err(|_| RecordingError("otherwise canonical malformed input"))?;
                    // Preserve the actual rejected reserved bytes, never a repaired request.
                    wire[64..68].copy_from_slice(&q.payload[44..48]);
                    malformed_count += 1;
                    require(malformed_count == 1, "one source-fixed malformed input")?;
                    wire
                }
                Err(_) => return Err(RecordingError("actual canonical Desktop request")),
            };
            rows.push(json!({"request":hex(&wire),"reply":hex(&desktop::encode_envelope_wire(r).map_err(|_|RecordingError("actual canonical Desktop reply"))?)}));
        }
        require(
            malformed_count == usize::from(malformed_case),
            "complete source-fixed malformed input trace",
        )?;
        let v = json!({"binding":self.binding(0),"exchanges":rows,"ledger":dto_resource_ledger(&e.ledger),"permit_count":e.permit_count,"transition_count":e.transition_count,"mutation_dispatch_count":e.mutation_dispatch_count,"volatile_backend":e.volatile_backend});
        self.record("desktop.json", &v)
    }
    pub fn producer_counts(
        &mut self,
        purpose: &'static str,
        c: &desktop::ProducerCounts,
    ) -> Result<()> {
        self.purpose(purpose)?;
        require(self.counts.len() < 512, "source producer observation cap")?;
        require(
            c.held <= 64 && c.io <= 2 && c.joining <= c.held,
            "actual shared roster bounds",
        )?;
        let n = u32::try_from(self.counts.len())
            .map_err(|_| RecordingError("checked local observation ordinal"))?;
        self.counts
            .push(json!({"observation_index":n,"purpose":purpose,"counts":dto_producer_counts(c)}));
        Ok(())
    }
    pub fn draft(&mut self, o: &desktop::NativeEditorObservation) -> Result<()> {
        require(self.drafts.len() < 512, "meaningful draft observation cap")?;
        self.drafts.push(dto_native_editor_observation(o));
        Ok(())
    }
    pub fn captures(
        &mut self,
        epoch: u32,
        rows: &[store::NativeSaveArtifactCapture],
    ) -> Result<()> {
        require(
            rows.len() <= 256 && self.capture_batches.len() < 4,
            "complete actual Core capture caps",
        )?;
        require(
            !self
                .capture_batches
                .iter()
                .any(|v| v["epoch_index"].as_u64() == Some(epoch as u64)),
            "one complete capture batch per actual Core",
        )?;
        let mut captures = Vec::with_capacity(rows.len());
        let mut prior = 0;
        for r in rows {
            require(r.capture_id > prior, "actual monotonic capture IDs")?;
            prior = r.capture_id;
            require(
                r.descriptor.byte_len as usize == r.bytes.len(),
                "complete actual capture bytes",
            )?;
            let (kind, raw_kind) = match r.kind {
                store::NativeSaveArtifactKind::SelectedTextCopy => ("SelectedTextCopy", "text64"),
                store::NativeSaveArtifactKind::DraftSourceFreeze => ("DraftSourceFreeze", "text64"),
                store::NativeSaveArtifactKind::ReceiptCopy => ("ReceiptCopy", "receipt176"),
            };
            let raw = self.raw_bytes(
                &format!("capture-{epoch}-{}.bin", r.capture_id),
                raw_kind,
                &r.bytes,
            )?;
            captures.push(json!({"capture_id":r.capture_id,"kind":kind,"actor":dto_editor_actor_v0(&r.actor),"object_id":r.object_id,"object_generation":r.object_generation,"original_backend_epoch":r.original_backend_epoch,"original_request":hex(&r.original_request),"original_reply":r.original_reply.as_ref().map(|v|hex(v)),"capture_request":hex(&r.capture_request),"capture_reply":r.capture_reply.as_ref().map(|v|hex(v)),"operation_id":r.operation_id,"intent_id":r.intent_id,"descriptor":descriptor(&r.descriptor),"raw":raw}));
        }
        self.capture_batches
            .push(json!({"epoch_index":epoch,"captures":captures}));
        Ok(())
    }
    pub fn frame(
        &mut self,
        frame: &desktop::ComposedFrame,
        draft: Option<&desktop::NativeEditorObservation>,
        join: Option<&desktop::NativeFrameObservation>,
        chrome: Option<&desktop::NativeSaveChromeObservation>,
    ) -> Result<()> {
        require(
            self.frames.len() < 128 && frame.bgra.len() == 1454080,
            "actual full frame bound",
        )?;
        let n = u32::try_from(self.frames.len())
            .map_err(|_| RecordingError("checked local observation ordinal"))?;
        let frame_file = format!("frame-{n}.bgra");
        let chrome_file = format!("chrome-{n}.bin");
        let raw = self.frame_raw_pair(
            (&frame_file, "composed_frame", &frame.bgra),
            chrome
                .and_then(|c| c.record248.as_ref())
                .map(|bytes| (chrome_file.as_str(), "chrome248", bytes.as_slice())),
        )?;
        let app = &frame.bgra[48 * 2560..528 * 2560];
        // The app crop is derived from this actual complete composed frame; retain its digest, not a second raw copy.
        let mut protected = Vec::with_capacity(88 * 2560);
        protected.extend_from_slice(&frame.bgra[..48 * 2560]);
        protected.extend_from_slice(&frame.bgra[528 * 2560..]);
        self.frames.push(json!({"observation_index":n,"session_id":frame.session_id,"instance_id":frame.instance_id,"focus_epoch":frame.focus_epoch,"sequence":frame.sequence,"raw":raw,"protected_crop_sha256":digest(&protected),"app_crop_sha256":digest(app),"expected_protected_label":Value::Null,"expected_unknown_banner":Value::Null,"draft":draft.map(dto_native_editor_observation),"frame_join":join.map(dto_native_frame_observation),"chrome":chrome.map(dto_native_save_chrome_observation)}));
        Ok(())
    }
    pub fn saved_frame_witness(
        &mut self,
        frame: &desktop::ComposedFrame,
        draft: &desktop::NativeEditorObservation,
        join: &desktop::NativeFrameObservation,
    ) -> Result<()> {
        require(
            frame.bgra.len() == 1454080,
            "actual complete Saved frame bytes",
        )?;
        require(
            matches!(draft.phase, desktop::NativeEditorPhase::Saved)
                && join.actor == draft.actor
                && join.document_id == draft.document_id
                && join.text_generation == draft.text_generation
                && join.view_version == draft.view_version
                && join.actor.session_id == frame.session_id
                && join.actor.instance_id == frame.instance_id
                && join.focus_epoch == frame.focus_epoch
                && join.sequence == frame.sequence
                && join.app_crop_sha256.as_slice()
                    == Sha256::digest(&frame.bgra[48 * 2560..528 * 2560]).as_slice(),
            "genuine current Saved draft and actual private render join",
        )?;
        let actual_draft = dto_native_editor_observation(draft);
        let actual_join = dto_native_frame_observation(join);
        let row = self.frames.last_mut().ok_or(RecordingError(
            "Saved witness follows existing actual frame capture",
        ))?;
        require(
            row["session_id"] == frame.session_id
                && row["instance_id"] == frame.instance_id
                && row["focus_epoch"] == frame.focus_epoch
                && row["sequence"] == frame.sequence
                && row["raw"]["byte_len"] == frame.bgra.len()
                && row["raw"]["sha256"] == digest(&frame.bgra)
                && row["chrome"]["label"] == "Saved"
                && row["chrome"]["current_frame_actor"] == actual_draft["actor"]
                && row["chrome"]["document_id"] == draft.document_id
                && row["chrome"]["text_generation"] == draft.text_generation
                && row["chrome"]["view_version"] == draft.view_version,
            "Saved witnesses match most recent immutable full frame and chrome",
        )?;
        require(
            (row["draft"].is_null() || row["draft"] == actual_draft)
                && (row["frame_join"].is_null() || row["frame_join"] == actual_join),
            "existing actual Saved witnesses cannot be replaced",
        )?;
        row["draft"] = actual_draft;
        row["frame_join"] = actual_join;
        Ok(())
    }
    pub fn frame_expectation(
        &mut self,
        frame: &desktop::ComposedFrame,
        label: &'static str,
        banner: &'static str,
    ) -> Result<()> {
        require(
            matches!(
                label,
                "LOADED" | "UNSAVED" | "SAVING" | "SAVED" | "CONFLICT" | "UNAVAILABLE"
            ) && matches!(banner, "" | "SAVE UNKNOWN" | "ALLOCATION UNKNOWN"),
            "source literal protected expectation",
        )?;
        let hash = digest(&frame.bgra);
        let row = self.frames.last_mut().ok_or(RecordingError(
            "source expectation follows actual frame capture",
        ))?;
        require(
            row["session_id"] == frame.session_id
                && row["instance_id"] == frame.instance_id
                && row["focus_epoch"] == frame.focus_epoch
                && row["sequence"] == frame.sequence
                && row["raw"]["sha256"] == hash,
            "source expectation matches its most recently captured actual frame",
        )?;
        require(
            row["expected_protected_label"].is_null()
                || (row["expected_protected_label"] == label
                    && row["expected_unknown_banner"] == banner),
            "same frame has consistent source expectation",
        )?;
        row["expected_protected_label"] = json!(label);
        row["expected_unknown_banner"] = json!(banner);
        Ok(())
    }
    pub fn app_frame_copy(&mut self, purpose: &'static str, bytes: &[u8]) -> Result<()> {
        self.purpose(purpose)?;
        require(
            bytes.len() == 1228800 && !self.raw.iter().any(|r| r["kind"] == "app_frame"),
            "one source-declared actual frozen Surface full copy",
        )?;
        self.raw_bytes("surface-frozen-copy-0.bgra", "app_frame", bytes)?;
        self.unit_control(
            purpose,
            "SurfaceReadLease.copy_into",
            "Ok",
            "DesktopStatus",
            0,
            None,
        )
    }
    pub fn grant(
        &mut self,
        phase: &'static str,
        q: &kernel_api::ipc::Envelope,
        r: &kernel_api::ipc::Envelope,
        bytes: &[u8],
    ) -> Result<()> {
        require(
            self.grants.len() < 33 && matches!(phase, "Preview" | "Active"),
            "actual grant manifest observations",
        )?;
        require(
            (q.protocol, q.msg_type, r.protocol, r.msg_type)
                == if phase == "Preview" {
                    (352, 1, 352, 2)
                } else {
                    (352, 5, 352, 6)
                },
            "grant actual corresponding typed exchange",
        )?;
        let n = u32::try_from(self.grants.len())
            .map_err(|_| RecordingError("checked local observation ordinal"))?;
        let raw = self.raw_bytes(&format!("grant-{n}.bin"), "grant464", bytes)?;
        self.grants.push(json!({"observation_index":n,"phase":phase,"raw":raw,"request":hex(&desktop::encode_envelope_wire(q).map_err(|_|RecordingError("grant canonical request"))?),"reply":hex(&desktop::encode_envelope_wire(r).map_err(|_|RecordingError("grant canonical reply"))?)}));
        Ok(())
    }
    pub fn native_join(&mut self, actual: &desktop::NativeJoinObservation) -> Result<()> {
        require(
            self.native_joins.len() < 65536,
            "actual complete native joined rows cap",
        )?;
        self.native_joins.push(dto_native_join_observation(actual));
        Ok(())
    }
    pub fn external_join(&mut self, rust_thread_id: &str, panicked: bool) -> Result<()> {
        require(
            self.external_joins.len() < 128
                && !rust_thread_id.is_empty()
                && rust_thread_id.len() <= 64,
            "actual external joined caller bound",
        )?;
        let n = u32::try_from(self.external_joins.len())
            .map_err(|_| RecordingError("checked local observation ordinal"))?;
        self.external_joins.push(json!({"observation_index":n,"rust_thread_id":rust_thread_id,"joined":true,"outcome":if panicked{"Panicked"}else{"Returned"},"service_producer_claim":false}));
        Ok(())
    }
    pub fn store_snapshot(
        &mut self,
        epoch: u32,
        index: u32,
        purpose: &'static str,
        phase: &'static str,
        e: &store::StoreEvidence,
        owner: &Path,
        fence: Option<&store::NativeSaveFenceSnapshot>,
        reopen_error: Option<&store::FixtureError>,
    ) -> Result<()> {
        self.purpose(purpose)?;
        require(
            self.snapshots.len() < 64,
            "source-required Store snapshots cap",
        )?;
        require(
            matches!(
                phase,
                "source_checkpoint"
                    | "after_owned_join_before_transfer"
                    | "actual_failed_reopen_closed"
                    | "final_before_cleanup"
                    | "before_storage_fault"
                    | "after_storage_fault"
            ),
            "closed Store snapshot phase",
        )?;
        require(
            !self.snapshots.iter().any(|v| {
                v["epoch_index"].as_u64() == Some(epoch as u64)
                    && v["observation_index"]
                        .as_u64()
                        .is_some_and(|n| n >= index as u64)
            }),
            "Store snapshot strict ordinal",
        )?;
        require(
            e.exchanges.len() <= 131072
                && e.objects.len() <= 3
                && e.barriers.len() <= 64
                && e.io_events.len() <= 65536,
            "complete Store evidence caps",
        )?;
        let file_rows = self.snapshot_files(owner, epoch, index)?;
        let file = format!("store-epoch-{epoch}-snapshot-{index}.json");
        let v = json!({"binding":self.binding(epoch),"observation_index":index,"purpose":purpose,"phase":phase,"exchanges":e.exchanges.iter().map(dto_exchange).collect::<Vec<_>>(),"objects":e.objects.iter().map(dto_object_evidence).collect::<Vec<_>>(),"barriers":e.barriers.iter().map(dto_barrier_snapshot).collect::<Vec<_>>(),"io_events":e.io_events.iter().map(dto_io_event).collect::<Vec<_>>(),"ledger":dto_store_ledger(&e.ledger),"files":file_rows,"native_fence":fence.map(dto_native_save_fence_snapshot),"reopen_error":reopen_error.map(|e|json!({"status":e.status as u32,"reason":fixture_reason(&e.reason)}))});
        self.record(&file, &v)?;
        self.snapshots.push(
            json!({"epoch_index":epoch,"observation_index":index,"purpose":purpose,"record":file}),
        );
        if !self.epochs.contains(&epoch) {
            require(self.epochs.len() < 4, "actual Core sequence cap")?;
            self.epochs.push(epoch);
        }
        if let Some(f) = fence {
            let actual = json!({"epoch_index":epoch,"original_tickets":f.original_tickets.iter().map(dto_original_ticket_observation).collect::<Vec<_>>()});
            if let Some(previous) = self
                .ticket_batches
                .iter()
                .find(|v| v["epoch_index"].as_u64() == Some(epoch as u64))
            {
                require(
                    previous == &actual,
                    "same actual closed Core ticket table remains immutable",
                )?;
            } else {
                require(
                    self.ticket_batches.len() < 4,
                    "complete actual Core fence ticket tables cap",
                )?;
                self.ticket_batches.push(actual);
            }
        }
        Ok(())
    }
}
fn descriptor(d: &store::ObjectDescriptor) -> Value {
    observed(require(
        d.handle.index <= u16::MAX as u32
            && d.handle.generation > 0
            && d.handle.generation <= u32::MAX as u64,
        "actual checked descriptor parts",
    ));
    json!({"kind":match d.kind {store::SharedObjectKind::SelectedText=>"SelectedText",store::SharedObjectKind::DraftSource=>"DraftSource",store::SharedObjectKind::Receipt=>"Receipt"},"handle_kind":d.handle.kind as u32,"handle_index":d.handle.index,"handle_generation":d.handle.generation,"object_generation":d.object_generation,"byte_len":d.byte_len})
}
fn fixture_reason(r: &store::FixtureErrorKind) -> &'static str {
    match r {
        store::FixtureErrorKind::InvalidProfile => "InvalidProfile",
        store::FixtureErrorKind::Io => "Io",
        store::FixtureErrorKind::Corrupt => "Corrupt",
        store::FixtureErrorKind::Unfenced => "Unfenced",
        store::FixtureErrorKind::Capacity => "Capacity",
        store::FixtureErrorKind::Exhausted => "Exhausted",
        store::FixtureErrorKind::Timeout => "Timeout",
    }
}

impl ScenarioRecorder {
    pub fn storage_fault_applied(
        &mut self,
        object_id: u64,
        actual_fault: store::StorageFault,
    ) -> Result<()> {
        require(
            object_id > 0 && self.applied_storage_faults.len() < 3,
            "source-owned successful storage-fault context bound",
        )?;
        let path = format!("object-{object_id}/selection.json");
        require(
            !self.applied_storage_faults.contains_key(&path),
            "one actual fault context per source object",
        )?;
        self.applied_storage_faults.insert(path, actual_fault);
        Ok(())
    }
    fn snapshot_files(&mut self, owner: &Path, epoch: u32, snapshot: u32) -> Result<Vec<Value>> {
        let held = held_directory(owner)?;
        let mut rows = Vec::new();
        let mut entries = 0usize;
        let mut present = BTreeSet::new();
        self.walk_snapshot(
            owner,
            owner,
            epoch,
            snapshot,
            0,
            &mut entries,
            &mut present,
            &mut rows,
        )?;
        check_directory(owner, &held)?;
        let prior: Vec<_> = self
            .known_files
            .iter()
            .filter(|v| !present.contains(*v))
            .cloned()
            .collect();
        require(
            rows.len() + prior.len() <= 2048,
            "complete file observations cap",
        )?;
        for relative in prior {
            let p = owner.join(&relative);
            let error = fs::symlink_metadata(p)
                .err()
                .ok_or(RecordingError("missing snapshot entry unexpectedly exists"))?;
            require(
                error.kind() == io::ErrorKind::NotFound,
                "actual absent file observation",
            )?;
            rows.push(json!({"relative_path":relative,"entry_kind":"absent","mode":Value::Null,"byte_len":Value::Null,"sha256":Value::Null,"raw":Value::Null,"symlink_target":Value::Null}));
        }
        self.known_files.extend(present);
        Ok(rows)
    }
    #[allow(clippy::too_many_arguments)]
    fn walk_snapshot(
        &mut self,
        owner: &Path,
        path: &Path,
        epoch: u32,
        snapshot: u32,
        depth: usize,
        entries: &mut usize,
        present: &mut BTreeSet<String>,
        rows: &mut Vec<Value>,
    ) -> Result<()> {
        require(depth <= 8, "fixture observation depth")?;
        let iterator =
            fs::read_dir(path).map_err(|_| RecordingError("actual fixture snapshot directory"))?;
        for e in iterator {
            *entries = entries
                .checked_add(1)
                .ok_or(RecordingError("fixture entry overflow"))?;
            require(
                *entries <= 2048 && rows.len() < 2048,
                "fixture observation entries cap",
            )?;
            let p = e
                .map_err(|_| RecordingError("fixture snapshot entry"))?
                .path();
            let relative = p
                .strip_prefix(owner)
                .map_err(|_| RecordingError("owner child relative observation"))?
                .to_str()
                .ok_or(RecordingError("fixture relative path UTF8"))?
                .to_owned();
            require(
                relative.len() <= 256
                    && !relative.split('/').any(|c| matches!(c, "" | "." | ".."))
                    && !relative.contains('\\'),
                "fixture relative path",
            )?;
            let before = fs::symlink_metadata(&p)
                .map_err(|_| RecordingError("actual fixture entry metadata"))?;
            require(
                present.insert(relative.clone()),
                "unique actual fixture file observation",
            )?;
            if before.file_type().is_symlink() {
                let target = fs::read_link(&p)
                    .map_err(|_| RecordingError("bounded actual symlink metadata"))?;
                let target = target
                    .to_str()
                    .ok_or(RecordingError("symlink target UTF8"))?;
                require(target.len() <= 4096, "symlink target bound")?;
                let after = fs::symlink_metadata(&p)
                    .map_err(|_| RecordingError("symlink final metadata"))?;
                require(identity(&before, &after), "stable link observation")?;
                rows.push(json!({"relative_path":relative,"entry_kind":"symlink","mode":before.mode(),"byte_len":before.len(),"sha256":Value::Null,"raw":Value::Null,"symlink_target":target}));
            } else if before.is_dir() {
                rows.push(json!({"relative_path":relative,"entry_kind":"directory","mode":before.mode(),"byte_len":before.len(),"sha256":Value::Null,"raw":Value::Null,"symlink_target":Value::Null}));
                self.walk_snapshot(
                    owner,
                    &p,
                    epoch,
                    snapshot,
                    depth + 1,
                    entries,
                    present,
                    rows,
                )?;
            } else {
                require(
                    before.is_file() && before.len() <= 131072 && before.nlink() == 1,
                    "bounded real fixture regular file",
                )?;
                let mut input = OpenOptions::new()
                    .read(true)
                    .custom_flags(NOFOLLOW)
                    .open(&p)
                    .map_err(|_| RecordingError("actual nofollow fixture read"))?;
                require(
                    identity(
                        &before,
                        &input
                            .metadata()
                            .map_err(|_| RecordingError("fixture held stat"))?,
                    ),
                    "fixture read identity",
                )?;
                let mut bytes = Vec::with_capacity(before.len() as usize);
                Read::by_ref(&mut input)
                    .take(before.len() + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|_| RecordingError("bounded fixture file copy"))?;
                require(
                    bytes.len() as u64 == before.len(),
                    "complete unchanged fixture file length",
                )?;
                let after = input
                    .metadata()
                    .map_err(|_| RecordingError("fixture final held stat"))?;
                require(
                    identity(&before, &after)
                        && before.len() == after.len()
                        && before.mtime() == after.mtime()
                        && before.mtime_nsec() == after.mtime_nsec(),
                    "stable fixture full copy",
                )?;
                let current = fs::symlink_metadata(&p)
                    .map_err(|_| RecordingError("fixture final path stat"))?;
                require(identity(&after, &current), "fixture final path identity")?;
                let kind = if p.file_name().is_some_and(|n| n == "selection.json") {
                    if self
                        .applied_storage_faults
                        .get(&relative)
                        .is_some_and(|fault| {
                            matches!(
                                fault,
                                store::StorageFault::TruncatedJournal
                                    | store::StorageFault::UnknownJournalVersion
                                    | store::StorageFault::ExtraJournalField
                                    | store::StorageFault::DuplicateJournalField
                                    | store::StorageFault::WrongJournalBinding
                                    | store::StorageFault::BrokenJournalChain
                                    | store::StorageFault::EarlierAllocatedOnlyJournal
                            )
                        })
                    {
                        "corrupt_journal"
                    } else {
                        "journal"
                    }
                } else if p.extension().is_some_and(|n| n == "blob") {
                    "cas_blob"
                } else {
                    "fixture_file"
                };
                let raw = self.raw_bytes(
                    &format!("file-{epoch}-{snapshot}-{}.bin", rows.len()),
                    kind,
                    &bytes,
                )?;
                rows.push(json!({"relative_path":relative,"entry_kind":"regular","mode":before.mode(),"byte_len":before.len(),"sha256":raw["sha256"],"raw":raw,"symlink_target":Value::Null}));
            }
        }
        Ok(())
    }
    pub fn negative_source(
        &mut self,
        control: &'static str,
        descriptor: &store::ObjectDescriptor,
        offset: u32,
        bytes: &[u8],
        write_status: store::StoreStatus,
        commit: &kernel_api::ipc::Envelope,
        reply: &kernel_api::ipc::Envelope,
        status: store::StoreStatus,
    ) -> Result<()> {
        self.purpose(control)?;
        require(self.negative.len() < 256, "source negative control cap")?;
        require(
            offset
                .checked_add(bytes.len() as u32)
                .is_some_and(|end| end <= descriptor.byte_len),
            "actual source write range",
        )?;
        require(
            commit.protocol == 368
                && commit.msg_type == 5
                && reply.protocol == 368
                && reply.msg_type == 6,
            "actual negative source Commit control",
        )?;
        let n = u32::try_from(self.negative.len())
            .map_err(|_| RecordingError("checked local observation ordinal"))?;
        let raw = self.raw_bytes(
            &format!("negative-source-{n}.bin"),
            "text64_negative",
            bytes,
        )?;
        self.negative.push(json!({"source_control":control,"kind":"text64_negative","raw":raw,"actual_denial_layer":"native_source_consumption","status_space":"StoreStatus","actual_status":status as u32,"source_descriptor":self::descriptor(descriptor),"actual_write":{"offset":offset,"byte_len":bytes.len(),"status":write_status as u32},"original_request":hex(&desktop::encode_envelope_wire(commit).map_err(|_|RecordingError("canonical source Commit request"))?),"actual_reply":hex(&desktop::encode_envelope_wire(reply).map_err(|_|RecordingError("canonical source denial"))?)}));
        Ok(())
    }
    pub fn negative_wire(
        &mut self,
        control: &'static str,
        bytes: &[u8; 88],
        status: desktop_service::dev::Status,
    ) -> Result<()> {
        self.purpose(control)?;
        require(self.negative.len() < 256, "wire negative control cap")?;
        let n = u32::try_from(self.negative.len())
            .map_err(|_| RecordingError("checked local observation ordinal"))?;
        let raw = self.raw_bytes(&format!("negative-wire-{n}.bin"), "wire88_negative", bytes)?;
        self.negative.push(json!({"source_control":control,"kind":"wire88_negative","raw":raw,"actual_denial_layer":"wire_decoder","status_space":"DesktopStatus","actual_status":status as u32,"source_descriptor":Value::Null,"actual_write":Value::Null,"original_request":Value::Null,"actual_reply":Value::Null}));
        Ok(())
    }
    fn final_records(
        &mut self,
        cleanup: Value,
        held_after: u32,
        failed_reopen: bool,
    ) -> Result<()> {
        require(
            held_after == 0,
            "actual joined shared roster before final evidence",
        )?;
        let artifacts = json!({"binding":self.binding(0),"partial_copies_are_complete_captures":false,"batches":self.capture_batches});
        self.record("artifacts.json", &artifacts)?;
        let joins = json!({"binding":self.binding(0),"native_join_observations":self.native_joins,"external_callers":self.external_joins,"held_after":held_after,"fence_original_ticket_batches":self.ticket_batches});
        self.record("joins.json", &joins)?;
        self.record("cleanup.json", &cleanup)?;
        let mut inventory = self.records.clone();
        inventory.push("summary.json".to_owned());
        let summary = json!({"binding":self.binding(0),"actual_store_epochs":self.epochs,"frames":self.frames,"app_observations":self.drafts,"source_case_requirement_ids":self.spec.requirements,"raw_inventory":self.raw,"record_inventory":inventory,"terminal_kind":if failed_reopen {"expected-fenced-reopen-failure-owned-close"} else {"normal-owned-close"},"negative_inputs":self.negative,"store_snapshots":self.snapshots,"grant_observations":self.grants,"producer_observations":self.counts,"control_observations":self.controls});
        self.record("summary.json", &summary)
    }
    pub fn finish_native_save(
        mut self,
        root: tempfile::TempDir,
        before: u32,
        after: u32,
        fences: &[store::NativeSaveFenceSnapshot],
        failed_reopen: bool,
    ) -> Result<()> {
        require(
            matches!(
                self.spec.scope,
                ScenarioScope::Composition | ScenarioScope::DirectNative
            ) && before <= 64
                && after == 0
                && !fences.is_empty()
                && fences.len() <= 4
                && fences.iter().all(|f| f.base.live_ownership_after_join == 0),
            "actual nativeSave cleanup scope",
        )?;
        let identity = root_identity(root.path())?;
        let path = root.path().to_owned();
        root.close()
            .map_err(|_| RecordingError("actual TempDir close failed"))?;
        let actual = fs::symlink_metadata(&path)
            .err()
            .ok_or(RecordingError("owned root still exists"))?;
        require(
            actual.kind() == io::ErrorKind::NotFound,
            "actual owned root absence",
        )?;
        let marker = self.case.root.join(".recording-reservation");
        require(
            fs::symlink_metadata(marker).is_err_and(|e| e.kind() == io::ErrorKind::NotFound),
            "all recording writes settled",
        )?;
        let cleanup = json!({"binding":self.binding(0),"root_identity":identity,"owner_relative":"owner","service_held_before_cleanup":before,"service_held_after_join":after,"external_callers":self.external_joins,"fences":fences.iter().map(dto_native_save_fence_snapshot).collect::<Vec<_>>(),"root_close_result":"Ok","root_absence_result":"NotFound","all_recording_reservations_settled":true,"scope":"owned host test fixture; no processkill/device/HIL or generic containment"});
        self.final_records(cleanup, after, failed_reopen)
    }
}
fn root_identity(path: &Path) -> Result<Value> {
    let held = held_directory(path)?;
    let m = held
        .metadata()
        .map_err(|_| RecordingError("actual root identity"))?;
    let path = path
        .to_str()
        .ok_or(RecordingError("actual root path UTF8"))?;
    require(path.len() <= 4096, "actual root path bound")?;
    Ok(
        json!({"path":path,"device":m.dev(),"inode":m.ino(),"mode":m.mode(),"uid":m.uid(),"gid":m.gid()}),
    )
}
