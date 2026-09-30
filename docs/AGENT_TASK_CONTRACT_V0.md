# Agent Task Contract V0 — SW0 A0

**Last Updated:** 2026-09-30
**Status:** Pure contract model and deterministic fixtures implemented
**Gate:** `just foundry-agent-task-contract-a0`

A0 freezes transaction semantics before service adapters. Types and the reference
model live in `artifact_store_schema::agent_task`; they perform no IO and supply
no service/kernel enforcement. The JSON fixture contains synthetic content IDs,
not a built validator, real task inputs, credentials, or measured execution.
This gate establishes `environment=host claim=contract-model` only.

## Identity and authority

`TaskContractV0` pins a nonzero task/domain, exact logical output resource,
schema/policy/validator content IDs, and execution limits. Version 1 rejects
unknown fields, invalid content IDs, and invalid budgets. Resource names are
opaque exact-match identities, never interpreted as host paths or prefix grants.

`TaskGrantV0` is a trusted executor input with task/domain/resource, mutation or
read kind, generation and monotonic expiry. It is not an authenticatable token.
All mutations reject a wrong task/domain/resource/kind, revoked generation, or
expired grant. Times come from the backend's controlled monotonic clock, never
an agent request. Natural-language content cannot create a grant. A1 must use
real broker/service authority and define native operations in IDL/codegen.

The accepted output is `(revision, content_id)`. Each successful new commit
increments the revision, including unchanged bytes or A→B→A. Both fields are
required preconditions, so content equality cannot conceal a concurrent change.

## Transition contract

| Operation | Preconditions | Accepted output effect |
|-----------|---------------|------------------------|
| Stage candidate | Current mutation grant; valid immutable content identity; capacity | None; retain private candidate identity |
| Record validation | Current mutation grant; staged candidate; task/domain/resource/generation and schema/policy/validator match | None; replace that candidate's observation, including failure |
| Commit new request | Current mutation grant; matching prior revision/content; staged candidate; matching unexpired successful validation within budgets; capacity | Advance accepted identity/revision and retain original request/receipt |
| Retry successful request | Current mutation grant; identical typed request fields for that request ID | Return original receipt; no new effect, even after later commits |
| Reuse request ID with changed fields | Never permitted for a retained successful request | Reject; no effect |
| Revoke | Increment generation without overflow | No output effect; old grants/validation unusable |

Validation records bind candidate ID, validator ID, schema ID, policy ID,
task/domain/resource, grant generation, validity deadline, outcome, elapsed guest
and wall times, diagnostic byte count and truncation flag. Truncated diagnostics
cannot support a successful commit. A serialized valid result does not
prove validator execution or authenticity. A1 must source it from the supervised
trusted worker, verify immutable bytes against pinned identities, and reject
agent-supplied success records. Revocation requires new authority and validation.

Invalid, timeout, or host-failure observations replace prior success for the same
candidate. Conflicts do not publish anything. A new attempt after conflict uses
fresh preconditions/request ID. Receipt lookup requires current authority; after
revocation, a renewed valid grant may retrieve the original receipt without
executing the old operation. The model retains at most 64 candidate identities
and 64 successful request receipts per task. Capacity fails closed without
eviction; task rollover/garbage collection requires a later explicit policy.

## Budgets and persistence boundary

The fixture declares guest 30000 ms, whole invocation 35000 ms, per-host-call
1000 ms, and 4096 diagnostic bytes. All limits are positive; guest and host-call
limits cannot exceed the outer deadline. A0 caps outer time at 300000 ms and
diagnostics at 65536 bytes, rejects over-budget validation at commit, and checks
that reported guest time does not exceed wall time. It does not run a timer or
measure a worker. A1 must enforce/measure these limits independently of reports.

The reference state and receipts are in memory. A1 must persist the output
reference, monotonic revision, request binding and receipt atomically before
success. Crash recovery must resolve a lost reply by returning the existing
receipt or accepting the first attempt once, never by a second publication.
Test crash points before/after durable publication and restart. The existing
Store CAS/ownership/index sequence does not itself provide that transaction.
Private staged bytes may survive an aborted attempt; accepted output must not.

## Runnable evidence and next milestone

`artifact_store_schema/tests/agent_task_contract.rs` exercises success,
unvalidated/invalid candidates, every identity binding, wrong authority,
revocation/renewal, concurrent/ABA conflicts, lost replies and changed retries,
over-budget/expired results, strict contract parsing, and bounded retained state.
The gate is wired into the extended Foundry suite and needs no network, model
credentials or hardware. A0 does not claim replay/audit completeness, a useful
configuration repair, Linux control equivalence, or target enforcement.

Next is A1: write actual boundary assertions, define/generate missing native
grant/stage/validate/commit operations, integrate one RamenOS scripted consumer,
add the outer validator worker/watchdog and bounded IPC, then prove durable
receipts, forced denials, complete evidence and replay. A2 adds Linux controls;
only then is Phase B model comparison eligible. Physical H0–H3 remain pending.
