# S11: Driver Factory MVP

**Last Updated:** 2026-10-03
**Status:** Oracle/replay and embedded-vector harness lane landed; native device I/O pending

**CHOSEN:** `virtio-net-pci` in a Linux QEMU Oracle capsule. The Driver Factory
captures specified device behavior, assembles pinned reference material, and
checks Rust replay against that evidence. The landed lane does not complete the
original goal of a native device-backed network driver.

## Capture and implementation discipline

1. Capture Linux Oracle behavior and record device, environment, request, and
   provenance in typed protocol traces.
2. Assemble the [virtio-net Reference Vault](../../drivers/reference_vaults/virtio-net/)
   with specification, trace, Oracle source references, and harness contract.
3. Derive register/queue behavior from those inputs. Keep hardware-facing unsafe
   code narrow, documented, and bounded; volatile access still requires valid
   pointers, mapping, ordering, and ownership.
4. Compare initialization and packet operations against the replay scoreboard.
5. Exercise the typed harness with a real consumer and negative assertions.
   Embedded test vectors and an attached native device are different evidence.

The canonical interface is [net_v1.toml](../../idl/harness/net_v1.toml): explicit
protocol/message IDs and fixed handle/offset/length fields for shared-memory
packet data. Dynamic `bytes` fields and informal `reply` attributes from the
original design sketch are not the implemented native IDL.

## Landed evidence

| Scope | Gate |
|-------|------|
| Contract/capture inventory | `tools/ci/foundry_s11_driver_factory_s11_0.sh` |
| Init and packet replay | `tools/ci/foundry_s11_replay.sh` |
| Reference Vault and required live Oracle provenance | `tools/ci/foundry_s11_reference_vault_s11_3.sh` |
| Typed shared-memory packet transfers over embedded vectors | `tools/ci/foundry_s11_runtime_net_s11_8.sh` |

`just s11` runs the replay, live-provenance vault, and runtime-vector checks.
Linux Oracle capture, including packet RX, describes the Oracle's device path;
it does not prove native RamenOS RX. The S11.8 QEMU init profile checks
`harness.net` payload transfers against embedded Oracle packets without a native
virtio-net device provider.

## Remaining device milestone

Define device attachment, queue setup, interrupt/polling behavior, capability and
DMA boundaries, and Oracle comparisons before implementing native send/receive.
Test affected shared-memory consumers and recovery paths. Broader hardware support
requires new reference material and evidence, not reuse of a passing fixture.

Exact replay agreement is a bounded conformance result. Native latency within
10% of the Linux Oracle remains an unmeasured target, not a result of `just s11`.
Trace coverage also cannot establish that all device states or failures are known.

See [Current Status](../../CURRENT_STATUS.md), [Next Tasks](../../NEXT_TASKS.md),
[Hardware Strategy](../HARDWARE_STRATEGY.md), and
[Evidence Levels](../../EVIDENCE_LEVELS.md). Native block storage has its own
[S13 contract](2026-06-21-s13-persistent-storage-design.md).
