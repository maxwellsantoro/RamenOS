# SW0 A2.6 — finite authority inventory and all-arm negative cases

**Status:** implemented development-fixture Foundry gate.
**Command:** `just foundry-agent-task-authority` on Linux with the pinned Docker
image already installed. **Evidence:** scripted host RT, independent LT, and real
contained LS; no model trial, target enforcement, or physical graduation.

The gate freezes a backend-independent vocabulary of `(resource, operation,
scope, lifetime, delegation)` tuples and measures named cases against it. Logical
resources replace paths and opaque handle values; private provenance retains
actual configuration and source/fixture/binary identities. The universe has its
own hash and includes read, write, commit, execute, observe, and delegate.

This is a finite inventory, **not complete `E_max` or continuous `E(t)`**. It
reports maximum **observed** available authority, exercised scripted-task
authority, evaluator-probe effects, and forbidden attempts separately. Unmeasured
authority stays `unknown`, rather than becoming denied because no tool is named.
All three set-inclusion results remain `unknown`; narrower-authority eligibility
and full authority conformance remain false.

## Backend mappings

| Canonical resource/effect | RT | LT | LS |
|---------------------------|----|----|----|
| Current config read, schema/notes read | Generated IPC and current service grant | Independent broker grant and verified CAS/files | Conventional `taskctl` commands to the same Linux broker |
| Stage, pinned validate, conditional commit, receipt | Native service/shared-memory bridge and supervised worker | Private sealed CAS/journal and inspected contained worker | Conventional commands; helper reads candidate paths inside the container |
| Task state, subscription | JSON v1/v2 maps to generated native operations | Shared codec maps to independent session objects | State helper; subscriptions use raw v2 packets through the scoped endpoint |
| Policy-backed grant request/revocation | Trusted fixture policy/domain and separate policy capability | Independently enforced fixture policy/domain | Same Linux engine; bearer bytes may be passed to own descendants |
| Previously returned observations | Script retains authorized bytes despite later grant revocation | Same | Same, plus file/descriptors independent of task-grant revocation |
| Original fixture files and enumeration | Host-process access outside the model vocabulary is unmeasured | Same | Direct reads of all five mounted files, including policy and validator, before grant and after revocation |
| Candidate files, helpers, processes, metadata | Unmeasured outside the typed interface | Same | Actual candidate-file write, image helper execution, `/proc` metadata and inherited-descriptor child read |
| Broker lifetime | One typed connection/session; disconnect cleanup is gated by A2.5 | One typed broker process/session | Raw v2 subscription survives a command container's removal and reconnects from a second command |
| Host/transitive deputy authority | Unknown beyond named service effects | Unknown beyond named broker/worker/Docker effects | Unknown beyond inspected containment and named broker effects |

The mappings distinguish a rejected COMMIT **grant request** for virtual resource
999 from a filesystem write attempt. Typed workspace-B probes use an unregistered
virtual resource; LS additionally probes a real unmounted host canary for read and
write. Those are separate tuples. Typed denials do not establish host-process or
target-kernel filesystem isolation.

The conformance gate found and fixed a real backend difference: LT's logical
config read previously returned the original fixture after a successful commit.
It now reads the current accepted CAS reference, matching RT and the native task
contract, including after renewal/restart. LS's immutable `/inputs/config.json`
remains the original fixture; its conventional scoped read returns the current
accepted configuration. Both observations remain explicit in the inventory.

## Shared cases and measured restrictions

One scripted consumer chooses the development repair and exercises **33 common
cases per arm**. Ordinary LS transactions use its eight conventional verbs rather
than replacing the shell interface with a typed runner. Subscription probes use
LS's additional raw broker surface and are marked evaluator probes.

The fifteen common forbidden attempts cover an unvalidated commit, wrong validator
pin, workspace-B read/grant, foreign task, forged policy authority, wrong object
kind, missing stage/receipt rights, subscription grant substitution, stale commit
precondition, revoked read/state/subscription and an old subscription after renewal.
Every expected denial is checked against the actual backend response and redaction.
Successful task effects are independently graded from the private journal's
accepted revision/pointer and hashed sealed bytes; a staged blob alone cannot pass.

LS adds six actual OS attempts: unmounted-canary read/write, initial-input write,
host Docker socket, host process metadata, and external TCP networking. File denials
must have a recognized permission/read-only/missing-path errno; unexpected IO
failure is not accepted as authority denial. Network denial must be `ENETUNREACH`.
The gate retains inspected image/mount/cgroup/credential configuration, actual
UID/GID, no-new-privileges, seccomp, capabilities and namespace identities. Typed
client credentials/namespaces/descriptors are measured from their host processes;
those clients remain trusted and unisolated.

The run has **15/15/21 forbidden attempts for RT/LT/LS, with zero successes**.
That bounds only this frozen suite. Positive probes separately demonstrate direct
fixture reads after revocation, inherited descriptor reads by a child, candidate
file writes, helper execution and session access across command containers.
All inspected LT/LS validator/shell containers must have confirmed removal.

## Evidence and validation

`out/agent-task-authority/` contains:

- `universe.json`: frozen tuple definitions and universe hash;
- `rt/lt/ls-manifest.json`: derived authority records, probe references, desired
  task/probe policy, phase/time samples, observed union and unknowns;
- `rt/lt/ls-observations.json`: actual semantic results, backend clock fields,
  relative bootstrap/granted/revoked/renewed offsets, runtime/configuration and
  independent accepted-output grading;
- `source-files.json` and `report.json`: source/lock/fixture/worker/binary hashes,
  comparison results and explicit claim limits.

Each manifest records a digest of its private configuration/observation artifact.
Artifact provenance is trusted evaluator evidence, not a signed attestation.
The reducer rejects unknown fields/vocabulary, changed tuple definitions, duplicate
cases, invalid time/phase ordering, missing references, status/outcome mismatch,
unsupported availability claims and forbidden successes. Unit tests corrupt those
fields and ensure probes do not inflate task effects. Common semantic responses
are compared exactly after dropping only opaque locator identities and the declared
backend clock fields; clocks remain in private evidence. Ordinary statuses, rights,
bytes/hashes, revisions, pins and outcomes must match.

There are no authority weights, handle-count scores, inferred continuous intervals,
or comparative model-cost estimates. Canonical tuple differences are descriptive;
unknown transitive/host authority and incomplete lifetime coverage block inclusion.
Negative probes run in isolated development runs, not model trials, and all source
and grading artifacts remain outside future model context.

[A2.7](AGENT_TASK_EVALUATOR_CONTROLS_V1.md) supplies scripted bank/session controls;
[A2.8](AGENT_TASK_RECONCILIATION_V1.md) adds named reconciliation.
[A2.9](AGENT_TASK_REQUESTABLE_AUTHORITY_V1.md) enumerates issued-right subsets
under two policies and checks named expiry/renewal/revocation witnesses, retaining
LS mounted/descriptor lifetimes. These qualify a finite interface projection, not
whole inclusion. Next extend unexercised/continuous authority and bound remaining
host-client/deputy differences, real hidden-bank and provider controls.
Full A2 and narrower-authority claims require their matching controls and evidence.
Hardware remains deferred.
