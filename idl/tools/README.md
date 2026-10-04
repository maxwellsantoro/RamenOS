# IDL Tools

**Last Updated:** 2026-10-03
**Status:** Current generator and contract reference

IDL TOML files are the canonical typed native control interfaces. Harnesses
live in [harness/](../harness/), user-facing portals in [portals/](../portals/),
and service protocols in [services/](../services/). Large data travels through
validated shared-memory contracts rather than pointer-bearing wire messages.

## Required schema

Every interface requires `namespace`, `version`, and a nonzero `protocol`.
Every message requires a nonzero `msg_type` and an ordered `fields` array.
This is the existing [ping contract](../harness/ping_harness.toml), not a protocol
number available to a new interface:

```toml
namespace = "harness.ping"
version = "1"
protocol = 1

[message.ping]
msg_type = 1
fields = ["nonce:u64"]

[message.pong]
msg_type = 2
fields = ["nonce:u64"]
```

Check existing IDLs and [idl_lint.py](../../tools/ci/idl_lint.py) before choosing
protocol/message IDs. Keep protocol ownership and request/reply routing explicit.
`just idl-lint` checks the repository contracts; generating one file is not a
substitute for that cross-interface check.

## Native wire types

| Type | Rust representation | Wire width |
|------|---------------------|------------|
| `u8` | `u8` | 1 byte |
| `u16` | `u16` | 2 bytes |
| `u32` | `u32` | 4 bytes |
| `u64` | `u64` | 8 bytes |
| `bytesN`, N from 1 to 64 | `[u8; N]` | N bytes |

Native Rust IPC generation rejects dynamic `string` and `bytes` fields. Some
language mappers recognize pointer-style types, but that does not make them a
valid fixed native wire contract. There are no nested structs or signed native
field types in the current generator. Use handles, fixed IDs, lengths, and offsets
for data-plane objects; validate the referenced object and range at consumption.

The generator emits `repr(C)` structs and protocol/message constants, not a
per-field serialization codec. Current `kernel_api::wire` helpers copy the native
struct representation into the bounded payload and check its size on reads.
Review alignment/padding, initialized bytes, and supported target byte order; do
not infer portable encoding from `repr(C)`. The outer bridge frame has an explicit
encoding in `ipc_frame.rs`. Typed preflight, schema validation, capability checks,
and referenced-object bounds remain consumer responsibilities.

## Regeneration

From the repository root:

```bash
just codegen
just idl-lint
```

[run_codegen.sh](../../tools/ci/run_codegen.sh) is the complete output registry:
Rust `kernel_api` bindings, capsule C headers, SDK WASM imports, native-runner WASM
host bindings, and its generated module aggregator. All generated content is
owned by this script and the generator.

For a single output while debugging:

```bash
cargo run -p idl_codegen -- \
  --in idl/harness/ping_harness.toml \
  --out kernel_api/src/generated/ping_harness.generated.rs
```

The generator requires `--in` and `--out`. `--lang` selects `rust`, `c`,
`wasm-imports`, or `wasm-host`; absent `--lang`, `.h` selects C and other output
extensions select Rust. See [idl_codegen](../../idl_codegen/src/main.rs) for the
actual parser and emitters.

## Adding or evolving a contract

1. Define ownership, request authority, observable authority, and the Foundry
   assertion before implementing the handler. Native fast-path capability
   validation belongs in the kernel; user-space brokers decide grants.
2. Add the IDL in the appropriate directory with explicit IDs, version, typed
   fields, error results, and any reserved-field rules.
3. Register the required outputs in `run_codegen.sh`. Register handwritten
   inclusion points such as [kernel_api/src/lib.rs](../../kernel_api/src/lib.rs)
   where necessary. Never edit a generated module or aggregator by hand.
4. Regenerate and run `just idl-lint`. Review both the IDL and generated diff.
5. Test a real producer/consumer and malformed, oversized, unauthorized, and
   unsupported-version requests. Test affected consumers when changing a contract.

Changing field order, width, count, or message meaning changes the wire contract.
Introduce a new version and migration plan for incompatible changes. An added
message still needs explicit consumer compatibility; adding a field named
`reserved` to an existing message changes its layout too. Reserved values already
present in a contract can evolve only under that contract's stated rules.

Generated files are committed for reproducibility. Regeneration is authorized;
manual edits to `*.generated.rs` or `generated/` are not. Use
[Constitution](../../CONSTITUTION.md), [Agent Instructions](../../AGENTS.md), and
[Decisions](../../DECISIONS.md) for architecture constraints.
