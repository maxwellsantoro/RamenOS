# SW0 A1.1 — scripted host service proof

**Status:** implemented, with deterministic host assertions.
**Gate:** `just foundry-agent-task-proof-rt`
**Evidence:** one useful scripted task and the named host service boundaries below.
No model comparison, target-kernel task enforcement or physical result is claimed.

The gate repairs a seeded configuration, executes its pinned WASM validator,
commits the immutable result and independently checks the output and preserved
settings. Requests use generated protocol-14 messages over actual Unix-stream
IPC. Tests bypass the convenience client to exercise backend denials and crash
recovery. The service is `store_service::agent_task`; the private worker is
`native_runner`'s `task_validator_worker` binary.

## Opt-in scope and trusted inputs

Both components require `agent_task_v1_dev`, disabled by default. **WARNING:
this is a host fixture boundary, not production authentication or a sandbox for
arbitrary host clients.** The normal Store listener and production broker do not
register protocol 14. A trusted launcher provides connected transports with a
fixed caller domain, the exact task fixture, policy capability and worker path.
There is no wire domain negotiation, filesystem path or arbitrary program field.
The data-plane provider is a bounded table of file-backed shared-memory mappings
in the host proof process; it checks domain, generation, rights and expiry.
The scripted client and service run on separate threads; the validator runs in
its own process. This gate does not establish process isolation for the client,
SCM_RIGHTS transfer, a native WASM client adapter or target kernel handle checks.

The strict `TaskPolicyV1` artifact names the task/domain/output resource, allowed
rights and maximum grant lifetime. Its bytes must match the pinned policy hash.
Requests exceeding that policy fail closed. Resource 100 is the pinned schema;
101 is untrusted task notes; the configured output resource is the current
configuration. These identifiers cannot collide. The fixture grants read access
to those inputs only. Workspace B is outside the service's resource inventory.

## Validator and budgets

The fixture uses an intentionally small equality-schema dialect: canonical JSON
must equal the supplied expected configuration. The worker parses and compactly
serializes candidate/schema JSON; the pinned WASM program compares their bytes.
It provides no repair helper. The evaluator separately checks the accepted
configuration and unrelated settings. This is not general JSON Schema support.

The worker has an empty Wasmtime linker: guest filesystem, network, process and
other host imports are unavailable. It verifies candidate, schema and program
CAS hashes itself. Those CAS reads, job/result IPC, JSON preparation, compilation,
instantiation and guest execution are inside the service's external watchdog.
Guest epoch interruption also covers module start sections. Candidate/schema
input and normalized buffers are each bounded to 65536 bytes; program size is
bounded to 1 MiB; guest linear memory is limited to 16 MiB.

`ValidatorInputHeader`, generated from IDL, occupies guest memory bytes 0–15:
magic `0x31545652`, candidate length, schema length and zero reserved field.
Candidate bytes start at 16; schema bytes follow them. The module exports
`memory` and `_start() -> i32`; zero means valid. IDL message type 20 reserves
this private data layout and is **not** a dispatchable IPC request. It creates
no host import or authority. Instantiation precedes data injection, so a module
start section cannot rely on these input bytes.

The CI fixture uses a 1500 ms guest budget, 2500 ms worker wall deadline,
1000 ms transport deadline and 4096 diagnostic bytes. Loop tests reduce the
guest budget. The experimental 30/35-second defaults remain a later A2 freeze
across all comparison arms. The watchdog kills the worker's private process
group and waits for the worker on every exit. Linux additionally acts as a
subreaper and proves owned-descendant reaping; macOS delegates orphan reaping
to launchd. Linux workers have CPU and 2 GiB address-space limits before
compilation. This is containment for the named worker, not a general Linux
sandbox. At most two validators run concurrently per task service.

Diagnostics are bounded, explicitly flag truncation, and never embed unrelated
inputs or host paths. A truncated or over-budget result cannot authorize commit.
The worker cannot publish output. After it returns, the service rechecks current
authority/generation; revocation while it runs discards the result. Normal
service durability operations still depend on the host filesystem and scheduler;
this gate makes no hard real-time or physical power-loss guarantee.

## Durable transaction and observations

Each service instance owns one fixed task/output resource and an exclusive
writer lock. It uses the existing immutable CAS publisher and durable domain
ownership registry in a private CAS namespace. Staging copies the registered
source mapping into immutable bytes before hashing/publication. Later source
writes cannot alter the candidate. Staging and validation do not publish output.

One bounded journal contains the accepted reference/revision, semantic request
bindings, original receipts, trusted validation observations and dispatched-call
audit. A candidate journal is written and synchronized, renamed over the current
journal, then its parent directory is synchronized before success. Persistence
failure poisons the live service until recovery, so tentative memory cannot
become an acknowledged output. Restart validates the journal, receipt chain,
object bindings and CAS hashes; it increments the grant generation and does not
resurrect authority or successful validation for new commits.

Candidates and receipt IDs remain stable, task/domain-scoped **locators** across
restart/revocation. They grant no operation by themselves. Every use requires a
current task grant; exact successful retries can substitute a renewed grant for
the original `task_cap`. Request ID, candidate locator and revision/hash
preconditions must still match the saved semantic binding. Such retries return
the original receipt without changing a newer output. Receipt lookup requires
COMMIT authority; OBSERVE alone is insufficient. The A0 candidate/receipt limits
remain 64 each without eviction. Other host registries are capped at 256 objects
and the dispatched-call audit at 2048 records; exhaustion fails closed.

`TaskSnapshotV1` is strict version-1 JSON, at most 4096 bytes. It reports this
task's accepted revision/hash, schema/policy/validator pins, input resource IDs,
current grant rights/generation/expiry, service `now_ms` and latest validation. Validation freshness
is distinct from its outcome. Expiry timestamps use the service instance's
monotonic clock; restart invalidates old grants before issuing new timestamps.
Snapshot/input/diagnostic mappings are read-only to the consumer and tied to its
grant. The host mapping bridge supports explicit release; expiry/revocation
removes access and cleanup releases backing mappings/files.

Subscriptions bind to their issuing connection and observer grant. Each retains
only the latest pending notification of each of the two event types; delivery
uses a fresh matching-revision snapshot and rechecks authority under the same
lock as revocation. Consumers can resynchronize through `get_task_state`.
The convenience RPC helper preflights replies and skips queued notifications;
`next_event` consumes notifications explicitly. Mapping exhaustion drops a
notification, so state reads are the recovery path. No reliable event history
or target-native shared-memory lifecycle is claimed.

## Assertions and inspectable evidence

The gate runs the wire/A0 suites, supervisor assertions and service tests for:

- useful repair, current input/state reads and preserved settings;
- forced domain/resource/rights/object-kind/pin denials with redacted replies;
- immutable staging, foreign-source denial and read-only scoped mappings;
- unvalidated/invalid/expired/revoked results, including revocation during a run;
- concurrent revision/hash conflicts, ABA and old-receipt retries;
- failed journal publication, exclusive writers and corrupt recovery records;
- abrupt crashes before publication and after durability but before a wire reply;
- actual CAS-backend stalls, guest/start loops, oversized/truncated worker output,
  partial-frame deadlines and bounded concurrent workers;
- connection-scoped events, expiry/revocation and private-canary preservation;
- independent receipt/validation replay and rejection of tampered audit records.

`out/agent-task-proof-rt/` contains `report.json`, private `journal.json`, source
file fingerprints and a host manifest. `RAMEN_TASK_EVIDENCE_DIR` selects another
output directory. The report contains exact fixture pins and accepted content
ID. The verifier binds external expected contract/initial bytes, checks ordered
receipt effects and replays each saved commit through independent A0 semantics.
The audit hashes detect the tested modifications; they are not signatures or
proof against an operator who can rewrite the entire trusted journal. It covers
dispatched calls and receipt replay, not a replay of every OS event or byte that
failed transport parsing. Keep evaluator journals out of the agent's context.

Downstream Linux adapters, shared serialization, finite authority cases, evaluator
controls and reconciliation have their own [landed scopes](../CURRENT_STATUS.md).
Full A2 conformance, remaining host/deputy and continuous authority, real study
controls and comparative runs remain pending.
Production routing/authentication and task-specific kernel enforcement remain
separate work; physical HIL continues to await test-hardware setup.
