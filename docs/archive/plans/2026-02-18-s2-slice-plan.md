# S2 Slice Plan (Gate-First)

**Last Updated:** 2026-02-18
**Status:** Historical
**Scope:** Slice S2 planning only.

## Goal
A compatibility domain boots and runs one trivial program under hard boundaries.
Virtualization-first (VM/microVM), no syscall translation.

## Definition of Done
- A compat domain VM/microVM boots from a minimal image.
- A trivial program runs and prints a sentinel line.
- The boundary is enforced (no host filesystem access beyond a read-only artifact mount).
- Foundry gate asserts the sentinel output and exits cleanly.

## Gate (Proposed)
`tools/ci/foundry_compat_s2.sh`
- Build a minimal compat initrd.
- Build a tiny ext4 artifact image.
- Launch microVM/QEMU with a single read-only artifact mount.
- Ingest kernel/initrd/artifact into the installed content store and use content IDs in the plan.
- Wait for sentinel string and exit.

Expected output example:
- `COMPAT_S2: hello`
- `COMPAT_S2: read artifact ok`
- `COMPAT_S2: write blocked ok`

Helper:
- `tools/compat/build_compat_initrd.sh` builds a tiny initrd with `/init`.
- `tools/compat/build_compat_artifact_img.sh` builds a tiny ext4 artifact image.

## Minimal Interfaces
- Console output only (serial).
- One read-only artifact mount (virtio-blk ext4 image).
  
Boundary check:
- Write attempts to the artifact mount must fail (read-only enforcement).

## Non-Goals
- Full Linux userspace.
- Networking, GPU, or windowing.
- Filesystem integration beyond a read-only mount.
- Syscall translation.

## Notes
Keep the compat capsule visibly labeled (logs and Store UI) to preserve the
"compatibility without surrender" principle.
Gate assumes ext4 support is built-in for the kernel image used in CI.
CI can pin a kernel image via `S2_COMPAT_KERNEL_URL` and `S2_COMPAT_KERNEL_SHA256`.
