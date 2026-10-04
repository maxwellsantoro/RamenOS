# Agent Task Reconciliation V1

SW0 A2.8 supplies **named host lifecycle and interrupted-commit recovery evidence**
through actual RT/LT adapters and contained LS commands. Run
`just foundry-agent-task-reconciliation` on Linux with the existing pinned Docker
image. Features and native/JSON contracts retain their existing opt-in scope.
This is scripted development evidence; full A2, model comparison, target-kernel
isolation and physical testing remain pending.

## Lifecycle evidence

A trusted evaluator session gives LT/LS a private owner-only ledger directory and
random scope. Every contained invocation persists an intent before Docker create,
then the exact immutable container ID returned by a successful create, then
verified removal. The ledger bounds entry count and validates schema, scope,
identity, transitions, uniqueness, ownership and permissions. Atomic replacement
and file/directory fsync preserve acknowledged checkpoints. Bounded lock
acquisition fails closed instead of stalling evaluator termination. It is unsigned trusted
host evidence, inaccessible through the container mounts, not protection against
the host owner or daemon administrator.

Before stopping owned observed processes, the evaluator persistently fences the
scope. New intents are rejected; an already-issued create can still finish.
Cleanup removes observed scoped objects, checks the empty label inventory, and
verifies acknowledged IDs are absent. Certification requires a fenced valid
ledger, every intent resolved to a removed ID, no untracked scoped containers,
and successful verification within the separate cleanup deadline.

An empty inventory **cannot resolve an unacknowledged create intent**. That run
stays uncertified. The gate prepares such an intent, interrupts the evaluator,
observes an empty inventory, then creates the actual Docker object late. Only
explicit acknowledgement and verified removal permit subsequent certification;
a stale start using the old immutable ID fails. This controlled late-create case
models the unresolved RPC boundary; it does not intercept or prove all daemon
failure modes. Acknowledged forced runs can now certify this named lifecycle
boundary, superseding A2.7's coarse uncertainty for every container-capable call.

Unobserved process escapes, daemon restart, host crash/power loss, malicious
host-client/deputy behavior and general isolation remain unproved. Unknown or
malformed evidence fails closed. There is no guardian service or automatic
poisoned-session resume, and cleanup does not cancel already-published task effects.

## Explicit journal reconciliation and receipt lookup

LT validators and LS shells persist a pending run checkpoint with the invocation
identity before creating a container. An interrupted checkpoint blocks ordinary
restart. A separate trusted reconciliation function accepts the matching fenced
ledger proof, takes the existing nonblocking exclusive writer lock, and replaces
only unresolved run cleanup rows whose exact identities have removed receipts.
It preserves every other journal field, including accepted revision/content,
commit receipts, candidates and grants. Each repaired row retains the prior row
hash and reconciliation scope/ledger hash. Repeated reconciliation changes no rows;
an active writer, changed proof or unknown intent cannot repair the journal.
The evaluator never invokes this function automatically on session failure.

The gate prepares a valid candidate in all three arms and interrupts either
before dispatching commit or after observing durable publication while abandoning
the reply. Each restart denies the old task capability. A fresh authorized grant
includes commit rights, as required by the existing receipt contract. Two explicit
receipt lookups confirm either no publication/receipt, or exactly one revision,
one original receipt and the expected accepted CAS bytes. The commit is never
resent. LS's after-publication command remains sleeping when interrupted, so the
case exercises a real pending shell checkpoint and its explicit repair. The
existing native abrupt-process-crash test also covers failure before journal
publication and exit after durable publication.

## Gate artifacts and remaining work

`out/agent-task-reconciliation/` retains six cases, session context/journals,
private lifecycle ledgers, the late-create case, source/lock/binary hashes and an
aggregate report. Recovery checks compare non-cleanup journal fields directly;
private grading confirms revision, receipt count and accepted bytes independently
of consumer final text. No pilot/final bank selection or model credentials are used.

[A2.9](AGENT_TASK_REQUESTABLE_AUTHORITY_V1.md) adds finite requestable-right
and named lifetime witnesses. Extend broader unexercised/continuous authority
and resolve or explicitly bound host-client/deputy differences. Real bank/study
freezing and provider/token accounting precede full A2 or comparative collection.
Unacknowledged daemon work remains quarantined until explicit evidence resolves it;
this gate does not imply a universal forced-termination guarantee.
