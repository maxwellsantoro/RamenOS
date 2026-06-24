# ROADMAP

**Last Updated:** 2026-06-23
**Status:** Active
Scope: Execution roadmap across slices (OS + Store + Foundry).

## 0) Now (current focus)
Active focus: **S12.4 HIL appliance v0 physical loop** (serial observer first, then power/reset actuator), then **S13 metal HIL graduation** (`RAMEN_HIL_APPLIANCE=1 RAMEN_HIL_GRADUATION=1 just s13-hil` on Tier-1 / lab hardware). S14 USB xHCI + HID **deferred** to design pass.

Authoritative pair: `CURRENT_STATUS.md` + `NEXT_TASKS.md`. Evidence terminology: `EVIDENCE_LEVELS.md`.

S13 QEMU Driver Factory loop is mature (S13.0–S13.6). S13.7/S13.8 are **metal gate scaffolds** (PASS/QEMU); metal graduation pending live HIL evidence.

Parallel planning/control track: **G0 Org Kernel + Research Office scaffold** (`docs/plans/2026-06-23-research-backed-ramenorg.md`, `just foundry-org-governance-g0`). This track is A0/A1 for board/planning/research and A2-local only for explicitly bounded implementation trials; it must not supersede the active HIL/storage execution track.

**S10.2 — Semantic State Substrate (v1.1 COMPLETE)**
- ✅ `semantic_state_v1` IDL + codegen (kernel_api, SDK, native_runner host)
- ✅ `PlatformSnapshotV0` schema with JSON/Markdown + `DomainInventoryEntry` builder
- ✅ WASM `semantic_state` service shmem delivery path
- ✅ Foundry gate `foundry_semantic_state_s10_2.sh` (includes native_runner E2E + subscribe delivery + cap filter)
- ✅ Live domain_manager snapshot emission (`--emit-semantic-snapshot`, `--ingest-semantic-snapshot`)
- ✅ S10.2.1 subscribe reactor (`SemanticReactor`, `state_changed_event` typed delivery)
- ✅ S10.2 v1.1 capability-filtered snapshots + domain_manager reactor publish
- 📋 Deferred: multi-source aggregator, WASM guest reactor loop

**S10.3 — Projection Storage (PHASES COMPLETE)**
- 📋 Design: `docs/plans/2026-02-20-s10-3-projection-storage.md`
- 📋 Backend decision: `docs/plans/2026-06-17-s10-3-1-projection-index-backend.md`
- ✅ Scaffold: schemas, `semantic_store_v1` IDL, `store_service` path/tag queries, gate
- ✅ S10.3.1 durable index; S10.3.2 ingest-time index updates
- ✅ S10.3.3 read-only VFS; S10.3.4 CoW projection writes

**S10.4 — Capability-Scheduled Execution Fabric (SCAFFOLD COMPLETE)**
- 📋 Design: `docs/plans/2026-06-17-s10-4-execution-fabric.md`
- ✅ v0: IDL, schemas, simulation, canonical launch-plan parsing, gate
- ✅ S10.4.1 fabric wiring (canonical emit-plan + supervisor always-local policy)
- 📋 Next: S13 metal HIL graduation; physical HIL via `just s12-hil` / `just s13-hil` when hardware available

**S10.5 — Host-to-Target Integration**
- ✅ S10.5.0: QEMU `semantic_snapshot` gate PASS (`snapshot_sha256_prefix=9c0de4419f03f426`)
- ✅ S10.5.1: broker/proxy bridge — `foundry_broker_kernel_bridge_s10_5_1.sh` PASS
- ✅ S10.5.2: QEMU IPC bridge — `foundry_qemu_ipc_bridge_s10_5_2.sh` PASS

**S11 — Driver Factory MVP**
- ✅ Device choice: `virtio-net-pci` in QEMU Linux Oracle capsule
- ✅ S11.1 capture scaffold: `pci_mmio_tracer`, `DriverProtocolTraceV0`, capsule relay trace-kind discovery
- ✅ S11.2 replay scoreboard: `foundry_s11_replay.sh` PASS
- ✅ S11.3 virtio-net Reference Vault + live Oracle capture: `foundry_s11_reference_vault_s11_3.sh` PASS with `REQUIRE_LIVE_ORACLE_TRACE=1`
- ✅ S11.3 trace fixture translation: `driver_foundry replay-trace` + `foundry_s11_replay.sh` PASS
- ✅ S11.4 init driver distillation: `driver_foundry::virtio_net_init` vault replay
- ✅ S11.5 packet I/O distillation: `NetPacketTraceV0`, `MockPacketHarness`, `driver_foundry::virtio_net_packet`
- ✅ S11.6 live packet Oracle capture: `capture_virtio_net_packet_oracle.sh` + live `oracle_packet_trace.json`
- ✅ S11.7 live hardware packet RX: kernel netdev (`virtio_net.ko`) + AF_PACKET ARP; `assert-hardware-packet-trace`
- ✅ S11.8 runtime `harness.net` packet I/O in QEMU: `foundry_s11_runtime_net_s11_8.sh` PASS; S11 Definition of Done complete (`just s11`)

**S12 — First Metal (Golden Machine)**
- ✅ S12.0 contract scaffold: `foundry_s12_golden_machine_s12_0.sh` PASS; `hardware/golden_machine_v0.toml`
- ✅ S12.1 UEFI GOP probe: `foundry_s12_gop_probe_s12_1.sh` PASS (QEMU OVMF)
- ✅ S12.2 physical HIL boot gate (`foundry_s12_hil_boot_s12_2.sh`, `RAMEN_HIL_GOLDEN_MACHINE=1`)
- ✅ S12.3 IOMMU inventory gate (`foundry_s12_iommu_inventory_s12_3.sh`, `RAMEN_HIL_GOLDEN_MACHINE=1`)

**S5.1 — Port It Now Wizard E2E (DEFERRED)**
- Schemas and `propose-policy` exist (S5); full orchestration deferred until S10.3 index + S10.2 subscribe land
- See `ROADMAP.md` §13

**Security (S9.0 - COMPLETE)**
- ✅ V-001 through V-015: All 11 security vulnerabilities addressed in Phase 1
- ✅ V-006 Phase 1: POSIX runner feature flag gating and warnings
- ✅ V-007 Phase 1: Services dependency cleanup (use schema types only)
- ✅ V-012 Phase 1: Per-domain trace ring buffers
- ✅ New Foundry gates: `foundry_posix_runner_s9_0_mitigation.sh`, `foundry_boundary_s9_0_cleanup.sh`, `foundry_trace_isolation_s9_0_per_domain.sh`
- 📋 Security posture improved from 11 unresolved vulnerabilities to 0 critical gaps (phased remediation complete)

**Security (S9.1 - COMPLETE)**
- ✅ V-007 Phase 2: Store service IPC (Unix domain sockets, bincode serialization)
- ✅ V-007 Phase 3: Store service integration for artifact validation
- ✅ V-007 Phase 4: Cryptographic signatures and SO_PEERCRED
- ✅ V-012 Phase 2: Domain-scoped writers with capability validation
- ✅ V-012 Phase 3: Trace capability-based access control
- 📋 Design doc: `docs/plans/v007_phase2_store_service_ipc_design.md`
- 📋 Remediation plan: `docs/plans/security_remediation_v006_v007_v012.md`

**Security (S9.2 - COMPLETE)**
- ✅ V-006 Phase 3: POSIX runner store service integration
- ✅ Store-integrated execution with IPC
- ✅ Ed25519 signature verification before execution
- ✅ Defense-in-depth maintained

**Security (S9.3 - COMPLETE)**
- ✅ V-007 Phase 5: Enhanced store security
- ✅ Capability-based access control (CBAC)
- ✅ Domain-scoped artifact visibility
- ✅ Production signature validation (RequireSignature)
- ✅ Enhanced audit logging with domain_id
- ✅ Key management support (TrustedKeys::load_from_file)
- 📋 Migration guide: `docs/plans/S9_3_MIGRATION_GUIDE.md`

**Now Shipping**
- ✅ V-012 Phase 5: User-space trace service client
- ✅ Trace service integration with domain manager
- ✅ Trace service client library

**Trace (V-012 - COMPLETE)**
- ✅ V-012 Phase 1: Per-domain trace ring buffers
- ✅ V-012 Phase 2: Domain-scoped trace writers
- ✅ V-012 Phase 3: Trace capability-based access control
- ✅ V-012 Phase 4: Kernel-side trace service implementation
  - Domain-scoped trace buffer management
  - Capability-based trace access control
  - Trace service IDL contract
  - Integration gate: `foundry_v012_phase4_trace_service.sh` (16/16 tests passing)
  - Unit tests: `trace_service` module (16/16 tests passing)

**OS (S8 - COMPLETE)**
- ✅ S8 Phase 1-3: Shared-memory control-plane IDL, kernel capability validation, frame allocator
- ✅ S8 Phase 4: Data-plane implementation with QEMU-based integration tests
  - Bitmap allocator, MMU programming interface, per-domain address space table
  - Integration gate: `foundry_shmem_dataplane_s8_phase4_integration.sh` (6/6 tests passing)
  - Core assertions: `foundry_shmem_dataplane_s8_phase4.sh` (40/40 tests)

**Store**
- Preserve Store consumption of OS capabilities and evidence discipline while S9 security sequencing executes.

**Foundry**
- Security-focused gates for S9.1: sandbox isolation, IPC transport, capability validation
- Measurable gate thresholds, deterministic replay checks, and auditable evidence pass/fail criteria.

**RamenOrg / Research Office (G0 scaffold)**
- Plan: `docs/plans/2026-06-23-research-backed-ramenorg.md`.
- Org Kernel docs: `docs/org/`.
- Research program docs: `docs/research/`.
- Gate: `tools/ci/foundry_org_governance_g0.sh`; `just foundry-org-governance-g0`.
- Current authority: A0/A1 for read-only board and issue/doc proposal; A2-local
  for explicitly bounded implementation trials. Merge, release, self-approval,
  HIL actuation, and public support authority require later explicit decisions.
- Product framing: RamenOS is research-backed, not a research OS. Research outputs must stay tied to product risk, claim boundary, evidence plan, and implementation landing path.

## 1) Completed: S9.0 — Security Remediation Phase 1
**Security** ✓
- All 11 vulnerabilities from comprehensive security audit addressed in Phase 1
- V-001..V-015: Complete (kernel vulnerabilities, POSIX runner, store IO, trace isolation)
- V-006 Phase 1: POSIX runner feature flag gating and warnings
- V-007 Phase 1: Services dependency cleanup, use schema types only
- V-012 Phase 1: Per-domain trace ring buffers
- Architectural boundary enforcement: "kernel ≠ services ≠ store"

**Foundry** ✓
- Added `foundry_posix_runner_s9_0_mitigation.sh` - POSIX runner warning and feature flag tests
- Added `foundry_boundary_s9_0_cleanup.sh` - Dependency cleanup and schema-only tests
- Added `foundry_trace_isolation_s9_0_per_domain.sh` - Per-domain buffer isolation tests
- Added `foundry_v007_phase2_store_service_ipc.sh` - Store service IPC design validation
- Added `foundry_v007_phase3_store_hardening.sh` - Store service hardening tests

## 2) Completed: S9.2 — Security Remediation Phase 3
**Security** ✓
- V-006 Phase 3: POSIX runner store service integration
- Store-integrated execution functions with IPC
- Ed25519 signature verification before execution
- Defense-in-depth: Sandbox + IPC + signatures

**Foundry** ✓
- Added `foundry_posix_runner_s9_2_store_integration.sh` (15/15 tests passing)

## 3) Completed: S9.3 — Security Remediation Phase 5
**Security** ✓
- V-007 Phase 5: Enhanced store security with CBAC
- Capability-based access control (StoreCapability)
- Domain-scoped artifact visibility (DomainArtifactRegistry)
- Production signature validation (RequireSignature)
- Enhanced audit logging with domain_id tracking
- Key management support (TrustedKeys::load_from_file)

**Foundry** ✓
- Added `foundry_v007_phase5_enhanced_store_security.sh` (39/39 tests passing)

## 4) Completed: S10.1 — Native Runner Production Integration
**Native Runner** ✓
- ✅ `native_wasm_v0` manifest schema with capability validation
- ✅ Capability broker with transactional grants in Domain Manager
- ✅ Runtime supervisor `native_wasm_runner` dispatch
- ✅ Real kernel IPC via native_runner kernel bridge
- ✅ Fail-closed execution (missing capability = load error)

**Foundry** ✓
- Added `foundry_native_runner_s10_1.sh` (19/19 assertions passing)
- Manifest schema validation tests (5)
- Broker integration tests (4)
- Runtime supervisor tests (2)
- End-to-end execution tests (5)
- Negative fail-closed tests (3)

## 5) Forward: S10.3 phases → S10.5 native integration

**S10.3 sub-phases** (gate-first; see design doc):
- **S10.3.0 (DONE):** Schemas, IDL, read-only `store_service` queries, gate
- **S10.3.1 (DONE):** Durable CAS-backed `ProjectionIndexV0` artifact + atomic working copy
- **S10.3.2 (DONE):** Index updates on artifact ingest
- **S10.3.3 (DONE):** Read-only VFS projection (virtio-9p; host materialize gate)
- **S10.3.4 (DONE):** CoW projection commit (`commit_projection_write`)

**S10.2 v1** (after scaffold): subscribe reactor, capability-filtered snapshots, live aggregator.

**S10.4.1 (DONE):** `store_cli` emits `ExecutionLaunchPlanV0`; supervisor `consult_always_local` fabric hook

**S10.5:** First host→QEMU native service path; broker/kernel bridge where required. Plan: `docs/plans/2026-06-17-s10-5-host-to-target-integration.md`.

## 6) Completed: S6 — Domain Manager + Extended Portals
**OS** ✓
- Domain Manager v1 landed (typed lifecycle API + restart-policy handling).
- Expanded portal suite landed (clipboard, notifications, screen capture) with typed protocol traces.

**Foundry** ✓
- Added `foundry_domain_manager_s6.sh` and `foundry_portal_suite_s6.sh`.
- Added S0→S6 umbrella gate (`foundry_all_s0_s1_s2_s3_s4_s5_s6.sh`).

## 7) Completed: S5 — Port It Now Wizard v1
**Store** ✓
- crash_context_v0 schema for structured crash bundles (Semantic State v2).
- graduation_v0 schema for tracking progression across target levels.
- minimal_policy_v0 schema with capability proposals and strictness scoring.
- Wizard flow: `propose-policy` generates minimal policy from observed capabilities.

**Foundry** ✓
- Validates crash contexts, graduation tracking, and policy proposals.

## 8) Completed: S3.x — Driver Capsule v0
**OS** ✓
- Host-only capsule relay using capsule.control + harness.echo IDLs.
- VM backend with virtio-serial transport to Linux microVM.
- Protocol traces emitted for control + echo harness boundaries.
- Wire format uses little-endian for cross-arch determinism.

**Foundry** ✓
- Replay gate validates relay protocol traces and evidence chain.
- Tests both host-only and VM modes (VM requires S2_COMPAT_KERNEL + QEMU).

## 9) Completed: S4 — Voting Queue + prerequisites graph
**Store** ✓
- Vote-to-port + priority scoring: `(vote_weight × leverage × reuse) / (effort × risk)`.
- Prerequisites graph (JSON + DOT) with high-leverage prereq detection.
- Claim/lock workflow (offline-first, content-addressed).

**Foundry** ✓
- Queue entries validated against repro trace + capability profile + target level.
- Negative assertions for invalid priority values.

## 10) Long-range (Post-S10.5)
- Multi-domain scheduling controls and per-domain resource accounting (builds on S10.4 contracts)
- Capability UX improvements for portal grant explainability
- Expanded harness support for native runner
- S5.1 Wizard E2E orchestration (deferred — see §13)

## 11) Invariants (always)
- Vertical slice discipline: each OS capability ships with a consumer + a gate.
- No POSIX-native APIs; compatibility is a runner, not a standard.
- Kernel validates capability handles on fast paths; brokers decide grants.
- Control plane is typed messages; data plane is zero-copy shared memory.
- Compatibility paths must accumulate graduation evidence (including driver capsules).

## 12) The Path to Bare Metal (S11-S15)
To escape VM purgatory, execute hardware-focused slices **after S10.5** proves one native service path on QEMU:

- **S11: The Driver Factory MVP.** Pick one simple device (e.g., `virtio-net` or a specific `NVMe` controller). Run it in the Linux Oracle capsule, capture the `protocol_trace` (MMIO/PCI/IRQ), and distill it into a native Rust component that passes the replay gate. *Device choice resolved: virtio-net-pci.* S11 COMPLETE (S11.1–S11.8): init + packet replay, live Oracle provenance, kernel netdev RX, runtime `harness.net` I/O in QEMU (`just s11`).
- **S12: First Metal (Golden Machine).** UEFI boot to a real framebuffer (GOP) with serial logging on the chosen Tier-1 physical machine. *S12.0 landed: Intel NUC 12/13 reference contract + smoke gate (`just s12`). S12.1+ GOP probe and HIL.*
- **S13: Persistent Storage.** virtio-blk Oracle in QEMU → `harness.block` runtime I/O → metal NVMe boot + Store A/B atomic update. *S13.0 landed: storage contract + `harness.block` IDL + smoke gate (`just s13`).*
- **S14: Interactivity.** Native USB xHCI + HID input components.
- **S15: The Sane Desktop.** Native Window Compositor consuming surfaces from the shared-memory data plane.

## 12b) Research-Backed OS / RamenOrg

This is a parallel control-plane roadmap, not a replacement for the OS slice
sequence.

- **G0: Org Kernel scaffold.** Define role capabilities, work orders, handoffs, board votes, heartbeats, claim safety, status-drift checks, and a governance Foundry gate.
- **G0.1: Board packet and packet validators.** Add JSON schemas, generated example packets, a read-only board packet renderer, and stdlib-only validators wired into the governance gate.
- **G0.2: Active task + cross-packet consistency.** Render packets from `docs/org/current_task.yaml`; enforce cross-packet agreement for repo SHA, work-order/proposal id, task, gates, authority level, fail-closed gate refs, and typed evidence buckets.
- **G0.3: CurrentTaskV0 + negative validator cases.** Validate the active-task source before rendering, bind board votes to repo SHA, enforce exactly-one scaffold refs, and prove known-bad packet/current-task cases are rejected.
- **G0.3.1: Governance label + claim-boundary hygiene.** Keep generated labels current, require explicit denial of merge/release/HIL-actuation/public-support authority, and reject PASS/METAL claims without HIL evidence refs.
- **G0.4: Read-only steward heartbeat and board brief.** Generate a human-readable board brief from validated packets after packet validation passes; no authority increase.
- **G0.5: Agent intake bundle and freshness binding.** Bind the brief, packets, current-task source, and validation report by SHA-256; reject stale validation state; no authority increase.
- **G0.6: Intake-only agent trial.** Give a fresh agent only the portable bundle, record the bounded-plan result and missing-context findings, and avoid ad hoc prompt repair; no authority increase.
- **G0.7: Bounded context grant.** Hash-bind selected input context, distinguish read/patch access, model authorized new output paths, and prove a fresh-agent `PASS/PATCH-PLAN`; no authority increase.
- **G0.8: Bounded implementation trial.** Use the bounded grant to land the S12.4.1 serial observer scaffold, record `PASS/PATCH`, and keep all authority denials intact.
- **G0.8.1: Implementation authority and serial observer claim hygiene.** Reclassify code-writing as A2-local, validate run ids, reject empty transcripts, and distinguish replay/live evidence modes.
- **RQ-0001: Offer-shaped service boundaries.** Mature provider-authored offers, `Lang`/`ObsContract`, re-timing airlocks, error membranes, and governed leakage into a RamenOS design pass before runtime implementation.
- **RQ-0002: AI-governed Org Kernel.** Mature RamenOrg from docs + drift checks into board packets, packet validators, read-only heartbeats, and staged authority levels.
- **Research Office lane.** Keep doctrine-level unknowns attached to prior art, claim boundaries, implementation paths, and gates.
- **Autonomy staging.** Do not advance beyond A2-local without explicit decisions, branch/release protections, credential scoping, evidence validation, and separated review.

## 13) Deferred decisions (require design pass before implementation)

Do not start these without a short plan doc + Foundry gate definition. Tracked in `NEXT_TASKS.md` §Deferred.

| Topic | Why deferred |
|-------|----------------|
| Vector / graph semantic search (S10.6+) | Not needed to prove path/tag projection or VFS mask |
| S5.1 Wizard E2E orchestration | Depends on semantic index + subscribe path |
| Real execution fabric transport | S10.4 v0 is simulation-only by design |
| `SimulatedKernelOps` → real kernel broker | S10.5.1 one-harness bridge complete; full migration deferred |
| S10.5.0 option choice | **Resolved:** Option A; init-bridge (see DECISIONS.md 2026-06-17) |
| Native runner on QEMU inventory | **Resolved:** host Wasmtime only; design doc §1 |
| S11 Oracle device selection | **Resolved:** virtio-net-pci in QEMU Oracle capsule (see `docs/plans/2026-02-20-s11-driver-factory-mvp.md` §0) |
| Tier-1 golden machine spec | **Resolved (S12.0):** `hardware/golden_machine_v0.toml`; GOP/HIL in S12.1+ |
| V-10 supervisor TCB migration | Kernel policy scope undefined |
| V-13 portal TOCTOU full fix | Broker architecture pass |
| Offer-shaped service boundary runtime | Requires RQ-0001 design pass, IDL plan, observable-contract claim levels, and Foundry gate |
| RamenOrg autonomy above A1 | Requires G0 validators, branch/release controls, explicit credentials, and board/evidence decisions |
