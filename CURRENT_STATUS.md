# Current Status

**Last Updated:** 2026-06-24
**Status:** Active and authoritative for landed state
**Current Slice:** S12.4 HIL appliance v0 physical loop

## Active Execution Track

S12.4 is building the physical HIL appliance loop: serial observation first,
then power/reset actuation. Once that loop is stable, the preferred S13 metal
HIL graduation path runs through the appliance on Tier-1 or lab hardware.
Standalone golden-machine graduation remains a distinct `PASS/METAL` path only
when per-gate evidence stamps `claim_path: operator-golden-machine`. S14 USB
xHCI and HID stays deferred until the appliance loop is proven.

The next executable step is maintained in [NEXT_TASKS.md](NEXT_TASKS.md).
Medium-range sequencing and deferred decisions live in [ROADMAP.md](ROADMAP.md).

## Evidence Boundary

| Area | Current evidence | What remains |
|------|------------------|--------------|
| S11 Driver Factory | Complete; `just s11` | Broader device coverage is future work |
| S12 golden machine | QEMU probes and HIL gate scaffolds landed | Appliance-mediated live capture and physical graduation |
| S13 storage | QEMU Oracle, replay, and runtime block I/O landed | Live NVMe boot plus two-boot atomic rollback evidence |
| S12.4 appliance | Manifest, evidence schema, gate, and serial-observer scaffold landed | Stable live serial capture, then controlled power/reset |
| G0 RamenOrg | Governance schemas, packets, validators, trials, and gate landed | Research packets and stronger identity-level role separation |

`PASS/QEMU` is not metal evidence. `PASS/HIL-LOG`, `PASS/HIL-LIVE`,
`PASS/HIL-APPLIANCE`, and `PASS/METAL` have distinct provenance requirements;
see [EVIDENCE_LEVELS.md](EVIDENCE_LEVELS.md).

## Landed Milestones

### S12 and S13

- S12.0 golden-machine contract and Intel NUC Tier-1 profile.
- S12.1 UEFI GOP probe in QEMU OVMF.
- S12.2 physical HIL boot gate scaffold.
- S12.3 IOMMU inventory probe and gate.
- S12.4.0 HIL appliance manifest, evidence wrapper, and inventory gate.
- S12.4.1 serial-observer scaffold with run-id validation, empty-transcript
  rejection, and replay/live evidence separation.
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
