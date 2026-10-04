---
name: new-idl
description: Create a new IDL interface definition and generate Rust bindings
disable-model-invocation: true
allowed-tools: Read, Write, Edit, Bash, Glob, Grep
---

Create the interface requested in $ARGUMENTS. Read `AGENTS.md` and
`idl/tools/README.md`; use an existing relevant IDL as a template.

1. Resolve ownership and interface purpose from context. Choose `idl/harness/`,
   `idl/portals/`, or `idl/services/`. Clarify only missing requirements that
   prevent a useful contract; routine naming choices need no confirmation.
2. Check existing IDLs and `tools/ci/idl_lint.py` for protocol ownership. Supply
   explicit nonzero `protocol` and per-message `msg_type`, plus `namespace`,
   `version`, and ordered `fields`. Never copy another interface's protocol ID.
3. Native Rust wire fields support `u8`, `u16`, `u32`, `u64`, and fixed `bytesN`
   (1–64), not signed integers or dynamic strings/slices. Keep the total payload
   bounded; use validated handles/offsets for bulk shared-memory data.
4. Register outputs in `tools/ci/run_codegen.sh`, which owns codegen for Just/CI.
   Register handwritten inclusion points such as `kernel_api/src/lib.rs` using
   the existing pattern. Never hand-edit generated files or aggregators.
5. Write the consumer's Foundry assertion before its handler, including denial,
   version, length, reserved-field, and range cases relevant to the contract.
6. Run `just codegen`, `just idl-lint`, and `cargo build -p kernel_api`; inspect
   generated diffs and affected producer/consumer checks.
7. Report spec/output paths, checks, and remaining implementation scope. Generated
   syntax is not evidence of authority enforcement or target availability.
