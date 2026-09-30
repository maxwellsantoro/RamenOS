# Current Status

**Last Updated:** 2026-09-30
**Status:** Active and authoritative for landed state
**Current Slice:** S12.4 HIL appliance v0 physical loop
**Software Lane:** SW0 A1.1 scripted host service proof implemented; A2 controls/conformance are next

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
the A1.1 useful host task is implemented. Linux controls and the model comparison
remain pending. No live capture or actuation is scheduled while hardware setup is pending.
These labels
do not allocate new slice numbers or change governance authority.

The next action in each lane is maintained in [NEXT_TASKS.md](NEXT_TASKS.md).
Medium-range sequencing and deferred decisions live in [ROADMAP.md](ROADMAP.md).

## Evidence Boundary

| Area | Current evidence | What remains |
|------|------------------|--------------|
| S11 Driver Factory | Complete; `just s11` | Broader device coverage is future work |
| S12 golden machine | QEMU probes and HIL gate scaffolds landed | Appliance-mediated live capture and physical graduation |
| S13 storage | QEMU Oracle, replay, and runtime block I/O landed | Live NVMe boot plus two-boot atomic rollback evidence |
| S12.4 appliance | Manifest, evidence schema, gate, serial-observer scaffold, and physical wiring landed | First live serial capture, then provisioned and validated AMT control |
| Agent Task Proof (SW0) | A0/A1.0 contracts and opt-in A1.1 scripted host service proof | A2 controls/model-facing adapters and authority conformance, bounded Phase B comparison, production/target integration |
| G0 RamenOrg | Governance schemas, packets, validators, trials, and gate landed | Research packets and stronger identity-level role separation |

`PASS/QEMU` is not metal evidence. `PASS/HIL-LOG`, `PASS/HIL-LIVE`,
`PASS/HIL-APPLIANCE`, and `PASS/METAL` have distinct provenance requirements;
see [EVIDENCE_LEVELS.md](EVIDENCE_LEVELS.md).

## Landed Milestones

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
- S13.6 runtime `harness.block` sector I/O in QEMU.
- S13.7 NVMe boot and S13.8 atomic-update gate scaffolds at `PASS/QEMU`.

### S10 and S11

- Native WASM runner and production integration.
- Semantic State snapshots, subscribe reactor, and capability-filtered views.
- Projection storage through copy-on-write commits.
- Execution-fabric contract and canonical launch plans.
- Host-to-target semantic snapshot, broker bridge, and QEMU IPC bridge.
- S11 virtio-net Driver Factory loop through runtime packet I/O in QEMU.

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
