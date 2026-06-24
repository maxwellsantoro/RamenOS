# S12: First Metal (Golden Machine)

**Last Updated:** 2026-06-21
**Status:** Active (S12.1 GOP probe complete; S12.2 HIL next)
**Gate:** `tools/ci/foundry_s12_golden_machine_s12_0.sh`
**Related:** `docs/HARDWARE_STRATEGY.md`, `ROADMAP.md` §12, `hardware/golden_machine_v0.toml`

---

## Executive Summary

S12 escapes VM-only bring-up by pinning a **Tier-1 golden machine** contract and proving **UEFI boot to a visible framebuffer (GOP)** with **serial logging** on physical silicon. S11 closed the Driver Factory loop in QEMU; S12 is the first bare-metal vertical slice.

**S12.0 (this milestone):** Pin the hardware contract, add a machine manifest, and land a Foundry smoke gate (inventory + negative assertions). No physical HIL required in default CI.

**S12.1+:** GOP probe implementation (QEMU OVMF dev path, then physical HIL).

---

## 0. Tier-1 reference machine (resolved 2026-06-21)

**CHOSEN:** Intel NUC 12/13 class (x86_64, UEFI, integrated Intel graphics GOP).

| Criterion | Intel NUC 12/13 | Framework Laptop 13/16 |
|-----------|-----------------|------------------------|
| UEFI + GOP | Standard PC firmware, GOP on iGPU | Yes, but board variance higher |
| VT-d (IOMMU) | Typically enabled in firmware | Typically enabled |
| Lab reproducibility | Fixed small-form-factor profile | Multiple mainboard generations |
| Contributor access | Common refurb market | Less uniform in farms |
| Downstream S13 NVMe | M.2 NVMe standard | M.2 NVMe standard |

**Deferred secondary:** Framework Laptop 13 (Intel) as a second Tier-1 profile after NUC path is green. ARM64 Tier-1 (e.g. Apple-silicon class with SMMU) is post-S12.

**Manifest:** `hardware/golden_machine_v0.toml` is the machine-auditable source of truth for gates and agents.

---

## 1. Tier-1 hardware contract

Aligned with `docs/HARDWARE_STRATEGY.md`. A golden machine **must** expose:

| Capability | S12 requirement | Evidence |
|------------|-----------------|----------|
| UEFI boot | Required | `BOOTX64.EFI` from RamenOS `kernel_uefi` |
| Serial logging | Required | UART or USB-serial; same banner discipline as QEMU gates |
| GOP framebuffer | Required (S12 DoD) | UEFI `GraphicsOutputProtocol`; deterministic fill or probe pattern |
| VT-d / IOMMU | Required (Tier-1) | ACPI DMAR present; firmware VT-d enabled |
| PCIe + NVMe | Inventory only in S12 | Exercised in S13 |
| USB xHCI | Inventory only in S12 | Exercised in S14 |

**Non-negotiable:** Tier-2 boards without IOMMU may run in degraded trust mode later; they do **not** define the golden contract.

---

## 2. Implementation phases

### Phase 0 — Contract scaffold (S12.0) ✅ target now

- `docs/plans/2026-06-21-s12-golden-machine-design.md` (this doc)
- `hardware/golden_machine_v0.toml`
- `tools/ci/foundry_s12_golden_machine_s12_0.sh` — inventory + negative assertions; **PASS without hardware**

### Phase 1 — GOP probe (S12.1) ✅

- `kernel_uefi/src/gop_probe.rs`: locate GOP via UEFI boot services; query mode; 64×64 `VideoFill`
- QEMU stepping stone: OVMF GOP before physical HIL (gate PASS: 1280×800 BGR)
- Init profile `gop_probe` + `OP_GOP_PROBE` serial markers:

```
golden_machine: gop_probe ok
golden_machine: gop_width=<u32>
golden_machine: gop_height=<u32>
golden_machine: gop_pixel_format=<u32>
```

- Gate: `foundry_s12_gop_probe_s12_1.sh` (QEMU OVMF first)

### Phase 2 — Physical HIL boot (S12.2)

- USB boot stick or PXE flow documented for lab operators
- Gate: `foundry_s12_hil_boot_s12_2.sh` runs only when `RAMEN_HIL_GOLDEN_MACHINE=1`
- Default CI: **skip** HIL (fail-closed under `RAMEN_CI_STRICT=1` if skip attempted without explicit policy — same pattern as S2 compat)

### Phase 3 — IOMMU inventory marker (S12.3)

- ACPI DMAR walk or firmware table probe; serial marker `golden_machine: iommu_present=1`
- No full IOMMU programming in S12; isolation enforcement matures with driver deployment

---

## 3. S12 Definition of Done (full slice)

S12 is complete when:

1. **Contract pinned** — manifest + design doc + S12.0 smoke gate PASS.
2. **GOP probe** — S12.1 gate PASS (QEMU OVMF or physical).
3. **Physical boot** — S12.2 HIL gate PASS on the Tier-1 reference machine with serial banner + GOP markers.
4. **IOMMU inventory** — S12.3 gate asserts DMAR/VT-d visibility on the same machine class.

Fast-path (future): `just s12` = S12.0 + S12.1; HIL legs opt-in via env.

---

## 4. HIL / CI policy

| Mode | Env | Behavior |
|------|-----|----------|
| Default CI | (none) | S12.0 + S12.1 QEMU only; HIL gates **skip** |
| Lab HIL | `RAMEN_HIL_GOLDEN_MACHINE=1` | Run S12.2 + S12.3 against attached hardware |
| Strict CI | `RAMEN_CI_STRICT=1` | Skips become FAIL (compat gate pattern) |

**Negative assertions (S12.0 gate):**

- Must not require physical hardware in default `foundry_ci_extended.sh` path.
- Must not weaken Tier-1 IOMMU requirement in manifest.
- Must not add ioctl-style framebuffer escapes; GOP stays UEFI boot-services path until native display harness (S15).

---

## 5. Inventory (2026-06-21)

| Component | Status | Notes |
|-----------|--------|-------|
| `kernel_uefi` UEFI entry | ✅ | Serial banner + GOP probe |
| `hardware/golden_machine_v0.toml` | ✅ S12.0 | Tier-1 contract |
| GOP probe in `kernel_uefi` | ✅ S12.1 | `gop_probe.rs` |
| `OP_GOP_PROBE` init profile | ✅ S12.1 | `gop_probe` profile |
| Physical HIL gate | ✅ S12.2 | `foundry_s12_hil_boot_s12_2.sh` + `tools/hil/build_usb_boot_image.sh` |
| IOMMU ACPI probe | ✅ S12.3 | `foundry_s12_iommu_inventory_s12_3.sh` + `kernel_uefi/src/iommu_probe.rs` |

---

## 6. Scope guard

**In scope:** Tier-1 contract, GOP visibility, serial discipline, HIL gate skeleton.

**Out of scope (later slices):**

- S13: NVMe boot + atomic update on metal
- S14: USB xHCI + HID
- S15: Native window compositor / display harness
- Full IOMMU programming and user-space driver sandbox on metal
- Tier-2 degraded-trust profiles

---

## 7. Gates

| Gate | Phase | Default CI |
|------|-------|------------|
| `foundry_s12_golden_machine_s12_0.sh` | S12.0 | Yes |
| `foundry_s12_gop_probe_s12_1.sh` | S12.1 | Yes (QEMU) |
| `foundry_s12_hil_boot_s12_2.sh` | S12.2 | No (HIL opt-in) |
| `foundry_s12_iommu_inventory_s12_3.sh` | S12.3 | No (HIL opt-in) |