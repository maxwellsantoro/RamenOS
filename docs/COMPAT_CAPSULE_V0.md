# Compat Capsule Format v0

**Last Updated:** 2026-02-18
**Status:** Active

## Purpose

A compat capsule descriptor is a JSON document that fully specifies a
compatibility domain. The Foundry gate generates it; the compat runner
consumes it to launch a VM/microVM. This document defines format version
`compat_capsule_v0`.

## Schema

| Field | Type | Description |
|-------|------|-------------|
| `kernel_content_id` | string | Content-addressed hash of the Linux kernel image (e.g. `"sha256:..."`). |
| `initrd_content_id` | string | Content-addressed hash of the initrd/initramfs (e.g. `"sha256:..."`). |
| `artifact_disks` | array | One or more artifact disk entries (see below). |
| `cmdline` | string | Kernel command line (e.g. `"console=ttyS0"`). |
| `resources` | object | Resource limits for the domain (see below). |
| `log_path` | string (optional) | Path to a serial log file (Foundry gate use). |

Notes:
- Content IDs are resolved via the installed artifact store (`out/installed/artifacts`).
- Absolute file paths are not permitted in production plans.

### artifact_disks entry

| Field | Type | Valid values (v0) |
|-------|------|-------------------|
| `content_id` | string | Content-addressed hash of the disk image. |
| `mount_policy` | string | `"read_only"` |
| `device_type` | string | `"virtio_blk"` |

### resources

| Field | Type | Example |
|-------|------|---------|
| `memory_mb` | integer | `512` |
| `cpus` | integer | `1` |

## Example

This matches what the current S2 Foundry gate (`tools/ci/foundry_compat_s2.sh`)
produces: a single kernel, initrd, one read-only virtio-blk artifact disk,
serial console, 512 MB RAM, 1 CPU.

```json
{
  "kernel_content_id": "sha256:placeholder_kernel",
  "initrd_content_id": "sha256:placeholder_initrd",
  "artifact_disks": [
    {
      "content_id": "sha256:placeholder_artifact",
      "mount_policy": "read_only",
      "device_type": "virtio_blk"
    }
  ],
  "cmdline": "console=ttyS0",
  "log_path": "/tmp/qemu_compat.log",
  "resources": {
    "memory_mb": 512,
    "cpus": 1
  }
}
```

## Architectural Constraints

These are hard boundaries, not deferred features.

- **Virtualization-first.** Compat domains always run in a VM/microVM. Never
  as containers, never as translated syscalls. This is a constitutional
  invariant: POSIX is compatibility-only and must not leak into native
  interfaces.

- **GPU quarantine.** GPU access for compat domains is deferred. When it
  arrives it will require explicit policy and a dedicated grant path. It is
  not part of v0.

- **Read-only artifacts.** All artifact mounts are read-only in v0. Writable
  scratch storage is a future extension and will require its own policy
  surface.

- **No network.** Compat domains have no network access in v0. Network
  capability will require an explicit grant when introduced.
