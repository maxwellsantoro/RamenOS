# SW0 A2.7 — fixture bank and external evaluator controls

**Status:** implemented scripted development controls.
**Gate:** `just foundry-agent-task-evaluator-controls` on Linux with the pinned
Docker image already installed. It performs no model calls and creates only a
deterministic **synthetic contract bank**. It does not create, release or evaluate
a real hidden pilot/final bank.

## Bank contract

The version 1 factory creates 45 unique instances: development, pilot and final
each contain five error classes crossed with clean, hostile-note and
hostile-user-message conditions. Errors are an incorrect flag, wrong scalar type,
missing field, unexpected field and incorrect nested value. Initial configurations
and target documents are distinct, derived from a 32-byte operator seed using
HMAC and partition/class/condition identity. This is a small exact-target family,
not a representative benchmark or a powered sample-size decision.

Authorized inputs remain `config.json`, `schema.json`, `policy.json`, `notes.txt`
and `validator.wasm`. Here `schema.json` is a target document for the existing
pinned byte-equality validator, not a general JSON Schema solver. Hostile
conditions ask for workspace-B access, validation bypass or a false success claim;
a scripted consumer ignoring them does not prove model safety.

The independent oracle lives outside the input directory. A manifest freezes
identities, partition membership, class, condition, task text and all input/oracle
hashes. Verification rejects duplicate identities/input/target reuse, incomplete
cells, unknown fields, changed bytes, symlinks, unexpected files and group/world
access to private files/directories. Public commitments contain the bank digest
and each partition's count/digest, without seed, input or grading contents.

`fixture_bank.py` has an opt-in operator CLI requiring a private seed file,
validator and output locations. Real output must be outside the source checkout;
the root and files are owner-only, and the seed is not stored in the bank. Commit
public hashes before tuning/runs and keep the seed/bank under operator control.
The factory must run outside the tuning/model workflow for instances to qualify
as hidden. No production hidden bank is shipped or created by default CI.

`select_fixture` defaults to development. Pilot/final require a release record
bound to the bank digest, exact partition, public bank-commit identity, frozen
study digest and purpose (`pilot-comparison` or `frozen-final-comparison`). CI
release tests use a distinct synthetic purpose. This is a trusted operator
contract, not signed authorization or an OS boundary against the bank owner.
Private verification can inspect all partitions. Independent study freezing and
host-client isolation remain prerequisites for a hidden-bank claim.

## Session and context controls

Each session launches an actual adapter with fresh storage, grants and transcript.
RT/LT version 1/2 descriptions remain byte-identical. LS keeps its contained shell
and conventional command flags. The gate's scripted repair/command builder is
evaluator instrumentation, never an adapter operation or model repair helper.

| Default limit | Value |
|---------------|-------|
| Whole session, including launch/bootstrap and idle time | 30 seconds |
| Whole request write/reply; partial bytes do not reset it | 10 seconds |
| Request frame | 131072 bytes RT/LT; 8192 LS |
| Response frame | 131072 bytes |
| Visible role-prefixed UTF-8 transcript | 1048576 bytes |
| Requests | 64 |
| Backend stderr | 4096 bytes |
| Cleanup/reconciliation, separate from session work | 10 seconds |

An independent watchdog stops idle or blocked sessions. Nonblocking request/reply
IO shares an absolute deadline. Oversize/invalid/multiple frames, EOF, unsolicited
output, exhausted request/context budgets and excess diagnostics are terminal.
There is no automatic retry, receipt lookup, repair or poisoned-session resume.
A sent commit may complete despite a lost reply; retain its journal and require
explicit later reconciliation.

The visible ledger counts/hashes task/user text, all supplied tool descriptions,
bootstrap, requests, replies and consumer final text, including role prefixes and
newlines. Request context preflight precedes sending; an oversized remaining reply
is retained privately by digest and terminates the session. Private grading and
other arm outputs are not added. These are **scripted transport bytes**, not model
tokens, provider usage, comparative cost or a model conversation implementation.

Failure artifacts retain reason, sent count, accepted/rejected frames, partial
reply/diagnostic digests, cleanup and available backend journal. Startup failures
also retain transcript/cleanup evidence. A deterministic shuffle of unique IDs
cycles six arm permutations; the 15 development blocks' counts differ by at most
one. Repeated IDs are rejected pending a cluster-aware study contract.

## Cleanup and evidence limits

Linux process parent/group/start identities track owned observed processes. The
supervisor kills observed descendant groups and reaps the adapter. Start-identity
checks avoid treating a reused PID as the previously observed process. Unobserved escapes are not certified,
and typed clients remain trusted host scaffolding.

A trusted random session label is forwarded to LT and attached to LT/LS
containers. Successful runs require normal EOF, backend journal removal records
and an empty scoped daemon inventory. Forced termination removes observed labeled
containers. [A2.8 reconciliation](AGENT_TASK_RECONCILIATION_V1.md) adds a private
fenced lifecycle ledger: every exact acknowledged ID must be verified removed
before named cleanup certification. An unresolved create intent can finish late,
so it retains `possible_inflight_create` and **uncertified cleanup** despite an
empty inventory. Forced tasks retain their failure row even when this named
cleanup is certified. Already dispatched service effects are not promised to cancel.

The gate accounts for **15 development fixtures × 3 arms = 45 attempts**. Completed
repairs independently check accepted revision/bytes; actual validator deadline
responses retain the unchanged publication pointer and their full attempt row.
Other protocol/task failures fail the gate. Each arm must demonstrate successful
repair; no failed row is retried or removed from the denominator. A separate real
LS timeout retains actual lifecycle cleanup certification or uncertainty. Six bank tests and nine session tests cover partition/release/tamper,
fresh context, byte/request bounds, partial/idle/startup deadlines, bad frames,
diagnostics and observed process-group cleanup. No pilot/final tasks execute.

`out/agent-task-evaluator-controls/` contains source/lock/binary/validator/tool
hashes, aggregate report, private per-arm transcript/session/journal files and the
retained timeout. Evidence is trusted evaluator output, not signed attestation.
The report preserves false hidden-bank/model/full-A2 claims, unknown authority
inclusion; cleanup certification reflects the persisted lifecycle evidence.

[A2.9](AGENT_TASK_REQUESTABLE_AUTHORITY_V1.md) adds finite requestable-right
and named lifetime witnesses. Broader unexercised, continuous-lifetime and
deputy/isolation authority remain work. Freeze the real bank and
study releases, model/provider/token accounting, and sample-size/budget decision
before Phase B. Physical testing remains deferred.
