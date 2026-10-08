//! Payloads and bounded diagnostics; none of these mint a live permit or fence.
use artifact_store_schema::editor_save::*;
use kernel_api::cap::Handle;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum StoreStatus {
    Ok = 0,
    Denied = 1,
    Invalid = 2,
    Unsupported = 3,
    Stale = 4,
    Exhausted = 5,
    NotReady = 6,
    Conflict = 7,
    Timeout = 8,
    Disconnected = 9,
    Unknown = 10,
    Internal = 11,
}
#[derive(Clone, Copy, Debug)]
pub enum FixtureErrorKind {
    InvalidProfile,
    Io,
    Corrupt,
    Unfenced,
    Capacity,
    Exhausted,
    Timeout,
}
#[derive(Debug)]
pub struct FixtureError {
    pub status: StoreStatus,
    pub reason: FixtureErrorKind,
}
impl FixtureError {
    pub(super) fn new(status: StoreStatus) -> Self {
        Self {
            status,
            reason: match status {
                StoreStatus::Invalid => FixtureErrorKind::InvalidProfile,
                StoreStatus::Exhausted => FixtureErrorKind::Exhausted,
                StoreStatus::NotReady => FixtureErrorKind::Unfenced,
                StoreStatus::Unknown => FixtureErrorKind::Corrupt,
                _ => FixtureErrorKind::Io,
            },
        }
    }
}
#[derive(Clone)]
pub struct InitialObject {
    pub owner_id: u64,
    pub object_id: u64,
    pub generation: u64,
    pub revision: u64,
    pub bytes: Vec<u8>,
}
#[derive(Clone)]
pub struct FixtureProfile {
    pub origin_ms: u64,
    pub initial_objects: Vec<InitialObject>,
    pub fixture_signing_seed: [u8; 32],
    pub fixture_key_id: String,
}
pub struct GrantSpec {
    pub actor: EditorActorV0,
    pub selected_object_id: u64,
    pub selected_generation: u64,
    pub rights: u32,
    pub ttl_ms: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndpointClass {
    Artifact,
    RecoveryReceipt,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedObjectKind {
    SelectedText,
    DraftSource,
    Receipt,
}
#[derive(Clone)]
pub struct ObjectDescriptor {
    pub handle: Handle,
    pub kind: SharedObjectKind,
    pub object_generation: u64,
    pub byte_len: u32,
}
#[derive(Clone, Copy)]
pub enum RetirementReason {
    Revoked,
    ServiceRetired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CounterKind {
    OperationHighWater,
    WriterHighWater,
    ServiceEpoch,
    JournalSequence,
    Identity,
    TimeOrigin,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PausePoint {
    BeforeAllocationJournal,
    BeforeSubmissionJournal,
    BeforeCommitPermit,
    AfterCommitPermit,
    BeforeCandidateWrite,
    AfterCandidatePublication,
    BeforeSelectionRename,
    AfterSelectionRename,
    AfterAcknowledgement,
    BeforeReadReply,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IoPoint {
    CandidateIntent,
    CandidateBlobWrite,
    CandidateBlobSync,
    CandidateManifest,
    CandidateOwner,
    CandidateDirectorySync,
    CandidateReadback,
    JournalWrite,
    JournalSync,
    JournalRename,
    JournalDirectorySync,
    JournalReadback,
}
#[derive(Clone, Copy)]
pub enum StorageFault {
    MissingJournal,
    TruncatedJournal,
    UnknownJournalVersion,
    ExtraJournalField,
    DuplicateJournalField,
    WrongJournalBinding,
    BrokenJournalChain,
    EarlierAllocatedOnlyJournal,
    CorruptBlob,
    UnsignedManifest,
    WrongSigningKey,
    UnboundPublicationIntent,
    UnsignedPublicationIntent,
    SymlinkEntry,
}
#[derive(Clone, Debug, Serialize)]
pub struct BarrierSnapshot {
    pub request_id: u64,
    pub operation_id: u64,
    pub entered: bool,
    pub released: bool,
    pub settled: bool,
    pub permit_issued: bool,
}
#[derive(Clone)]
pub struct Exchange {
    pub actor: EditorActorV0,
    pub endpoint_class: EndpointClass,
    pub selected_object_id: u64,
    pub request_wire: [u8; 88],
    pub actual_reply_wire: Option<[u8; 88]>,
    pub reply_suppressed: bool,
    pub operation_id: u64,
    pub elapsed_us: u64,
    pub observed_ms: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IoOutcome {
    Began,
    Completed,
    SimulatedFailure,
    ActualFailure,
}
#[derive(Clone)]
pub struct IoEvent {
    pub selected_object_id: u64,
    pub operation_id: u64,
    pub writer_id: u64,
    pub sequence: u64,
    pub point: IoPoint,
    pub outcome: IoOutcome,
    pub artifact_hash: Option<Hash32>,
}
#[derive(Clone)]
pub struct ObjectEvidence {
    pub selected: SelectedArtifactV0,
    pub operations: Vec<EditorOperationRecordV0>,
    pub mutation_dispatch_count: u32,
    pub permit_count: u32,
    pub transition_count: u32,
    pub journal_hash: Hash32,
    pub journal_sequence: u64,
    pub blob_hash: Hash32,
    pub manifest_hash: Hash32,
    pub ownership_hash: Hash32,
    pub mutation_quarantined: bool,
    pub pending_writer_id: Option<u64>,
    pub service_epoch: u64,
    pub reopen_witness: Option<ReopenWitness>,
}
#[derive(Clone)]
pub struct StoreLedger {
    pub sessions: u32,
    pub selected_objects: u32,
    pub original_instances: u32,
    pub endpoints: u32,
    pub shared_objects: u32,
    pub active_io_workers: u32,
    pub active_supervisors: u32,
    pub held_service_producers: u32,
    pub operations_per_object: Vec<(u64, u32)>,
    pub cas_entries_per_object: Vec<(u64, u32)>,
    pub bytes_per_object: Vec<(u64, u64)>,
}
pub struct StoreEvidence {
    pub exchanges: Vec<Exchange>,
    pub objects: Vec<ObjectEvidence>,
    pub barriers: Vec<BarrierSnapshot>,
    pub io_events: Vec<IoEvent>,
    pub ledger: StoreLedger,
}
#[derive(Clone, Debug, Serialize)]
pub enum ProducerKind {
    Dispatcher,
    Supervisor,
    ObjectIoWorker,
}
#[derive(Clone, Debug, Serialize)]
pub enum ProducerJoinOutcome {
    Returned,
    Panicked,
}
#[derive(Clone, Debug, Serialize)]
pub struct ProducerFence {
    pub producer_id: u64,
    pub producer_generation: u64,
    pub kind: ProducerKind,
    pub native_pid: u32,
    pub rust_thread_id: String,
    pub selected_object_id: Option<u64>,
    pub request_id: Option<u64>,
    pub operation_id: Option<u64>,
    pub completed_join: bool,
    pub join_outcome: ProducerJoinOutcome,
    pub settled_barrier: Option<BarrierSnapshot>,
    pub live_ownership_after_join: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct OperationClosureWitness {
    pub allocation: EditorSaveAllocationV0,
    pub submitted_binding: Option<EditorSaveBindingV0>,
    pub original_request_id: u64,
    pub original_producer_id: u64,
    pub original_producer_generation: u64,
    pub closure: EditorClosureV0,
}
#[derive(Clone, Debug, Serialize)]
pub struct ObjectFenceWitness {
    pub owner_id: u64,
    pub selected_object_id: u64,
    pub selected_generation: u64,
    pub service_epoch_at_fence: u64,
    pub reserved_successor_epoch: Option<u64>,
    pub durable_journal_sequence: u64,
    pub durable_journal_hash: Hash32,
    pub operation_high_water_min: u64,
    pub writer_high_water_min: u64,
    pub admission_epoch_min: u64,
    pub known_durable_operations: Vec<EditorOperationRecordV0>,
    pub issued_runtime_permits: Vec<EditorCommitPermitV0>,
    pub original_closures: Vec<OperationClosureWitness>,
}
#[derive(Clone, Debug, Serialize)]
pub struct FenceSnapshot {
    pub schema_version: u32,
    pub namespace_id: u64,
    pub fence_id: u64,
    pub profile_hash: Hash32,
    pub semantic_clock_min: u64,
    pub identity_counter_min: u64,
    pub service_producers: Vec<ProducerFence>,
    pub object_workers: Vec<ProducerFence>,
    pub objects: Vec<ObjectFenceWitness>,
    pub live_ownership_after_join: u32,
}
#[derive(Clone)]
pub struct ReopenWitness {
    pub fence_id: u64,
    pub fence_snapshot_hash: Hash32,
    pub selected_object_id: u64,
    pub selected_generation: u64,
    pub prior_service_epoch: u64,
    pub actual_service_epoch: u64,
    pub reopened_journal_sequence: u64,
    pub reopened_journal_hash: Hash32,
}
