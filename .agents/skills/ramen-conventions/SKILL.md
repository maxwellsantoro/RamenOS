---
name: ramen-conventions
description: RamenOS architecture invariants and coding patterns. Apply when writing or reviewing kernel, service, or store code.
user-invocable: false
---

## Non-Negotiables (CONSTITUTION.md)

1. Native interfaces are **typed Harnesses/Portals** -- no ioctl-style escape hatches
2. **POSIX is compatibility-only** -- never design native APIs around POSIX semantics
3. **Capability validation is kernel-side** for fast-path ops; user-space brokers decide grants
4. Control plane uses **typed messages**; data plane is **zero-copy shared memory**
5. Preserve boundaries: **kernel != services != store**

## Kernel Code Rules

- No dynamic allocation until mm is stable
- Keep arch-specific code in `kernel/arch/`
- IPC message formats must be typed and versionable (defined in `kernel_api`)
- No external crate dependencies in `kernel/` or `kernel_api/`

## Interface Discipline

- New interfaces must be added to `/idl` as TOML specs
- Code-generate Rust bindings via `just codegen` (uses `idl_codegen`)
- Never hand-write code that should be generated

## Slice Discipline

- Every change must: improve boot/run, implement an IDL contract, add a Foundry gate, or implement a Store feature
- Gate-first: write the Foundry gate assertion before the implementation
- No "temporary hacks" that violate the Constitution
- Prefer small diffs with tests over big refactors

## File Organization

| Directory | Purpose | Dependencies |
|-----------|---------|-------------|
| `kernel/` | Core kernel library | None (no external crates) |
| `kernel_api/` | Shared types for kernel/runtime | None |
| `kernel_aarch64/` | aarch64 bootstrap | kernel |
| `kernel_uefi/` | x86_64 UEFI boot | kernel, uefi crate |
| `idl/` | Interface definitions (TOML) | N/A |
| `services/` | User-space services | Must not reach into kernel |
| `store/` | Software store metadata | Independent of kernel |
| `tools/ci/` | Foundry gate scripts | N/A |

## Style

- Clippy with `-D warnings` (deny all warnings)
- `cargo fmt` for formatting
- Update `CURRENT_STATUS.md` and `CHANGELOG.md` per milestone
- Record design choices in `DECISIONS.md`
