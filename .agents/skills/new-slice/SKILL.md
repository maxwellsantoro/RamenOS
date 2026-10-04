---
name: new-slice
description: Scaffold a new vertical slice with Foundry gate, IDL spec, and implementation stub
disable-model-invocation: true
allowed-tools: Read, Write, Edit, Bash, Glob, Grep
---

Scaffold the slice requested in $ARGUMENTS. Read the status/task pair, `AGENTS.md`,
the relevant `SLICES.md` definition, and `docs/research/SLICE_NAMESPACING.md`.
Extend an allocated slice when that is the requested work; do not invent a new
slice or queue merely to split a task among agents.

1. Define a bounded OS behavior or typed contract, a real consumer, and a Foundry
   assertion. A Store consumer is useful when relevant; the Constitution does
   not require every slice to implement a Store feature.
2. Choose the OS/research/governance namespace without reusing allocated IDs.
   Record actual code dependencies separately from evidence needed for a claim.
   In a team, freeze the contract and assign shared files with the coordinator.
3. Write gate assertions before implementation. An empty scaffold must fail with
   an explicit unimplemented assertion, not return PASS with zero tests. Avoid
   `((counter++))` under `set -e` because its first result is zero.
4. Add the gate recipe to `justfile` through its assigned integration owner.
   Define new native interfaces through IDL; use `new-idl` when applicable.
   Contract assertions and ID allocation precede dependent worker fan-out.
5. Add the smallest compiling stub in the owning crate. Preserve kernel/service/
   Store boundaries and default-off host development features.
6. Run the new assertion and record expected failures honestly. Scaffold completion
   does not satisfy the behavior's definition of done.
7. Hand off scope, paths, contract/gate evidence, and remaining work. The shared
   documentation owner updates slice definitions and next work, with status and
   changelog entries only for the milestone actually achieved. A solo author
   owns these steps directly.

If interface purpose or external authority is missing, continue independent
scaffolding and clarify the actual blocker. Do not invent grants or physical evidence.
