# S10.1: Native Runner Production Integration

**Last Updated:** 2026-02-19
**Status:** Approved
**Related:** S10.0 Native Runner Phase 2, V-006 Security Remediation, V-012 Trace Service

---

## Executive Summary

This document specifies S10.1: wiring the native runner into the real OS control plane with manifest parsing, broker-based capability grants, real kernel IPC, and end-to-end Foundry validation.

**Key Deliverables:**
- `native_wasm_v0` manifest schema with fail-closed validation
- Capability broker API in Domain Manager (transactional grants)
- Runtime supervisor integration for native WASM execution
- Real kernel IPC replacing MockKernelBridge
- Foundry gate `foundry_native_runner_s10_1.sh`

**Design Principles:**
- Broker owns policy; runner is executor-only
- All grants are transactional (all-or-nothing)
- Fail-closed on missing/invalid capabilities
- Typed IDL for all IPC boundaries
- Wire compatibility: version bump, never mutate existing protocols
- No privilege escalation: broker derives policy from manifest, not caller

---

## 1. Manifest Schema (`native_wasm_v0`)

### 1.1 Schema Types

```rust
// artifact_store_schema/src/native_wasm.rs

/// Native WASM artifact manifest (v0) - uses composition
#[derive(Debug, Serialize, Deserialize)]
pub struct NativeWasmManifestV0 {
    #[serde(flatten)]
    pub manifest: Manifest,
    pub native_wasm: NativeWasmV0,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NativeWasmV0 {
    #[serde(default = "default_entrypoint")]
    pub entrypoint: String,
    pub required_capabilities: Vec<RequiredCapability>,
    #[serde(default)]
    pub declares_no_capabilities: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequiredCapability {
    pub export_name: String,    // e.g., "RAMEN_CAP_ECHO_V0"
    pub interface: String,      // e.g., "harness.echo_v0"
    pub rights: u64,            // Non-zero rights mask
    pub purpose: String,        // Audit trail
}
```

### 1.2 Validation Rules

1. **Export name format:** Must start with `RAMEN_CAP_`, followed by `[A-Z0-9_]+`
2. **Export name uniqueness:** No duplicates in `required_capabilities`
3. **Interface format:** Must match `<namespace>.<name>_v<N>` pattern
4. **Rights non-zero:** `rights == 0` is invalid
5. **Purpose required:** Non-empty string for audit trail
6. **Capless explicit:** Empty `required_capabilities` requires `declares_no_capabilities: true`
7. **Kind must match:** `manifest.kind == "native_wasm_v0"`

### 1.3 Example Manifest

```json
{
  "schema_version": 1,
  "content_id": "sha256:abc123...",
  "size_bytes": 4096,
  "kind": "native_wasm_v0",
  "channels": ["Experimental"],
  "native_wasm": {
    "entrypoint": "_start",
    "required_capabilities": [
      {
        "export_name": "RAMEN_CAP_ECHO_V0",
        "interface": "harness.echo_v0",
        "rights": 1,
        "purpose": "Send and receive echo messages"
      },
      {
        "export_name": "RAMEN_CAP_TRACE_V1",
        "interface": "harness.trace_v1",
        "rights": 3,
        "purpose": "Read and write execution traces"
      }
    ]
  },
  "signatures": ["sig1:..."]
}
```

---

## 2. Capability Broker (Domain Manager)

### 2.1 Broker Contract

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│    Manifest     │────▶│     Broker      │────▶│  granted_handles│
│ (required_caps) │     │ (policy check)  │     │ {name → handle} │
└─────────────────┘     └────────┬────────┘     └─────────────────┘
                                 │
                                 ▼
                        ┌─────────────────┐
                        │    Kernel       │
                        │ (grant caps)    │
                        └─────────────────┘
```

### 2.2 IDL Addition

**Channel Trust Model:** Caller does NOT provide channel. Broker fetches manifest from store by `content_id` and derives policy inputs from the artifact's declared `channels`. This prevents privilege escalation.

```toml
# idl/harness/domain_manager_v1.toml (additions)

[message.grant_capabilities]
# content_id_hash: 32-byte binary SHA256 hash (no "sha256:" prefix)
fields = [
    "request_id:u64",
    "domain_id:u64",
    "content_id_hash:bytes32",  # Binary hash, not hex string
]

[message.grant_capabilities_reply]
fields = [
    "request_id:u64",
    "domain_id:u64",
    "status:u32",
    "handle_count:u32",
    "reserved:u32",
    "reserved2:u32",
]

[message.revoke_capabilities]
fields = ["request_id:u64", "domain_id:u64"]

[message.revoke_capabilities_reply]
fields = ["request_id:u64", "domain_id:u64", "status:u32", "revoked_count:u32"]
```

### 2.3 Broker API

```rust
pub trait PolicyEngine {
    fn evaluate(
        &self,
        domain_id: u64,
        artifact_channels: &[String],
        cap: &RequiredCapability,
    ) -> Result<(), DenialReason>;
}

pub struct CapabilityBroker<P: PolicyEngine> {
    policy: P,
    store_client: StoreClient,
    active_grants: HashMap<u64, Vec<(String, u64)>>,
}

impl<P: PolicyEngine> CapabilityBroker<P> {
    /// Grant capabilities - TRANSACTIONAL (all-or-nothing)
    pub fn grant_capabilities(
        &mut self,
        content_id: &str,
        domain_id: u64,
    ) -> Result<GrantResult, BrokerError>;

    /// Revoke all capabilities for a domain
    pub fn revoke_domain(&mut self, domain_id: u64) -> Result<u32, BrokerError>;
}
```

### 2.4 Error Types

```rust
pub enum BrokerError {
    ManifestInvalid(String),
    InterfaceUnknown { interface: String },
    CapabilityDenied { export_name: String, reason: DenialReason },
    KernelGrantFailed { interface: String, status: u32 },
    StoreFetchFailed { content_id: String, error: String },
}

pub enum DenialReason {
    InterfaceNotAllowedForChannel { channel: String },
    RightsExceedMaximum { requested: u64, maximum: u64 },
    PolicyExplicitDeny,
}
```

### 2.5 Transactional Semantics

**Critical:** If any capability is denied or kernel grant fails:
1. **Revoke in kernel:** Call kernel IPC to revoke each previously granted handle
2. **Clear local state:** Remove domain from `active_grants`
3. Return error to caller
4. No partial capability leaks

This ensures "all-or-nothing" semantics at the kernel level, not just in broker memory.

---

## 3. Runtime Supervisor Integration

### 3.1 Launch Plan Extension

```rust
#[derive(Debug, Deserialize)]
struct LaunchPlan {
    program_id: String,
    runner: String,
    artifact_ref: String,
    // ... existing fields ...

    #[serde(default)]
    native_wasm: Option<NativeWasmConfig>,
}

#[derive(Debug, Deserialize)]
struct NativeWasmConfig {
    kernel_ipc: String,        // Default: /run/ramen/kernel.sock
    domain_id: u64,
    timeout_ms: u64,           // Default: 0 (no limit)
    trace_output: Option<PathBuf>,
}
```

### 3.2 Execution Flow

```
1. LOAD PLAN
   └── runner: "native_wasm_v0"

2. FETCH ARTIFACT
   └── StoreClient.get_artifact_bytes(artifact_ref)
   └── Signature verification

3. BROKER GRANT
   └── DomainManager.grant_capabilities(content_id, domain_id)
   └── Returns: {export_name → handle}

4. EXECUTE
   └── NativeRunner.load_and_run(wasm_bytes, granted_handles)
   └── Host functions use kernel IPC

5. COLLECT RESULT
   └── exit_code, stdout, trace_artifacts
```

### 3.3 Example Launch Plan

```json
{
  "program_id": "org.ramen.hello_wasm",
  "runner": "native_wasm_v0",
  "artifact_ref": "sha256:abc123...",
  "native_wasm": {
    "kernel_ipc": "/run/ramen/kernel.sock",
    "domain_id": 100,
    "timeout_ms": 30000,
    "trace_output": "out/trace/hello_wasm.json"
  }
}
```

---

## 4. Real Kernel IPC

### 4.1 KernelBridge Trait

```rust
pub trait KernelBridgeOps {
    fn echo_request(&self, cap_handle: u64, request_id: u64, payload: &[u8]) -> Result<Vec<u8>, Status>;
    fn echo_reply(&self, cap_handle: u64, request_id: u64, status: u32, payload: &[u8]) -> Result<(), Status>;
    fn trace_read(&self, cap_handle: u64, offset: u64, max_len: usize) -> Result<Vec<u8>, Status>;
    fn trace_write(&self, cap_handle: u64, data: &[u8]) -> Result<(), Status>;
}
```

### 4.2 New Capability-Based Harness Protocols

**Wire Compatibility:** Do NOT mutate existing `echo_harness_v0` or `trace_service_v1`. Instead, create new versions with capability handles. Existing protocols remain unchanged for backward compatibility.

```toml
# idl/harness/echo_harness_v1.toml (NEW - capability-based)

namespace = "harness.echo"
version = "1"

[message.echo_request]
fields = ["cap_handle:u64", "request_id:u64", "payload_len:u32", "reserved:u32"]

[message.echo_request_reply]
fields = ["cap_handle:u64", "request_id:u64", "payload_len:u32", "status:u32"]

[message.echo_reply]
fields = ["cap_handle:u64", "request_id:u64", "status:u32", "payload_len:u32"]

[message.echo_reply_reply]
fields = ["cap_handle:u64", "request_id:u64", "status:u32", "reserved:u32"]
```

```toml
# idl/harness/trace_service_v2.toml (NEW - capability-based)

namespace = "harness.trace"
version = "2"

[message.trace_read]
fields = ["cap_handle:u64", "offset:u64", "max_len:u32", "reserved:u32"]

[message.trace_read_reply]
fields = ["cap_handle:u64", "data_len:u32", "status:u32", "reserved:u32"]

[message.trace_write]
fields = ["cap_handle:u64", "data_len:u32", "reserved:u32", "reserved2:u32"]

[message.trace_write_reply]
fields = ["cap_handle:u64", "status:u32", "reserved:u32", "reserved2:u32"]
```

**Manifest Interface Names:** Native WASM manifests declare `harness.echo_v1` and `harness.trace_v2` for the new capability-based protocols.

### 4.3 IPC Pattern

**Envelope Size:** Use `kernel_api::ipc::ENVELOPE_SIZE` constant, not a magic number. This ensures wire format correctness.

```rust
use kernel_api::ipc::ENVELOPE_SIZE;

impl KernelBridge {
    fn transact(&self, request: Envelope) -> Result<Envelope, RunnerError> {
        let mut stream = UnixStream::connect(&self.socket_path)?;
        stream.write_all(request.as_bytes())?;

        let mut reply_bytes = [0u8; ENVELOPE_SIZE];
        stream.read_exact(&mut reply_bytes)?;

        Envelope::from_bytes(&reply_bytes)
    }
}
```

**Inline Payload Limit:** Envelope payload space is limited. For payloads exceeding this limit, fail with `Status::PayloadTooLarge` (shared memory support is S10.2+).

### 4.4 Payload Transport

| Payload Size | Transport |
|--------------|-----------|
| ≤56 bytes | Inline in envelope |
| >56 bytes | Shared memory region (capability-backed) |

---

## 5. Foundry Gate

### 5.1 Gate Location

`tools/ci/foundry_native_runner_s10_1.sh`

### 5.2 Assertion Summary

| Phase | Count | Coverage |
|-------|-------|----------|
| 1. Manifest Schema | 5 | Format, uniqueness, fail-closed |
| 2. Broker Integration | 4 | Grants, denials, transactional |
| 3. Runtime Supervisor | 2 | Dispatch, config validation |
| 4. End-to-End | 5 | Build, store, execute, inject, IPC |
| 5. Negative Tests | 3 | Fail-closed, error handling |
| **Total** | **19** | |

### 5.3 Key Assertions

- **1.1** Valid manifest parses
- **1.2** Invalid export_name rejected
- **1.3** Empty capabilities without flag rejected
- **2.4** Transactional grant rollback
- **4.4** Capability injection succeeds
- **5.1** Missing capability fails closed

### 5.4 CI vs E2E Split

**CI-Safe Assertions (run in every CI):**
- Schema validation tests (1.1-1.5)
- Broker unit tests (2.1-2.4)
- Supervisor dispatch tests (3.1-3.2)
- All negative tests (5.1-5.3)

**E2E Assertions (require services, skip in CI via env var):**
- Store service integration (4.2)
- Full execution path (4.3)
- Real kernel IPC (4.5)

Gate script should check `SKIP_E2E_ASSERTIONS=1` env var to skip E2E tests in CI.

---

## 6. Implementation Checklist

### Phase S10.1 (This Document)

- [ ] Create `artifact_store_schema/src/native_wasm.rs`
- [ ] Add `validate_native_wasm_manifest()` function
- [ ] Update IDL: `domain_manager_v1.toml` (grant/revoke messages)
- [ ] Update IDL: `echo_harness_v0.toml` (cap_handle field)
- [ ] Update IDL: `trace_service_v1.toml` (cap_handle field)
- [ ] Run `just codegen` to generate bindings
- [ ] Create `services/domain_manager/src/broker.rs`
- [ ] Implement `CapabilityBroker` with transactional grants
- [ ] Implement `PolicyEngine` trait + `ChannelAllowlistPolicy`
- [ ] Create `runtime_supervisor/src/native_wasm_runner.rs`
- [ ] Add `native_wasm_v0` case to runner dispatch
- [ ] Implement `KernelBridge` with real IPC
- [ ] Add `KernelBridgeOps` trait for mocking
- [ ] Create `tools/ci/foundry_native_runner_s10_1.sh`
- [ ] Add unit tests for manifest validation
- [ ] Add unit tests for broker logic
- [ ] Add integration tests for supervisor path
- [ ] Wire into justfile: `just foundry-native-runner-s10-1`
- [ ] Update `CURRENT_STATUS.md`

---

## 7. Dependencies

| Component | Dependency | Status |
|-----------|------------|--------|
| Manifest Schema | `artifact_store_schema` | Exists |
| Broker | `store_service` client | Exists |
| Broker | `kernel_api` IDL bindings | Exists |
| Supervisor | `native_runner` (S10.0) | Complete |
| IPC | `kernel_api::ipc::Envelope` | Exists |
| IPC | `kernel_api::wire` | Exists |

---

## 8. Security Considerations

### 8.1 Fail-Closed Guarantees

1. **Manifest validation:** Invalid manifests rejected at schema level
2. **Broker grants:** Unknown interfaces denied, not ignored
3. **Capability injection:** Missing capability causes load failure
4. **Transactional grants:** No partial capability leaks on error

### 8.2 Boundary Discipline

| Boundary | Protocol |
|----------|----------|
| Supervisor → Store | IPC (store_service_v1) |
| Supervisor → Broker | IPC (domain_manager_v1) |
| Supervisor → Runner | Library call |
| Runner → Kernel | IPC (harness protocols) |

### 8.3 No Raw JSON Over IPC

The broker fetches manifests from store by `content_id`, never accepts raw JSON from callers. This prevents privilege escalation via crafted manifests.

---

## 9. Future Work (Post-S10.1)

- Per-domain policy files
- Signed policy bundles
- Domain class (quarantine vs trusted)
- Shared memory for large payloads
- Capability revocation on domain crash
- Observability integration

---

**Document Version:** 1.0
**Last Updated:** 2026-02-19
**Status:** Approved
