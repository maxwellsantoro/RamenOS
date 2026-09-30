# SW0 A2.2 — shared JSON contract and RT adapter

**Status:** scripted host adapter implemented; full A2 and model evaluation pending.
**Gate:** `just foundry-agent-task-adapter`
**Claim:** the useful task runs from an external scripted JSON consumer through
actual protocol-14 IPC and the named A1.1 host service boundary.

`tools/agent_task_adapter` owns one backend-independent request/response codec,
tool description/schema artifact and canonical encoding. Its default library has
no Store backend dependency. The RT bridge and standalone executable require
`agent_task_v1_dev`, disabled by default. It is trusted host fixture tooling, not
production authentication, a client sandbox, a model provider integration or a
target-kernel authority boundary. The independent LT backend remains next.

## Operations and data

The contract publishes eight operations: `request_grant`, `read_input`,
`stage_candidate`, `validate_candidate`, `commit_candidate`, `get_receipt`,
`get_task_state` and `revoke_grant`. Each translates to one corresponding generated
native request. Stage also installs and releases its source mapping; reads,
state and validation consume and release reply mappings. The adapter never
chooses a repair, validates in place of the pinned worker, retries a mutation,
or calls evaluator/audit methods on behalf of the consumer.

There is no shell, filesystem path, domain, arbitrary program, CAS root, execution
budget override or diagnostics path in a request. The trusted launcher supplies
fixture/store/worker paths and a fixed caller context outside this JSON contract.
A well-formed guessed cap, foreign resource, wrong kind, wrong validator pin or
excess right goes to the actual service for its authorization decision. Syntax
rejection is not counted as backend enforcement. A policy capability permits
requests within the pinned task policy and generation-wide revocation; it grants
no wider resource allowlist or production broker authority.

The executable emits a bootstrap record with task ID, policy cap and three
resource bindings. Development IDs are `resource:0000000000000001` for
`workspace:a/config`, `resource:0000000000000064` for `task:schema`, and
`resource:0000000000000065` for `task:notes`. These are virtual resource names for
both typed arms, with explicit backend mapping required for LT. The bootstrap
contains no private workspace inventory, evaluator journal or grading result.

Each request has `schema_version: 1`, a nonzero decimal-string `request_id` and a
`call` object with `operation` plus the operation's exact fields. For example:

```json
{"schema_version":1,"request_id":"2","call":{"operation":"read_input","task_cap":"cap:0123456789abcdef","resource":"resource:0000000000000001"}}
```

Use the actual grant cap returned by `request_grant`; the example cap mints no
authority. Caps are nonzero `cap:` plus 16 lowercase hex digits. Resource IDs
use the same nonzero hex width after `resource:`. Content IDs are canonical
`sha256:` plus 64 lowercase hex digits. Request IDs, revisions, generations,
task IDs and millisecond offsets are decimal strings through `u64::MAX`, without
signs or leading zeroes. This preserves integers above JavaScript's exact-number
range. Rights are a duplicate-free list from read/stage/validate/commit/observe.

Bulk bytes use canonical padded base64, decoded at most 65536 bytes. A candidate
must be nonempty. Each JSON request/response is capped at 131072 bytes. Unknown
fields/operations/versions, duplicate fields, noncanonical encodings and excessive
sizes fail before dispatch. The request/response/bootstrap schemas are emitted
by `agent_task_rt_adapter --describe` from the same shared library; independent
JSON Schema checks exercise valid requests and important bounds. The Rust decoder
remains normative for strict parsing, including integer lexical forms.

Responses have version, request ID, status and a typed `result` or null. Syntax
errors use a null request ID because no request was accepted. Denied/expired
responses always have null result. Validation outcomes and bounded diagnostic
bytes are explicit; other backend errors never acquire success-shaped data.
Task state reports real pins, accepted revision/hash, current grant rights,
expiry, service clock offsets, latest validation and its separate freshness flag.
No target or model result is synthesized.

## Transport, recovery and observations

A trusted launcher connects the bridge to the Store task service with a private
Unix-stream pair bound to the fixture domain. The separate scripted consumer
uses stdin/stdout JSON lines. Native control messages stay typed and generated;
bulk messages use A1.1's scoped host mapping provider. The JSON bridge copies
bytes for serialization and makes no target zero-copy or separate client-process
sandbox claim.

The CLI bounds frame allocation before parsing; oversized frames terminate the
session without dispatch. It waits for input interactively, so whole model-session
idle/cost budgets belong to the future evaluator. Native reply waits and validator
execution retain A1.1's transport/worker deadlines. EOF shuts down the connection
and joins its handler before releasing the writer lock. A lost reply or transport
failure ends the CLI with a generic diagnostic and no automatic retry; a commit
may already be durable. The caller must explicitly obtain a current grant and
retry the same commit or query its original request ID after recovery.

This standalone launcher suppresses existing libraries' verbose stderr during
backend operation. Structured stdout is the model transport; generic startup/
termination diagnostics disclose no host paths or backend records. The saved
stderr descriptor is close-on-exec. Durable service audit remains in the private
journal; in-process evaluator APIs and that journal never become JSON operations.

Subscriptions remain available only in the native A1.1 API. This adapter does
not subscribe, paginate or deliver buffered events. Explicit `get_task_state`
reads provide its current observation contract. Shared model subscription,
coalescing and revalidation semantics need a separate versioned extension before
full A2 equivalence. No generated native interface is altered by this wrapper.

## Gate evidence and next work

The gate needs Python `jsonschema` with Draft 2020-12 support; CI installs the
system package. It runs default-feature codec assertions and opt-in RT tests,
then exercises the built executable with an independent consumer/schema reader.
The consumer inspects inputs/state, chooses its repair, stages/validates/commits,
checks receipts and exact retries, revokes authority, and retries after restart.
Forced cases cover foreign context/resources, object kinds, missing rights,
wrong pins, invalid/unvalidated candidates, reused commit IDs, malformed fields,
base64/ID bounds and oversized stream frames. Mapping counts return to zero.
An independent A0 verifier replays the service's durable receipt evidence.

`out/agent-task-adapter/` contains the shared tool contract, ordered consumer
transcript, private evaluator journal, source fingerprints and report with binary,
worker, lockfile and schema hashes. `RAMEN_TASK_ADAPTER_EVIDENCE_DIR` selects an
output directory. Keep these evaluator artifacts outside model context except
the tool descriptions, bootstrap and the current run's authorized responses.
Count all exposed schemas and messages in future interaction-cost measurements.
Tool-contract bytes are identical across hosts; opaque caps, native timings,
binaries and transcripts legitimately differ and retain their own provenance.

LT/RT equivalence cannot be reported until LT uses this same codec/descriptions
and an independently enforced Linux transaction backend. LS transaction commands,
model subscriptions, common lifecycle probes, complete effective/exercised
canonical authority mapping and a disjoint hidden fixture bank also remain.
Full A2, Phase B comparison, production/target integration and physical HIL are
still pending. No model credentials or paid service are used by this gate.
