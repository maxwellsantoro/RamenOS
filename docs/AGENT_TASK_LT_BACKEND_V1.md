# SW0 A2.3 — independent Linux typed transactions

**Status:** scripted development-fixture point operations implemented.
**Gate:** `just foundry-agent-task-lt` (Linux, Docker, installed pinned image).
**Claim:** an independent Linux broker repairs, validates and durably commits the
shared task through the same JSON contract as RT. Named point cases are compared;
full A2 conformance and model comparison remain pending.

The executable `agent_task_lt_adapter` requires `linux_task_v1_dev`, disabled by
default. It imports the existing Rust request decoder, descriptions, response
serializer and bounded stdin/stdout transport. Its private Python broker owns LT
policy checks and transaction state. LT does not invoke `TaskService`, generated
native IPC, RT's capability table or the A0 reference model for enforcement. The
trusted launcher fixes fixture, store, worker and caller context; requests cannot
select host paths, domains, programs, budgets or a different enforcement backend.
This is a host experiment arm, with no production registration or target claim.

## Transaction boundary

The pinned policy restricts requests to task 17, domain 7 and output resource 1.
Opaque grants carry rights, a generation and monotonic expiry. Each operation
checks current authority in the broker. The policy cap is a separate object kind.
Missing rights, foreign context/resources and wrong validator pins return redacted
denials even in direct broker tests that bypass Rust's syntax checks. Revocation
advances the task generation and removes all outstanding grants. Restart advances
it again, invalidating prior grants and validation freshness. Candidate handles
and receipt IDs remain durable locators requiring a current authorized grant.

The three logical read resources map to sealed initial configuration, schema and
untrusted notes. Staging seals bounded bytes into private SHA-256 files. The broker
rechecks content on use; agent-supplied outcomes and filesystem result files have
no role. Only a successful, untruncated, budget-compliant observation of the pinned
validator for that candidate and current generation permits a new commit. Starting
another validation clears its previous success before running the worker. A commit
compares both revision and content hash, preventing stale and ABA publication.

One private journal atomically publishes accepted revision, content pointer,
request binding, original receipt and validation evidence. Writes synchronize the
file, replace it, then synchronize its directory. A writer lock excludes concurrent
brokers. IO uncertainty poisons the live session. A lost response can represent a
durable commit: after restart the caller obtains current authority and explicitly
looks up or retries the original request. Exact retry returns its original receipt
without another effect or rolling back a later revision; reusing the ID with changed
semantic fields fails. Recovery verifies pins, sealed bytes, revision order and
original validation evidence; unconfirmed container removal blocks reopening until
trusted reconciliation. It does not rely on JSON key ordering.

Bounds are 64 candidates, 64 receipts, 256 session grants, 128 validator run records,
a 4 MB journal, 65536 candidate/input bytes and the shared 131072-byte JSON frame.
Runtime binaries are streamed with a 512 MiB ceiling and copied to a private sealed
file; this avoids loading a large debug binary into a single buffer. The accepted
artifact lives in CAS behind the journal pointer, not a second unsynchronized
published output file. Trusted test artifacts are not writable by the validator.

## Linux enforcement and available authority

The broker uses A2.1's inspected Docker substrate. A validation receives only a
read-only sealed worker and a read-only subset containing that candidate, schema
and validator. The journal, other candidates/receipts, workspace B and host daemon
socket are absent from its mounts. The worker runs as UID/GID 65534, with read-only
root, dropped capabilities, no-new-privileges, default seccomp, private namespaces,
network none and bounded CPU/memory/PIDs/scratch/output. Named containers are removed
and absence checked before a run is accepted. Configuration, image, elapsed time
and cleanup observations remain private evaluator evidence.

The trusted broker has wider host filesystem and Docker authority as a deputy.
The model transport offers only the eight typed point operations. This gate does
not isolate the external consumer from the broker at the host process/user level;
its consumer is a trusted scripted harness. The validator still has image files,
private scratch and process metadata available beyond its three task blobs. A2.1
supplies the underlying runtime probes, including retained-descriptor behavior.
These facts do not establish equal or narrower available authority relative to RT.

Validation uses the same 1500 ms guest / 2500 ms wall / 1000 ms host-call / 4096-byte
diagnostics budget as RT. Preparation and remaining worker time consume the wall
budget. Cleanup uses the substrate's separate bounded waits; slow cleanup can make
LT fail the overall elapsed check. Launcher startup, interactive input waits and
stdout backpressure do not have a whole model-session deadline. Abrupt destruction
of the host broker/daemon is not a proved container cleanup path. A future evaluator
must add session supervision and reconciliation before unattended model trials.

## Evidence and comparison limits

The gate runs direct broker assertions, Rust codec tests, the independent executable
consumer/schema reader, and shared RT/LT development cases. Direct tests include
unvalidated/invalid publication, wrong rights/kinds/context/pins, expired/revoked
grants, CAS tampering, writer exclusion, changed request reuse, stale revisions,
retry after later revisions/restart, corrupted receipt evidence and an injected
failure after journal replacement. That last case proves honest uncertainty and
explicit recovery, rather than treating an IO response as proof of no commit.

Both executables emit byte-identical `--describe` artifacts. The shared case driver
compares request/response traces, preserving rights, statuses, outcomes, bytes,
content/pin hashes, revisions and diagnostics. It normalizes only consistently
aliased opaque capability identities and the declared clock/duration fields:
`expires_at_ms`, `now_ms`, `valid_until_ms`, `wall_elapsed_ms`, `guest_elapsed_ms`.
It checks real successful validation and publication separately from trace equality.
This is a bounded point-case check, not a proof of complete protocol equivalence.

`out/agent-task-lt/` contains the tool contract, consumer and paired point traces,
source/lock/binary fingerprints, actual Docker run observations and a report.
LT has no RT-style complete durable operation audit or asynchronous event stream.
[A2.4](AGENT_TASK_LS_TRANSACTIONS_V1.md) supplies LS commands sharing this Linux
engine. [A2.5](AGENT_TASK_SUBSCRIPTIONS_V2.md) adds an independent in-memory typed
subscription lifecycle and shared version 2 description/codec. All-arm authority
inventories/conformance, hidden fixtures and model evaluation remain next. There
are no model calls, physical runs, merge authority or security/graduation claims.
