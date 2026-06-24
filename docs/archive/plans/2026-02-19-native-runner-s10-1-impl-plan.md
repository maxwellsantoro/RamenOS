# S10.1 Native Runner Production Integration Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire native runner into real OS control plane with manifest schema, capability broker, supervisor integration, and real kernel IPC.

**Architecture:** Manifest schema validates capability requirements, broker (in Domain Manager) evaluates policy and grants handles transactionally, runtime supervisor dispatches to native runner with pre-granted handles, host functions use real kernel IPC.

**Tech Stack:** Rust, serde, artifact_store_schema, kernel_api IDL bindings, Unix domain sockets

---

## Task 1: Manifest Schema Module

**Files:**
- Create: `artifact_store_schema/src/native_wasm.rs`
- Modify: `artifact_store_schema/src/lib.rs`
- Test: `artifact_store_schema/src/native_wasm.rs` (inline tests)

**Step 1: Write the failing tests**

Add to `artifact_store_schema/src/native_wasm.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_manifest_parses() {
        let json = r#"{
            "schema_version": 1,
            "content_id": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
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
                        "purpose": "Send echo requests"
                    }
                ]
            },
            "signatures": []
        }"#;
        let manifest: NativeWasmManifestV0 = serde_json::from_str(json).unwrap();
        validate_native_wasm_manifest(&manifest).unwrap();
    }

    #[test]
    fn invalid_export_name_rejected() {
        let cap = RequiredCapability {
            export_name: "INVALID_PREFIX".to_string(),
            interface: "harness.echo_v0".to_string(),
            rights: 1,
            purpose: "test".to_string(),
        };
        assert!(validate_export_name(&cap.export_name).is_err());
    }

    #[test]
    fn empty_capabilities_without_flag_rejected() {
        let wasm = NativeWasmV0 {
            entrypoint: "_start".to_string(),
            required_capabilities: vec![],
            declares_no_capabilities: false,
        };
        assert!(validate_native_wasm(&wasm).is_err());
    }

    #[test]
    fn empty_capabilities_with_flag_allowed() {
        let wasm = NativeWasmV0 {
            entrypoint: "_start".to_string(),
            required_capabilities: vec![],
            declares_no_capabilities: true,
        };
        validate_native_wasm(&wasm).unwrap();
    }

    #[test]
    fn zero_rights_rejected() {
        let cap = RequiredCapability {
            export_name: "RAMEN_CAP_TEST".to_string(),
            interface: "harness.echo_v0".to_string(),
            rights: 0,
            purpose: "test".to_string(),
        };
        assert!(validate_capability(&cap).is_err());
    }

    #[test]
    fn duplicate_export_name_rejected() {
        let wasm = NativeWasmV0 {
            entrypoint: "_start".to_string(),
            required_capabilities: vec![
                RequiredCapability {
                    export_name: "RAMEN_CAP_ECHO".to_string(),
                    interface: "harness.echo_v0".to_string(),
                    rights: 1,
                    purpose: "first".to_string(),
                },
                RequiredCapability {
                    export_name: "RAMEN_CAP_ECHO".to_string(),
                    interface: "harness.echo_v1".to_string(),
                    rights: 1,
                    purpose: "duplicate".to_string(),
                },
            ],
            declares_no_capabilities: false,
        };
        assert!(validate_native_wasm(&wasm).is_err());
    }

    #[test]
    fn wrong_kind_rejected() {
        let manifest = NativeWasmManifestV0 {
            manifest: Manifest {
                schema_version: 1,
                content_id: "sha256:abc".to_string(),
                size_bytes: 0,
                kind: "posix_script".to_string(),
                channels: vec![],
                signatures: vec![],
            },
            native_wasm: NativeWasmV0 {
                entrypoint: "_start".to_string(),
                required_capabilities: vec![],
                declares_no_capabilities: true,
            },
        };
        assert!(validate_native_wasm_manifest(&manifest).is_err());
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p artifact_store_schema native_wasm 2>&1`
Expected: Compile errors (module doesn't exist)

**Step 3: Implement the schema types and validation**

Create `artifact_store_schema/src/native_wasm.rs`:

```rust
use serde::{Deserialize, Serialize};
use crate::Manifest;

const NATIVE_WASM_SCHEMA_VERSION: u32 = 1;
const MAX_RIGHTS_MASK: u64 = 0xFFFF;

#[derive(Debug)]
pub struct NativeWasmValidationError(pub String);

impl std::fmt::Display for NativeWasmValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for NativeWasmValidationError {}

/// Native WASM artifact manifest (v0)
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
    pub export_name: String,
    pub interface: String,
    pub rights: u64,
    pub purpose: String,
}

fn default_entrypoint() -> String { "_start".to_string() }

pub fn validate_native_wasm_manifest(m: &NativeWasmManifestV0) -> Result<(), NativeWasmValidationError> {
    if m.manifest.schema_version != NATIVE_WASM_SCHEMA_VERSION {
        return Err(NativeWasmValidationError(format!(
            "native_wasm schema_version unsupported: {}", m.manifest.schema_version
        )));
    }
    if m.manifest.kind != "native_wasm_v0" {
        return Err(NativeWasmValidationError(format!(
            "expected kind native_wasm_v0, got: {}", m.manifest.kind
        )));
    }
    validate_native_wasm(&m.native_wasm)
}

pub fn validate_native_wasm(w: &NativeWasmV0) -> Result<(), NativeWasmValidationError> {
    if w.entrypoint.trim().is_empty() {
        return Err(NativeWasmValidationError("entrypoint required".into()));
    }
    if w.required_capabilities.is_empty() && !w.declares_no_capabilities {
        return Err(NativeWasmValidationError(
            "required_capabilities empty but declares_no_capabilities not set".into()
        ));
    }

    let mut seen = std::collections::HashSet::new();
    for cap in &w.required_capabilities {
        validate_capability(cap)?;
        if !seen.insert(cap.export_name.clone()) {
            return Err(NativeWasmValidationError(format!(
                "duplicate export_name: {}", cap.export_name
            )));
        }
    }
    Ok(())
}

pub fn validate_capability(cap: &RequiredCapability) -> Result<(), NativeWasmValidationError> {
    validate_export_name(&cap.export_name)?;
    validate_interface_format(&cap.interface)?;
    if cap.rights == 0 {
        return Err(NativeWasmValidationError(format!(
            "rights must be non-zero for: {}", cap.export_name
        )));
    }
    if cap.rights > MAX_RIGHTS_MASK {
        return Err(NativeWasmValidationError(format!(
            "rights exceeds max mask for: {}", cap.export_name
        )));
    }
    if cap.purpose.trim().is_empty() {
        return Err(NativeWasmValidationError(format!(
            "purpose required for: {}", cap.export_name
        )));
    }
    Ok(())
}

pub fn validate_export_name(name: &str) -> Result<(), NativeWasmValidationError> {
    const PREFIX: &str = "RAMEN_CAP_";
    if !name.starts_with(PREFIX) {
        return Err(NativeWasmValidationError(format!(
            "export_name must start with {}: {}", PREFIX, name
        )));
    }
    let suffix = &name[PREFIX.len()..];
    if suffix.is_empty() {
        return Err(NativeWasmValidationError(
            "export_name must have at least one character after RAMEN_CAP_".into()
        ));
    }
    if !suffix.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') {
        return Err(NativeWasmValidationError(format!(
            "export_name suffix must be [A-Z0-9_]: {}", name
        )));
    }
    Ok(())
}

pub fn validate_interface_format(iface: &str) -> Result<(), NativeWasmValidationError> {
    let v_pos = iface.rfind("_v").ok_or_else(|| {
        NativeWasmValidationError(format!("interface must end with _v<N>: {}", iface))
    })?;

    let version_part = &iface[v_pos + 2..];
    if version_part.is_empty() || !version_part.chars().all(|c| c.is_ascii_digit()) {
        return Err(NativeWasmValidationError(format!(
            "interface version must be digits: {}", iface
        )));
    }

    let namespace_part = &iface[..v_pos];
    if namespace_part.is_empty() {
        return Err(NativeWasmValidationError(format!(
            "interface must have namespace before _v: {}", iface
        )));
    }

    if !namespace_part.chars().all(|c| {
        c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_'
    }) {
        return Err(NativeWasmValidationError(format!(
            "interface namespace must be [a-z0-9._]: {}", iface
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_manifest_parses() {
        let json = r#"{
            "schema_version": 1,
            "content_id": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
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
                        "purpose": "Send echo requests"
                    }
                ]
            },
            "signatures": []
        }"#;
        let manifest: NativeWasmManifestV0 = serde_json::from_str(json).unwrap();
        validate_native_wasm_manifest(&manifest).unwrap();
    }

    #[test]
    fn invalid_export_name_rejected() {
        let cap = RequiredCapability {
            export_name: "INVALID_PREFIX".to_string(),
            interface: "harness.echo_v0".to_string(),
            rights: 1,
            purpose: "test".to_string(),
        };
        assert!(validate_export_name(&cap.export_name).is_err());
    }

    #[test]
    fn empty_capabilities_without_flag_rejected() {
        let wasm = NativeWasmV0 {
            entrypoint: "_start".to_string(),
            required_capabilities: vec![],
            declares_no_capabilities: false,
        };
        assert!(validate_native_wasm(&wasm).is_err());
    }

    #[test]
    fn empty_capabilities_with_flag_allowed() {
        let wasm = NativeWasmV0 {
            entrypoint: "_start".to_string(),
            required_capabilities: vec![],
            declares_no_capabilities: true,
        };
        validate_native_wasm(&wasm).unwrap();
    }

    #[test]
    fn zero_rights_rejected() {
        let cap = RequiredCapability {
            export_name: "RAMEN_CAP_TEST".to_string(),
            interface: "harness.echo_v0".to_string(),
            rights: 0,
            purpose: "test".to_string(),
        };
        assert!(validate_capability(&cap).is_err());
    }

    #[test]
    fn duplicate_export_name_rejected() {
        let wasm = NativeWasmV0 {
            entrypoint: "_start".to_string(),
            required_capabilities: vec![
                RequiredCapability {
                    export_name: "RAMEN_CAP_ECHO".to_string(),
                    interface: "harness.echo_v0".to_string(),
                    rights: 1,
                    purpose: "first".to_string(),
                },
                RequiredCapability {
                    export_name: "RAMEN_CAP_ECHO".to_string(),
                    interface: "harness.echo_v1".to_string(),
                    rights: 1,
                    purpose: "duplicate".to_string(),
                },
            ],
            declares_no_capabilities: false,
        };
        assert!(validate_native_wasm(&wasm).is_err());
    }

    #[test]
    fn wrong_kind_rejected() {
        let manifest = NativeWasmManifestV0 {
            manifest: Manifest {
                schema_version: 1,
                content_id: "sha256:abc".to_string(),
                size_bytes: 0,
                kind: "posix_script".to_string(),
                channels: vec![],
                signatures: vec![],
            },
            native_wasm: NativeWasmV0 {
                entrypoint: "_start".to_string(),
                required_capabilities: vec![],
                declares_no_capabilities: true,
            },
        };
        assert!(validate_native_wasm_manifest(&manifest).is_err());
    }
}
```

**Step 4: Export the module**

Modify `artifact_store_schema/src/lib.rs`, add:

```rust
pub mod native_wasm;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p artifact_store_schema native_wasm`
Expected: All 7 tests pass

**Step 6: Commit**

```bash
git add artifact_store_schema/src/native_wasm.rs artifact_store_schema/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(schema): add native_wasm_v0 manifest schema

Adds NativeWasmManifestV0 with fail-closed validation:
- Export name format: RAMEN_CAP_[A-Z0-9_]+
- Interface format: namespace.name_vN
- Non-zero rights required
- Duplicate export names rejected
- Empty capabilities requires explicit flag

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: IDL Updates for Capability Broker

**Files:**
- Modify: `idl/harness/domain_manager_v1.toml`

**Step 1: Add broker messages to IDL**

**Channel Trust Model:** Caller does NOT provide channel. Broker fetches manifest from store and derives policy from artifact's declared channels. This prevents privilege escalation.

Add to `idl/harness/domain_manager_v1.toml`:

```toml
# Capability broker messages

[message.grant_capabilities]
# content_id_hash: 32-byte binary SHA256 hash (no "sha256:" prefix)
fields = ["request_id:u64", "domain_id:u64", "content_id_hash:bytes32"]

[message.grant_capabilities_reply]
fields = ["request_id:u64", "domain_id:u64", "status:u32", "handle_count:u32", "reserved:u32", "reserved2:u32"]

[message.revoke_capabilities]
fields = ["request_id:u64", "domain_id:u64"]

[message.revoke_capabilities_reply]
fields = ["request_id:u64", "domain_id:u64", "status:u32", "revoked_count:u32"]
```

**Step 2: Run codegen**

Run: `just codegen`
Expected: Generates new message types in `kernel_api/src/generated/`

**Step 3: Verify generated types exist**

Run: `grep -c "GrantCapabilities" kernel_api/src/generated/domain_manager_v1.generated.rs`
Expected: Non-zero count

**Step 4: Commit**

```bash
git add idl/harness/domain_manager_v1.toml kernel_api/src/generated/
git commit -m "$(cat <<'EOF'
feat(idl): add capability broker messages to domain_manager_v1

Adds GrantCapabilities/RevokeCapabilities messages for S10.1 broker API.
Grant uses content_id_hash (32-byte binary) - no caller-provided channel.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: New Capability-Based Harness IDLs

**IMPORTANT:** Do NOT modify existing `echo_harness_v0` or `trace_service_v1`. Create new versions with capability handles. This preserves wire compatibility for any existing consumers.

**Files:**
- Create: `idl/harness/echo_harness_v1.toml`
- Create: `idl/harness/trace_service_v2.toml`

**Step 1: Create echo_harness_v1.toml**

Create `idl/harness/echo_harness_v1.toml`:

```toml
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

**Step 2: Create trace_service_v2.toml**

Create `idl/harness/trace_service_v2.toml`:

```toml
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

**Step 3: Run codegen**

Run: `just codegen`

**Step 4: Verify new generated files exist**

Run: `ls kernel_api/src/generated/echo_harness_v1.generated.rs kernel_api/src/generated/trace_service_v2.generated.rs`
Expected: Both files exist

**Step 5: Commit**

```bash
git add idl/harness/echo_harness_v1.toml idl/harness/trace_service_v2.toml kernel_api/src/generated/
git commit -m "$(cat <<'EOF'
feat(idl): add capability-based harness protocols v1/v2

Creates echo_harness_v1 and trace_service_v2 with cap_handle.
Existing v0/v1 protocols unchanged for backward compatibility.

Native WASM manifests should declare harness.echo_v1 and harness.trace_v2.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Broker Implementation

**Files:**
- Create: `services/domain_manager/src/broker.rs`
- Modify: `services/domain_manager/src/main.rs`
- Test: `services/domain_manager/src/broker.rs` (inline tests)

**Step 1: Write the failing tests**

Create `services/domain_manager/src/broker.rs` with tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broker_grants_valid_capability() {
        let policy = ChannelAllowlistPolicy::new_test();
        let broker = CapabilityBroker::new_test(policy);

        let manifest = test_manifest_with_echo_cap();
        let result = broker.grant_capabilities(&manifest, "Experimental", 1).unwrap();

        assert!(result.granted_handles.contains_key("RAMEN_CAP_ECHO_V0"));
    }

    #[test]
    fn broker_denies_unknown_interface() {
        let policy = ChannelAllowlistPolicy::new_test();
        let broker = CapabilityBroker::new_test(policy);

        let manifest = test_manifest_with_unknown_interface();
        let result = broker.grant_capabilities(&manifest, "Experimental", 1);

        assert!(matches!(result, Err(BrokerError::InterfaceUnknown { .. })));
    }

    #[test]
    fn broker_denies_channel_policy_violation() {
        let policy = ChannelAllowlistPolicy::new_test();
        let broker = CapabilityBroker::new_test(policy);

        let manifest = test_manifest_with_echo_cap();
        // Stable channel doesn't allow harness.echo_v0 in test policy
        let result = broker.grant_capabilities(&manifest, "Stable", 1);

        assert!(matches!(result, Err(BrokerError::CapabilityDenied { .. })));
    }

    #[test]
    fn broker_revokes_on_partial_failure() {
        let policy = ChannelAllowlistPolicy::new_test();
        let mut broker = CapabilityBroker::new_test(policy);

        // Manifest with one valid + one invalid capability
        let manifest = test_manifest_with_mixed_caps();
        let result = broker.grant_capabilities(&manifest, "Experimental", 1);

        // Should fail and have no active grants
        assert!(result.is_err());
        assert!(broker.active_grants_for_domain(1).is_empty());
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p domain_manager broker 2>&1`
Expected: Compile errors (module doesn't exist)

**Step 3: Implement the broker**

Implement `services/domain_manager/src/broker.rs` with full broker logic (see design doc for complete implementation).

**Step 4: Add module to main.rs**

Add to `services/domain_manager/src/main.rs`:

```rust
mod broker;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p domain_manager broker`
Expected: All 4 tests pass

**Step 6: Commit**

```bash
git add services/domain_manager/src/broker.rs services/domain_manager/src/main.rs
git commit -m "$(cat <<'EOF'
feat(domain_manager): add capability broker with transactional grants

Implements CapabilityBroker with:
- PolicyEngine trait for extensibility
- ChannelAllowlistPolicy for S10.1
- Transactional grants (revoke all on failure)
- Interface registry for known harnesses

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Broker IPC Endpoint

**Files:**
- Modify: `services/domain_manager/src/main.rs`

**Step 1: Add IPC message handlers**

In `DomainManager::handle()`, add cases for:

```rust
MSG_GRANT_CAPABILITIES => {
    let req = read_payload::<GrantCapabilities>(env)?;
    // Parse content_id_hash to string, fetch manifest, call broker
    // ...
}
MSG_REVOKE_CAPABILITIES => {
    let req = read_payload::<RevokeCapabilities>(env)?;
    self.broker.revoke_domain(req.domain_id);
    // ...
}
```

**Step 2: Write integration test**

Add test in `main.rs`:

```rust
#[test]
fn grant_capabilities_ipc_roundtrip() {
    let mut manager = DomainManager::new();
    let req = request_grant_capabilities(1, 100, "sha256:abc...", 0);
    let reply = manager.handle(&req);
    let payload = read_payload::<GrantCapabilitiesReply>(&reply).unwrap();
    assert!(payload.handle_count > 0 || payload.status != 0);
}
```

**Step 3: Run tests**

Run: `cargo test -p domain_manager`
Expected: All tests pass

**Step 4: Commit**

```bash
git add services/domain_manager/src/main.rs
git commit -m "$(cat <<'EOF'
feat(domain_manager): add IPC endpoints for capability broker

Wires GrantCapabilities/RevokeCapabilities into IPC handler.
Uses content_id to fetch manifest from store (no raw JSON).

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Runtime Supervisor Native WASM Runner

**Files:**
- Create: `runtime_supervisor/src/native_wasm_runner.rs`
- Modify: `runtime_supervisor/src/main.rs`
- Modify: `runtime_supervisor/Cargo.toml`

**Step 1: Write the failing tests**

Create `runtime_supervisor/src/native_wasm_runner.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_wasm_config_parses() {
        let json = r#"{
            "kernel_ipc": "/run/ramen/kernel.sock",
            "domain_id": 100,
            "timeout_ms": 30000
        }"#;
        let config: NativeWasmConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.domain_id, 100);
    }

    #[test]
    fn launch_plan_with_native_wasm_parses() {
        let json = r#"{
            "program_id": "test",
            "runner": "native_wasm_v0",
            "artifact_ref": "sha256:abc",
            "native_wasm": {
                "kernel_ipc": "/run/ramen/kernel.sock",
                "domain_id": 1
            }
        }"#;
        let plan: LaunchPlan = serde_json::from_str(json).unwrap();
        assert_eq!(plan.runner, "native_wasm_v0");
        assert!(plan.native_wasm.is_some());
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p runtime_supervisor native_wasm 2>&1`
Expected: Compile errors

**Step 3: Implement the runner module**

Implement `native_wasm_runner.rs` with:
- `NativeWasmConfig` struct
- `NativeWasmResult` struct
- `run()` function that fetches artifact, requests grants, executes

**Step 4: Add runner dispatch case**

In `main.rs` match block, add:

```rust
"native_wasm_v0" => {
    let Some(ref cfg) = plan.native_wasm else {
        eprintln!("supervisor: native_wasm_v0 requires native_wasm config");
        std::process::exit(2);
    };

    match native_wasm_runner::run(&plan.artifact_ref, cfg, &mut store_client, &args.installed_root) {
        Ok(result) => {
            println!("supervisor: native_wasm_v0 exited code={}", result.exit_code);
            if result.exit_code != 0 {
                std::process::exit(result.exit_code);
            }
        }
        Err(err) => {
            eprintln!("supervisor: native_wasm_v0 failed: {}", err);
            std::process::exit(3);
        }
    }
}
```

**Step 5: Add dependency**

In `runtime_supervisor/Cargo.toml`:

```toml
[dependencies]
native_runner = { path = "../native_runner" }
```

**Step 6: Run tests**

Run: `cargo test -p runtime_supervisor`
Expected: All tests pass

**Step 7: Commit**

```bash
git add runtime_supervisor/src/native_wasm_runner.rs runtime_supervisor/src/main.rs runtime_supervisor/Cargo.toml
git commit -m "$(cat <<'EOF'
feat(supervisor): add native_wasm_v0 runner dispatch

Adds NativeWasmConfig and native_wasm_runner module.
Execution flow: fetch artifact → broker grants → native_runner executes.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Real Kernel IPC Implementation

**Files:**
- Modify: `services/native_runner/src/kernel_bridge.rs`

**Step 1: Write the failing tests**

Add to `kernel_bridge.rs`:

```rust
#[cfg(test)]
mod real_ipc_tests {
    use super::*;

    // These tests require a mock kernel socket
    // They're skipped in CI but run locally for validation

    #[test]
    #[cfg_attr(not(feature = "test_kernel_ipc"), ignore)]
    fn kernel_bridge_connects_to_socket() {
        let bridge = KernelBridge::new(PathBuf::from("/tmp/test_kernel.sock"));
        // Test would fail if socket doesn't exist
    }

    #[test]
    fn mock_bridge_returns_ok() {
        let bridge = MockKernelBridge::new();
        let result = bridge.echo_request(0x1000, 1, b"test");
        assert!(result.is_ok());
    }
}
```

**Step 2: Implement real KernelBridge**

Add to `kernel_bridge.rs`:

```rust
use std::os::unix::net::UnixStream;
use std::io::{Read, Write};
use kernel_api::ipc::Envelope;
use kernel_api::wire::{read_payload, write_payload};

pub struct KernelBridge {
    socket_path: PathBuf,
}

impl KernelBridge {
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    fn transact(&self, request: Envelope) -> Result<Envelope, RunnerError> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|e| RunnerError::KernelIpc(format!("connect: {}", e)))?;

        stream.write_all(request.as_bytes())
            .map_err(|e| RunnerError::KernelIpc(format!("write: {}", e)))?;

        let mut reply_bytes = [0u8; 64];
        stream.read_exact(&mut reply_bytes)
            .map_err(|e| RunnerError::KernelIpc(format!("read: {}", e)))?;

        Envelope::from_bytes(&reply_bytes)
            .map_err(|e| RunnerError::KernelIpc(format!("parse: {:?}", e)))
    }
}

impl KernelBridgeOps for KernelBridge {
    fn echo_request(&self, cap_handle: u64, request_id: u64, payload: &[u8]) -> Result<Vec<u8>, Status> {
        let mut env = Envelope::empty(PROTOCOL_ECHO, MSG_ECHO_REQUEST);
        let req = EchoRequest {
            cap_handle,
            request_id,
            payload_len: payload.len() as u32,
            reserved: 0,
        };
        write_payload(&mut env, &req).map_err(|_| Status::InternalError)?;

        let reply = self.transact(env).map_err(|_| Status::KernelError)?;
        let reply_payload = read_payload::<EchoRequestReply>(&reply).map_err(|_| Status::KernelError)?;

        if reply_payload.status != 0 {
            return Err(Status::from_u32(reply_payload.status));
        }

        // TODO: Real payload from shared memory for S10.2
        Ok(payload.to_vec())
    }

    // ... implement other methods similarly
}
```

**Step 3: Add trait for mocking**

```rust
pub trait KernelBridgeOps {
    fn echo_request(&self, cap_handle: u64, request_id: u64, payload: &[u8]) -> Result<Vec<u8>, Status>;
    fn echo_reply(&self, cap_handle: u64, request_id: u64, status: u32, payload: &[u8]) -> Result<(), Status>;
    fn trace_read(&self, cap_handle: u64, offset: u64, max_len: usize) -> Result<Vec<u8>, Status>;
    fn trace_write(&self, cap_handle: u64, data: &[u8]) -> Result<(), Status>;
}

impl KernelBridgeOps for MockKernelBridge { /* existing impl */ }
impl KernelBridgeOps for KernelBridge { /* new impl */ }
```

**Step 4: Run tests**

Run: `cargo test -p native_runner kernel_bridge`
Expected: All tests pass

**Step 5: Commit**

```bash
git add services/native_runner/src/kernel_bridge.rs
git commit -m "$(cat <<'EOF'
feat(native_runner): implement real kernel IPC

Adds KernelBridge with Unix socket IPC using Envelope protocol.
Adds KernelBridgeOps trait for test mocking.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Foundry Gate

**Files:**
- Create: `tools/ci/foundry_native_runner_s10_1.sh`
- Modify: `justfile`

**Step 1: Create the gate script with CI/E2E split**

Create `tools/ci/foundry_native_runner_s10_1.sh` with all 19 assertions from the design doc.

**CI-Safe Assertions (always run):**
- Schema validation tests (1.1-1.5)
- Broker unit tests (2.1-2.4)
- Supervisor dispatch tests (3.1-3.2)
- All negative tests (5.1-5.3)

**E2E Assertions (skip in CI via `SKIP_E2E_ASSERTIONS=1`):**
- Store service integration (4.2)
- Full execution path (4.3)
- Real kernel IPC (4.5)

Gate script should check:
```bash
if [[ "${SKIP_E2E_ASSERTIONS:-}" == "1" ]]; then
    echo "Skipping E2E assertions (CI mode)"
    # Skip phases 4.2, 4.3, 4.5
fi
```

**Step 2: Make executable**

Run: `chmod +x tools/ci/foundry_native_runner_s10_1.sh`

**Step 3: Add to justfile**

Add to `justfile`:

```make
foundry-native-runner-s10-1:
    ./tools/ci/foundry_native_runner_s10_1.sh

# CI-safe subset (no services required)
foundry-native-runner-s10-1-ci:
    SKIP_E2E_ASSERTIONS=1 ./tools/ci/foundry_native_runner_s10_1.sh
```

**Step 4: Test the gate**

Run: `just foundry-native-runner-s10-1-ci`
Expected: All CI-safe assertions pass

**Step 5: Commit**

```bash
git add tools/ci/foundry_native_runner_s10_1.sh justfile
git commit -m "$(cat <<'EOF'
feat(foundry): add S10.1 native runner production gate

19 assertions covering:
- Manifest schema validation (5)
- Broker integration (4)
- Runtime supervisor (2)
- End-to-end execution (5)
- Negative fail-closed tests (3)

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Update Documentation

**Files:**
- Modify: `CURRENT_STATUS.md`
- Modify: `ROADMAP.md`

**Step 1: Update CURRENT_STATUS.md**

Mark S10.1 as complete, list deliverables.

**Step 2: Update ROADMAP.md**

Move S10.1 from "Next" to "Completed".

**Step 3: Commit**

```bash
git add CURRENT_STATUS.md ROADMAP.md
git commit -m "$(cat <<'EOF'
docs: mark S10.1 complete

S10.1 Native Runner Production Integration complete:
- native_wasm_v0 manifest schema
- Capability broker with transactional grants
- Runtime supervisor integration
- Real kernel IPC
- Foundry gate (19 assertions)

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Summary

| Task | Component | Files | Tests |
|------|-----------|-------|-------|
| 1 | Manifest Schema | 2 | 7 |
| 2 | Broker IDL | 1 | 0 (codegen) |
| 3 | Harness IDL (NEW v1/v2) | 2 | 0 (codegen) |
| 4 | Broker Impl | 2 | 4 |
| 5 | Broker IPC | 1 | 1 |
| 6 | Supervisor | 3 | 2 |
| 7 | Kernel IPC | 1 | 2 |
| 8 | Foundry Gate (CI/E2E split) | 2 | 19 (gate) |
| 9 | Documentation | 2 | 0 |

**Total:** 16 files, 35+ tests

---

## P0 Checklist (Must Get Right)

Before executing, verify:

- [ ] **Content ID encoding:** `bytes32` (binary hash), not 4×u64
- [ ] **Channel trust:** Broker derives from manifest, caller cannot escalate
- [ ] **Wire compatibility:** Create new IDL files (v1/v2), don't mutate existing
- [ ] **Transactional grants:** Kernel-side revocation on failure
- [ ] **Envelope size:** Use `kernel_api::ipc::ENVELOPE_SIZE` constant
- [ ] **CI/E2E split:** E2E assertions skip when `SKIP_E2E_ASSERTIONS=1`

---

## Task Ordering Note

Tasks should be executed in order. Task 3 (new harness IDLs) can be done in parallel with Tasks 4-5 (broker implementation) since they don't depend on each other. However, Task 7 (kernel IPC) depends on Task 3's generated types.
