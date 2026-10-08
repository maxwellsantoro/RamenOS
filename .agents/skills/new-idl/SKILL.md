---
name: new-idl
description: Create a new IDL interface definition and generate Rust bindings
---

Create or evolve the requested interface. Read `AGENTS.md` and
`idl/tools/README.md`; use a relevant existing IDL for the current syntax and wire
limits. Generated syntax is not evidence of enforcement or target availability.

1. Resolve ownership and interface purpose from context. Choose `idl/harness/`,
   `idl/portals/`, or `idl/services/`. Clarify only missing requirements that
   prevent a useful contract; routine naming choices need no confirmation.
2. Check existing IDLs and `tools/ci/idl_lint.py` for protocol ownership. Allocate
   protocol/message IDs and freeze the producer/consumer contract with the
   coordinator before parallel implementation. Never copy another protocol's ID.
3. Specify bounded fields, error results, request and observable authority, and
   referenced-object lifetime/range checks. Review alignment and initialized
   padding; use the supported wire types from the generator reference. Version
   incompatible layouts or meanings and identify affected consumers.
4. Write consumer assertions before handlers: behavior, denial, invalid version,
   length, reserved fields, and ranges as applicable. An interface-only scaffold
   must not claim these consumers are implemented.
5. Register outputs in `tools/ci/run_codegen.sh` and handwritten inclusion points
   such as `kernel_api/src/lib.rs`. In a team, send these shared edits to their
   assigned owner; that owner runs codegen once the batch's IDLs are settled.
   Never hand-edit generated files or aggregators.
6. Run `just codegen`, `just idl-lint`, `cargo build -p kernel_api`, and affected
   producer/consumer checks under that ownership. Inspect generated diffs.
7. Report contract/output paths, compatibility decisions, checks and evidence,
   and remaining implementation scope through the assigned handoff.
