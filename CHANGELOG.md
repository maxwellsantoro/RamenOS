# CHANGELOG

## [Unreleased]

### Added
- G0 RamenOrg / research-backed OS scaffold:
  - Added `docs/plans/2026-06-23-research-backed-ramenorg.md` to make RamenOrg and research-backed development first-class planning tracks without displacing S12.4/S13 HIL work.
  - Added `docs/org/` with RamenOrg constitution, role charter, authority levels, heartbeats, `WorkOrderV0`, `HandoffPacketV0`, `BoardVoteV0`, and claim safety.
  - Added `docs/research/` with the research program, RQ-0001 offer-shaped service boundaries, and RQ-0002 AI-governed Org Kernel.
  - Added `tools/org/status_drift.py` and `tools/ci/foundry_org_governance_g0.sh` to catch active-track drift across `AGENTS.md`, `CURRENT_STATUS.md`, `NEXT_TASKS.md`, and `ROADMAP.md`.
  - Added `just foundry-org-governance-g0` / `just org-g0`; wired the governance gate into `foundry_ci_extended.sh`.
- G0.1 board packet and packet validators:
  - Added `docs/plans/2026-06-23-g0-1-board-packet-validators.md`.
  - Added JSON schemas under `schemas/org/` for `WorkOrderV0`, `HandoffPacketV0`, `BoardVoteV0`, and `BoardPacketV0`.
  - Added `tools/org/render_board_packet.py` to emit S12.4.1 example packets under `out/org/examples/` and `out/org/current_board_packet.json`.
  - Added `tools/org/validate_packets.py` to validate required fields, role separation, evidence refs, gate refs, context refs, packet refs, HIL evidence constraints, and A0/A1 authority boundaries.
  - Extended `foundry_org_governance_g0.sh` to render and validate packet examples.
- G0.2 active task + cross-packet consistency:
  - Added `docs/plans/2026-06-23-g0-2-active-task-cross-packet.md` and `docs/org/current_task.yaml`.
  - Updated `render_board_packet.py` to render packets from the current-task file instead of hardcoded S12.4.1 constants.
  - Updated `validate_packets.py` to fail closed on unknown gate refs and enforce cross-packet agreement for repo SHA, work-order/proposal id, task, required gates, and authority level.
  - Updated `BoardVoteV0` to use typed evidence buckets: design, gate, claim, HIL, and release evidence refs.
- G0.3 CurrentTaskV0 schema and negative validator cases:
  - Added `schemas/org/current_task_v0.schema.json`, `docs/org/CURRENT_TASK_V0.md`, and `docs/plans/2026-06-23-g0-3-current-task-negative-fixtures.md`.
  - Governance gate now validates `docs/org/current_task.yaml` before rendering packets.
  - Added `BoardVoteV0.repo_sha` and cross-validation that vote SHA matches board/work-order/handoff SHA.
  - Board packet refs are exactly-one for work order, handoff, and vote in the scaffold phase.
  - Added `tools/org/test_validate_packets.py` with known-bad cases for mismatched SHA, missing evidence, unknown gate syntax, A2 authority, stale HIL claims, wrong handoff work-order id, vote proposal mismatch, vote SHA mismatch, too many refs, and malformed current-task source.
- G0.3.1 governance label and claim-boundary hygiene:
  - Added `docs/plans/2026-06-23-g0-3-1-governance-label-claim-boundary.md`.
  - Updated current-task labels, renderer vote claim text, and validator diagnostics from stale G0.1/G0.2 wording to G0.3.1.
  - Validator now requires claim boundaries to explicitly include no merge, no release, no HIL actuation, and no public support authority.
  - Added negative validator cases for missing release/public-support denial and PASS/METAL without HIL evidence refs.
- G0.4 read-only steward heartbeat / board brief:
  - Added `docs/plans/2026-06-23-g0-4-read-only-steward-heartbeat.md` and `docs/org/BOARD_BRIEF_V0.md`.
  - Added `tools/org/render_board_brief.py` to emit `out/org/current_board_brief.md` from validated packets.
  - Governance gate now renders the board brief only after packet validation passes and checks required brief sections.
- G0.5 agent intake bundle and freshness binding:
  - Added `docs/plans/2026-06-23-g0-5-agent-intake-freshness-binding.md`, `docs/org/INTAKE_BUNDLE_V0.md`, and `schemas/org/intake_manifest_v0.schema.json`.
  - Packet validation reports now bind every checked packet and the current-task source by SHA-256.
  - Board brief rendering rejects stale validation reports and emits `out/org/intake_manifest.json` with paths and hashes for the complete intake bundle.
  - Added independent manifest validation and a negative test proving changed packets cannot reuse an earlier passing report.
- G0.6 intake-only agent trial:
  - Ran a fresh agent with no inherited thread context using only the board brief, intake manifest, and four referenced packets.
  - Recorded `PASS/PLAN`: task identity, packet refs, authority denials, gates, and bounded plan were recovered without hidden chat context or external file reads.
  - Recorded the patch-readiness limit as a finding: referenced context and scoped source contents are not carried by the six-file intake.
  - Added the G0.6 plan and captured trial evidence under `docs/org/trials/`.
- G0.7 bounded context grant:
  - Added `ContextGrantV0`, `context_grant_refs`, `authorized_new_paths`, grant rendering/validation, and negative grant fixtures.
  - Extended the board brief and intake manifest with Granted Context, Not Granted / Out of Scope, hash-bound granted files, and authorized new output paths.
  - Recorded a fresh-agent `PASS/PATCH-PLAN` trial for S12.4.1 with sufficient bounded context, no expansion request, no hidden chat, no external reads, and no implementation.
- G0.8 bounded implementation trial:
  - Added `tools/hil/appliance_capture_serial.sh` for the S12.4.1 HIL appliance serial observer scaffold.
  - Extended `foundry_hil_appliance_s12_4.sh` with a serial-observer contract fixture and a stale-log graduation negative case.
  - Moved the serial observer from `authorized_new_paths` into hash-bound granted context once the file exists.
  - Recorded a `PASS/PATCH` implementation trial; G0.8.1 reclassifies this as A2-local rather than A1 proposal authority.
- G0.8.1 implementation authority and serial observer claim hygiene:
  - Reclassified code-writing trials as A2-local while still denying merge, release, self-approval, HIL actuation, and public support authority.
  - Hardened the serial observer with run-id allowlist validation, empty-transcript rejection, and machine-readable `serial_input_kind`.
  - Split development replay evidence from live appliance evidence: `PASS/HIL-LOG` for `RAMEN_HIL_SERIAL_LOG`, `PASS/HIL-APPLIANCE` for `RAMEN_HIL_SERIAL_DEV`.
  - Added HIL gate negatives for unsafe run ids and empty transcripts.
- Offer-boundary doctrine intake:
  - Recorded `Lang` versus `ObsContract` as an explicit future service-boundary design requirement.
  - Added claim-safety guardrails so topology hiding and request minimization are not advertised as hidden-affordance noninterference without measured evidence.
- Active-track doc sync:
  - Updated `AGENTS.md`, `CURRENT_STATUS.md`, `NEXT_TASKS.md`, `ROADMAP.md`, `docs/INDEX.md`, and `DECISIONS.md` to reflect S12.4 HIL appliance physical loop as the immediate OS execution focus and G0/RQ work as a parallel project-control track.
- HIL evidence discipline:
  - Added `EVIDENCE_LEVELS.md` (PASS/QEMU, PASS/HIL-LOG, PASS/HIL-LIVE, PASS/METAL).
  - Added `hil_evidence:` serial provenance markers (`git_sha`, `init_profile`, `machine_id`, manifest/EFI/init hashes, `boot_epoch_nonce`).
  - Added `RAMEN_HIL_GRADUATION=1` mode (live serial only); evidence JSON under `out/evidence/`.
  - Added `tools/hil/hil_gate_common.sh`, `set_ramenos_boot_nonce.sh`, `kernel_uefi/build.rs` two-pass hash embed.
- S13.8 atomic update/rollback gate scaffold:
  - Added `kernel_uefi/src/ab_slot_probe.rs` — reads `RamenAbSlot` UEFI vendor variable (A/B slot + rollback_ready).
  - Added `kernel::boot::AtomicUpdateProbeInfo` storage and `OP_ATOMIC_UPDATE` init profile (`atomic_update`).
  - Added `tools/hil/{build_atomic_update_image,stage_ab_slot_rollback,set_ramenos_ab_slot}.sh`.
  - Added `tools/ci/foundry_s13_atomic_update_s13_8.sh` — QEMU negative smoke + opt-in HIL serial validation; extended `just s13-hil`.
  - Pinned A/B metadata in `hardware/storage_contract_v0.toml` `[metal.ab_slot]`.
- S13.7 metal NVMe boot gate scaffold:
  - Added `kernel_uefi/src/nvme_boot_probe.rs` — loaded-image device path walk for NVMe namespace nodes.
  - Added `kernel::boot::NvmeBootProbeInfo` storage and `OP_NVME_BOOT` init profile (`nvme_boot`) with `persistent_storage: nvme_boot ok` marker.
  - Added `tools/hil/build_nvme_boot_image.sh` — NVMe ESP boot tree builder.
  - Added `tools/ci/foundry_s13_nvme_boot_s13_7.sh` — QEMU negative smoke (`not_nvme` on FAT boot) + opt-in HIL serial validation; `just s13-hil`.
  - Review hygiene: harness `dead_code` tranche5 fix (`cfg_attr` on `block_harness`/`net_harness`); `identity_op` allow on UART THR offset.
- S13.4–S13.5 block sector Oracle capture + distillation:
  - Added `block_sector_trace_v0` schema (`artifact_store_schema::block_sector_trace`).
  - Added `kernel_api::mock::block_harness::{MockBlockShmem, MockBlockHarness}`.
  - Added `driver_foundry::virtio_blk_sector` and sector trace import/replay CLI.
  - Added `capture_virtio_blk_sector_oracle.sh`, `promote_virtio_blk_sector_capture.sh`, live `oracle_block_trace.json`.
  - Added `foundry_s13_block_sector_oracle_s13_4.sh`; extended `foundry_s13_replay.sh` and `just s13`.
- S13.6 runtime `harness.block` sector I/O in QEMU:
  - Added `kernel_api::block_oracle_vector` with baked sector read/write vectors and init trace SHA-256 prefix.
  - Added `kernel/src/block_harness.rs` BLOCK_V1 IPC provider; `OP_BLOCK_IO` init profile (`block_io`).
  - Added `tools/ci/foundry_s13_runtime_block_s13_6.sh`; extended `just s13` and `foundry_ci_extended.sh`.
- S13.3 block replay scoreboard:
  - Added `tools/ci/foundry_s13_replay.sh` — live `oracle_init_trace.json` replay through `MockPciDevice` + `virtio_blk_init`.
- S13.2 virtio-blk Oracle capture:
  - Added `tools/trace/virtio_blk_oracle_capture.c`, `capture_virtio_blk_oracle.sh`, `promote_virtio_blk_capture.sh`.
  - Added `driver_foundry::virtio_blk_init` distilled init driver and CLI `replay-blk-init-trace`.
  - Live `drivers/reference_vaults/virtio-blk/traces/oracle_init_trace.json` (15 events, `sha256:eb816f36…`).
  - Added `tools/ci/foundry_s13_virtio_blk_oracle_s13_2.sh`; extended `just s13` and `foundry_ci_extended.sh`.
- S13.0 persistent storage contract scaffold:
  - Added `docs/plans/2026-06-21-s13-persistent-storage-design.md` (virtio-blk Oracle + NVMe metal graduation path).
  - Added `hardware/storage_contract_v0.toml` storage manifest.
  - Added `idl/harness/block_v1.toml` (`harness.block` read/write with shmem data plane) + codegen.
  - Added `drivers/reference_vaults/virtio-blk/` Reference Vault scaffold.
  - Added `tools/ci/foundry_s13_persistent_storage_s13_0.sh`; `just s13` fast-path; wired into `foundry_ci_extended.sh`.
- S12.3 IOMMU inventory marker:
  - Added `kernel_uefi/src/iommu_probe.rs` — ACPI XSDT/RSDT walk for DMAR table presence.
  - Added `kernel::boot::IommuProbeInfo` storage and `OP_IOMMU_INVENTORY` init profile with `golden_machine: iommu_present=1` serial marker.
  - Added `tools/ci/foundry_s12_iommu_inventory_s12_3.sh` — opt-in HIL gate; extended `just s12-hil` and S12.0 HIL delegation.
- S12.2 physical HIL boot gate scaffold:
  - Added `OP_HIL_BOOT` init profile (`hil_boot`) emitting `golden_machine: hil_boot ok` after GOP probe success.
  - Added `tools/hil/build_usb_boot_image.sh` — builds EFI/BOOT USB boot tree (`BOOTX64.EFI` + `init.img`).
  - Added `tools/ci/foundry_s12_hil_boot_s12_2.sh` — opt-in HIL gate (`RAMEN_HIL_GOLDEN_MACHINE=1`); serial capture via `RAMEN_HIL_SERIAL_DEV` or log replay via `RAMEN_HIL_SERIAL_LOG`.
  - Added `just s12-hil` opt-in alias; S12.0 delegates to S12.2 when HIL env is set.
- S12.1 UEFI GOP probe (QEMU OVMF stepping stone):
  - Added `kernel_uefi/src/gop_probe.rs` — GOP mode query + deterministic 64×64 `VideoFill`.
  - Added `kernel::boot::GopProbeInfo` storage and `OP_GOP_PROBE` init profile with `golden_machine:` serial markers.
  - Added `tools/ci/foundry_s12_gop_probe_s12_1.sh`; extended `just s12` and `foundry_ci_extended.sh`.
- S12.0 golden machine contract scaffold:
  - Added `docs/plans/2026-06-21-s12-golden-machine-design.md` (Tier-1 Intel NUC 12/13 reference, IOMMU + GOP + HIL policy).
  - Added `hardware/golden_machine_v0.toml` machine manifest.
  - Added `tools/ci/foundry_s12_golden_machine_s12_0.sh` smoke gate (inventory + negative assertions; RED markers for S12.1+ work).
  - Wired into `foundry_ci_extended.sh`; `just s12` fast-path alias.
- S11.8 runtime `harness.net` packet I/O in QEMU (S11 COMPLETE):
  - Added `kernel_api::net_packet_oracle_vector` with baked live Oracle ARP send/receive payloads and trace SHA-256 prefix.
  - Added `kernel/src/net_harness.rs` NET_V1 IPC provider with shmem-backed send/receive validation.
  - Added `OP_NET_PACKET_IO` init profile (`net_packet_io`) exercising typed `harness.net` control messages in QEMU.
  - Added `tools/ci/foundry_s11_runtime_net_s11_8.sh`; wired into `foundry_ci_extended.sh` and `just s11`.
  - S11 Definition of Done: init replay + packet replay + live Oracle provenance + runtime harness I/O (`just s11`).
- S11.7 live hardware packet RX capture:
  - Added `tools/trace/fetch_virtio_net_modules.sh` to bundle Ubuntu mainline `failover.ko`, `net_failover.ko`, and `virtio_net.ko` into the packet-capture initrd.
  - `virtio_net_packet_oracle_capture.c` primary path: load kernel modules, bring up `eth0`, capture live ARP send/receive via AF_PACKET; userspace virtqueue path is fallback only; slirp-derived receive fallback removed.
  - `driver_foundry::assert_hardware_packet_rx` + CLI `assert-hardware-packet-trace`; rejects `slirp-arp-reply-derived` notes.
  - `REQUIRE_LIVE_ORACLE_TRACE=1 foundry_s11_reference_vault_s11_3.sh` now runs hardware RX assertion on `oracle_packet_trace.json`.
  - Promoted live packet fixture (`trace_id=sha256:482af3005a3520aa7b98e9e80286875eb3abd88a3e8fb7b9e95ec54bdbcb953d`); capture ~3s.
- S11.6 live virtio-net packet Oracle capture:
  - Added `tools/trace/virtio_net_packet_oracle_capture.c`, `capture_virtio_net_packet_oracle.sh`, and `promote_virtio_net_packet_capture.sh` for harness-level JSONL capture and vault promotion.
  - Added `driver_foundry import-packet-jsonl` and `assert-live-packet-trace` with `sha256:` provenance checks for live packet fixtures.
  - Promoted live `drivers/reference_vaults/virtio-net/traces/oracle_packet_trace.json`; `REQUIRE_LIVE_ORACLE_TRACE=1` now asserts both init and packet trace provenance.
  - `driver_foundry::virtio_net_packet` replays send/receive directly from trace events (no hardcoded scaffold payloads).
- S11.5 virtio-net packet I/O distillation:
  - Added `artifact_store_schema::net_packet_trace::NetPacketTraceV0` for harness-level send/receive Oracle events with shared-memory caps, offsets, lengths, and payload hex.
  - Added `kernel_api::mock::packet_harness::{MockPacketShmem, MockPacketHarness, PacketReplayScoreboard}` for deterministic `harness.net` replay scoring.
  - `driver_foundry::virtio_net_packet` distills ARP probe send and reply receive against `drivers/reference_vaults/virtio-net/traces/oracle_packet_trace.json`.
  - Extended `foundry_s11_replay.sh` and `foundry_s11_reference_vault_s11_3.sh` with packet trace schema validation and replay.
- S11.4 virtio-net init driver distillation:
  - Extended `virtio_net_oracle_capture.c` to record feature negotiation, MAC reads, RX/TX queue setup, and `DRIVER_OK` (20 live events).
  - `driver_foundry::virtio_net_init` replays the expanded vault trace, including conditional `FEATURES_OK` when the device offers feature bits.
  - `foundry_s11_replay.sh` asserts `replay_vault_init_trace`.
- S11.3 live virtio-net Oracle capture:
  - `tools/trace/capture_virtio_net_oracle.sh` boots QEMU `virtio-net-pci`, captures JSONL from `virtio_net_oracle_capture.c`, and promotes the vault fixture through `promote_virtio_net_capture.sh`.
  - `tools/trace/build_oracle_capture_initrd.sh` cross-compiles the capture init via `zig cc` on macOS.
  - `foundry_ci_extended.sh` now runs S11.3 with `REQUIRE_LIVE_ORACLE_TRACE=1`.
- S11.4 virtio-net init driver scaffold:
  - `driver_foundry::virtio_net_init` replays PCI discovery and device-status ACK against `MockPciDevice`.
  - `foundry_s11_replay.sh` runs the init-driver unit tests.

### Fixed
- HIL appliance scaffold gate now validates `hardware/hil_appliance_v0.toml` without requiring Python 3.11 `tomllib` or an external `tomli` package, keeping the gate runnable on the local Python 3.9 host.
- `artifact_store_schema::net_packet_trace` uses `is_multiple_of(2)` for payload hex length check (clippy).
- `tools/compat/fetch_compat_kernel.sh` extracts `.deb` packages without `dpkg-deb` using `ar` + `tar`.
- `kernel/src/trace_cap.rs` now returns typed `TraceCapError` instead of `Result<(), ()>`.
- S11/S10 gate hygiene:
  - Wired `tools/ci/foundry_s11_driver_factory_s11_0.sh` into `foundry_ci_extended.sh`.
  - Wired green `tools/ci/foundry_s11_replay.sh` into `foundry_ci_extended.sh` after the replay scoreboard landed.
  - Added strict lint tranche 6 to `tools/ci/foundry_preflight.sh` for local/CI parity.
  - Hardened the S10.5.2 semantic IPC relay scanner to reject plausible oversize frame lengths instead of only the known `5000` test value.
  - Kept the S10.5.2 semantic IPC relay alive after partial/bad frames so host retries are meaningful instead of timing out against a stopped target loop.
  - Added a positive QEMU boot retry to `foundry_qemu_ipc_bridge_s10_5_2.sh` for host chardev startup cases where the first QEMU instance never receives COM2 bytes.
  - Made the S10.5.2 negative oversize gate retry the host send/reject wait and fail explicitly if the client cannot connect.
  - Updated status docs through S11.2 completion and corrected the S8 Phase 4 assertion count to 40/40.
- Review follow-up gate hygiene:
  - Updated Store S0/S1/POSIX launch-plan gates to assert the canonical `store: emitted execution launch plan:` sentinel.
  - Updated the S9.0 boundary cleanup gate to validate current store-service IPC and execution-fabric schema wiring instead of stale temporary-dependency strings.
  - Made `idl_codegen` normalize source comments for absolute input paths and rustfmt generated Rust output before writing.
  - Isolated `store_service` env-flag tests from shared `TEST_FLAG` state for parallel workspace test reliability.
  - Restored the S7 GPU launch-plan fixture by using a non-zero expected display capability token.

### Added
- S11.2 Driver Factory replay scoreboard:
  - Added `kernel_api::mock::pci_device::{PciReplayEvent, ReplayScoreboard, MockPciDevice}` as no-alloc fixed-slice replay primitives for Oracle PCI/MMIO traces.
  - `ReplayScoreboard` reports deterministic mismatch reasons for op, BAR, offset, width, value, incomplete traces, and extra accesses.
  - `MockPciDevice` replays Oracle read values and scores writes against the expected trace.
  - `tools/ci/foundry_s11_replay.sh` now passes and runs focused `kernel_api` replay tests.
- S11.3 virtio-net Reference Vault scaffold:
  - Added wire-safe `idl/harness/net_v1.toml` and generated `kernel_api` binding for shared-memory packet send/receive control messages.
  - Added `drivers/reference_vaults/virtio-net/` with harness context, notes, and schema-valid `traces/oracle_init_trace.json`.
  - Added pinned OASIS VIRTIO v1.3 source notes under `datasheets/` for PCI discovery, capabilities, initialization status, queue layout, network feature bits, and config fields.
  - Added `tools/ci/foundry_s11_reference_vault_s11_3.sh` and wired it into `foundry_ci_extended.sh`.
  - Added schema and datasheet inventory assertions for the virtio-net vault fixture.
- S11.3 trace fixture translation:
  - Added `driver_foundry` host crate to translate `DriverProtocolTraceV0` PCI/MMIO events into `kernel_api::mock::pci_device::PciReplayEvent`.
  - `driver_foundry replay-trace` parses, schema-validates, translates, and replays a trace fixture through `MockPciDevice`; `import-jsonl` converts `pci_mmio_tracer` debugfs JSONL into `DriverProtocolTraceV0` and stamps missing trace IDs from the source JSONL SHA-256 digest.
  - `driver_foundry assert-live-trace` rejects scaffold fixtures and requires full `sha256:` trace IDs, timestamped events, and contiguous `seq=1..N` events for live Oracle capture promotion.
  - Added `tools/trace/promote_virtio_net_capture.sh` to import live JSONL, replay it, assert live provenance, and replace the virtio-net vault fixture only after validation.
  - Extended `foundry_s11_replay.sh` and `foundry_s11_reference_vault_s11_3.sh` to replay the virtio-net vault fixture.
  - Added `driver_foundry` to strict lint tranche 6 and added a live-capture promotion dry-run to the S11.3 vault gate.
- S11.2-pre IDL/Wire Contract Integrity Gate:
  - `tools/ci/foundry_idl_lint.sh` / `just idl-lint` now fail on missing/duplicate protocol IDs, missing/duplicate `msg_type`s, dynamic `string`/`bytes` envelope fields, and generated Rust IPC host references.
  - `idl_codegen` now requires non-zero `protocol` and explicit non-zero `msg_type`, emits generated protocol/message constants, and rejects dynamic direct Rust IPC fields.
  - Local preflight now runs `foundry_idl_lint.sh` immediately after codegen.
  - Canonicalized active IDL protocols and message IDs across harnesses, portals, and services.
  - Replaced direct-envelope `string`/dynamic `bytes` fields in execution fabric, store service, semantic store, and shmem write contracts with `bytes32`, tokens, shmem caps, offsets, and lengths.
  - Packed semantic harness handles as `kind | (domain_id << 16) | export_id` with collision and out-of-range regression tests.
  - Restricted semantic snapshot viewers no longer receive `compute_fabric` state; added a leak regression covering nodes, leases, executions, and duplicate groups.
- S11.1 Driver Factory Oracle capture scaffold:
  - `tools/trace/pci_mmio_tracer.c` Linux guest module scaffold with debugfs JSONL event stream and explicit `ramen_pci_mmio_trace_record` helper.
  - `artifact_store_schema::driver_protocol_trace` with `DriverProtocolTraceV0` schema + validation tests for virtio-net Oracle traces.
  - `capsule_relay --list-trace-kinds` advertises `driver_protocol_trace_v0`.
  - `tools/ci/foundry_s11_driver_factory_s11_0.sh` now passes for S11.0/S11.1 inventory + capture scaffold.
  - `tools/ci/foundry_s11_replay.sh` added as the initial red S11.2 replay-scoreboard gate.
- S10.2 v1.1 capability-filtered snapshots + domain_manager reactor publish:
  - `docs/plans/2026-06-17-s10-2-v1-1-cap-filtered-snapshots.md`
  - `filter_platform_snapshot_for_viewer` in `artifact_store_schema::semantic_state`
  - `SemanticReactor::publish_domain_inventory_changed` with per-subscriber `shm_cap` tokens
  - `domain_manager` inventory publish on start/stop; `KernelOpsBackend::from_env()` for `RAMEN_SEMANTIC_HARNESS_BRIDGE=1`
  - Extended `foundry_semantic_state_s10_2.sh`; S10.5.0/10.5.1/10.5.2 added to `foundry_ci_extended.sh`
- S10.5.2 QEMU IPC bridge implementation:
  - `docs/plans/2026-06-17-s10-5-2-qemu-ipc-bridge.md` — COM2 unix chardev framing, sliding-window sync, `get_snapshot` pull only.
  - `kernel_api::ipc_frame`, `kernel` COM2 UART + `OP_SEMANTIC_IPC_RELAY`, `tools/ci/ipc_bridge_client.py`, `chardev_serial` transport.
  - `tools/ci/foundry_qemu_ipc_bridge_s10_5_2.sh` — PASS (`snapshot_sha256_prefix=9c0de4419f03f426`); added to `foundry_ci_extended.sh`.
  - `just foundry-qemu-ipc-bridge-s10-5-2` recipe.
  - `ChardevKernelBridge` + `runtime_supervisor` `kernel_ipc_transport` / `RAMEN_KERNEL_IPC_TRANSPORT` wiring.
- S10.5.1 broker/kernel bridge implementation:
  - `docs/plans/2026-06-17-s10-5-1-broker-kernel-bridge.md` — `SemanticHarnessGrantOps`, `KernelHarnessProxy`, `get_domain_grant_handles` IPC, deterministic sha256 contract.
  - `tools/ci/foundry_broker_kernel_bridge_s10_5_1.sh` — PASS: broker allowlist, proxy roundtrip, supervisor E2E.
  - `services/kernel_harness_proxy` — host-only envelope proxy with cap-validated semantic `get_snapshot` and in-process shmem map.
  - `runtime_supervisor` native WASM bridge wiring for DomainManager grant fetch + proxy-backed runner execution.
  - `kernel_api::semantic_snapshot_vector` shared S10.5 snapshot bytes (`snapshot_sha256_prefix=9c0de4419f03f426`).
  - `just foundry-broker-kernel-bridge-s10-5-1` recipe.
- S10.5.0 QEMU validation: `foundry_host_target_s10_5.sh` PASS (`snapshot_sha256_prefix=9c0de4419f03f426`).
- S10.5 host→target init bridge:
  - Expanded `docs/plans/2026-06-17-s10-5-host-to-target-integration.md` — Option A chosen, inventory table, init-bridge S10.5.0 plan, PASS markers, scope guard.
  - New `tools/ci/foundry_host_target_s10_5.sh` — Phase 0 inventory PASS; Phase 1 QEMU semantic snapshot validation.
  - Added `semantic_snapshot` init profile and `OP_SEMANTIC_SNAPSHOT` typed init handler.
  - Handler builds a semantic-state `GetSnapshot` request/reply path, allocates shmem, writes deterministic snapshot bytes, and emits `semantic_state: get_snapshot ok` plus a real SHA-256 prefix.
  - `just foundry-host-target-s10-5` recipe (not in CI extended until QEMU validation is green).
- S10.2.1 subscribe reactor:
  - `docs/plans/2026-06-17-s10-2-1-subscribe-reactor.md` — reactor ownership, event mask, delivery flow.
  - `SemanticReactor` in `services/semantic_state`: `handle_subscribe`, `publish_domain_state_changed`, `reactor_tick()` typed `state_changed_event` envelopes.
  - Replaced `subscribe_stub` gate step with `subscribe_delivery` in `foundry_semantic_state_s10_2.sh`.
- S10.4.1 execution fabric wiring:
  - `store_cli emit-plan` emits canonical `ExecutionLaunchPlanV0` with `ExecutionRunnerConfigPayloadV0` in `runner_config.config_json`.
  - `runtime_supervisor` `fabric_policy::consult_always_local` records simulation lease/trace; dispatch stays local.
  - Canonical plan parsing extracts compat/GPU/native wasm config; legacy launch plans still supported.
  - Extended `foundry_execution_fabric_s10_4.sh` with `emit_plan_canonical_roundtrip`.
- S10.3.4 CoW projection writes:
  - `docs/plans/2026-06-17-s10-3-4-cow-projection-writes.md` — typed `commit_projection_write`; 9p stays read-only.
  - `projection_cow::commit_projection_write` ingests replacement bytes as a fresh CAS blob/manifest and repoints the virtual path.
  - Preserves the prior CAS blob/manifest and leaves the S10.3.3 `readonly=on` 9p export unchanged.
  - Extended `foundry_projection_storage_s10_3.sh` with `projection_cow_commit_repoints_path_preserves_prior_blob`.
- S10.3.3 read-only VFS projection:
  - Design: `docs/plans/2026-06-17-s10-3-3-read-only-vfs-projection.md` (virtio-9p via QEMU `-virtfs`; virtio-fs deferred).
  - `store_service::projection_vfs::materialize_read_only` — symlink tree from projection index to CAS blobs.
  - `compat_runner` optional `projection_vfs` mount (`mount_tag=ramen_store`, `readonly=on`).
  - Extended `foundry_projection_storage_s10_3.sh` with `read_only_vfs_projection`.
- S10.3.2 index-on-ingest:
  - `IngestArtifact` now updates the durable projection index after successful blob/manifest writes.
  - Added deterministic path aliases under `/store/{kind}/{channel}/{source_filename}` with sanitized path segments.
  - Added kind/channel tags for `query_by_tag`.
  - Extended `foundry_projection_storage_s10_3.sh` with `ingest_updates_projection_index`.
- S10.3.1 durable projection index writer:
  - `ProjectionIndexStore::load_or_empty`, `upsert_entry`, `upsert_path_projection`, `persist_atomic`.
  - Atomic `{store_root}/projection_index.json` working copy plus CAS snapshot (`projection_index_v0` manifest/blob).
  - Read-only override when `RAMEN_STORE_PROJECTION_INDEX` is outside `store_root`.
  - Extended `foundry_projection_storage_s10_3.sh` with durable roundtrip and corrupt-index fail-closed tests.
- S10.3.1 projection index backend design:
  - Added `docs/plans/2026-06-17-s10-3-1-projection-index-backend.md`.
  - Chose CAS-backed `ProjectionIndexV0` artifacts with an atomic `{store_root}/projection_index.json` working copy.
  - Deferred SQLite until measured path/tag latency or post-VFS query semantics justify it.
- S10.4 Execution Fabric contract/readiness plan:
  - Added `docs/plans/2026-06-17-s10-4-execution-fabric.md`.
  - Documented Execution Fabric as a policy/control service for resource leases, runner/domain/node selection, duplicate suppression, execution traces, and Semantic State compute visibility.
  - Added the planned S10.4 slice to `SLICES.md`, `ROADMAP.md`, `CURRENT_STATUS.md`, and `NEXT_TASKS.md`.
- Security fix: Implemented seccomp BPF filter enforcement for POSIX runner sandbox (fixes NEW-005)
  - Added `seccompiler` dependency for Linux seccomp support
  - Implemented `build_seccomp_filter()` to construct BPF program from syscall whitelist
  - Implemented `apply_seccomp_filter_to_program()` to load filter via prctl(PR_SET_SECCOMP)
  - Added comprehensive tests for seccomp filter functionality
  - Blocks dangerous syscalls: execve, fork, clone, socket, bind, listen, accept, connect, setuid, setgid, etc.
  - Allows safe syscalls: read, write, mmap, exit, exit_group, clock_gettime, etc.
  - Files modified: `runtime_supervisor/Cargo.toml`, `runtime_supervisor/src/sandbox.rs`
- Security investigation report with evidence log and reconciled findings: `docs/plans/investigation_deep_security_review_2026-02-13.md`.
- Store-service regression tests for fail-closed signature policy on read/verify paths:
  - `get_blob_validation_failed_when_signature_required_and_manifest_unsigned`
  - `verify_artifact_validation_failed_when_signature_required_and_manifest_unsigned`
- Workspace lint baseline gate for staged clippy adoption:
  - Added `tools/ci/foundry_lint_baseline.sh` (host-workspace clippy run with machine-readable warning metrics).
  - Added `just clippy-baseline`, `just clippy-strict`, and `just clippy-baseline-soft` targets.
  - CI now runs lint baseline in `.github/workflows/ci.yml`.
  - Current baseline metric: `FOUNDRY_LINT_BASELINE: METRIC warning_count=0`.
  - Policy update (effective 2026-02-12): baseline gate is strict by default; local warning-tolerant runs must set `LINT_ALLOW_WARNINGS=1`.
- Strict Clippy rollout (Tranche 1):
  - Added `tools/ci/foundry_lint_strict_tranche1.sh`.
  - Added `just clippy-strict-tranche1`.
  - CI now enforces strict Clippy (`-D warnings`, `--no-deps`) for:
    - `artifact_store_schema`
    - `store_cli`
    - `domain_manager`
    - `portals`
  - Fixed tranche-1 warnings in those crates (unused imports, unnecessary casts, clone-from-ref, lint-only cfg, needless borrow).
- Strict Clippy rollout (Tranche 2):
  - Added `tools/ci/foundry_lint_strict_tranche2.sh`.
  - Added `just clippy-strict-tranche2`.
  - CI now enforces strict Clippy (`-D warnings`, `--no-deps`) for:
    - `artifact_store_core`
    - `idl_codegen`
    - `capsule_relay`
    - `runtime_supervisor`
  - Fixed tranche-2 warnings in `runtime_supervisor` via feature/test cfg scoping for POSIX warning helpers and test-only GPU token helpers.
- Strict Clippy rollout (Tranche 3):
  - Added `tools/ci/foundry_lint_strict_tranche3.sh`.
  - Added `just clippy-strict-tranche3`.
  - CI now enforces strict Clippy (`-D warnings`, `--no-deps`) for:
    - `kernel_api`
  - Replaced `CapTable` unit error returns with typed `CapTableError` to satisfy strict clippy without suppressions.
- Strict Clippy rollout (Tranche 4):
  - Added `tools/ci/foundry_lint_strict_tranche4.sh`.
  - Added `just clippy-strict-tranche4`.
  - CI now enforces strict Clippy (`-D warnings`, `--no-deps`) for:
    - `store_service`
  - Refactored `store_service` binary to consume shared library modules (instead of duplicate local `mod` copies), then fixed remaining strict-clippy findings.
- Strict Clippy rollout (Tranche 5):
  - Added `tools/ci/foundry_lint_strict_tranche5.sh`.
  - Added `just clippy-strict-tranche5`.
  - CI now enforces strict Clippy (`-D warnings`, `--no-deps`) for:
    - `kernel`
  - Fixed remaining kernel strict-clippy findings across boot/MM/shmem/init, and completed aarch64 MMU lint cleanup (test-only helpers cfg-gated, dead code trimmed, and cfg-safe TLB flush path).
- Added `tools/ci/foundry_preflight.sh` and `just preflight` for a one-command local gate (fmt check + codegen + strict lint + host tests + Foundry umbrella).
- Added contributor workflow and lint debt governance docs:
  - `CONTRIBUTING.md`
  - `docs/LINT_DEBT.md`
- S8 Phase 5.0: Physical address validation for UEFI init images (COMPLETE)
  - Implemented `is_valid_phys_addr()` function in kernel/src/init.rs
  - Validates physical addresses from UEFI firmware (NULL check, overflow protection, memory bounds)
  - Rejects MMIO region addresses (3-4 GiB range)
  - Properly aligned pointer validation (4-byte alignment required)
  - Comprehensive unit tests (now 6/6 tests passing after overlap-boundary coverage)
  - Commits: 9747eae (implementation), 5142c1f (tests)
- S8 Phase 5.1: Data-plane ring buffer core (COMPLETE)
  - Added lock-free SPSC ring buffer v0 in `kernel_api/src/ring_buffer.rs`
  - Monotonic producer/consumer indices with acquire/release memory ordering
  - Zero-copy read/write/query operations and cache-mode metadata
  - Added safety checks in `from_raw_parts` (non-null pointers, non-zero capacity, size bounds)
  - Unit tests: 12 ring-buffer tests passing (including panic guard for zero capacity)
  - Foundry gate: `tools/ci/foundry_ring_buffer_core_s8_phase5_1.sh`
- S8 Phase 5.2: Multi-domain MMU page-table allocation foundation (COMPLETE)
  - Added page-table allocation methods to MMU trait and arch backends (x86_64/aarch64)
  - Added on-demand intermediate page-table allocation in mapping paths
  - Enabled per-domain root lookups for domain-scoped mapping/unmapping/flush operations
- S8 Phase 4: Data-Plane Integration (COMPLETE - with QEMU-based integration tests)
  - BitmapAllocator for reusable frame allocation (kernel/src/mm/bitmap.rs) - 12/12 tests passing
  - AddressSpaceTable for per-domain page table root tracking (kernel/src/mm/address_space.rs) - 8/8 tests passing
  - MMU Programming Interface with architecture-agnostic trait (kernel/src/arch/mmu.rs)
  - x86_64 MMU implementation (kernel/src/arch/x86_64/mmu.rs)
  - aarch64 MMU implementation (kernel/src/arch/aarch64/mmu.rs)
  - Data-plane integration in ShmemRegionTable (kernel/src/shmem.rs) - 20/20 tests passing (with QEMU integration)
  - IPC handler updates with trace events (kernel/src/ipc_v0.rs)
  - Boot integration for allocator and address space table (kernel/src/boot.rs, kernel/src/mm/mod.rs)
  - Foundry gate: foundry_shmem_dataplane_s8_phase4.sh (40/40 core assertions passing)
  - **QEMU Integration Tests**: All 6 MMU programming scenarios validated
    - Gate: `foundry_shmem_dataplane_s8_phase4_integration.sh` (6/6 tests passing ✅)
    - Tests: map_region_increments_refcount, map_region_multiple_times_increments_refcount, unmap_region_decrements_refcount, close_region_fails_with_active_mappings, close_region_succeeds_after_all_unmaps, map_region_checks_rights_against_flags
    - Implementation: OP_SHMEM_TEST handler in kernel/src/init.rs
    - Test Profile: shmem_test in tools/init/build_init_image.py
    - Validates: CR3 manipulation, page table walks, TLB invalidation (x86_64), TTBR0_EL1 manipulation (aarch64)

### Changed
- Documentation synchronization and cruft cleanup:
  - Aligned active execution focus across `AGENTS.md`, `CURRENT_STATUS.md`, `ROADMAP.md`, and `NEXT_TASKS.md` to V-006 Phase 4 native runner work.
  - Resolved contradictory V-006 phase labeling in `CURRENT_STATUS.md` by splitting completed follow-up (`Phase 4a`) from remaining implementation work (`Phase 4b`).
  - Archived superseded planning/remediation documents to `docs/archive/plans/` and updated surviving references.
  - Fixed roadmap numbering drift and stale references to non-existent plan artifacts.
  - Verified markdown link integrity across the repository (`MISSING_COUNT=0`).
- Native WASM SDK follow-up hardening:
  - `examples/hello_wasm`: fixed host-test compatibility by gating wasm-only symbols and panic handler to wasm32, with conditional `no_std`.
  - `idl_codegen` wasm-imports renderer now fails closed without `unwrap()` panics on malformed/unknown IDL fields.
  - Generated WASM SDK client API now returns `(Status, usize)` from `call(...)`, exposing host-written reply length and clamping to output buffer bounds.
  - `kernel/src/mm/mod.rs`: removed invalid/duplicated in-function test blocks that caused kernel test compile failures.
  - `kernel/src/trace_ring.rs`: `reset_for_test()` now resets legacy SMP state, eliminating order-dependent legacy trace test failures.
  - Validation: `cargo test -p idl_codegen`, `cargo test -p ramen_sdk`, `cargo test -p hello_wasm`, `cargo test --workspace --exclude kernel_uefi --exclude kernel_aarch64`, and `tools/ci/foundry_native_wasm_s9_3.sh` all pass.
- Runtime/store env parsing now accepts boolish values (`1/0/true/false/yes/no/on/off`) for security-sensitive toggles:
  - `RAMEN_POSIX_RUNNER_ACK_RISK`
  - `RAMEN_POSIX_RUNNER_DISABLE_SANDBOX`
  - `RAMEN_STORE_DEV_MODE`
- Fixed `runtime_supervisor` POSIX-runner feature path compile issue by removing invalid `str::as_str()` usage on `&str`.
- S7 Foundry gate scripts corrected for evidence discipline:
  - fixed `EVIDENCE_DIR` typo in `foundry_s7_posix_runner_security.sh`
  - switched store_service gate invocations from unsupported CLI flags to env vars (`RAMEN_STORE_SOCKET`, `RAMEN_STORE_ROOT`) in `foundry_s7_store_signature_security.sh` and `foundry_s7_access_control_security.sh`
  - repaired shell syntax issue (`}` -> `fi`) in access-control gate
  - reworked POSIX gate to use deterministic `runtime_supervisor` unit tests instead of binary startup paths tied to store connectivity
  - fixed summary heredoc command-substitution side effects in S7 scripts by quoting heredocs
- Store signature enforcement now applies consistently to `GetBlob` and `VerifyArtifact` (not only `GetManifest`), returning `STATUS_VALIDATION_FAILED` on signature-policy rejection.
- runtime_supervisor POSIX-runner tests now serialize env-var mutations with a test mutex and include explicit sandbox-disabled execution coverage.
- Store capability trusted-key loading now resolves key sources in explicit fail-closed order (`RAMEN_STORE_CAP_TRUSTED_KEYS` → `RAMEN_STORE_TRUSTED_KEYS` file/inline fallback → dev-only default key fallback).
- Stabilized `store_service` env-var signature tests by serializing `RAMEN_STORE_TRUSTED_KEYS` access in tests to prevent parallel-test races.
- Replaced BumpAllocator with BitmapAllocator in kernel/src/mm/mod.rs
- Updated FRAME_ALLOCATOR to support deallocation
- Added get_current_page_table_root() to x86_64 and aarch64 modules
- Module structure: x86_64 now uses single-file pattern (removed x86_64/mod.rs)
- Pointer validation now enforces physical-address MMIO overlap rejection for init image ranges
- IPC shared-memory tests now perform explicit MM setup for deterministic domain validation
- Trace ring legacy test path now serializes global writer/reset operations to avoid parallel-test races

### Technical Notes
- Control/data-plane separation maintained
- Kernel-side capability validation preserved
- Type safety enforced with PhysAddr and PhysFrame wrappers
- Static allocation only (no heap usage)
- All operations emit trace events for auditability
- **MMU Programming Validation**: QEMU-based integration tests validate actual page table manipulation
  - Unit tests validate component logic (BitmapAllocator, AddressSpaceTable, basic ShmemRegionTable)
  - Integration tests validate MMU programming with real page tables (CR3, TTBR0_EL1)
  - Tests use domain_id 0 (kernel domain) which has valid page table setup

## 0.0.27 (2026-02-10)
- S7 Security Hardening Phase 2: implemented fail-closed store signature policy with RAMEN_STORE_TRUSTED_KEYS requirement.
- S7 Security Hardening Phase 2: added RAMEN_STORE_DEV_MODE environment variable for development (with prominent warnings).
- S7 Security Hardening Phase 2: changed access control default to RequireCredentials (fail-closed).
- S7 Security Hardening Phase 2: added RAMEN_STORE_ACCESS_POLICY environment variable support.
- S7 Security Hardening Phase 2: fixed exe whitelisting to use exact path matching with canonicalization.
- S7 Security Hardening Phase 2: integrated DomainArtifactRegistry with manifest metadata and directory structure support.
- S7 Security Hardening Phase 2: added POSIX runner runtime enforcement with RAMEN_POSIX_RUNNER_ACK_RISK=1 requirement.
- S7 Security Hardening Phase 2: enabled sandbox by default for POSIX runner.
- S7 Security Hardening Phase 2: added comprehensive logging for all security violations.
- S7 Security Hardening Phase 2: added 6 new unit tests for security hardening.
- S7 Security Hardening Phase 2: see [`docs/S7_SECURITY_HARDENING_PHASE2.md`](docs/S7_SECURITY_HARDENING_PHASE2.md) for details.

## 0.0.26 (2026-02-08)
- S8 Phase 3 (data-plane): implemented physical memory frame allocator with [`kernel/src/mm`](kernel/src/mm/mod.rs) module.
- S8 Phase 3 (data-plane): added type-safe [`PhysAddr`](kernel/src/mm/address.rs) and [`PhysFrame`](kernel/src/mm/address.rs) wrappers to prevent physical/virtual address confusion.
- S8 Phase 3 (data-plane): implemented [`FrameAllocator`](kernel/src/mm/frame.rs) trait for architecture-agnostic frame allocation.
- S8 Phase 3 (data-plane): implemented [`BumpAllocator`](kernel/src/mm/bump.rs) for early boot with static backing storage (512 MiB max, 131072 frames).
- S8 Phase 3 (data-plane): BumpAllocator uses simple "next free" pointer, never frees individual frames (graduation to bitmap planned for S8 Phase 4).
- S8 Phase 3 (data-plane): Foundry gate [`tools/ci/foundry_frame_allocator_s8_phase3.sh`](tools/ci/foundry_frame_allocator_s8_phase3.sh) with 25 assertions.
- S8 Phase 3 (data-plane): wired frame allocator to boot system.
- S8 Phase 3 (data-plane): UEFI boot path retrieves memory map and populates global allocator (x86_64, ~107500 frames).
- S8 Phase 3 (data-plane): AArch64 boot path initializes allocator with hardcoded RAM region (QEMU virt, 65536 frames).
- S8 Phase 3 (data-plane): [`foundry_s0.sh`](tools/ci/foundry_s0.sh) gate now asserts "mm: allocator ready" for both architectures.
- S8 Phase 3 (data-plane): 76 kernel tests passing (50 existing + 26 new mm tests).

## 0.0.25 (2026-02-08)
- S8 Phase 2 (control-plane): implemented kernel-side shared-memory region management with [`ShmemRegionTable`](kernel/src/shmem.rs).
- S8 Phase 2 (control-plane): static-array-backed region table (16 regions) with generation counters and refcount-based lifecycle.
- S8 Phase 2 (control-plane): added IPC handlers for `PROTOCOL_SHMEM_CONTROL` (protocol 8) with CreateRegion/MapRegion/UnmapRegion/CloseRegion.
- S8 Phase 2 (control-plane): CreateRegion allows `Handle::INVALID` for bootstrap; MapRegion/CloseRegion require valid capabilities.
- S8 Phase 2 (control-plane): all rights validated against region flags (REGION_FLAG_READABLE/WRITABLE/EXECUTABLE).
- S8 Phase 2 (control-plane): Foundry gate [`tools/ci/foundry_shmem_control_s8_phase2.sh`](tools/ci/foundry_shmem_control_s8_phase2.sh) with 32 assertions.
- S8 Phase 2 (control-plane): 46 kernel tests passing (shmem: 17 tests, cap_table: 7 tests, ipc_v0: 9 tests, others: 13 tests).
- Kernel improvements: fixed `StaticCapTable` 1-based indexing (index 0 reserved for INVALID).
- Kernel improvements: exported [`shmem`](kernel/src/shmem.rs) module from [`kernel/src/lib.rs`](kernel/src/lib.rs).
- Kernel improvements: extended `ipc_v0::handle_envelope` to accept `ShmemRegionTable` parameter for shmem control operations.

## 0.0.24 (2026-02-08)
- Wave C: SC-09 (kernel/services/store boundary split, V-11) completed with new [`artifact_store_schema`](artifact_store_schema) crate.
- Wave C: SC-09: [`artifact_store_core`](artifact_store_core) now re-exports schema types and provides IO-only functions.
- Wave C: SC-09: services ([`domain_manager`](services/domain_manager), [`portals`](services/portals)) updated to use schema for types/validation.
- Wave C: SC-09: compatibility shim maintained via re-exports in [`artifact_store_core`](artifact_store_core).
- Wave C: SC-10 (pin nightly toolchain, V-15) completed with pinned date `nightly-2026-02-08` in [`rust-toolchain.toml`](rust-toolchain.toml).
- Wave C: SC-10: toolchain rotation cadence documented as monthly or when new unstable features are needed.
- Wave C: SC-11 (unsafe safety comments, governance) completed with RFC 2585-style `// SAFETY:` blocks in arch modules.
- Wave C: SC-11: [`kernel/src/arch/x86_64.rs`](kernel/src/arch/x86_64.rs) and [`kernel/src/arch/aarch64.rs`](kernel/src/arch/aarch64.rs) now document invariants for port I/O, MMIO, and assembly.
- Wave C: SC-12 (multi-encoding evidence redaction) completed with hex/base64 marker support in [`evidence_policy`](artifact_store_schema/src/evidence_policy.rs).
- Wave C: SC-12: [`docs/EVIDENCE_POLICY_V0.md`](docs/EVIDENCE_POLICY_V0.md) updated with multi-encoding documentation.
- Wave C: Foundry gate [`tools/ci/foundry_hardening_wave_c.sh`](tools/ci/foundry_hardening_wave_c.sh) added with auditable PASS/FAIL markers.
- Wave C: V-11, V-15, SC-11, SC-12 mitigated via deterministic gate assertions.

## 0.0.23 (2026-02-08)
- Wave B Batch 2: SC-04 (unforgeable capability tokens, V-04) completed with gate [`tools/ci/foundry_hardening_wave_b_batch2.sh`](tools/ci/foundry_hardening_wave_b_batch2.sh).
- Wave B Batch 2: SC-05 (kernel capability table, V-05/V-06) completed with gate [`tools/ci/foundry_hardening_wave_b_batch2.sh`](tools/ci/foundry_hardening_wave_b_batch2.sh).
- Wave B Batch 2: kernel-side capability validation enabled for shared-memory control-plane operations via SC-04/SC-05.
- Wave B Batch 2: V-04, V-05, and V-06 mitigated via deterministic gate assertions and kernel capability table implementation.

## 0.0.22 (2026-02-08)
- Wave B Batch 1: SC-06 (trace ring ordering, V-07) completed with gate [`tools/ci/foundry_hardening_wave_b_batch1.sh`](tools/ci/foundry_hardening_wave_b_batch1.sh).
- Wave B Batch 1: SC-07 (init parser checked arithmetic, V-08) completed with gate [`tools/ci/foundry_hardening_wave_b_batch1.sh`](tools/ci/foundry_hardening_wave_b_batch1.sh).
- Wave B Batch 1: gate validates trace ring monotonic ordering and init parser arithmetic overflow checks.
- Wave B Batch 1: V-07 and V-08 mitigated via deterministic evidence assertions and negative gate checks.

## 0.0.21 (2026-02-08)
- Wave A Batch 2: SC-03 (posix_runner default-off, V-03) completed with gate [`tools/ci/foundry_hardening_wave_a_batch2.sh`](tools/ci/foundry_hardening_wave_a_batch2.sh).
- Wave A Batch 2: SC-02 (wire safety + codegen fail-closed, V-02/V-14) completed with gate [`tools/ci/foundry_hardening_wave_a_batch2.sh`](tools/ci/foundry_hardening_wave_a_batch2.sh).
- Wave A Batch 2: posix_runner now defaults to disabled (opt-in via explicit flag) to prevent accidental host exposure.
- Wave A Batch 2: wire format safety hardened with explicit little-endian encoding and codegen fail-closed on schema errors.
- Wave A Batch 2: V-02, V-03, and V-14 mitigated via gate-first hardening with machine-auditable assertions.

## 0.0.20 (2026-02-08)
- Wave A Batch 1: SC-01 (ContentId validation, V-01) completed with gate [`tools/ci/foundry_hardening_wave_a_batch1.sh`](tools/ci/foundry_hardening_wave_a_batch1.sh).
- Wave A Batch 1: SC-08 (log path confinement, V-09) completed with gate [`tools/ci/foundry_hardening_wave_a_batch1.sh`](tools/ci/foundry_hardening_wave_a_batch1.sh).
- Wave A Batch 1: ContentId validation added to artifact_store_core with strict SHA-256 format enforcement.
- Wave A Batch 1: log path confinement enforced in runtime_supervisor to prevent directory traversal.
- Wave A Batch 1: V-01 and V-09 mitigated via deterministic gate checks and negative assertions.

## 0.0.19 (2026-02-08)
- S8 Phase 1: added versioned shared-memory control-plane IDL contract [`idl/harness/shmem_control_v1.toml`](idl/harness/shmem_control_v1.toml) with typed `create_region`/`map_region`/`unmap_region`/`close_region` request+reply messages.
- S8 Phase 1: wired codegen outputs for `shmem_control_v1` in [`justfile`](justfile), CI IDL generation in [`.github/workflows/ci.yml`](.github/workflows/ci.yml), and generated bindings in [`kernel_api/src/generated/shmem_control_v1.generated.rs`](kernel_api/src/generated/shmem_control_v1.generated.rs).
- S8 Phase 1: exported new generated bindings from [`kernel_api/src/lib.rs`](kernel_api/src/lib.rs) and added focused contract tests for typed roundtrip + IPC envelope size fit (`shmem_control_contract_*`).
- S8 Phase 1: added deterministic contract gate [`tools/ci/foundry_shmem_contract_s8_phase1.sh`](tools/ci/foundry_shmem_contract_s8_phase1.sh) and integrated it into the umbrella Foundry flow [`tools/ci/foundry_all_s0_s1_s2_s3_s4_s5_s6.sh`](tools/ci/foundry_all_s0_s1_s2_s3_s4_s5_s6.sh).
- Validation: S8 Phase 1 gate now emits machine-auditable signals `FOUNDRY_SHMEM_CONTRACT_S8_PHASE1: METRIC ...` and `FOUNDRY_SHMEM_CONTRACT_S8_PHASE1: ok`, preserving S7 gate behavior.

### Additional 0.0.20 notes (2026-02-08)
- S7 deterministic replay hardening: [`tools/trace/replay_protocol_trace.py`](tools/trace/replay_protocol_trace.py) now emits stable machine-auditable outcomes (`REPLAY_PROTOCOL_TRACE: METRIC ...`, `MATCH`, `FAIL code=... detail=...`, `ok`) and computes a canonical replay digest over normalized request/response pairs.
- S7 replay validation now enforces strict deterministic invariants in replay: monotonic sequential `seq` values, strict request/response alternation, expected request→reply op pairing (including GPU quarantine operations), and deterministic digest comparison via `--compare`.
- S7 gate integration: [`tools/ci/foundry_gpu_quarantine_s7.sh`](tools/ci/foundry_gpu_quarantine_s7.sh) now performs dual domain-manager runs and hard-fails if replay digest or trace/observed/scenario content IDs diverge across runs.
- S7 gate now emits concise deterministic replay sentinels: `FOUNDRY_GPU_QUARANTINE_S7: METRIC replay_digest=...`, `FOUNDRY_GPU_QUARANTINE_S7: METRIC replay_trace_id=... replay_observed_id=... replay_scenario_id=...`, and `FOUNDRY_GPU_QUARANTINE_S7: REPLAY_DETERMINISM ok`.
- Evidence-discipline hardening: [`artifact_store_schema/src/evidence_policy.rs`](artifact_store_schema/src/evidence_policy.rs) error paths now include stable reason codes (for schema/version/max-bytes/utf8/parse/read/size-limit failures), and the S7 policy negative assertion now requires the stable `EVIDENCE_POLICY_SIZE_LIMIT_EXCEEDED` code.

### Additional 0.0.19 notes (2026-02-08)
- S7 hardening: upgraded [`tools/ci/foundry_gpu_quarantine_s7.sh`](tools/ci/foundry_gpu_quarantine_s7.sh) from sentinel-only checks to measurable threshold gates with deterministic machine-auditable fail output (`FOUNDRY_GPU_QUARANTINE_S7: FAIL code=... detail=...`).
- S7 thresholds now assert protocol/scenario/observed-cap evidence metrics and invariants: minimum protocol events/pairs, even request/response pairing, monotonic protocol seq, required scenario event names, minimum observed capability count, export capability granted/used minima, and minimum export dimensions.
- S7 gate now emits stable metric lines (`FOUNDRY_GPU_QUARANTINE_S7: METRIC ...`) plus `THRESHOLDS ok`, while preserving and hard-failing all prior negative assertions.
- S7 evidence discipline: gate now validates evidence-policy ingestion success for protocol/observed/scenario artifacts and includes a required policy-violation negative check (tiny max-bytes policy must fail ingest).
- CI now exports explicit S7 threshold env vars and evidence-policy path for the S0→S7 umbrella invocation in [`ci.yml`](.github/workflows/ci.yml).
- Evidence policy alignment: [`evidence_policy.toml`](evidence_policy.toml) now includes `scenario_trace` in supported kinds for S7 policy enforcement coverage.

## 0.0.18 (2026-02-07)
- Docs alignment milestone: synchronized AGENTS/CURRENT_STATUS/ROADMAP/NEXT_TASKS on one active execution track.
- Active focus is now consistently S7 hardening (measurable gate metrics + evidence discipline), with S8 shared-memory primitives as next phased focus.
- Clarified historical-vs-active sequencing language to prevent focus drift while preserving AGENTS non-negotiables.

## 0.0.17 (2026-02-07)
- S7: Added GPU quarantine IDL contract (`idl/harness/gpu_quarantine_v1.toml`) with generated bindings in `kernel_api`.
- S7: Domain Manager now handles typed GPU quarantine control messages (`start_quarantine_domain`, `export_display`, `report_scanout`, `stop_quarantine_domain`) and emits protocol/observed/scenario evidence.
- S7: Added `gpu_quarantine_v1` runner config to Store catalog and launch plan emission in `store_cli`.
- S7: Added `runtime_supervisor` GPU runner path (`runtime_supervisor/src/gpu_runner.rs`) with capability and dimensions validation.
- S7 Foundry: added `tools/ci/foundry_gpu_quarantine_s7.sh` covering positive flow, evidence validation, replay, and negative assertions.
- S7 Foundry: umbrella gate now executes the S7 gate and emits `FOUNDRY_ALL_S0_S1_S2_S3_S4_S5_S6_S7: ok`.
- CI: codegen step now generates `gpu_quarantine_v1` bindings and CI label updated to S0→S7 scope.

## 0.0.16 (2026-02-06)
- S6: Added `domain_manager` service with typed lifecycle API (`start/stop/status/list/report_exit`) and restart-policy handling.
- S6: Added Domain Manager IDL contract (`idl/harness/domain_manager_v1.toml`) and generated bindings in `kernel_api`.
- S6 Foundry: added `foundry_domain_manager_s6.sh` gate for multi-domain orchestration + restart policy assertions.
- S6: Added expanded portal suite binary (`portal_suite`) for clipboard, notifications, and screen capture capability evidence.
- S6: Added portal IDLs (`clipboard_v1`, `notifications_v1`, `screen_capture_v1`) and generated bindings.
- S6 Foundry: added `foundry_portal_suite_s6.sh` evidence gate validating protocol traces, observed caps, scenario traces, and replay.
- Foundry umbrella: added `foundry_all_s0_s1_s2_s3_s4_s5_s6.sh`.
- Boot-gate reliability: increased x86/aarch64 QEMU assertion wait windows in `foundry_s0.sh` and `foundry_init_s2_2.sh` to reduce false negatives on slower OVMF paths.
- CI now runs the S0→S6 umbrella gate.

## 0.0.15 (2026-02-06)
- S4: Added claim-chain resolution in core + CLI (`store_cli resolve-claim`) with "latest valid claim wins" and lease-expiry handling.
- S4 Foundry: claim workflow gate now verifies winner resolution and expired-claim behavior.
- S3.x: `idl_codegen` now emits C headers (`.h`) and generates `tools/capsule/generated/capsule_control_v0.h`.
- S3.x: capsule guest agent now consumes generated `capsule_control_v0.h` instead of hand-maintained control structs.
- S3: Added evidence policy module + `evidence_policy.toml` hook for redaction/size checks before ingestion.
- S3 Foundry: trace gate now validates redaction by ingesting a secret marker and asserting stored artifact scrubbing.
- S5+: Added minimal `posix_runner_v0` execution path in `runtime_supervisor` and new Foundry gate (`foundry_posix_s5.sh`).
- CI umbrella now runs the POSIX runner gate via `foundry_all_s0_s1_s2_s3_s4_s5`.

## 0.0.14 (2026-02-06)
- S4: claim timestamps now use real UTC RFC 3339 formatting (`time` crate) instead of approximate epoch math.
- S4: claim validation now parses RFC 3339 timestamps (rejects malformed date-time strings).
- S0 trace ring: reader now fast-forwards on overflow to avoid replaying overwritten slots as if they were historical events.
- S0 kernel: added trace ring overflow unit test coverage.
- Build hygiene: fixed aarch64 UART constant to satisfy `clippy -D warnings`.
- Docs: README and NEXT_TASKS updated to match current post-S5 status and priorities.

## 0.0.13 (2026-02-05)
- S5: Added RunnerIdentityV0 — stamps crash contexts and graduation attempts with runner kind/version/build_hash.
- S5: Added EvidenceBundleV0 — typed evidence fields (stdout_tail, stderr_tail, runner_log, core_dump, extras) on crash context.
- S5: Added ExitMetricsV0 — wall/cpu budgets, memory peak/budget, OOM events ref on crash context.
- S5: Deterministic minimal_policy output: capabilities/excluded sorted by cap name; proposer_version field added.
- S5: Fixed graduation JSON-load bug: progression_summary() now works after deserialization (was returning empty string).
- S5: Fixed pre-existing explain_priority test (effort threshold mismatch).
- S5: Enhanced Foundry gate with backward compat, evidence bundle validation, runner identity, deterministic output diff, and additional negative assertions.

## 0.0.12 (2026-02-05)
- S5: Added crash_context_v0 schema for structured crash bundles (Semantic State v2).
- S5: Added graduation_v0 schema for tracking progression across target levels (compat→posix→wasi→native).
- S5: Added minimal_policy_v0 schema with capability proposals and strictness scoring.
- S5: Wizard flow: `propose-policy` generates minimal policy from observed capabilities.
- S5: Added `store_cli` commands: validate-crash-context, validate-graduation, validate-minimal-policy, propose-policy, graduation-status.
- S5: Foundry gate validates crash contexts, graduation tracking, policy proposals, and negative assertions.

## 0.0.11 (2026-02-05)
- S4: Added queue_item_v0 artifact schema with program_id, target_level, evidence refs, prereqs, and scoring inputs.
- S4: Priority scoring formula: `priority = (vote_weight × leverage × reuse) / (effort × risk)` with 1-5 scales.
- S4: Human-readable priority explanations via `store_cli explain-priority`.
- S4: Prerequisites graph generation (JSON + DOT) with high-leverage prereq identification.
- S4: Claim/lock workflow (offline-first, content-addressed claims with lease duration).
- S4: Added `store_cli` commands: validate-queue-item, explain-priority, prereq-graph, claim, validate-claim.
- S4: Foundry gate validates queue items, priority explanations, prereq graphs, claims, and negative assertions.
- Schema docs: `docs/QUEUE_ITEM_V0.md`, `docs/CLAIM_V0.md`.

## 0.0.10 (2026-02-05)
- Added VM backend to capsule_relay (`--mode vm`) with virtio-serial transport.
- Added C capsule agent for Linux guest (`tools/capsule/capsule_agent.c`).
- Added initrd build script for capsule agent (`tools/capsule/build_capsule_initrd.sh`).
- S3.x gate tests both host-only and VM modes (VM requires S2_COMPAT_KERNEL + QEMU).
- Wire format uses little-endian for cross-arch determinism (traces as spec).

## 0.0.9 (2026-02-04)
- Added capsule.control v0 + harness.echo v0 IDLs and codegen outputs.
- Added host-only capsule relay service with mock capsule agent and protocol trace emission.
- Relay emits observed_caps_v0 + scenario_trace artifacts and ingests all trace artifacts.
- Added Foundry S3.x driver capsule gate and wired into the umbrella gate.
- Updated protocol trace replay to validate capsule control/echo message lengths and echo headers.
- S3.x gate now replays both control and echo traces (was echo-only).
- S3.x gate validates payload_ref presence in scenario trace.
- Fixed Python heredoc quoting in the S2 compat gate (CI).

## 0.0.8 (2026-02-04)
- Added observed_caps_v0 schema + validation and store_cli subcommand.
- Portal file picker now emits observed_caps + scenario_trace artifacts.
- Foundry portal gate validates observed caps + scenario trace references.

## 0.0.7 (2026-02-04)
- Added portal.file_picker v1 IDL + codegen and a host-side RO portal stub.
- Added Foundry gate for RO file picker with protocol trace + replay.

## 0.0.6 (2026-02-04)
- Added trace_artifact_v0 schema doc (`docs/TRACE_ARTIFACT_V0.md`).
- Added trace artifact validation in store_cli and protocol trace replay gate.

## 0.0.5 (2026-02-04)
- Init image format + builder script (`tools/init/build_init_image.py`) and kernel loader wiring.
- UEFI loads `init.img`; aarch64 boot loads init image from a fixed loader address.
- Added S2.2 init boundary gate (swap/malformed init) and wired into umbrella S0+S1+S2 gate.

## 0.0.4 (2026-02-04)
- Store CLI now requires explicit program_id selection for plan emission.
- Store-driven compat plans ingest assets and emit path-free content IDs.
- Compat gate now uses Store plan output and supervisor log-path override.

## 0.0.3 (2026-02-04)
- Added Driver Capsule v0 spec (`DRIVER_CAPSULE_SPEC.md`).
- Added kernel_api wire helpers and a trace ring single-writer token.
- Added Foundry negative assertions for IPC payload length and manifest schema mismatch.

## 0.0.2 (2026-02-04)
- S2 compat gate now mounts a read-only ext4 artifact image via virtio-blk
- S2 compat gate asserts write-blocked sentinel on the artifact mount
- Added S0+S1+S2 umbrella Foundry gate and CI wiring
- S2 compat gate builds initrd and artifact image when missing
- CI pins a compat kernel via URL + SHA-256 for deterministic S2 runs
- Compat Capsule Format v0 spec (docs/COMPAT_CAPSULE_V0.md)
- Compat runner v0 spawns QEMU from capsule config (runtime_supervisor/src/compat_runner.rs)
- Store emits linux_vm_v0 runner plans with compat_capsule config
- runtime_supervisor routes linux_vm_v0 plans to compat runner
- CI strict mode (RAMEN_CI_STRICT=1) fails on gate skip
- Fixed kernel fallback priority: URL fetch before /boot scan
- Aligned capsule spec with flat-field schema used by code
- Fixed QEMU virtio drive flag (virtio-blk → virtio)
- Compat runner now spawns real QEMU (no longer dry-run)
- S2 gate exercises full runner contract: gate → supervisor → compat runner → QEMU
- Removed --skip-artifact-verify flag (gate now uses content-addressed assets)
- Added store_cli ingest command for compat assets in installed content store
- S2 gate now ingests kernel/initrd/disk into content store and uses content IDs
- Supervisor now handles QEMU child cleanup on shutdown (no PID scraping in gate)
- CI caches pinned compat kernel and fetch script self-heals corrupt cache entries
- Added mirror_compat_kernel workflow and switched CI to use release asset URL

## 0.0.1 (2026-02-03)
- Added UEFI kernel entry for x86_64
- Added bare-metal aarch64 kernel entry for QEMU direct boot
- Added serial logging, init stub, IPC ping/pong, and trace ring buffer
- Foundry S0 gate now runs QEMU and asserts output
- Trace ring now uses monotonic indices to avoid wrap ambiguity
- Foundry gate waits for expected strings and disables reboot/shutdown
- Added Store S0 host tooling (store_cli + catalog + launch plan) and a Store S0 Foundry gate
- Began S1 host-only artifact pipeline (artifact_store_core + S1 gate)
- S1: manifest hygiene and core verification helper; S0+S1 umbrella gate
- S1.25: installed layout contract enforced (out/installed/artifacts)
- Added CONSTITUTION and fixed build/codegen workflow friction

## 0.0.0 (2026-02-03)
- Initialized repo structure
- Added PLATFORM_OVERVIEW + SLICES
- Added minimal IDL + codegen skeleton
- Added agent workflow + Slice S0 task list
