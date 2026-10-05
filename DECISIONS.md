# DECISIONS (ADR-lite)

**Last Updated:** 2026-10-04
**Status:** Active

## 2026-10-04 — Recover StoreClient transports without replaying uncertain effects

Use one socket-configuration path for initial and replacement Store connections.
Before dispatch, a nonblocking peek can detect an idle peer's already visible
closure and permit reconnecting. Once request writes begin, any exchange failure
drops the stream and returns the error. The next explicit operation reconnects;
the client never automatically replays a dispatched ingestion whose publication
may have succeeded. Callers must reconcile artifact/ownership state before a
deliberate retry. This fixes retained consumers such as the file picker without
changing Store authority, receipt semantics, or native IDL contracts.

The configured timeout bounds the whole response frame and each blocking write;
connect and request dispatch are not one invocation-wide budget. Frame reads use
readiness polling and per-call nonblocking receive against one absolute deadline,
including partial reads. This also preserves complete queued replies after a
peer closes: changing receive timeouts at that point fails on the macOS host.

Gate-first regressions reproduce the old initial unbounded wait, repeated dead
stream reuse, real-server idle expiry and response trickles. Fake peers observe
complete ingestion dispatch followed by lost, malformed or truncated replies
and check that only a later explicit read reaches the replacement connection.
The existing review-boundaries gate includes these tests. This is host transport
evidence, not target-native enforcement or general filesystem containment.

## 2026-02-03 — Monorepo with hard boundaries
We start as a monorepo to move quickly while IDLs stabilize. Boundaries are enforced by directory structure and contracts.

## 2026-02-03 — Rust-first kernel and services; Python for orchestration/tooling
Kernel and core services are Rust-first for safety and maintainability.
Python is allowed for tooling/CI orchestration, backed by Rust libraries where needed.

## 2026-02-03 — POSIX is compatibility-only
Native interfaces are Harnesses/Portals. POSIX exists only in compatibility layers/runners.

## 2026-02-03 — Nightly toolchain initially
Bare-metal bring-up will likely require nightly. We use `channel = nightly` and will pin a date later once boot is stable.

## 2026-02-03 — Semantic interfaces scope
Semantic interfaces are applied to OS metadata and service APIs (e.g., introspection, portals).
We will not attempt a semantic filesystem in early slices.

## 2026-02-03 — Linux compatibility domain is virtualization-first
Linux compatibility runs in a VM/microVM boundary, not syscall translation.

## 2026-02-03 — GPU is a hostile/quarantined device boundary
GPU stacks run in a quarantine domain; no kernel-space blobs or ambient trust.

## 2026-02-03 — UEFI boot path for Slice S0
To minimize bootloader complexity, Slice S0 uses UEFI applications for both x86_64 and aarch64.
Kernel bring-up runs as a UEFI app and writes directly to serial (COM1 on x86_64, PL011 on aarch64).
This is a scaffolding choice for early QEMU gates and will be replaced by a dedicated boot chain later.

## 2026-02-03 — Init component baked into kernel image for S0
For Slice S0 only, the init component is linked into the kernel image and invoked directly.
This preserves the init flow while deferring process/loader work to later slices.

## 2026-02-03 — aarch64 QEMU uses direct kernel boot for S0
Homebrew AAVMF on macOS failed to locate the removable media entry for BOOTAA64.EFI.
To keep Slice S0 moving, the Foundry gate boots aarch64 via QEMU `-kernel` with a minimal bare-metal entry.
We keep the UEFI path for x86_64 and can revisit aarch64 UEFI once firmware/vars are stable.
Revisit criteria: switch back when CI host has aarch64 UEFI vars + removable boot entry working reliably.

## 2026-02-04 — Driver Capsule v0 relay starts host-only
To unblock S3.x, the relay protocol, tracing, and replay gate are validated in a host-only capsule relay service.
The microVM + virtio-serial wiring remains the next milestone.

## 2026-02-07 — S7 GPU quarantine starts with typed host-sim handshake
To keep vertical-slice velocity while preserving architecture invariants, S7 introduces GPU quarantine via a typed control-plane handshake (`gpu_quarantine_v1`) implemented first in host simulation.
The slice includes:
- IDL-first contract + generated bindings,
- a domain-manager GPU control path (`start/export/scanout/stop`),
- a Store launch-plan consumer (`gpu_quarantine_v1`),
- and a Foundry gate with replay + negative assertions.

Deferred by design:
- real GPU isolation and native scanout scheduling internals,
- data-plane zero-copy surface transport details.

Revisit criteria:
- promote from host-sim to real quarantine runtime once kernel/service boundaries for surface transport are stabilized and measurable latency/throughput gates are defined.

## 2026-02-10 — Fail-Closed Signature Validation (S7-001)
Store service requires `RAMEN_STORE_TRUSTED_KEYS` in production mode and aborts startup if not configured. This prevents silent fallback to `AllowUnsigned` policy which could allow unsigned artifacts to be accepted. Development mode (`RAMEN_STORE_DEV_MODE=1`) allows unsigned artifacts with prominent warnings. Breaking change, but critical for security.

## 2026-02-10 — Fail-Closed Access Control (S7-002)
Store service defaults to `RequireCredentials` access control policy instead of `AllowAll`. This prevents unauthorized access by default and requires valid Unix credentials (PID, UID, GID) for all operations. Policy can be overridden via `RAMEN_STORE_ACCESS_POLICY` environment variable for development. Breaking change, but essential for security.

## 2026-02-10 — Exact Path Matching (S7-003)
Store service uses `std::fs::canonicalize()` for exact path matching in exe whitelisting instead of substring matching. This prevents bypass attacks via paths like `/tmp/domain_manager_malicious`. Slightly more complex, but eliminates substring-based bypass vector.

## 2026-02-10 — POSIX Runner Runtime Enforcement (S7-004)
POSIX runner requires `RAMEN_POSIX_RUNNER_ACK_RISK=1` at runtime instead of using a compile-time feature flag. Sandbox is enabled by default for defense-in-depth. This replaces the previous `posix_runner_v0_dev` feature flag with a runtime kill-switch. More flexible, but requires explicit acknowledgment of security risks.

## 2026-02-10 — DomainArtifactRegistry Integration (S7-005)
Store service makes "global" artifact visibility explicit via directory structure (`store_root/global/` for global artifacts, `store_root/domains/{domain_id}/` for domain-specific artifacts). Ownership is read from manifest metadata during scan. This prevents accidental global artifact registration and provides clear ownership semantics. Requires directory reorganization, but clearer semantics.

## 2026-06-17 — S10.3.1 Projection Index Backend
S10.3.1 uses a CAS-backed `ProjectionIndexV0` artifact with `{store_root}/projection_index.json` as the atomic working copy. `store_service` loads the working copy for path/tag queries, persists validated JSON atomically after index mutation, and snapshots that JSON through the existing Store artifact path.

SQLite inside `store_service` is deferred. It may be reconsidered only after measured path/tag latency, JSON rewrite cost, or post-VFS vector/graph query semantics justify the extra TCB, migration, and backup complexity.

Design: `docs/archive/plans/2026-06-17-s10-3-1-projection-index-backend.md`.

## 2026-06-17 — S10.3.3 Read-Only VFS Transport
S10.3.3 uses QEMU virtio-9p (`-virtfs local,readonly=on`) to expose a host-materialized projection directory to compat guests. The host builds a read-only symlink tree from `ProjectionIndexV0.path_projections` to CAS blobs; `compat_runner` exports it with mount tag `ramen_store`.

virtio-fs is deferred until measured 9p latency in QEMU or multi-VM shared export requirements justify adding `virtiofsd` to the supervisor TCB.

Design: `docs/archive/plans/2026-06-17-s10-3-3-read-only-vfs-projection.md`.

## 2026-06-17 — S10.5.0 Option A with init-bridge (inventory-confirmed)
**Chosen:** Option A — Semantic State on QEMU.

**Inventory finding:** `native_runner` is host-only (Wasmtime + `std`); QEMU runs kernel + init bytecode only. `semantic_state.wasm` does not execute on target today.

**S10.5.0 implementation path:** Kernel init op `OP_SEMANTIC_SNAPSHOT` + `semantic_snapshot` init profile — proves typed `get_snapshot` bytes + shmem on QEMU serial (pattern: `shmem_test`). Does **not** run WASM inside QEMU.

**Deferred to S10.5.1:** Broker/kernel bridge for one harness path (`shmem_control` + `semantic_state`) to unlock host `native_runner` real caps. Full broker migration and `store_service` on QEMU remain out of scope.

Design + red gate: `docs/plans/2026-06-17-s10-5-host-to-target-integration.md`, `tools/ci/foundry_host_target_s10_5.sh`.

## 2026-06-17 — Direct IPC IDL is fixed-wire only
Direct Rust IPC bindings generated into `kernel_api` must contain only fixed-size wire-safe fields. Dynamic `string` and `bytes` fields are host ABI concepts, not durable envelope fields, and are rejected for direct IPC structs.

Large or rich values must cross the control plane as `bytes32` content hashes, capability/token handles, or shared-memory descriptors such as `{shm_cap, offset, len}`. IDL files must also declare explicit non-zero `protocol` and `msg_type` values so generated metadata is the canonical source for services and replay tooling.

This keeps S11 replay artifacts from depending on host pointers, map ordering, or local Rust struct roundtrips.

## 2026-06-18 — S11.3 live Oracle trace promotion is explicit
The virtio-net Reference Vault may keep a scaffold `oracle_init_trace.json` so normal CI can validate schema, translation, and replay plumbing before a live Linux Oracle capture is available.

Promotion to a live Oracle trace is explicit: `REQUIRE_LIVE_ORACLE_TRACE=1 tools/ci/foundry_s11_reference_vault_s11_3.sh` must pass. The strict path rejects scaffold trace IDs and requires a full `sha256:` trace ID, `timestamp_ns` on every event, and contiguous `seq=1..N` event numbering. Captured `pci_mmio_tracer` debugfs JSONL is converted with `driver_foundry import-jsonl`, then replayed with `driver_foundry replay-trace`.

This avoids claiming live hardware evidence from hand-authored fixtures while preserving a green replay harness for S11.3 development.

## 2026-06-18 — S11.3 virtio-net vault pins OASIS VIRTIO v1.3
The virtio-net Reference Vault uses OASIS VIRTIO Version 1.3 as its pinned source for S11.3 distillation notes.

The vault stores compact derived notes rather than a copied specification dump. The notes name the PCI discovery IDs, virtio PCI capability constants, initialization status sequence, virtqueue layout, network feature bits, and network configuration fields that must explain the Oracle trace before native driver code is written.

This gives agents stable source anchors while keeping the vault small and avoiding register names inferred from memory alone.

## 2026-06-18 — S11.3 live capture promotion is one-command and fail-closed
The live virtio-net Oracle capture must be promoted through `tools/trace/promote_virtio_net_capture.sh`.

The script writes to a temporary trace first, then runs JSONL import, replay translation, and `assert-live-trace` before copying over the Reference Vault fixture. `driver_foundry import-jsonl` stamps a missing `trace_id` from the source JSONL SHA-256 digest, and `assert-live-trace` requires that `sha256:` provenance marker plus contiguous event numbering. The S11.3 gate dry-runs this path with a live-shaped sample so the promotion workflow stays executable even before the real Linux capsule artifact exists.

This keeps scaffold CI green while preventing a partial or non-live capture from replacing `oracle_init_trace.json`.

## 2026-06-21 — S11.7 packet receive uses kernel netdev, not userspace virtqueue
TCG QEMU userspace legacy virtqueue RX did not complete in the Oracle capsule (used rings stayed 0). The compat kernel exposed `virtio_pci` but not `virtio_net`, so no `eth0` appeared for AF_PACKET capture until kernel modules were bundled.

**Chosen:** Load the Ubuntu mainline `failover` → `net_failover` → `virtio_net` module chain inside the packet-capture initrd, bring up `eth0`, and capture live ARP send/receive via AF_PACKET. Userspace virtqueue programming remains a fallback only when module load fails.

**Rejected:** Slirp-derived ARP reply synthesis as Oracle evidence (`slirp-arp-reply-derived` notes are rejected by `assert-hardware-packet-trace`). `/dev/mem` + pagemap PFN experiments did not unblock virtqueue RX.

**Gate:** `REQUIRE_LIVE_ORACLE_TRACE=1 foundry_s11_reference_vault_s11_3.sh` runs `assert-hardware-packet-trace` on `oracle_packet_trace.json`.

**Follow-up (resolved 2026-06-21):** S11.8 landed via `foundry_s11_runtime_net_s11_8.sh`; S11 Definition of Done complete (`just s11`).

## 2026-06-21 — S12 Tier-1 golden machine reference
S12 requires a single reproducible bare-metal profile before GOP/HIL implementation.

**Chosen:** Intel NUC 12/13 class (x86_64 UEFI, integrated Intel GOP, VT-d). Manifest: `hardware/golden_machine_v0.toml`.

**Rejected for primary reference:** Framework Laptop (higher board variance for lab farms); Tier-2 SBCs without IOMMU (degraded trust only, per `docs/HARDWARE_STRATEGY.md`).

**CI policy:** Default CI runs S12.0 smoke gate only; physical HIL gates require `RAMEN_HIL_GOLDEN_MACHINE=1`.

**Gate:** `foundry_s12_golden_machine_s12_0.sh`; fast-path `just s12`.

## 2026-06-26 — S12 Tier-1 golden machine reference update
The Intel NUC 12/13 reference was a suitable PC-class baseline, but its serial
path depended on model-specific headers or added USB serial hardware. The
active HIL loop needs a directly repeatable target-side serial interface.

**Chosen:** Lenovo ThinkCentre M720s Small Form Factor with an Intel Core
i5-8400, at least 8 GiB RAM, and at least 256 GiB M.2 NVMe storage. Its rear
RS-232/DB9 port is now a Tier-1 contract requirement, alongside UEFI GOP,
VT-d, NVMe, and xHCI. Manifest ID:
`lenovo-thinkcentre-m720s-i5-8400-reference`.

**Required preflight:** Enable UEFI boot, Intel VT-d, rear serial, and USB boot
in firmware before physical evidence runs. Physical evidence remains pending;
selecting the reference does not claim `PASS/HIL-LIVE` or `PASS/METAL`.

**Supersedes:** The 2026-06-21 Intel NUC 12/13 selection as the active Tier-1
reference. That entry remains the historical basis for S12.0.

**Gate:** `foundry_s12_golden_machine_s12_0.sh`; fast-path `just s12`.

## 2026-07-01 — Acquired ThinkCentre M900 becomes the S12 Tier-1 reference

The planned M720s was not the machine acquired for the physical lab. Keeping
its identity as the active default would make controller and target evidence
name hardware that was not actually under test.

**Chosen:** Pin the acquired Lenovo ThinkCentre M900 Small Form Factor, machine
type 10FH and model 00SNUS, as
`lenovo-thinkcentre-m900-i7-6700-lab-01`. Operator photos establish an Intel
Core i7-6700, 8 GiB RAM, active integrated graphics, and a populated rear
RS-232/DB9 connector. Replace the existing SATA disk with a nominal 128 GB M.2 NVMe
drive before physical HIL execution.

**Evidence boundary:** The photographs are inventory evidence only. UEFI GOP,
VT-d/DMAR, live serial capture, NVMe boot, and atomic rollback still require
their matching gates and target-emitted provenance markers. Until the NVMe is
installed and firmware preflight passes, the manifest remains
`acquired_pending_nvme_install_and_preflight`.

**Supersedes:** The 2026-06-26 M720s selection as the active Tier-1 reference.
That entry remains as decision history.

**Gate:** `foundry_s12_golden_machine_s12_0.sh`; fast-path `just s12`.

## 2026-07-01 — Stage S12 on SATA and validate AMT before fallback hardware

The acquired M900 already contains a 1 TB SATA HDD, its reserved NVMe does not
fit correctly, and its Core i7-6700 platform exposes Intel vPro / AMT 11. These
facts allow the physical observation loop to start without either blocking on
new storage or modifying the proprietary front-panel harness.

**Chosen (storage):** Permit S12 boot, GOP, serial, IOMMU inventory, and HIL
appliance work on the installed SATA HDD. Keep compatible M.2 2280 PCIe NVMe as
an explicit S13 metal-graduation prerequisite; SATA evidence cannot satisfy the
S13 NVMe boot or two-boot atomic-update claims.

**Chosen (actuation):** Provision and validate AMT 11 over the trusted wired lab
network as the M900's primary status, power-on, power-off, reset, and
power-cycle path. AMT credentials are runtime secrets and must not enter Git,
logs, or evidence JSON.

**Deferred:** Do not purchase a smart plug/PDU or front-panel relay until AMT
has been tested while the target is running, soft-off, and hung in the target
OS. A failed or incomplete AMT recovery matrix is the evidence required to add
fallback hardware.

**Supersedes:** The unexecuted 2026-07-01 assumption that NVMe installation must
precede every physical HIL run, and the generic relay-first actuator ordering
for this M900 target. It does not relax the S13 NVMe graduation boundary.

**Gates:** `just s12`; `just hil-appliance`; later AMT live-actuation gate.

## 2026-07-19 — M900 storage swap and physical HIL lab ready

The M900's original 1 TB SATA HDD was replaced with a 240 GB SanDisk SATA
drive. The Pi controller, FTDI USB-serial path, null-modem adapters, and M900
rear DB9 serial chain are physically installed and ready for the first live
HIL runs.

**Chosen (storage):** Record the installed system drive as a 240 GB SanDisk
SATA SSD (`sata_ssd`) in `hardware/golden_machine_v0.toml`. S12 work continues
on this SATA device; S13 metal graduation still requires a compatible M.2 2280
PCIe NVMe drive.

**Chosen (lab readiness):** Mark the golden machine and HIL appliance manifests
`physically_ready` pending firmware/AMT preflight and the first live serial
capture. Wiring is no longer a blocker; the next executable step is firmware
preflight plus `RAMEN_HIL_APPLIANCE=1 just hil-appliance` live capture.

**Evidence boundary:** Operator report establishes inventory and physical setup
only. No `PASS/HIL-LIVE`, `PASS/HIL-APPLIANCE`, or `PASS/METAL` claim is
implied.

**Supersedes:** The 2026-07-01 inventory assumption of a 1 TB SATA HDD as the
installed S12 drive.

**Gates:** `just s12`; `just hil-appliance`; `just s12-hil` after live capture.

## 2026-06-21 — S13 Oracle block device selection
S13 requires a QEMU stepping stone before metal NVMe graduation.

**Chosen (Oracle):** `virtio-blk-pci` in QEMU Linux capsule — mirrors S11 virtio-net pattern. Harness: `harness.block` (`idl/harness/block_v1.toml`). Manifest: `hardware/storage_contract_v0.toml`.

**Chosen (metal):** M.2 NVMe PCIe on Tier-1 golden machine class; A/B GPT slots for atomic update (S13.8). Specific NVMe controller vendor unpinned until lab capture.

**Rejected for Oracle MVP:** Native NVMe passthrough in QEMU (vendor variance, boot-critical complexity before replay loop exists).

## 2026-06-22 — HIL appliance before deeper bare metal
Manual reboot/capture workflows do not scale to agentic OS development or hardware fuzzing.

**Chosen:** Add a Raspberry Pi-class HIL appliance controller before deeper metal automation. The appliance is always-on lab infrastructure that performs serial capture, power/reset actuation, evidence bundling, and later KVM/virtual-media control. Manifest: `hardware/hil_appliance_v0.toml`; plan: `docs/plans/2026-06-22-hil-appliance-controller.md`.

**Controller/target boundary:** The appliance is not part of the RamenOS target TCB. It observes and actuates; it does not create target truth. Graduation still requires live target-emitted `hil_evidence:` markers and evidence JSON.

**Electrical rule:** Default serial path is target COM/DB9/header → RS-232 adapter/cable → USB serial adapter → Pi USB. Raw Pi GPIO UART is TTL-only and must not be connected directly to PC RS-232/DB9.

**CI policy:** Default CI validates docs/manifests only. Hardware controller checks require `RAMEN_HIL_APPLIANCE=1`. Graduation runs require `RAMEN_HIL_APPLIANCE=1 RAMEN_HIL_GRADUATION=1` and disallow stale serial-log replay.

## 2026-06-24 — HIL metal claims must disclose the run path
The evidence taxonomy allows standalone Tier-1 golden-machine `PASS/METAL`
when `RAMEN_HIL_GRADUATION=1`, live serial capture, target-emitted
`hil_evidence:` markers, and per-gate evidence JSON are present. The active
S12.4/S13 execution queue still prefers appliance-mediated graduation before
claiming the S13 slice complete.

**Chosen:** Keep standalone golden-machine graduation legitimate, but make the
path machine-readable. Per-gate HIL JSON now carries `claim_path`:
`operator-golden-machine` for standalone live graduation and
`appliance-mediated` when `RAMEN_HIL_APPLIANCE=1`. Appliance-mediated runs also
include an `appliance` object with controller identity and evidence references.

**Rejected:** Silently treating every `PASS/METAL` as appliance-mediated, or
requiring the appliance for all future `PASS/METAL` claims before S12.4.2 power
actuation has landed.

**Gate:** `just hil-appliance` validates both claim paths in per-gate evidence
fixtures.

## 2026-06-24 — POSIX runner default profile is rlimits-only
The POSIX compatibility runner remains a development scaffold. Its default
`posix_run_v0_sandboxed` profile uses a host-portable configuration:
`seccomp=false`, `namespaces=false`, `chroot=false`, and `rlimits=true`.
Seccomp, namespace, and chroot helpers remain implemented and tested in
`sandbox.rs`, but are not wired into the default runner path because they are
not portable on unprivileged CI hosts.

**Chosen:** Report the actual profile in runtime logs and documentation instead
of claiming full sandbox containment. Keep `RAMEN_POSIX_RUNNER_ACK_RISK=1` as
the explicit execution gate and track full default sandboxing as future work.

**Rejected:** Enabling seccomp/chroot/namespaces in this honesty patch without a
new portable pre-exec design and gate-first rollout.

**Gate:** `just foundry-s7-posix-runner-security` now checks the logged default
profile and a unit contract for the rlimits-only configuration.

## 2026-06-23 — RamenOrg starts as an Org Kernel, not an ambient AI board
The project already uses agents heavily, but the founder is still often acting as
the message bus between planning, implementation, review, evidence, and status
updates.

**Chosen:** Add a G0 RamenOrg scaffold with bounded artifacts: role charters,
authority levels, heartbeats, `WorkOrderV0`, `HandoffPacketV0`, `BoardVoteV0`,
claim safety, and a status-drift Foundry gate. Plan:
`docs/plans/2026-06-23-research-backed-ramenorg.md`.

**Authority boundary:** G0 is A0/A1 only. It may add docs, gates, drift checks,
and proposal artifacts. It does not grant merge, release, HIL actuation, or
public support authority.

**Rejected for now:** A broad "AI board" with ambient repo/tool access. RamenOrg
must use the same capability discipline as RamenOS: no undocumented handoff, no
unsupported claim, no same-agent write/approve/merge/announce path.

## 2026-06-23 — Research is a first-class production lane
The offers/airlock work and RamenOrg governance work are not side essays. They
address project risks that affect architecture, security claims, autonomy, and
delivery at OS scale.

**Chosen:** Treat RamenOS as research-backed, not a research OS. Doctrine-level
novelty must be tracked as research questions with product risk, claim boundary,
required outputs, landing path, and evidence plan. Initial questions:
`docs/research/questions/RQ-0001-offer-boundaries.md` and
`docs/research/questions/RQ-0002-ai-org-kernel.md`.

**Guardrail:** Research may block shallow implementation when the underlying
claim is not understood, but every research item must stay tied to a shipping
path and Foundry/evidence plan.

## 2026-06-23 — Offer-boundary doctrine splits request and observable authority
The provider-authored offers paper sharpens RamenOS capability doctrine for
agent-facing and cross-domain boundaries.

**Chosen:** Future designs in this area must separate `Lang` (what a holder may
ask) from `ObsContract` (what a holder may learn). Request minimization and
topology hiding are action-safety and surface-reduction techniques; they are not
noninterference claims without measured observable-channel evidence.

**Implementation boundary:** No runtime offer-boundary implementation lands
until RQ-0001 produces a RamenOS-specific design pass, IDL plan, claim levels,
and Foundry gate.

## 2026-06-23 — G0.1 validates packets before autonomy
G0 created Markdown packet definitions and a status-drift gate. The next useful
hardening step is not broader authority; it is validating actual packets.

**Chosen:** Add JSON schemas for `WorkOrderV0`, `HandoffPacketV0`,
`BoardVoteV0`, and `BoardPacketV0`, plus stdlib-only renderer/validator tools
that generate S12.4.1 example packets under `out/org/` during the governance
gate.

**Authority boundary:** G0.1 remains A0/A1. Packet validation may make handoffs
more mechanical, but it does not grant merge, release, HIL actuation, hardware
credentials, or public support authority.

**Gate:** `just foundry-org-governance-g0` renders and validates packet examples
after the existing status-drift check.

## 2026-06-23 — G0.2 makes packets agree about one active task
G0.1 validated packet shape, but the renderer still carried task constants and
the validator did not prove that referenced packets described the same work.

**Chosen:** Add `docs/org/current_task.yaml` as the machine-readable source for
the active packet set. The renderer reads that file, and the validator now
performs cross-packet checks for repo SHA, work-order/proposal identity, task,
required gates, authority level, and typed evidence buckets.

**Gate-ref policy:** Unknown gate syntax fails closed. Valid gate refs are
`just <recipe>` or existing executable-style repo paths.

**Authority boundary:** G0.2 remains A0/A1. It improves state transfer and
validation, not autonomy.

## 2026-06-23 — G0.3 validates the active-task source and bad cases
The current-task YAML became the source of packet truth in G0.2, but it needed a
schema and negative validator evidence.

**Chosen:** Add `CurrentTaskV0`, validate `docs/org/current_task.yaml` before
rendering, bind `BoardVoteV0` to repo SHA, enforce exactly-one board refs for the
scaffold phase, and add negative validator cases for malformed packet/current
task state.

**Authority boundary:** G0.3 remains A0/A1 and does not introduce credentials,
merge authority, release authority, HIL actuation, or public support authority.

## 2026-06-23 — G0.3.1 makes claim-boundary denial explicit
G0.3 hardened packet validation, but a few generated labels still referenced
older G0 slices and the claim-boundary check accepted partial denial phrasing.

**Chosen:** Add a G0.3.1 hygiene slice, update current-task labels, renderer
claim text, and validator diagnostics, and require `claim_boundary` to include
all four explicit denials: no merge, no release, no HIL actuation, and no public
support authority. Also reject PASS/METAL claims when HIL evidence refs are
absent.

**Authority boundary:** G0.3.1 remains A0/A1 and does not introduce merge,
release, HIL actuation, public support, credentials, or identity-level role
separation authority.

## 2026-06-23 — G0.4 emits a read-only board brief after validation
Validated packets are machine-readable, but the next agent still needs a compact
human-readable handoff surface.

**Chosen:** Add `BoardBriefV0` and `tools/org/render_board_brief.py`. The
governance gate renders `out/org/current_board_brief.md` only after packet
validation passes, and then checks for active task, authority boundary, required
gates, context refs, evidence refs, and allowed next-agent actions.

**Authority boundary:** G0.4 remains A0/A1. The brief is read-only and grants no
merge, release, HIL actuation, public support, credentials, or identity-level
role separation authority.

## 2026-06-23 — G0.5 binds agent intake artifacts by SHA-256
The G0.4 gate rendered immediately after validation, but a reused passing report
did not prove that the packet files were still the bytes that had been checked.

**Chosen:** Record a SHA-256 ledger in packet validation reports. Require the
board brief renderer to verify every checked artifact before rendering, then
emit `IntakeManifestV0` binding the brief, packet set, current-task source, and
validation report. Validate the manifest independently and retain a negative
case that changes a packet after validation.

**Authority boundary:** G0.5 remains A0/A1. Hash binding is evidence integrity,
not merge, release, HIL actuation, public support, credential, or identity-level
role authority.

## 2026-06-23 — G0.6 tests intake before adding more packet machinery
The G0.5 bundle was structurally portable, but it had not been tested as the
only context given to a fresh agent.

**Chosen:** Run an isolated plan trial with inherited thread context disabled.
Supply exactly the board brief, intake manifest, board packet, work order,
handoff, and vote. Record the response and gate the resulting evidence
for packet citations, authority denials, required gates, hidden-chat use, and
external file reads.

**Finding:** The bundle is sufficient for a bounded plan without hidden chat.
It is not patch-complete because it cites but does not contain the controller
plan, evidence policy, status/task sources, or scoped implementation files. Keep
that as an explicit artifact-availability finding; do not repair it with ad hoc
prompt context.

**Authority boundary:** G0.6 remains A0/A1 and grants no merge, release, HIL
actuation, public support, credential, or identity-level role authority.

## 2026-06-23 — G0.7 separates input context from authorized new outputs
The G0.6 bundle was sufficient for planning but not for a patch plan because it
named needed context without granting file contents.

**Chosen:** Add `ContextGrantV0`. The grant hashes existing input files and
classifies them as read or patch access. It also carries `authorized_new_paths`
for scoped outputs that do not exist yet and therefore cannot be hash-bound as
input context. The intake manifest binds both the grant and every granted file.

**Finding:** The first G0.7 trial correctly requested expansion when the new
serial-observer path was not modeled. After adding `authorized_new_paths`, a
fresh agent produced `PASS/PATCH-PLAN` using only supplied artifacts, with no
hidden chat, no external reads, no expansion request, and no implementation.

**Authority boundary:** G0.7 remains A0/A1 and grants no merge, release, HIL
actuation, public support, credential, or identity-level role authority.

## 2026-06-23 — G0.8 may implement a bounded S12.4.1 scaffold patch
G0.7 proved the intake bundle could produce a responsible patch plan, but it did
not prove a patch could land without widening authority.

**Chosen:** Authorize a narrow implementation trial for the S12.4.1 serial
observer. G0.8.1 reclassifies this as A2-local rather than A1 proposal
authority. The implementation surface is the new observer script and the HIL
appliance gate contract checks. Once the observer file exists, it is no longer an
`authorized_new_path`; it becomes hash-bound granted context for subsequent work
orders.

**Finding:** The observer can be gated without hardware by using a synthetic
serial transcript and a stale-log graduation negative case. Live appliance use
still requires `RAMEN_HIL_APPLIANCE=1` and a serial device; this is not a metal
graduation claim.

**Authority boundary:** G0.8 is reclassified as A2-local and grants no merge,
release, self-approval, HIL actuation, public support, credential, or
identity-level role authority.

## 2026-06-23 — G0.8.1 makes implementation authority and serial evidence explicit
The G0.8 code result was good, but it called a code-writing trial A1 even though
the authority ladder reserves implementation work for A2.

**Chosen:** Add A2-local as the only implementation authority available to G0:
bounded code and gate changes inside the active work order, without merge,
release, self-approval, HIL actuation, public support, credentials, or identity
role expansion. A3+ remains blocked by the validators.

**Serial evidence hygiene:** `RAMEN_HIL_RUN_ID` is allowlisted before it becomes
part of an evidence path. Empty transcripts fail closed. Evidence JSON now
records `serial_input_kind`. Development log replay emits `PASS/HIL-LOG`; live
serial capture emits `PASS/HIL-APPLIANCE`.

**Authority boundary:** G0.8.1 is A2-local only and grants no merge, release,
self-approval, HIL actuation, public support, credential, or identity-level role
authority.

## 2026-06-25 — Public identity is "evidence-gated OS lab"; governance surfaced secondarily
The README's opening identity is "RamenOS is an evidence-gated OS lab for
agent-native computing." We chose "OS lab" over the prior "Rust OS experiment"
to lead with the testable, evidence-gated nature of the project rather than the
implementation language; Rust remains visible in the workspace table, build
section, and badges. RamenOrg governance and the research program are now
surfaced as a single secondary "Governance and Research" section, kept strictly
parallel to the OS execution track and granting no merge, release, hardware, or
public-support authority on their own — the public hook stays OS-first while the
dual-product reality is stated honestly. This is a positioning decision, not a
technical or constitutional change.

## 2026-09-16 — Persist Store ownership separately from signed artifact manifests

Per-artifact `.ownership.json` records are server-owned authority metadata. Commit
and sync them before exposing an ingested artifact in projection replies. Unknown,
malformed, or conflicting ownership denies access; restart never promotes it to
global. Existing explicitly attributed legacy records remain readable. Existing
unattributed stores require trusted re-ingestion with the intended capability;
there is no automatic global migration. A content ID keeps one owner: cross-domain
re-ingestion cannot reassign it. A future sharing model requires a separate design.
Projection queries intersect their metadata domain with durable artifact ownership.

Legacy ownership recovery treats raw manifest `metadata.domain_id/is_global`
and the ownership directory layout as trusted, administrator-controlled migration
inputs, not authorization data authenticated by the typed artifact signature.
Untrusted callers must not be able to write those records. Current server-owned
`.ownership.json` sidecars take precedence; a corrupt sidecar never falls back.

## 2026-09-16 — Bind HIL evidence to prepared artifacts and a fresh boot challenge

The target emits a random `kernel_build_id` embedded before linking. A host
`provenance.json` binds that ID to the final EFI digest, init digest, profile,
base Git commit, machine, and storage contract. It replaces the impossible
self-referential EFI hash marker. Build scripts track all provenance environment
inputs. Live gate records validate a single complete target boot against the
prepared manifest; graduation additionally requires a nonzero caller-selected
expected nonce. Appliance-mediated claims require a matching run, target,
controller transcript, and exactly the same serial bytes. These are trusted lab
provenance records, not cryptographic hardware attestation. Dirty builds remain
identified by their final hashes and unique build ID; the Git SHA is the base
commit, not an assertion of a clean worktree.

Firmware helpers support a redirected `RAMEN_HIL_EFIVAR_DIR` solely for byte-level
host testing. Physical writes still default to Linux efivarfs and remain explicit
operator actions. No test gate writes real firmware or actuates a target.

## 2026-09-16 — Reconcile existing kernel dependencies and boot test-image scope

The existing `spin` dependency in the kernel is the bounded synchronization
exception; `kernel_api` remains dependency-free. The UEFI boot crate already uses
`uefi` for firmware bindings. This records current implementation rather than
adding dependencies. Additional kernel dependencies need an explicit decision.
Both boot crates currently enable `test_protocols` for Foundry/init test images.
They are bring-up images, not a production/release configuration; removing those
protocols requires a separate production boot consumer and corresponding gates.

## 2026-09-16 — Close shared-memory, WASM, Store, and trace review boundaries

Retain the current common-address shared-memory ABI. Reserve that address in
every recipient and reject collisions rather than overwrite another region.
Repeated mappings in one domain must use identical rights/cache mode and hold
the PTE until the last reference is released. Clear each full allocated frame
before publishing its capability, using the boot identity-mapping contract already
required by both MMU implementations. Preserve NX in x86 entries and enable
EFER.NXE before programming mappings; unsupported CPUs fail closed. QEMU checks
the leaf bit and CPU enablement, not execution-fault recovery (there is no kernel
page-fault test handler yet).

Native WASM modules export `memory`; generated host imports resolve the caller's
export on each invocation. Missing memory or invalid reply ranges return
`InvalidArgument`, and replies are never truncated. The wire/IDL layouts stay
unchanged; host bindings are regenerated from the generator.

Manifest signing payload v1 is compact `serde_json` serialization of the typed
`Manifest` in declaration order with `signatures` omitted. Publishers and readers
use `manifest_signing_bytes`; all other typed fields and array order remain
covered. Whitespace and JSON property order are not signed. Schema changes must
consider this signing contract. Embedded signatures cannot sign themselves.

Store ingestion opens a regular file once without following symlinks or blocking
on FIFOs, hashes the bytes written to an exclusive private staging file, syncs
that file, and atomically renames it into the CAS. Concurrent source writes can
produce a mixed source snapshot, but its content ID always names the exact bytes
staged. Failed/abandoned staging is cleaned on drop; crash-left temporary files
are not published artifacts. This does not add a filesystem snapshot guarantee.

Per-domain trace buffers use the existing `spin` dependency to synchronize buffer
contents and cursors together. This supports concurrent readers/writers without
torn events or mutable global aliases in safe helpers. Interrupt reentry while
holding the same domain lock is unsupported; this is not full kernel SMP/IRQ
graduation. Production kernel allocation policy and dependency boundaries remain
unchanged; aligned backing allocation exists only in host tests.

## 2026-09-16 — Honor the WASM SDK's declared output capacity

The SDK's `out_len` word is a signed i32 capacity on entry and actual byte count
on success. Host bindings capture and validate it before crossing the kernel
bridge: negative values, out-of-memory spans, overlapping output/length ranges,
and insufficient capacity fail with `InvalidArgument` without output writes.
Preflight uses the exact `{message}_reply` IDL type's Rust wire size, the requested
byte count for shared-memory reads, or the envelope payload maximum when no
paired reply type exists. Shared-memory writes return no bytes, so they validate
a zero-byte requirement and set the returned length to zero on success. The final
writer also rejects an unexpectedly large backend reply before changing either
destination. Import signatures and wire layouts are unchanged. Raw WAT callers
must initialize capacity, just as the generated Rust SDK already does.

## 2026-09-29 — Immutable Store publication and bounded WASM guest execution

Store-owned CAS writers serialize publication using the service's existing
registry/index lock. CoW receives that same live ownership registry, resolves
only domain-visible source paths, and checks destination ownership before
publishing. A readable global source may receive a domain-local overlay; global
readability never grants permission to replace the source artifact's metadata.
Fresh artifacts persist ownership before the new projection is exposed. Existing
same-owner content is verified and reused byte-for-byte, including its manifest
and signatures. Unattributed, corrupt, or incomplete pre-existing artifacts fail
closed and require an explicit trusted repair/migration. Ordinary ingestion uses
the same rules. Internal aggregate index snapshots preserve existing CAS metadata
and remain unattributed, so they do not become globally readable. Snapshot CAS
publication precedes working-copy publication; CoW and ordinary ingestion install
their cloned indexes only after persistence succeeds. The final working-copy
rename is followed by parent-directory synchronization. This retains the single
serialized writer model; it does not introduce cross-process writer coordination
or a multi-file transaction.

Native WASM guest execution has a nonzero wall-clock budget (30 seconds by
default), supplied by the launch plan or the native runner CLI. Wasmtime epoch
interruption instruments guest code, including module start sections. Each run
has a monotonic deadline; a shared engine epoch checks each invocation's own
clock so one timer cannot interrupt another run early. The timer is cancelled
and joined on every exit path. Zero/unrepresentable budgets fail closed, and
expiration returns a distinct execution-timeout error. Compilation and blocking
host calls are not preempted by epoch interruption; this is a guest-execution
bound, not full host-process containment. Process watchdogs bound regression
runs independently of the implementation under test.

AArch64 descriptor assembly is a pure architecture helper compiled in host tests
on either architecture. Leaf descriptors include the page-type bit, and both
address and flag updates retain the supported PXN/UXN attributes. Tests inspect
stored descriptors, including execute transitions and address replacement.
This adds descriptor/build evidence, not target fault-recovery or metal claims.

## 2026-09-30 — SW0 contract model before service integration

SW0 Phase A is split into A0 contract fixtures, A1 one RamenOS scripted service
proof, and A2 Linux/control conformance. A0 is a pure model in the schema crate,
not a service or new native wire interface; A1 must define/generate missing
operations through IDL before implementing boundary handlers. A0's synthetic
fixture and trusted executor inputs provide no credential or validation
authenticity. All IO, clocks, grant verification, watchdogs and persistence live
outside the schema module. The existing CoW helper remains a foundation, not
the task's transaction implementation.

Accepted output publication follows immutable staging and successful validation
bound to candidate/validator/schema/policy and task/domain/resource/generation.
Commit checks revision plus content identity to reject ABA, rechecks authority
and validation expiry, and retains the successful request binding and receipt.
An exact retry returns that receipt without another effect; changed request-ID
reuse fails. Receipt access requires current authority; revoked authority must
be renewed before retrieval. Renewed authority requires fresh validation for a
new commit. A0 bounds candidates and successful receipts to 64 each per task;
capacity fails closed without eviction. A1 must atomically persist accepted
reference, revision and receipt, with lost-reply/crash/restart assertions.

A0 declares separate guest, whole-invocation, host-call and diagnostic limits.
A1 uses a supervised worker/watchdog, bounded IPC/memory and explicit cleanup;
epoch interruption alone does not meet the whole-invocation contract. Budget
declarations and reported outcomes passing A0 do not prove timers or containment.

The proposed Phase B ceiling is USD 100, 500 final matched blocks plus the
30-block pilot, and 8 active model-run hours; a funded work order must freeze
actual settings before collection. An unaffordable powered sample becomes an
explicit exploratory report, not a truncated powered trial. S14 depends on a
recorded proceed/defer decision over A1/A2 and that bounded report, H0/H1, and
its own design/IDL/Oracle/gate; a positive comparative outcome is not required.

Physical work awaits test-hardware setup. H3 graduation additionally requires
implemented slot publication/readback, selected-slot boot, and rollback/recovery
with fresh per-boot provenance and artifact identities. Firmware NVMe detection
and A/B metadata alone cannot establish those transitions or native block I/O.

## 2026-09-30 — SW0 A1.0 control contract before service registration

Reserve protocol 14 for the bounded task transaction IDL. A1.0 freezes fixed
control layouts and request syntax; A1.1 must freeze bulk-state serialization,
reply validation and service assertions before registering an endpoint. Do not
add this interface to the current broker registry until task/resource/lifetime
bindings can be enforced. Existing Semantic Store query, projection CoW and
Semantic State prototypes do not provide that enforcement.

Use opaque service-owned task/candidate/receipt/subscription objects and existing
kernel Shmem handles for bulk bytes. Derive caller domain from trusted transport;
never accept a caller-selected domain or validation attestation. Read/stage/
validate/commit/observe rights are distinct. Receipt retrieval requires current
COMMIT authority, consistent with A0 retry semantics; OBSERVE does not reveal
commit receipts. Policy authority is separate from task rights. Revalidate
subscriptions on every delivery, including expiry/revocation, and seal candidate
bytes before validation. A1.0 preflight establishes none of these backend checks.

Retain A1.1's durable transaction and supervised worker requirements. Keep its
host service proof independent of Linux controls, model runs and hardware setup.
The new gate reports `environment=host claim=wire-contract` only.

## 2026-09-30 — SW0 A1.1 bounded host fixture service

Implement the generated task contract in an opt-in Store task service, with a
separate Native Runner worker and strict schema records. Both require
`agent_task_v1_dev`, disabled by default; no production endpoint/broker is
registered. A trusted launcher assigns domains to connected host transports and
provides scoped file-backed mappings. These are named host proof boundaries,
not production authentication, a client process sandbox or target enforcement.
Use a tiny canonical-JSON equality dialect for the first useful fixture; general
schema execution and model-facing adapters are subsequent work.

Store keeps output reference/revision, semantic commit binding, original receipt,
trusted validation and dispatched-call audit in one synchronized journal under
an exclusive writer lock. A failed persistence attempt poisons the service until
recovery. Restart invalidates grants/validation by advancing generation; stable
candidate/receipt locators survive for exact successful retries. A renewed
`task_cap` is permitted for a retry because current authority is checked afresh;
all semantic fields must still match. Locators alone grant no authority. This
clarifies A1.0's blanket generation wording without reviving revoked grants.

The validator has no custom host imports. Its private guest input header goes
through IDL/codegen as reserved type 20, never accepted in task request dispatch.
Its worker verifies CAS hashes and handles JSON normalization; the pinned WASM
program performs the comparison. CAS reads and compilation run inside the outer
watchdog. Two validators per task is the viable initial admission limit; check
caller authority before revealing exhaustion. Linux proves process-group cleanup
and descendant reaping and imposes an address-space ceiling; macOS has a narrower
containment claim. Storage/scheduler hard real-time and physical power-loss
behavior are outside this host gate.

Coalesce subscriptions to one pending event per type, bind delivery to the
issuing connection/current observer grant and use fresh snapshots. Consumers
resynchronize by state read after notification loss. The independent verifier
checks receipt/validation replay and audit hash integrity; it does not establish
operator-resistant authenticity or complete OS-event replay. A2 remains required
before comparative model collection, and physical work stays deferred.


## 2026-09-30 — SW0 A2.1 measured Linux scoped-shell foundation

Land the Linux substrate separately from full A2 conformance. Bubblewrap on
`bigman` cannot initialize the isolated network namespace under current policy;
use its available Docker engine without changing host security settings. Pin an
installed Python image by digest and resolve its immutable ID. CI pulls that
exact digest before the gate; no unavailable control or fallback can report PASS.
Keep daemon/mount selection entirely in trusted tooling, outside the consumer.

Inspect actual namespaces, UID/capabilities/seccomp, mounts and cgroup configuration
and execute forced probes. Share A1.1's development fixture bytes and exact WASM
worker; seal staged bytes into a read-only private CAS subset before validation.
Remove the whole container on every exit and fail on uncertain creation/cleanup.
Record broader shell helpers/metadata, process delegation and open descriptors
surviving mode changes. Do not call these equivalent to RT grant revocation.

The accepted artifact is evaluator evidence, not an LS durable commit receipt.
One development instance is sufficient for this substrate milestone; it is not
a hidden fixture bank or model comparison. Next are shared serializer/RT adapter,
independent LT transactions, LS transaction commands and full all-arm authority
mapping. A2 and Phase B stay pending; physical HIL remains deferred.


## 2026-09-30 — SW0 A2.2 shared JSON codec and opt-in RT bridge

Put the model-facing contract in a backend-free host tooling library, with the
Store bridge/launcher behind `agent_task_v1_dev`. Services do not acquire new
cross-service IO dependencies. Wrap eight existing generated task operations;
no new native authority, raw path, shell or caller-domain field is introduced.
Use canonical string IDs through u64::MAX and bounded padded base64 for bytes.
Backend grants and denials remain authoritative; syntax checks mint no authority.

Publish one tool description and request/response schema artifact for both typed
arms, independently exercised by a scripted executable consumer. Release source
and reply mappings, preserve exact successful commit retry semantics, and never
solve or retry a mutation in the adapter. Suppress legacy library stderr only in
this standalone opt-in launcher; structured stdout remains the model transport
and durable service audit remains in the private journal. Keep transport failure
honest about potentially durable commits and require explicit recovery/retry.

Defer model subscriptions to a separately versioned lifecycle contract. This
bridge has no subscription, hidden event queue or automatic observation calls.
The LT backend must import these descriptions/codec and map the same virtual
resources; its transactions, LS commands, full canonical authority conformance
and hidden-bank partitioning remain next. No paid/model or physical run is begun.

## 2026-09-30 — SW0 A2.3 independent Linux transaction enforcement

Use a separate Python Linux broker behind the existing default-off Rust typed
transport. It owns grants, generation/expiry, private file sealing and a single
synchronized revision/receipt journal; it imports neither RT service enforcement
nor the A0 reference state machine. The shared codec/descriptions remain the model
contract, and direct broker tests independently check authority behind that codec.
No new native interface, production registration or kernel dependency is added.

Reuse the measured Docker substrate for a pinned worker and read-only three-blob
subset. Stream and seal the large debug worker instead of buffering it wholesale.
Invalidate prior validation before a new attempt. Treat IO uncertainty as a poisoned
session; require explicit current-authority receipt lookup/retry after recovery.
Replay receipt revisions independently of JSON key order and retain the original
validation observation with each receipt. Host client isolation and abrupt broker
cleanup remain unproved; this milestone uses trusted scripted consumers.

Compare shared named RT/LT development cases with byte-identical descriptions,
allowing only consistently aliased opaque handles and declared clock/duration
fields. Do not generalize those cases to full equivalence, narrower authority or a
model result. LS transactions, subscriptions, full authority inventories, hidden
fixtures and evaluator session supervision remain next; hardware stays deferred.

## 2026-09-30 — SW0 A2.4 contained shell commands share Linux transactions

Expose conventional task commands inside the measured Linux container, backed by
LT's independent Linux transaction engine. This isolates the Linux interface
contrast; LS does not acquire a second repair/validation/publication implementation.
A read-only task mount contains bootstrap/helper and one private Unix endpoint.
The host broker requires real UID/GID 65534 peer credentials and fixes domain/task
outside request bytes. Model file paths never reach it: the helper opens/bounds
bytes inside the container. This is host experiment tooling, not a native OS API.

Preserve ordinary command stdout/stderr and nonzero exit status explicitly. The
shared substrate's strict nonzero failure remains the default for validators and
existing gates. Persist a pending shell cleanup checkpoint before launch and a
measured removal record afterward, alongside the existing shared container history.
Uncertain create/removal/journal writes poison the session and block restart until
trusted reconciliation. A dispatched commit can finish after a lost reply; never
retry or infer no effect automatically. Transport audit records socket-write success,
not delivery acknowledgment, and remain session-local rather than crash-persistent.

Record the broader shell file/helper/process/delegation envelope, readable policy/
validator pins, actual shell isolation versus the typed scripted harnesses, and
additional checkpoint instrumentation. None establishes equal/narrower authority
or substrate advantage. Shared subscriptions, canonical inventories, hidden-bank/
evaluator/session controls remain next. No model or physical trial begins here.

## 2026-09-30 — SW0 A2.5 explicit typed subscription polling

Use a separately described version 2 model contract with explicit subscribe,
poll and cancel operations. Keep version 1's eight-operation artifact unchanged.
Bound each connection/session to 16 subscriptions, with two coalesced pending
event types each. Poll returns fresh authorized state only when an event is pending;
there is no unsolicited JSON, background observation, initial event or repair policy.
Recheck the original OBSERVE grant and connection/session before lookup/delivery.
Cancellation, generation revocation, expiry and disconnect/restart discard queues.

Add protocol-14 typed message pairs 21–26 through IDL/codegen, retaining existing
push subscribe/events. RT uses the native service's signal path and shared-memory
snapshot lifecycle; LT owns an independent in-memory implementation. Draining is
at-most-once, so a lost poll response requires explicit state resynchronization.
Revocation counts live grants after expiry reclamation. Named comparison permits
only opaque-handle aliasing, declared clock fields and the predeclared expiry
case's redacted expired/denied difference; other outputs must agree.

LS's helper remains eight conventional version 1 verbs. Raw LS clients can reach
the shared broker's version 2 operations with launcher-session lifetimes; record
that available authority and difference in the upcoming all-arm inventory. This
milestone adds finite typed lifecycle evidence, not full protocol/authority
equivalence, durable notification replay or model/target/metal evidence. Continue
with canonical inventories and negative cases, then hidden-bank/evaluator controls.

## 2026-09-30 — SW0 A2.6 finite canonical authority evidence

Freeze a logical tuple universe and collect actual host RT, independent LT and
contained LS observations through one scripted development consumer. Preserve
separate task effects and evaluator probes; compare semantic point results and
negative outcomes without substituting policy intent for available authority.
A rejected virtual-resource grant is a delegate attempt, not a filesystem write;
real unmounted LS canary reads/writes are distinct tuples. Retain actual credentials,
namespace/mount/container data, backend clocks and independent accepted journal/byte
checks. Previously returned observations survive grant revocation, and LS direct
fixture/descriptors and raw session access retain their measured broader lifetimes.

Use maximum observed availability and time-indexed samples as finite artifacts.
Do not label these complete E_max/E(t), infer continuous access, or count handles
as authority. Unmeasured host-client/transitive/unexercised authority stays unknown;
set inclusion and narrower-claim eligibility remain blocked. The reducer rejects
unsupported/tampered claims and any successful or unconfirmed forbidden probe.
All data belong to isolated evaluator runs, not model context or cost samples.

The shared post-commit read case revealed LT returning stale original fixture
bytes while RT read accepted output. Make LT resource A resolve the current
accepted CAS pointer, matching the existing native contract; preserve the original
LS fixture mount as a separate observation. Gate renewal/restart reads accordingly.
Continue with hidden-bank/evaluator session controls and remaining authority
coverage. Full A2, model trials, target enforcement and physical runs remain pending.

## 2026-09-30 — SW0 A2.7 private bank contract and external session controls

Keep development, pilot and final instance identities/initial/target hashes
disjoint. Use five exact-target repair classes crossed with three instruction
conditions per partition, HMAC-derived from an operator seed. A separate oracle
and owner-only private root remain outside model input directories. Public
commitments freeze bank/partition hashes; pilot/final selection requires a trusted
bank-commit/study-bound release. This contract is not a signature or an OS boundary
against the owner. Generate no real hidden bank during tuning or default CI.

Run one actual adapter per fresh arm storage/grant/transcript, with external idle/
whole-session and whole-frame deadlines, request/output/diagnostic/context bounds,
and no implicit retry. Count every supplied visible description, task/bootstrap,
request/reply and final text as role-prefixed UTF-8 transport bytes. This is not
model token accounting. The scripted gate cycles six orders over unique IDs and
retains all 45 attempted arm rows; actual validator deadline responses are failed
tasks with unchanged publication pointers, not successful repairs or removed rows.
Malformed/other task failures fail closed; every arm must demonstrate successful
independently graded publication. Budgets are not widened to hide timeout effects.

Track owned observed process identities/groups, stop them and reap the adapter.
Give LT/LS containers an optional trusted evaluator label for scoped removal and
inventory. Normal EOF/journal removal records plus empty inventory qualify normal
gate cleanup. A forcibly interrupted daemon create may finish later, so even
observed removal cannot certify that run: retain and quarantine uncertainty.
Do not rewrite a poisoned journal or silently resume it. Resolve forced create/
commit reconciliation, broader authority/lifetime/deputy mappings, actual hidden
bank/study releases and provider/token accounting before full A2/Phase B claims.

## 2026-10-01 — SW0 A2.8 named lifecycle receipts and explicit reconciliation

Persist a bounded private intent before each evaluator-contained Docker create,
then its immutable acknowledged ID and verified removal. Fence the scope before
stopping observed processes; forbid new intents but allow an already-issued RPC's
acknowledgement to resolve later. Certify only fenced scopes with every intent
resolved and every acknowledged object removed, empty inventory and no untracked
objects. An empty inventory alone never resolves pending work. Use a controlled
late actual create to exercise that distinction, then verify stale start fails.
This is named trusted host lifecycle evidence, not general daemon/host crash,
power-loss, unobserved escape or malicious-host isolation proof. Keep uncertainty.

LT validators and LS shells persist invocation-linked pending cleanup checkpoints.
Provide a separate trusted, explicit reconciliation function using the matching
ledger proof and existing exclusive writer lock. Repair only cleanup rows and
retain their prior hashes; preserve all other journal fields. Ordinary failed
sessions never rewrite or resume themselves. Restart with fresh grants and look
up original receipts explicitly; do not retry commits. Gate before-dispatch and
after-publication interruptions in all three arms, including an LS shell still
running after durable commit, and the existing native abrupt crash cases.

A smaller durable ledger suffices for these named cases; defer a guardian service
and general daemon restart/recovery contract until evidence requires them. Keep
full A2 authority/lifetime/deputy coverage, real hidden-bank/provider controls,
model comparison and target integration separate. Hardware remains deferred.

## 2026-10-01 — SW0 A2.9 finite issuance projections and lifetime witnesses

Enumerate every nonempty subset of the five declared task rights under two fresh
fixture policies: full (31) and read/observe-only (17), with the same 60000 ms
lifetime ceiling. Bind a separate right-to-canonical-tuple catalog and the existing
A2.6 universe hash. Require exact issued-right echoes and redacted denials; prove
actual subset read/observe access and separately execute useful single-right
read/stage/validate/commit/observe effects using explicit evaluator prerequisites.
Do not equate permission issuance with successful data effects, or compare opaque
locator aliases. Independently grade bytes, pins, accepted references and receipts.

Qualify equality only within this declared-interface issued-right projection.
A commit bit does not bypass validation or revision/content guards. Keep VALIDATE
unexercised in issuance enumeration and demonstrate its effect in a separate
witness. Trace auxiliary probes separately from future model task/cost records.
Whole E_max/E(t), arbitrary policies/resources and host/transitive reach remain
unknown; no narrower-authority score or readiness follows from these finite cases.

Use actual backend clocks to witness short-grant expiry, test every task operation
again after renewal, and revoke the generation. Fresh policy-backed grants remain
requestable, without reviving old grants/subscriptions. Retain raw denied/expired
statuses and normalize only their terminal rejection for this named conclusion.
Demonstrate that LS mounted reads and open descriptors passed to children survive
expiry/revocation; do not erase them from its authority inventory. These are named
points, not continuous scheduling or isolation proof. Continue with remaining
host-client/deputy/unexercised and continuous coverage plus real bank/study/provider
controls. Keep physical testing deferred.

## 2026-10-03 — Shared product vision for humans and AI agents

**Context:** Agent-task work and public introductions had narrowed the description
of RamenOS to an experimental OS for agents. The founder reaffirmed the broader
destination: an everyday, post-Unix OS for humans and AI agents, aiming for fast
execution, hardware adaptability, safety, and ease of use.

**Chosen:** Use `VISION.md` as the shared product direction and align maintained
project, architecture, roadmap, contributor, governance, and research guidance.
Human interfaces and agent interfaces are first-class parts of one product.
Drivers and software should evolve behind typed contracts with isolated execution,
bounded failures, and Foundry checks for effects on consumers. Compatibility is
a useful path while native design remains free to evolve beyond Unix constraints.

Clarify `CONSTITUTION.md` with human usability and policy authority alongside
agent interfaces, and with modularity that acknowledges dependencies and recovery
requirements. Core human interaction remains usable without an AI model. Existing
capability, IDL, kernel/service/Store, and evidence invariants remain in force.

**Consequences:** SW0 is a bounded proof of the agent proposition within the
broader product. S14/S15 retain their human-input and desktop purpose. Current
hardware/software ordering and prerequisites remain intact. Public pre-alpha is
the current stage; performance, full isolation, hardware breadth, and everyday
readiness require matching evidence. Historical decisions, plans, and trial reports
retain their original chronology. This decision adds no implementation evidence.

## 2026-10-03 — Review fixes for source authority, durable publication, and IPC bounds

**Context:** Review reproduced signed-artifact identity substitution, corrupt
GetBlob success, ambient host pathname ingestion, CAS/ownership crash orphans,
IPC calls exceeding native deadlines, uncertain request replay, and LT duplicate
staging drift. Local preflight omitted Linux SW0 gates run by CI, and S11.8/S13.6
claims exceeded their embedded-vector providers' actual evidence.

**Chosen:** Store compares the canonical requested ID with the already
signature-checked manifest and the blob's opened byte stream. The WASM consumer
independently hashes its exact compilation/execution snapshot. Keep the host
client's path API, but open regular sources in the caller and transfer one
SCM_RIGHTS descriptor under a new host message type (7). The service never opens
the label; legacy message 4 closes fail-closed. Native `src_shm_cap`/`src_len` IDL
is unchanged. This preserves large-file workflows without issuing ambient source
read authority to Store write-capability holders.

A Store-owned publication helper durably records canonical identity, exact
manifest, domain and global flag before publishing CAS names. Startup and retries
complete matching valid content or abort an empty intent; corrupt/conflicting
state fails closed. Unattributed content without an intent cannot be adopted.
Existing same-owner artifacts retain their prior manifest/signatures; internal
aggregate projection snapshots remain unattributed.

Both native IPC transports use nonblocking connect and absolute invocation
read/write deadlines, including partial progress. A lost/failed dispatch has an
uncertain effect and is never replayed; the reusable Unix session is poisoned.
Standalone bridges retain bounded per-transaction budgets. No kernel deduplication
or successful rollback of uncertain effects is claimed.

LT staging counts distinct content IDs and preserves duplicate caps/validation.
CI and full preflight share the complete SW0 sequence; missing Linux/Docker/image
or Python requirements mean incomplete proof, not a passing platform skip.
S11.8/S13.6 are contract/shared-memory checks against embedded Oracle vectors.
Device-backed native execution remains work requiring Oracle-grounded gates.
QEMU staging, mutable firmware variables and serial logs are unique per run.

**Consequences:** Updated host clients and services must be deployed together;
there is no unsafe legacy pathname fallback. Recovery trusts the private Store
root's durable intent, not requester assertions, and does not automatically repair
preexisting orphan/corrupt artifacts. These changes add bounded host/QEMU evidence
and no physical, native device-I/O, whole-system isolation, or readiness claim.

## 2026-10-04 — Fresh validation observations and bounded host Store preparation

**Decision:** Preserve the existing RT meaning of `validation_current`: the last
observation matches the generation and has not expired. LT uses that same
predicate for observations and keeps its stronger successful/untruncated/in-budget
predicate for commit. This resolves a shared-interface ambiguity without changing
the versioned schema or admitting invalid candidates.

Ordinary host Store ingestion authenticates before descriptor receipt, then
copies/hashes/synchronizes the caller-opened regular file in a same-binary worker
outside registry/projection locks. The operator configures a byte ceiling
(default 1 GiB), concurrent preparation reservations (4), preparation deadline
(30 seconds), active connections (32) and absolute request/descriptor budget
(5 seconds). A byte ceiling is enforced during copying, including growing files;
reservations remain held through publication and uncertain worker termination.
Disconnect during preparation cancels; deadline sends termination and bounds the
request's reap wait. A kernel-stalled worker retains its reservation for reaping
rather than freeing capacity for unbounded replacements. The helper consumes
inherited descriptors and emits a bounded hash/size receipt; it is trusted host
code, not a new native interface or general sandbox.

Publication reacquires the existing locks and revalidates write authority/access
and owner-bound durable intent. The published bytes are the hashed snapshot.
The default staging ceiling is four GiB in flight, distinct from SW0 limits and
from total retained CAS quota. Local staging creation and durable publication IO
still depend on the filesystem; those operations and crash-orphan staging
reclamation are not certified by the source watchdog. Cancellation after
publication begins retains ordinary lost-reply uncertainty.

**Evidence:** Gate-first transport assertions fail on the prior descriptor
handshake. Host tests now cover two clients during a stalled preparation,
admission rejection, disconnect cleanup, worker timeout/reaping, byte ceilings
and withdrawn authority before publication. RT state delivery and portable LT
predicates are checked locally; expanded paired Linux cases require their
Linux/Docker gate. No model, target-kernel, physical, release or whole-authority
claim follows from these controls.

## 2026-10-04 — Match the COM1 boot console to the HIL serial contract

Use 115200 8N1 for the x86_64 COM1 console, as already specified by
`hardware/hil_appliance_v0.toml` and the serial capture scripts. Correct the
existing divisor from 3 to 1 rather than lowering the controller contract to
38400. COM2's framed IPC configuration remains separate and unchanged.

The S12 GOP gate observes QEMU's `serial_update_parameters` protocol trace and
requires the final UART state to be 115200 8N1 alongside successful boot/GOP
output. QEMU's file-backed serial channel does not enforce baud matching, so
readable boot logs alone previously missed the mismatch. The assertion fails
on the original kernel's observed 38400 8N1 state.

Reference evidence for this bounded existing-console correction is retained in
`out/evidence/serial_console_reference_vault/`: hash-pinned QEMU UART model
sources, emulator version, and the pre-fix protocol trace. This is QEMU model
and target-initialization evidence, not a Linux Oracle capture, a new driver
Harness qualification, or physical UART evidence. First live Pi/ThinkCentre
capture and metal graduation remain pending.

## 2026-10-04 — Dependency-driven roadmap and coordinated parallel development

**Context:** The founder requested the fastest practical path through a coordinator
and simultaneous sub-agents. Requiring a paid SW0 comparison and a live HIL loop
before any input/desktop development serialized work that can be designed and
tested independently. Duplicated skill files and active-task wording also raised
the cost of handing work between agents.

**Chosen:** Prioritize a thin integrated human task and dispatch bounded work from
`NEXT_TASKS.md` by actual dependencies. S14/S15 contracts, host consumers, replay,
and QEMU work do not depend on the SW0 Phase B report or physical graduation.
Target execution still requires the relevant loader/runtime and real service
boundaries; device implementation still requires a Reference Vault, Oracle
`protocol_trace`, typed IDL and Foundry assertions. Physical input integration
requires the H0/H1 observation/actuation loop, controller-specific evidence, and
explicit actuation authority. H2/H3 and the two-boot storage graduation protocol
retain their requirements. Simulated input and host persistence cannot stand in
for target input or native storage in an integrated claim.

SW0 remaining authority and study-control work may proceed in parallel against
agreed contracts. Named Phase C target enforcement can proceed independently of
Phase B when its actual target contracts, runtime and gates are available.
Complete A2 conformance, frozen study/provider controls, and a
funded work order still precede model collection. The bounded report informs
agent-specific decisions; a positive comparative result is not a desktop
prerequisite. No scientific acceptance, funding limit, or evidence standard is
weakened. Research blocks only work that consumes its unresolved design decision.

One coordinator selects ready packets, owns shared interface allocation/codegen,
build registration and planning integration, and reserves shared validation
resources. Sub-agents receive bounded file scopes, prerequisite artifacts,
consumer/gate acceptance and evidence limits. Independent review and affected
consumer gates precede integration completion. Work-in-progress follows available
agent/review capacity; the queue's initial three-worker allocation is a default,
not a requirement to run every lane at once. Delegation adds no merge, release,
self-approval, paid-service, physical-actuation or public-support authority.

Keep stable rules in `AGENTS.md`, detailed coordination in `docs/AGENTIC_WORKFLOW.md`,
the ready queue in `NEXT_TASKS.md`, and landed evidence in `CURRENT_STATUS.md`.
Canonical project skills live under `.agents/skills/`; `.claude/skills/` links to
them. Preserve gate-bound historical paths and packets, label their scope, and
prune redundant active instructions. The drift gate checks agent routing to the
planning owners instead of requiring a duplicate active-task label.

**Supersedes:** Only the S14 sequencing clause in the 2026-09-30 “SW0 contract
model before service integration” decision and the retained-order clause in the
2026-10-03 “Shared product vision” decision. The Constitution, implementation
contracts, historical results, and authority boundaries remain in force.

**Consequences:** Software work can advance while lab access or study funding is
pending. Early integration may expose missing runtime, driver, or storage work;
record it as a dependency rather than weakening a gate. This is a scheduling and
documentation change, not a speed measurement or new OS/hardware readiness claim.

## 2026-10-04 — First desktop consumer and offline usage accounting

Accept the independently reviewed [desktop v0 design](docs/plans/desktop-v0.md)
as UI0 design completion. Use a bounded Rust ASCII artifact editor, keyboard-only
trusted launcher/recovery chrome and CPU-rendered shared-memory surfaces for the
first human task. Choose a static `no_std` x86_64 ELF64 application for target
execution; its loader, authenticated user-mode traps, private memory and actual
interrupt deadline are implementation prerequisites. Existing init bytecode and
host Wasmtime do not satisfy target execution. The compositor owns reserved
chrome pixels and the confirmation route; an app can still imitate UI inside its
own region, so no general anti-spoofing claim follows.

Split the first executable host packet (UI1.0) at permission preview, single-use
confirmation, exact grants and instance lifetime with a real non-rendering
witness. Editor/surface/Store behavior follows in UI1.1; target and persistence
joins retain their distinct gates. Register concrete typed fields/rights and
initial useful/denial/failure assertions before handlers. The accepted design is
not a desktop implementation, an IDL allocation or an executable gate.

For SW-E, freeze an accounting plan separately from the externally frozen
comparison study. `accounting_study_sha256` hashes the entire plan, including
the opaque release study digest, avoiding circular hashes or excluded fields.
The portable ledger prices two trusted uncached-token categories with integer
micro-USD estimates and retains failed, unknown, over-budget and pending rows.
Incomplete usage cannot certify the declared ceiling. This is deterministic
offline accounting, not a statistical study freeze, provider attestation,
invoice or authority to spend. Actual provider capture/supervision and independent
private-bank/funded-run controls remain prerequisites for model collection.

## 2026-10-04 — Attribute finite host-file authority to the consumer process

Measure one unrelated owner-only canary at three lifetime points for each arm.
RT/LT observations name the actual trusted Python evaluator/host consumer that
issues Session calls; LS observations name the contained Python consumer. Actual
PID/UID/GID and mount/PID/network namespace identities bind those actors. The
canary lives outside inspected task mounts and retained evidence contains only
its hash or recognized denial errno.

Bind the accompanying successful and denied reads to the issued/revoked handles,
exact generations and the same logical resource, including a successful renewed
read. Independent review found that mismatched handles or resources could otherwise
attribute an ordinary scope denial to a lifetime transition; negative regressions
now reject those traces.

This is a finite observation of consumer authority, not adapter or model-interface
authority. The Linux/Docker gates pass, while whole-authority inclusion, continuous
lifetime certification, noninterference, target enforcement and full A2 remain
unproved. It grants no spending or provider-collection authority.

## 2026-10-04 — Execute the first desktop grant/lifetime contract on a host

Register protocol 336 for `portal.desktop_session` version 1 and gate its exact
17-case inventory before handlers. The `desktop_v0_dev` feature enables a trusted
Unix host broker, private verified executable snapshot and real Rust witness.
An opaque per-registry context binds chrome/child endpoint classes to owner,
session and instance identity. One synthetic Enter event authorizes exactly one
plan; the child receives only its own observation endpoint.

Separate pipe IO from the Child-owning supervisor and capture the absolute deadline
before spawn. Kill and reap independently of later service calls. Reserve 16 total
instance slots, including live children, so all eventual terminal evidence fits
without eviction; stop and reap at the 64-exchange bound. Clamp controlled time
under the enforcing lock for expiry and TTL origins. Independent review exposed a
backward-time confirmation inconsistency, now covered by a failing-then-passing
regression.

The gate retains actual PID/hash/bootstrap/canonical exchanges and independent
OS disappearance evidence, with strict named-case and artifact checks. Cache only
the immutable expected test-witness identity to remove redundant test setup from
the responsiveness measurement; service-side verification remains independent.
Preserve earlier failed runs. Passing host fixtures do not provide scheduler
latency guarantees, process containment, native kernel enforcement, real keyboard
routing, display/editor/Store behavior or target execution. UI1.1 and RUN0 retain
those separate contracts and evidence joins.

## 2026-10-04 — Validate boot-pool admission before changing firmware ownership

Land an allocation-free numeric selector and its 17 gate-first assertions before
wiring the legacy UEFI handoff or allocator. Expand the maintained boot map to
256 descriptors and preserve a private sticky flag on lost insertion. Public
count changes cannot erase overflow evidence; the selector checks completeness
and count before indexing. Validate every descriptor, including non-candidates,
and require all seven retention reasons inside non-usable descriptors. Compare
complete eligible intervals before clipping to the existing 131072-frame limit,
with stable lowest-base ties and no descriptor merging or rounding up.

The actual exit result, raw EFI classification, complete retained-object sources
and CPU mapping observations remain adapter responsibilities. Synthetic stage
and retention records cannot supply those observations. In particular, a raw
RAM descriptor and low physical address do not establish readable identity
mapping for the first page-table dereference. Obtain a bounded Reference Vault/
Oracle profile before hardware glue, then separately prove actual final-map
ownership, safe allocation/write/read/reuse and the S8 consumer in QEMU.

Independent review accepted the pure source and strict inventory/evidence gate;
focused kernel consumers, both target/UEFI builds and affected integration gates
pass. Keep their claims separate from the earlier strict Linux preflight for
`a44993e`. No firmware transition, allocator installation, target execution or
physical-machine claim is added by this prerequisite.

## 2026-10-04 — Split editor evidence and order saves against authority retirement

Keep protocol 336's observation-only host witness unchanged. Prepare a separately
allocated editor contract with four bounded joins: volatile in-process editor,
real Store-owned selection/receipt transaction, integrated human task, and actual
host editor process. Registry-checked shared-object leases and a logical keyboard
corpus provide host boundary evidence; they do not establish raw mapping
revocation or native controller execution. Freeze the concrete shared contracts
and exact assertion inventories before dependent handlers.

Independent review found that a watchdog cannot cancel journal rename/fsync
already in progress. Order one immutable per-object commit permit against
revocation/deadline under a short state lock before irreversible IO. Without a
permit, paused work cannot publish. A prior permit can finish only its bound
original transition, with Unknown and mutation quarantine until receipt
reconciliation. Do not infer noncommit or automatically retry. Require prior
writer quiescence and journal validation before service-epoch recovery. Gate both
sides of this boundary while keeping unrelated objects usable.

The companion Oracle proposal uses a distinct CPU-inspection trace rather than
forged PCI/MMIO events. External hardware-breakpoint observation and bounded RAM
reads prepare initial-access evidence; they do not execute guest loads/stores or
prove firmware exit. The actual relocated EFI entry resolution is still a required
input before capture. Review accepts these as proposed dependency packets, not
runtime, Store durability or target qualification evidence.

## 2026-10-04 — Register editor wire types and separate build from server readiness

Allocate input 802, focus 832, surface 833, editor session 352 and artifact 368
according to the reviewed UI1.1 proposal. One writer authored the canonical IDLs;
the coordinator registered and generated all five modules, and independent
review checked all 43 messages against their definitions. Keep protocol 336
unchanged. Generated types prepare consumers but do not enforce codecs or
authority. Freeze opaque peer/lease APIs, the keyboard corpus and executable
RED assertions before UI1.1 handlers.

A fresh strict Linux preflight exposed a Store S0 smoke-gate cold-start failure:
its ten-second readiness budget included compiling the server. Compile
synchronously, fail closed unless Cargo reports success and exactly one absolute
non-test server executable, then launch that executable and own its actual PID.
Preserve the readiness budget, explicit development mode and task assertions.
Independent parser regressions and a real cold Linux build with a deliberate
twelve-second delay verify the boundary. This changes gate reliability, not Store
runtime authority or target persistence.

## 2026-10-04 — Freeze host editor authority and observable race assertions

Accept the independently reviewed UI1.1a shared API and logical US-key fixture.
Keep producer pressed/modifier state distinct from focus delivery state: consume
old-focus releases without forwarding them, and require an actual Enter release
before a fresh confirmation press. Reserve the concrete surface identity at
confirmation and backing buffers at Create. Checked copy leases retire every
writer alias before Present/Consume; they are host registry enforcement.

Freeze explicit endpoint-class/bit/operation meanings and bounded observable
pause entry/settlement before implementing race tests. Save claims retain live
per-object admission state, sharing the retirement mutex with permit admission;
snapshot permissions cannot admit a later effect. Owned live SaveStatus and
original recovery reads may reconcile Unknown without replay. Retired or foreign
new requests expose no operation data.

Pin an upstream public-domain ASCII bitmap font without a runtime dependency,
convert its documented bit order once, and independently prepare old/new crop
digests and samples before handlers. These are assertion inputs, not rendering
proof. The 13 gate-first cases and scoped implementations are the next packet.

## 2026-10-05 — Bound the editor Store transaction profile and recovery witnesses

Choose a default-off Store-owned Artifact368 consumer for UI1.1b, with one private
CAS namespace and one IO writer per selected object. Two sessions selecting the
same object share admission state; unrelated objects have separate workers. Use
existing Store publication and ownership helpers within this finite profile.
Cross-object deduplication, shared-root locking and general quota integration
remain successors. UI1.1c separately freezes the authoritative live admission
bridge from the desktop; b does not reuse a copied volatile authorization verdict.

Keep selection and the original receipt in one bounded whole journal. Fenced
reopen requires a joined supported writer and a private retained owner witness
covering durable allocations, submitted/permit bindings, transitions, counter
minima and issued permit/closure state. Preserve all sixteen operation records
across reopen and durably reserve fresh checked ID ranges before exposure.
Authenticate bounded pending CAS intents, manifests and ownership against the
permitted operation or initial provisioning binding before calling helpers that
automatically recover publications. Corrupt or unauthenticated input cannot gain
metadata effects through recovery. Exact schema/service signatures and seven
reviewed RED assertions precede implementation; this design supplies no cold-start
anti-rollback, device-flush or target persistence evidence.

Freeze the exact UI1.1b schema/service API before assertions. After durable
Submitted metadata completes, a pre-permit pause releases namespace IO ownership
while retaining active admission. Sticky timeout closure releases that unpermitted
admission; a fresh serialized metadata owner persists its bound noncommit and new
allocation. The old dispatcher cannot publish a stale journal snapshot. One
open Submitted or unsettled issued permit reserves mutation admission per object.
Earlier metadata IO pauses still require supported settlement.

Public response completion releases only the endpoint invocation guard. Every
held dispatcher/supervisor remains charged to the combined 64-producer limit until
actual join, alongside at most two object IO workers. Opaque quiescence tokens
expose bounded diagnostic fence snapshots with actual join identities and retained
state minima. A checked reserved epoch is distinct from the actual reopened epoch;
new-owner evidence binds its real journal publication to the prior fence digest.
