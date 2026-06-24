# Design: Code Quality & Security Remediation

**Last Updated:** 2026-02-17
Status: Approved
Scope: Address 10 issues identified in comprehensive project review

## Overview

This design addresses security vulnerabilities, documentation gaps, and code quality issues across RamenOS. The remediation is organized into 4 work streams executed in dependency order.

## Issues Addressed

| # | Priority | Issue | Location |
|---|----------|-------|----------|
| 1 | High | `RAMEN_STORE_DEV_MODE` runtime bypass | `store_service/capability.rs` |
| 2 | High | Excessive `expect()` causing DoS risk | `domain_manager/main.rs` |
| 3 | High | 40+ unsafe blocks without safety comments | `kernel/src/**/*.rs` |
| 4 | High | XOR-based session IDs (trivially spoofable) | `capsule_relay/main.rs` |
| 5 | Medium | Path traversal risk | `capsule_relay/main.rs` |
| 6 | Medium | Duplicate IPC error reply code | `kernel/ipc_v0.rs` |
| 7 | Low | GPU operations mixed with domain_manager | `services/domain_manager` |
| 8 | Low | Magic numbers instead of named constants | Various |
| 9 | Low | Missing rustdoc on public APIs | `kernel_api`, `kernel` |
| 10 | Low | Bump allocator TODO comment | `kernel/mm/bump.rs` |

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    WORK STREAM 1: Security                      │
│  store_service     domain_manager     capsule_relay            │
│  • Dev mode →      • expect() →       • XOR session            │
│    compile-time      Result<T,E>        → crypto rand          │
│    feature flag    • Error replies     • Path sanitize         │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                    WORK STREAM 2: Documentation                 │
│  kernel/src/ - Add // SAFETY: comments to all 40+ unsafe blocks│
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                    WORK STREAM 3: Refactoring                   │
│  kernel/ipc_v0     services/gpu_manager (NEW)                   │
│  • Error reply     • Extract from domain_manager                │
│    helper fn       • Own IDL contract                           │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                    WORK STREAM 4: Code Quality                  │
│  • Named constants • Rustdoc • Clean up TODOs                   │
└─────────────────────────────────────────────────────────────────┘
```

## Work Stream 1: Security Fixes

### 1.1 Store Service Dev Mode

Replace runtime `RAMEN_STORE_DEV_MODE` env check with compile-time feature flag.

**File:** `services/store_service/Cargo.toml`
```toml
[features]
default = []
dev_insecure = []  # MUST NOT be in default
```

**File:** `services/store_service/src/capability.rs`
```rust
#[cfg(feature = "dev_insecure")]
fn dev_mode_fallback_key() -> Vec<EdVerifyingKey> {
    // Development mode fallback key (RFC 8032 test vector).
    let default_key_bytes = [...];
    EdVerifyingKey::from_bytes(&default_key_bytes)
        .map(|key| vec![key])
        .unwrap_or_default()
}

#[cfg(not(feature = "dev_insecure"))]
fn dev_mode_fallback_key() -> Vec<EdVerifyingKey> {
    Vec::new()  // Always empty in production builds
}
```

### 1.2 Domain Manager Error Handling

Replace `expect()` with `Result<T, E>` and proper error replies.

**File:** `services/domain_manager/src/error.rs` (new)
```rust
#[derive(Debug)]
pub enum DomainManagerError {
    PayloadSerialization(String),
    InvalidRequest(String),
    DomainNotFound(u64),
    InternalError(String),
}
```

**File:** `services/domain_manager/src/main.rs`
```rust
fn start_reply(...) -> Result<Envelope, DomainManagerError> {
    let mut env = Envelope::empty(PROTOCOL_DOMAIN_MANAGER_V1, MSG_START_DOMAIN_REPLY);
    write_payload(&mut env, &payload)
        .map_err(|e| DomainManagerError::PayloadSerialization(e.to_string()))?;
    Ok(env)
}
```

### 1.3 Capsule Relay Security

**Session ID:** Replace XOR with cryptographic random.
```rust
use rand::RngCore;

fn generate_session_id() -> u64 {
    rand::thread_rng().next_u64()
}
```

**Path Sanitization:** Validate paths don't escape allowed directory.
```rust
fn ensure_payload(path: &Path) -> Result<(), Box<dyn Error>> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let base = Path::new(env::var("CAPSULE_PAYLOAD_DIR")
        .unwrap_or_else(|_| "/tmp/capsule_payloads".to_string()));

    if !canonical.starts_with(base) {
        return Err("path traversal attempt blocked".into());
    }
    // ... existing logic
}
```

## Work Stream 2: Unsafe Block Documentation

All `unsafe` blocks receive `// SAFETY:` comments following RFC 2585 style.

### Documentation Pattern

```rust
// SAFETY: <what makes this safe>
// - <invariant 1>
// - <invariant 2>
unsafe { ... }
```

### Files Requiring Documentation

| File | Unsafe Blocks | Primary Safety Concerns |
|------|---------------|------------------------|
| `kernel/src/trace_ring.rs` | 17 | Static mut access, concurrent reader/writer |
| `kernel/src/init.rs` | 3 | Raw pointer dereferencing, physical memory |
| `kernel/src/boot.rs` | 4 | Static mut access, UEFI memory map |
| `kernel/src/arch/aarch64.rs` | 8 | MMIO register access |
| `kernel/src/arch/x86_64.rs` | 6 | Port I/O, CR3 manipulation |
| `kernel/src/arch/*/mmu.rs` | 20+ | Page table manipulation, TLB flushes |
| `kernel/src/mm/*.rs` | 15+ | Physical address construction |
| `kernel/src/shmem.rs` | 4 | Virtual address mapping |
| `kernel/src/ipc_v0.rs` | 2 | Domain page table access |

## Work Stream 3: Refactoring

### 3.1 IPC Error Reply Helper

**File:** `kernel/src/ipc_v0.rs`
```rust
fn shmem_error_reply(env: &Envelope, status: u32) -> Envelope {
    let reply_msg_type = env.msg_type + 1;
    let mut out = Envelope::empty(env.protocol, reply_msg_type);

    match env.msg_type {
        MSG_CREATE_REGION => {
            let reply = CreateRegionReply {
                request_id: 0, region_id: 0, shm_cap: 0,
                status, reserved: 0,
            };
            let _ = write_payload(&mut out, &reply);
        }
        MSG_MAP_REGION => { /* ... */ }
        MSG_UNMAP_REGION => { /* ... */ }
        MSG_CLOSE_REGION => { /* ... */ }
        _ => {}
    }
    out
}
```

### 3.2 GPU Manager Service Extraction

**New service structure:**
```
services/gpu_manager/
├── Cargo.toml
└── src/
    ├── main.rs          # Service entry point
    ├── gpu_control.rs   # GPU quarantine control logic
    └── error.rs         # Error types
```

**New IDL:** `idl/harness/gpu_manager_v1.toml`

**Migration:**
1. Create gpu_manager with IDL and generated bindings
2. Move GPU logic from domain_manager to gpu_manager
3. Domain manager calls gpu_manager via IPC
4. Update Foundry gates

## Work Stream 4: Code Quality

### 4.1 Named Constants

**File:** `kernel/src/mm/constants.rs` (new)
```rust
pub mod layout {
    pub const KERNEL_PAGE_TABLES_START: u64 = 0x1000;
    pub const DUMMY_PAGE_TABLE: u64 = 0x5000;
    pub const DOMAIN_PAGE_TABLES_START: u64 = 0x10000;
    pub const DOMAIN_PAGE_TABLE_SIZE: u64 = 0x1000;
    pub const TEST_REGION_BASE: u64 = 0x1_0000_0000;
}
```

### 4.2 Rustdoc

Priority modules:
- `kernel_api/src/lib.rs` - Module overview
- `kernel_api/src/cap.rs` - Capability semantics
- `kernel_api/src/ipc.rs` - IPC message format
- `kernel/src/cap_table.rs` - Capability table interface
- `kernel/src/mm/*.rs` - Memory type safety

### 4.3 Bump Allocator Comment

Update TODO to document intentional design decision and reference BitmapAllocator.

## Dependencies Added

| Crate | Dependency | Reason |
|-------|------------|--------|
| `capsule_relay` | `rand` | Cryptographic session IDs |

## Foundry Gates

| Gate | Change |
|------|--------|
| `foundry_gpu_quarantine_s7.sh` | Call gpu_manager instead of domain_manager |
| `foundry_domain_manager_s6.sh` | Remove GPU assertions |
| **NEW** `foundry_gpu_manager_s6.sh` | Test gpu_manager independently |
| `foundry_store_security.sh` | Test dev_insecure feature flag |
| `foundry_capsule_relay.sh` | Session ID randomness test |

## Implementation Order

```
Phase 1: Security Fixes (least risk, highest impact)
  ├── 1.1 Store service dev mode → compile-time gate
  ├── 1.2 Domain manager error handling
  └── 1.3 Capsule relay security (session + path)

Phase 2: Documentation (no functional changes)
  └── 2.1 Add SAFETY comments to all unsafe blocks

Phase 3: Refactoring (most complex)
  ├── 3.1 IPC error reply helper
  └── 3.2 GPU manager service extraction

Phase 4: Code Quality (polish)
  ├── 4.1 Named constants
  ├── 4.2 Rustdoc
  └── 4.3 Bump allocator comment
```

## Files Changed Summary

| Work Stream | Files Changed | New Files | Lines (est.) |
|-------------|---------------|-----------|--------------|
| Security | 6 | 1 | ~300 |
| Documentation | 15 | 0 | ~150 |
| Refactoring | 8 | 10+ | ~800 |
| Code Quality | 8 | 1 | ~100 |
| **Total** | **37** | **12+** | **~1,350** |
