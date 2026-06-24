# Services Boundary Fix Design

**Last Updated:** 2026-02-17
**Status:** Approved
**Related:** S9.0 Security Remediation, V-007 Phase 3

## Summary

Address code review findings by:
1. Fixing minor code quality issues in kernel
2. Migrating `portals` and `capsule_relay` services to use store service IPC

## Issues Addressed

### Critical: Architectural Boundary Violations
- `services/portals` uses direct IO from `artifact_store_core`
- `services/capsule_relay` uses direct IO from `artifact_store_core`

### Important: Code Quality
- Duplicate size check in `kernel/src/shmem.rs:133-135`
- Magic number `16` instead of `MAX_DOMAINS` in MMU code

## Design

### Part 1: Quick Fixes

#### 1.1 Remove Duplicate Size Check

File: `kernel/src/shmem.rs`
- Lines 123-124: `if size_bytes == 0 { return Err(STATUS_INVALID_SIZE); }`
- Lines 133-135: Same check (dead code) - **REMOVE**

#### 1.2 Use MAX_DOMAINS Constant

Files: `kernel/src/arch/x86_64/mmu.rs`, `kernel/src/arch/aarch64/mmu.rs`
- Replace `if domain_id >= 16` with `if domain_id >= MAX_DOMAINS as u64`
- Import: `use crate::domain_registry::MAX_DOMAINS;`

### Part 2: Services IPC Migration

#### 2.1 Pattern (from domain_manager refactoring)

```rust
// Cargo.toml
store_service = { path = "../store_service" }

// main.rs
use store_service::{StoreClient, StoreClientError};
use store_service::capability::{StoreCapability, STORE_RIGHT_READ, STORE_RIGHT_WRITE};

// Connect with capability
let cap = StoreCapability::new(domain_id, STORE_RIGHT_READ | STORE_RIGHT_WRITE, 0);
let mut client = StoreClient::connect_with_capability(socket_path, domain_id, Some(cap))?;

// Ingest artifact via IPC
let reply = client.ingest_artifact(kind, channel, &src_path)?;
let content_id = reply.content_id;
```

#### 2.2 Portal Service Changes

**Cargo.toml:**
- Remove: `artifact_store_core`
- Add: `store_service`

**main.rs:**
- Add `--store-socket-path` argument
- Create `StoreClient` at startup
- Refactor `ingest_file()` to use `client.ingest_artifact()`
- Remove direct IO imports

#### 2.3 Capsule Relay Service Changes

**Same pattern as portals:**
- Remove: `artifact_store_core`
- Add: `store_service`
- Add socket path argument
- Refactor `ingest_file()` to use IPC

#### 2.4 Capability Strategy

Both services need `STORE_RIGHT_READ | STORE_RIGHT_WRITE` for full artifact access.

Default domain assignments:
- Portals service: Domain 1
- Capsule relay: Domain 2

## Implementation Order

1. Fix duplicate check in `shmem.rs`
2. Fix magic number in MMU code (both architectures)
3. Refactor `portals` service
4. Refactor `capsule_relay` service
5. Run foundry gates to validate

## Testing

- Run `just foundry-s0` (boot validation)
- Run `just foundry-portal-file-ro-s3` (portal service)
- Run existing capsule relay tests
- Verify no regression in artifact operations

## Risks

1. **Store service availability:** Services now require store service running
   - Mitigation: Clear error messages if store service unavailable

2. **Performance:** IPC overhead for artifact operations
   - Mitigation: Acceptable for control-plane operations; data plane uses shared memory

## Success Criteria

- [ ] No direct IO imports in portals/capsule_relay
- [ ] All artifact operations go through store service IPC
- [ ] Foundry gates pass
- [ ] Capability validation enforced at store service boundary
