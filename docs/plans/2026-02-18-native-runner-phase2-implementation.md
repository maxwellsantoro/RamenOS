# S10.0: Native Runner Phase 2 Implementation

**Last Updated:** 2026-02-18
**Status:** Approved
**Related:** V-006 Security Remediation, Native Runner Design (2026-02-18-native-runner-design.md)

---

## Executive Summary

This document specifies the implementation of the native runner executor — the service that loads, injects capabilities into, and executes WebAssembly modules with RamenOS-native semantics.

**Key Design Decisions:**
- Runner is **executor-only**: no policy, no grant decisions
- Capability injection via **exported globals** (`RAMEN_CAP_*`)
- Host functions are **plumbing**: decode → single kernel IPC → encode
- Broker supplies pre-granted handles; runner never derives names

---

## 1. Architecture

### Component Boundaries

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            COMPONENT BOUNDARIES                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   ┌───────────────┐         ┌───────────────┐         ┌───────────────┐    │
│   │    Broker     │         │    Runner     │         │    Kernel     │    │
│   │ (Domain Mgr)  │         │  (Executor)   │         │ (Enforcement) │    │
│   ├───────────────┤         ├───────────────┤         ├───────────────┤    │
│   │ • Policy      │ ──────▶ │ • Load WASM   │ ──────▶ │ • Validate    │    │
│   │ • Grant       │  handles│ • Inject caps │  ops    │ • Execute     │    │
│   │ • Revoke      │         │ • Run _start  │         │ • Enforce     │    │
│   └───────────────┘         └───────────────┘         └───────────────┘    │
│                                                                              │
│   Runner NEVER decides policy — it only executes with granted handles       │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Directory Structure

```
services/native_runner/
├── Cargo.toml
├── src/
│   ├── lib.rs                 ← Public API, error types
│   ├── runner.rs              ← Wasmtime integration, instance lifecycle
│   ├── capability.rs          ← Thin IPC bridge to kernel
│   ├── harness/
│   │   ├── mod.rs             ← Harness registration, dispatch
│   │   ├── echo.rs            ← echo_harness_v0 host functions
│   │   └── trace.rs           ← trace_service_v1 host functions
│   ├── kernel_bridge.rs       ← IPC client for kernel operations
│   └── bin/
│       └── native_runner.rs   ← CLI entry point
```

### Dependency Graph

```
lib.rs
  └── runner.rs ──────────────┬── capability.rs
       │                      │
       └── harness/mod.rs ────┤
            │                 │
            ├── echo.rs ──────┴── kernel_bridge.rs
            │
            └── trace.rs ────── kernel_bridge.rs
```

**Dependency rule:** All harness code goes through `kernel_bridge.rs` for operations. No direct kernel access.

---

## 2. Data Flow

### Complete Execution Flow

```
1. BROKER GRANTS (happens before runner sees it)
   ┌─────────────┐     ┌─────────────┐     ┌─────────────────┐
   │  Manifest   │────▶│   Broker    │────▶│  Kernel grants  │
   │  caps list  │     │  (policy)   │     │  each cap       │
   └─────────────┘     └─────────────┘     └─────────────────┘
                              │                     │
                              │     ┌───────────────┘
                              ▼     ▼
   ┌─────────────────────────────────────────────────────────┐
   │  Broker returns: granted_handles = {                    │
   │    "RAMEN_CAP_ECHO_REQUEST": 0x0001_0000_0000_1234,    │
   │    "RAMEN_CAP_ECHO_REPLY": 0x0001_0000_0000_1235,      │
   │    "RAMEN_CAP_TRACE": 0x0001_0000_0000_5678,           │
   │  }                                                       │
   └─────────────────────────────────────────────────────────┘

2. RUNNER LOADS
   ┌─────────────┐     ┌─────────────┐     ┌─────────────────┐
   │  WASM bytes │────▶│  Wasmtime   │────▶│  Instance       │
   │             │     │  Engine     │     │  ready          │
   └─────────────┘     └─────────────┘     └─────────────────┘

3. RUNNER INJECTS (via exported globals)
   ┌─────────────────────────────────────────────────────────┐
   │  For each export starting with RAMEN_CAP_:              │
   │    - Look up handle in granted_handles                  │
   │    - If missing → error (fail closed)                   │
   │    - Set global to handle value                         │
   └─────────────────────────────────────────────────────────┘

4. RUNNER EXECUTES
   ┌─────────────┐     ┌─────────────┐
   │  _start()   │────▶│  WASM runs  │
   └─────────────┘     └─────────────┘

5. HARNESS CALL (single kernel crossing)
   ┌─────────────┐     ┌─────────────┐     ┌─────────────────┐
   │  WASM call  │────▶│  Host func  │────▶│  Kernel IPC     │
   │  echo_send  │     │  (plumbing) │     │  (validates+does)│
   └─────────────┘     └─────────────┘     └─────────────────┘
```

### Host Function Pattern

**Key principle:** Host functions are pure plumbing. They do NOT validate capabilities — the kernel does that as part of the operation.

```
Host Function:
  1. Decode args from WASM linear memory (bounds-checked)
  2. Make ONE kernel IPC call (validation is inherent)
  3. Encode reply into WASM linear memory
  4. Return status code
```

---

## 3. Capability Injection

### Contract

1. WASM module exports globals named `RAMEN_CAP_*`
2. `granted_handles` is keyed by export global name
3. Runner enumerates exports, sets each global
4. Fail closed: if module expects a cap we don't have, error

### Implementation

```rust
// runner.rs

/// Inject capabilities by setting exported globals.
fn inject_capabilities(
    store: &mut Store<InstanceContext>,
    instance: &Instance,
    granted_handles: &HashMap<String, u64>,
) -> Result<(), RunnerError> {
    // Enumerate all exports starting with RAMEN_CAP_
    for export in instance.exports(store) {
        let name = export.name();
        if !name.starts_with("RAMEN_CAP_") {
            continue;
        }

        let global = match export.into_global() {
            Some(g) => g,
            None => continue,
        };

        // Look up handle for this export
        let handle = granted_handles.get(name)
            .ok_or_else(|| RunnerError::MissingCapability(name.to_string()))?;

        // Set the global
        global.set(store, Val::I64(*handle as i64))
            .map_err(|e| RunnerError::GlobalSet(format!("{}: {}", name, e)))?;
    }

    Ok(())
}
```

---

## 4. Host Function Signatures

### echo_harness_v0

**Module:** `ramen::harness.echo`

**Functions:** Match generated SDK in `sdk/src/generated/harness_echo_v0.rs`

```rust
// harness/echo.rs

/// Host function for echo_request::call
pub fn host_echo_request_call(
    mut caller: Caller<'_, InstanceContext>,
    cap_handle: u64,
    request_id: u64,
    payload_ptr: u32,
    payload_len: u32,
    out_ptr: u32,
    out_len_ptr: u32,
) -> i32 {
    // 1. Read payload (bounds-checked)
    let memory = caller.data().memory;
    let data = memory.data(&caller);

    let payload = match data.get(
        payload_ptr as usize..payload_ptr as usize + payload_len as usize
    ) {
        Some(slice) => slice,
        None => return STATUS_INVALID_ARGUMENT,
    };

    // 2. Single kernel-mediated operation
    let result = caller.data().kernel_bridge.echo_request(
        cap_handle,
        request_id,
        payload,
    );

    // 3. Write reply
    match result {
        Ok(reply) => {
            let data = memory.data_mut(&mut caller);

            // Write reply bytes
            let copy_len = reply.len().min(data.len() - out_ptr as usize);
            data[out_ptr as usize..out_ptr as usize + copy_len]
                .copy_from_slice(&reply[..copy_len]);

            // Write actual length
            if out_len_ptr as usize + 4 <= data.len() {
                let len_bytes = (copy_len as u32).to_le_bytes();
                data[out_len_ptr as usize..out_len_ptr as usize + 4]
                    .copy_from_slice(&len_bytes);
            }

            STATUS_OK
        }
        Err(status) => status as i32,
    }
}

/// Host function for echo_reply::call
pub fn host_echo_reply_call(
    mut caller: Caller<'_, InstanceContext>,
    cap_handle: u64,
    request_id: u64,
    status: u32,
    payload_ptr: u32,
    payload_len: u32,
) -> i32 {
    // Similar pattern to echo_request_call
    // ...
}
```

### trace_service_v1

**Module:** `ramen::harness.trace`

**Functions:** Match generated SDK

```rust
// harness/trace.rs

/// Host function for trace_read::call
pub fn host_trace_read_call(
    mut caller: Caller<'_, InstanceContext>,
    cap_handle: u64,
    offset: u64,
    out_ptr: u32,
    out_cap: u32,
    out_len_ptr: u32,
) -> i32 {
    let memory = caller.data().memory;

    let result = caller.data().kernel_bridge.trace_read(
        cap_handle,
        offset,
        out_cap as usize,
    );

    match result {
        Ok(data) => {
            let mem_data = memory.data_mut(&mut caller);
            let copy_len = data.len().min(out_cap as usize);

            mem_data[out_ptr as usize..out_ptr as usize + copy_len]
                .copy_from_slice(&data[..copy_len]);

            if out_len_ptr as usize + 4 <= mem_data.len() {
                let len_bytes = (copy_len as u32).to_le_bytes();
                mem_data[out_len_ptr as usize..out_len_ptr as usize + 4]
                    .copy_from_slice(&len_bytes);
            }

            STATUS_OK
        }
        Err(status) => status as i32,
    }
}

/// Host function for trace_write::call
pub fn host_trace_write_call(
    mut caller: Caller<'_, InstanceContext>,
    cap_handle: u64,
    data_ptr: u32,
    data_len: u32,
) -> i32 {
    // ...
}
```

---

## 5. Public API

### Library API

```rust
// src/lib.rs

/// Native runner executor.
pub struct NativeRunner {
    config: RunnerConfig,
}

/// Runner configuration.
pub struct RunnerConfig {
    /// Path to kernel IPC socket.
    pub kernel_ipc: PathBuf,

    /// Optional trace output path.
    pub trace_output: Option<PathBuf>,
}

/// Run configuration (per-execution).
pub struct RunConfig {
    /// Pre-granted capability handles, keyed by export global name.
    ///
    /// Example:
    /// {
    ///   "RAMEN_CAP_ECHO_REQUEST": 0x0001_0000_0000_1234,
    ///   "RAMEN_CAP_TRACE": 0x0001_0000_0000_5678,
    /// }
    pub granted_handles: HashMap<String, u64>,
}

/// Result of a WASM execution.
pub struct RunResult {
    /// Exit status from WASM.
    pub exit_code: i32,
    /// Stdout captured from execution.
    pub stdout: Vec<u8>,
    /// Trace artifacts (if trace capability was granted).
    pub trace: Option<Vec<u8>>,
}

impl NativeRunner {
    /// Create a new runner with the given configuration.
    pub fn new(config: RunnerConfig) -> Self;

    /// Load a WASM module.
    pub fn load(&mut self, wasm_bytes: &[u8]) -> Result<LoadedModule, RunnerError>;

    /// Run a loaded module with granted capabilities.
    pub fn run(
        &mut self,
        module: LoadedModule,
        config: RunConfig,
    ) -> Result<RunResult, RunnerError>;

    /// Load and run in one step.
    pub fn load_and_run(
        &mut self,
        wasm_bytes: &[u8],
        config: RunConfig,
    ) -> Result<RunResult, RunnerError>;
}
```

### Error Types

```rust
/// Top-level runner errors.
#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    #[error("WASM compilation failed: {0}")]
    WasmCompile(String),

    #[error("WASM instantiation failed: {0}")]
    WasmInstantiate(String),

    #[error("Missing required capability: {0}")]
    MissingCapability(String),

    #[error("Failed to set global: {0}")]
    GlobalSet(String),

    #[error("Harness call failed: {0}")]
    HarnessCall(String),

    #[error("Kernel IPC error: {0}")]
    KernelIpc(String),
}
```

### CLI

```bash
# Development mode (direct handles)
native_runner \
  --wasm ./hello_wasm.wasm \
  --kernel-ipc /run/ramen/kernel.sock \
  --cap RAMEN_CAP_ECHO_REQUEST=0x1000000001234 \
  --cap RAMEN_CAP_TRACE=0x1000000005678

# Options:
#   --wasm PATH           Path to WASM module (required)
#   --kernel-ipc PATH     Unix socket for kernel operations (required)
#   --cap NAME=HANDLE     Grant capability (repeatable)
#   --trace-output PATH   Write execution trace to file
#   -v, --verbose         Enable debug logging
```

---

## 6. Kernel Bridge

### Interface

```rust
// kernel_bridge.rs

/// IPC client for kernel operations.
///
/// This is the ONLY way host functions interact with the kernel.
/// Each method makes a single IPC call that includes capability validation.
pub struct KernelBridge {
    socket_path: PathBuf,
}

impl KernelBridge {
    pub fn new(socket_path: PathBuf) -> Self;

    /// Echo harness: send request.
    /// Kernel validates cap_handle as part of this operation.
    pub fn echo_request(
        &self,
        cap_handle: u64,
        request_id: u64,
        payload: &[u8],
    ) -> Result<Vec<u8>, KernelStatus>;

    /// Echo harness: send reply.
    pub fn echo_reply(
        &self,
        cap_handle: u64,
        request_id: u64,
        status: u32,
        payload: &[u8],
    ) -> Result<(), KernelStatus>;

    /// Trace service: read trace data.
    pub fn trace_read(
        &self,
        cap_handle: u64,
        offset: u64,
        max_len: usize,
    ) -> Result<Vec<u8>, KernelStatus>;

    /// Trace service: write trace data.
    pub fn trace_write(
        &self,
        cap_handle: u64,
        data: &[u8],
    ) -> Result<(), KernelStatus>;
}
```

### Mock for Testing

```rust
/// Mock kernel bridge for testing.
/// Returns canned responses, records calls for assertion.
pub struct MockKernelBridge {
    responses: HashMap<String, Vec<u8>>,
    calls: Vec<KernelCall>,
}

impl MockKernelBridge {
    pub fn new() -> Self;

    /// Set a canned response for an operation.
    pub fn set_response(&mut self, op: &str, response: Vec<u8>);

    /// Get recorded calls for assertion.
    pub fn get_calls(&self) -> &[KernelCall];
}

#[derive(Debug, Clone)]
pub struct KernelCall {
    pub operation: String,
    pub cap_handle: u64,
    pub args: Vec<u8>,
}
```

---

## 7. Testing Strategy

### Test Layers

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Layer 3: Foundry Gate (End-to-End)                                         │
│  foundry_native_runner_s10_0.sh                                             │
│  - Builds hello_wasm.wasm                                                   │
│  - Runs native_runner with mock kernel                                      │
│  - Asserts output + exit code                                               │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Layer 2: Integration Tests (in crate)                                      │
│  tests/integration_*.rs                                                     │
│  - Runner loads real WASM with mock bridge                                  │
│  - Host functions execute against mock kernel                               │
│  - Verifies capability injection + call flow                                │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Layer 1: Unit Tests (#[cfg(test)] in modules)                              │
│  - kernel_bridge.rs: IPC serialization, status mapping                      │
│  - harness/echo.rs: host function logic (with mock context)                 │
│  - runner.rs: capability injection, error handling                          │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Foundry Gate

```bash
#!/usr/bin/env bash
# tools/ci/foundry_native_runner_s10_0.sh

# Assertions:
# 1. native_runner binary builds
# 2. hello_wasm.wasm builds (dependency)
# 3. native_runner can load hello_wasm.wasm
# 4. Capability injection succeeds (RAMEN_CAP_* globals set)
# 5. Host functions execute (mock kernel bridge receives calls)
# 6. Exit code is 0
# 7. Stdout contains expected output

echo "FOUNDRY_NATIVE_RUNNER_S10_0: METRIC ..."
```

---

## 8. Implementation Checklist

### Phase S10.0 (This Document)

- [ ] Create `services/native_runner/` crate structure
- [ ] Implement `NativeRunner` public API
- [ ] Implement wasmtime integration (`runner.rs`)
- [ ] Implement export-global injection
- [ ] Implement `kernel_bridge.rs` with mock
- [ ] Implement `harness/echo.rs` host functions
- [ ] Implement `harness/trace.rs` host functions
- [ ] Implement CLI (`bin/native_runner.rs`)
- [ ] Add unit tests
- [ ] Add integration tests
- [ ] Add Foundry gate `foundry_native_runner_s10_0.sh`
- [ ] Wire into justfile
- [ ] Update CURRENT_STATUS.md

### Phase S10.1 (Future)

- [ ] Manifest parsing (`native_wasm_v0`)
- [ ] Broker integration (Domain Manager)
- [ ] Runtime supervisor integration
- [ ] Revocation path
- [ ] Foundry gate `foundry_native_runner_s10_1.sh`

---

## 9. Dependencies

| Crate | Purpose | Version |
|-------|---------|---------|
| `wasmtime` | WASM execution | Latest stable |
| `wasmtime-wasi` | WASI support (minimal) | Latest stable |
| `thiserror` | Error types | Workspace |
| `serde` | Serialization | Workspace |
| `serde_json` | JSON parsing | Workspace |
| `kernel_api` | Status codes, types | Workspace |

---

**Document Version:** 1.0
**Last Updated:** 2026-02-18
**Status:** Approved
