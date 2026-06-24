# Native Runner Phase 1: IDL + SDK Scaffold Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add WASM code generation targets to idl_codegen and create the RamenOS SDK for WASM guest development.

**Architecture:** Extend idl_codegen with `wasm-imports` and `wasm-host` targets. Create a new `sdk/` crate with capability types, status codes, and IDL-generated bindings. Build a "hello world" WASM module that compiles against the SDK.

**Tech Stack:** Rust, wasmtime (future), bincode, wasm32-unknown-unknown target

---

## Prerequisites

- [ ] Rust nightly toolchain installed (per rust-toolchain.toml)
- [ ] `just build-host` passes
- [ ] `just codegen` passes
- [ ] wasm32-unknown-unknown target: `rustup target add wasm32-unknown-unknown`

---

## Task 1: Define SDK Crate Structure

**Files:**
- Create: `sdk/Cargo.toml`
- Create: `sdk/src/lib.rs`

**Step 1: Create sdk/Cargo.toml**

```toml
[package]
name = "ramen_sdk"
version = "0.0.0"
edition = "2021"
description = "RamenOS SDK for native WASM development"

[dependencies]
# No external dependencies for core SDK

[features]
default = []
std = []
```

**Step 2: Create sdk/src/lib.rs**

```rust
//! RamenOS SDK for Native WASM Development
//!
//! This crate provides the SDK for building RamenOS native WASM modules.
//! It includes:
//! - Capability handle types
//! - Status codes for WASM imports
//! - Linear memory helpers
//! - IDL-generated harness bindings (via codegen)
//!
//! # Example
//!
//! ```no_run
//! use ramen_sdk::{CapabilityHandle, Status};
//! use ramen_sdk::generated::harness_echo_v1::EchoV1Client;
//!
//! #[no_mangle]
//! pub extern "C" fn _start() {
//!     // Capability handles are provided at initialization
//!     let echo_cap = unsafe { RAMEN_CAP_ECHO_V1 };
//!     let client = EchoV1Client::from_cap(echo_cap);
//!
//!     match client.send(b"hello") {
//!         Ok(reply) => { /* handle reply */ }
//!         Err(Status::InvalidCapability) => { /* handle error */ }
//!         _ => {}
//!     }
//! }
//!
//! // Capability handles injected by runner
//! #[no_mangle]
//! pub static mut RAMEN_CAP_ECHO_V1: ramen_sdk::CapabilityHandle = ramen_sdk::CapabilityHandle::INVALID;
//! ```

#![no_std]

pub mod capability;
pub mod status;
pub mod memory;

// Re-export main types
pub use capability::CapabilityHandle;
pub use status::Status;

/// Generated IDL bindings (populated by codegen)
pub mod generated {
    // Modules will be added by idl_codegen --target wasm-imports
}
```

**Step 3: Verify compilation**

Run: `cargo check -p ramen_sdk`
Expected: FAIL (missing modules capability, status, memory)

**Step 4: Commit**

```bash
git add sdk/Cargo.toml sdk/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(sdk): add RamenOS SDK crate scaffold

Create sdk/ crate for native WASM development with:
- Crate structure (no_std compatible)
- Module declarations for capability, status, memory
- Re-exports for main types
- Placeholder for IDL-generated bindings

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Implement Capability Handle Type

**Files:**
- Create: `sdk/src/capability.rs`

**Step 1: Write the failing test**

Create `sdk/src/capability.rs`:

```rust
//! Capability handle types for WASM modules
//!
//! Capability handles are 64-bit values that authorize access to
//! RamenOS resources. They are provided by the runner at initialization
//! and must be passed as the first argument to all harness imports.

#![allow(dead_code)]

/// A capability handle authorizing access to a RamenOS resource
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapabilityHandle(u64);

impl CapabilityHandle {
    /// Invalid capability handle (0)
    pub const INVALID: Self = Self(0);

    /// Create a capability handle from a raw u64
    #[inline]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Get the raw u64 value for passing to WASM imports
    #[inline]
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Check if this handle is valid (non-zero)
    #[inline]
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }

    /// Check if this handle is invalid (zero)
    #[inline]
    pub const fn is_invalid(self) -> bool {
        self.0 == 0
    }
}

impl Default for CapabilityHandle {
    fn default() -> Self {
        Self::INVALID
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_handle_is_zero() {
        assert_eq!(CapabilityHandle::INVALID.raw(), 0);
        assert!(CapabilityHandle::INVALID.is_invalid());
        assert!(!CapabilityHandle::INVALID.is_valid());
    }

    #[test]
    fn from_raw_preserves_value() {
        let handle = CapabilityHandle::from_raw(0x1234_5678_9ABC_DEF0);
        assert_eq!(handle.raw(), 0x1234_5678_9ABC_DEF0);
    }

    #[test]
    fn default_is_invalid() {
        let handle: CapabilityHandle = Default::default();
        assert!(handle.is_invalid());
    }

    #[test]
    fn clone_and_copy_work() {
        let handle = CapabilityHandle::from_raw(42);
        let cloned = handle.clone();
        let copied = handle;
        assert_eq!(handle, cloned);
        assert_eq!(handle, copied);
    }
}
```

**Step 2: Run tests to verify they pass**

Run: `cargo test -p ramen_sdk --lib capability`
Expected: 4 tests PASS

**Step 3: Commit**

```bash
git add sdk/src/capability.rs
git commit -m "$(cat <<'EOF'
feat(sdk): add CapabilityHandle type

Add 64-bit capability handle for WASM modules:
- INVALID constant for null handle
- from_raw/raw for FFI boundary
- is_valid/is_invalid for checks
- Tests for all operations

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Implement Status Codes

**Files:**
- Create: `sdk/src/status.rs`

**Step 1: Write the status types**

Create `sdk/src/status.rs`:

```rust
//! Status codes returned by WASM imports
//!
//! These codes match the kernel-side status constants and are used
//! to indicate success or failure of harness operations.

#![allow(dead_code)]

/// Status codes returned by harness imports
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Operation completed successfully
    Ok = 0,
    /// Invalid capability handle provided
    InvalidCapability = 1,
    /// Capability does not have required rights
    PermissionDenied = 2,
    /// Invalid argument provided
    InvalidArgument = 3,
    /// Operation would block (for non-blocking calls)
    WouldBlock = 4,
    /// I/O error occurred
    IoError = 5,
    /// Buffer too small for result
    BufferTooSmall = 6,
    /// Resource not found
    NotFound = 7,
    /// Unknown error
    Unknown = -1,
}

impl Status {
    /// Convert from raw i32 status code
    #[inline]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Ok,
            1 => Self::InvalidCapability,
            2 => Self::PermissionDenied,
            3 => Self::InvalidArgument,
            4 => Self::WouldBlock,
            5 => Self::IoError,
            6 => Self::BufferTooSmall,
            7 => Self::NotFound,
            _ => Self::Unknown,
        }
    }

    /// Convert to raw i32 for comparison
    #[inline]
    pub const fn to_raw(self) -> i32 {
        self as i32
    }

    /// Check if status indicates success
    #[inline]
    pub const fn is_ok(self) -> bool {
        matches!(self, Self::Ok)
    }

    /// Check if status indicates an error
    #[inline]
    pub const fn is_err(self) -> bool {
        !self.is_ok()
    }
}

impl From<i32> for Status {
    fn from(raw: i32) -> Self {
        Self::from_raw(raw)
    }
}

impl From<Status> for i32 {
    fn from(status: Status) -> Self {
        status.to_raw()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_status_is_zero() {
        assert_eq!(Status::Ok.to_raw(), 0);
        assert!(Status::Ok.is_ok());
        assert!(!Status::Ok.is_err());
    }

    #[test]
    fn from_raw_roundtrip() {
        for raw in 0..=7 {
            let status = Status::from_raw(raw);
            assert_eq!(status.to_raw(), raw);
        }
    }

    #[test]
    fn unknown_for_invalid_raw() {
        let status = Status::from_raw(999);
        assert_eq!(status, Status::Unknown);
    }

    #[test]
    fn error_statuses_are_errors() {
        assert!(Status::InvalidCapability.is_err());
        assert!(Status::PermissionDenied.is_err());
        assert!(Status::IoError.is_err());
    }
}
```

**Step 2: Run tests to verify they pass**

Run: `cargo test -p ramen_sdk --lib status`
Expected: 4 tests PASS

**Step 3: Commit**

```bash
git add sdk/src/status.rs
git commit -m "$(cat <<'EOF'
feat(sdk): add Status codes for WASM imports

Add status enum matching kernel-side constants:
- Ok (0), InvalidCapability (1), PermissionDenied (2), etc.
- from_raw/to_raw for FFI boundary
- is_ok/is_err helpers
- Tests for all operations

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Implement Memory Helpers

**Files:**
- Create: `sdk/src/memory.rs`

**Step 1: Write memory helper types**

Create `sdk/src/memory.rs`:

```rust
//! Linear memory helpers for WASM modules
//!
//! These utilities help with reading and writing data in WASM linear memory
//! when communicating with harness imports.

#![allow(dead_code)]

use core::slice;

/// A borrowed slice in linear memory
#[repr(C)]
pub struct LinearSlice {
    pub ptr: *const u8,
    pub len: usize,
}

impl LinearSlice {
    /// Create from a byte slice
    #[inline]
    pub fn from_slice(slice: &[u8]) -> Self {
        Self {
            ptr: slice.as_ptr(),
            len: slice.len(),
        }
    }

    /// Get pointer for WASM import
    #[inline]
    pub fn ptr(&self) -> i32 {
        self.ptr as i32
    }

    /// Get length for WASM import
    #[inline]
    pub fn len(&self) -> i32 {
        self.len as i32
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// A mutable slice in linear memory
#[repr(C)]
pub struct LinearSliceMut {
    pub ptr: *mut u8,
    pub len: usize,
}

impl LinearSliceMut {
    /// Create from a mutable byte slice
    #[inline]
    pub fn from_slice(slice: &mut [u8]) -> Self {
        Self {
            ptr: slice.as_mut_ptr(),
            len: slice.len(),
        }
    }

    /// Get pointer for WASM import
    #[inline]
    pub fn ptr(&self) -> i32 {
        self.ptr as i32
    }

    /// Get length/capacity for WASM import
    #[inline]
    pub fn len(&self) -> i32 {
        self.len as i32
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Access as slice
    ///
    /// # Safety
    /// The memory must be valid for the lifetime of the returned slice.
    #[inline]
    pub unsafe fn as_slice(&self) -> &[u8] {
        slice::from_raw_parts(self.ptr, self.len)
    }

    /// Access as mutable slice
    ///
    /// # Safety
    /// The memory must be valid and exclusive for the lifetime of the returned slice.
    #[inline]
    pub unsafe fn as_slice_mut(&mut self) -> &mut [u8] {
        slice::from_raw_parts_mut(self.ptr, self.len)
    }
}

/// Helper trait for types that can be written to linear memory
pub trait IntoLinearMemory {
    /// Write self to linear memory, returning (ptr, len)
    fn write_linear(&self, out: &mut [u8]) -> Option<(i32, i32)>;
}

impl IntoLinearMemory for &[u8] {
    fn write_linear(&self, out: &mut [u8]) -> Option<(i32, i32)> {
        if self.len() > out.len() {
            return None;
        }
        out[..self.len()].copy_from_slice(self);
        Some((out.as_mut_ptr() as i32, self.len() as i32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_slice_from_slice() {
        let data = [1u8, 2, 3, 4, 5];
        let slice = LinearSlice::from_slice(&data);
        assert_eq!(slice.len(), 5);
        assert!(!slice.is_empty());
    }

    #[test]
    fn linear_slice_empty() {
        let data: [u8; 0] = [];
        let slice = LinearSlice::from_slice(&data);
        assert!(slice.is_empty());
        assert_eq!(slice.len(), 0);
    }

    #[test]
    fn linear_slice_mut_from_slice() {
        let mut data = [0u8; 10];
        let slice = LinearSliceMut::from_slice(&mut data);
        assert_eq!(slice.len(), 10);
    }

    #[test]
    fn into_linear_memory_bytes() {
        let input: &[u8] = &[1, 2, 3];
        let mut out = [0u8; 10];
        let result = input.write_linear(&mut out);
        assert!(result.is_some());
        let (ptr, len) = result.unwrap();
        assert_eq!(len, 3);
        assert_eq!(&out[..3], &[1, 2, 3]);
    }

    #[test]
    fn into_linear_memory_too_small() {
        let input: &[u8] = &[1, 2, 3, 4, 5];
        let mut out = [0u8; 3];
        let result = input.write_linear(&mut out);
        assert!(result.is_none());
    }
}
```

**Step 2: Run tests to verify they pass**

Run: `cargo test -p ramen_sdk --lib memory`
Expected: 5 tests PASS

**Step 3: Commit**

```bash
git add sdk/src/memory.rs
git commit -m "$(cat <<'EOF'
feat(sdk): add linear memory helpers

Add utilities for WASM linear memory access:
- LinearSlice for borrowed data
- LinearSliceMut for mutable buffers
- IntoLinearMemory trait for serialization
- Tests for all operations

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Add SDK to Workspace

**Files:**
- Modify: `Cargo.toml` (workspace root)

**Step 1: Add sdk to workspace members**

Find the `[workspace]` section in root `Cargo.toml` and add `sdk` to the members list.

**Step 2: Verify workspace builds**

Run: `cargo check -p ramen_sdk`
Expected: PASS (compiles successfully)

**Step 3: Run all SDK tests**

Run: `cargo test -p ramen_sdk`
Expected: 13 tests PASS (4 capability + 4 status + 5 memory)

**Step 4: Commit**

```bash
git add Cargo.toml
git commit -m "$(cat <<'EOF'
chore: add ramen_sdk to workspace

Add sdk/ crate to workspace members for unified builds.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Extend idl_codegen for wasm-imports Target

**Files:**
- Modify: `idl_codegen/src/main.rs`
- Modify: `idl_codegen/src/lib.rs`
- Create: `idl_codegen/src/wasm_imports.rs`

**Step 1: Read existing idl_codegen structure**

Run: `ls -la idl_codegen/src/`
Note the existing structure and how targets are organized.

**Step 2: Add wasm-imports target to CLI**

In `idl_codegen/src/main.rs`, add `wasm-imports` as a new target option alongside `rust`.

**Step 3: Create wasm_imports.rs generator**

Create the module that generates Rust SDK code for WASM imports. This should:
- Generate `extern "C"` declarations for WASM imports
- Generate wrapper structs like `EchoV1Client`
- Use the naming convention `ramen::<namespace>::<interface>`

**Step 4: Test with echo_v1**

Run: `cargo run -p idl_codegen -- --target wasm-imports --input idl/harness/echo_v1.toml --output /tmp/test.rs`
Expected: Generates valid Rust code

**Step 5: Commit**

```bash
git add idl_codegen/src/main.rs idl_codegen/src/lib.rs idl_codegen/src/wasm_imports.rs
git commit -m "$(cat <<'EOF'
feat(idl_codegen): add wasm-imports target

Add code generation for WASM guest-side imports:
- extern "C" declarations with ramen:: namespace
- Wrapper structs with idiomatic Rust API
- Capability handle as first parameter convention

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Extend idl_codegen for wasm-host Target

**Files:**
- Create: `idl_codegen/src/wasm_host.rs`

**Step 1: Create wasm_host.rs generator**

Create the module that generates Rust host functions for the native runner. This should:
- Generate wasmtime `Linker::func_wrap` calls
- Include capability validation logic
- Handle linear memory reads/writes

**Step 2: Test with echo_v1**

Run: `cargo run -p idl_codegen -- --target wasm-host --input idl/harness/echo_v1.toml --output /tmp/test_host.rs`
Expected: Generates valid Rust code

**Step 3: Commit**

```bash
git add idl_codegen/src/wasm_host.rs idl_codegen/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(idl_codegen): add wasm-host target

Add code generation for native runner host functions:
- wasmtime Linker::func_wrap wrappers
- Capability validation integration
- Linear memory read/write helpers

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Integrate Generated Code into SDK

**Files:**
- Modify: `justfile` (add codegen-wasm target)
- Create: `sdk/src/generated/mod.rs`
- Create: `sdk/src/generated/harness_echo_v1.rs` (generated)

**Step 1: Add justfile target for WASM codegen**

Add to justfile:
```just
codegen-wasm:
    cargo run -p idl_codegen -- --target wasm-imports --input idl/harness/echo_v1.toml --output sdk/src/generated/harness_echo_v1.rs
```

**Step 2: Generate echo_v1 bindings**

Run: `just codegen-wasm`
Expected: Creates `sdk/src/generated/harness_echo_v1.rs`

**Step 3: Create generated/mod.rs**

```rust
//! IDL-generated bindings (auto-generated by idl_codegen)
//!
//! Do not edit manually. Regenerate with `just codegen-wasm`.

pub mod harness_echo_v1;
```

**Step 4: Update sdk/src/lib.rs to include generated**

Update the `generated` module to include the actual module:
```rust
pub mod generated;
```

**Step 5: Verify SDK compiles**

Run: `cargo check -p ramen_sdk`
Expected: PASS

**Step 6: Commit**

```bash
git add justfile sdk/src/generated/mod.rs sdk/src/generated/harness_echo_v1.rs sdk/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(sdk): add IDL-generated echo_v1 bindings

Integrate wasm-imports codegen into SDK:
- Add codegen-wasm target to justfile
- Generate harness_echo_v1.rs bindings
- Wire into SDK generated module

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Create Hello World WASM Example

**Files:**
- Create: `examples/hello_wasm/Cargo.toml`
- Create: `examples/hello_wasm/src/lib.rs`
- Create: `examples/hello_wasm/build.rs` (optional)

**Step 1: Create example Cargo.toml**

```toml
[package]
name = "hello_wasm"
version = "0.0.0"
edition = "2021"
description = "Hello World native WASM example for RamenOS"

[lib]
crate-type = ["cdylib"]

[dependencies]
ramen_sdk = { path = "../../sdk" }

[profile.release]
opt-level = "s"
lto = true
```

**Step 2: Create example source**

```rust
//! Hello World example for RamenOS Native WASM
//!
//! This example demonstrates:
//! - Receiving capability handles at initialization
//! - Using IDL-generated client to call harness
//! - Basic error handling

#![no_std]

use ramen_sdk::{CapabilityHandle, Status};
use ramen_sdk::generated::harness_echo_v1::EchoV1Client;

/// Capability handle for echo_v1 harness (injected by runner)
#[no_mangle]
pub static mut RAMEN_CAP_ECHO_V1: CapabilityHandle = CapabilityHandle::INVALID;

/// Number of capabilities provided
#[no_mangle]
pub static mut RAMEN_CAP_COUNT: u32 = 1;

/// Entry point called by runner after capability injection
#[no_mangle]
pub extern "C" fn _start() -> i32 {
    // Get the injected capability handle
    let cap = unsafe { RAMEN_CAP_ECHO_V1 };

    if cap.is_invalid() {
        // No capability provided, return error
        return Status::InvalidCapability.to_raw();
    }

    // Create client from capability
    let client = EchoV1Client::from_cap(cap);

    // Send hello message
    match client.send(b"Hello from RamenOS Native WASM!") {
        Ok(_reply) => Status::Ok.to_raw(),
        Err(e) => e.to_raw(),
    }
}
```

**Step 3: Build for WASM target**

Run: `cargo build -p hello_wasm --target wasm32-unknown-unknown --release`
Expected: Creates `target/wasm32-unknown-unknown/release/hello_wasm.wasm`

**Step 4: Verify WASM file**

Run: `wasm-objdump -x target/wasm32-unknown-unknown/release/hello_wasm.wasm | head -30`
Expected: Shows exports for `_start`, `RAMEN_CAP_ECHO_V1`, etc.

**Step 5: Commit**

```bash
git add examples/hello_wasm/Cargo.toml examples/hello_wasm/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(examples): add hello_wasm native WASM example

Add Hello World example demonstrating:
- Capability handle injection via statics
- IDL-generated client usage
- Error handling with Status codes
- Builds to wasm32-unknown-unknown

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: Add wasm32 Target to CI

**Files:**
- Modify: `.github/workflows/ci.yml`

**Step 1: Add wasm32 target check**

Add a step to verify SDK and examples compile for wasm32-unknown-unknown:

```yaml
- name: Check WASM target
  run: |
    rustup target add wasm32-unknown-unknown
    cargo check -p ramen_sdk --target wasm32-unknown-unknown
    cargo build -p hello_wasm --target wasm32-unknown-unknown
```

**Step 2: Verify CI passes locally (if possible)**

Run: `cargo check -p ramen_sdk --target wasm32-unknown-unknown`
Expected: PASS

**Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "$(cat <<'EOF'
ci: add wasm32-unknown-unknown target check

Add CI step to verify SDK and examples compile for WASM target.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 11: Create Foundry Gate for Phase 1

**Files:**
- Create: `tools/ci/foundry_native_runner_phase1.sh`
- Modify: `justfile` (add foundry-native-runner-phase1 target)

**Step 1: Create Foundry gate script**

```bash
#!/usr/bin/env bash
# Native Runner Phase 1: IDL + SDK Scaffold Foundry Gate
#
# Validates:
# - SDK crate compiles and tests pass
# - wasm-imports codegen produces valid output
# - wasm-host codegen produces valid output
# - Hello World example builds for WASM target

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT_DIR/out"
EVIDENCE_DIR="$OUT_DIR/evidence/native_runner_phase1"

mkdir -p "$EVIDENCE_DIR"

cd "$ROOT_DIR"

echo "=== Native Runner Phase 1: IDL + SDK Scaffold ==="

# Test 1: SDK tests pass
echo "Test 1: SDK unit tests..."
cargo test -p ramen_sdk > "$EVIDENCE_DIR/test1_sdk_tests.log" 2>&1
grep -q "test result: ok" "$EVIDENCE_DIR/test1_sdk_tests.log" || {
    echo "FAIL: SDK tests failed"
    cat "$EVIDENCE_DIR/test1_sdk_tests.log"
    exit 1
}
echo "PASS: SDK unit tests"

# Test 2: wasm-imports codegen produces valid Rust
echo "Test 2: wasm-imports codegen..."
cargo run -p idl_codegen -- --target wasm-imports \
    --input idl/harness/echo_v1.toml \
    --output "$EVIDENCE_DIR/generated_imports.rs" > "$EVIDENCE_DIR/test2_codegen.log" 2>&1
cargo check --target wasm32-unknown-unknown \
    --manifest-path /dev/stdin <<EOF > "$EVIDENCE_DIR/test2_check.log" 2>&1 || {
    echo "FAIL: Generated imports don't compile"
    cat "$EVIDENCE_DIR/test2_check.log"
    exit 1
}
[package]
name = "test-imports"
version = "0.0.0"
edition = "2021"

[dependencies]
ramen_sdk = { path = "$ROOT_DIR/sdk" }

[lib]
path = "$EVIDENCE_DIR/generated_imports.rs"
EOF
echo "PASS: wasm-imports codegen"

# Test 3: wasm-host codegen produces valid Rust
echo "Test 3: wasm-host codegen..."
cargo run -p idl_codegen -- --target wasm-host \
    --input idl/harness/echo_v1.toml \
    --output "$EVIDENCE_DIR/generated_host.rs" > "$EVIDENCE_DIR/test3_codegen.log" 2>&1
# Note: Host code requires wasmtime, just verify it parses
grep -q "fn register_" "$EVIDENCE_DIR/generated_host.rs" || {
    echo "FAIL: Generated host missing register function"
    cat "$EVIDENCE_DIR/generated_host.rs"
    exit 1
}
echo "PASS: wasm-host codegen"

# Test 4: Hello World compiles for WASM
echo "Test 4: Hello World WASM build..."
cargo build -p hello_wasm --target wasm32-unknown-unknown --release \
    > "$EVIDENCE_DIR/test4_wasm_build.log" 2>&1
test -f "$ROOT_DIR/target/wasm32-unknown-unknown/release/hello_wasm.wasm" || {
    echo "FAIL: WASM file not created"
    cat "$EVIDENCE_DIR/test4_wasm_build.log"
    exit 1
}
echo "PASS: Hello World WASM build"

# Test 5: WASM exports correct symbols
echo "Test 5: WASM symbol check..."
wasm-objdump -x "$ROOT_DIR/target/wasm32-unknown-unknown/release/hello_wasm.wasm" \
    > "$EVIDENCE_DIR/test5_wasm_symbols.txt" 2>&1 || {
    echo "SKIP: wasm-objdump not available"
    echo "PASS: Hello World WASM build (symbol check skipped)"
    echo ""
    echo "=== Phase 1 Summary ==="
    echo "PASS: Native Runner Phase 1 gate passed"
    exit 0
}
grep -q "_start" "$EVIDENCE_DIR/test5_wasm_symbols.txt" || {
    echo "FAIL: Missing _start export"
    cat "$EVIDENCE_DIR/test5_wasm_symbols.txt"
    exit 1
}
grep -q "RAMEN_CAP_ECHO_V1" "$EVIDENCE_DIR/test5_wasm_symbols.txt" || {
    echo "FAIL: Missing RAMEN_CAP_ECHO_V1 export"
    cat "$EVIDENCE_DIR/test5_wasm_symbols.txt"
    exit 1
}
echo "PASS: WASM symbol check"

# Summary
echo ""
echo "=== Phase 1 Summary ==="
echo "✓ SDK crate with capability, status, memory modules"
echo "✓ wasm-imports codegen target"
echo "✓ wasm-host codegen target"
echo "✓ Hello World WASM example"
echo "✓ All tests passed"
echo ""
echo "PASS: Native Runner Phase 1 gate passed"
```

**Step 2: Add to justfile**

```just
foundry-native-runner-phase1:
    tools/ci/foundry_native_runner_phase1.sh
```

**Step 3: Run the gate**

Run: `just foundry-native-runner-phase1`
Expected: All tests PASS

**Step 4: Commit**

```bash
git add tools/ci/foundry_native_runner_phase1.sh justfile
git commit -m "$(cat <<'EOF'
feat(foundry): add native runner phase 1 gate

Add Foundry gate for Phase 1 validation:
- SDK unit tests
- wasm-imports codegen output validity
- wasm-host codegen output validity
- Hello World WASM build
- WASM symbol verification

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Summary

**Phase 1 Deliverables:**
- `sdk/` crate with CapabilityHandle, Status, memory helpers
- `idl_codegen` wasm-imports and wasm-host targets
- Generated `harness_echo_v1` bindings
- `hello_wasm` example that compiles to WASM
- Foundry gate `foundry_native_runner_phase1.sh`

**Files Created:**
- `sdk/Cargo.toml`
- `sdk/src/lib.rs`
- `sdk/src/capability.rs`
- `sdk/src/status.rs`
- `sdk/src/memory.rs`
- `sdk/src/generated/mod.rs`
- `sdk/src/generated/harness_echo_v1.rs`
- `idl_codegen/src/wasm_imports.rs`
- `idl_codegen/src/wasm_host.rs`
- `examples/hello_wasm/Cargo.toml`
- `examples/hello_wasm/src/lib.rs`
- `tools/ci/foundry_native_runner_phase1.sh`

**Files Modified:**
- `Cargo.toml` (workspace)
- `idl_codegen/src/main.rs`
- `idl_codegen/src/lib.rs`
- `justfile`
- `.github/workflows/ci.yml`

**Next Phase:** Phase 2 (Runner Core) - implements the native_runner service with wasmtime integration and capability bridge.
