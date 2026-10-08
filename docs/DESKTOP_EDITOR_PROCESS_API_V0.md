# Desktop editor process API V0

**Status:** FROZEN_CONTRACT_RUNTIME_NOT_IMPLEMENTED · 2026-10-06.

This inventory defines the default-off Linux process boundary. The [combined contract](contracts/editor-process-v0.json) owns authority, bounds and assertions. Declarations below are requirements; executable gate-first review and missing-API RED precede implementation.

```rust
// Maintained destination: docs/DESKTOP_EDITOR_PROCESS_API_V0.md
// FROZEN CONTRACT SIGNATURE INVENTORY — not a Rust implementation.
// Proposed paths: services/desktop/src/editor_dev/process_editor.rs (new)
// and services/desktop/src/editor_dev/process_editor/linux.rs (new).
// Root decisions specify the paths, default-off feature graph and sole-supervisor ownership. Independent review precedes Root adoption/freeze.
// Every declaration below is default-off behind host_editor_process_v0_dev.
// Fields of opaque structs stay private; no Clone, public field, from_bytes,
// from_actor, from_pid, from_fd, from_path or caller-implemented witness trait.

pub struct ProcessEditorEnrollment { /* private */ }
pub struct PinnedProcessExecutable { /* private; bound to enrollment */ }
pub struct OwnedEditorChild { /* private; opaque supervised record/channel owners, no direct wait authority */ }
pub struct OwnedEditorChildExitProof { /* private; genuine wait + supervisor join */ }
pub struct ProcessPrepareFailure { /* private; retains consumed NativeEditor */ }
pub struct ProcessLaunchFailure { /* private; retains recovery owner */ }
pub struct ProcessJoinFailure { /* private; retains unreaped child */ }
pub struct ProcessStepFailure { /* private; child remains with borrower */ }
pub struct ProcessEditorStep { /* private diagnostic fields only */ }
pub struct ProcessChildObservation { /* private descriptive copies only */ }
pub enum ProcessWaitOutcome { Exited(i32), Signaled(i32) }
pub struct ChildBudget { /* private checked milliseconds */ }
// Internal ProcessSupervisor exclusively owns OwnedLinuxProcess { fork-derived
// PID, pidfd, birth and waitpid authority }. OwnedEditorChild retains only its
// authentic supervised record and JoinHandle. No second waiter is permitted.

pub enum ProcessRetireReason {
    HumanClose, Revoked, Expired, ChildFault, InvalidTransport, Deadline,
}
pub enum ProcessLaunchRecovery {
    Unspawned {
        enrollment: ProcessEditorEnrollment,
        executable: PinnedProcessExecutable,
    },
    Spawned(OwnedEditorChild),
}

impl ChildBudget {
    pub fn checked(milliseconds: u32) -> Result<Self, Status>; // 1..=5000; otherwise Status::Invalid
}
impl ProcessPrepareFailure {
    pub fn into_parts(self) -> (Status, NativeEditor);
}
impl ProcessLaunchFailure {
    pub fn into_parts(self) -> (Status, ProcessLaunchRecovery);
}
impl ProcessJoinFailure {
    pub fn into_parts(self) -> (Status, OwnedEditorChild);
}
impl ProcessStepFailure {
    pub fn status(&self) -> Status;
}
impl ProcessEditorStep {
    pub fn consumed_sequence(&self) -> Option<u64>;
    pub fn publication_accepted(&self) -> bool;
    pub fn save_intent_available(&self) -> bool;
    pub fn child_exit_observed(&self) -> bool; // not a reap proof
}

impl ProcessChildObservation {
    pub fn process_id(&self) -> u32;
    pub fn birth_ticks(&self) -> u64;
    pub fn executable_sha256(&self) -> [u8; 32];
    pub fn acknowledged_sequence(&self) -> u64;
    pub fn publication_generation(&self) -> u64;
}
impl OwnedEditorChildExitProof {
    pub fn process_id(&self) -> u32;
    pub fn birth_ticks(&self) -> u64;
    pub fn executable_sha256(&self) -> [u8; 32];
    pub fn wait_outcome(&self) -> ProcessWaitOutcome;
    pub fn supervisor_joined(&self) -> bool;
}
// Observation constructors/fields are private. Fork-derived PID and actual
// parent-observed birth plus held ELF digest are descriptive only; terminal
// outcome comes only from supervisor waitpid. No raw FD authority getter.

impl HostDesktop {
    pub fn observe_process_child(
        &self, child: &OwnedEditorChild,
    ) -> Result<ProcessChildObservation, Status>;

    pub fn prepare_process_editor(
        &self, editor: NativeEditor,
    ) -> Result<ProcessEditorEnrollment, ProcessPrepareFailure>;

    // Looks up only the coordinator-registered editor artifact internally.
    // Takes no path, hash, raw FD or executable bytes from the consumer.
    // Failure retains enrollment with its borrower. Default-off build fails
    // closed if no actual registered pinned executable exists.
    pub fn registered_editor_process_executable(
        &self, enrollment: &ProcessEditorEnrollment,
    ) -> Result<PinnedProcessExecutable, Status>;

    pub fn spawn_process_editor(
        &self, enrollment: ProcessEditorEnrollment,
        executable: PinnedProcessExecutable, budget: ChildBudget, now_ms: u64,
    ) -> Result<OwnedEditorChild, ProcessLaunchFailure>;

    // Nonblocking bounded poll; real input-first Focus dequeue, individual
    // child acknowledgments, max128 acknowledged deliveries OR64 key-downs including modifiers publication batching, private mapping
    // verification/adoption and immediate genuine Ctrl+S flush. Caller cannot
    // submit text, raster, logical keys or publication witnesses here.
    // now_ms is enforcing Core time observation, never a replacement Instant
    // or a way to renew child/Save deadlines. Work captures actual Instant.
    pub fn pump_process_editor(
        &self, child: &mut OwnedEditorChild, now_ms: u64,
    ) -> Result<ProcessEditorStep, ProcessStepFailure>;

    // Optional early human render flush. Only current owned acknowledged
    // state can be requested. No caller data or Save ticket is accepted.
    pub fn request_process_editor_frame(
        &self, child: &mut OwnedEditorChild, now_ms: u64,
    ) -> Result<(), ProcessStepFailure>;

    pub fn take_process_save_intent(
        &self, child: &mut OwnedEditorChild,
    ) -> Result<NativeSaveIntent, Status>;

    pub fn observe_process_editor(
        &self, child: &OwnedEditorChild, now_ms: u64,
    ) -> Result<NativeEditorObservation, Status>;

    pub fn retire_process_editor(
        &self, child: &mut OwnedEditorChild,
        reason: ProcessRetireReason, now_ms: u64,
    ) -> Result<(), ProcessStepFailure>;

    // Supervisor alone performs waitpid. Consuming join validates its authentic terminal result, then joins the actual supervisor. Error returns sole unreaped/quarantined supervised owner.
    // Original Store source/ticket/IO owners remain in the common parent SaveChains.
    // retire only requests signaling from supervisor; it cannot call waitpid.
    pub fn join_process_editor(
        &self, child: OwnedEditorChild,
    ) -> Result<OwnedEditorChildExitProof, ProcessJoinFailure>;
}

// Internal-only in process_editor.rs; never public/reexported.
struct DeliveredProcessInput { /* genuine Focus dequeue + current epochs */ }
struct VerifiedProcessPublication { /* actual held pair + seals/maps/state */ }
struct ProcessSaveCapture { /* genuine Ctrl+S original Instant + ticket */ }
struct AcceptedProcessStep { /* private accepted document transition */ }
struct ProcessAdoptionFailure { /* retains publication/capture owners */ }
fn adopt_process_publication(
    authority: &NativeAuthority,
    child: &mut OwnedEditorChild,
    publication: VerifiedProcessPublication,
    capture: Option<ProcessSaveCapture>,
    now_ms: u64,
) -> Result<AcceptedProcessStep, ProcessAdoptionFailure>;

// ReceiptNotice is produced only by the private pump after existing Core
// receipt validation updates this same document's confirmed tuple. There is
// no public receipt/body injection method. App reuses the existing genuine
// Read/Allocate/Commit/Status/Recovery orchestration and input-first ordering.
// Required public consumer assertion calls the approved existing launch/read,
// consumes its initialized NativeEditor through prepare, obtains only the
// registered pinned executable, spawns, pumps, obtains NativeSaveIntent and
// runs the existing Save chain. Missing declarations yield real compile RED;
// compiling stubs without child behavior must still fail behavior assertions.

// Root-frozen first-frame route: pump validates the real first child mapped
// pair, then privately uses existing genuine Surface.Create/Focus.Assign/
// Acquire/Present based on actual Core registry and CreateReply, preserving
// epochs/caps/error retention. No public numeric surface tuple is needed.
// Consumer calls existing compose_native_save(actual compositor, now_ms).
// The parent must never call legacy render/step for a process-mode document.

// acknowledged_sequence is the actual delivered Focus sequence, including
// only genuine service reset deliveries; never a fabricated process counter.
```
