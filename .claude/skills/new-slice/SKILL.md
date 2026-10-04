---
name: new-slice
description: Scaffold a new vertical slice with Foundry gate, IDL spec, and implementation stub
disable-model-invocation: true
allowed-tools: Read, Write, Edit, Bash, Glob, Grep
---

Scaffold the slice requested in $ARGUMENTS. Read `CURRENT_STATUS.md`,
`NEXT_TASKS.md`, `SLICES.md`, `AGENTS.md`, and `docs/research/SLICE_NAMESPACING.md`.
Use `ROADMAP.md` for direction, not the executable queue.

1. Define a bounded OS behavior or typed contract, a real consumer, and a Foundry
   assertion. A Store consumer is useful when relevant; the Constitution does
   not require every slice to implement a Store feature.
2. Choose the existing OS/research/governance namespace without reusing allocated
   identifiers. Preserve independent lane prerequisites and authority limits.
3. Write gate assertions before implementation. An empty scaffold must fail with
   an explicit unimplemented assertion, not return PASS with zero tests. Avoid
   `((counter++))` under `set -e` because its first result is zero.
4. Add the gate recipe to `justfile`. Define new native interfaces through IDL
   and `tools/ci/run_codegen.sh`; use the `new-idl` workflow when applicable.
5. Add the smallest compiling stub in the owning crate. Preserve kernel/service/
   Store boundaries and default-off host development features.
6. Run the new assertion and record expected failures honestly. Scaffold completion
   does not satisfy the behavior's definition of done.
7. Update slice definitions and next work; update status/changelog only for the
   milestone actually achieved. Report scope, paths, validation and remaining work.

If interface purpose or external authority is missing, continue independent
scaffolding and clarify the actual blocker. Do not invent grants or physical evidence.
