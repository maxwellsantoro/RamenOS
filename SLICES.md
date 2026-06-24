# Vertical Slices

**Last Updated:** 2026-06-24
**Status:** Reference summary

A slice delivers a usable capability across boundaries: an OS behavior or typed
contract, a real consumer, and a Foundry gate. Detailed chronology belongs in
[CHANGELOG.md](CHANGELOG.md).

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
| S11 | virtio-net Driver Factory MVP | Complete |
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
- Add bounded, fail-safe power/reset actuation.
- Run S12 and S13 graduation using appliance evidence plus target evidence.

## S13 Graduation Boundary

The QEMU Driver Factory path is landed:

- `harness.block` IDL and storage contract.
- virtio-blk initialization and sector Oracle traces.
- Replay scoreboards and runtime block I/O.
- NVMe boot and atomic-update gate scaffolds.

S13 is not complete until Tier-1 hardware produces the required live NVMe and
two-boot rollback evidence. Default `just s13` success is `PASS/QEMU`, not
`PASS/METAL`.

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
