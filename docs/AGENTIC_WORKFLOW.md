# Agentic Workflow & Guardrails

**Last Updated:** 2026-09-16
**Status:** Contributor workflow; tooling checks are not OS security boundaries

RamenOS uses AI coding agents within the same vertical-slice and evidence
requirements as other contributors. [AGENTS.md](../AGENTS.md) is the stable
agent contract; [Current Status](../CURRENT_STATUS.md) and
[Next Tasks](../NEXT_TASKS.md) own landed state and execution order. The
[Agent Task Proof](plans/2026-09-16-agent-task-proof.md) is a separate, planned
experiment about agents using the OS. Development with agents does not by
itself demonstrate the OS thesis or a measured development-speed advantage.

## Local hooks

[`.claude/settings.json`](../.claude/settings.json) configures hooks for Claude
`Edit` and `Write` tool events:

- Post-tool hooks attempt `rustfmt` on Rust files and package-level Clippy,
  excluding the target boot crates from that Clippy invocation.
- Pre-tool hooks reject those direct edits to `Cargo.lock`, `*.generated.rs`,
  and `CONSTITUTION.md`, with guidance on the intended workflow.

These are client-specific development checks. They do not cover arbitrary shell
writes or every agent client, and their output/exit handling does not guarantee
that lint failures block work. They are not a sandbox or hardware enforcement.
Generate bindings with `just codegen`, update lockfiles through Cargo, and follow
[AGENTS.md](../AGENTS.md) for Constitution changes regardless of hook availability.

## Review and validation

Review must check kernel/service/Store boundaries, native IDL contracts,
capability validation, negative behavior, and the evidence needed for the claim.
Local reviewer configurations can help organize that work; they do not grant
approval or merge authority. Follow the separate author/reviewer identities in
[the PR workflow](org/RAMEN_IMPLEMENTER_BOT.md).

Run the relevant Foundry gate for each change. `just preflight` performs format,
codegen, IDL, target-build, strict lint, host test, umbrella, and extended Foundry
checks; CI's path-scoped merge gate decides which checks a PR requires. A green
hook is not a substitute for those checks, and a host/QEMU pass is not hardware
graduation. Report commands actually run, skipped checks, and environment limits.

## Slice workflow

1. Define a bounded behavior and its consumer, using the authoritative task queue.
2. Write a Foundry assertion, including failure and denial cases, before the
   implementation. Register the gate in the `justfile`.
3. Define new native operations in IDL and generate their bindings.
4. Implement the smallest path across the intended ownership boundary.
5. Run the relevant gates and inspect the result and evidence level.
6. Update Current Status and the changelog when the milestone lands.

The repository's `new-slice`, `new-idl`, and `foundry-gate` skills assist this
workflow. Their use does not make agent behavior deterministic or remove the
need to inspect the implementation and its evidence.
