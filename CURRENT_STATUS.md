# Current Status

**Last Updated:** 2026-10-03
**Status:** Active and authoritative for landed state
**Current Slice:** S12.4 HIL appliance v0 physical loop
**Software Lane:** SW0 A2.9 finite requestable rights and lifetime points implemented; remaining host/deputy and continuous authority coverage is next

The [product vision](VISION.md) is an everyday, post-Unix OS for humans and AI
agents. This document records the pre-alpha foundations and their evidence;
desktop usability, broad hardware adaptability, and production readiness remain
goals. SW0 and the physical lane are complementary steps toward that product.

## Active Execution Track

S12.4's physical HIL appliance loop awaits test-hardware setup: serial observation
first, then Intel AMT 11 power/reset actuation. Once that loop is stable, the preferred S13 metal
HIL graduation path runs through the appliance on Tier-1 or lab hardware.
Standalone golden-machine graduation remains a distinct `PASS/METAL` path only
when per-gate evidence stamps `claim_path: operator-golden-machine`. S14 USB
xHCI and HID stays deferred until the H0/H1 appliance loop is proven, SW0
A1/A2 evidence and the bounded Phase B report have a recorded proceed/defer
decision, and its own design/IDL/Oracle/gate prerequisites land.

H0–H3 name the physical queue. SW0 is an independent software queue and can
continue without waiting for NVMe graduation. Its A0 schema/reference model and
deterministic gate plus A1.0's native control layouts/preflight are implemented;
the A1.1 useful host task and A2.1 Linux scoped-shell foundation are implemented.
A2.2 supplies shared JSON/RT transport; A2.3 supplies independent Linux typed
transactions and named RT/LT point-case checks. A2.4 supplies contained LS
commands and original-receipt recovery. A2.5 supplies the shared version 2 typed
subscription lifecycle. A2.6 adds a finite canonical inventory and shared all-arm
negative cases, preserving unknown authority. A2.7 adds a disjoint bank contract
and bounded scripted sessions. A2.8 adds named forced lifecycle cleanup and explicit
interrupted-commit recovery. A2.9 adds finite issued-right projections and named
expiry/renewal/revocation witnesses. Full A2 conformance and model
comparison remain pending. No live capture or actuation is scheduled while hardware
setup is pending. These labels do not allocate new slice
numbers or change governance authority.

The next action in each lane is maintained in [NEXT_TASKS.md](NEXT_TASKS.md).
Medium-range sequencing and deferred decisions live in [ROADMAP.md](ROADMAP.md).

## Evidence Boundary

| Area | Current evidence | What remains |
|------|------------------|--------------|
| S11 Driver Factory | Oracle capture/replay and embedded-vector harness transfers; `just s11` | Device-backed native virtio-net I/O, then broader device coverage |
| S12 golden machine | QEMU probes and HIL gate scaffolds landed | Appliance-mediated live capture and physical graduation |
| S13 storage | QEMU Oracle capture/replay and harness transfers against embedded vectors landed | Device-backed native block I/O plus live NVMe boot and two-boot atomic rollback evidence |
| S12.4 appliance | Manifest, evidence schema, gate, serial-observer scaffold, and physical wiring landed | First live serial capture, then provisioned and validated AMT control |
| Agent Task Proof (SW0) | A0/A1.0, A1.1/A2.1 foundations, A2.2 JSON/RT and A2.3 LT, A2.4 LS transactions, A2.5 typed subscriptions, A2.6 finite authority inventory, A2.7 scripted evaluator controls, A2.8 named reconciliation and A2.9 finite requestable/lifetime points | Remaining host/deputy/unexercised and continuous authority coverage, real hidden-bank/study/provider controls, bounded Phase B comparison, production/target integration |
| G0 RamenOrg | Governance schemas, packets, validators, trials, and gate landed | Research packets and stronger identity-level role separation |

`PASS/QEMU` is not metal evidence. `PASS/HIL-LOG`, `PASS/HIL-LIVE`,
`PASS/HIL-APPLIANCE`, and `PASS/METAL` have distinct provenance requirements;
see [EVIDENCE_LEVELS.md](EVIDENCE_LEVELS.md).

## Landed Milestones

### Project review boundary fixes (2026-10-03)

- Store checks requested ID against authenticated metadata and blob bytes;
  native execution hashes the byte snapshot it actually consumes.
- Host ingestion transfers a caller-opened regular-file descriptor and treats
  source paths as projection labels. Legacy pathname-only requests fail closed.
  This host transport does not replace the native IDL shared-memory source contract.
- Owned CAS publication persists an owner/manifest intent before publishing,
  recovers partial publication at startup and retry, and denies unrelated orphans.
- Native Unix/chardev IPC observes the invocation's absolute deadline through
  connect and partial transfers; uncertain dispatch is never automatically replayed.
- LT repeated staging preserves candidate capability/validation and counts unique
  IDs, matching RT. Portable and executable conformance regressions cover the limit.
- Full preflight and CI consume one complete SW0 sequence and require the Linux
  proof substrate. Host/QEMU checks do not substitute for Linux containment evidence.
- S11.8/S13.6 explicitly validate embedded Oracle vectors, with isolated QEMU
  staging; device-backed native net/block evidence remains outstanding.
- S1 creates its own output directory independently of S0; consolidated-PR CI
  exposed the missing directory on a clean checkout after S0 fixture isolation.
- Local macOS validation: 797 host workspace tests passed (11 ignored), strict
  workspace Clippy and all six lint tranches passed, bare-metal targets built,
  and the S0–S8 umbrella, S7 security, S11/S12/S13, governance G0, RT proof, JSON
  adapter, and review-boundary gates passed. S11.8/S13.6 also passed concurrently.
  An isolated working-tree copy on the configured Linux host passed the complete
  shared A0–A2.9 SW0 suite, Store/native runner tests, and real descriptor/deadline
  regressions. macOS full preflight correctly reports
  `INCOMPLETE reason=Linux-required` before performing a partial run; Linux proof
  prerequisites pass. These component runs do not claim a full single-host
  preflight or physical graduation.

### Product vision alignment (2026-10-03)

- Added `VISION.md` and aligned project, architecture, roadmap, contributor,
  agent, governance, and research guidance around an everyday OS for humans and
  AI agents, with modular components and useful compatibility.
- Preserved hardware/software execution order and evidence boundaries. This is
  a documentation milestone; it adds no runtime, performance, isolation,
  comparative, desktop, or physical-readiness evidence.

### SW0 A2.9 finite requestable rights and lifetime points (2026-10-01)

- `just foundry-agent-task-requestable-authority` enumerates all 31 nonempty
  rights subsets in RT/LT/LS under full and read/observe-only fixture policies.
  The full policy issues 31 subsets; the attenuated policy issues exactly three
  and denies the other 28 with null results. Both reject an over-ceiling lifetime.
- Subset operation probes check missing rights and preconditions. Separate
  single-right grants demonstrate actual read, stage, validate, commit/receipt and
  observe effects. Independent bytes, pins, accepted CAS and receipt checks keep
  issuance distinct from task effects and opaque locator aliases.
- Nine operations are blocked after expiry, after renewal of another grant and
  after generation revocation. Fresh policy-backed generation grants work without
  reviving old capabilities/subscriptions. LS mounted reads and inherited child
  descriptors survive those grant transitions and remain separate observations.
- [Requestable authority scope](docs/AGENT_TASK_REQUESTABLE_AUTHORITY_V1.md)
  qualifies equality only for the declared-interface issued-right projection.
  Whole authority inclusion, continuous lifetimes, host/deputy/unexercised reach,
  real hidden-bank/provider/model and target/physical claims remain pending.

### SW0 A2.8 named lifecycle and interrupted commit recovery (2026-10-01)

- `just foundry-agent-task-reconciliation` checks six actual RT/LT/LS interrupted
  commit cases: before dispatch and after durable publication with an abandoned
  reply. Restart denies old capabilities; fresh grants perform explicit repeated
  receipt lookup without resending commit, preserving zero or one accepted effect.
- Private durable intent/acknowledged-ID/removal ledgers fence new invocations
  before evaluator termination. Verified removal of every acknowledged ID permits
  named cleanup certification; an unresolved intent cannot certify from an empty
  inventory. A controlled late actual Docker create requires explicit resolution,
  and stale start by the removed ID fails.
- Trusted explicit LT/LS cleanup-checkpoint reconciliation holds the writer lock
  and preserves all non-cleanup journal fields. The real sleeping LS commit case
  exercises a pending checkpoint; native abrupt-process-crash recovery also runs.
- [Reconciliation scope](docs/AGENT_TASK_RECONCILIATION_V1.md) excludes general
  daemon/host crash and escape guarantees. Full authority, real hidden-bank,
  provider/token, model and target/physical claims remain pending.

### SW0 A2.7 synthetic bank and scripted evaluator controls (2026-09-30)

- `just foundry-agent-task-evaluator-controls` checks a 45-instance disjoint bank
  contract with five error classes and three instruction conditions per partition.
  Private oracles stay outside input roots; real pilot/final selections require
  bank/study-bound releases. CI creates synthetic instances and executes development.
- External sessions bound launch/idle/request time, frames, request count,
  diagnostics and visible bytes, including descriptions/bootstrap and final text.
  Fresh arm roots/transcripts and six arm orders cover 45 scripted attempts;
  successful repairs receive independent accepted revision/byte grading, and
  validator timeouts retain their unchanged publication pointer and attempt row.
- Failure and startup cases retain evidence. A real LS timeout removes observed
  labeled containers but remains quarantined: interrupted create RPCs do not
  certify cleanup. A2.8 subsequently distinguishes persisted acknowledgements from
  unresolved intents. Observed host-process groups are stopped and adapters reaped.
- [Evaluator scope](docs/AGENT_TASK_EVALUATOR_CONTROLS_V1.md) preserves unknown
  authority and incomplete real hidden-bank, provider/token and host-isolation
  controls. Full A2, model and physical trials remain pending.

### SW0 A2.6 finite authority inventory and all-arm negative cases (2026-09-30)

- `just foundry-agent-task-authority` freezes canonical resource/operation/scope/
  lifetime/delegation tuples and runs 33 common cases through actual RT/LT tools
  and contained LS commands. Fifteen common forbidden probes per arm plus six
  additional LS OS probes have zero successes; private journal pointers and sealed
  output bytes independently verify successful publication.
- Available observations, scripted-task effects, evaluator-probe effects, phase/
  clock samples, configuration provenance and unknowns remain separate. Measured
  LS file/process/descriptor/raw-session access does not become a whole-system
  comparison: full E_max/E(t), transitive authority and narrower claims remain unproved.
- The gate exposed and fixed LT's post-commit logical config read: it now returns
  current accepted CAS bytes, matching RT after renewal/restart. LS's original
  fixture mount remains a distinct observation.
- [Authority scope](docs/AGENT_TASK_AUTHORITY_V1.md) records virtual versus real
  workspace-B probes, typed host-client isolation limits and unknown inclusion
  results. Hidden-bank/evaluator controls and remaining authority coverage are next;
  model trials and physical hardware remain deferred.

### SW0 A2.5 shared typed subscription lifecycle (2026-09-30)

- `just foundry-agent-task-subscriptions` compares 50 version 2 operations per
  adapter through actual RT IPC and independent Linux LT enforcement. Version 1's
  eight-operation description artifact remains byte-identical.
- Generated typed pull subscribe/poll/cancel messages preserve existing push
  behavior. Each connection/session retains at most 16 subscriptions and two
  pending event types each; polls recheck original observation authority and
  return fresh scoped state, with no unsolicited model messages.
- Wire/service/adapter assertions cover connection and grant substitution,
  coalescing/filtering, capacity reuse, cancellation, queued-event revocation,
  expiry redaction, disconnect/restart removal and mapping cleanup.
- [Subscription scope](docs/AGENT_TASK_SUBSCRIPTIONS_V2.md) records at-most-once
  notification delivery, explicit resynchronization, expiry status normalization
  and LS's raw broker/session differences. Full authority conformance, hidden-bank
  controls and model evaluation remain pending; physical hardware stays deferred.

### SW0 A2.4 Linux scoped-shell durable transactions (2026-09-30)

- `just foundry-agent-task-ls-transactions` drives a real contained shell through
  eight conventional commands sharing Linux transaction enforcement with LT.
  Candidate paths remain inside the container; the host broker sees bounded bytes
  and fixed caller context, not a model-selected host path.
- A standalone opt-in launcher keeps one session across commands. It preserves
  bounded stdout/stderr and nonzero exit feedback, rejects budget overrides,
  and never chooses a repair or automatically retries a transaction.
- Independent journal grading, actual abandoned-reply receipt lookup, restart,
  rights/pin/revocation denials, wrong-peer UID, malformed/stalled packet bounds,
  descriptor inventory, protected mounts and uncertain-shell cleanup are gated.
  Pending/uncertain cleanup checkpoints block restart until reconciliation.
- [LS scope](docs/AGENT_TASK_LS_TRANSACTIONS_V1.md) records broader file/helper/
  process/delegation authority, host deputy privileges, shell checkpoint overhead
  and in-memory transport audit limits. Subscriptions, complete authority/hidden-bank
  conformance and model evaluation remain pending; physical hardware stays deferred.

### SW0 A2.3 independent Linux typed transactions (2026-09-30)

- `just foundry-agent-task-lt` runs the shared JSON task against a separate Linux
  broker with no RT/A0 enforcement dependency. Default-off host tooling owns
  policy grants, sealed CAS bytes, validation, revision/hash publication and receipts.
- The pinned worker runs in inspected Docker containment against a read-only
  candidate/schema/validator subset. IO uncertainty poisons the session; journal
  recovery verifies receipt history, pins and original validation evidence.
- Direct broker tests bypass Rust syntax filtering for real authority denials,
  expiry/revocation, writer exclusion, tampered bytes, stale revisions, later-revision
  retry and an injected uncertain publication. An external JSON/schema consumer
  checks useful repair, commit and restart. Named RT/LT point traces share descriptions
  and compare results with declared opaque-handle/clock normalization.
- [LT scope](docs/AGENT_TASK_LT_BACKEND_V1.md): development fixtures and scripted
  consumers; no host client isolation, full authority equivalence or model result.
  LS durable commands, subscriptions, complete audit/conformance and hidden-bank
  evaluation remain pending. Physical hardware remains deferred.

### SW0 A2.2 shared JSON contract and RT adapter (2026-09-30)

- `just foundry-agent-task-adapter` runs an external scripted consumer over bounded
  JSON lines through actual generated native IPC. It repairs, validates and commits
  the fixture; checks receipts/restart retries, revocation and redacted denials.
- One default backend-free library owns eight operations, descriptions, strict
  encodings, request/response schemas and serializer for both typed arms. IDs use
  lossless strings; bytes are bounded canonical base64. The opt-in RT bridge does
  no solving or mutation retry and releases consumed/source mappings.
- Independent schema/executable tests reject caller identity/path fields,
  malformed/duplicate data, wrong rights/kinds/pins and oversized frames. Service
  receipts replay through A0. CLI backend diagnostics stay outside model transport.
- [Adapter scope](docs/AGENT_TASK_ADAPTER_V1.md): a model-facing transport exercised
  by a script, with no model trial or complete equivalence. A2.3 adds LT. Native subscriptions
  are not yet exposed in JSON. LS transactions, full conformance, hidden-bank
  partitioning, production/target integration and physical HIL remain pending.

### SW0 A2.1 Linux scoped-shell foundation (2026-09-30)

- `just foundry-agent-task-linux-control` runs a useful scripted repair and the
  same pinned WASM worker/fixture as RT inside inspected Docker containment.
- Real probes cover private inputs, host process/socket/network reach, read-only
  writes, unsafe staging, immutable candidates, invalid/forged results, output and
  wall bounds, PID limits, background cleanup and retained Linux descriptors.
- Image/worker/source pins, actual namespaces/configs and a bounded authority
  inventory are recorded. Open descriptors surviving chmod and broader shell
  helpers are disclosed rather than treated as equivalent to RT grant revocation.
- [Linux control scope](docs/AGENT_TASK_LINUX_CONTROL_V1.md): one development
  fixture and evaluator acceptance, with no LS durable transaction, LT backend,
  model-facing serializer, hidden bank or full A2 conformance yet. CI requires
  the real Linux gate; unavailable containment cannot produce PASS.

### SW0 A1.1 scripted host service proof (2026-09-30)

- `just foundry-agent-task-proof-rt` repairs one configuration across generated
  native control messages and actual host stream IPC. A separate Native Runner
  worker verifies CAS pins and executes a WASM validator with no host imports.
- Current grants, immutable staging, revocation, revision/hash conflicts, ABA,
  durable retries and private observation/mapping boundaries have backend tests.
  One synchronized journal publishes the accepted reference and receipt together;
  process-crash tests cover publication failure and lost replies after durability.
- The worker watchdog includes CAS reads, IPC and compilation; guest/start loops,
  a real stalled backend, bounded diagnostics and two-worker admission are tested.
  Linux adds address-space limits and owned-descendant reaping.
- [Service proof scope](docs/AGENT_TASK_SERVICE_PROOF_V1.md): opt-in development
  features, launcher-bound domains, host mapping provider and a small equality
  schema. No production listener/broker registration, client process sandbox,
  target task authority, model advantage or physical result is established.
- A2 controls, model-facing adapters and canonical authority conformance are next.

### SW0 A1.0 native control contract (2026-09-30)

- Protocol 14 defines nine task request/reply pairs and one event through IDL
  and generated bindings. Every control layout fits the 64-byte payload.
- Allocation-free request preflight rejects unknown operations, reply injection,
  malformed sizes/identifiers, excessive grants/candidates and noncanonical
  shared-memory handles. `just foundry-agent-task-protocol-a1-0` runs in extended CI.
- [Agent Task Protocol V1](docs/AGENT_TASK_PROTOCOL_V1.md) inventories real call
  paths and specifies A1.1 service assertions. No handler or broker registration
  is enabled. Bulk-state schema, response checks, service-owned grants, durable
  transactions and worker containment remain required before the useful task.

### SW0 A0 contract fixtures and revised execution plan (2026-09-30)

- `artifact_store_schema::agent_task` defines pure stage → validate → commit
  semantics, exact identity/authority bindings, monotonic output revisions,
  revocation/renewal, bounded retained requests and idempotent successful retries.
- `just foundry-agent-task-contract-a0` runs deterministic positive/negative
  fixtures in the extended CI suite. Synthetic IDs and trusted executor inputs
  establish a contract model only; no service/kernel authority or validator
  authenticity is supplied by these serializable types.
- A1's service deliverable is one RT task with durable receipt
  publication/recovery, bounded worker/IPC execution, forced denials, audit and
  replay. A2 adds Linux controls before comparative runs. No useful task or model
  advantage has been demonstrated by A0.
- H3 now explicitly requires the implemented reboot/rollback protocol rather
  than a successful metadata scaffold. Physical work awaits hardware setup.

### Project review boundary fixes (2026-09-29)

- AArch64 leaf descriptors use the level-three page type and preserve PXN/UXN
  through flag and address updates. Pure descriptor assertions also run on x86
  hosts; this establishes encoding correctness, not physical fault recovery.
- Projection commits resolve sources through domain-aware visibility, reject
  conflicting or invalid ownership before CAS publication, and durably register
  fresh ownership before exposing the new projection. The live registry and
  restarted Store queries both retain owner access and deny unrelated domains.
- CoW, ordinary ingestion, and internal projection snapshots share immutable CAS
  publication: existing blobs/manifests are validated and reused without changing
  publisher metadata or signatures. Aggregate snapshots remain private internal
  artifacts. Failed CoW/ingestion persistence does not publish its candidate in
  memory. The final working-copy rename synchronizes its parent directory.
- Native WASM execution enforces the launch plan's nonzero `timeout_ms` through
  Wasmtime epoch interruption, including module start sections. Concurrent runs
  have independent wall-clock deadlines, and completion cancels the timer.
  This bounds guest execution; compilation and blocking host calls are outside
  epoch preemption. The default guest budget is 30 seconds.
- `just foundry-review-boundaries` and the projection gate cover these regressions,
  including process watchdogs that prevent a broken guest deadline from hanging
  the test harness. SW0 integration and physical graduation remain pending.

### Agent-facing proof plan and public docs (2026-09-16)

- Public entry points now lead with the agent interaction problem and distinguish
  host services, selected QEMU paths, simulation, and pending hardware evidence.
- The [Agent Task Proof plan](docs/plans/2026-09-16-agent-task-proof.md) specifies
  Linux scoped shell, Linux typed, and RamenOS typed arms, a shared typed protocol,
  canonical authority manifests, forced backend probes, and separate completion,
  authority, cost, and audit/replay claims. Pilot estimates feed a predeclared
  power calculation; final evaluation uses held-out fixtures. None of those runs
  has occurred yet.
- [Platform Overview](PLATFORM_OVERVIEW.md) now marks components as Landed,
  Partial, or Target architecture. H0–H3 and SW0 are independent execution lanes.
- This milestone is documentation and planning only. No executable task proof,
  measured agent advantage, new enforcement path, or physical result has landed.

### Memory, native runner, and Store review fixes (2026-09-16)

- Shared-memory allocation clears full backing frames, including partial-page
  tails. Every recipient reserves the region's common virtual address; conflicting
  mappings fail closed, and repeated references retain the PTE until final unmap.
- x86 page-table encoding preserves NX, and mapping enables EFER.NXE on supported
  CPUs. The shared-memory QEMU gate checks the actual leaf entry and NX enablement.
- Generated native WASM host bindings use the calling guest's exported memory and
  honor the SDK's declared output-slice capacity. Invalid or undersized buffers
  fail before predictable bridge operations; replies never truncate or overwrite
  adjacent guest state.
- Store signature verification uses the typed manifest's deterministic unsigned
  serialization. Ingestion hashes the same bytes it stages and atomically publishes.
- Per-domain trace buffers synchronize readers and writers, including ring wrap.
- `just foundry-review-boundaries` includes the new host regressions. Evidence is
  host tests and QEMU; no physical graduation or complete SMP/IRQ support is claimed.

### Review boundary fixes (2026-09-16)

- Store ownership persists across restart; unattributed artifacts deny access.
  Projection queries enforce the capability domain and durable ownership.
- Shared-memory requests accept only the allocator's supported 4 KiB page size.
- CI includes executable tooling and fails closed on classification errors.
- HIL opt-in captures serial after isolated fixture checks; firmware staging and
  run-bound provenance have host regressions, including synthetic serial devices.
- `just foundry-review-boundaries` covers these fixes. This is host/QEMU evidence;
  first live Pi↔M900 capture and physical graduation are still pending.

### S12 and S13

- S12.0 golden-machine contract updated to the acquired Lenovo ThinkCentre M900
  (machine type 10FH, Core i7-6700, 8 GiB) Tier-1 profile. Its installed 240 GB
  SanDisk SATA SSD is sufficient to begin S12; firmware/AMT preflight and the
  first live serial capture are next, and a compatible M.2 2280 PCIe NVMe drive
  is still required for S13 graduation.
- S12.1 UEFI GOP probe in QEMU OVMF.
- S12.2 physical HIL boot gate scaffold.
- S12.3 IOMMU inventory probe and gate.
- S12.4.0 HIL appliance manifest, evidence wrapper, and inventory gate.
- S12.4.1 serial-observer scaffold with run-id validation, empty-transcript
  rejection, and replay/live evidence separation.
- Physical serial-loop inventory records the Raspberry Pi 4 Model B (4 GiB), FTDI
  USB-to-RS-232 adapter, null-modem adapters, and ThinkCentre M900 target. The
  chain is physically installed and ready; the first live serial capture is the
  next evidence milestone.
- Per-gate HIL evidence now stamps `claim_path` and appliance metadata so
  standalone golden-machine runs cannot be mistaken for appliance-mediated
  graduation.
- S13.0 persistent-storage contract and `harness.block` IDL.
- S13.2-S13.5 virtio-blk Oracle capture and replay scoreboards.
- S13.6 typed `harness.block` transfers against embedded Oracle sector vectors in QEMU; device-backed native block I/O remains unproven.
- S13.7 NVMe boot and S13.8 atomic-update gate scaffolds at `PASS/QEMU`.

### S10 and S11

- Native WASM runner and production integration.
- Semantic State snapshots, subscribe reactor, and capability-filtered views.
- Projection storage through copy-on-write commits.
- Execution-fabric contract and canonical launch plans.
- Host-to-target semantic snapshot, broker bridge, and QEMU IPC bridge.
- S11 virtio-net Oracle capture/replay and typed `harness.net` transfers against embedded packet vectors in QEMU; device-backed native packet I/O remains unproven.

### Foundations

- S0 dual-architecture QEMU boot, IPC ping/pong, and tracing baseline.
- Typed IDL/codegen and wire-contract integrity gates.
- Shared-memory control/data planes and SPSC ring-buffer foundation.
- Store, compatibility, portal, domain-manager, and security-hardening slices.

Detailed chronology belongs in [CHANGELOG.md](CHANGELOG.md); slice-level outcomes
are summarized in [SLICES.md](SLICES.md).

## Parallel Project-Control Track

The G0 RamenOrg and Research Office scaffold is a parallel governance track. It
may produce docs, research artifacts, governance gates, and explicitly bounded
A2-local implementation trials. It does not supersede the OS execution track.

Current authority denies merge, release, self-approval, HIL actuation, and
public-support authority. The machine-readable task is
[docs/org/current_task.yaml](docs/org/current_task.yaml), and the governing plan
is [docs/plans/2026-06-23-research-backed-ramenorg.md](docs/plans/2026-06-23-research-backed-ramenorg.md).

## Known Gaps

- No model-facing three-arm Agent Task Proof or measured comparison with shell/tool agents;
  A1.1 proves a bounded host fixture; A2 controls and production/target integration remain.
- Native runner, Store, and Semantic State reactor remain host-side; default
  snapshot metadata includes placeholders, and execution-fabric routing is simulated.
- No `PASS/METAL` claim for S12 or S13 yet.
- S13 atomic rollback still needs the complete two-boot physical protocol.
- S14 interactivity has no approved implementation plan.
- Full execution-fabric transport and broader kernel broker migration are
  deferred design work.
- V-10 supervisor TCB breadth and V-13 portal TOCTOU remain architectural risk.
- RamenOrg authority above A2-local is not granted.

## Validation Commands

```bash
just s11
just s12
just s13
just hil-appliance
just foundry-org-governance-g0
```

Physical gates are opt-in and require the documented environment and provenance
markers. Do not infer hardware success from a default gate run.
