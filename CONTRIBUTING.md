# Contributing

**Last Updated:** 2026-10-03
**Status:** Active

RamenOS is being built as an everyday, post-Unix OS for humans and AI agents,
through small, evidence-bearing vertical slices. Read [Vision](VISION.md) for
the product direction. Before changing a subsystem, read [AGENTS.md](AGENTS.md),
[CONSTITUTION.md](CONSTITUTION.md), and
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

`just preflight` checks Linux evaluator prerequisites, then format/codegen/IDL,
target builds, strict lint, host tests, and umbrella/extended Foundry suites.
Missing Linux, JSON-schema support, Docker/seccomp, or the pinned image reports
INCOMPLETE. Use the focused slice gate while iterating and follow
[Getting Started](docs/GETTING_STARTED.md) for environment-specific checks.

## Change Discipline

- Preserve kernel, services, and Store ownership boundaries.
- Keep capability validation for fast-path operations in the kernel.
- Pair each new capability with a consumer and a Foundry gate.
- Use typed control messages and shared memory for bulk data.
- Do not design native APIs around POSIX or add ioctl-like escape hatches.
- For driver work, begin with the Reference Vault and Oracle traces.
- Connect each slice to a human, agent, hardware, or developer need. Independent
  components still need conformance and recovery checks with affected consumers.
- Keep core human interactions usable without an AI model; agent assistance
  follows explicit policy and bounded grants.

## Documentation

- Preserve the shared framing in `VISION.md`: an everyday OS for humans and AI
  agents. Describe speed, safety, adaptability, and ease of use as goals until
  supported by matching evidence; distinguish pre-alpha status from the destination.
- Update `CURRENT_STATUS.md` and `CHANGELOG.md` when a milestone lands.
- Update `NEXT_TASKS.md` when tasks complete or execution order/dependencies change.
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
