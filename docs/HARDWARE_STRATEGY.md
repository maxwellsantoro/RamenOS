# Hardware Strategy

**Last Updated:** 2026-06-22
**Status:** Active

To avoid eternal VM purgatory and "snowflake SoC" death, RamenOS targets specific hardware profiles.

## Tier-1: The Golden Platform (PC-Class)
Our primary target for bare-metal graduation. To be Tier-1, the hardware MUST support:
- UEFI Boot
- PCIe enumeration
- NVMe storage
- USB xHCI
- **A working IOMMU** (VT-d, AMD-Vi, or ARM SMMU)

*Strategy:* We optimize for one specific x86_64 machine first (e.g., an Intel NUC or Framework laptop), followed by one PC-class ARM64 machine. IOMMU is strictly required so that user-space drivers are safely sandboxed in silicon, not just software.

**S12 reference (2026-06-21):** Intel NUC 12/13 class is the pinned Tier-1 golden machine. See `hardware/golden_machine_v0.toml` and `docs/plans/2026-06-21-s12-golden-machine-design.md`.

## HIL Appliance Controller
A Raspberry Pi-class controller is the preferred always-on lab appliance for physical development. It is **not** a RamenOS target and is **not** part of the target TCB. It observes and actuates the golden machine so agents can run bare-metal loops without manual reboot/cable/log work.

Minimum appliance duties:
- serial capture from target COM/DB9/header through a USB RS-232 adapter;
- power/reset actuation through relays or opto-isolated switches;
- timestamped evidence bundle generation;
- later KVM-grade HDMI capture, USB HID injection, and virtual boot media.

The S12.4.0 scaffold gate (`tools/ci/foundry_hil_appliance_s12_4.sh`) protects the docs/manifest/evidence-schema contract in normal CI. The next physical implementation work is S12.4.1 serial observation followed by S12.4.2 power/reset actuation.

Electrical rule: Pi GPIO UART is 3.3V TTL only. Do not connect Pi GPIO directly to PC RS-232/DB9. See `hardware/hil_appliance_v0.toml`, `docs/plans/2026-06-22-hil-appliance-controller.md`, and `tools/ci/foundry_hil_appliance_s12_4.sh`.

## Tier-2: Lab & Outreach (SBCs)
Devices like the Raspberry Pi.
*Strategy:* Extremely useful for cheap test farms, headless CI nodes, HIL appliances, and contributor onboarding. However, they are **not allowed to warp the kernel architecture**. If a Tier-2 board lacks IOMMU isolation, it runs in a degraded trust mode. We do not change the OS capability model to accommodate legacy SoC quirks.

## GPU Strategy
GPUs are treated as hostile ecosystems.
*Strategy:* They start in quarantined black-boxes (Linux compatibility domains) exporting only display surfaces via shared memory. They are only distilled into native components via the Foundry pipeline once the control plane is perfectly mapped and understood.
