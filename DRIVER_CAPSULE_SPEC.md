# Driver Capsule Specification

**Last Updated:** 2026-10-04
**Status:** Maintained architecture and bounded S3.x implementation reference

A Driver Capsule is the intended compatibility boundary for legacy drivers:
clients use typed Harnesses while the legacy backend runs in a quarantined
component. It supports the [Vision](VISION.md) of replaceable hardware support
without making legacy APIs native. Isolation, device access and recovery require
backend-specific evidence; the current echo demonstration is not a qualified
hardware driver.

## Implemented path and evidence

[Capsule relay](services/capsule_relay/src/main.rs) implements two modes:

| Mode | What executes | Evidence boundary |
|------|---------------|-------------------|
| Default `host-only` | An in-process mock handles typed hello, health, echo and shutdown messages; the relay stores artifacts through StoreClient | Host contract/artifact demonstration; no VM or device containment |
| Optional `vm` | A Linux capsule agent exchanges envelopes through QEMU virtio-serial | A separate VM path when explicitly exercised; no general DMA or physical-device qualification |

The actual interfaces are
[`capsule.control/v0`](idl/harness/capsule_control_v0.toml) and
[`harness.echo/v0`](idl/harness/echo_harness_v0.toml). The echo request carries a
request ID and payload length; it does not transfer the artifact's bytes through
a native device data plane. The payload is separately ingested into the Store.
There is no implemented generic `capsule.harness_relay/v0` native interface.
New operations require their own typed IDL, authority and failure contract.

Run `just foundry-driver-capsule-s3x`. Its
[gate](tools/ci/foundry_driver_capsule_s3x.sh) checks host control/echo exchanges,
a malformed payload, persisted trace/observed-capability/scenario artifacts,
and their cross-references. VM mode additionally needs `S2_COMPAT_KERNEL`, QEMU,
and a buildable capsule initrd; read the log to distinguish execution from skips.
The final gate success alone does not establish that VM mode ran.

The gate's [protocol replay tool](tools/trace/replay_protocol_trace.py) validates
recorded pairing, sequence and operation lengths, and computes a deterministic
transcript digest. It does not launch the backend, replay hardware interactions, or minimize
a failing trace. Those require separate consumers and gates.

## Architecture requirements for a real driver

- Treat the capsule and legacy driver as untrusted. Name the actual trusted host
  or target supervisor, broker, transport and Store boundary in each implementation.
- Native control uses versioned, bounded typed messages. Bulk native data uses
  capability-checked shared memory; the compatibility envelope is not permission
  to introduce an untyped native command channel.
- Kernel fast paths validate capabilities; brokers decide grants. Define request
  authority (`Lang`) and observable authority (`ObsContract`) separately, including
  device access, mappings, lifetime and revocation.
- Device/MMIO/DMA access, host filesystem access and networking require explicit
  scoped authority. Test the actual backend's denial and isolation behavior.
- Keep artifacts immutable; outputs cross the Store's checked publication boundary.
  Service-facing artifact types come from schema crates; Store IO stays in Store.
- Bound resource use, stalled calls and crash/restart behavior. A component's
  separation alone does not prove unaffected consumers or recoverable failures.
- Keep traces local by default. Define and enforce redaction, size/retention and
  export policy for the producer; artifact schema fields alone do not enforce it.

## Contract and implementation entry points

- [Trace artifacts](docs/TRACE_ARTIFACT_V0.md) own the harness/scenario JSON shape.
- [Observed capabilities](docs/OBSERVED_CAPS_V0.md) own scenario-bounded observations;
  one run is not all possible authority.
- [Reference Vaults](drivers/reference_vaults/README.md) own the driver evidence
  workflow. Obtain documentation and Oracle `protocol_trace` artifacts before
  deriving registers or device behavior.
- [Compatibility capsule](docs/COMPAT_CAPSULE_V0.md) and
  [Platform Overview](PLATFORM_OVERVIEW.md) explain the adjacent boundaries.
- [Current Status](CURRENT_STATUS.md), [Next Tasks](NEXT_TASKS.md), and
  [Evidence Levels](EVIDENCE_LEVELS.md) own implementation scope, scheduling and
  hardware claims. Do not use this reference as a second task queue.

## Completion for a new capsule-backed device

A bounded device slice needs a real consumer, reviewed Vault/Oracle evidence,
typed contracts, actual device transfers, malformed/unauthorized-call denials,
resource-exhaustion and crash/recovery assertions, and trace comparison against
the Oracle. Test affected consumers and shared resources before substituting a
native driver. Qualify physical claims on the named device with fresh provenance;
a host echo or embedded-vector pass cannot satisfy those requirements.
