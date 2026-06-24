---
name: new-idl
description: Create a new IDL interface definition and generate Rust bindings
disable-model-invocation: true
allowed-tools: Read, Write, Edit, Bash, Glob, Grep
---

Create a new IDL interface for: $ARGUMENTS

## Steps

### 1. Determine interface type and name

Parse $ARGUMENTS to determine:
- **Type**: `harness` (kernel↔component contract) or `portal` (user-mediated access like file picker, clipboard)
- **Name**: snake_case name (e.g., `audio_playback`, `clipboard_v1`)

If not clear from arguments, ask the user.

### 2. Review existing IDL specs for format reference

Read one existing spec as a template:
- Harness example: `idl/harness/ping_harness.toml`
- Portal example: `idl/portals/file_picker_v1.toml`

### 3. Create the TOML spec

Write the new spec to the appropriate directory:
- Harnesses: `idl/harness/<name>.toml`
- Portals: `idl/portals/<name>.toml`

Follow the format:
```toml
namespace = "<type>.<name>"
version = "1"

[message.<message_name>]
fields = ["field_name:type", ...]
```

Supported field types: `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`.

### 4. Add codegen entry to justfile

Add a new `cargo run -p idl_codegen` line to the `codegen` recipe in `justfile`:
```
cargo run -p idl_codegen -- \
  --in idl/<type>/<name>.toml \
  --out kernel_api/src/generated/<name>.generated.rs
```

### 5. Register the generated module

Add `pub mod <name>;` (using the generated filename without `.generated.rs`) to the appropriate module file in `kernel_api/src/generated/`.

If there is a `mod.rs` in `kernel_api/src/generated/`, add it there. Otherwise, check how existing generated modules are imported and follow the same pattern.

### 6. Run codegen and verify

```sh
just codegen
cargo build -p kernel_api
```

### 7. Report

Print a summary:
- Spec location
- Generated file location
- Next steps (implement handler, write Foundry gate)
