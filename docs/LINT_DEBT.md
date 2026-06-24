# Lint Debt Register

**Last Updated:** 2026-06-17
**Status:** Active
**Owner:** Kernel maintainers

This file tracks explicit lint allowances that remain after strict-clippy rollout.
Each item must have an exit criterion and should be removed as soon as feasible.

## Workspace Baseline (2026-06-17)

Host workspace clippy baseline reports **0 warnings** (2026-06-16 burn-down). Strict tranches 1–6 pass with `-D warnings` on selected crates including `ramen_sdk`, `native_runner`, `semantic_state`, and `execution_fabric`.

## Active Allowances

1. `kernel/src/lib.rs`
- Allowance: `#![allow(static_mut_refs)]`
- Why now: boot-time global singletons still rely on `static mut` access patterns.
- Exit criteria: migrate global kernel singletons to interior-mutability wrappers with explicit synchronization semantics and remove direct mutable static references.

2. `kernel/src/trace_ring.rs`
- Allowance: `#[allow(clippy::declare_interior_mutable_const)]` in const ring initialization.
- Why now: const array initialization for atomic state uses the current pattern to avoid runtime init overhead in no-std.
- Exit criteria: replace with lint-clean const/static initialization approach supported by current toolchain without safety/perf regression.

3. `kernel/src/arch/aarch64/mmu.rs`
- Allowances: `#![cfg_attr(not(target_arch = "aarch64"), allow(dead_code, unused_imports))]` and `#[cfg_attr(test, allow(dead_code))]`.
- Why now: host-target strict lint compiles architecture code and test-only helpers that are intentionally unused outside aarch64/hardware paths.
- Exit criteria: split test-only helpers and host-only stubs into cfg-separated modules so dead-code suppression is no longer needed.
