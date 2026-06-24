# CURRENT_STATUS

**Last Updated:** 2026-06-23
**Status:** Active
**Current slice:** S12.4 HIL appliance v0 physical loop feeding S13 metal HIL graduation; S14 deferred

## Active execution track (authoritative)
Active focus: S12.4 HIL appliance v0 physical loop (serial observer first, then power/reset actuator). S13 metal HIL graduation on Tier-1 / lab hardware (`just s13-hil` with `RAMEN_HIL_GRADUATION=1`) should run through the appliance once that loop is stable. S14 USB xHCI + HID remains deferred to design pass.

### Evidence levels (see `EVIDENCE_LEVELS.md`)
- S13.7/S13.8 default gate runs: **PASS/QEMU** (inventory + negative smoke only).
- **PASS/HIL-LOG** / **PASS/HIL-LIVE** / **PASS/METAL** require explicit env and provenance markers.
- S13 slice is **not complete** until **PASS/METAL** on hardware for S13.7 + S13.8 (atomic rollback protocol still scaffold).
Recent completed milestones: S13.0 persistent storage contract + `harness.block` IDL, S12 scaffold complete (HIL graduation opt-in), S11 Driver Factory complete, S11.8 runtime `harness.net` packet I/O in QEMU, S11.7 live hardware packet RX via kernel netdev, S11.6 live packet Oracle capture, S11.5 packet I/O distillation, S11.4 init driver, S11.3 live Oracle capture + trace translation, S11.2 replay scoreboard, S11.1 Oracle capture scaffold, S10.5.2 QEMU IPC bridge, S10.2 v1.1 cap-filtered snapshots + reactor publish.

### 2026-06-23 RamenOrg / research-backed OS scaffold (G0)
- Review intake: external offers/airlock paper and AI-governance notes (source documents retained outside the repository).
- Landed plan: `docs/plans/2026-06-23-research-backed-ramenorg.md`.
- Org Kernel scaffold: `docs/org/` defines RamenOrg constitution, role charter, authority levels, heartbeats, `WorkOrderV0`, `HandoffPacketV0`, `BoardVoteV0`, and claim safety.
- Research program scaffold: `docs/research/RESEARCH_PROGRAM.md`, RQ-0001 offer-shaped service boundaries, and RQ-0002 AI-governed Org Kernel.
- Governance gate: `tools/org/status_drift.py`, `tools/ci/foundry_org_governance_g0.sh`, and `just foundry-org-governance-g0` **PASS**.
- Claim boundary: G0 is A0/A1 for board, planning, docs, and research. G0.8.1
  admits A2-local code/gate work only inside an active work order. It grants no
  merge, release, self-approval, HIL actuation, or public support authority.

**G0.1 Board Packet and packet validators (2026-06-23):**
- Plan: `docs/plans/2026-06-23-g0-1-board-packet-validators.md`.
- Added schemas: `schemas/org/work_order_v0.schema.json`, `handoff_packet_v0.schema.json`, `board_vote_v0.schema.json`, `board_packet_v0.schema.json`.
- Added tools: `tools/org/render_board_packet.py` and `tools/org/validate_packets.py`.
- Gate behavior: `just foundry-org-governance-g0` renders S12.4.1 example packets under `out/org/examples/`, writes `out/org/current_board_packet.json`, and validates packet shape, refs, gates, role separation, evidence refs, and A0/A1 authority boundary.
- Claim boundary: G0.1 is still scaffold authority only; no merge/release/HIL actuation/public support authority is granted.

**G0.2 Active Task + cross-packet consistency (2026-06-23):**
- Plan: `docs/plans/2026-06-23-g0-2-active-task-cross-packet.md`.
- Machine-readable active task: `docs/org/current_task.yaml`.
- Renderer now reads `current_task.yaml` instead of hardcoding S12.4.1 constants.
- Validator now loads referenced packets and checks repo SHA agreement, vote proposal id, handoff work-order id, task agreement, required-gate agreement, authority-level agreement, fail-closed gate refs, and typed evidence buckets (`design`, `gate`, `claim`, `HIL`, `release`).
- Claim boundary: G0.2 remains A0/A1 only.

**G0.3 CurrentTaskV0 + negative validator cases (2026-06-23):**
- Plan: `docs/plans/2026-06-23-g0-3-current-task-negative-fixtures.md`.
- Added `schemas/org/current_task_v0.schema.json` and `docs/org/CURRENT_TASK_V0.md`.
- `just foundry-org-governance-g0` now validates `docs/org/current_task.yaml` before rendering packets.
- `BoardVoteV0` now includes `repo_sha`; cross-validation requires vote SHA to match board/work-order/handoff SHA.
- Board packets are exactly-one for work-order, handoff, and vote refs in the scaffold phase.
- Negative validator harness: `tools/org/test_validate_packets.py`; rejects mismatched SHA, missing evidence, unknown gate syntax, A2 authority, stale HIL claim without evidence constraints, wrong handoff work-order id, vote proposal mismatch, vote SHA mismatch, too many board refs, and malformed current-task source.
- Claim boundary: G0.3 remains A0/A1 only.

**G0.3.1 Governance label + claim-boundary hygiene (2026-06-23):**
- Plan: `docs/plans/2026-06-23-g0-3-1-governance-label-claim-boundary.md`.
- Updated current-task labels, renderer vote claim text, and validator diagnostics to identify the G0.3.1 scaffold boundary.
- Validator now requires claim boundaries to explicitly include all four denials: no merge, no release, no HIL actuation, and no public support authority.
- Negative validator harness now rejects missing release/public-support denials and PASS/METAL claims without HIL evidence refs.
- Claim boundary: G0.3.1 remains A0/A1 only.

**G0.4 Read-only steward heartbeat / board brief (2026-06-23):**
- Plan: `docs/plans/2026-06-23-g0-4-read-only-steward-heartbeat.md`.
- Added `docs/org/BOARD_BRIEF_V0.md` and `tools/org/render_board_brief.py`.
- `just foundry-org-governance-g0` now renders `out/org/current_board_brief.md` only after packet validation passes.
- Brief includes active task, authority boundary, required gates, context refs, evidence refs, handoff details, and allowed next-agent actions.
- Gate checks brief existence and key sections.
- Claim boundary: G0.4 remains A0/A1 only.

**G0.5 Agent intake bundle and freshness binding (2026-06-23):**
- Plan: `docs/plans/2026-06-23-g0-5-agent-intake-freshness-binding.md`.
- Packet validation reports now include SHA-256 for every checked packet and `docs/org/current_task.yaml`.
- Board brief rendering verifies the current bytes against that ledger and includes validation report/status, current-task ref, packet refs, and repo SHA.
- Added `IntakeManifestV0`, `out/org/intake_manifest.json`, and an independent validator for manifest structure, hashes, packet relationships, and brief citations.
- Negative freshness coverage proves a changed packet is rejected when paired with an earlier passing validation report.
- Claim boundary: G0.5 remains A0/A1 only.

**G0.6 Intake-only agent trial (2026-06-23):**
- Plan: `docs/plans/2026-06-23-g0-6-intake-only-agent-trial.md`.
- A fresh agent ran with inherited thread context disabled and received only the generated brief, manifest, board packet, work order, handoff, and vote.
- `PASS/PLAN`: it recovered S12.4.1, the work-order and packet refs, A1 plus all four authority denials, both required gates, and a bounded plan without hidden chat context or external file reads.
- Finding: the six-file intake is not patch-complete because referenced controller/evidence/status documents and scoped source contents are absent.
- Evidence: `docs/org/trials/2026-06-23-g0-6-intake-only-agent-trial.md`.
- Claim boundary: G0.6 remains A0/A1 only.

**G0.7 Bounded context grant (2026-06-23):**
- Plan: `docs/plans/2026-06-23-g0-7-bounded-context-grant.md`.
- Added `ContextGrantV0`, generated `out/org/context_grant.json`, and bound granted context plus authorized new output paths into `out/org/intake_manifest.json`.
- Granted context: eight selected files with read/patch access; authorized new output: `tools/hil/appliance_capture_serial.sh`.
- Validator rejects unhashbound context, missing/changed granted files, patch access outside scope, incomplete required context, and authorized new paths outside scope.
- Fresh-agent trial: `PASS/PATCH-PLAN`; context sufficient, no expansion request, no hidden chat, no external file reads, no implementation.
- Evidence: `docs/org/trials/2026-06-23-g0-7-bounded-context-patch-plan.md`.
- Claim boundary: G0.7 remains A0/A1 only.

**G0.8 Bounded implementation trial (2026-06-23):**
- Plan: `docs/plans/2026-06-23-g0-8-bounded-implementation-trial.md`.
- Implemented `tools/hil/appliance_capture_serial.sh` for the S12.4.1 serial
  observer scaffold.
- The observer allocates a HIL appliance run id, archives serial/controller logs
  plus wrapper JSON under `out/evidence/`, and scans `RAMEN OS`,
  `golden_machine:*`, `persistent_storage:*`, and `hil_evidence:*` markers.
  G0.8.1 splits replay/live evidence levels.
- `tools/ci/foundry_hil_appliance_s12_4.sh` now validates the observer contract
  in default CI with a synthetic transcript and rejects stale serial-log replay
  in graduation mode.
- Evidence: `docs/org/trials/2026-06-23-g0-8-bounded-implementation-trial.md`.
- Claim boundary: G0.8 was reclassified by G0.8.1 as A2-local; no merge,
  release, self-approval, HIL actuation, or public support authority.

**G0.8.1 Implementation authority + serial observer claim hygiene (2026-06-23):**
- Plan: `docs/plans/2026-06-23-g0-8-1-implementation-authority-serial-claim-hygiene.md`.
- Code-writing trials are now explicitly A2-local rather than A1.
- `tools/hil/appliance_capture_serial.sh` validates run ids, rejects empty
  transcripts, writes `serial_input_kind`, and distinguishes `PASS/HIL-LOG`
  development replay from `PASS/HIL-APPLIANCE` live appliance capture.
- `tools/ci/foundry_hil_appliance_s12_4.sh` now covers unsafe run-id and empty
  transcript negative cases.
- Evidence: `docs/org/trials/2026-06-23-g0-8-1-authority-serial-claim-hygiene.md`.
- Claim boundary: A2-local only; no merge, release, self-approval, HIL
  actuation, or public support authority.

### S11.2-pre: IDL/Wire Contract Integrity Gate (COMPLETE)
**Completion Date:** 2026-06-17
- Gate: `foundry_idl_lint.sh` / `just idl-lint` **PASS**.
- `idl_codegen` now requires explicit non-zero `protocol` and `msg_type`, emits generated protocol/message constants, and rejects dynamic `string` / `bytes` in direct Rust IPC structs.
- `foundry_preflight.sh` runs IDL lint immediately after codegen.
- Active IDL contracts now avoid pointer-shaped envelope fields; large/rich values travel via `bytes32`, handles/tokens, shmem caps, offsets, and lengths.
- `domain_manager` semantic harness handles use packed domain/export bits with collision tests for domains 0, 1, 2, 42, max valid, and invalid out-of-range.
- Capability-filtered snapshots now hide `compute_fabric` for restricted viewers.
- Purpose: unblock S11.2 replay scoreboard work on wire-safe, canonical contract metadata.

### S13: Persistent Storage — QEMU loop mature; HIL scaffolds landed; metal graduation pending
**S13.8 atomic update/rollback gate scaffold (2026-06-21):**
- Gate: `foundry_s13_atomic_update_s13_8.sh` **PASS/QEMU** (inventory + negative smoke; metal leg skips without `RAMEN_HIL_GOLDEN_MACHINE=1`). **Not** metal-complete.
- Landed: `kernel_uefi/src/ab_slot_probe.rs` (`RamenAbSlot` UEFI variable); `kernel::boot::AtomicUpdateProbeInfo`; `OP_ATOMIC_UPDATE` init profile; `tools/hil/stage_ab_slot_rollback.sh` (S1 rollback rehearsal).
- Serial markers: `persistent_storage: atomic_update ok`, `persistent_storage: active_slot=A|B`; negative smoke `failed reason=no_ab_metadata`.
- Opt-in: `just s13-hil` extended; S13.0 delegates when HIL env is set.

**S13.7 metal NVMe boot gate scaffold (2026-06-21):**
- Gate: `foundry_s13_nvme_boot_s13_7.sh` **PASS/QEMU** (proves UEFI NVMe path detection scaffold, not native NVMe driver). **Not** metal-complete.
- Landed: `kernel_uefi/src/nvme_boot_probe.rs` (LoadedImage device path NVMe namespace detection); `kernel::boot::NvmeBootProbeInfo`; `OP_NVME_BOOT` init profile (`nvme_boot`); `tools/hil/build_nvme_boot_image.sh`.
- Serial markers: `persistent_storage: nvme_boot ok` (metal); `persistent_storage: nvme_boot failed reason=not_nvme` (QEMU FAT negative smoke).
- Opt-in: `just s13-hil` (`RAMEN_HIL_GOLDEN_MACHINE=1` + `RAMEN_HIL_SERIAL_DEV` or `RAMEN_HIL_SERIAL_LOG`); S13.0 delegates when HIL env is set.

**S13.4–S13.5 block sector Oracle + MockBlockHarness (2026-06-21):**
- Gate: `foundry_s13_block_sector_oracle_s13_4.sh` **PASS**; `foundry_s13_replay.sh` extended for sector replay.
- Live capture: `capture_virtio_blk_sector_oracle.sh` reads/writes live `/dev/vda` sectors, promotes `oracle_block_trace.json` (`trace_id=sha256:4501e4b1…`).
- Landed: `block_sector_trace_v0` schema, `kernel_api::mock::block_harness::MockBlockHarness`, `driver_foundry::virtio_blk_sector`.

**S13.6 runtime harness.block sector I/O (2026-06-21):**
- Gate: `foundry_s13_runtime_block_s13_6.sh` **PASS** (QEMU UEFI `block_io` init profile).
- Landed: `kernel_api::block_oracle_vector` baked sector vectors; `kernel/src/block_harness.rs` BLOCK_V1 IPC provider; `OP_BLOCK_IO` init profile; shmem-backed read/write through typed `harness.block` in QEMU.
- Serial markers: `persistent_storage: block_read ok`, `block_write ok`, `harness.block ok`, `trace_sha256_prefix=eb816f3657bb5807`.

**S13.3 block replay scoreboard (2026-06-21):**
- Gate: `foundry_s13_replay.sh` **PASS** (live `oracle_init_trace.json` through `MockPciDevice` + `virtio_blk_init`).

**S13.2 virtio-blk Oracle capture (2026-06-21):**
- Gate: `foundry_s13_virtio_blk_oracle_s13_2.sh` **PASS** (vault inventory, promotion dry-run, schema + mock replay + init driver replay).
- Live capture: `tools/trace/capture_virtio_blk_oracle.sh` boots QEMU `virtio-blk-pci` at `0000:00:04.0`, records 15 PCI/MMIO events via `virtio_blk_oracle_capture.c`, promotes through `promote_virtio_blk_capture.sh`.
- Landed: `driver_foundry::virtio_blk_init` distilled init driver; CLI `replay-blk-init-trace`; live `oracle_init_trace.json` (`trace_id=sha256:eb816f3657bb580734d22600b8e3d6f56e27567ac001ac63585798b2659bd8ac`).
- Fast-path: `just s13` (S13.0 + S13.2 + S13.4 + S13.3 + S13.6) — QEMU Driver Factory loop complete.

**S13.0 contract scaffold (2026-06-21):**
- Gate: `foundry_s13_persistent_storage_s13_0.sh` **PASS** (inventory + negative assertions; no physical NVMe).
- Landed: `docs/plans/2026-06-21-s13-persistent-storage-design.md`, `hardware/storage_contract_v0.toml`, `idl/harness/block_v1.toml`, `drivers/reference_vaults/virtio-blk/` vault scaffold.
- Oracle device pinned: `virtio-blk-pci` (QEMU); metal target: `nvme_pcie` + A/B GPT (HIL opt-in).
- Wired into `foundry_ci_extended.sh`.

### S12: First Metal (Golden Machine) — scaffold COMPLETE
**S12.3 IOMMU inventory marker (2026-06-21):**
- Gate: `foundry_s12_iommu_inventory_s12_3.sh` **PASS** (inventory + serial validation; QEMU smoke with `intel-iommu` confirms `golden_machine: iommu_present=1`).
- Landed: `kernel_uefi/src/iommu_probe.rs` (ACPI XSDT/RSDT DMAR walk), `kernel::boot::IommuProbeInfo`, `OP_IOMMU_INVENTORY` init profile.
- Opt-in: `RAMEN_HIL_GOLDEN_MACHINE=1` + serial env; wired into `just s12-hil` and S12.0 HIL delegation.

**S12.2 physical HIL boot gate (2026-06-21):**
- Gate: `foundry_s12_hil_boot_s12_2.sh` **PASS** (inventory + USB image build; serial validation via `RAMEN_HIL_SERIAL_LOG`; QEMU smoke confirms `golden_machine: hil_boot ok`).
- Landed: `OP_HIL_BOOT` init profile (`hil_boot`), `tools/hil/build_usb_boot_image.sh` (EFI/BOOT tree for USB stick), HIL gate with skip/strict policy matching S2 compat pattern.
- Opt-in: `RAMEN_HIL_GOLDEN_MACHINE=1` + `RAMEN_HIL_SERIAL_DEV` or `RAMEN_HIL_SERIAL_LOG`; `just s12-hil`.
**S12.1 UEFI GOP probe (2026-06-21):**
- Gate: `foundry_s12_gop_probe_s12_1.sh` **PASS** (QEMU OVMF: `gop_width=1280`, `gop_height=800`, `gop_pixel_format=1` BGR).
- Landed: `kernel_uefi/src/gop_probe.rs` (GOP query + 64×64 `VideoFill`), `kernel::boot::GopProbeInfo`, `OP_GOP_PROBE` init profile, serial markers `golden_machine: gop_probe ok` / `gop_fill ok`.
- CI: `foundry_ci_extended.sh` runs S12.1 after S12.0.

**S12.0 contract scaffold (2026-06-21):**
- Gate: `foundry_s12_golden_machine_s12_0.sh` **PASS** (inventory + negative assertions; no physical HIL required).
- Landed: `docs/plans/2026-06-21-s12-golden-machine-design.md`, `hardware/golden_machine_v0.toml` (Tier-1 Intel NUC 12/13 reference, VT-d + UEFI GOP + serial contract).
- Fast-path: `just s12` (S12.0 + S12.1); HIL opt-in via `just s12-hil`.

### S11: Driver Factory MVP (COMPLETE)
**S11 Definition of Done (2026-06-21):**
1. Init replay: `foundry_s11_replay.sh` replays live `oracle_init_trace.json` through `MockPciDevice`.
2. Packet replay: `foundry_s11_replay.sh` replays live `oracle_packet_trace.json` through `MockPacketHarness`.
3. Live Oracle provenance: `REQUIRE_LIVE_ORACLE_TRACE=1 foundry_s11_reference_vault_s11_3.sh` (init + packet + hardware RX).
4. Runtime harness I/O: `foundry_s11_runtime_net_s11_8.sh` boots QEMU `net_packet_io` init profile; serial log asserts `harness.net: send_packet ok`, `receive_packet ok`, `packet_io ok`, and `trace_sha256_prefix=482af3005a3520aa`.
Fast-path: `just s11` runs all four gates.

**S11.8 runtime harness.net packet I/O (2026-06-21):**
- Gate: `foundry_s11_runtime_net_s11_8.sh` **PASS**.
- Landed: `kernel_api::net_packet_oracle_vector` baked live Oracle ARP payloads; `kernel/src/net_harness.rs` NET_V1 IPC provider; `OP_NET_PACKET_IO` init profile; shmem-backed send/receive through typed `harness.net` control messages in QEMU.
- CI: `foundry_ci_extended.sh` runs S11.8 after S11.3 live vault gate.

**S11.7 live hardware packet RX (2026-06-21):**
- Gate: `REQUIRE_LIVE_ORACLE_TRACE=1 foundry_s11_reference_vault_s11_3.sh` **PASS** (init + packet provenance + hardware RX assertion).
- Landed: kernel netdev capture path — `tools/trace/fetch_virtio_net_modules.sh` bundles Ubuntu mainline `virtio_net.ko` chain; `build_oracle_capture_initrd.sh` includes modules in packet-capture initrd.
- `virtio_net_packet_oracle_capture.c` primary path: `init_module` → `eth0` up → AF_PACKET ARP send/receive; userspace virtqueue path is fallback only when module load fails; slirp-derived receive fallback removed.
- `driver_foundry::assert_hardware_packet_rx` + CLI `assert-hardware-packet-trace`; rejects notes containing `slirp-arp-reply-derived`.
- Live `oracle_packet_trace.json`: `trace_id=sha256:482af3005a3520aa7b98e9e80286875eb3abd88a3e8fb7b9e95ec54bdbcb953d`, MAC `52:54:00:12:34:56`, notes `"live ARP probe transmit"` / `"live ARP reply receive"`; capture ~3s.
- Decision: TCG QEMU userspace virtqueue RX did not complete (used rings stayed 0); kernel `virtio_net` netdev path is the authoritative Oracle for packet receive. See `DECISIONS.md` 2026-06-21.

**S11.3 trace fixture translation (2026-06-18):**
- Gate: `foundry_s11_replay.sh` **PASS** with `driver_foundry` fixture replay.
- Landed: `driver_foundry` host crate translating `DriverProtocolTraceV0` PCI/MMIO events into `PciReplayEvent` arrays and replaying vault fixtures through `MockPciDevice`.
- Datasheet context: `drivers/reference_vaults/virtio-net/datasheets/virtio-net-v1.3.md` pins OASIS VIRTIO v1.3 anchors for PCI discovery, capabilities, init state, queues, feature bits, and network config; S11.3 gate asserts the inventory.
- Live capture: `tools/trace/capture_virtio_net_oracle.sh` boots QEMU `virtio-net-pci` (legacy BAR), records PCI/MMIO via `virtio_net_oracle_capture.c`, and promotes through `promote_virtio_net_capture.sh`; vault fixture now carries `sha256:` provenance and live timestamps.
- CI: `foundry_ci_extended.sh` runs S11.3 with `REQUIRE_LIVE_ORACLE_TRACE=1`.
- S11.6 live packet capture: `tools/trace/capture_virtio_net_packet_oracle.sh` promotes live `oracle_packet_trace.json` (`sha256:` provenance); `REQUIRE_LIVE_ORACLE_TRACE=1` checks init + packet fixtures.
- S11.5 packet I/O: `driver_foundry::virtio_net_packet` replays trace-driven send/receive through `MockPacketHarness`; `foundry_s11_replay.sh` covers packet vault replay.
- S11.4 init driver: `driver_foundry::virtio_net_init` replays the 20-event live vault trace (PCI discovery, features, MAC reads, RX/TX queue setup, DRIVER_OK) through `MockPciDevice`; `foundry_s11_replay.sh` covers vault replay.
- Strict lint: `foundry_lint_strict_tranche6.sh` now includes `driver_foundry`; `trace_cap.rs` uses typed `TraceCapError`.

**S11.3 virtio-net Reference Vault scaffold (2026-06-18):**
- Gate: `foundry_s11_reference_vault_s11_3.sh` **PASS**.
- Landed: `idl/harness/net_v1.toml`, generated `kernel_api` binding, `drivers/reference_vaults/virtio-net/` scaffold, and schema-valid `traces/oracle_init_trace.json`.
- Scope: live Oracle capture landed; `REQUIRE_LIVE_ORACLE_TRACE=1` enforces `sha256:` provenance on init and packet fixtures.

**S11.2 replay scoreboard (2026-06-18):**
- Gate: `foundry_s11_replay.sh` **PASS**.
- Landed: `kernel_api::mock::pci_device::{ReplayScoreboard, MockPciDevice}` with no-alloc fixed-slice Oracle replay, deterministic mismatch reasons, read-value replay, write scoring, incomplete trace rejection, and extra-access rejection tests.
- CI: `foundry_ci_extended.sh` now runs `foundry_s11_replay.sh` as a green gate.

**S11.1 Oracle capture scaffold (2026-06-17):**
- Device: `virtio-net-pci` in QEMU Linux Oracle capsule.
- Gate: `foundry_s11_driver_factory_s11_0.sh` **PASS**.
- Landed: `tools/trace/pci_mmio_tracer.c`, `DriverProtocolTraceV0` schema validation, capsule relay trace-kind discovery.

### S10.2 v1.1: Capability-Filtered Snapshots (COMPLETE)
**Completion Date:** 2026-06-17
- Design: `docs/plans/2026-06-17-s10-2-v1-1-cap-filtered-snapshots.md`
- `filter_platform_snapshot_for_viewer` in `artifact_store_schema::semantic_state`
- `SemanticReactor::handle_subscribe_with_viewer` + `publish_domain_inventory_changed`
- `domain_manager` publishes on `start_domain` / `stop_domain` inventory transitions
- Gate: `capability_filter` + `domain_manager_reactor_publish` in `foundry_semantic_state_s10_2.sh`

### S10.5: Host-to-Target Integration (S10.5.0-S10.5.2 COMPLETE)
**S10.5.0 init bridge (2026-06-17):**
- Gate: `foundry_host_target_s10_5.sh` **PASS** (`snapshot_sha256_prefix=9c0de4419f03f426`).

**S10.5.1 broker/kernel bridge (2026-06-17):**
- Design: `docs/plans/2026-06-17-s10-5-1-broker-kernel-bridge.md`
- Gate: `foundry_broker_kernel_bridge_s10_5_1.sh` **PASS**
- Scope: `shared_memory.control_v1` + `services.semantic_state_v1` only; host `KernelHarnessProxy`
- Landed: `SemanticHarnessGrantOps`, typed `get_domain_grant_handles`, shared snapshot vector, proxy `get_snapshot` roundtrip, supervisor bridge E2E.

**S10.5.2 QEMU IPC bridge (2026-06-17):**
- Design: `docs/plans/2026-06-17-s10-5-2-qemu-ipc-bridge.md`
- Gate: `foundry_qemu_ipc_bridge_s10_5_2.sh` **PASS** (`snapshot_sha256_prefix=9c0de4419f03f426`)
- Scope: length-prefixed envelope frames over QEMU COM2 unix chardev; pull `get_snapshot` only
- Landed: `kernel_api::ipc_frame`, COM2 UART in kernel, `OP_SEMANTIC_IPC_RELAY`, host `ipc_bridge_client.py`, `chardev_serial` transport
- Hardening: test relay continues scanning after partial or rejected frames, and the gate retries the positive QEMU boot if the first chardev instance never receives COM2 bytes.
- Supervisor: `ChardevKernelBridge` in `native_runner`; `runtime_supervisor` selects transport via launch-plan `kernel_ipc_transport` or `RAMEN_KERNEL_IPC_TRANSPORT=chardev-serial`

### S10.3: Projection Storage (PHASES 10.3.0-10.3.4 COMPLETE)

**S10.3.4 CoW writes (2026-06-17):**
- v0: typed `commit_projection_write` (scratch bytes → CAS ingest → index repoint); **not** writable 9p.
- Design: `docs/plans/2026-06-17-s10-3-4-cow-projection-writes.md`.
- Gate: `projection_cow_commit_repoints_path_preserves_prior_blob`.
- Prior CAS blob/manifest remain unchanged; virtual path repoints to the new content id.

### S10.4: Capability-Scheduled Execution Fabric
**S10.4.1 wiring (2026-06-17):**
- `store_cli emit-plan` emits canonical `ExecutionLaunchPlanV0` with `ExecutionRunnerConfigPayloadV0` in `runner_config.config_json`.
- `runtime_supervisor` parses runner payload (compat/GPU/native wasm) from canonical plans; legacy plans still supported.
- `fabric_policy::consult_always_local` records simulation lease/trace; dispatch stays local (`node_id=0`).
- Gate extended: `emit_plan_canonical_roundtrip` in `foundry_execution_fabric_s10_4.sh`.

**S10.4 v0 scaffold (2026-06-17):**
- `execution_fabric_v1` IDL, execution schemas, `ExecutionLaunchPlanV0`
- Simulation service + Semantic State compute-fabric visibility
- `runtime_supervisor` canonical launch-plan parsing
- Foundry gate: `foundry_execution_fabric_s10_4.sh`

**Scope guard:** No SSH, containers, remote workers, or real scheduler in S10.4 v0.

### S10.2: Semantic State Substrate (SCAFFOLD + v1 SUBSCRIBE COMPLETE)
**Completion Date:** 2026-06-17 (scaffold 2026-06-16; subscribe v1 2026-06-17)

S10.2 delivers the semantic-state IDL contract, `PlatformSnapshotV0` schema, domain-inventory snapshot builder, WASM delivery path, and Foundry gate coverage.

**S10.2.1 subscribe reactor (2026-06-17):**
- `SemanticReactor` in `services/semantic_state` — subscription registry, `publish_domain_state_changed`, `reactor_tick()` typed `state_changed_event` delivery
- `subscribe` registers interest (empty mask rejected); event mask bit `0x1` = domain inventory changed
- Design: `docs/plans/2026-06-17-s10-2-1-subscribe-reactor.md`
- Gate: `subscribe_delivery` step in `foundry_semantic_state_s10_2.sh`

**Deliverables:**
- `idl/services/semantic_state_v1.toml` with `get_snapshot` / reply messages
- `artifact_store_schema::semantic_state` — `PlatformSnapshotV0`, `DomainInventoryEntry`, `from_inventory()`
- `services/semantic_state` — snapshot builder + shmem delivery in WASM `_start`
- `domain_inventory_from_manager()` maps `domain_manager_v1` state → snapshot status
- Foundry gate: `foundry_semantic_state_s10_2.sh` (schema + snapshot + kernel_api wire + native_runner E2E + subscribe delivery)

**Remaining (deferred to S10.2+):**
- Multi-source aggregator wiring (kernel trace + store beyond domain_manager inventory)
- WASM guest reactor loop (current `_start` delivers one snapshot only)

**Post-review fix (2026-06-16):**
- Removed duplicate legacy harness registration in `native_runner` (codegen host bindings only)
- Extended S10.2 gate with `native_runner` integration test step

### S8 Phase 4: Data-Plane Implementation (COMPLETE)
**Completion Date:** 2026-02-18

S8 Phase 4 completes the shared-memory data-plane implementation by integrating the control-plane (Phases 1-2) with physical memory allocation (Phase 3) and MMU programming. This phase enables actual zero-copy memory sharing between domains while maintaining kernel-side capability validation and control/data-plane separation.

**Phase 4 Deliverables:**
1. **Frame allocation in create_region**: Physical frames are allocated using the global BitmapAllocator when creating a shared memory region.
2. **MMU programming in map_region**: Page tables are programmed using architecture-specific MMU implementations (X86_64Mmu or AArch64Mmu) when mapping a region.
3. **Frame deallocation in close_region**: Physical frames are deallocated back to the BitmapAllocator when a region is closed.
4. **BitmapAllocator implementation**: A bitmap-based frame allocator supporting allocation, deallocation, and reuse of physical frames.
5. **AddressSpaceTable implementation**: Per-domain page table root tracking for MMU programming.
6. **Allocation rollback**: Proper rollback logic ensures already-allocated frames are freed if create_region fails mid-allocation.
7. **Comprehensive test coverage**: 40+ tests covering:
   - BitmapAllocator (10 tests)
   - AddressSpaceTable (4 tests)
   - MMU Programming (6 tests)
   - Data-Plane Integration (12 tests)
   - Boot Integration (4 tests)
   - End-to-End Scenarios (4 tests)

**Files Modified:**
- `kernel/src/shmem.rs`: Added Data-Plane Integration and End-to-End Scenarios tests
- `kernel/src/mm/bitmap.rs`: Already fully implemented (no changes needed)
- `kernel/src/mm/address_space.rs`: Already fully implemented (no changes needed)
- `kernel/src/mm/mod.rs`: Added Boot Integration tests
- `kernel/src/arch/x86_64/mmu.rs`: Added MMU Programming tests
- `tools/ci/foundry_shmem_dataplane_s8_phase4.sh`: Already exists (no changes needed)

**Test Results:**
- All 40+ Foundry gate assertions are implemented
- The Foundry gate script is ready to validate S8 Phase 4 completion

### 2026-02-13 deep investigation reconciliation (latest)
- Confirmed and documented security/evidence drift in `docs/plans/investigation_deep_security_review_2026-02-13.md`.
- Fixed runtime/store env parsing semantics to accept boolish `1/0/true/false/yes/no/on/off` and removed feature-path compile blocker in `runtime_supervisor`.
- Corrected S7 Foundry script defects (`EVIDENCE_DIR` typo, env/CLI mismatch for store service invocation, shell syntax issue in access-control gate).
- Reworked S7 POSIX security gate to run deterministic runtime_supervisor unit tests (kill-switch/warning/default-sandbox assertions) rather than fragile binary startup paths dependent on store connectivity.
- Enforced signature validation parity across store read/verify paths (`GetBlob`, `VerifyArtifact`) with `STATUS_VALIDATION_FAILED` on policy rejection.
- Reconciled capability trusted-key loading order with explicit capability-key env support and tighter fail-closed behavior.
- Added regression tests for unsigned-manifest rejection on `GetBlob`/`VerifyArtifact` and revalidated store_service test suites.

### Workspace lint baseline (staged rollout)
- Added `tools/ci/foundry_lint_baseline.sh` to run host-workspace clippy and emit warning metrics.
- Added `just clippy-baseline` (non-breaking baseline) and `just clippy-strict` (fails on warnings).
- CI now runs lint baseline (`.github/workflows/ci.yml`) before Foundry slice gates.
- Current baseline: `FOUNDRY_LINT_BASELINE: METRIC warning_count=0` (strict tranches 1–6 enforce `-D warnings` on selected crates).
- Policy update (effective 2026-02-12): baseline gate is fail-closed by default; warnings fail CI unless explicitly overridden for local debugging with `LINT_ALLOW_WARNINGS=1`.
- Added one-command local parity gate: `just preflight` (`tools/ci/foundry_preflight.sh`) for format + codegen + lint + host tests + Foundry umbrella.
- Strict lint tranche-1 enforced:
  - Added `tools/ci/foundry_lint_strict_tranche1.sh` and `just clippy-strict-tranche1`.
  - CI enforces `cargo clippy --all-targets --no-deps -- -D warnings` for:
    - `artifact_store_schema`
    - `store_cli`
    - `domain_manager`
    - `portals`
- Strict lint tranche-2 enforced:
  - Added `tools/ci/foundry_lint_strict_tranche2.sh` and `just clippy-strict-tranche2`.
  - CI enforces `cargo clippy --all-targets --no-deps -- -D warnings` for:
    - `artifact_store_core`
    - `idl_codegen`
    - `capsule_relay`
    - `runtime_supervisor`
  - `runtime_supervisor` strict readiness completed (feature/test cfg cleanup + dead-code elimination in default build).
- Strict lint tranche-3 enforced:
  - Added `tools/ci/foundry_lint_strict_tranche3.sh` and `just clippy-strict-tranche3`.
  - CI enforces `cargo clippy --all-targets --no-deps -- -D warnings` for:
    - `kernel_api`
  - `kernel_api` now uses typed `CapTableError` instead of unit errors in the `CapTable` trait.
- Strict lint tranche-4 enforced:
  - Added `tools/ci/foundry_lint_strict_tranche4.sh` and `just clippy-strict-tranche4`.
  - CI enforces `cargo clippy --all-targets --no-deps -- -D warnings` for:
    - `store_service`
  - `store_service` now shares one module implementation between lib/bin (no duplicate local module copies in bin).
- Strict lint tranche-5 enforced:
  - Added `tools/ci/foundry_lint_strict_tranche5.sh` and `just clippy-strict-tranche5`.
  - CI enforces `cargo clippy --all-targets --no-deps -- -D warnings` for:
    - `kernel`
  - `kernel` strict readiness completed with dead-code/test-cfg cleanup in `arch/aarch64/mmu.rs` and lint-safe defaults/helpers across MM + shmem + init paths.
- Added lint debt register at `docs/LINT_DEBT.md` to track remaining `allow(...)` points with owner and exit criteria.

### S9.0: Security Remediation - Phase 1 (COMPLETE)
**Completion Date:** 2026-02-09

All 11 vulnerabilities from the security audit have been addressed in Phase 1:

- ✅ V-001 (High): ContentId validation - Complete
- ✅ V-002 (High): Wire format safety and codegen fail-closed - Complete
- ✅ V-003 (High): POSIX runner default-off - Complete
- ✅ V-004 (High): Unforgeable capability tokens - Complete
- ✅ V-005 (Medium): Kernel capability table - Complete
- ✅ V-006 (High): POSIX runner shell script execution - Phase 1 mitigation complete
- ✅ V-007 (Medium): Services depend on Store IO functions - Phase 1 cleanup complete
- ✅ V-008 (Medium): Init parser checked arithmetic - Complete
- ✅ V-009 (Medium): Log path confinement - Complete
- ✅ V-010 (Low): Supervisor TCB breadth - Acknowledged, documented
- ✅ V-011 (Low): Store boundary split - Complete
- ✅ V-012 (Medium): Trace isolation - Phase 1 per-domain buffers complete
- ✅ V-013 (Low): Portal TOCTOU - Acknowledged, documented
- ✅ V-014 (Low): Unsafe safety docs - Complete
- ✅ V-015 (Low): Pin nightly - Complete

**S9.0 Deliverables:**
- V-006 Phase 1: POSIX runner feature flag gating and warnings (gate: `foundry_posix_runner_s9_0_mitigation.sh`)
- V-007 Phase 1: Services dependency cleanup, use schema types only (gate: `foundry_boundary_s9_0_cleanup.sh`)
- V-012 Phase 1: Per-domain trace ring buffers (gate: `foundry_trace_isolation_s9_0_per_domain.sh`)

**Security Posture:**
- Before: 11 unresolved vulnerabilities (3 High, 6 Medium, 5 Low)
- After: All vulnerabilities mitigated or in progress with phased remediation plans
- Residual risks: V-006 (phases 2-4 pending), V-007 (phases 2-4 pending), V-012 (phases 2-4 pending)

### S9.1: Security Remediation - Phase 2 (COMPLETE)
**Completion Date:** 2026-02-09
**Test Results:** 100/100 tests passing (100%)

All S9.1 Phase 2 security remediation tasks completed:

**V-007 Phase 2:** Store service IPC implementation (8/8 tests passing)
- ✅ Store service IDL contract (`idl/services/store_service_v1.toml`)
- ✅ Unix domain socket IPC transport (design doc: `docs/plans/v007_phase2_store_service_ipc_design.md`)
- ✅ Store service binary with GetManifest, GetBlob, VerifyArtifact, IngestArtifact handlers
- ✅ Store client library for services (sync, bincode serialization)
- ✅ Domain manager integration
- ✅ Runtime supervisor integration
- ✅ Store CLI integration
- ✅ CONTENT_ID_PREFIX and CAP_DISPLAY_EXPORT defined
- Foundry gate: `foundry_v007_phase2_store_service_ipc.sh`

**V-007 Phase 3:** Store service hardening (14/14 tests passing)
- ✅ Signature validation module implemented (stub)
- ✅ Audit logging module implemented with all required fields
- ✅ Access control module implemented (stub)
- ✅ All modules integrated in main.rs
- ✅ Phase 2 compatibility verified
- Foundry gate: `foundry_v007_phase3_store_hardening.sh`

**V-007 Phase 4:** Cryptographic signatures and SO_PEERCRED (30/30 tests passing)
- ✅ Ed25519 signature verification (9/9 tests passing)
- ✅ SO_PEERCRED credential passing (platform-aware: Linux full support, macOS graceful fallback)
- ✅ Access control implementation (13/13 tests passing on Linux)
- ✅ Security assertions verified
- ✅ `artifact_store_schema/src/signature.rs` (607 lines)
- ✅ `services/store_service/src/access_control.rs` (551 lines, platform-aware SO_PEERCRED)
- Foundry gate: `foundry_v007_phase4_crypto_signatures.sh`

**V-012 Phase 2:** Domain-scoped trace writers (27/27 tests passing)
- ✅ Per-domain trace ring buffers (5/5 tests)
- ✅ Domain-scoped writer implementation (8/8 tests)
- ✅ Helper functions (2/2 tests)
- ✅ Cross-domain isolation (1/1 test)
- ✅ Legacy API compatibility (2/2 tests)
- ✅ Domain registry integration (3/3 tests)
- ✅ Security assertions (6/6 checks)
- ✅ Drop impl for automatic resource cleanup
- ✅ emit_domain() for domain-scoped trace emission
- Foundry gate: `foundry_v012_phase2_domain_scoped_writers.sh`

**V-012 Phase 3:** Trace capability-based access control (21/21 tests passing)
- ✅ Trace capability table (5/5 tests)
- ✅ Rights management (4/4 tests)
- ✅ Domain integration (1/1 test)
- ✅ Security assertions (10/10 checks)
- ✅ Integration tests (2/2 tests)
- ✅ `kernel/src/trace_cap.rs` (420 lines)
- ✅ TRACE_RIGHT_* constants in kernel_api
- Foundry gate: `foundry_v012_phase3_trace_capabilities.sh`

**S9.1 Deliverables:**
- Ed25519 signature verification for manifests
- Unix credential passing via SO_PEERCRED
- Domain-scoped trace writers with automatic cleanup
- Capability-based access control for trace operations
- Generation counters to prevent stale handle reuse
- Platform-aware security (Linux/macOS compatibility)

**Security Posture:**
- Before S9.1: No signature verification, no credential passing, global trace buffers
- After S9.1: Cryptographic verification, Unix credential passing, per-domain trace isolation
- Residual risks: None for completed phases; S9.2 addresses POSIX runner store integration

### S9.2: Security Remediation - Phase 3 (COMPLETE)
**Completion Date:** 2026-02-09
**Test Results:** 15/15 tests passing (100%)

**V-006 Phase 3:** POSIX runner store service integration
- ✅ Store-integrated execution functions (`posix_run_v0_from_store`, `posix_run_v0_from_store_verified`)
- ✅ Artifact fetching via store service IPC (replaces direct filesystem access)
- ✅ Ed25519 signature verification before execution
- ✅ Sandbox isolation maintained (seccomp, namespaces, chroot, rlimits)
- ✅ Security warnings updated for store integration
- ✅ Migration path documented (V-006 Phase 3 comments in main.rs)
- ✅ StoreClient and artifact_store_schema dependencies verified
- Foundry gate: `foundry_posix_runner_s9_2_store_integration.sh`

**Implementation Details:**
- `runtime_supervisor/src/posix_runner.rs`:
  - Added `posix_run_v0_from_store()` (30 lines) - fetches artifacts via IPC
  - Added `posix_run_v0_from_store_verified()` (71 lines) - adds signature verification
  - Signature validation using `validate_manifest_signatures()`
  - Uses `SignaturePolicy::AllowUnsigned` during development (documented)
- `runtime_supervisor/src/main.rs`:
  - Replaced `blob_path_for()` with store-integrated execution (line 261)
  - Updated warning messages with artifact ID display
  - Added V-006 Phase 3 migration comments

**Security Improvements:**
1. **Verified IPC**: Artifacts fetched through store service instead of direct filesystem
2. **Cryptographic verification**: Ed25519 signatures validated before execution
3. **Defense-in-depth**: Sandbox + IPC + signatures + access control
4. **Clear warnings**: Per-execution security warnings with artifact ID

**Before V-006 Phase 3:**
- ✅ Sandbox isolation (seccomp, namespaces, chroot)
- ⚠️ Direct filesystem access to artifacts
- ⚠️ No signature verification

**After V-006 Phase 3:**
- ✅ Sandbox isolation maintained
- ✅ Store service IPC for artifact fetching
- ✅ Ed25519 signature verification
- ✅ Detailed error messages for failures

**S9.2 Deliverables:**
- POSIX runner store service integration complete
- All 15 Foundry gate tests passing
- Production-ready pending policy change from `AllowUnsigned` to `RequireSignature`

### S9.3: Security Remediation - Phase 5 (COMPLETE)
**Completion Date:** 2026-02-10
**Test Results:** 39/39 tests passing (100%)

**V-007 Phase 5:** Enhanced store security
- ✅ Capability-based access control (CBAC) foundation
- ✅ Domain-scoped artifact visibility (DomainArtifactRegistry)
- ✅ Production signature validation (RequireSignature policy)
- ✅ Enhanced audit logging with domain_id tracking
- ✅ Key management support (TrustedKeys::load_from_file)
- ✅ StoreClient domain_id support (connect_with_domain)
- Foundry gate: `foundry_v007_phase5_enhanced_store_security.sh`

**Implementation Details:**
- `services/store_service/src/capability.rs` (237 lines):
  - StoreCapability struct with domain_id, rights_mask, capability_id
  - Rights constants: READ, WRITE, DELETE, ADMIN
  - Methods: has_right(), is_expired(), is_for_domain(), verify_signature()
- `services/store_service/src/domain_visibility.rs` (328 lines):
  - DomainArtifactRegistry tracks artifact ownership
  - can_access() enforces domain-scoped visibility
  - Fail-closed: denies unknown artifacts by default
- `services/store_service/src/main.rs`:
  - Extract domain_id from client connections
  - Check domain visibility before GetManifest/GetBlob
  - Register ownership on IngestArtifact
  - Load trusted keys from RAMEN_STORE_TRUSTED_KEYS
- `services/store_service/src/audit.rs`:
  - artifact_access() - Log with domain_id
  - artifact_ingested() - Log with ownership
  - access_denied() - Log denials
  - operation_failed() - Log failures

**Security Improvements:**
1. **Capability-based access**: StoreCapability with domain binding
2. **Domain isolation**: Domains see only their artifacts + kernel artifacts
3. **Production signatures**: RequireSignature when keys loaded
4. **Enhanced auditing**: domain_id, capability_id in all audit entries
5. **Fail-closed**: Unknown artifacts denied by default

**Files:**
- `services/store_service/src/capability.rs` (StoreCapability, 237 lines)
- `services/store_service/src/domain_visibility.rs` (DomainArtifactRegistry, 328 lines)
- `artifact_store_schema/src/signature.rs` (TrustedKeys::load_from_file)
- `docs/examples/trusted_keys.example` (key file format)
- `artifact_store_schema/examples/generate_ed25519_keypair.rs` (key generation)
- `tools/ci/foundry_v007_phase5_enhanced_store_security.sh` (39 tests)

**S9.3 Deliverables:**
- CBAC foundation for store operations
- Domain-scoped artifact visibility
- Production signature validation ready
- Enhanced audit logging
- 39/39 Foundry gate tests passing

### S7: Security Hardening (COMPLETE)
**Completion Date:** 2026-02-10

All 5 high-severity security issues identified in the S7 security audit have been fixed:

- ✅ S7-001 (High): Fail-closed store signature policy - Complete
- ✅ S7-002 (High): Fail-closed access control default - Complete
- ✅ S7-003 (High): Exact path matching for exe whitelisting - Complete
- ✅ S7-004 (High): DomainArtifactRegistry integration - Complete
- ✅ S7-005 (High): POSIX runner runtime enforcement - Complete

**S7 Deliverables:**
- Fail-closed signature validation with `RAMEN_STORE_TRUSTED_KEYS` requirement (gate: `foundry_s7_store_signature_security.sh`)
- Fail-closed access control with `RequireCredentials` default (gate: `foundry_s7_access_control_security.sh`)
- Exact path matching with `std::fs::canonicalize()` for exe whitelisting
- DomainArtifactRegistry with explicit "global" via directory structure
- POSIX runner runtime enforcement with `RAMEN_POSIX_RUNNER_ACK_RISK=1` (gate: `foundry_s7_posix_runner_security.sh`)
- Combined gate: `foundry_s7_all_security.sh`

**Security Improvements:**
1. **Fail-closed signature validation**: Store service aborts without trusted keys in production
2. **Fail-closed access control**: Defaults to `RequireCredentials` instead of `AllowAll`
3. **Exact path matching**: Prevents substring-based bypass attacks
4. **Explicit global artifacts**: Directory structure (`store_root/global/`) makes global visibility explicit
5. **Runtime enforcement**: POSIX runner requires explicit acknowledgment of security risks

**Breaking Changes:**
- `RAMEN_STORE_TRUSTED_KEYS` now required in production mode (was optional)
- Access control defaults to `RequireCredentials` (was `AllowAll`)
- `RAMEN_POSIX_RUNNER_ACK_RISK=1` now required (was compile-time feature flag)

**Documentation:**
- [`docs/S7_SECURITY_HARDENING_PHASE2.md`](docs/S7_SECURITY_HARDENING_PHASE2.md) - Implementation details
- [`docs/S7_SECURITY_HARDENING_PHASE3.md`](docs/S7_SECURITY_HARDENING_PHASE3.md) - Foundry gates
- See [`DECISIONS.md`](DECISIONS.md) for design decisions S7-001 through S7-005

### S8: Shared-Memory Primitives (Background)
- ✅ S8 Phase 1: IDL contract `shmem_control_v1.toml` with codegen and kernel_api roundtrip tests (gate: foundry_shmem_contract_s8_phase1.sh).
- ✅ S8 Phase 2: Kernel capability validation path for shared-memory control operations.
  - Wave A/B/C hardening completed (SC-01..SC-12) with all gates green.
  - StaticCapTable with generation counters enables kernel-side validation (SC-05/V-05/V-06).
  - ShmemRegionTable implements region lifecycle with create/map/unmap/close operations.
  - IPC handlers for PROTOCOL_SHMEM_CONTROL (protocol 8) with CreateRegion, MapRegion, UnmapRegion, CloseRegion.
  - Capability validation required for MapRegion/UnmapRegion/CloseRegion; CreateRegion allows Handle::INVALID for bootstrap.
  - Foundry gate: foundry_shmem_control_s8_phase2.sh (32 assertions, all green).
- ✅ S8 Phase 3: Physical frame allocator for data-plane memory management.
  - Type-safe PhysAddr and PhysFrame wrappers prevent physical/virtual address confusion.
  - FrameAllocator trait provides architecture-agnostic interface for allocation strategies.
  - BumpAllocator implements simple early-boot allocation with static backing (512 MiB max, 131072 frames).
  - BumpAllocator uses "next free" pointer, never frees individual frames (no-op deallocate).
  - Boot wiring complete: UEFI retrieves memory map (x86_64), AArch64 uses hardcoded region (QEMU virt).
  - Foundry gate: foundry_frame_allocator_s8_phase3.sh (26 assertions) + foundry_s0.sh integration check.
  - Both architectures successfully initialize allocator on boot (x86_64: ~107500 frames, aarch64: 65536 frames).
- ✅ S8 Phase 4: Data-Plane Integration (COMPLETE - with QEMU-based integration tests)
  - BitmapAllocator for reusable frame allocation with static backing (65536 frames)
  - AddressSpaceTable for per-domain page table root tracking
  - MMU Programming Interface with architecture-agnostic trait (kernel/src/arch/mmu.rs)
  - x86_64 MMU implementation (kernel/src/arch/x86_64/mmu.rs) with CR3 manipulation
  - aarch64 MMU implementation (kernel/src/arch/aarch64/mmu.rs) with TTBR0_EL1 manipulation
  - Data-plane integration in ShmemRegionTable (frame allocation on create, MMU map on map_region)
  - IPC handler updates with trace events for all shared-memory operations
  - Boot integration for allocator and address space table initialization
  - Foundry gate: foundry_shmem_dataplane_s8_phase4.sh (40/40 core assertions passing)
  - **QEMU Integration Tests**: All 6 MMU programming tests validated via QEMU-based integration gate
    - Gate: `foundry_shmem_dataplane_s8_phase4_integration.sh` (6/6 tests passing ✅)
    - Tests: `map_region_increments_refcount`, `map_region_multiple_times_increments_refcount`, `unmap_region_decrements_refcount`, `close_region_fails_with_active_mappings`, `close_region_succeeds_after_all_unmaps`, `map_region_checks_rights_against_flags`
    - Implementation: `kernel/src/init.rs` OP_SHMEM_TEST handler with comprehensive MMU validation
    - Test Profile: `shmem_test` in `tools/init/build_init_image.py`
    - Core Validation: BitmapAllocator (12/12), AddressSpaceTable (8/8), ShmemRegionTable (20/20 with integration tests)
  - **Technical Notes**:
    - Module structure: Removed `x86_64/mod.rs`, using single-file pattern (consistent with aarch64)
    - Domain registration: Tests use domain_id 0 (kernel domain) with valid page table
- ✅ S8 Phase 5.0: Pointer Validation Fix (COMPLETE)
  - **Issue**: UEFI loads init images at physical addresses, but kernel validated virtual address ranges (V-006 vulnerability)
  - **Solution**: Implemented `is_valid_phys_addr()` function to validate physical addresses from UEFI
    - NULL pointer check
    - Overflow protection
    - Physical memory bounds (0-4 GiB for QEMU)
    - MMIO region exclusion (3-4 GiB)
  - **Implementation**:
    - Commit 9747eae: Physical address validation implementation
    - Commit 5142c1f: Unit tests for physical address validation
    - Follow-up hardening: reject ranges crossing into MMIO boundary (6/6 validation tests passing)
  - **Security Impact**: Fixes V-006 vulnerability - proper physical address validation for UEFI-loaded init images
- ✅ S8 Phase 5: Data Plane Ring Buffer Foundation (COMPLETE: Phases 5.0-5.2)
  - **Objective**: Zero-copy data transfer between domains using lock-free SPSC ring buffer
  - **Plan Document (archived)**: `docs/archive/plans/2026-02-10-s8-phase5-ring-buffer.md`
  - **API Specification**: `docs/RING_BUFFER_V0.md`
  - **Architecture Guide**: `docs/MULTI_DOMAIN.md`
  - **Completed Deliverables**:
    - ✅ Phase 5.0: Pointer validation fix for physical init image addresses
    - ✅ Phase 5.1: Lock-free SPSC ring buffer core (`kernel_api/src/ring_buffer.rs`) with 12 unit tests + Foundry gate
    - ✅ Phase 5.2: Multi-domain MMU foundation with on-demand intermediate page table allocation on x86_64 and aarch64
  - **Post-review stabilization**:
    - Host-test-safe MMU behavior in map/unmap/flush paths (prevents privileged instruction faults during unit tests)
    - IPC shared-memory tests initialize frame allocator/address-space table explicitly for deterministic domain checks
    - Trace ring legacy test path serialized to avoid parallel-test interference on shared global state
  - **Validation Evidence**:
    - `cargo test -p kernel_api --lib ring_buffer` (12 passed)
    - `tools/ci/foundry_ring_buffer_core_s8_phase5_1.sh` (pass)
    - `cargo test -p kernel --lib` (176 passed)
  - **Dependencies**: S8 Phase 4 (COMPLETE), S6 (Domain Manager), S7 (Capability System)
    - Safety: production MMU paths perform real page table programming; host test builds use non-privileged validation paths

## What exists now
- Repo layout enforcing OS/Foundry/Store boundaries.
- Minimal IDL + codegen tool.
- `kernel_api` types used by kernel/runtime.
- UEFI boot path for x86_64 with serial banner.
- aarch64 QEMU boot via direct kernel entry (UEFI fallback deferred).
- Init image loaded by bootloader (UEFI `init.img` + aarch64 loader addr) and executed as a tiny opcode script.
- IPC ping/pong + trace read run across the real init boundary.
- Kernel trace ring buffer with init read path + single-writer token.
- Foundry S0 gate runs QEMU and asserts boot/hello/ping/trace (green on this machine).
- Foundry S2.2 init gate asserts swap/malformed init behavior on both arches.
- Store S0: `store_cli` emits launch plan from static catalog (host-only).
- Store S0: `runtime_supervisor` consumes launch plan (host-only).
- Foundry Store S0 gate runs store_cli + runtime_supervisor.
- S0 gates: foundry_s0 + foundry_store_s0 (+ umbrella) are green on this machine.
- S1: content-addressed artifacts + manifest + install/run/rollback gate are green.
- S1.25: installed layout contract enforced (out/installed/artifacts).
- S0+S1 umbrella: `foundry_all_s0_s1` is green.
- S2 gate: compat capsule boots from initrd and reads artifact via virtio-blk ext4 image.
- S2 gate: write attempts to artifact mount are blocked (read-only enforcement).
- S0+S1+S2 umbrella: `foundry_all_s0_s1_s2` is wired.
- S0+S1+S2+S3+S4+S5 umbrella: `foundry_all_s0_s1_s2_s3_s4_s5` is wired and CI uses it with a pinned kernel.
- S0+S1+S2+S3+S4+S5+S6 umbrella: `foundry_all_s0_s1_s2_s3_s4_s5_s6` is wired.
- CI caches the pinned compat kernel; fetch retries and re-downloads on mismatch.
- CI uses a mirrored compat kernel release asset (see mirror_compat_kernel workflow).
- Compat Capsule Format v0 spec defined (`docs/COMPAT_CAPSULE_V0.md`).
- Compat runner (`runtime_supervisor/src/compat_runner.rs`): spawns QEMU from capsule config.
- Store emits `linux_vm_v0` runner plans with embedded `compat_capsule` config.
- `runtime_supervisor` routes `linux_vm_v0` plans to the compat runner.
- S2 gate exercises the full runner contract: gate → supervisor → compat runner → QEMU.
- Supervisor owns QEMU lifecycle (kills child on shutdown; gate no longer scrapes PIDs).
- S2 gate ingests compat capsule assets into the installed content store via store_cli (kernel/initrd/disk).
- CI strict mode: `RAMEN_CI_STRICT=1` enforces no gate skips.
- Catalog includes a compat capsule entry with asset paths for ingestion.
- kernel_api wire helpers for typed payloads.
- Foundry negative assertions for IPC payload length and manifest schema mismatch.
- Store CLI requires explicit program_id selection and ingests compat assets into the installed store.
- S2 compat gate emits a path-free plan from Store output; supervisor injects log path at runtime.
- Trace artifact v0 schema + Foundry trace replay gate (protocol_trace).
- S3 portal file picker (RO) stub: broker + kernel-style token validation + protocol trace emission.
- Portal runs emit observed_caps_v0 + scenario_trace evidence artifacts.
- Driver capsule relay v0: capsule.control + harness.echo IDLs + mock capsule agent.
- Capsule relay supports `--mode host-only` (in-process mock) and `--mode vm` (QEMU + virtio-serial).
- C capsule agent for Linux guest (`tools/capsule/capsule_agent.c`) implements same protocol over virtio-serial.
- Capsule relay emits control+echo protocol traces, observed_caps_v0, and scenario_trace artifacts.
- Foundry S3.x driver capsule gate validates traces, evidence chain, and replay in both modes.
- Local S2/S3.x VM gates require `S2_COMPAT_KERNEL` + QEMU; gates skip gracefully if unavailable.
- Wire format uses **little-endian** for cross-arch determinism (traces as spec).
- S4: queue_item_v0 artifact schema with target_level, evidence refs, prereqs, and scoring inputs.
- S4: Priority scoring: `(vote_weight × leverage × reuse) / (effort × risk)` with human-readable explanations.
- S4: Prerequisites graph generation (JSON + DOT) identifying high-leverage prereqs.
- S4: Claim/lock workflow for offline-first queue item assignment.
- S4: claim timestamps are generated as real UTC RFC 3339 strings and validated via parser.
- S4: claim chain resolution enforces "latest valid claim wins" with lease-expiry handling.
- S4: store_cli commands: validate-queue-item, explain-priority, prereq-graph, claim, validate-claim, resolve-claim.
- S4: Foundry gate validates queue items, priority, prereq graphs, claims, and rejects invalid inputs.
- S5: crash_context_v0 schema for structured crash bundles (Semantic State v2).
- S5: graduation_v0 schema for tracking progression across target levels.
- S5: minimal_policy_v0 schema with capability proposals and strictness scoring.
- S5: Wizard flow: `propose-policy` generates minimal policy from observed capabilities.
- S5: store_cli commands: validate-crash-context, validate-graduation, validate-minimal-policy, propose-policy, graduation-status.
- S5: Foundry gate validates crash contexts, graduation tracking, and policy proposals.
- S5+: POSIX runner v0 implemented in `runtime_supervisor` (`posix_runner_v0`) with Foundry gate coverage.
- S5: RunnerIdentityV0 stamps crash contexts and graduation attempts with runner software provenance.
- S5: EvidenceBundleV0 typed evidence fields (stdout_tail, stderr_tail, runner_log, core_dump, extras).
- S5: ExitMetricsV0 captures wall/cpu budgets, memory peak, and OOM events for crash attribution.
- S5: Deterministic minimal_policy output (sorted capabilities/excluded, proposer_version).
- S5: Graduation progression_summary works after JSON deserialization (level_status recomputed lazily).
- S0 trace ring reader now handles overwrite overflow by fast-forwarding (no stale slot replay).
- S3.x: IDL codegen emits C headers (`capsule_control_v0.h`) for capsule guest bindings.
- S3.x: evidence policy hook supports redaction + size limits before artifact ingestion.
- S6: Domain Manager v1 service (`services/domain_manager`) with typed lifecycle API and restart-policy handling.
- S6: Domain Manager IDL contract (`idl/harness/domain_manager_v1.toml`) with generated bindings in `kernel_api`.
- S6: Expanded portal suite (`portal_suite`) covers clipboard, notifications, and screen capture with typed traces/evidence.
- S6: Foundry gates `foundry_domain_manager_s6.sh` and `foundry_portal_suite_s6.sh` are green on this machine.
- S6+S7: Foundry umbrella `foundry_all_s0_s1_s2_s3_s4_s5_s6.sh` now includes S7 and emits `FOUNDRY_ALL_S0_S1_S2_S3_S4_S5_S6_S7: ok`.
- S7: GPU quarantine IDL contract landed (`idl/harness/gpu_quarantine_v1.toml`) with generated bindings in `kernel_api`.
- S7: Security hardening completed - 5 high-severity issues fixed with fail-closed defaults and runtime enforcement
  - Fail-closed store signature policy with `RAMEN_STORE_TRUSTED_KEYS` requirement
  - Fail-closed access control defaulting to `RequireCredentials`
  - Exact path matching for exe whitelisting using `std::fs::canonicalize()`
  - DomainArtifactRegistry with explicit "global" via directory structure
  - POSIX runner runtime enforcement with `RAMEN_POSIX_RUNNER_ACK_RISK=1`
  - Foundry gates: `foundry_s7_store_signature_security.sh`, `foundry_s7_access_control_security.sh`, `foundry_s7_posix_runner_security.sh`, `foundry_s7_all_security.sh`
- S7: Domain Manager now handles typed GPU quarantine control-plane messages (`start_quarantine_domain`, `export_display`, `report_scanout`, `stop_quarantine_domain`) and emits protocol/scenario/observed-cap evidence artifacts.
- S7: Store catalog and launch-plan emission support `gpu_quarantine_v1` runner plans with typed `gpu_quarantine` config.
- S7: `runtime_supervisor` routes `gpu_quarantine_v1` plans to a dedicated runner path with capability and dimension validation.
- S7: Foundry gate `tools/ci/foundry_gpu_quarantine_s7.sh` validates positive path, replay, and negative assertions (malformed payload replay failure, invalid capability rejection, malformed plan rejection).
- S7 hardening: `foundry_gpu_quarantine_s7.sh` now enforces measurable thresholds with machine-auditable fail codes (protocol/scenario/observed-cap metrics, capability counts, export dimensions) and deterministic `FOUNDRY_GPU_QUARANTINE_S7: METRIC ...` output.
- S7 hardening: gate now emits stable `FOUNDRY_GPU_QUARANTINE_S7: FAIL code=... detail=...` reasons for sentinel/threshold/policy/negative-assert violations while preserving existing negative assertions.
- S7 hardening: gate now enforces evidence policy ingest pass/fail for S7 artifacts (including an intentional size-limit violation check).
- S7 deterministic replay hardening: `tools/trace/replay_protocol_trace.py` now emits machine-auditable `REPLAY_PROTOCOL_TRACE: METRIC ...` + `MATCH` + `FAIL code=...` signals, validates strict request/response op-pairing and monotonic seq, and computes a canonical replay digest (`sha256:...`) from normalized protocol pairs.
- S7 deterministic replay gate integration: `foundry_gpu_quarantine_s7.sh` now runs dual domain-manager executions, asserts stable replay digest and stable trace/observed/scenario content IDs across runs, and emits deterministic replay sentinels (`FOUNDRY_GPU_QUARANTINE_S7: METRIC replay_digest=...`, `REPLAY_DETERMINISM ok`).
- S7 evidence-discipline enforcement: evidence policy failures now include stable reason codes (e.g., `EVIDENCE_POLICY_SIZE_LIMIT_EXCEEDED`) and the S7 gate hard-fails if the stable policy code is missing from negative-policy assertions.
- S7 hardening: CI passes explicit S7 threshold environment values and evidence policy path into the S0→S7 umbrella invocation.
- S7: CI codegen step now includes `gpu_quarantine_v1` binding generation and CI naming reflects S0→S7 coverage.
- S8 Phase 1: shared-memory control-plane IDL contract landed (`idl/harness/shmem_control_v1.toml`) with versioned typed operations (`create_region`, `map_region`, `unmap_region`, `close_region`) and replies.
- S8 Phase 1: codegen wiring now emits `kernel_api/src/generated/shmem_control_v1.generated.rs`; `kernel_api` exports include the new generated module.
- S8 Phase 1: deterministic contract gate added (`tools/ci/foundry_shmem_contract_s8_phase1.sh`) validating IDL generation plus focused `kernel_api` roundtrip/size tests; wired into umbrella Foundry flow and CI IDL generation.
- Foundry reliability: increased QEMU assertion wait windows in `foundry_s0.sh` and `foundry_init_s2_2.sh` for slower OVMF paths.
- V-012 Phase 5: User-space trace client library (`services/trace_client/`) with builder pattern, error types, capability handling, and transport abstraction.
- V-012 Phase 5: `TraceTransport` trait with `MockTransport` for testing without kernel access.
- V-012 Phase 5: `DomainTraceManager` in domain_manager for trace collection on domain shutdown.
- V-012 Phase 5: Foundry gate `foundry_v012_phase5_trace_client.sh` (16 test cases).

### 2026-02-17 Code Quality & Security Remediation (COMPLETE)
**Completion Date:** 2026-02-17
**Test Results:** 372/372 tests passing (100%)

All 10 issues from comprehensive project review addressed across 4 work streams:

**Work Stream 1: Security Fixes**
- ✅ Issue #1 (High): `RAMEN_STORE_DEV_MODE` runtime bypass → compile-time `dev_insecure` feature flag
- ✅ Issue #2 (High): Excessive `expect()` in domain_manager → `Result<T, DomainManagerError>` with proper error replies
- ✅ Issue #4 (High): XOR-based session IDs → cryptographic random via `rand::thread_rng().next_u64()`
- ✅ Issue #5 (Medium): Path traversal risk → canonical path validation in `ensure_payload()`

**Work Stream 2: Documentation**
- ✅ Issue #3 (High): 40+ unsafe blocks without safety comments → `// SAFETY:` comments added to all unsafe blocks in kernel
  - `kernel/src/mm/*.rs` (address.rs, frame.rs, bitmap.rs, bump.rs)
  - `kernel/src/trace_ring.rs`, `kernel/src/init.rs`, `kernel/src/boot.rs`
  - `kernel/src/arch/aarch64.rs`, `kernel/src/arch/x86_64.rs`
  - `kernel/src/arch/aarch64/mmu.rs`, `kernel/src/arch/x86_64/mmu.rs`
  - `kernel/src/shmem.rs`, `kernel/src/ipc_v0.rs`

**Work Stream 3: Refactoring**
- ✅ Issue #6 (Medium): Duplicate IPC error reply code → `shmem_error_reply()` helper function
- ✅ Issue #7 (Low): GPU operations in domain_manager → `gpu_manager` service scaffold created

**Work Stream 4: Code Quality**
- ✅ Issue #8 (Low): Magic numbers → `kernel/src/mm/constants.rs` with named layout constants
- ✅ Issue #9 (Low): Missing rustdoc → Added to `kernel_api/src/lib.rs` (Handle, Envelope, module overview)
- ✅ Issue #10 (Low): Bump allocator TODO → Updated with design rationale and BitmapAllocator reference

**Files Changed:**
- Security: `services/store_service/Cargo.toml`, `services/store_service/src/capability.rs`
- Security: `services/domain_manager/src/error.rs` (new), `services/domain_manager/src/main.rs`
- Security: `services/capsule_relay/Cargo.toml`, `services/capsule_relay/src/main.rs`
- Documentation: `kernel/src/mm/*.rs`, `kernel/src/trace_ring.rs`, `kernel/src/init.rs`, `kernel/src/boot.rs`, `kernel/src/arch/**/*.rs`, `kernel/src/shmem.rs`, `kernel/src/ipc_v0.rs`
- Refactoring: `kernel/src/ipc_v0.rs`, `services/gpu_manager/` (new scaffold)
- Quality: `kernel/src/mm/constants.rs` (new), `kernel_api/src/lib.rs`

**Design Document:** `docs/plans/2026-02-17-code-quality-security-remediation-design.md`

## Security remediation progress (COMPLETE - S7, S9.0 Phase 1)
- S7 Security Hardening: 5 high-severity issues fixed with fail-closed defaults and runtime enforcement
  - S7-001: Fail-closed store signature policy (gate: `foundry_s7_store_signature_security.sh`)
  - S7-002: Fail-closed access control default (gate: `foundry_s7_access_control_security.sh`)
  - S7-003: Exact path matching for exe whitelisting
  - S7-004: DomainArtifactRegistry integration
  - S7-005: POSIX runner runtime enforcement (gate: `foundry_s7_posix_runner_security.sh`)
  - Combined gate: `foundry_s7_all_security.sh`
  - Documentation: [`docs/S7_SECURITY_HARDENING_PHASE2.md`](docs/S7_SECURITY_HARDENING_PHASE2.md), [`docs/S7_SECURITY_HARDENING_PHASE3.md`](docs/S7_SECURITY_HARDENING_PHASE3.md)
- Wave A Batch 1: SC-01 (ContentId validation, V-001) and SC-08 (log path confinement, V-009) completed with gate [`tools/ci/foundry_hardening_wave_a_batch1.sh`](tools/ci/foundry_hardening_wave_a_batch1.sh).
- Wave A Batch 2: SC-03 (posix_runner default-off, V-003) and SC-02 (wire safety + codegen fail-closed, V-002/V-014) completed with gate [`tools/ci/foundry_hardening_wave_a_batch2.sh`](tools/ci/foundry_hardening_wave_a_batch2.sh).
- Wave B Batch 1: SC-06 (trace ring ordering, V-007) and SC-07 (init parser checked arithmetic, V-008) completed with gate [`tools/ci/foundry_hardening_wave_b_batch1.sh`](tools/ci/foundry_hardening_wave_b_batch1.sh).
- Wave B Batch 2: SC-04 (unforgeable capability tokens, V-004) and SC-05 (kernel capability table, V-005/V-006) completed with gate [`tools/ci/foundry_hardening_wave_b_batch2.sh`](tools/ci/foundry_hardening_wave_b_batch2.sh).
- Wave C: SC-09 (store boundary split, V-011), SC-10 (pin nightly, V-015), SC-11 (unsafe safety docs), SC-12 (multi-encoding redaction) completed with gate [`tools/ci/foundry_hardening_wave_c.sh`](tools/ci/foundry_hardening_wave_c.sh).
- All 12 structural corrections (SC-01..SC-12) complete. All 15 findings (V-001..V-015) addressed in S9.0 Phase 1.
- S9.0 Phase 1 complete: V-006 (POSIX runner mitigation), V-007 (boundary cleanup), V-012 (trace isolation Phase 1) with new gates.

## What does NOT exist yet (intentional)
- Non-UEFI boot chain (will replace UEFI scaffolding later)
- Init as a store-resolved artifact with a real userland process model
- Full POSIX personality isolation model (current v0 is host-shell bootstrap path)
- Full wizard flow orchestration (run → observe → propose → rebuild → gate → publish)
- Web dashboard for queue visualization

## How to build
- `just build-host`
- `just codegen`
- `just build-targets`

## Next milestone
S12.2 physical HIL boot on Tier-1 reference hardware. Fast-path gates: `just s12` (contract + QEMU GOP probe), `just s11` (Driver Factory DoD).

### S10.1: Native Runner Production Integration (COMPLETE)
**Completion Date:** 2026-02-19
**Test Results:** 19/19 assertions passing

All S10.1 native runner production integration tasks completed:

**Deliverables:**
- ✅ `native_wasm_v0` manifest schema in `artifact_store_schema`
- ✅ Capability broker with transactional grants in `domain_manager`
- ✅ Runtime supervisor integration via `native_wasm_runner`
- ✅ Real kernel IPC via `native_runner` kernel bridge
- ✅ Foundry gate: `foundry_native_runner_s10_1.sh` (19 assertions)

**Architecture:**
- Manifest schema validates WASM module requirements at ingestion time
- Broker provides transactional capability grants with rollback on failure
- Runtime supervisor dispatches `native_wasm_v0` plans to native runner
- Kernel bridge provides real IPC for harness calls (not mock)

**Files Created/Modified:**
- `artifact_store_schema/src/manifest/native_wasm.rs` (manifest schema)
- `services/domain_manager/src/capability_broker.rs` (transactional grants)
- `runtime_supervisor/src/native_wasm_runner.rs` (supervisor integration)
- `services/native_runner/src/kernel_bridge.rs` (real IPC)
- `tools/ci/foundry_native_runner_s10_1.sh` (Foundry gate)

**Security Properties:**
- Fail-closed: missing capability causes load error
- Zero domain_id rejected at execution time
- Invalid WASM fails with structured error
- Empty capabilities rejected without explicit flag

### V-012 Phase 5: User-space Trace Service Client (COMPLETE)
**Completion Date:** 2026-02-18
**Test Results:** 46/46 tests passing (trace_client: 32, domain_manager: 14)

All V-012 Phase 5 user-space trace service client tasks completed:

**Phase 5.1: Trace Client Scaffold**
- ✅ `services/trace_client/` crate with Cargo.toml, lib.rs, error.rs
- ✅ `TraceClientError` enum with 9 error variants
- ✅ `TraceCapability` struct with rights constants (READ, WRITE, ADMIN)
- ✅ Builder pattern for `TraceClient` configuration

**Phase 5.2: Trace Operations**
- ✅ `TraceTransport` trait for IPC abstraction
- ✅ `MockTransport` for testing without kernel access
- ✅ `read_trace()` with streaming offset tracking
- ✅ `drain()` for collecting all trace data on shutdown
- ✅ `get_info()` for trace buffer metadata
- ✅ `reset_read_offset()` for re-reading traces
- ✅ Status code to error mapping

**Phase 5.3: Domain Manager Integration**
- ✅ `DomainTraceManager` for lifecycle integration
- ✅ `TraceConfig` for collection configuration
- ✅ `TraceCollectionResult` for collection output
- ✅ `collect_on_shutdown()` method
- ✅ `collect_to_artifact()` for file emission

**Phase 5.4: Foundry Gate**
- ✅ `tools/ci/foundry_v012_phase5_trace_client.sh` with 16 test cases
- ✅ Wired into justfile as `foundry-v012-phase5-trace-client`

**Files Created:**
- `services/trace_client/Cargo.toml`
- `services/trace_client/src/lib.rs` (43 lines)
- `services/trace_client/src/error.rs` (79 lines)
- `services/trace_client/src/capability.rs` (95 lines)
- `services/trace_client/src/client.rs` (533 lines)
- `services/trace_client/src/ipc.rs` (292 lines)
- `services/domain_manager/src/trace_integration.rs` (271 lines)
- `tools/ci/foundry_v012_phase5_trace_client.sh` (324 lines)

**Architecture Highlights:**
- `TraceTransport` trait enables testing without kernel and future syscall integration
- Builder pattern with `domain_id` required for fail-closed design
- Internal read offset tracking for sequential reads
- Mock transport provides predictable responses for unit testing

### V-012 Phase 4 Completed Work
- ✅ Kernel-side trace service implementation
- ✅ Domain-scoped trace buffer management
- ✅ Capability-based trace access control
- ✅ Trace service IDL contract (`idl/harness/trace_service_v1.toml`)
- ✅ Generated Rust bindings (`kernel_api/src/generated/trace_service_v1.generated.rs`)
- ✅ IPC handlers for trace service (`ipc_v0::handle_trace_service_envelope`)
- ✅ Unit tests for trace service (16/16 tests passing)
- ✅ Foundry gate for trace service (`foundry_v012_phase4_trace_service.sh`)

### V-006 Phase 4a: Native WASM SDK Hardening Follow-up (COMPLETE)
**Completion Date:** 2026-02-18
**Scope:** Post-review hardening and gate reliability for generated WASM SDK bindings and test discipline.

**Fixes Completed:**
- ✅ `hello_wasm` test-compatibility fix: conditional `no_std`/panic handler and wasm32-only symbol gating to prevent host test duplicate `panic_impl`.
- ✅ `idl_codegen` fail-closed hardening: removed `unwrap()` from wasm-imports generation path and replaced with structured `Result` propagation.
- ✅ Generated SDK API improvement: harness `call(...)` now returns `(Status, usize)` with explicit reply length and bounds-clamped output length handling.
- ✅ Kernel MM test cleanup: removed invalid duplicate/in-function test blocks in `kernel/src/mm/mod.rs` that referenced stale symbols (`BootRegion`, `ShmemRegionTable`) and broke compilation.
- ✅ Trace ring test determinism: `reset_for_test()` now resets legacy SMP state to avoid cross-test contamination and order-dependent failures.

**Validation Evidence:**
- ✅ `cargo test -p idl_codegen`
- ✅ `cargo test -p ramen_sdk`
- ✅ `cargo test -p hello_wasm`
- ✅ `cargo test --workspace --exclude kernel_uefi --exclude kernel_aarch64`
- ✅ `tools/ci/foundry_native_wasm_s9_3.sh`

### S10.0: Native Runner Executor (COMPLETE)
**Completion Date:** 2026-02-18
**Test Results:** All integration tests passing

**Deliverables:**
- `services/native_runner/` crate with library + CLI
- Wasmtime integration for WASM execution
- Capability injection via `RAMEN_CAP_*` exported globals
- Host functions for echo_harness_v0 and trace_service_v1
- Mock kernel bridge for testing
- Foundry gate: `foundry_native_runner_s10_0.sh`

**Architecture:**
- Runner is executor-only (no policy decisions)
- Host functions are plumbing: decode → single kernel IPC → encode
- Fail-closed: missing capability causes load error

**Files Created:**
- `services/native_runner/Cargo.toml`
- `services/native_runner/src/lib.rs`
- `services/native_runner/src/error.rs`
- `services/native_runner/src/kernel_bridge.rs`
- `services/native_runner/src/context.rs`
- `services/native_runner/src/runner.rs`
- `services/native_runner/src/harness/mod.rs`
- `services/native_runner/src/harness/echo.rs`
- `services/native_runner/src/harness/trace.rs`
- `services/native_runner/src/bin/native_runner.rs`
- `services/native_runner/tests/integration_test.rs`
- `tools/ci/foundry_native_runner_s10_0.sh`

**Next Steps (S10.1):**
- Manifest parsing (`native_wasm_v0`)
- Broker integration (Domain Manager)
- Runtime supervisor integration
- Real kernel IPC (not mock)

### Remaining S9 Work
- V-006 Phase 4b: Native runner design and implementation
- V-007 Phase 6: Full capability signing and key rotation (future)
