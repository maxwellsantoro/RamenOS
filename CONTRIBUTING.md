# Contributing

**Last Updated:** 2026-10-04
**Status:** Active

RamenOS is being built as an everyday, post-Unix OS for humans and AI agents
through small, evidence-bearing vertical slices. Start with [AGENTS.md](AGENTS.md),
[CURRENT_STATUS.md](CURRENT_STATUS.md), and [NEXT_TASKS.md](NEXT_TASKS.md), then
read the subsystem contract from [the docs index](docs/INDEX.md).
[VISION.md](VISION.md) owns the destination and [CONSTITUTION.md](CONSTITUTION.md)
the architectural invariants.

## Toolchain

- Use the pinned toolchain in `rust-toolchain.toml`.
- Keep formatting compatible with `rustfmt.toml`.
- Add native interfaces under `idl/` and regenerate bindings.

## Delivery and checks

Choose a ready packet with one bounded behavior, a real consumer, and an observable
completion signal. Write behavior, denial, and failure assertions before the
implementation. Run its focused gate during development; test affected consumers
and shared resources before handoff. For native contract changes, run
`just codegen` and `just idl-lint`, then inspect the generated diff. Never edit
generated outputs manually.

Run `cargo fmt --all --check` and the relevant lint checks for Rust changes.
The integrating owner runs the required combined checks once the reviewed packets
are together; individual worker passes do not establish integration success.

`just preflight` checks Linux evaluator prerequisites, then format/codegen/IDL,
target builds, strict lint, host tests, and umbrella/extended Foundry suites.
Missing Linux, JSON-schema support, Docker/seccomp, or the pinned image reports
INCOMPLETE. Follow
[Getting Started](docs/GETTING_STARTED.md) for environment-specific checks.
Report the exact commands, result, tested revision, retained evidence, and
environment limitations. Resolve current recipes from [justfile](justfile).

## Working as a team

Use [Agentic Workflow](docs/AGENTIC_WORKFLOW.md) for concurrent agent work. The
coordinator dispatches ready packets after contracts and assertions are in place,
assigns non-overlapping write scopes and shared-file ownership, and integrates
independently reviewed changes. Keep fixed gate outputs and lab devices under one
owner. Replan when a dependency or contract changes instead of letting workers
invent incompatible interfaces.

PRs follow [the implementer-bot workflow](docs/org/RAMEN_IMPLEMENTER_BOT.md):
the A2 bot authors; a distinct authorized A3 identity approves and merges.
Local agent review does not supply that approval. The path classifier determines
CI requirements; documentation scope is not a waiver for failing governance checks.

## Documentation

- Update `CURRENT_STATUS.md` and `CHANGELOG.md` when a milestone lands.
- Update `NEXT_TASKS.md` when tasks complete or execution order/dependencies change.
- Record design choices in `DECISIONS.md`.
- Keep each fact in its authoritative document and link to it from instructions,
  skills, and subsystem guides. See [the docs index](docs/INDEX.md) for ownership
  and archive rules; inspect code/gate references before moving historical files.
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
