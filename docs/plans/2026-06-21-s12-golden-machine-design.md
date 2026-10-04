# S12: First Metal — Golden Machine

**Last Updated:** 2026-10-03
**Status:** Machine contract and probes landed; physical graduation pending

S12 qualifies UEFI boot, visible GOP output, serial evidence, and IOMMU inventory
on a pinned Tier-1 machine. QEMU probes and physical setup are preparation for
that result. Default CI requires no physical target.

## Reference machine

**CHOSEN:** Lenovo ThinkCentre M900 SFF, machine type 10FH/model 00SNUS,
Core i7-6700, 8 GiB RAM. Its populated RS-232/DB9 port and integrated graphics
support the planned serial/GOP path. S12 starts on the installed 240 GB SanDisk
SATA SSD; a compatible M.2 2280 PCIe NVMe remains required for S13 graduation.

[golden_machine_v0.toml](../../hardware/golden_machine_v0.toml) is the
machine-auditable contract. [Current Status](../../CURRENT_STATUS.md) owns
installed inventory; [Next Tasks](../../NEXT_TASKS.md) owns live-run sequencing.
The Pi/adapter/null-modem chain is installed, while firmware/AMT preflight and
first live serial capture remain pending.

Before physical evidence, check UEFI/USB boot, GOP on integrated graphics, serial,
and VT-d. Provision Intel AMT 11 on the trusted wired lab network and keep its
credentials outside evidence. Front-panel relay/PDU fallback is deferred until
AMT testing identifies a concrete recovery gap.

## Tier-1 contract

| Capability | Evidence required |
|------------|-------------------|
| UEFI boot | Prepared RamenOS `BOOTX64.EFI` identity and live boot |
| Serial | Fresh target transcript correlated with the run |
| GOP framebuffer | Probe/fill and target mode markers |
| IOMMU | ACPI DMAR/VT-d inventory on the reference profile |
| PCIe/NVMe | Inventory in S12; boot/storage evidence in S13 |
| USB xHCI | Inventory in S12; typed input in future S14 |

IOMMU presence does not establish configured DMA containment. Tier-2 profiles
may later declare degraded trust but cannot redefine the Tier-1 requirement.
A second x86 profile and ARM64/SMMU qualification remain later work.

## Landed phases and gates

| Phase | Implemented path | Recipe |
|-------|------------------|--------|
| S12.0 | Manifest, contract, inventory/negative assertions | `just foundry-s12-golden-machine-s12-0` |
| S12.1 | UEFI GOP mode/fill probe; QEMU OVMF assertion | `just foundry-s12-gop-probe-s12-1` |
| S12.2 | Physical boot gate and USB image tooling | `just foundry-s12-hil-boot-s12-2` |
| S12.3 | IOMMU inventory probe/gate | `just foundry-s12-iommu-inventory-s12-3` |
| S12.4 | Appliance manifest/evidence and serial-observer tooling | `just hil-appliance` |

`tools/ci/foundry_s12_golden_machine_s12_0.sh` consumes this stable design path.
`just s12` runs golden-machine inventory, GOP/QEMU, and appliance checks.
`just s12-hil` selects the physical boot/IOMMU legs; they require their documented
opt-in inputs and prepared target. See the [justfile](../../justfile) for recipes
and scripts rather than inferring coverage from a slice number.

## Graduation and claim boundary

Full S12 completion requires the pinned contract, passing GOP probe, physical
boot/GOP evidence on the Tier-1 reference, and matching IOMMU inventory. Fresh
provenance must bind the prepared artifact and target run according to
[Evidence Levels](../../EVIDENCE_LEVELS.md). Default inventory/QEMU success and
an installed cable do not establish that physical result.

The [HIL Appliance Controller](2026-06-22-hil-appliance-controller.md) is the
preferred observation/actuation path. Standalone golden-machine runs retain a
separate stamped claim path. The appliance wraps target/per-gate evidence and
cannot replace it.

[S13](2026-06-21-s13-persistent-storage-design.md) owns NVMe boot and the required
update/rollback protocol. USB/HID, native display/compositor, full IOMMU
programming, and user-space driver containment require their own later gates.
