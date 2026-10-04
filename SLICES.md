# Vertical Slices

**Last Updated:** 2026-10-03
**Status:** Reference summary

A slice delivers a usable capability across boundaries: an OS behavior or typed
contract, a real consumer, and a Foundry gate. Detailed chronology belongs in
[CHANGELOG.md](CHANGELOG.md).

Slices build toward the [Vision](VISION.md) of an everyday, post-Unix OS for
humans and AI agents. Hardware, human interaction, agent authority, Foundry,
and Store outcomes all contribute; a completed foundation slice does not imply
whole-product readiness.

## Slice Index

| Slice | Outcome | State |
|-------|---------|-------|
| S0 | Dual-architecture QEMU boot, IPC ping/pong, and trace baseline | Complete |
| S1 | Content-addressed Artifact Store and first native package path | Complete |
| S2 | Gate-first compatibility-domain boot and boundary checks | Complete |
| S3 | Portals, observed capability profiles, and Driver Capsule v0 | Complete |
| S4 | Vote-to-port queue and prerequisite graph | Complete |
| S5 | Port-It-Now policy proposal and graduation artifacts | Complete; orchestration deferred |
| S6 | Domain Manager and expanded portal suite | Complete |
| S7 | GPU quarantine scaffold and security hardening gates | Complete |
| S8 | Shared-memory control/data planes and ring-buffer foundation | Complete |
| S9 | Store, runner, trace-isolation, and access-control remediation | Complete |
| S10 | Native runner, Semantic State, projection storage, execution fabric, and QEMU bridge | Core phases complete |
| S11 | virtio-net Driver Factory MVP | Oracle/replay and harness vectors landed; native device I/O pending |
| S12 | First-metal golden machine and HIL appliance | Active at S12.4 |
| S13 | Persistent storage from Oracle capture to metal graduation | QEMU loop complete; metal pending |
| S14 | USB xHCI and HID interactivity | Deferred design pass |
| S15 | Native compositor and desktop integration | Future |

## Current Slice: S12.4

**Goal:** Make physical HIL repeatable through a dedicated appliance rather than
manual serial-log handling.

Landed:

- S12.0 golden-machine contract and Tier-1 profile.
- S12.1 GOP probe in QEMU OVMF.
- S12.2 physical boot gate scaffold.
- S12.3 IOMMU inventory.
- S12.4.0 appliance manifest, evidence wrapper, and inventory gate.
- S12.4.1 serial-observer scaffold.

Remaining:

- Prove stable live serial capture through the appliance.
- Provision and validate bounded Intel AMT 11 power/reset actuation from the appliance.
- Begin S12 on the installed SanDisk SATA SSD; add M.2 NVMe before S13 graduation.
- Run S12 and S13 graduation using appliance evidence plus target evidence.

## S13 Graduation Boundary

The QEMU Oracle/replay and harness-vector validation path is landed:

- `harness.block` IDL and storage contract.
- virtio-blk initialization and sector Oracle traces.
- Replay scoreboards and typed harness transfers against embedded Oracle sector vectors.
- NVMe boot and atomic-update gate scaffolds.

Native device-backed virtio-net/virtio-blk harness execution remains a separate
evidence requirement; these vector gates do not prove native device I/O.

S13 is not complete until Tier-1 hardware produces the required live NVMe and
two-boot rollback evidence. Default `just s13` success is `PASS/QEMU`, not
`PASS/METAL`.

## Planned Software Integration

The independent SW0 software lane has implemented A0's pure contract model,
A1's scripted host service task, and bounded A2 Linux adapters and controls for
the [Agent Task Proof](docs/plans/2026-09-16-agent-task-proof.md). Remaining
authority coverage and study controls live in [Next Tasks](NEXT_TASKS.md), with
exact landed scope in [Current Status](CURRENT_STATUS.md). SW0 does not wait
for hardware graduation and is not a new S-number, completed slice, or agent
performance result. S12.4 remains the active physical slice. S14 requires a stable
H0/H1 loop, reviewed A1/A2 evidence, a recorded decision on the bounded Phase B
report, and its own design/IDL/Oracle/gate plan. Physical runs await hardware setup.

## Definition of Done

Every new slice or sub-slice must include:

1. A bounded behavior or typed IDL contract.
2. Kernel-side capability validation for fast-path operations where applicable.
3. A consumer that crosses the intended ownership boundary.
4. A deterministic Foundry gate with negative cases.
5. Evidence terminology that matches [EVIDENCE_LEVELS.md](EVIDENCE_LEVELS.md).
6. Updates to [CURRENT_STATUS.md](CURRENT_STATUS.md) and
   [CHANGELOG.md](CHANGELOG.md) when the milestone lands.

## Historical Detail

Completed implementation plans and investigations are retained under
[docs/archive/plans](docs/archive/plans/). Active and gate-bound plans are listed
in [docs/INDEX.md](docs/INDEX.md).
