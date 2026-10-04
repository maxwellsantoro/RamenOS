# Hardware Strategy

**Last Updated:** 2026-10-03
**Status:** Active

Hardware support serves the [Vision](../VISION.md) of a fast, adaptable everyday
OS for humans and AI agents. RamenOS qualifies specific hardware profiles first,
then expands support through typed driver contracts, isolated domains, Oracle
traces, and Foundry gates. Hardware breadth and fault containment require
per-device evidence; they are not established by the architecture alone.

## Tier-1: The Golden Platform (PC-Class)
Our primary target for bare-metal graduation. To be Tier-1, the hardware MUST support:
- UEFI Boot
- PCIe enumeration
- NVMe storage
- USB xHCI
- **A working IOMMU** (VT-d, AMD-Vi, or ARM SMMU)

*Strategy:* We optimize for one specific x86_64 machine first (the acquired Lenovo ThinkCentre M900), followed by one PC-class ARM64 machine. An IOMMU is required for the intended DMA containment boundary; inventory
alone does not prove that boundary is configured or enforced.

**S12 reference (2026-07-19):** The acquired Lenovo ThinkCentre M900 Small Form Factor (machine type 10FH, model 00SNUS) with an Intel Core i7-6700 and 8 GiB RAM is the pinned Tier-1 golden machine. Its populated rear RS-232/DB9 port makes the serial-observer HIL path direct and repeatable. The Pi↔M900 serial chain is physically installed and ready. S12 runs on the installed 240 GB SanDisk SATA SSD; a compatible M.2 2280 PCIe NVMe drive remains required for S13 metal graduation. See `hardware/golden_machine_v0.toml` and `docs/plans/2026-06-21-s12-golden-machine-design.md`.

## HIL Appliance Controller
A Raspberry Pi-class controller is the preferred always-on lab appliance for physical development. It is **not** a RamenOS target and is **not** part of the target TCB. It observes and actuates the golden machine so agents can run bare-metal loops without manual reboot/cable/log work.

Minimum appliance duties:
- serial capture from target COM/DB9/header through a USB RS-232 adapter;
- power/reset actuation through the target's Intel AMT 11 interface;
- timestamped evidence bundle generation;
- later KVM-grade HDMI capture, USB HID injection, and virtual boot media.

The S12.4.0 scaffold gate (`tools/ci/foundry_hil_appliance_s12_4.sh`) protects the docs/manifest/evidence-schema contract in normal CI. Serial-observer tooling is landed; the next physical work is its first live
capture followed by S12.4.2 AMT power/reset implementation and validation. A front-panel relay or smart plug/PDU is a deferred fallback, not a current purchase requirement.

Electrical rule: Pi GPIO UART is 3.3V TTL only. Do not connect Pi GPIO directly to PC RS-232/DB9. See `hardware/hil_appliance_v0.toml`, `docs/plans/2026-06-22-hil-appliance-controller.md`, and `tools/ci/foundry_hil_appliance_s12_4.sh`.

## Tier-2: Lab & Outreach (SBCs)
Devices like the Raspberry Pi.
*Strategy:* Extremely useful for cheap test farms, headless CI nodes, HIL appliances, and contributor onboarding. However, they are **not allowed to warp the kernel architecture**. If a Tier-2 board lacks IOMMU isolation, it runs in a degraded trust mode. We do not change the OS capability model to accommodate legacy SoC quirks.

## GPU Strategy
GPUs are treated as hostile ecosystems.
*Strategy:* They start in quarantined black-boxes (Linux compatibility domains) exporting only display surfaces via shared memory. They are only distilled into native components via the Foundry pipeline after pinned traces, contracts, and gates establish the selected control-plane behavior.
