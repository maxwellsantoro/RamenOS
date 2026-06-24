# IDL Tools

Interface Definition Language (IDL) contracts define versioned, typed message formats for RamenOS inter-process communication (IPC). This document describes the IDL schema, code generation workflow, and best practices for creating and maintaining interface definitions.

## Overview

### Purpose

The IDL system provides:

- **Type-safe IPC**: Compile-time verification of message structures across kernel and user-space boundaries
- **Versioned interfaces**: Explicit versioning enables backward-compatible protocol evolution
- **Multi-language support**: Single source of truth generates Rust and C bindings
- **Documentation**: IDL files serve as canonical protocol documentation

### Architecture Integration

RamenOS uses IDL contracts to enforce the "kernel != services != store" boundary principle. All cross-component communication flows through typed IDL-defined messages:

```
+------------------+     IDL Messages      +------------------+
|    Kernel        | <-------------------> |    Services      |
|  (Harnesses)     |                       |  (Portals)       |
+------------------+                       +------------------+
         ^                                          ^
         | IDL Messages                             | IDL Messages
         v                                          v
+------------------+                       +------------------+
|  kernel_api/     |                       |  Store Service   |
|  generated/      |                       |                  |
+------------------+                       +------------------+
```

### Interface Categories

| Category | Directory | Description |
|----------|-----------|-------------|
| **Harnesses** | `idl/harness/` | Kernel-level service interfaces (IPC, memory management, tracing) |
| **Portals** | `idl/portals/` | User-space capability-gated interfaces (clipboard, file picker, notifications) |
| **Services** | `idl/services/` | Service-to-service interfaces (artifact storage, domain management) |

## TOML Schema Specification

### File Structure

Every IDL file is a TOML document with the following structure:

```toml
namespace = "<category>.<name>"
version = "<major>"

[message.<message_name>]
fields = ["<field1>:<type1>", "<field2>:<type2>", ...]

[message.<another_message>]
fields = [...]
```

### Top-Level Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `namespace` | string | Yes | Dot-separated identifier: `<category>.<name>` |
| `version` | string | Yes | Major version number (e.g., "1", "2") |

### Message Sections

Each message is defined in a `[message.<name>]` section:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `fields` | array | Yes | List of field definitions in `name:type` format |

### Field Definition Format

Fields are specified as strings in the format `"name:type"`:

```toml
fields = ["request_id:u64", "status:u32", "payload_len:u32"]
```

**Naming conventions:**
- Use `snake_case` for field names
- Use descriptive names that convey purpose
- Follow request/reply pairing conventions (see Best Practices)

### Supported Types

| IDL Type | Rust Type | C Type | Description |
|----------|-----------|--------|-------------|
| `u8` | `u8` | `uint8_t` | 8-bit unsigned integer |
| `u16` | `u16` | `uint16_t` | 16-bit unsigned integer |
| `u32` | `u32` | `uint32_t` | 32-bit unsigned integer |
| `u64` | `u64` | `uint64_t` | 64-bit unsigned integer |
| `string` | `&'static str` | `const char*` | Static string reference |
| `bytes` | `&'static [u8]` | `const uint8_t*` | Static byte slice reference |

**Type limitations:**
- `string` and `bytes` types generate static lifetime references, suitable for constant data
- No nested structs or arrays in the current schema
- All numeric types are unsigned only

## Code Generation Workflow

### Running the Code Generator

The IDL code generator is a command-line tool located at [`idl_codegen/src/main.rs`](../../idl_codegen/src/main.rs):

```bash
# Generate Rust code
cargo run --manifest-path idl_codegen/Cargo.toml -- \
    --in idl/harness/ping_harness.toml \
    --out kernel_api/src/generated/ping_harness.generated.rs

# Generate C header
cargo run --manifest-path idl_codegen/Cargo.toml -- \
    --in idl/harness/ping_harness.toml \
    --out include/ping_harness.h
```

### Command-Line Options

| Option | Required | Description |
|--------|----------|-------------|
| `--in <path>` | Yes | Input IDL TOML file path |
| `--out <path>` | Yes | Output file path (extension determines language) |
| `--lang <lang>` | No | Output language: `rust` or `c` (inferred from `--out` extension) |

### Language Inference

The generator infers the output language from the file extension:

- `.h` extension -> C header
- Any other extension -> Rust

### Input/Output Mapping

```
idl/harness/ping_harness.toml
    |
    v  [idl_codegen]
kernel_api/src/generated/ping_harness.generated.rs
```

### Build Integration

Generated files are committed to the repository. To regenerate all IDL outputs:

```bash
# Example: regenerate all harness interfaces
for f in idl/harness/*.toml; do
    name=$(basename "$f" .toml)
    cargo run --manifest-path idl_codegen/Cargo.toml -- \
        --in "$f" \
        --out "kernel_api/src/generated/${name}.generated.rs"
done
```

## Example: Ping Harness Walkthrough

### IDL Definition

[`idl/harness/ping_harness.toml`](../harness/ping_harness.toml):

```toml
namespace = "harness.ping"
version = "1"

[message.ping]
fields = ["nonce:u64"]

[message.pong]
fields = ["nonce:u64"]
```

### Breakdown

| Element | Value | Purpose |
|---------|-------|---------|
| `namespace` | `"harness.ping"` | Identifies this as a kernel harness interface for ping protocol |
| `version` | `"1"` | Major version 1 of this interface |
| `[message.ping]` | | Request message type |
| `fields` | `["nonce:u64"]` | Single 64-bit nonce field for request correlation |
| `[message.pong]` | | Reply message type |
| `fields` | `["nonce:u64"]` | Echoes the nonce back for correlation |

### Generated Rust Code

[`kernel_api/src/generated/ping_harness.generated.rs`](../../kernel_api/src/generated/ping_harness.generated.rs):

```rust
// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/ping_harness.toml

// namespace = harness.ping, version = 1

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Ping {
    pub nonce: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Pong {
    pub nonce: u64,
}
```

### Key Generation Details

1. **Header comment**: Documents source file and metadata
2. **`#[repr(C)]`**: Ensures C-compatible memory layout for IPC
3. **`#[derive(Copy, Clone, Debug)]`**: Adds essential traits for message handling
4. **Pascal case conversion**: Message names `ping` -> `Ping`, `pong` -> `Pong`
5. **Public fields**: All fields are `pub` for direct access

## Example: Domain Manager v1 Walkthrough

A more complex example showing request/reply patterns:

### IDL Definition

[`idl/harness/domain_manager_v1.toml`](../harness/domain_manager_v1.toml) (excerpt):

```toml
namespace = "domain.manager"
version = "1"

[message.start_domain]
fields = ["request_id:u64", "domain_id:u64", "runner_kind:u32", "restart_policy:u32"]

[message.start_domain_reply]
fields = ["request_id:u64", "domain_id:u64", "status:u32", "generation:u32"]

[message.stop_domain]
fields = ["request_id:u64", "domain_id:u64"]

[message.stop_domain_reply]
fields = ["request_id:u64", "domain_id:u64", "status:u32", "generation:u32"]
```

### Request/Reply Pattern

Each operation follows a consistent pattern:

1. **Request message**: Contains `request_id` for correlation, operation parameters
2. **Reply message**: Contains matching `request_id`, operation results, `status` field

### Generated Structures

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StartDomain {
    pub request_id: u64,
    pub domain_id: u64,
    pub runner_kind: u32,
    pub restart_policy: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StartDomainReply {
    pub request_id: u64,
    pub domain_id: u64,
    pub status: u32,
    pub generation: u32,
}
```

## Creating New Interfaces

### Step-by-Step Guide

1. **Choose category and namespace**:
   - Harness: `harness.<name>` for kernel interfaces
   - Portal: `portal.<name>` for user-space capability interfaces
   - Service: `<name>.service` or `<domain>.<name>` for service-to-service

2. **Create the TOML file**:
   ```bash
   touch idl/harness/my_interface_v1.toml
   ```

3. **Define the interface**:
   ```toml
   namespace = "harness.my_interface"
   version = "1"
   
   [message.my_request]
   fields = ["request_id:u64", "param1:u32", "param2:u32"]
   
   [message.my_request_reply]
   fields = ["request_id:u64", "status:u32", "result:u64"]
   ```

4. **Generate the code**:
   ```bash
   cargo run --manifest-path idl_codegen/Cargo.toml -- \
       --in idl/harness/my_interface_v1.toml \
       --out kernel_api/src/generated/my_interface_v1.generated.rs
   ```

5. **Include in kernel_api**:
   Add to [`kernel_api/src/lib.rs`](../../kernel_api/src/lib.rs):
   ```rust
   mod generated::my_interface_v1;
   ```

6. **Commit both files**:
   ```bash
   git add idl/harness/my_interface_v1.toml
   git add kernel_api/src/generated/my_interface_v1.generated.rs
   git commit -m "Add my_interface v1 IDL contract"
   ```

### Naming Conventions

| Element | Convention | Example |
|---------|------------|---------|
| File name | `<name>_v<version>.toml` | `domain_manager_v1.toml` |
| Namespace | `<category>.<name>` | `domain.manager` |
| Message names | `snake_case` | `start_domain` |
| Generated struct | `PascalCase` | `StartDomain` |
| Field names | `snake_case` | `domain_id` |

### Versioning Guidelines

1. **Start at version "1"** for new interfaces
2. **Increment major version** for breaking changes:
   - Removing fields
   - Changing field types
   - Changing field order
   - Removing messages

3. **Minor changes** that don't break compatibility:
   - Adding new messages (not modifying existing)
   - Adding reserved/padding fields (already in place)

4. **Create new file** for new major version:
   - `my_interface_v1.toml` -> `my_interface_v2.toml`
   - Keep old version for backward compatibility

## Supported Types Reference

### Complete Type Mapping Table

| IDL Type | Rust Type | C Type | Size | Use Case |
|----------|-----------|--------|------|----------|
| `u8` | `u8` | `uint8_t` | 1 byte | Small flags, booleans, byte values |
| `u16` | `u16` | `uint16_t` | 2 bytes | Port numbers, small counts |
| `u32` | `u32` | `uint32_t` | 4 bytes | Status codes, lengths, IDs |
| `u64` | `u64` | `uint64_t` | 8 bytes | Request IDs, timestamps, large IDs |
| `string` | `&'static str` | `const char*` | pointer | Static string identifiers |
| `bytes` | `&'static [u8]` | `const uint8_t*` | pointer | Static binary data, capabilities |

### Type Selection Guidelines

- **IDs and counters**: Use `u64` for request IDs, domain IDs, and large counters
- **Status and error codes**: Use `u32` for status fields
- **Flags and options**: Use `u32` for flag bitmasks, or `u8` for boolean-like flags
- **Padding/reserved**: Use `u32` or appropriate size to maintain alignment
- **String identifiers**: Use `string` for content IDs, paths, names
- **Binary data**: Use `bytes` for capability tokens, signatures

### Alignment Considerations

Generated structs use `#[repr(C)]` for predictable layout. For best alignment:

- Order fields by size (largest first)
- Use explicit reserved fields to fill gaps
- Consider total struct size for cache line alignment

## Best Practices

### Message Design

1. **Always include `request_id:u64`** in request/reply pairs for correlation
2. **Always include `status:u32`** in reply messages for error reporting
3. **Use consistent field ordering** across related messages
4. **Add reserved fields** for future expansion:
   ```toml
   fields = ["request_id:u64", "param:u32", "reserved:u32"]
   ```

### Request/Reply Naming

Follow the `<operation>` / `<operation>_reply` pattern:

```toml
[message.start_domain]      # Request
fields = [...]

[message.start_domain_reply]  # Reply
fields = [...]
```

### Documentation

Add comments to explain non-obvious fields:

```toml
[message.map_region]
# rights: bitmask of MAP_READ|MAP_WRITE|MAP_EXECUTE
# cache_mode: 0=uncached, 1=cached, 2=write-combining
fields = ["request_id:u64", "region_id:u64", "rights:u32", "cache_mode:u32"]
```

### Versioning Strategy

1. **Plan for evolution**: Add reserved fields proactively
2. **Document changes**: Update comments when adding new versions
3. **Maintain compatibility**: Keep old versions during transition periods
4. **Clean migration**: Provide clear upgrade paths between versions

### Backward Compatibility

When evolving interfaces:

| Change | Breaking? | Approach |
|--------|-----------|----------|
| Add new message | No | Safe to add |
| Add field to existing message | Yes | Create new version |
| Remove field | Yes | Create new version |
| Change field type | Yes | Create new version |
| Add reserved field | No | Safe (already placeholder) |

### Code Organization

- Keep related messages in the same IDL file
- One logical interface per file
- Group request/reply pairs together
- Order messages by logical operation flow

## Future Evolution

The current IDL schema is minimal by design. Planned enhancements include:

1. **Capability discovery**: Interface metadata for runtime capability negotiation
2. **Extension points**: Optional fields and backward-compatible additions
3. **Documentation generation**: Automatic generation of protocol documentation
4. **Validation rules**: Field constraints and validation logic
5. **Async patterns**: First-class support for async request/reply patterns

See [DECISIONS.md](../../DECISIONS.md) for design decisions regarding IDL evolution.
