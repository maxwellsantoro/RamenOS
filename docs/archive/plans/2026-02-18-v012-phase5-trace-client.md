# V-012 Phase 5: User-Space Trace Service Client

**Last Updated:** 2026-02-18
**Status:** PLANNED
**Depends On:** V-012 Phase 4 (Kernel-side trace service - COMPLETE)

## Overview

Implement a user-space trace service client library and integrate it with the domain manager for per-domain trace collection and artifact emission.

## Goals

1. Provide a Rust library for services to access kernel trace buffers
2. Enable domain manager to collect traces from domains it manages
3. Emit trace artifacts on domain lifecycle events (shutdown, crash)
4. Maintain capability-based access control for trace operations

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    User Space                               │
│  ┌─────────────────┐     ┌─────────────────────────────┐   │
│  │ Domain Manager  │────▶│ Trace Client Library        │   │
│  │                 │     │ (trace_client crate)        │   │
│  └────────┬────────┘     │  - TraceClient struct       │   │
│           │              │  - Domain-scoped reads      │   │
│           │              │  - Capability validation    │   │
│           ▼              └──────────────┬──────────────┘   │
│  ┌─────────────────┐                    │                  │
│  │ Trace Artifacts │◀───────────────────┘                  │
│  │ (store ingest)  │                                       │
│  └─────────────────┘                                       │
└─────────────────────────────────────────────────────────────┘
                         │
                         │ IPC (trace_service_v1 protocol)
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                      Kernel                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Trace Service (Phase 4 - COMPLETE)                  │   │
│  │  - Per-domain trace buffers                         │   │
│  │  - Capability table for trace access                │   │
│  │  - IPC handlers for trace_service_v1                │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Deliverables

### 1. Trace Client Library (`services/trace_client/`)

New crate providing trace access for user-space services.

**Files:**
- `services/trace_client/Cargo.toml`
- `services/trace_client/src/lib.rs` - Public API
- `services/trace_client/src/client.rs` - TraceClient implementation
- `services/trace_client/src/error.rs` - Error types
- `services/trace_client/src/capability.rs` - Trace capability handling

**Key Types:**

```rust
/// Trace client for accessing kernel trace buffers
pub struct TraceClient {
    /// IPC connection to kernel trace service
    ipc_client: IpcClient,
    /// Domain ID this client is bound to
    domain_id: u64,
    /// Capability handle for trace access
    trace_cap: Handle,
}

/// Builder for configuring trace client
pub struct TraceClientBuilder {
    domain_id: Option<u64>,
    trace_cap: Option<Handle>,
}

/// Errors from trace operations
#[derive(Debug, thiserror::Error)]
pub enum TraceClientError {
    #[error("IPC error: {0}")]
    IpcError(#[from] IpcError),
    #[error("Capability denied for domain {domain_id}")]
    CapabilityDenied { domain_id: u64 },
    #[error("Invalid trace handle")]
    InvalidHandle,
    #[error("Trace buffer not found")]
    BufferNotFound,
}
```

**Key Methods:**

```rust
impl TraceClient {
    /// Connect to trace service for a specific domain
    pub fn connect(domain_id: u64) -> Result<Self, TraceClientError>;

    /// Read trace data from domain's buffer
    pub fn read_trace(&mut self, buf: &mut [u8]) -> Result<usize, TraceClientError>;

    /// Get trace buffer metadata
    pub fn get_info(&self) -> Result<TraceInfo, TraceClientError>;

    /// Create a trace capability for another domain
    pub fn create_capability(&self, target_domain: u64) -> Result<Handle, TraceClientError>;

    /// Drain all available trace data
    pub fn drain(&mut self) -> Result<Vec<u8>, TraceClientError>;
}
```

### 2. Domain Manager Integration

**Files:**
- `services/domain_manager/src/trace_integration.rs` (new)
- `services/domain_manager/src/domain_lifecycle.rs` (modify)

**Integration Points:**

1. **Domain Creation:**
   - Create trace buffer for new domain via trace service
   - Store trace capability in domain metadata

2. **Domain Runtime:**
   - Periodically drain trace data (configurable interval)
   - Buffer trace data for artifact emission

3. **Domain Shutdown:**
   - Final trace drain before cleanup
   - Emit trace artifact to store
   - Destroy trace buffer via trace service

**Trace Artifact Schema:**

```toml
# idl/artifacts/trace_artifact_v0.toml
namespace = "artifact.trace"
version = "0"

[message.header]
fields = ["domain_id:u64", "start_time:u64", "end_time:u64", "event_count:u64"]

[message.event]
fields = ["timestamp:u64", "domain_id:u64", "event_type:u32", "payload_len:u32"]
```

### 3. Foundry Gate

**File:** `tools/ci/foundry_v012_phase5_trace_client.sh`

**Test Cases:**

1. **Trace Client Creation**
   - `trace_client_connects_to_kernel_service`
   - `trace_client_rejects_invalid_domain`

2. **Trace Reading**
   - `trace_read_returns_data_from_buffer`
   - `trace_read_returns_empty_when_no_data`
   - `trace_drain_collects_all_data`

3. **Capability Validation**
   - `trace_access_denied_without_capability`
   - `trace_capability_scoped_to_domain`

4. **Domain Manager Integration**
   - `domain_manager_creates_trace_buffer`
   - `domain_manager_emits_trace_artifact_on_shutdown`
   - `trace_artifact_contains_domain_events`

5. **Negative Assertions**
   - `cross_domain_trace_access_denied`
   - `trace_read_after_buffer_destroy_fails`

## Implementation Sequence

### Phase 5.1: Trace Client Library Scaffold (3 tasks)

1. **Create `trace_client` crate structure**
   - `Cargo.toml` with dependencies: `kernel_api`, `thiserror`, `bincode`
   - `src/lib.rs` with module declarations
   - `src/error.rs` with `TraceClientError` enum

2. **Implement `TraceClientBuilder` and basic `TraceClient`**
   - Builder pattern for configuration
   - IPC connection setup
   - Basic `connect()` method

3. **Add unit tests for client construction**
   - Mock IPC layer for testing
   - Test error conditions

### Phase 5.2: Trace Operations (3 tasks)

4. **Implement `read_trace()` and `drain()`**
   - Use `read_trace` IPC from `trace_service_v1`
   - Handle buffer wraparound
   - Efficient chunked reading

5. **Implement `get_info()`**
   - Use `get_trace_info` IPC
   - Return `TraceInfo` struct

6. **Add unit tests for trace operations**
   - Mock kernel responses
   - Test edge cases (empty buffer, full buffer)

### Phase 5.3: Domain Manager Integration (3 tasks)

7. **Create `trace_integration.rs` module**
   - `DomainTraceManager` struct
   - Integration with domain lifecycle

8. **Implement trace collection on domain shutdown**
   - Final trace drain
   - Artifact emission to store

9. **Add integration tests**
   - Full lifecycle test with trace collection
   - Artifact validation

### Phase 5.4: Foundry Gate (2 tasks)

10. **Create `foundry_v012_phase5_trace_client.sh`**
    - Test trace client library
    - Test domain manager integration
    - Negative assertions

11. **Wire into CI umbrella gate**
    - Add to `foundry_all_s0_s1_s2_s3_s4_s5_s6_s7_s8.sh`
    - Update CI workflow

## Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| `kernel_api` | workspace | Handle, Envelope types |
| `artifact_store_schema` | workspace | Trace artifact schema |
| `thiserror` | ^1.0 | Error derive macro |
| `bincode` | ^1.3 | IPC serialization |

## Security Considerations

1. **Capability Validation:**
   - Every trace operation validates the capability handle
   - Capabilities are scoped to specific domain IDs
   - Stale capabilities are rejected (generation counter)

2. **Domain Isolation:**
   - A domain can only read its own traces (or kernel traces with admin cap)
   - Cross-domain trace access requires explicit capability grant

3. **Audit Logging:**
   - All trace operations are logged with domain_id and operation type
   - Failed capability checks are logged as security events

## Success Criteria

- [ ] `trace_client` crate compiles and passes unit tests
- [ ] Domain manager successfully collects traces from managed domains
- [ ] Trace artifacts are emitted to store on domain shutdown
- [ ] Foundry gate passes with all 11+ test cases
- [ ] Cross-domain trace access is denied
- [ ] All trace operations are audit-logged

## Future Work (Post-Phase 5)

- **Phase 6:** Real-time trace streaming (ring buffer subscription)
- **Phase 7:** Trace aggregation across multiple domains
- **Phase 8:** Trace-based debugging tools (trace replay, diff)

---

## Notes on Native Runner (Future)

User indicated preference for **WASM** as the native executable format. This will be addressed in a future plan:

- WASM provides natural sandboxing
- Good tooling support (wasmtime, wasmer)
- Portable across architectures
- Can compile Rust/C/C++ to WASM with existing toolchains

Design document to be created: `docs/plans/native_runner_wasm_design.md`
