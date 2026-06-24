# Native Runner S10.0 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the native runner executor that loads WASM modules, injects capabilities via exported globals, and executes with host functions that perform single kernel IPC crossings.

**Architecture:** Library crate (`native_runner`) with thin CLI binary. Runner is executor-only — no policy decisions. Host functions decode args, make one kernel IPC call, encode reply. Capability injection via `RAMEN_CAP_*` exported globals.

**Tech Stack:** Rust, wasmtime, thiserror, kernel_api types

---

## Task 1: Create Crate Structure

**Files:**
- Create: `services/native_runner/Cargo.toml`
- Create: `services/native_runner/src/lib.rs`
- Modify: `Cargo.toml` (workspace members)

**Step 1: Create Cargo.toml**

```toml
# services/native_runner/Cargo.toml
[package]
name = "native_runner"
version = "0.0.0"
edition = "2021"

[lib]
name = "native_runner"
path = "src/lib.rs"

[[bin]]
name = "native_runner"
path = "src/bin/native_runner.rs"

[dependencies]
wasmtime = "27"
thiserror = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
kernel_api = { path = "../../kernel_api" }
anyhow = "1"

[dev-dependencies]
tempfile = "3"
```

**Step 2: Create lib.rs stub**

```rust
// services/native_runner/src/lib.rs
//! Native Runner - WASM executor for RamenOS native workloads.
//!
//! This crate provides the execution layer for WebAssembly modules
//! with RamenOS-native semantics. It is executor-only: no policy
//! decisions, no capability grants. Those are handled by the broker.

pub mod error;
pub mod runner;
pub mod kernel_bridge;
pub mod harness;

pub use error::RunnerError;
pub use runner::{NativeRunner, RunConfig, RunResult, RunnerConfig};
pub use kernel_bridge::KernelBridge;
```

**Step 3: Create error.rs**

```rust
// services/native_runner/src/error.rs
//! Error types for the native runner.

use thiserror::Error;

/// Top-level runner errors.
#[derive(Debug, Error)]
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

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

/// Status codes returned to WASM modules.
/// Must match kernel_api status codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Status {
    Ok = 0,
    InvalidCapability = 1,
    PermissionDenied = 2,
    InvalidArgument = 3,
    WouldBlock = 4,
    IoError = 5,
}

impl From<Status> for i32 {
    fn from(status: Status) -> i32 {
        status as i32
    }
}
```

**Step 4: Add to workspace**

Modify `Cargo.toml`:
```toml
# In the workspace.members array, add:
members = [
    # ... existing members ...
    "services/native_runner",
]
```

**Step 5: Verify build**

Run: `cargo check -p native_runner`
Expected: Compiles without errors

**Step 6: Commit**

```bash
git add services/native_runner/Cargo.toml
git add services/native_runner/src/lib.rs
git add services/native_runner/src/error.rs
git add Cargo.toml
git commit -m "$(cat <<'EOF'
feat(native-runner): create crate structure

Add native_runner crate for WASM execution:
- Library + binary structure
- Error types with thiserror
- Status codes matching kernel_api

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Implement Kernel Bridge (Mock First)

**Files:**
- Create: `services/native_runner/src/kernel_bridge.rs`

**Step 1: Write the failing test**

```rust
// services/native_runner/src/kernel_bridge.rs
//! Kernel IPC bridge for WASM host functions.
//!
//! This module provides the interface for host functions to communicate
//! with the kernel. Each method makes a single IPC call that includes
//! capability validation.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_kernel_bridge_records_calls() {
        let mut bridge = MockKernelBridge::new();

        bridge.set_echo_response(vec![1, 2, 3, 4]);

        let result = bridge.echo_request(0x1234, 42, &[5, 6, 7]).unwrap();
        assert_eq!(result, vec![1, 2, 3, 4]);

        let calls = bridge.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].operation, "echo_request");
        assert_eq!(calls[0].cap_handle, 0x1234);
    }

    #[test]
    fn mock_kernel_bridge_returns_error_for_missing_response() {
        let bridge = MockKernelBridge::new();

        let result = bridge.echo_request(0x1234, 42, &[5, 6, 7]);
        assert!(result.is_err());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p native_runner --lib kernel_bridge`
Expected: FAIL with "use of undeclared type `MockKernelBridge`"

**Step 3: Implement the trait and mock**

```rust
// services/native_runner/src/kernel_bridge.rs (add after tests)

use std::collections::HashMap;

/// Trait for kernel communication.
/// Implementations can be real IPC or mock for testing.
pub trait KernelBridge {
    /// Echo harness: send request.
    /// Kernel validates cap_handle as part of this operation.
    fn echo_request(
        &mut self,
        cap_handle: u64,
        request_id: u64,
        payload: &[u8],
    ) -> Result<Vec<u8>, crate::error::Status>;

    /// Echo harness: send reply.
    fn echo_reply(
        &mut self,
        cap_handle: u64,
        request_id: u64,
        status: u32,
        payload: &[u8],
    ) -> Result<(), crate::error::Status>;

    /// Trace service: read trace data.
    fn trace_read(
        &mut self,
        cap_handle: u64,
        offset: u64,
        max_len: usize,
    ) -> Result<Vec<u8>, crate::error::Status>;

    /// Trace service: write trace data.
    fn trace_write(
        &mut self,
        cap_handle: u64,
        data: &[u8],
    ) -> Result<(), crate::error::Status>;
}

/// Recorded kernel call for testing assertions.
#[derive(Debug, Clone)]
pub struct KernelCall {
    pub operation: String,
    pub cap_handle: u64,
    pub args: Vec<u8>,
}

/// Mock kernel bridge for testing.
/// Returns canned responses, records calls for assertion.
pub struct MockKernelBridge {
    echo_response: Option<Vec<u8>>,
    calls: Vec<KernelCall>,
}

impl MockKernelBridge {
    pub fn new() -> Self {
        Self {
            echo_response: None,
            calls: Vec::new(),
        }
    }

    /// Set the canned response for echo_request.
    pub fn set_echo_response(&mut self, response: Vec<u8>) {
        self.echo_response = Some(response);
    }

    /// Get recorded calls for assertion.
    pub fn get_calls(&self) -> &[KernelCall] {
        &self.calls
    }
}

impl KernelBridge for MockKernelBridge {
    fn echo_request(
        &mut self,
        cap_handle: u64,
        request_id: u64,
        payload: &[u8],
    ) -> Result<Vec<u8>, crate::error::Status> {
        // Record the call
        let mut args = vec![];
        args.extend_from_slice(&request_id.to_le_bytes());
        args.extend_from_slice(payload);

        self.calls.push(KernelCall {
            operation: "echo_request".to_string(),
            cap_handle,
            args,
        });

        // Return canned response or error
        self.echo_response.clone().ok_or(crate::error::Status::IoError)
    }

    fn echo_reply(
        &mut self,
        cap_handle: u64,
        request_id: u64,
        status: u32,
        payload: &[u8],
    ) -> Result<(), crate::error::Status> {
        let mut args = vec![];
        args.extend_from_slice(&request_id.to_le_bytes());
        args.extend_from_slice(&status.to_le_bytes());
        args.extend_from_slice(payload);

        self.calls.push(KernelCall {
            operation: "echo_reply".to_string(),
            cap_handle,
            args,
        });

        Ok(())
    }

    fn trace_read(
        &mut self,
        cap_handle: u64,
        offset: u64,
        max_len: usize,
    ) -> Result<Vec<u8>, crate::error::Status> {
        let mut args = vec![];
        args.extend_from_slice(&offset.to_le_bytes());
        args.extend_from_slice(&(max_len as u64).to_le_bytes());

        self.calls.push(KernelCall {
            operation: "trace_read".to_string(),
            cap_handle,
            args,
        });

        // Return empty data for now
        Ok(vec![0; max_len.min(64)])
    }

    fn trace_write(
        &mut self,
        cap_handle: u64,
        data: &[u8],
    ) -> Result<(), crate::error::Status> {
        self.calls.push(KernelCall {
            operation: "trace_write".to_string(),
            cap_handle,
            args: data.to_vec(),
        });

        Ok(())
    }
}

impl Default for MockKernelBridge {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p native_runner --lib kernel_bridge`
Expected: 2 tests pass

**Step 5: Commit**

```bash
git add services/native_runner/src/kernel_bridge.rs
git commit -m "$(cat <<'EOF'
feat(native-runner): add kernel bridge trait and mock

- KernelBridge trait defines IPC interface
- MockKernelBridge records calls for testing
- Canned responses for echo_request

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Implement Instance Context

**Files:**
- Create: `services/native_runner/src/context.rs`

**Step 1: Write the failing test**

```rust
// services/native_runner/src/context.rs
//! Instance context for WASM execution.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel_bridge::MockKernelBridge;

    #[test]
    fn instance_context_holds_memory_and_bridge() {
        let bridge = MockKernelBridge::new();

        // Context can be created with bridge
        let _ctx = InstanceContext {
            kernel_bridge: Box::new(bridge),
            stdout: Vec::new(),
        };
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p native_runner --lib context`
Expected: FAIL with "use of undeclared type `InstanceContext`"

**Step 3: Implement InstanceContext**

```rust
// services/native_runner/src/context.rs (add after tests)

use crate::kernel_bridge::KernelBridge;

/// Context passed to WASM host functions.
///
/// This is the `Caller<T>` data for all host function invocations.
/// It provides access to the kernel bridge and captures stdout.
pub struct InstanceContext {
    /// Kernel bridge for IPC operations.
    pub kernel_bridge: Box<dyn KernelBridge>,

    /// Captured stdout from WASM execution.
    pub stdout: Vec<u8>,
}

impl InstanceContext {
    /// Create a new context with the given kernel bridge.
    pub fn new(kernel_bridge: Box<dyn KernelBridge>) -> Self {
        Self {
            kernel_bridge,
            stdout: Vec::new(),
        }
    }

    /// Create a context with mock kernel bridge for testing.
    pub fn with_mock() -> Self {
        Self::new(Box::new(crate::kernel_bridge::MockKernelBridge::new()))
    }
}
```

**Step 4: Update lib.rs to export context**

```rust
// services/native_runner/src/lib.rs (update)

pub mod error;
pub mod runner;
pub mod kernel_bridge;
pub mod harness;
pub mod context;

pub use error::RunnerError;
pub use runner::{NativeRunner, RunConfig, RunResult, RunnerConfig};
pub use kernel_bridge::{KernelBridge, MockKernelBridge};
pub use context::InstanceContext;
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p native_runner --lib context`
Expected: 1 test passes

**Step 6: Commit**

```bash
git add services/native_runner/src/context.rs
git add services/native_runner/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(native-runner): add instance context

InstanceContext holds kernel bridge and stdout capture
for WASM host function invocations.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Implement Echo Harness Host Functions

**Files:**
- Create: `services/native_runner/src/harness/mod.rs`
- Create: `services/native_runner/src/harness/echo.rs`

**Step 1: Write the failing test**

```rust
// services/native_runner/src/harness/echo.rs
//! Host functions for echo_harness_v0.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::InstanceContext;
    use crate::kernel_bridge::MockKernelBridge;
    use wasmtime::*;

    #[test]
    fn echo_request_host_function_returns_ok() {
        let engine = Engine::default();
        let mut store = Store::new(
            &engine,
            InstanceContext::with_mock(),
        );

        // Create a mock memory
        let memory_type = MemoryType::new(1, None);
        let memory = Memory::new(&mut store, memory_type).unwrap();

        // Write test payload to memory
        let payload = b"hello";
        let data = memory.data_mut(&mut store);
        data[0..payload.len()].copy_from_slice(payload);

        // Create host function
        let func = create_echo_request_host(&mut store, memory);

        // Call the function
        let result: i32 = func
            .call(&mut store, (0x1234, 42u64, 0u32, 5u32, 100u32, 104u32))
            .unwrap();

        // Should return OK (0) because mock returns canned response
        // But our mock doesn't have a response set, so it returns error
        assert_ne!(result, 0);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p native_runner --lib harness::echo`
Expected: FAIL with "use of undeclared type `Memory`" or similar

**Step 3: Implement echo host functions**

```rust
// services/native_runner/src/harness/echo.rs (add after tests)

use crate::context::InstanceContext;
use crate::error::Status;
use wasmtime::*;

/// Create the echo_request host function.
///
/// Signature matches generated SDK:
/// - cap_handle: u64 - capability handle
/// - request_id: u64 - request identifier
/// - payload_ptr: u32 - pointer to payload in linear memory
/// - payload_len: u32 - length of payload
/// - out_ptr: u32 - pointer to output buffer
/// - out_len_ptr: u32 - pointer to write actual output length
/// - returns: i32 - status code
pub fn create_echo_request_host(
    store: &mut Store<InstanceContext>,
    memory: Memory,
) -> Func {
    Func::wrap(
        store,
        move |
            mut caller: Caller<'_, InstanceContext>,
            cap_handle: u64,
            request_id: u64,
            payload_ptr: u32,
            payload_len: u32,
            out_ptr: u32,
            out_len_ptr: u32,
        | -> i32 {
            // 1. Read payload from linear memory (bounds-checked)
            let data = memory.data(&caller);

            let payload_end = match payload_ptr.checked_add(payload_len) {
                Some(end) => end as usize,
                None => return Status::InvalidArgument as i32,
            };

            if payload_end > data.len() {
                return Status::InvalidArgument as i32;
            }

            let payload = &data[payload_ptr as usize..payload_end];

            // 2. Make single kernel IPC call (validation inherent)
            let result = caller.data_mut().kernel_bridge.echo_request(
                cap_handle,
                request_id,
                payload,
            );

            // 3. Write reply to linear memory
            match result {
                Ok(reply) => {
                    let data = memory.data_mut(&mut caller);

                    // Write reply bytes
                    let copy_len = reply.len().min(data.len().saturating_sub(out_ptr as usize));
                    if out_ptr as usize + copy_len <= data.len() {
                        data[out_ptr as usize..out_ptr as usize + copy_len]
                            .copy_from_slice(&reply[..copy_len]);
                    }

                    // Write actual length
                    if out_len_ptr as usize + 4 <= data.len() {
                        let len_bytes = (copy_len as u32).to_le_bytes();
                        data[out_len_ptr as usize..out_len_ptr as usize + 4]
                            .copy_from_slice(&len_bytes);
                    }

                    Status::Ok as i32
                }
                Err(status) => status as i32,
            }
        },
    )
}

/// Create the echo_reply host function.
pub fn create_echo_reply_host(
    store: &mut Store<InstanceContext>,
    memory: Memory,
) -> Func {
    Func::wrap(
        store,
        move |
            mut caller: Caller<'_, InstanceContext>,
            cap_handle: u64,
            request_id: u64,
            payload_ptr: u32,
            payload_len: u32,
            status_code: u32,
            out_ptr: u32,
            out_len_ptr: u32,
        | -> i32 {
            let data = memory.data(&caller);

            let payload_end = match payload_ptr.checked_add(payload_len) {
                Some(end) => end as usize,
                None => return Status::InvalidArgument as i32,
            };

            if payload_end > data.len() {
                return Status::InvalidArgument as i32;
            }

            let payload = &data[payload_ptr as usize..payload_end];

            let result = caller.data_mut().kernel_bridge.echo_reply(
                cap_handle,
                request_id,
                status_code,
                payload,
            );

            match result {
                Ok(()) => Status::Ok as i32,
                Err(status) => status as i32,
            }
        },
    )
}
```

**Step 4: Create harness mod.rs**

```rust
// services/native_runner/src/harness/mod.rs
//! Harness host functions for WASM modules.

pub mod echo;

pub use echo::{create_echo_request_host, create_echo_reply_host};
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p native_runner --lib harness::echo`
Expected: 1 test passes

**Step 6: Commit**

```bash
git add services/native_runner/src/harness/mod.rs
git add services/native_runner/src/harness/echo.rs
git commit -m "$(cat <<'EOF'
feat(native-runner): add echo harness host functions

Host functions for echo_request and echo_reply:
- Bounds-checked memory access
- Single kernel IPC crossing
- Status code returns

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Implement Trace Harness Host Functions

**Files:**
- Create: `services/native_runner/src/harness/trace.rs`
- Modify: `services/native_runner/src/harness/mod.rs`

**Step 1: Write the failing test**

```rust
// services/native_runner/src/harness/trace.rs
//! Host functions for trace_service_v1.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::InstanceContext;
    use wasmtime::*;

    #[test]
    fn trace_read_host_function_returns_data() {
        let engine = Engine::default();
        let mut store = Store::new(
            &engine,
            InstanceContext::with_mock(),
        );

        let memory_type = MemoryType::new(1, None);
        let memory = Memory::new(&mut store, memory_type).unwrap();

        let func = create_trace_read_host(&mut store, memory);

        // Call with cap, offset, out_ptr, out_cap, out_len_ptr
        let result: i32 = func
            .call(&mut store, (0x1234, 0u64, 0u32, 64u32, 68u32))
            .unwrap();

        // Mock returns data, so should be OK
        assert_eq!(result, 0);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p native_runner --lib harness::trace`
Expected: FAIL with "use of undeclared type `Memory`"

**Step 3: Implement trace host functions**

```rust
// services/native_runner/src/harness/trace.rs (add after tests)

use crate::context::InstanceContext;
use crate::error::Status;
use wasmtime::*;

/// Create the trace_read host function.
pub fn create_trace_read_host(
    store: &mut Store<InstanceContext>,
    memory: Memory,
) -> Func {
    Func::wrap(
        store,
        move |
            mut caller: Caller<'_, InstanceContext>,
            cap_handle: u64,
            offset: u64,
            out_ptr: u32,
            out_cap: u32,
            out_len_ptr: u32,
        | -> i32 {
            let result = caller.data_mut().kernel_bridge.trace_read(
                cap_handle,
                offset,
                out_cap as usize,
            );

            match result {
                Ok(data) => {
                    let mem_data = memory.data_mut(&mut caller);

                    // Write data to output buffer
                    let copy_len = data.len().min(out_cap as usize);
                    if out_ptr as usize + copy_len <= mem_data.len() {
                        mem_data[out_ptr as usize..out_ptr as usize + copy_len]
                            .copy_from_slice(&data[..copy_len]);
                    }

                    // Write actual length
                    if out_len_ptr as usize + 4 <= mem_data.len() {
                        let len_bytes = (copy_len as u32).to_le_bytes();
                        mem_data[out_len_ptr as usize..out_len_ptr as usize + 4]
                            .copy_from_slice(&len_bytes);
                    }

                    Status::Ok as i32
                }
                Err(status) => status as i32,
            }
        },
    )
}

/// Create the trace_write host function.
pub fn create_trace_write_host(
    store: &mut Store<InstanceContext>,
    memory: Memory,
) -> Func {
    Func::wrap(
        store,
        move |
            mut caller: Caller<'_, InstanceContext>,
            cap_handle: u64,
            data_ptr: u32,
            data_len: u32,
        | -> i32 {
            let data = memory.data(&caller);

            let data_end = match data_ptr.checked_add(data_len) {
                Some(end) => end as usize,
                None => return Status::InvalidArgument as i32,
            };

            if data_end > data.len() {
                return Status::InvalidArgument as i32;
            }

            let trace_data = &data[data_ptr as usize..data_end];

            let result = caller.data_mut().kernel_bridge.trace_write(
                cap_handle,
                trace_data,
            );

            match result {
                Ok(()) => Status::Ok as i32,
                Err(status) => status as i32,
            }
        },
    )
}
```

**Step 4: Update harness mod.rs**

```rust
// services/native_runner/src/harness/mod.rs (update)

pub mod echo;
pub mod trace;

pub use echo::{create_echo_request_host, create_echo_reply_host};
pub use trace::{create_trace_read_host, create_trace_write_host};
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p native_runner --lib harness::trace`
Expected: 1 test passes

**Step 6: Commit**

```bash
git add services/native_runner/src/harness/trace.rs
git add services/native_runner/src/harness/mod.rs
git commit -m "$(cat <<'EOF'
feat(native-runner): add trace harness host functions

Host functions for trace_read and trace_write:
- Bounds-checked memory access
- Single kernel IPC crossing
- Status code returns

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Implement Runner Core (Capability Injection)

**Files:**
- Create: `services/native_runner/src/runner.rs`

**Step 1: Write the failing test**

```rust
// services/native_runner/src/runner.rs
//! Native runner core implementation.

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn run_config_accepts_granted_handles() {
        let config = RunConfig {
            granted_handles: HashMap::from([
                ("RAMEN_CAP_ECHO_REQUEST".to_string(), 0x1234),
            ]),
        };

        assert_eq!(
            config.granted_handles.get("RAMEN_CAP_ECHO_REQUEST"),
            Some(&0x1234)
        );
    }

    #[test]
    fn runner_config_has_kernel_ipc_path() {
        let config = RunnerConfig {
            kernel_ipc: "/run/ramen/kernel.sock".into(),
            trace_output: None,
        };

        assert_eq!(config.kernel_ipc.to_str(), Some("/run/ramen/kernel.sock"));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p native_runner --lib runner`
Expected: FAIL with "use of undeclared type `RunConfig`"

**Step 3: Implement types**

```rust
// services/native_runner/src/runner.rs (add after tests)

use std::collections::HashMap;
use std::path::PathBuf;

/// Native runner configuration (long-lived).
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// Path to kernel IPC socket.
    pub kernel_ipc: PathBuf,

    /// Optional trace output path.
    pub trace_output: Option<PathBuf>,
}

/// Run configuration (per-execution).
#[derive(Debug, Clone)]
pub struct RunConfig {
    /// Pre-granted capability handles, keyed by export global name.
    ///
    /// Example:
    /// ```ignore
    /// {
    ///   "RAMEN_CAP_ECHO_REQUEST": 0x0001_0000_0000_1234,
    ///   "RAMEN_CAP_TRACE": 0x0001_0000_0000_5678,
    /// }
    /// ```
    pub granted_handles: HashMap<String, u64>,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            granted_handles: HashMap::new(),
        }
    }
}

/// Result of a WASM execution.
#[derive(Debug, Clone)]
pub struct RunResult {
    /// Exit status from WASM.
    pub exit_code: i32,

    /// Stdout captured from execution.
    pub stdout: Vec<u8>,

    /// Trace artifacts (if trace capability was granted).
    pub trace: Option<Vec<u8>>,
}

/// A loaded WASM module (ready for execution).
pub struct LoadedModule {
    module: wasmtime::Module,
}

/// Native runner executor.
pub struct NativeRunner {
    config: RunnerConfig,
    engine: wasmtime::Engine,
}

impl NativeRunner {
    /// Create a new runner with the given configuration.
    pub fn new(config: RunnerConfig) -> Result<Self, crate::RunnerError> {
        let engine = wasmtime::Engine::default();

        Ok(Self { config, engine })
    }

    /// Create a runner for testing (with mock kernel bridge).
    pub fn for_testing() -> Self {
        Self {
            config: RunnerConfig {
                kernel_ipc: "/dev/null".into(),
                trace_output: None,
            },
            engine: wasmtime::Engine::default(),
        }
    }

    /// Load a WASM module.
    pub fn load(&self, wasm_bytes: &[u8]) -> Result<LoadedModule, crate::RunnerError> {
        let module = wasmtime::Module::from_binary(&self.engine, wasm_bytes)
            .map_err(|e| crate::RunnerError::WasmCompile(e.to_string()))?;

        Ok(LoadedModule { module })
    }

    /// Run a loaded module with granted capabilities.
    pub fn run(
        &self,
        module: LoadedModule,
        config: RunConfig,
    ) -> Result<RunResult, crate::RunnerError> {
        use crate::context::InstanceContext;
        use crate::kernel_bridge::MockKernelBridge;
        use wasmtime::*;

        // For now, use mock kernel bridge
        let context = InstanceContext::with_mock();
        let mut store = Store::new(&self.engine, context);

        // Create linker and register host functions
        let mut linker = Linker::new(&self.engine);

        // Get memory from module exports (we'll create it)
        let memory_type = MemoryType::new(1, None);
        let memory = Memory::new(&mut store, memory_type)
            .map_err(|e| crate::RunnerError::WasmInstantiate(e.to_string()))?;

        // Register host functions
        let echo_request = crate::harness::create_echo_request_host(&mut store, memory);
        let echo_reply = crate::harness::create_echo_reply_host(&mut store, memory);
        let trace_read = crate::harness::create_trace_read_host(&mut store, memory);
        let trace_write = crate::harness::create_trace_write_host(&mut store, memory);

        linker.define(
            &mut store,
            "ramen::harness.echo",
            "echo_request::call",
            echo_request,
        ).map_err(|e| crate::RunnerError::WasmInstantiate(e.to_string()))?;

        linker.define(
            &mut store,
            "ramen::harness.echo",
            "echo_reply::call",
            echo_reply,
        ).map_err(|e| crate::RunnerError::WasmInstantiate(e.to_string()))?;

        linker.define(
            &mut store,
            "ramen::harness.trace",
            "trace_read::call",
            trace_read,
        ).map_err(|e| crate::RunnerError::WasmInstantiate(e.to_string()))?;

        linker.define(
            &mut store,
            "ramen::harness.trace",
            "trace_write::call",
            trace_write,
        ).map_err(|e| crate::RunnerError::WasmInstantiate(e.to_string()))?;

        // Instantiate module
        let instance = linker
            .instantiate(&mut store, &module.module)
            .map_err(|e| crate::RunnerError::WasmInstantiate(e.to_string()))?;

        // Inject capabilities via exported globals
        inject_capabilities(&mut store, &instance, &config.granted_handles)?;

        // Find and call _start
        let start = instance
            .get_export(&mut store, "_start")
            .and_then(|e| e.into_func())
            .ok_or_else(|| crate::RunnerError::WasmInstantiate("_start not found".to_string()))?;

        let exit_code: i32 = start
            .call(&mut store, &[], &mut [])
            .map_err(|e| crate::RunnerError::HarnessCall(e.to_string()))?;

        // Collect results
        let context = store.into_data();

        Ok(RunResult {
            exit_code,
            stdout: context.stdout,
            trace: None,
        })
    }

    /// Load and run in one step.
    pub fn load_and_run(
        &self,
        wasm_bytes: &[u8],
        config: RunConfig,
    ) -> Result<RunResult, crate::RunnerError> {
        let module = self.load(wasm_bytes)?;
        self.run(module, config)
    }
}

/// Inject capabilities by setting exported globals.
fn inject_capabilities(
    store: &mut wasmtime::Store<crate::context::InstanceContext>,
    instance: &wasmtime::Instance,
    granted_handles: &HashMap<String, u64>,
) -> Result<(), crate::RunnerError> {
    use wasmtime::Val;

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
        let handle = granted_handles
            .get(name)
            .ok_or_else(|| crate::RunnerError::MissingCapability(name.to_string()))?;

        // Set the global
        global
            .set(store, Val::I64(*handle as i64))
            .map_err(|e| crate::RunnerError::GlobalSet(format!("{}: {}", name, e)))?;
    }

    Ok(())
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p native_runner --lib runner`
Expected: 2 tests pass

**Step 5: Commit**

```bash
git add services/native_runner/src/runner.rs
git commit -m "$(cat <<'EOF'
feat(native-runner): implement runner core

- RunnerConfig and RunConfig types
- NativeRunner with load/run methods
- Capability injection via exported globals
- Fail-closed if missing capability

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Implement CLI Binary

**Files:**
- Create: `services/native_runner/src/bin/native_runner.rs`

**Step 1: Create the CLI**

```rust
// services/native_runner/src/bin/native_runner.rs
//! Native runner CLI.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use native_runner::{NativeRunner, RunConfig, RunnerConfig};

/// Native runner for RamenOS WASM modules.
#[derive(Parser, Debug)]
#[command(name = "native_runner")]
#[command(about = "Execute RamenOS native WASM modules")]
struct Args {
    /// Path to WASM module.
    #[arg(short, long)]
    wasm: PathBuf,

    /// Path to kernel IPC socket.
    #[arg(short, long)]
    kernel_ipc: PathBuf,

    /// Grant capability (repeatable, format: NAME=HANDLE).
    #[arg(short, long = "cap")]
    capabilities: Vec<String>,

    /// Path to write execution trace.
    #[arg(short, long)]
    trace_output: Option<PathBuf>,

    /// Enable verbose output.
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();

    if args.verbose {
        eprintln!("[native_runner] Loading WASM: {:?}", args.wasm);
    }

    // Read WASM file
    let wasm_bytes = match std::fs::read(&args.wasm) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Error reading WASM file: {}", e);
            return ExitCode::from(1);
        }
    };

    // Parse capability grants
    let mut granted_handles = HashMap::new();
    for cap in &args.capabilities {
        let parts: Vec<&str> = cap.splitn(2, '=').collect();
        if parts.len() != 2 {
            eprintln!("Invalid capability format: {} (expected NAME=HANDLE)", cap);
            return ExitCode::from(1);
        }

        let name = parts[0].to_string();
        let handle: u64 = match u64::from_str_radix(parts[1].trim_start_matches("0x"), 16) {
            Ok(h) => h,
            Err(_) => match parts[1].parse::<u64>() {
                Ok(h) => h,
                Err(_) => {
                    eprintln!("Invalid handle value: {}", parts[1]);
                    return ExitCode::from(1);
                }
            },
        };

        if args.verbose {
            eprintln!("[native_runner] Granting {}: {:#x}", name, handle);
        }
        granted_handles.insert(name, handle);
    }

    // Create runner
    let config = RunnerConfig {
        kernel_ipc: args.kernel_ipc,
        trace_output: args.trace_output,
    };

    let runner = match NativeRunner::new(config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error creating runner: {}", e);
            return ExitCode::from(1);
        }
    };

    // Run
    let run_config = RunConfig { granted_handles };

    let result = match runner.load_and_run(&wasm_bytes, run_config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error executing WASM: {}", e);
            return ExitCode::from(1);
        }
    };

    // Output stdout
    if !result.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&result.stdout));
    }

    ExitCode::from(result.exit_code as u8)
}
```

**Step 2: Add clap dependency to Cargo.toml**

```toml
# services/native_runner/Cargo.toml (add to dependencies)
clap = { version = "4", features = ["derive"] }
```

**Step 3: Build and verify**

Run: `cargo build -p native_runner`
Expected: Builds successfully

**Step 4: Test CLI help**

Run: `cargo run -p native_runner -- --help`
Expected: Shows usage information

**Step 5: Commit**

```bash
git add services/native_runner/src/bin/native_runner.rs
git add services/native_runner/Cargo.toml
git commit -m "$(cat <<'EOF'
feat(native-runner): add CLI binary

CLI with options:
  --wasm PATH           WASM module to execute
  --kernel-ipc PATH     Kernel socket path
  --cap NAME=HANDLE     Grant capability (repeatable)
  --trace-output PATH   Trace output file
  --verbose             Debug output

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Integration Test with hello_wasm

**Files:**
- Create: `services/native_runner/tests/integration_test.rs`

**Step 1: Write the failing test**

```rust
// services/native_runner/tests/integration_test.rs
//! Integration tests for native runner.

use native_runner::{NativeRunner, RunConfig, RunnerConfig};
use std::collections::HashMap;
use std::path::PathBuf;

/// Get path to hello_wasm.wasm.
fn hello_wasm_path() -> PathBuf {
    // Look for the WASM file in the expected build location
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../target/wasm32-unknown-unknown/debug/hello_wasm.wasm");
    path
}

#[test]
fn runner_loads_hello_wasm() {
    let wasm_path = hello_wasm_path();
    if !wasm_path.exists() {
        eprintln!("Skipping test: hello_wasm.wasm not found at {:?}", wasm_path);
        eprintln!("Build it with: cargo build -p hello_wasm --target wasm32-unknown-unknown");
        return;
    }

    let wasm_bytes = std::fs::read(&wasm_path).expect("Failed to read WASM file");

    let runner = NativeRunner::for_testing();
    let module = runner.load(&wasm_bytes).expect("Failed to load WASM");

    // Just verify we can load it
    assert!(format!("{:?}", module).contains("Module"));
}

#[test]
fn runner_injects_capabilities() {
    let wasm_path = hello_wasm_path();
    if !wasm_path.exists() {
        eprintln!("Skipping test: hello_wasm.wasm not found");
        return;
    }

    let wasm_bytes = std::fs::read(&wasm_path).expect("Failed to read WASM file");

    let runner = NativeRunner::for_testing();

    let config = RunConfig {
        granted_handles: HashMap::from([
            ("RAMEN_CAP_ECHO_REQUEST".to_string(), 0x1234_5678_1234_5678),
        ]),
    };

    // This should succeed - the module expects RAMEN_CAP_ECHO_REQUEST
    let result = runner.load_and_run(&wasm_bytes, config);
    assert!(result.is_ok(), "Failed to run: {:?}", result.err());
}

#[test]
fn runner_fails_without_required_capability() {
    let wasm_path = hello_wasm_path();
    if !wasm_path.exists() {
        eprintln!("Skipping test: hello_wasm.wasm not found");
        return;
    }

    let wasm_bytes = std::fs::read(&wasm_path).expect("Failed to read WASM file");

    let runner = NativeRunner::for_testing();

    // Empty granted_handles - module should fail to run
    let config = RunConfig {
        granted_handles: HashMap::new(),
    };

    let result = runner.load_and_run(&wasm_bytes, config);
    assert!(result.is_err(), "Should fail without required capability");
}
```

**Step 2: Build hello_wasm**

Run: `cargo build -p hello_wasm --target wasm32-unknown-unknown`
Expected: Builds hello_wasm.wasm

**Step 3: Run integration tests**

Run: `cargo test -p native_runner --test integration_test`
Expected: 3 tests pass (or skip if WASM not built)

**Step 4: Commit**

```bash
git add services/native_runner/tests/integration_test.rs
git commit -m "$(cat <<'EOF'
test(native-runner): add integration tests

Tests for:
- Loading hello_wasm.wasm
- Capability injection
- Fail-closed without required capability

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Add Foundry Gate

**Files:**
- Create: `tools/ci/foundry_native_runner_s10_0.sh`
- Modify: `justfile`

**Step 1: Create Foundry gate script**

```bash
#!/usr/bin/env bash
# tools/ci/foundry_native_runner_s10_0.sh
#
# Foundry gate for S10.0 Native Runner.
#
# Assertions:
# 1. native_runner binary builds
# 2. hello_wasm.wasm builds
# 3. native_runner loads hello_wasm.wasm
# 4. Capability injection succeeds
# 5. Exit code is 0

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

echo "=== S10.0 Native Runner Foundry Gate ==="
echo

# Track pass/fail
PASSED=0
FAILED=0

pass() {
    echo "PASS: $1"
    ((PASSED++))
}

fail() {
    echo "FAIL: $1"
    ((FAILED++))
}

# Assertion 1: native_runner binary builds
echo "Asserting native_runner builds..."
if cargo build -p native_runner 2>&1; then
    pass "native_runner builds"
else
    fail "native_runner builds"
fi

# Assertion 2: hello_wasm.wasm builds
echo
echo "Asserting hello_wasm builds..."
if cargo build -p hello_wasm --target wasm32-unknown-unknown 2>&1; then
    pass "hello_wasm builds"
else
    fail "hello_wasm builds"
fi

# Assertion 3-5: Run integration tests
echo
echo "Running integration tests..."
if cargo test -p native_runner --test integration_test 2>&1; then
    pass "integration tests pass"
else
    fail "integration tests"
fi

# Summary
echo
echo "=== S10.0 Summary ==="
echo "PASSED: ${PASSED}"
echo "FAILED: ${FAILED}"

if [[ ${FAILED} -eq 0 ]]; then
    echo
    echo "FOUNDRY_NATIVE_RUNNER_S10_0: METRIC passed=${PASSED} failed=0"
    echo "FOUNDRY_NATIVE_RUNNER_S10_0: ok"
    exit 0
else
    echo
    echo "FOUNDRY_NATIVE_RUNNER_S10_0: METRIC passed=${PASSED} failed=${FAILED}"
    echo "FOUNDRY_NATIVE_RUNNER_S10_0: FAIL"
    exit 1
fi
```

**Step 2: Make executable**

Run: `chmod +x tools/ci/foundry_native_runner_s10_0.sh`

**Step 3: Add to justfile**

```make
# In justfile, add:

# S10.0 Native Runner gate
foundry-native-runner-s10-0:
    tools/ci/foundry_native_runner_s10_0.sh
```

**Step 4: Run the gate**

Run: `just foundry-native-runner-s10-0`
Expected: All assertions pass

**Step 5: Commit**

```bash
git add tools/ci/foundry_native_runner_s10_0.sh
git add justfile
git commit -m "$(cat <<'EOF'
feat(foundry): add S10.0 native runner gate

Gate assertions:
- native_runner binary builds
- hello_wasm.wasm builds
- Integration tests pass
- Capability injection succeeds

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: Update CURRENT_STATUS.md

**Files:**
- Modify: `CURRENT_STATUS.md`

**Step 1: Add S10.0 section to CURRENT_STATUS.md**

Add after the V-006 Phase 4a section:

```markdown
### S10.0: Native Runner Executor (COMPLETE)
**Completion Date:** 2026-02-18
**Test Results:** All integration tests passing

**Deliverables:**
- `services/native_runner/` crate with library + CLI
- Wasmtime integration for WASM execution
- Capability injection via `RAMEN_CAP_*` exported globals
- Host functions for echo_harness_v0 and trace_service_v1
- Mock kernel bridge for testing
- Foundry gate: `foundry_native_runner_s10_0.sh`

**Architecture:**
- Runner is executor-only (no policy decisions)
- Host functions are plumbing: decode → single kernel IPC → encode
- Fail-closed: missing capability causes load error

**Files Created:**
- `services/native_runner/Cargo.toml`
- `services/native_runner/src/lib.rs`
- `services/native_runner/src/error.rs`
- `services/native_runner/src/kernel_bridge.rs`
- `services/native_runner/src/context.rs`
- `services/native_runner/src/runner.rs`
- `services/native_runner/src/harness/mod.rs`
- `services/native_runner/src/harness/echo.rs`
- `services/native_runner/src/harness/trace.rs`
- `services/native_runner/src/bin/native_runner.rs`
- `services/native_runner/tests/integration_test.rs`
- `tools/ci/foundry_native_runner_s10_0.sh`

**Next Steps (S10.1):**
- Manifest parsing (`native_wasm_v0`)
- Broker integration (Domain Manager)
- Runtime supervisor integration
- Real kernel IPC (not mock)
```

**Step 2: Update the "Next milestone" section**

Change:
```markdown
## Next milestone
V-006 Phase 4: Native Runner Design - Design and implement native personality runner to replace deprecated POSIX runner v0.
```

To:
```markdown
## Next milestone
S10.1: Broker Integration - Connect native runner to Domain Manager for capability grants, add manifest parsing, and integrate with runtime_supervisor.
```

**Step 3: Commit**

```bash
git add CURRENT_STATUS.md
git commit -m "$(cat <<'EOF'
docs(status): add S10.0 native runner completion

Update CURRENT_STATUS.md with S10.0 deliverables
and set S10.1 as next milestone.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Summary

This plan implements S10.0 Native Runner Executor in 10 tasks:

1. **Crate Structure** - Cargo.toml, lib.rs, error.rs
2. **Kernel Bridge** - Trait + mock for testing
3. **Instance Context** - WASM execution context
4. **Echo Harness** - Host functions for echo_harness_v0
5. **Trace Harness** - Host functions for trace_service_v1
6. **Runner Core** - Load, inject, execute
7. **CLI Binary** - Command-line interface
8. **Integration Tests** - Test with hello_wasm
9. **Foundry Gate** - Automated gate script
10. **Documentation** - Update CURRENT_STATUS.md

Each task follows TDD: write failing test → implement → verify → commit.
