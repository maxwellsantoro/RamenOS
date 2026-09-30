# S13: Persistent Storage

**Last Updated:** 2026-09-30
**Status:** Active reference; QEMU loop landed, metal graduation pending
**Gate:** `tools/ci/foundry_s13_persistent_storage_s13_0.sh`
**Related:** `docs/plans/2026-02-20-s11-driver-factory-mvp.md`, `hardware/storage_contract_v0.toml`, `ROADMAP.md` §12

---

## Executive Summary

S13 delivers **native block storage** on Tier-1 hardware: distill a block driver via the Driver Factory, exercise it through `harness.block` in QEMU, then graduate to **NVMe boot + atomic update/rollback** on metal. S11 proved the Oracle→replay→harness loop for virtio-net; S13 repeats it for virtio-blk before touching real NVMe silicon.

**S13.0 (this milestone):** Pin the storage contract, land `harness.block` IDL, scaffold the virtio-blk Reference Vault, and add a Foundry smoke gate (inventory + negative assertions). No physical NVMe required in default CI.

**S13.1+:** Oracle capture, replay, runtime harness I/O in QEMU, then metal NVMe boot and Store-backed A/B updates.

---

## 0. Oracle device selection (resolved 2026-06-21)

**CHOSEN (QEMU Oracle):** `virtio-blk-pci` in the QEMU Linux Oracle capsule for S13 MVP stepping stone.

| Criterion | virtio-blk (QEMU) | Native NVMe (metal) |
|-----------|-------------------|---------------------|
| QEMU availability | Built-in `virtio-blk-pci`; no passthrough | Requires Tier-1 M.2 + firmware enable |
| Trace complexity | Moderate (virtqueues, PCI config; simpler than NVMe admin/IO queues) | High (admin/IO queues, many opcodes, vendor variance) |
| Harness target | `harness.block` (S13.0 IDL) | Same harness; metal controller behind it |
| Foundry reuse | Capsule relay + `pci_mmio_tracer` (S11.1) | Deferred to S13.7+ |
| Boot-critical risk | Low — lab/dev path first | High — S13 graduation evidence |

**CHOSEN (metal graduation):** Standard **M.2 NVMe PCIe** on the Tier-1 golden machine class (`hardware/golden_machine_v0.toml` inventory). Specific controller vendor is **not** pinned at S13.0; metal Oracle capture selects the lab controller after QEMU path is green.

**Deferred:** PCI passthrough of host NVMe into QEMU Oracle (optional lab acceleration, not required for DoD).

**Manifest:** `hardware/storage_contract_v0.toml` is the storage-auditable source of truth for gates and agents.

---

## 1. Tier-1 storage contract

Aligned with `hardware/golden_machine_v0.toml` and `docs/HARDWARE_STRATEGY.md`.

| Capability | S13 requirement | Evidence |
|------------|-----------------|----------|
| Block harness | Required | `idl/harness/block_v1.toml`; typed `harness.block` IPC |
| Oracle trace | Required (QEMU) | `driver_protocol_trace_v0` from Linux capsule + virtio-blk |
| Replay scoreboard | Required | `MockPciDevice` / block mock parity with Oracle |
| Runtime harness I/O | Required (QEMU) | Serial `persistent_storage: harness.block ok` |
| NVMe boot partition | Required (metal) | UEFI boots RamenOS; reads GPT slot A |
| Atomic update/rollback | Required (metal) | Store A/B slot flip + serial `persistent_storage: atomic_update ok` |
| IOMMU | Required (Tier-1) | Inherited from S12; block DMA must respect VT-d path |

**Non-negotiable:** Block I/O uses typed harness messages + shmem data plane — no ioctl-style block escapes.

---

## 2. Implementation phases

### Phase 0 — Contract scaffold (S13.0) ✅ target now

- `docs/plans/2026-06-21-s13-persistent-storage-design.md` (this doc)
- `hardware/storage_contract_v0.toml`
- `idl/harness/block_v1.toml` + codegen
- `drivers/reference_vaults/virtio-blk/` vault scaffold
- `tools/ci/foundry_s13_persistent_storage_s13_0.sh` — inventory + negative assertions; **PASS without hardware**

### Phase 1 — Block harness IDL (S13.1)

- Codegen wired into `kernel_api`; vault `harness.toml` aligned
- Gate asserts IDL lint + binding inclusion

### Phase 2 — Oracle capture (S13.2)

- `tools/trace/capture_virtio_blk_oracle.sh` — QEMU capsule boot with `virtio-blk-pci`
- Promote live trace to `drivers/reference_vaults/virtio-blk/traces/oracle_init_trace.json`

### Phase 3 — Reference vault + replay (S13.3)

- `driver_foundry::virtio_blk_init` vault replay through `MockPciDevice`
- Extend `foundry_s13_replay.sh` (or shared replay gate)

### Phase 4 — Block I/O distillation (S13.4–S13.5)

- Read/write sector Oracle traces; `MockBlockHarness` scoreboard
- `oracle_block_trace.json` fixture with live provenance option

### Phase 5 — Runtime harness.block in QEMU (S13.6)

- `kernel/src/block_harness.rs` NET_V1 analogue for block IPC
- Init profile `block_io` + serial markers:

```
persistent_storage: harness.block ok
persistent_storage: block_read ok
persistent_storage: block_write ok
```

### Phase 6 — Metal NVMe boot (S13.7)

- GPT layout with RamenOS boot partition on NVMe
- Gate runs only when `RAMEN_HIL_GOLDEN_MACHINE=1`
- Serial marker `persistent_storage: nvme_boot ok`

### Phase 7 — Atomic update/rollback (S13.8)

- A/B GPT slots; Store install publishes to inactive slot; firmware/kernel flip
- Reuse S1 artifact rollback discipline on metal
- Serial marker `persistent_storage: atomic_update ok`

#### Required software milestone before physical graduation

The present S13.8 marker is an A/B metadata probe, not proof of a transaction.
`RamenAbSlot.rollback_ready` is operator-supplied metadata; host S1 rehearsal
does not establish target slot publication or recovery. Implement the protocol
and verifier below before H3 graduation. Live test hardware is not available
for execution yet; begin with gate-first host/QEMU fault/recovery assertions.

1. Pin source/target artifacts, GPT slot identities and the storage backend.
   Publish the new artifact to the inactive slot, flush according to an explicit
   backend durability contract, and verify bytes by readback/content hash.
2. Durably record the old/new artifact identities, transaction revision and
   pending boot selection. Preserve the last known-good slot until the new
   artifact is verified. Define interrupted-write, interrupted-selection and
   failed-new-boot recovery before implementation.
3. Boot the new slot and capture its actual selected partition/artifact identity,
   transaction revision and fresh boot nonce. A variable naming slot B is not
   evidence that B's bytes executed. Verify the target's successful health result
   against that transaction before accepting it.
4. Exercise rollback/recovery in a separate boot, with a different fresh nonce,
   and verify the old slot/artifact identity and recovered state. Link both boot
   bundles and publication/readback evidence into one protocol result.
5. Reject missing, repeated, out-of-order, mismatched or failed phases. Inject
   failures at each durability boundary in the host/QEMU protocol model; physical
   reset/power-loss evidence is an explicit later hardware test, not inferred
   from those simulations.

At least the new-slot boot and rollback boot must have separate provenance-bound
captures. A single metadata-marker transcript cannot satisfy this protocol.
The verifier needs a versioned protocol evidence schema and fixtures before
implementation; today's `just s13-hil` remains a probe/evidence scaffold.

S13.7 separately establishes UEFI boot from an NVMe ESP device path. Native NVMe
`harness.block` operation requires a pinned controller Reference Vault and Oracle,
a distilled driver and target read/write/flush evidence. Neither firmware
selection nor the virtio-blk QEMU loop establishes native NVMe I/O on metal.

---

## 3. S13 Definition of Done (full slice)

S13 is complete when:

1. **Contract pinned** — manifest + design doc + S13.0 smoke gate PASS.
2. **QEMU Driver Factory loop** — virtio-blk Oracle capture, replay, and `harness.block` runtime I/O PASS (`just s13` fast-path).
3. **Metal NVMe boot** — S13.7 HIL gate PASS on Tier-1 class hardware.
4. **Atomic update** — completed protocol verifier PASS over publication/readback,
   new-slot boot and separate rollback/recovery boot with matching artifact/slot
   identities and fresh per-boot provenance. An S13.8 metadata scaffold pass alone
   is insufficient; name the actual storage enforcement backend separately.

Fast-path (target): `just s13` = S13.0 + S13.6 QEMU legs; metal legs opt-in via `RAMEN_HIL_GOLDEN_MACHINE=1`.

---

## 4. CI / HIL policy

| Mode | Env | Behavior |
|------|-----|----------|
| Default CI | (none) | S13.0 + future QEMU gates only; metal gates **skip** |
| Lab HIL | `RAMEN_HIL_GOLDEN_MACHINE=1` | Run S13.7 + S13.8 against attached NVMe hardware |
| Strict CI | `RAMEN_CI_STRICT=1` | Skips become FAIL (compat gate pattern) |

**Negative assertions (S13.0 gate):**

- Must not require physical NVMe in default `foundry_ci_extended.sh` path.
- Must not add ioctl-style block escapes in native harness IDL.
- Must not skip Oracle provenance discipline (`driver_protocol_trace_v0` + live SHA-256 when `REQUIRE_LIVE_ORACLE_TRACE=1`).

---

## 5. Inventory (2026-06-21)

| Component | Status | Notes |
|-----------|--------|-------|
| S13 design doc | ✅ S13.0 | This doc |
| `hardware/storage_contract_v0.toml` | ✅ S13.0 | Oracle + metal contract |
| `harness.block` IDL | ✅ S13.0 | `idl/harness/block_v1.toml` |
| virtio-blk Reference Vault | ✅ S13.2 | `drivers/reference_vaults/virtio-blk/` + live `oracle_init_trace.json` |
| Oracle capture script | ✅ S13.2 | `capture_virtio_blk_oracle.sh` |
| Block sector Oracle trace | ✅ S13.4 | `oracle_block_trace.json` |
| MockBlockHarness replay | ✅ S13.5 | `foundry_s13_replay.sh` sector leg |
| Replay gate | ✅ S13.3 | `foundry_s13_replay.sh` |
| Runtime harness.block | ✅ S13.6 | `foundry_s13_runtime_block_s13_6.sh` |
| Metal NVMe boot | ✅ S13.7 scaffold | HIL opt-in (`just s13-hil`); QEMU negative smoke in gate |
| Atomic update/rollback | ✅ S13.8 scaffold | HIL opt-in (`just s13-hil`); QEMU negative smoke in gate |

---

## 6. Scope guard

**In scope:** Block harness IDL, virtio-blk Oracle in QEMU, replay, runtime block I/O, metal NVMe boot partition, Store A/B update evidence.

**Out of scope (later slices):**

- Full filesystem (ext4/btrfs native) — compat domain until distilled
- NVMe multipath, RAID, encryption
- USB mass storage (S14 xHCI stack)
- Tier-2 SBC storage without IOMMU (degraded trust only)

---

## 7. Gates

| Gate | Phase | Default CI |
|------|-------|------------|
| `foundry_s13_persistent_storage_s13_0.sh` | S13.0 | Yes |
| `foundry_s13_block_harness_s13_1.sh` | S13.1 | Yes (TBD) |
| `foundry_s13_virtio_blk_oracle_s13_2.sh` | S13.2 | Yes |
| `foundry_s13_block_sector_oracle_s13_4.sh` | S13.4–S13.5 | Yes |
| `foundry_s13_replay.sh` | S13.3 | Yes |
| `foundry_s13_runtime_block_s13_6.sh` | S13.6 | Yes |
| `foundry_s13_nvme_boot_s13_7.sh` | S13.7 | No (HIL opt-in) |
| `foundry_s13_atomic_update_s13_8.sh` | S13.8 | No (HIL opt-in) |
