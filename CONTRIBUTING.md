# Contributing

**Last Updated:** 2026-06-24
**Status:** Active

RamenOS is built as small, evidence-bearing vertical slices. Before changing a
subsystem, read [AGENTS.md](AGENTS.md), [CONSTITUTION.md](CONSTITUTION.md), and
the active planning pair: [CURRENT_STATUS.md](CURRENT_STATUS.md) plus
[NEXT_TASKS.md](NEXT_TASKS.md).

## Toolchain

- Use the pinned toolchain in `rust-toolchain.toml`.
- Keep formatting compatible with `rustfmt.toml`.
- Add native interfaces under `idl/` and regenerate bindings.

## Local Checks

```bash
cargo fmt --all --check
just codegen
just clippy
just preflight
```

`just preflight` runs IDL lint/codegen, strict lint tranches, host tests, and the
Foundry umbrella. Run the narrow slice gate while iterating and the full
preflight before pushing when practical.

## Change Discipline

- Preserve kernel, services, and Store ownership boundaries.
- Keep capability validation for fast-path operations in the kernel.
- Pair each new capability with a consumer and a Foundry gate.
- Use typed control messages and shared memory for bulk data.
- Do not design native APIs around POSIX or add ioctl-like escape hatches.
- For driver work, begin with the Reference Vault and Oracle traces.

## Documentation

- Update `CURRENT_STATUS.md` and `CHANGELOG.md` when a milestone lands.
- Update `NEXT_TASKS.md` only when execution order changes.
- Record design choices in `DECISIONS.md`.
- Move completed, non-gate-bound plans to `docs/archive/plans/` and repair links.
- Use evidence labels from `EVIDENCE_LEVELS.md`; do not overstate QEMU or replay
  results as live hardware proof.

## Lint Debt

Clippy warnings fail closed in strict tranches. If an `allow(...)` is genuinely
required, record its reason, owner, and exit criteria in
[docs/LINT_DEBT.md](docs/LINT_DEBT.md). Warning-tolerant baseline runs are local
only:

```bash
just clippy-baseline-soft
```
