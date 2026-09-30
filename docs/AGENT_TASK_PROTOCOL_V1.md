# Agent Task control protocol V1 — SW0 A1.0

**Status:** IDL, generated kernel API bindings and request preflight implemented.
No service handler, broker interface registration or task authority is enabled.
**Gate:** `just foundry-agent-task-protocol-a1-0`
**Evidence:** host wire-contract assertions, not a useful task or enforcement proof.

The [A0 transaction model](AGENT_TASK_CONTRACT_V0.md) supplies the semantics.
[`agent_task_v1.toml`](../idl/harness/agent_task_v1.toml) reserves protocol 14
and defines nine request/reply pairs plus a task event. All control messages
fit the existing 64-byte envelope payload without implicit structure padding.
`just codegen` generates the bindings; the allocation-free
`kernel_api::agent_task_protocol::parse_request` checks exact payload sizes,
known request operations, nonzero identifiers, bounded scalar fields and
canonical shared-memory handle encoding. Replies and events cannot enter
request dispatch. A successful parse never establishes authority.

## Request and observation authority

The transport supplies the authenticated caller domain and endpoint authority.
There is no caller-selected domain field. `policy_cap` is authority to request
or revoke a fixture-approved grant; task rights cannot mint policy authority.
The broker must intersect each request with its task/resource allowlist and
maximum lifetime. Requesting known rights does not entitle a caller to them.

| Request | Required authority | Allowed result |
|---------|--------------------|----------------|
| `request_grant` | Policy capability for the exact task/resource | A service-owned task capability, generation, granted rights and monotonic expiry |
| `read_input` | READ (1), exact allowed input resource | Read-only shared-memory bytes and their content hash |
| `stage_candidate` | STAGE (2), task output resource; readable source mapping | Immutable private candidate capability and content hash |
| `validate_candidate` | VALIDATE (4), same task output and pinned validator | Trusted result and bounded diagnostics; no accepted output change |
| `commit_candidate` | COMMIT (8), same task output | Durable receipt and new revision/hash, or the exact earlier receipt |
| `get_receipt` | Current COMMIT (8) for the same task/resource | Receipt for the named commit request, including its original revision/hash |
| `get_task_state` | OBSERVE (16) for this task | Scoped state snapshot |
| `revoke_grant` | Policy capability for the named task capability | Generation change invalidating dependent authority/validation |
| `subscribe_task` | OBSERVE (16) for this task | Scoped subscription and starting revision |

Rights are a bitmask bounded by 31; grant lifetime is 1–300000 ms. The backend
uses its trusted monotonic clock, and rechecks lifetime and generation before
every effect and observation, including event delivery and receipt retrieval.
Task, candidate, receipt and subscription capabilities are opaque service
registry objects, not additional kernel `HandleKind` values. Their registry
must bind caller domain, object kind, task, resource and generation. Nonzero
numbers alone are insufficient. The endpoint's kernel handle validation is
separate from these service checks.

`source_shm_cap` uses the existing packed kernel Shmem handle. Preflight rejects
wrong kinds, zero generations and reserved encoding bits; it does not look up
the handle table or prove ownership, access rights or mapping extent. Staging
must obtain a stable snapshot of 1–65536 bytes and publish immutable bytes before
returning their hash. Writable aliases must not change the candidate after
validation. No host path, arbitrary program or shell command enters the contract.

Validation requests identify the pinned validator hash but contain no success
flag or caller attestation. The service resolves schema/policy pins from trusted
task state, verifies the validator pin, and supervises execution against the
exact immutable candidate. The result binds all A0 identities, current grant
generation, deadlines and execution measurements. The validator may not commit.
Commit checks both expected revision and expected hash, so returning to the same
bytes cannot bypass an intervening revision. Revocation requires a fresh grant
and validation before a new commit. Exact successful retries require current
authority and return the old receipt without rolling back newer output.

Request IDs are nonzero and scoped by task/domain. The backend retains the full
commit request binding with its receipt; changed fields under a successful ID
fail closed. Accepted reference, revision, binding and receipt must become
durable together before replying. The existing projection helper does not
provide this transaction. Overflow and restart recovery must preserve monotonic
revisions and the A0 bounded-state behavior.

## Replies and bulk payloads

All status values are fixed in `kernel_api::agent_task_protocol`: OK=0,
DENIED=1, INVALID=2, CONFLICT=3, VALIDATION_FAILED=4, EXPIRED=5, TIMEOUT=6,
CAPACITY=7, IO=8, REQUEST_REUSE=9, NOT_FOUND=10. Validation outcomes are
NOT_RUN=0, VALID=1, INVALID=2, TIMEOUT=3, HOST_FAILURE=4. Unknown values fail
closed at the future consumer. `diagnostics_flags` bit 0 reports truncation;
all other bits and all reserved fields must be zero. Truncated diagnostics
cannot establish valid attestation under the A0 contract.

Every reply echoes its request ID. Denied or unauthorized replies expose no
object existence, revision, content hash, diagnostics or capabilities: these
fields are zero, with NOT_RUN for validation. A not-found result is available
only within already-authorized scope. Other errors may expose only the caller's
authorized state. Failed validation may return its own bounded diagnostics,
never unrelated task metadata. Subscribe masks admit output-change (1) and
validation-change (2); each event has exactly one declared event type and an
authorized subscription. Revocation/expiry ends delivery and releases resources.

Input, diagnostic and task-state bytes travel through shared memory with an
explicit length. The A1.1 service must define/version the bulk state schema and
freeze response preflight, mapping rights, bounded lengths, mapping release and
subscription overflow/resynchronization behavior before registering the handler.
A1.0 establishes fixed control layouts only. It does not claim zero-copy through
the current host/WASM bridge, which can copy into guest memory.

## Actual call-path inventory

| Path | Available behavior | Integration gap |
|------|--------------------|-----------------|
| `idl/harness/semantic_store_v1.toml` | Query-only discovery | No task stage/validate/commit operations |
| `services/store_service/src/projection_cow.rs` | Immutable CAS and domain ownership publication | No task endpoint, expected-revision transaction or durable request receipts |
| `services/domain_manager/src/broker.rs` | Manifest/channel policy and interface rights grants | No task/resource/lifetime-bound registry; protocol 14 is not registered |
| `services/semantic_state/src/reactor.rs` | Host snapshot/subscription prototype | Synthetic handles are not real task mappings or run-scoped provenance |
| `services/native_runner/src/runner.rs` | Pinned WASM execution and guest epoch deadline | No isolated validator input/result worker or outer compilation/host-call watchdog |
| `services/native_runner/src/kernel_bridge.rs` | Unix/serial bridge transport | Unix service waits need task deadlines and cancellation; serial timeout is not whole-invocation containment |

## A1.1 assertions required before implementation

The following are planned service tests, not assertions supplied by this gate.
Write them against a real host service transport with forged requests bypassing
the scripted consumer, then implement the bounded RT adapter and worker.

| Assertion | Required observable evidence |
|-----------|------------------------------|
| Useful repair | Seeded config repaired, unrelated settings intact, pinned validator executed, independently checked output reported |
| Forced denial | Wrong domain/task/resource/object kind/rights/generation and workspace B requests denied with no private metadata or effect |
| Stable candidate | Foreign/stale/mutable-alias sources cannot replace bytes after hashing or validation |
| Authentic validation | Wrong candidate/schema/policy/validator pins, forged success and invalid/expired results cannot commit |
| Concurrent output | Revision/hash conflict and A→B→A transitions cannot overwrite a newer accepted output |
| Durable retry | Lost reply and crashes around publication recover one receipt/effect; changed fields under the same ID are rejected |
| Revocation | Revocation during validation or before commit blocks effects; renewal requires fresh validation; expired subscriptions stop |
| Bounded execution | Guest/start loops, stalled backend and compilation timeout terminate/reap worker descendants and release transports/mappings |
| Bounded output | Oversized/truncated diagnostics and state/event queues fail or resynchronize explicitly without leaking unrelated state |
| Audit/replay | Task-scoped ordered evidence binds requests, identities, validation, effects and receipts; independent verifier rejects tampering |

The RT gate remains independently runnable without Linux comparison controls,
models, paid API calls or lab hardware. Its later report must name host service
enforcement per operation; target kernel enforcement and comparative outcomes
require their own evidence.
