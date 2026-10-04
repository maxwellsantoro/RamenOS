# S13: Persistent Storage

**Last Updated:** 2026-10-03
**Status:** Maintained reference; QEMU Oracle/vector loop landed, metal graduation pending

S13 builds from a typed block contract and Linux Oracle capture toward native
storage and physical update/rollback evidence. [storage_contract_v0.toml](../../hardware/storage_contract_v0.toml)
and [block_v1.toml](../../idl/harness/block_v1.toml) own the machine and interface
contracts. `tools/ci/foundry_s13_persistent_storage_s13_0.sh` consumes this design.

## Device selection and boundaries

**CHOSEN (QEMU Oracle):** `virtio-blk-pci` in the Linux capsule. Its
[Reference Vault](../../drivers/reference_vaults/virtio-blk/) contains pinned
specification and initialization/sector evidence for replay.

**CHOSEN (metal):** M.2 2280 PCIe NVMe on the Tier-1 golden machine. A controller
vendor and native Oracle/driver still need to be pinned. Firmware boot from an
NVMe ESP is distinct from a RamenOS-native NVMe block driver.

Current block harness tests transfer embedded Oracle sectors through typed IPC
and shared memory. They attach no target virtio-blk device and prove no native
read/write/flush persistence. Native device-backed operation needs a separate
consumer, recorded Oracle comparison, and explicit ordering/durability assertions.

## Landed phases and gate coverage

| Phase | Scope | Script under `tools/ci/` |
|-------|-------|-------------------------|
| S13.0–S13.1 | Storage manifest, block IDL/codegen and vault contract | `foundry_s13_persistent_storage_s13_0.sh` |
| S13.2 | Linux Oracle initialization capture | `foundry_s13_virtio_blk_oracle_s13_2.sh` |
| S13.3 | Initialization replay scoreboard | `foundry_s13_replay.sh` |
| S13.4–S13.5 | Sector Oracle and mock block replay | `foundry_s13_block_sector_oracle_s13_4.sh`, `foundry_s13_replay.sh` |
| S13.6 | QEMU embedded-vector harness transfers | `foundry_s13_runtime_block_s13_6.sh` |
| S13.7 | NVMe ESP boot probe and physical gate scaffold | `foundry_s13_nvme_boot_s13_7.sh` |
| S13.8 | A/B metadata probe and physical gate scaffold | `foundry_s13_atomic_update_s13_8.sh` |

`just s13` runs contract, initialization Oracle, sector Oracle, replay, and
runtime-vector checks. `just s13-hil` selects the NVMe boot/metadata legs.
Physical modes require prepared media, live capture, and matching provenance;
these scaffolds do not yet implement the full update/rollback verifier.

## Required software milestone before physical graduation

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

## Definition of done

1. Pin and validate the storage/IDL contracts and Oracle provenance.
2. Pass the virtio-blk Oracle/replay and typed harness-vector lane, then separately
   establish native device-backed read/write/flush behavior.
3. Establish live Tier-1 NVMe ESP boot with matching build/run provenance.
4. Pass the completed publication/readback/new-slot/rollback verifier across
   separate boots, naming the actual storage enforcement backend.

Default QEMU/vector success is not `PASS/METAL`. Appliance-mediated and standalone
operator runs use different `claim_path` stamps; both require fresh target
provenance under [Evidence Levels](../../EVIDENCE_LEVELS.md). The
[HIL appliance](2026-06-22-hil-appliance-controller.md) wraps per-gate evidence.

Full filesystems, multipath/RAID/encryption, USB mass storage, and degraded-trust
SBC storage remain later work. [Current Status](../../CURRENT_STATUS.md) owns
landed state; [Next Tasks](../../NEXT_TASKS.md) owns the H0–H3 physical order and
software protocol prerequisite. SW0 does not wait for S13 graduation.
