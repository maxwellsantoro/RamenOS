# V-006 Phase 4: Native Runner Design

**Last Updated:** 2026-02-18
**Status:** Approved
**Related:** V-006 Security Remediation, S8 Shared Memory, V-012 Trace Service

---

## Executive Summary

This document defines the architecture for RamenOS's native personality runner, which uses WebAssembly as pure bytecode with RamenOS-native semantics. This replaces the deprecated POSIX runner v0, eliminating shell script execution risks while providing typed, capability-backed execution.

**Key Design Decisions:**
- WASM as native bytecode format (not WASI compatibility layer)
- Direct harness/portal imports (no POSIX abstractions)
- Capability-validated execution (kernel fast-path)
- Zero-copy shared memory for data plane

---

## 1. Overview & Goals

### Objective

Design and specify a native personality runner that uses WebAssembly as pure bytecode with RamenOS-native semantics, replacing the deprecated POSIX runner v0.

### Design Goals

| Goal | Rationale |
|------|-----------|
| **Typed Harness Imports** | WASM modules import harness endpoints directly, not POSIX file descriptors |
| **Capability-Backed Execution** | Every import requires a valid capability; kernel validates on fast-path |
| **Zero-Copy Data Plane** | Bulk data via shared memory regions, not copied through WASM linear memory |
| **IDL-Generated Bindings** | All native interfaces defined in `/idl`, code-generated for WASM targets |
| **No POSIX Baggage** | No fd_write, path_open, or Unix concepts in the native ABI |

### What This Is NOT

- NOT a WASI compliance layer (that's a separate compatibility runner)
- NOT a POSIX personality (that's the deprecated runner we're replacing)
- NOT a generic WebAssembly runtime (this is RamenOS-native execution)

### Success Criteria

1. A "hello world" native personality can be compiled with a RamenOS SDK
2. The runner executes WASM with capability-validated harness calls
3. Data-plane operations use zero-copy shared memory
4. Foundry gate validates end-to-end execution

---

## 2. Architecture

### Component Overview

```
+-----------------------------------------------------------------------------+
|                           Native Runner Service                              |
+-----------------------------------------------------------------------------+
|                                                                              |
|  +--------------+    +--------------+    +------------------------------+   |
|  |   WASM       |    |  Capability  |    |      IDL-Generated           |   |
|  |   Runtime    |<-->|   Bridge     |<-->|      Harness Stubs           |   |
|  |  (wasmtime)  |    |              |    |   (compiled to WASM imports)  |   |
|  +------+-------+    +------+-------+    +------------------------------+   |
|         |                   |                                                |
|         |                   v                                                |
|         |          +--------------+                                         |
|         |          |   Kernel     |                                         |
|         +--------->|  Capability  |<-------- Validation fast-path          |
|                    |    Table     |                                         |
|                    +--------------+                                         |
|                                                                              |
+----------------------------------+-------------------------------------------+
                                   |
                                   v
+------------------------------------------------------------------------------+
|                              Kernel Space                                     |
+------------------------------------------------------------------------------+
|                                                                               |
|   +-------------+   +-----------------+   +----------------------------+    |
|   |   IPC v0    |   |   Shared Mem    |   |     Capability Table       |    |
|   |  (Control)  |   |   (Data Plane)  |   |     (Validation)           |    |
|   +-------------+   +-----------------+   +----------------------------+    |
|                                                                               |
+------------------------------------------------------------------------------+
```

### Execution Flow

```
+-----------------------------------------------------------------------------+
|                         Native WASM Execution Flow                           |
+-----------------------------------------------------------------------------+
|                                                                              |
|  1. LOAD                                                                     |
|     +---------+    +-------------+    +-----------------------+            |
|     |  Store  |--->|  Validate   |--->|  Instantiate WASM     |            |
|     | Artifact|    |  Signature  |    |  Module               |            |
|     +---------+    +-------------+    +-----------------------+            |
|                                                |                             |
|  2. CAPABILITY SETUP                          v                             |
|     +-----------------+    +---------------------------+                   |
|     |  Grant Caps     |<---|  Broker evaluates policy  |                   |
|     |  to Instance    |    |  based on manifest        |                   |
|     +-----------------+    +---------------------------+                   |
|            |                                                                 |
|            v                                                                 |
|  3. EXECUTE                                                                  |
|     +-----------------+    +---------------------------+                   |
|     | WASM calls      |--->| Capability Bridge checks  |                   |
|     | harness import  |    | cap handle + rights       |                   |
|     +-----------------+    +---------------------------+                   |
|                                     |                                        |
|                                     v                                        |
|     +-----------------+    +---------------------------+                   |
|     | Return result   |<---| Kernel validates cap,     |                   |
|     | to WASM         |    | executes IPC operation    |                   |
|     +-----------------+    +---------------------------+                   |
|                                                                              |
+-----------------------------------------------------------------------------+
```

### Key Components

| Component | Responsibility |
|-----------|---------------|
| **WASM Runtime** | Executes WASM bytecode; provides linear memory isolation |
| **Capability Bridge** | Maps WASM import calls to kernel capability validation |
| **Harness Stubs** | IDL-generated WASM imports for each harness/portal |
| **Kernel CapTable** | Validates capabilities on fast-path (existing) |

---

## 3. WASM Native ABI

### Design Principle

The RamenOS Native ABI treats WASM imports as direct harness calls. Each import is a typed operation backed by a capability handle. No POSIX abstractions leak through.

### Import Naming Convention

```
(import "ramen::<namespace>::<interface>" "<method>" (func ...))
```

**Examples:**
```wat
;; Harness imports (kernel-mediated)
(import "ramen::harness::echo_v1" "send"
  (func $echo_send (param $cap_handle i64) (param $msg_ptr i32) (param $msg_len i32) (result i32)))

(import "ramen::harness::shmem_control_v1" "create_region"
  (func $shmem_create (param $cap_handle i64) (param $size i64) (result i64)))

;; Portal imports (user-mediated)
(import "ramen::portal::file_picker_v1" "pick_ro"
  (func $file_pick (param $cap_handle i64) (param $hint_ptr i32) (param $hint_len i32) (result i32)))
```

### Capability Handle Convention

All imports that require authorization take a **capability handle** as the first parameter:

| Type | Meaning |
|------|---------|
| `i64` | Capability handle (64-bit: kind + generation + index) |
| `i32` | Return status code or offset into linear memory |
| `i64` | Sizes, offsets, or large scalar values |

### Return Status Codes

```rust
// Defined in kernel_api, exported to WASM SDK
pub const STATUS_OK: i32 = 0;
pub const STATUS_INVALID_CAPABILITY: i32 = 1;
pub const STATUS_PERMISSION_DENIED: i32 = 2;
pub const STATUS_INVALID_ARGUMENT: i32 = 3;
pub const STATUS_WOULD_BLOCK: i32 = 4;
pub const STATUS_IO_ERROR: i32 = 5;
```

### IDL to WASM Mapping

Each IDL message type generates a WASM import signature:

**IDL Definition (`idl/harness/echo_v1.toml`):**
```toml
namespace = "harness.echo"
version = "1"

[message.send]
fields = ["request_id:u64", "payload:bytes"]

[message.send_reply]
fields = ["request_id:u64", "status:u32", "echo_len:u32"]
```

**Generated WASM Import:**
```wat
(import "ramen::harness::echo_v1" "send"
  (func (param $cap i64)              ;; capability handle
        (param $request_id i64)       ;; request_id
        (param $payload_ptr i32)      ;; payload bytes offset
        (param $payload_len i32)      ;; payload bytes length
        (result i32)))                ;; status code
        ;; reply written to linear memory at caller-provided offset
```

### Memory Convention

- **Linear Memory:** WASM module owns its linear memory (minimum 1 page, can grow)
- **Buffers:** Caller provides pointer + length for all variable-length data
- **Replies:** Caller pre-allocates buffer; callee writes and returns actual length

```wat
;; Example: get_info writes to caller's buffer
(import "ramen::harness::trace_v1" "get_info"
  (func (param $cap i64)
        (param $out_ptr i32)          ;; caller-allocated buffer
        (param $out_cap i32)          ;; buffer capacity
        (result i32)))                ;; actual bytes written (or error)
```

### No Implicit State

Unlike WASI, there are no implicit file descriptors or pre-opened directories. Every resource access requires an explicit capability handle:

```wat
;; WRONG: WASI style (implicit fd 3 for stdout)
(fd_write (i32.const 1) ...)  ;; fd 1 = stdout

;; CORRECT: RamenOS style (explicit capability)
(import "ramen::harness::console_v1" "write"
  (func $console_write (param $cap i64) (param $buf i32) (param $len i32) (result i32)))
```

---

## 4. Capability Model

### Capability Lifecycle for WASM Instances

```
+------------------------------------------------------------------------------+
|                    WASM Instance Capability Lifecycle                         |
+------------------------------------------------------------------------------+
|                                                                               |
|  +-------------+     +-------------+     +-------------+     +-----------+   |
|  |   MANIFEST  |---->|   BROKER    |---->|   KERNEL    |---->|  INSTANCE |   |
|  |  Declares   |     |  Evaluates  |     |   Grants    |     |  Receives |   |
|  |  Needed Caps|     |   Policy    |     |   Handles   |     |   Caps    |   |
|  +-------------+     +-------------+     +-------------+     +-----------+   |
|                                                                               |
+------------------------------------------------------------------------------+
```

### Manifest Declaration

Native WASM artifacts declare required capabilities in their manifest:

```json
{
  "kind": "native_wasm_v0",
  "content_id": "sha256:abc123...",
  "native_wasm": {
    "entrypoint": "_start",
    "required_capabilities": [
      {
        "interface": "harness::echo_v1",
        "rights": ["send"],
        "purpose": "Echo test harness"
      },
      {
        "interface": "harness::trace_v1",
        "rights": ["read"],
        "purpose": "Log execution traces"
      },
      {
        "interface": "portal::file_picker_v1",
        "rights": ["pick_ro"],
        "purpose": "Open user-selected files"
      }
    ]
  }
}
```

### Capability Grant Flow

```
1. LOAD ARTIFACT
   +-- Store service validates manifest signature
   +-- Extract required_capabilities list

2. POLICY EVALUATION (Capability Broker)
   +-- Broker checks: Is this artifact allowed these capabilities?
   +-- Factors: channel (experimental/candidate/stable), observed behavior, admin policy
   +-- Output: granted_capabilities subset

3. KERNEL GRANT
   +-- For each granted capability:
       |-- Allocate handle in CapTable
       |-- Bind to specific interface + rights
       +-- Return handle to runner

4. INSTANCE SETUP
   +-- Runner injects capability table into WASM instance
   +-- WASM code receives handles via initialization export
```

### Instance Capability Table

Each WASM instance has a local capability table, populated at startup:

```rust
// Runner-maintained per-instance state
struct InstanceCapTable {
    /// Handle -> (interface, rights, kernel_handle)
    caps: HashMap<u32, (InterfaceId, RightsMask, KernelHandle)>,
}

// Injected into WASM via initialization export
#[export_name = "ramen_init"]
pub extern "C" fn ramen_init(
    cap_count: u32,
    cap_handles_ptr: *const u64,  // Array of kernel capability handles
    cap_ifaces_ptr: *const u32,   // Array of interface IDs
    cap_rights_ptr: *const u32,   // Array of rights masks
)
```

### Validation on Each Call

```
WASM calls harness import
        |
        v
+---------------------+
|  1. Extract cap_handle from param   |
|  2. Lookup in InstanceCapTable      |
|  3. Check rights mask allows op     |
+---------------------+
        |
        v
+---------------------+
|  4. Forward to kernel with          |
|     kernel_handle                   |
|  5. Kernel validates handle         |
|     (fast-path, constant-time)      |
|  6. Execute IPC operation           |
+---------------------+
        |
        v
    Return result to WASM
```

### Rights Mask Format

```rust
// Per-interface rights, defined in IDL
pub const RIGHTS_NONE: u32 = 0x00;
pub const RIGHTS_READ: u32 = 0x01;
pub const RIGHTS_WRITE: u32 = 0x02;
pub const RIGHTS_ADMIN: u32 = 0x04;

// Example: trace_v1 rights
pub const TRACE_RIGHT_READ: u32 = 0x01;   // read_trace
pub const TRACE_RIGHT_WRITE: u32 = 0x02;  // emit_trace (for self-domain)
pub const TRACE_RIGHT_ADMIN: u32 = 0x04;  // read other domains
```

### Fail-Closed Semantics

| Scenario | Behavior |
|----------|----------|
| Manifest declares cap not granted | Module load fails with `CAPABILITY_DENIED` |
| WASM calls with invalid handle | Returns `STATUS_INVALID_CAPABILITY` |
| WASM calls with insufficient rights | Returns `STATUS_PERMISSION_DENIED` |
| WASM calls with revoked handle | Returns `STATUS_INVALID_CAPABILITY` |

---

## 5. Data Plane (Zero-Copy Shared Memory)

### Challenge: WASM Linear Memory Isolation

WASM modules have isolated linear memory. Direct pointer sharing isn't possible. We need a bridge that enables zero-copy without compromising isolation.

### Solution: Shared Memory Regions with Explicit Mapping

```
+------------------------------------------------------------------------------+
|                         Zero-Copy Data Plane                                  |
+------------------------------------------------------------------------------+
|                                                                               |
|   WASM Instance                         Native Service                        |
|   +-------------------+                +-------------------+                 |
|   | Linear Memory     |                | Native Memory     |                 |
|   | +---------------+ |                | +---------------+ |                 |
|   | | Private       | |                | | Private       | |                 |
|   | | 0x0000-0xFFFF | |                | |               | |                 |
|   | +---------------+ |                | +---------------+ |                 |
|   | | MAPPED REGION | |<-------------->| | MAPPED REGION | |                 |
|   | | 0x10000+      | |   Same Phys    | |               | |                 |
|   | | (shmem_cap)   | |   Frames       | | (shmem_cap)   | |                 |
|   | +---------------+ |                | +---------------+ |                 |
|   +-------------------+                +-------------------+                 |
|                                                                               |
|                      ^                       ^                                |
|                      |                       |                                |
|                      +-----------------------+                                |
|                         Kernel validates                                      |
|                         capability access                                     |
|                                                                               |
+------------------------------------------------------------------------------+
```

### Shared Memory Capability Extension

We extend `shmem_control_v1` with WASM-specific mapping:

```wat
;; Request shared memory region mapped into linear memory
(import "ramen::harness::shmem_control_v1" "map_to_linear"
  (func $shmem_map
    (param $cap i64)           ;; shmem capability handle
    (param $region_id i64)     ;; region to map
    (param $linear_offset i32) ;; where in linear memory to map
    (param $size i32)          ;; size to map
    (result i32)))             ;; status code
```

### Data Transfer Pattern

**Producer to Consumer (Zero-Copy):**

```
1. PRODUCER creates shared memory region
   +-- shmem_control_v1.create_region(size, flags) -> region_cap

2. PRODUCER writes data to region
   +-- Direct write to mapped linear memory (no copy)

3. PRODUCER sends region_cap to consumer via IPC
   +-- harness.echo_v1.send(region_cap, metadata)

4. CONSUMER maps region into own linear memory
   +-- shmem_control_v1.map_to_linear(region_cap, offset, size)

5. CONSUMER reads data directly
   +-- Direct read from mapped linear memory (no copy)
```

### Ring Buffer Integration (S8 Phase 5)

For streaming data, we reuse the S8 Phase 5 ring buffer implementation:

```rust
// Ring buffer header in shared memory
struct RingBufferHeader {
    /// Producer write index (atomic)
    write_idx: AtomicUsize,
    /// Consumer read index (atomic)
    read_idx: AtomicUsize,
    /// Buffer capacity
    capacity: usize,
    /// Data follows immediately after header
}

// WASM consumer reads from ring buffer
// (import "ramen::harness::ring_buffer_v0" "read"
//   (param $cap i64) (param $out_ptr i32) (param $out_cap i32) (result i32))
```

### Security Guarantees

| Property | Enforcement |
|----------|-------------|
| **Isolation** | WASM cannot access unmapped memory |
| **Capability check** | Kernel validates on every map operation |
| **No pointer forgery** | Only kernel-issued region_ids work |
| **Revocation** | Kernel can revoke shmem capability; future access fails |
| **Domain isolation** | Cross-domain sharing requires explicit capability grant |

### When to Use Data Plane vs Control Plane

| Data Type | Plane | Rationale |
|-----------|-------|-----------|
| Small messages (<4KB) | Control (IPC) | Overhead of shmem setup exceeds benefit |
| Large buffers (>4KB) | Data (shmem) | Zero-copy wins |
| Streaming data | Data (ring buffer) | SPSC pattern from S8 Phase 5 |
| Capability transfer | Control (IPC) | Handles passed in messages |

---

## 6. IDL Integration

### Code Generation Targets

The IDL code generator (`idl_codegen`) will support WASM as a new target alongside the existing Rust target:

| Target | Output | Purpose |
|--------|--------|---------|
| `rust` | `kernel_api/src/generated/*.rs` | Kernel and services |
| `wasm-imports` | Rust SDK imports | WASM guest code |
| `wasm-host` | Rust host functions | Native Runner service |

### IDL Extension for WASM

```toml
# idl/harness/echo_v1.toml
namespace = "harness.echo"
version = "1"

[meta]
wasm_module = "ramen::harness::echo_v1"  # Import namespace

[message.send]
fields = ["request_id:u64", "payload:bytes"]
wasm.import = "send"  # Function name in WASM

[message.send_reply]
fields = ["request_id:u64", "status:u32", "echo_len:u32"]
```

### Generated WASM Import (Rust SDK)

The SDK provides idiomatic Rust wrappers:

```rust
// Generated by idl_codegen --target wasm-imports
// File: sdk/src/generated/harness_echo_v1.rs

use ramen_sdk::{CapabilityHandle, Status, IntoLinearMemory};

/// Echo harness client (WASM guest side)
pub struct EchoV1Client {
    cap: CapabilityHandle,
}

impl EchoV1Client {
    /// Create client from granted capability
    pub fn from_cap(cap: CapabilityHandle) -> Self {
        Self { cap }
    }

    /// Send a message and receive echo
    pub async fn send(&self, payload: &[u8]) -> Result<Vec<u8>, Status> {
        extern "C" {
            #[link_name = "ramen::harness::echo_v1::send"]
            fn __ramen_echo_send(
                cap: u64,
                request_id: u64,
                payload_ptr: *const u8,
                payload_len: u32,
                reply_ptr: *mut u8,
                reply_cap: u32,
            ) -> i32;
        }

        let mut reply_buf = vec![0u8; 4096];
        let status = unsafe {
            __ramen_echo_send(
                self.cap.raw(),
                0, // request_id
                payload.as_ptr(),
                payload.len() as u32,
                reply_buf.as_mut_ptr(),
                reply_buf.len() as u32,
            )
        };

        if status != 0 {
            return Err(Status::from_raw(status));
        }

        // Parse reply from buffer
        Ok(reply_buf)
    }
}
```

### Generated Host Functions (Runner Service)

```rust
// Generated by idl_codegen --target wasm-host
// File: services/native_runner/src/generated/echo_v1_host.rs

use wasmtime::*;

/// Register echo_v1 host functions with a WASM linker
pub fn register_echo_v1_host(
    linker: &mut Linker<InstanceContext>,
) -> Result<(), Error> {
    linker.func_wrap(
        "ramen::harness::echo_v1",
        "send",
        |mut caller: Caller<'_, InstanceContext>,
         cap: u64,
         request_id: u64,
         payload_ptr: u32,
         payload_len: u32,
         reply_ptr: u32,
         reply_cap: u32| -> i32 {
            let ctx = caller.data_mut();

            // 1. Validate capability
            let kernel_handle = match ctx.cap_table.lookup(cap) {
                Some(h) => h,
                None => return STATUS_INVALID_CAPABILITY,
            };

            // 2. Read payload from WASM linear memory
            let memory = ctx.memory;
            let payload = &memory.data(&caller)[payload_ptr as usize..][..payload_len as usize];

            // 3. Perform IPC operation via kernel
            let result = ipc_echo_send(kernel_handle, request_id, payload);

            // 4. Write reply to WASM linear memory
            match result {
                Ok(reply) => {
                    let reply_slice = &memory.data_mut(&mut caller)[reply_ptr as usize..];
                    let copy_len = reply.len().min(reply_cap as usize);
                    reply_slice[..copy_len].copy_from_slice(&reply[..copy_len]);
                    STATUS_OK
                }
                Err(e) => e.to_status(),
            }
        },
    )?;

    Ok(())
}
```

### Build Flow

```
+-----------------------------------------------------------------------------+
|                         WASM Native Build Flow                               |
+-----------------------------------------------------------------------------+
|                                                                              |
|  +-------------+     +--------------+     +-----------------------------+    |
|  |  IDL Toml   |---->|  idl_codegen |---->|  sdk/src/generated/*.rs     |    |
|  |  (harness)  |     |  --target    |     |  (WASM guest bindings)      |    |
|  +-------------+     |  wasm-imports|     +-----------------------------+    |
|                      +--------------+                 |                      |
|                                                       v                      |
|  +-------------+     +--------------+     +-----------------------------+    |
|  |  IDL Toml   |---->|  idl_codegen |---->|  native_runner/src/gen/*.rs |    |
|  |  (harness)  |     |  --target    |     |  (Host function wrappers)   |    |
|  +-------------+     |  wasm-host   |     +-----------------------------+    |
|                      +--------------+                                        |
|                                                                              |
|  +-------------+     +--------------+     +-----------------------------+    |
|  |  App Code   |---->| cargo build  |---->|  module.wasm                |    |
|  |  (Rust SDK) |     | --target     |     |  (Native artifact)          |    |
|  +-------------+     |  wasm32-unk  |     +-----------------------------+    |
|                      +--------------+                                        |
|                                                                              |
+------------------------------------------------------------------------------+
```

### SDK Structure

```
sdk/
+-- Cargo.toml
+-- src/
    +-- lib.rs              # SDK root
    +-- capability.rs       # CapabilityHandle type
    +-- status.rs           # Status codes
    +-- memory.rs           # Linear memory helpers
    +-- generated/          # IDL-generated bindings
        +-- harness_echo_v1.rs
        +-- harness_trace_v1.rs
        +-- portal_file_picker_v1.rs
        +-- ...
```

---

## 7. Migration Path

### POSIX to Native Migration Strategy

```
+------------------------------------------------------------------------------+
|                         Migration Trajectory                                  |
+------------------------------------------------------------------------------+
|                                                                               |
|   Phase A           Phase B           Phase C           Phase D              |
|   +-------+        +-------+        +-------+        +-------+              |
|   | POSIX |        | POSIX |        | POSIX |        |       |              |
|   |  Only |------->| +Obs  |------->| +Grad |------->|Native |              |
|   |       |        |       |        |       |        | Only  |              |
|   +-------+        +-------+        +-------+        +-------+              |
|                                                                               |
|   Current          Observe          Graduation       POSIX                   |
|   (sandboxed)      capabilities     candidate        deprecated             |
|                    profile          native build                            |
|                                                                               |
+------------------------------------------------------------------------------+
```

### Phase A: Current State (S7 Complete)

- POSIX runner v0 with sandbox (seccomp, namespaces, chroot)
- Runtime kill-switch (`RAMEN_POSIX_RUNNER_ACK_RISK=1`)
- Store-integrated artifact verification

### Phase B: Observation Phase (S10.1)

**Goal:** Accumulate capability profiles from POSIX execution to inform native builds.

```
+--------------------------------------------------------------------------+
|                    Observation Phase Flow                                 |
+--------------------------------------------------------------------------+
|                                                                           |
|  1. POSIX script executes                                                |
|           |                                                               |
|           v                                                               |
|  2. Sandbox intercepts syscalls (seccomp)                                |
|           |                                                               |
|           v                                                               |
|  3. Observer logs observed behavior:                                     |
|     - Files accessed (paths, read/write)                                 |
|     - Network endpoints contacted                                        |
|     - IPC operations performed                                           |
|     - Resource usage patterns                                            |
|           |                                                               |
|           v                                                               |
|  4. Generate observed_caps_v0 artifact                                   |
|           |                                                               |
|           v                                                               |
|  5. Store artifact linked to content_id                                  |
|                                                                           |
+--------------------------------------------------------------------------+
```

**Observed Capability Schema:**
```json
{
  "kind": "observed_caps_v0",
  "source_content_id": "sha256:abc123...",
  "observations": [
    {
      "interface": "portal::file_picker_v1",
      "operations": ["pick_ro"],
      "paths_accessed": ["/home/user/docs"],
      "frequency": 42
    },
    {
      "interface": "harness::trace_v1",
      "operations": ["write"],
      "domains": ["self"],
      "frequency": 1000
    }
  ],
  "recommendation": {
    "native_equivalent": "native_wasm_v0",
    "suggested_capabilities": [...]
  }
}
```

### Phase C: Graduation Phase (S10.2)

**Goal:** Build and test native WASM artifacts that match observed behavior.

```
+--------------------------------------------------------------------------+
|                    Graduation Flow                                        |
+--------------------------------------------------------------------------+
|                                                                           |
|  1. Developer requests graduation for content_id                          |
|           |                                                               |
|           v                                                               |
|  2. Store retrieves observed_caps_v0 for artifact                        |
|           |                                                               |
|           v                                                               |
|  3. Wizard generates:                                                     |
|     - Native WASM manifest with declared capabilities                     |
|     - SDK scaffolding with IDL-generated bindings                         |
|     - Test harness for behavior comparison                                |
|           |                                                               |
|           v                                                               |
|  4. Developer ports logic to native Rust/WASM                            |
|           |                                                               |
|           v                                                               |
|  5. Foundry gate compares:                                                |
|     - POSIX execution trace vs Native execution trace                     |
|     - Functional equivalence tests                                        |
|     - Capability usage matches declaration                                |
|           |                                                               |
|           v                                                               |
|  6. Graduate to channel (experimental -> candidate -> stable)            |
|                                                                           |
+--------------------------------------------------------------------------+
```

### Phase D: Deprecation (S10.3)

Once native artifacts exist for common workloads:

1. **Default to Native:** Store prefers native artifacts over POSIX scripts
2. **POSIX Warning:** Deprecated warning on POSIX execution (not blocked)
3. **Remove Kill-Switch:** Eventually `RAMEN_POSIX_RUNNER_ACK_RISK` becomes hard block
4. **Archive POSIX Runner:** Move to legacy compatibility layer

### Backward Compatibility

| Phase | POSIX Scripts | Native WASM | Notes |
|-------|---------------|-------------|-------|
| A (now) | Full support | Not yet | Kill-switch required |
| B | Full support | Experimental | Observation active |
| C | Deprecated warning | Preferred | Graduation tooling |
| D | Hard block | Only path | POSIX archived |

### Migration Tooling

```bash
# Observe POSIX execution
store_cli observe-posix --content-id sha256:abc123... --output observed_caps.json

# Generate native scaffolding
store_cli scaffold-native --from-observed observed_caps.json --output ./native-port/

# Compare execution traces
foundry-compare-traces posix_trace.json native_trace.json

# Graduate artifact
store_cli graduate --content-id sha256:native456... --from sha256:abc123...
```

---

## 8. Implementation Phases

### Overview

Implementation is sequenced into 4 phases, each delivering vertical-slice value with Foundry gates.

```
+------------------------------------------------------------------------------+
|                      Implementation Phases                                   |
+------------------------------------------------------------------------------+
|                                                                               |
|  Phase 1          Phase 2          Phase 3          Phase 4                 |
|  +------------+   +------------+   +------------+   +------------+          |
|  |   IDL +    |   |   Runner   |   |   Data     |   |  Migration |          |
|  |   SDK      |-->|   Core     |-->|   Plane    |-->|  Tooling   |          |
|  |   Scaffold |   |            |   |   Integ    |   |            |          |
|  +------------+   +------------+   +------------+   +------------+          |
|                                                                               |
+------------------------------------------------------------------------------+
```

### Phase 1: IDL + SDK Scaffold (Foundation)

**Deliverables:**
| Item | Description |
|------|-------------|
| IDL extension | Add `wasm-imports` and `wasm-host` targets to `idl_codegen` |
| SDK crate | `sdk/` with capability types, status codes, memory helpers |
| Generated bindings | First harness (e.g., `echo_v1`) as WASM import + host |
| "Hello World" | Minimal WASM module that calls echo harness |

**Foundry Gate:**
```bash
foundry_native_runner_phase1.sh
+-- Test: idl_codegen --target wasm-imports produces valid Rust
+-- Test: idl_codegen --target wasm-host produces valid host functions
+-- Test: SDK compiles for wasm32-unknown-unknown
+-- Test: "Hello World" WASM module compiles and links
```

### Phase 2: Runner Core (Execution)

**Deliverables:**
| Item | Description |
|------|-------------|
| Native Runner service | `services/native_runner/` with wasmtime integration |
| Capability bridge | Maps WASM imports to kernel capability validation |
| Instance management | Load, grant caps, execute, teardown |
| Manifest validation | Parse `native_wasm_v0` manifests, validate signatures |

**Foundry Gate:**
```bash
foundry_native_runner_phase2.sh
+-- Test: Runner loads valid WASM artifact
+-- Test: Runner rejects unsigned/invalid artifacts
+-- Test: Capability bridge validates handles
+-- Test: "Hello World" executes and returns result
+-- Test: Invalid capability calls return STATUS_INVALID_CAPABILITY
```

### Phase 3: Data Plane Integration

**Deliverables:**
| Item | Description |
|------|-------------|
| Shmem mapping | `shmem_control_v1.map_to_linear` host function |
| Ring buffer support | S8 Phase 5 ring buffers accessible from WASM |
| Zero-copy transfer | Large buffer transfer without copying |
| Streaming API | Ring buffer consumer/producer for WASM |

**Foundry Gate:**
```bash
foundry_native_runner_phase3.sh
+-- Test: WASM can map shared memory region
+-- Test: WASM can write to mapped region
+-- Test: Native service can read WASM's written data
+-- Test: Ring buffer read/write from WASM
+-- Test: Zero-copy verification (no memcpy in trace)
```

### Phase 4: Migration Tooling

**Deliverables:**
| Item | Description |
|------|-------------|
| POSIX Observer | Syscall interception with capability inference |
| observed_caps_v0 | Schema for observed capability profiles |
| Graduation wizard | Scaffold native WASM from observed profile |
| Trace comparison | Compare POSIX vs native execution traces |

**Foundry Gate:**
```bash
foundry_native_runner_phase4.sh
+-- Test: Observer generates valid observed_caps_v0
+-- Test: Wizard scaffolds compilable native project
+-- Test: Trace comparison detects functional equivalence
+-- Test: End-to-end graduation flow (POSIX -> Native)
+-- Test: Native artifact passes same Foundry gates as POSIX
```

### Dependencies

| Phase | Dependencies | Blockers |
|-------|--------------|----------|
| 1 | None | None |
| 2 | Phase 1 | None |
| 3 | Phase 2, S8 Phase 5 | S8 Phase 5 Complete |
| 4 | Phase 3, V-012 | None |

### Success Metrics

| Metric | Target |
|--------|--------|
| WASM startup latency | <10ms (cold), <1ms (warm) |
| Capability check overhead | <100ns (kernel fast-path) |
| Zero-copy throughput | >1GB/s for large buffers |
| Migration tool coverage | >80% of POSIX syscalls mapped |

---

## 9. Related Documents

- [`CONSTITUTION.md`](../../../CONSTITUTION.md) - System invariants
- [`PLATFORM_OVERVIEW.md`](../../../PLATFORM_OVERVIEW.md) - Architecture overview
- [`docs/plans/security_remediation_v006_v007_v012.md`](../../plans/security_remediation_v006_v007_v012.md) - V-006 context
- [`docs/RING_BUFFER_V0.md`](../../RING_BUFFER_V0.md) - S8 Phase 5 ring buffer spec
- [`docs/MULTI_DOMAIN.md`](../../MULTI_DOMAIN.md) - Multi-domain architecture

---

**Document Version:** 1.0
**Last Updated:** 2026-02-18
**Status:** Approved
