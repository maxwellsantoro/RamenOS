# AGENTS.md
Coding Agent Instructions

**Last Updated:** 2026-06-23
**Status:** Authoritative

## 0) Mission
Implement the OS via vertical slices.
Do not build large subsystems in isolation.

Every change must:
- improve boot/run behavior OR
- implement a defined IDL contract OR
- add a Foundry gate/test OR
- implement a Store feature that consumes an OS capability.

Historical baseline (completed): Slice S0
Boot + IPC + Capabilities + Tracing stub in QEMU.

Current active execution track (authoritative):
- **Now:** S12.4 HIL appliance v0 physical loop: serial observer first, then power/reset actuator. S13 metal HIL graduation runs through the appliance once that loop is stable.
- **Planning pair:** `CURRENT_STATUS.md` + `NEXT_TASKS.md` (deferred decisions in `ROADMAP.md` §13)
- **Parallel project-control track:** G0 RamenOrg / research-backed OS scaffold (`docs/plans/2026-06-23-research-backed-ramenorg.md`) is allowed only as docs + Foundry governance gates until higher authority is explicitly granted.

Recent completed milestones: S12.4.0 HIL appliance scaffold, S13.6 runtime harness.block I/O in QEMU, S13.4-S13.5 block sector Oracle, S13.3 block replay scoreboard, S13.2 virtio-blk Oracle capture, S13.0 persistent storage contract, S12.3 IOMMU inventory, S12.2 physical HIL boot gate, S12.1 UEFI GOP probe in QEMU OVMF, S12.0 golden machine contract scaffold, S11.8 runtime harness.net packet I/O in QEMU (S11 COMPLETE), S11.7 live hardware packet RX via kernel netdev, S11.6 live packet Oracle capture, S11.5 packet I/O distillation, S11.4 init driver, S11.3 live Oracle capture + trace translation, S11.2 Driver Factory replay scoreboard, S11.2-pre IDL/Wire Contract Integrity Gate, S11.1 Oracle capture scaffold, S10.5.2 QEMU IPC bridge, S10.2 v1.1 cap-filtered snapshots, S10.5.1 broker/proxy harness bridge, S10.5.0 QEMU semantic snapshot.

## 1) Non-negotiables
- Rust-first kernel and core services.
- POSIX is compatibility-only (do not design native APIs around POSIX).
- No ioctl-like escape hatches in native interfaces.
- Capability validation for fast-path operations must be kernel-side; user-space brokers are for grant decisions.
- Split control plane (typed messages) from data plane (zero-copy shared memory).
- Preserve boundaries: kernel ≠ services ≠ store.

## 2) Slice S0 Definition of Done (historical baseline)
S0 is done when:
- x86_64 QEMU boots to an init component and prints a log banner.
- init and kernel can exchange ping/pong via IPC.
- kernel emits structured trace events to a ring buffer.
- Foundry gate script runs QEMU and asserts:
  - boot banner
  - init "hello"
  - ping/pong success
- Repeat for aarch64 QEMU.

## 3) Work style
- Prefer small diffs with tests over big refactors.
- Update CURRENT_STATUS.md and CHANGELOG.md per milestone.
- Any new interface must be added to /idl and code-generated.

## 4) Execution order (historical baseline; completed)
A) Make QEMU boot work for x86_64 (minimal kernel entry + linker + serial output).
B) Add init component that prints "hello" (can be a baked-in payload initially).
C) Implement IPC v0 ping/pong (use kernel_api::ipc types).
D) Add trace ring buffer + a user-space read path.
E) Implement Foundry gate script that runs QEMU and greps output.

## 4b) Current near-term sequencing (active)
A) S12.4.1 HIL appliance serial observer (`tools/hil/appliance_capture_serial.sh`, `RAMEN_HIL_APPLIANCE=1 just hil-appliance`).
B) S12.4.2 HIL appliance power/reset actuator (`tools/hil/appliance_press_power.sh`, `tools/hil/appliance_press_reset.sh`, controller evidence JSON).
C) S13 metal HIL graduation through appliance-mediated live capture (`RAMEN_HIL_APPLIANCE=1 RAMEN_HIL_GRADUATION=1 just s13-hil`).
D) Keep default gates green: `just s12`, `just s13`, `just s11`, and `just foundry-org-governance-g0` when touching org/research planning.
E) Use `CURRENT_STATUS.md` for landed state and `NEXT_TASKS.md` for the next executable task. Do not treat `ROADMAP.md` as operational truth.

## 5) Guardrails
- Keep arch-specific code in kernel/arch/.
- Avoid dynamic allocation in kernel until mm is stable.
- Keep IPC message formats typed and versionable (kernel_api).
- Don’t invent “temporary hacks” that violate the Constitution.

## 6) If blocked
Pick the simplest viable default and record it in DECISIONS.md.
Do not stop for “perfect design.”

## 7) AI & Foundry Workflow
- **When building a driver:** Do not attempt to write hardware interactions from scratch based on pre-training. You must request the **Reference Vault** and the `protocol_trace` artifacts first. Your goal is to write Rust code that produces an identical trace to the Oracle.
- **When porting applications:** Rely on the `observed_caps_v0` artifact to generate the exact minimal capability manifest required. Do not guess what an app needs; measure it.

## 8) RamenOrg & research-backed work
- RamenOS is a research-backed OS, not a research OS. Research is first-class only when it is tied to a product risk, claim boundary, evidence plan, and implementation landing path.
- RamenOrg governance work must use bounded artifacts (`WorkOrderV0`, `HandoffPacketV0`, `BoardVoteV0`) and must not grant merge, release, hardware, or public-support authority without an explicit later decision.
- Do not let the same agent write, approve, merge, and announce a material change.
- For agent-facing or cross-domain service-boundary designs, separate request authority (`Lang`: what a holder may ask) from observable authority (`ObsContract`: what a holder may learn). Request minimization is action safety, not output safety.
- Do not claim hidden-affordance noninterference, metal graduation, security, or release readiness without the matching evidence level and claim boundary.
