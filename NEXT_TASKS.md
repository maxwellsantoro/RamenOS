# NEXT_TASKS

**Last Updated:** 2026-06-23
**Status:** Active

> **Authoritative pair:** `CURRENT_STATUS.md` (what landed) + this file (what's next).
> Completed slice checklists live in `SLICES.md`. Historical security waves are archived below.

## Active execution track

**Now:** Implement the Raspberry Pi-class HIL appliance v0 physical loop: serial observer first, then power/reset actuator. S13 metal HIL graduation should run through the appliance once that loop is stable.

| Priority | Task | Gate / doc |
|----------|------|------------|
| P0 | S12.4.1 HIL appliance serial observer | `tools/hil/appliance_capture_serial.sh` + `RAMEN_HIL_APPLIANCE=1 just hil-appliance` |
| P1 | S12.4.2 HIL appliance power/reset actuator | `tools/hil/appliance_press_power.sh`, `tools/hil/appliance_press_reset.sh`, controller evidence JSON |
| P2 | S13 metal HIL graduation through appliance-mediated live capture | `RAMEN_HIL_APPLIANCE=1 RAMEN_HIL_GRADUATION=1 just s13-hil` |
| P3 | Physical S12 HIL graduation through appliance-mediated live capture | `RAMEN_HIL_APPLIANCE=1 RAMEN_HIL_GOLDEN_MACHINE=1 just s12-hil` |
| P4 | S14 USB xHCI + HID design pass | `ROADMAP.md` §S14; do not implement until appliance loop is stable |

## Parallel project-control / research track

This track is first-class, but it does not supersede the OS execution track above.
It exists so RamenOrg and research-backed planning stop living only in chat.

| Priority | Task | Gate / doc |
|----------|------|------------|
| GP0 | G0.8.1 implementation authority and serial claim hygiene | `docs/plans/2026-06-23-g0-8-1-implementation-authority-serial-claim-hygiene.md`, `just foundry-org-governance-g0` |
| GP1 | RQ-0002 AI-governed Org Kernel research packet | `docs/research/questions/RQ-0002-ai-org-kernel.md`, `tools/org/render_board_packet.py` |
| GP2 | RQ-0001 offer-shaped service boundary research packet | `docs/research/questions/RQ-0001-offer-boundaries.md`, future offer-boundary design pass |
| GP3 | Identity-level role separation | Future agent identity and review-domain model; no authority increase |
| GP4 | Fresh implementation agent reproduction | Repeat G0.8.1 with an isolated agent bundle before widening authority; no authority increase |

**Authority guard:** G0 remains A0/A1 for board, planning, docs, and research.
G0.8.1 admits A2-local implementation trials only inside an active work order.
It must not grant merge, release, self-approval, HIL actuation, or public
support authority without a later explicit decision.

**Recently scaffolded (2026-06-23):**
- G0.8.1 implementation authority and serial observer claim hygiene: code-writing
  trials are now classified as A2-local, not A1. The serial observer validates
  `RAMEN_HIL_RUN_ID`, rejects empty transcripts, records `serial_input_kind`,
  emits `PASS/HIL-LOG` for development replay and `PASS/HIL-APPLIANCE` for live
  serial capture, and the HIL gate covers invalid-run-id and empty-log negatives.
- G0.8 bounded implementation trial: `tools/hil/appliance_capture_serial.sh`
  now implements the S12.4.1 serial observer scaffold. The HIL appliance gate
  checks the observer contract with a synthetic transcript and proves stale
  `RAMEN_HIL_SERIAL_LOG` replay is rejected in graduation mode. The trial is
  recorded as `PASS/PATCH`; no merge, release, HIL actuation, or public support
  authority was granted.
- G0.7 bounded context grant: `ContextGrantV0` now binds eight selected context
  files by path/hash/access and names `tools/hil/appliance_capture_serial.sh` as
  an authorized new output path. The fresh-agent trial passed as
  `PASS/PATCH-PLAN`: sufficient context, no expansion request, no hidden chat,
  no external reads, and no implementation.
- G0.6 intake-only agent trial: a fresh agent with no inherited thread context
  received only the brief, manifest, and four packets. It recovered the bounded
  plan, refs, gates, and authority without hidden chat context. Finding: those
  six artifacts are sufficient for planning but not a responsible patch because
  referenced context and scoped source contents are absent.
- G0.5 agent intake bundle and freshness binding: packet validation reports a
  SHA-256 ledger; the board brief verifies that ledger before rendering; and
  `out/org/intake_manifest.json` binds the brief, packets, current task, and
  validation report. The governance gate independently validates the manifest
  and proves stale validation reports are rejected.
- G0.4 read-only steward heartbeat / board brief: `tools/org/render_board_brief.py`
  writes `out/org/current_board_brief.md` only after packet validation passes;
  the governance gate checks Active Task, Authority Boundary, Required Gates,
  Context Refs, Evidence Refs, and Allowed Next-Agent Actions sections.
- G0.3.1 Governance label + claim-boundary hygiene: current-task labels,
  renderer claim text, and validator diagnostics now identify G0.3.1; claim
  boundaries must explicitly include no merge, no release, no HIL actuation, and
  no public support authority; negative cases cover missing denials and
  PASS/METAL without HIL evidence refs.
- G0.3 CurrentTaskV0 + negative fixtures: `schemas/org/current_task_v0.schema.json`, `BoardVoteV0.repo_sha`, exactly-one board packet refs, and `tools/org/test_validate_packets.py` proving known-bad packet/current-task cases are rejected.
- G0.2 Active Task and cross-packet consistency: `docs/org/current_task.yaml` now drives packet rendering; validator loads referenced packets and checks repo SHA, work-order/proposal id, task, gate set, authority level, fail-closed gate refs, and typed evidence buckets.
- G0.1 Board Packet and packet validators: `schemas/org/*.schema.json`, `tools/org/render_board_packet.py`, `tools/org/validate_packets.py`; governance gate now renders and validates example packets under `out/org/examples/`.
- G0 Org Kernel docs under `docs/org/`: constitution, role charter, authority levels, heartbeats, work orders, handoffs, votes, and claim safety.
- Research-backed OS program under `docs/research/`, including RQ-0001 offer boundaries and RQ-0002 AI Org Kernel.
- G0 plan: `docs/plans/2026-06-23-research-backed-ramenorg.md`.
- Governance drift checker: `tools/org/status_drift.py`; gate: `tools/ci/foundry_org_governance_g0.sh`.

**Recently completed / scaffolded (2026-06-22):**
- S12.4.0 HIL appliance scaffold gate: `tools/ci/foundry_hil_appliance_s12_4.sh`, `just hil-appliance`, and `foundry_ci_extended.sh` docs/manifest path.
- Added wrapper evidence schema: `docs/HIL_APPLIANCE_EVIDENCE_V0.md`.
- S12.4 / S13.9 HIL Appliance Controller plan upgraded from planned note to scaffold: `docs/plans/2026-06-22-hil-appliance-controller.md`.
- `hardware/hil_appliance_v0.toml` now records canonical naming, gate path, RS-232/TTL safety, relay defaults, and required wrapper evidence fields.
- Policy: Pi/controller is lab infrastructure, not target TCB; raw Pi GPIO UART is TTL-only; default PC serial path is target COM/DB9/header → USB RS-232 adapter → Pi USB.

**Recently completed (2026-06-21):**
- Evidence discipline: `EVIDENCE_LEVELS.md`, HIL provenance serial markers, `RAMEN_HIL_GRADUATION=1` mode, evidence JSON bundles under `out/evidence/`
- S13.8 atomic update/rollback gate scaffold: `ab_slot_probe.rs`, `OP_ATOMIC_UPDATE`, `foundry_s13_atomic_update_s13_8.sh` (QEMU negative smoke PASS); `just s13-hil` extended; S13 QEMU + HIL scaffolds complete
- S13.7 metal NVMe boot gate scaffold: `kernel_uefi/src/nvme_boot_probe.rs`, `OP_NVME_BOOT`, `foundry_s13_nvme_boot_s13_7.sh` (QEMU negative smoke PASS); `just s13-hil` opt-in; tranche5 harness lint hygiene
- S13.4–S13.5 block sector Oracle: `block_sector_trace_v0`, `MockBlockHarness`, `virtio_blk_sector`, live `oracle_block_trace.json`, `foundry_s13_block_sector_oracle_s13_4.sh`; `just s13` extended
- S13.6 runtime harness.block I/O: `kernel_api::block_oracle_vector`, `kernel/src/block_harness.rs`, `OP_BLOCK_IO`, `foundry_s13_runtime_block_s13_6.sh`; `just s13` extended
- S13.3 block replay scoreboard: `foundry_s13_replay.sh` PASS
- S13.2 virtio-blk Oracle capture: `capture_virtio_blk_oracle.sh`, `promote_virtio_blk_capture.sh`, `driver_foundry::virtio_blk_init`, live `oracle_init_trace.json` (15 events), `foundry_s13_virtio_blk_oracle_s13_2.sh`; `just s13` extended
- S13.0 persistent storage contract: `docs/plans/2026-06-21-s13-persistent-storage-design.md`, `hardware/storage_contract_v0.toml`, `idl/harness/block_v1.toml`, virtio-blk vault scaffold, `foundry_s13_persistent_storage_s13_0.sh`; `just s13`
- S12.3 IOMMU inventory: `kernel_uefi/src/iommu_probe.rs`, `OP_IOMMU_INVENTORY`, `foundry_s12_iommu_inventory_s12_3.sh`; `just s12-hil` extended
- S12.2 physical HIL boot gate: `OP_HIL_BOOT`, `hil_boot` init profile, `tools/hil/build_usb_boot_image.sh`, `foundry_s12_hil_boot_s12_2.sh`; `just s12-hil` opt-in alias
- S12.1 UEFI GOP probe: `kernel_uefi/src/gop_probe.rs`, `OP_GOP_PROBE`, `foundry_s12_gop_probe_s12_1.sh` PASS (QEMU OVMF 1280×800 BGR); `just s12` extended
- S12.0 golden machine contract: `docs/plans/2026-06-21-s12-golden-machine-design.md`, `hardware/golden_machine_v0.toml`, `foundry_s12_golden_machine_s12_0.sh`; wired into `foundry_ci_extended.sh`

**S11 complete (2026-06-21):** Definition of Done satisfied via `just s11`:
- `foundry_s11_replay.sh` — init + packet host replay
- `REQUIRE_LIVE_ORACLE_TRACE=1 foundry_s11_reference_vault_s11_3.sh` — live Oracle provenance + hardware RX
- `foundry_s11_runtime_net_s11_8.sh` — runtime `harness.net` packet I/O in QEMU

**Recently completed (2026-06-21):**
- S11.8 runtime harness.net packet I/O: `kernel_api::net_packet_oracle_vector`, `kernel/src/net_harness.rs`, `OP_NET_PACKET_IO`, `foundry_s11_runtime_net_s11_8.sh`; `just s11` extended
- `just s11` fast-path alias: runs replay + live vault + runtime harness I/O
- S11.7 live hardware packet RX: kernel netdev capture via `virtio_net.ko` + AF_PACKET ARP; slirp-derived receive fallback removed; `assert-hardware-packet-trace` in `driver_foundry`; `REQUIRE_LIVE_ORACLE_TRACE=1` runs hardware RX assertion; live `oracle_packet_trace.json` (`sha256:482af300…`)
- S11.6 live packet Oracle capture: `capture_virtio_net_packet_oracle.sh`, `promote_virtio_net_packet_capture.sh`, live `oracle_packet_trace.json`; `REQUIRE_LIVE_ORACLE_TRACE=1` asserts packet provenance
- S11.5 virtio-net packet I/O: `NetPacketTraceV0`, `MockPacketHarness`, `driver_foundry::virtio_net_packet`, and `oracle_packet_trace.json`; `foundry_s11_replay.sh` extended
- S11.4 virtio-net init driver: extended live capture (20 events: features, MAC, RX/TX queues, DRIVER_OK) and `driver_foundry::virtio_net_init` vault replay; `foundry_s11_replay.sh` extended
- S11.3 live Oracle capture: `tools/trace/capture_virtio_net_oracle.sh` boots QEMU `virtio-net-pci`, captures JSONL, promotes live `oracle_init_trace.json`; `REQUIRE_LIVE_ORACLE_TRACE=1` enforced in `foundry_ci_extended.sh`
- Kernel lint debt: `TraceCapError` replaces `Result<(), ()>` in `trace_cap.rs`; `fetch_compat_kernel.sh` extracts `.deb` without `dpkg-deb`

**Recently completed (2026-06-18):**
- S11.3 live-capture promotion path: `tools/trace/promote_virtio_net_capture.sh` imports JSONL, stamps missing trace IDs from source SHA-256, replays, asserts live provenance plus contiguous event numbering, and the S11.3 gate dry-runs it
- S11.3 virtio-net datasheet/spec notes: pinned OASIS VIRTIO v1.3 anchors under `drivers/reference_vaults/virtio-net/datasheets/`; `foundry_s11_reference_vault_s11_3.sh` asserts datasheet inventory
- S11.3 live-capture import/provenance tooling: `driver_foundry import-jsonl`, `driver_foundry assert-live-trace`, and `REQUIRE_LIVE_ORACLE_TRACE=1 foundry_s11_reference_vault_s11_3.sh`
- S11.3 trace fixture translation: `driver_foundry` converts `DriverProtocolTraceV0` PCI/MMIO events into `PciReplayEvent` and replays the vault fixture through `MockPciDevice`; `foundry_s11_replay.sh` PASS
- S11.3 virtio-net Reference Vault scaffold: `idl/harness/net_v1.toml`, generated binding, schema-valid vault fixture; `foundry_s11_reference_vault_s11_3.sh` PASS
- S11.2 replay scoreboard: `kernel_api::mock::pci_device::{ReplayScoreboard, MockPciDevice}`; `foundry_s11_replay.sh` PASS

**Recently completed (2026-06-17):**
- S11.1 Oracle capture scaffold: `foundry_s11_driver_factory_s11_0.sh` PASS
- S10.5.2 QEMU IPC bridge: `foundry_qemu_ipc_bridge_s10_5_2.sh` PASS (`snapshot_sha256_prefix=9c0de4419f03f426`)
- S10.2 v1.1 capability-filtered snapshots + domain_manager reactor publish
- S10.5.0 QEMU validation: `foundry_host_target_s10_5.sh` PASS (`snapshot_sha256_prefix=9c0de4419f03f426`)
- S10.5.1 broker/kernel bridge: `foundry_broker_kernel_bridge_s10_5_1.sh` PASS
- S10.5.1 design/gate pass: broker/proxy bridge plan + Foundry gate
- S10.5 design/gate pass: inventory pinned, Option A chosen, QEMU gate defined
- S10.2.1 subscribe reactor: `SemanticReactor`, `state_changed_event` typed delivery, `subscribe_delivery` gate
- S10.4.1 fabric wiring: canonical `emit-plan`, supervisor `consult_always_local` fabric hook, extended Foundry gate
- S10.3.4 CoW projection writes: `commit_projection_write` ingests replacement bytes, repoints projection path, preserves prior blob
- S10.3.3 read-only VFS: virtio-9p decision, `projection_vfs` materializer, compat_runner `-virtfs` wiring
- S10.3.2 index-on-ingest: `IngestArtifact` updates path/tag projection entries and persists the durable index
- S10.3.1 durable projection index: `ProjectionIndexStore` writer, CAS snapshot, extended Foundry gate
- S10.2 scaffold: live snapshot emit/ingest, `store_cli ingest-platform-snapshot`, subscribe stub
- S10.3 scaffold: schemas, IDL, `store_service` projection queries
- S10.4 scaffold: IDL, schemas, simulation, canonical launch-plan parsing in supervisor

## Deferred — requires design pass

See `ROADMAP.md` §13. Do not start without a short design doc + Foundry gate definition.

1. **S10.3 vector / graph API** — semantic search, GraphQL/Cypher; post-VFS slice
2. **S5.1 Wizard E2E** — run → observe → propose → rebuild → gate → publish orchestration
3. **Real execution fabric** — remote nodes, transport, policy beyond simulation
4. **Full capability broker kernel bridge** — replace remaining `SimulatedKernelOps` paths with real fast-path IPC (S10.5.1 semantic harness profile ships via `RAMEN_SEMANTIC_HARNESS_BRIDGE=1`)
5. **QEMU compat 9p read gate for S10.3.3** — initrd/kernel with 9p mount + projected path read
6. **Compat scratch → commit IPC** — how supervisor/compat reaches `commit_projection_write` without mutable 9p
7. **S11 device selection** — **Resolved:** virtio-net-pci QEMU Oracle (`docs/plans/2026-02-20-s11-driver-factory-mvp.md` §0)
8. **Tier-1 golden machine** — **Resolved (S12.0):** Intel NUC 12/13 reference + `hardware/golden_machine_v0.toml`; GOP/HIL implementation in S12.1+
9. **Portal TOCTOU hardening** (V-13) — full broker redesign
10. **Supervisor TCB reduction** (V-10) — kernel-side policy migration scope
11. **TCG userspace virtqueue RX** — blocked in Oracle capsule; kernel netdev is authoritative for packet receive (see `DECISIONS.md` 2026-06-21)
12. **Offer-shaped service boundary implementation** — requires RQ-0001 design pass, claim levels, IDL plan, and Foundry gate before runtime code.
13. **RamenOrg autonomy above A1** — requires G0 artifacts, status drift gate, packet validators, branch/release controls, and explicit decisions before merge/release/hardware authority.

---

## Archive: security remediation (Wave A/B/C) — COMPLETE

All SC-01..SC-12 and V-001..V-015 Phase-1 mitigations complete. Gates: `foundry_hardening_wave_*`, `foundry_s7_*`, `foundry_v007_*`, `foundry_v012_*`, `foundry_posix_runner_s9_*`.

Residual architectural risks: V-10, V-13 (see Deferred above).

## Archive: slice completion index

For per-slice checkbox history (S0–S10), see `SLICES.md`. All S0–S7, S8 Phases 1–5, S9.x security, S10.0–S10.1, and S10.2/10.3/10.4 scaffolds are documented there.
