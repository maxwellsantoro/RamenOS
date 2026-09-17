# DECISIONS (ADR-lite)

**Last Updated:** 2026-09-16
**Status:** Active

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
