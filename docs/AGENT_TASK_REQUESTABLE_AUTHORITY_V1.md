# Agent Task Requestable Authority V1

SW0 A2.9 gates a **finite issued-right projection and named lifetime points**
through actual RT/LT adapters and contained LS commands. Run
`just foundry-agent-task-requestable-authority` on Linux with the existing pinned
Docker image. This is scripted host development evidence; full A2, continuous
`E(t)`, whole `E_max`, narrower-authority, model and target/physical claims remain
unqualified.

## Issuance and effects

The independent catalog maps the five declared rights to nine existing A2.6
canonical tuples. Its own digest and A2.6 universe digest bind the projection.
For each arm, the gate requests all 31 nonempty rights subsets under two fresh
fixture policies: all five rights (mask 31), and read/observe (mask 17). Both use
an unchanged 60000 ms lifetime ceiling. The second fixture is created before any
backend launches, with its policy bytes separately hashed.

The full policy issues all 31 subsets, echoing exactly the requested rights. The
attenuated policy issues only read, observe and read+observe; the other 28 grants
are denied with null results. READ and OBSERVE effects confirm the narrowed
fixture's usable grants and reported policy pin. A 60001 ms request is denied in
both policies. Enumeration uses canonical rights order; list permutations,
malformed requests and arbitrary other policies are outside this finite matrix.

Full-policy probes check three resource reads, staging, unvalidated commit
preconditions, missing receipts, task state and v2 subscribe/poll/cancel for each
subset. Missing rights must deny and redact results. Commit authority remains
subject to validation, revision/content and receipt existence; a granted COMMIT
bit alone does not establish a successful publication. VALIDATE is deliberately
unexercised in the grant enumeration, rather than being labeled absent.

A separate fresh case demonstrates actual effects with read-only, stage-only,
validate-only, commit-only and observe-only grants. The evaluator prepares shared
input/candidate prerequisites using a separate full grant, then the appropriate
single-right holder reads, stages, executes the pinned validator, publishes and
looks up the original receipt, or observes state. Those setup rights are not
ascribed to the single-right holders. Opaque locator aliases are not compared;
bytes/content hashes and semantic effects are checked instead.

Grant issuance is recorded separately from data effects. The issuance reducer
cannot turn a granted but unexercised right into a successful data operation, nor
turn absence of an exercise into denial. Auxiliary probe effects and their exact
requests/replies stay private. The gate independently checks read/staged hashes,
state pins and accepted CAS bytes: matrix/lifetime runs retain revision zero with
no commit receipts; the single-right witness has exactly one publication/receipt.

## Lifetime witnesses

Each arm issues a short 500 ms grant and subscription, reads before expiration,
and waits until a separate live observer's backend clock has reached the recorded
expiry. Every one of the nine task operations is then blocked with null results:
read, stage, validate, commit, receipt, state, subscribe, poll and cancel. The gate
repeats those checks after a new grant is issued, so renewal cannot reactivate the
old grant or subscription. Generation revocation blocks all nine operations on
the renewed grant too. Policy authority can issue a fresh grant in the new
generation; the old grant and subscription remain unusable.

RT uses `denied` for these terminal cases; LT/LS may report `expired` until old
grants are pruned. Both are retained exactly and only normalized to terminal
rejection for this named lifecycle conclusion. Previously returned observations
remain in consumer memory. LS additionally performs actual mounted-input reads
and passes its already-open input descriptor to a child after expiry and after
revocation. Both return the original bytes: broker grant transitions do not
revoke those filesystem/descriptor lifetimes. The LS stage helper also writes
its local candidate input file before the broker call; the gate verifies those
files even when staging is denied. Such writes are separate from broker staging
authority and remain in its broader filesystem inventory.

These are measured points, not a continuously observed authority envelope or an
exact scheduling proof at the expiry instant. Raw backend times and consumer
offsets remain in private artifacts. Disconnect/restart and forced cleanup retain
their separate A2.5/A2.8 gates.

## Boundary and artifacts

The declared-interface issued-right projections are equal across these three
arms and two policies. This relation says nothing about whole authority inclusion.
LS file/process/raw-broker access, trusted unisolated typed host clients, and
transitive broker/service/worker/Docker authority remain separate; their full
unexercised authority and continuous lifetimes stay unknown. No handle counts,
weights, model costs, broad isolation or hidden-affordance claim is inferred.

There are 11 fresh cases per arm: eight full-policy mask batches, one attenuated
batch, one single-right witness and one lifetime case. RT/LT use bounded external
sessions; LS runs a bounded contained scripted consumer using conventional v1
`taskctl` verbs, with v2 subscription requests explicitly using the existing raw
broker surface. Instrumentation is not a new model-visible tool. No model trial
or hidden-bank selection runs.

`out/agent-task-requestable-authority/` retains source/lock/binary/fixture hashes,
the attenuated policy, per-case requests/replies, runtime/containment evidence,
private journals and a report with false full-A2/continuous/model/target claims.
Reducer tests reject missing/duplicate/malformed subsets, unexpected issuance,
extra returned rights and failed redaction. Next bound the remaining host-client,
deputy and unexercised/continuous authority, then freeze actual bank/study and
provider/token controls before comparative collection. Hardware remains deferred.
