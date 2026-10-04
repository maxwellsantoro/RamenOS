# SW0 A2.4 — Linux scoped-shell durable transactions

**Status:** scripted development-fixture commands and launcher implemented.
**Gate:** `just foundry-agent-task-ls-transactions` (Linux, Docker, installed pinned image).
**Claim:** a contained shell consumer repairs, seals, validates and durably commits
one task, with explicit original-receipt recovery. Full A2 and model trials remain pending.

The trusted `tools/agent_task/ls_launcher.py` takes fixture, journal, candidate and
worker paths as operator arguments. It accepts bounded JSON lines containing only
`schema_version`, decimal-string `request_id` and a shell `command`. Each command
runs through `/bin/sh` in the measured Linux sandbox; no host shell executes model
text. The launcher returns bounded container stdout/stderr and exit status. A
successful shell invocation with a nonzero exit code remains observable. Transport
or deadline failure is separate from command exit. Nothing repairs or retries on
the consumer's behalf. An abandoned commit response can represent a durable effect.

This launcher is opt-in host tooling, not a registered native service or production
model integration. It introduces no native interface or target-kernel claim. The
shell agent actually runs in a separate nonroot Linux container; this is more
client isolation than the trusted scripted LT/RT consumer harnesses currently prove.
A frozen comparison must account for that difference rather than assigning it to
an OS substrate advantage.

## Agent-visible files and commands

The shell sees a read-only `/inputs` directory, a writable `/candidate` directory,
and a read-only `/task` directory containing bootstrap data, command helper and a
single task-scoped Unix socket. Each command gets fresh private processes/scratch;
candidate files and broker state persist between commands. The current development
inputs include configuration, schema, notes, policy and validator bytes, as in
A2.1. Those extra readable pins and directory enumeration are broader observations
than RT's three logical read resources, and must remain in the authority inventory.

`/task/taskctl --request-id ID VERB ...` offers:

| Verb | Meaning |
|------|---------|
| `grant` | Request bounded task rights with the bootstrap policy cap |
| `read` | Read a logical input; optional `--output` writes a file inside the container |
| `stage` | Seal bytes from `--file` into a durable candidate |
| `validate` | Execute the pinned validator for that candidate |
| `commit` | Compare revision and content hash, then publish with a durable receipt |
| `receipt` | Retrieve the original commit receipt using current authority |
| `state` | Observe task pins, revision, rights and validation freshness |
| `revoke` | Advance the task generation and revoke outstanding grants |

The helper does not choose a repair. A stage path never reaches the host broker:
it opens a regular, nonsymlink, nonblocking file in the container, bounds its
bytes and sends only those bytes. Read output similarly writes only in the
container and rejects symlinks/FIFOs before truncating. A private agent-owned
file can remain unreadable to the host evaluator; grading uses accepted private
CAS bytes, never silently broadens filesystem permissions.

The helper returns a JSON command result and exits 0 for an OK transaction,
1 for a broker error, and 2 for a helper/transport failure. CLI errors and exit
statuses are ordinary shell feedback. Read-to-file omits base64 from stdout and
reports written byte count. This is deliberately the shell interface, not the
byte-identical typed tool-description contract shared by LT and RT.

## Transaction, connection and cleanup boundary

LS shares the independent Linux transaction engine with LT, keeping this Linux
interface contrast explicit. The broker owns policy grants, monotonic expiry,
generation, sealed CAS, supervised pinned worker and synchronized revision/receipt
journal. Current authority is required even for recovered receipt lookup. Reusing
a commit ID with changed semantic fields fails; exact retry returns the original
receipt without a second effect or rolling back a newer revision. Invalid or
unvalidated bytes, wrong rights/kinds/pins and foreign resources cannot publish.
Model files claiming a successful validation have no role.

The fixed socket is mounted only into that task. The host endpoint's parent stays
private; read-only mounts prevent replacing helper/bootstrap/socket entries. The
broker obtains actual Linux `SO_PEERCRED` and requires UID/GID 65534, with domain 7
bound by its trusted launcher. A host process with a different UID is denied even
if it knows the endpoint. This requires the tested rootful Docker UID mapping;
other mappings fail closed. Raw bounded socket packets are available to the shell
and its descendants, so Rust syntax filtering is not its authority boundary.
Duplicate/deep/oversized/partial records have no mutation authority. Request reads
use a whole one-second deadline. There is one request per connection, no pipelining,
no passed-descriptor API and no asynchronous subscriptions. Ancillary descriptor
injection is exercised without growth in the host broker's descriptor inventory.

Before launching a shell, the broker synchronizes a pending cleanup checkpoint.
After verified container removal it replaces that checkpoint with measured run
configuration/outcome. An uncertain create/removal or journal write poisons the
session; recovery refuses a pending or reconciliation-required run. The operator
must reconcile it before reopening. This is fail-closed recovery, not an automated
reconciliation service. Abrupt host/daemon failure remains unproved cleanup, but
cannot silently produce a clean restart from that checkpoint. Closing a normal
session waits for its dispatched broker operation before releasing the writer lock.
A commit can finish after its client disconnects, so callers use explicit receipts.

The validator receives a separate read-only sealed worker and candidate/schema/
validator subset. It retains LT's 1500 ms guest, 2500 ms wall, 1000 ms host-call and
4096-byte diagnostics budgets. The shell keeps network none, read-only root,
nonroot UID, dropped capabilities, no-new-privileges, default seccomp and bounded
CPU/memory/PIDs/scratch/output. It has image helpers, private processes/metadata,
candidate-directory access and own-descendant delegation beyond typed operations.
The trusted broker separately holds host filesystem and Docker deputy authority.
These facts establish neither equal nor narrower authority across arms.

The launcher allows 8192-byte command frames, 16384 combined output bytes, at most
128 commands and an operator-selected command deadline (default 10 s, maximum
35 s). Transaction RPC frames are bounded at 131072 bytes; candidate bytes remain
65536. The shared journal retains at most 128 container-run records, including
shells and validators. Broker transport records cap at 2048 per session. Startup,
interactive input, output backpressure and separate cleanup waits have no whole
model-session budget yet. Shell checkpoint/audit overhead is explicit extra trusted
instrumentation; future trials must freeze and account for it.

## Gate evidence and remaining work

The gate drives a real shell consumer through all eight commands, preserves its
unrelated configuration field, and grades accepted CAS bytes independently of
shell assertions. It exercises immutable staging after client edits/forged files,
current-authority denials, revocation, exact retry/reuse, receipt recovery after
restart, and abandonment of a real commit reply followed by explicit lookup.
Transport cases cover wrong peer UID, malformed/deep/oversized/stalled packets,
passed descriptors, protected writes/private reads, nonzero exit feedback,
launcher bounds and injected uncertain shell creation. Injected cases are labeled.

`out/agent-task-ls-transactions/` records source/lock/worker/helper fingerprints,
actual shell/validator runs, task pins and bounded broker transport records. Those
records contain request hashes, operation/ID, real peer identity, status and whether
the socket write succeeded. A successful write is not a client-delivery acknowledgment.
The journal durably reconstructs accepted effects/receipts; the session transport
log is in memory and does not prove complete crash-persistent operation audit.

Later typed subscriptions, finite authority cases, evaluator/session controls
and reconciliation have separately bounded gates. Complete time-indexed authority,
real study releases, provider/cost accounting and full all-arm conformance remain
pending in [Next Tasks](../NEXT_TASKS.md). No model or physical trial runs here.

A2.5 adds [version 2 typed subscriptions](AGENT_TASK_SUBSCRIPTIONS_V2.md) to
the shared Linux broker. The conventional helper remains eight version 1 verbs;
raw LS clients can reach those subscription packets. Their lifetime is the whole
LS launcher session across per-command socket closes. Include that additional
available observation authority and lifecycle difference in all-arm conformance.
