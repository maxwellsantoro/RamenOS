//! Native WASM v0 manifest schema — capability declarations for WASM modules.

use serde::{Deserialize, Serialize};

use crate::prelude::*;

use crate::Manifest;

const NATIVE_WASM_SCHEMA_VERSION: u32 = 1;
const MAX_RIGHTS_MASK: u64 = 0xFFFF;

#[derive(Debug)]
pub struct NativeWasmValidationError(pub String);

impl core::fmt::Display for NativeWasmValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NativeWasmValidationError {}

/// Native WASM artifact manifest (v0)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeWasmManifestV0 {
    #[serde(flatten)]
    pub manifest: Manifest,
    pub native_wasm: NativeWasmV0,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeWasmV0 {
    #[serde(default = "default_entrypoint")]
    pub entrypoint: String,
    pub required_capabilities: Vec<RequiredCapability>,
    #[serde(default)]
    pub declares_no_capabilities: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredCapability {
    pub export_name: String,
    pub interface: String,
    pub rights: u64,
    pub purpose: String,
}

fn default_entrypoint() -> String {
    "_start".to_string()
}

/// Validate a complete native WASM manifest.
pub fn validate_native_wasm_manifest(
    m: &NativeWasmManifestV0,
) -> Result<(), NativeWasmValidationError> {
    if m.manifest.schema_version != NATIVE_WASM_SCHEMA_VERSION {
        return Err(NativeWasmValidationError(format!(
            "native_wasm schema_version unsupported: {}",
            m.manifest.schema_version
        )));
    }
    if m.manifest.kind != "native_wasm_v0" {
        return Err(NativeWasmValidationError(format!(
            "expected kind native_wasm_v0, got: {}",
            m.manifest.kind
        )));
    }
    validate_native_wasm(&m.native_wasm)
}

/// Validate the native WASM-specific section.
pub fn validate_native_wasm(w: &NativeWasmV0) -> Result<(), NativeWasmValidationError> {
    if w.entrypoint.trim().is_empty() {
        return Err(NativeWasmValidationError("entrypoint required".into()));
    }
    if w.required_capabilities.is_empty() && !w.declares_no_capabilities {
        return Err(NativeWasmValidationError(
            "required_capabilities empty but declares_no_capabilities not set".into(),
        ));
    }

    // Manual duplicate check to avoid HashSet dependency in no_std
    for (i, cap) in w.required_capabilities.iter().enumerate() {
        validate_capability(cap)?;
        for prev in &w.required_capabilities[..i] {
            if cap.export_name == prev.export_name {
                return Err(NativeWasmValidationError(format!(
                    "duplicate export_name: {}",
                    cap.export_name
                )));
            }
        }
    }
    Ok(())
}

/// Validate a single capability requirement.
pub fn validate_capability(cap: &RequiredCapability) -> Result<(), NativeWasmValidationError> {
    validate_export_name(&cap.export_name)?;
    validate_interface_format(&cap.interface)?;
    if cap.rights == 0 {
        return Err(NativeWasmValidationError(format!(
            "rights must be non-zero for: {}",
            cap.export_name
        )));
    }
    if cap.rights > MAX_RIGHTS_MASK {
        return Err(NativeWasmValidationError(format!(
            "rights exceeds max mask for: {}",
            cap.export_name
        )));
    }
    if cap.purpose.trim().is_empty() {
        return Err(NativeWasmValidationError(format!(
            "purpose required for: {}",
            cap.export_name
        )));
    }
    Ok(())
}

/// Validate export name format: RAMEN_CAP_[A-Z0-9_]+
pub fn validate_export_name(name: &str) -> Result<(), NativeWasmValidationError> {
    const PREFIX: &str = "RAMEN_CAP_";
    if !name.starts_with(PREFIX) {
        return Err(NativeWasmValidationError(format!(
            "export_name must start with {}: {}",
            PREFIX, name
        )));
    }
    let suffix = &name[PREFIX.len()..];
    if suffix.is_empty() {
        return Err(NativeWasmValidationError(
            "export_name must have at least one character after RAMEN_CAP_".into(),
        ));
    }
    if !suffix
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(NativeWasmValidationError(format!(
            "export_name suffix must be [A-Z0-9_]: {}",
            name
        )));
    }
    Ok(())
}

/// Validate interface format: namespace.name_vN (e.g., harness.echo_v0)
pub fn validate_interface_format(iface: &str) -> Result<(), NativeWasmValidationError> {
    let v_pos = iface.rfind("_v").ok_or_else(|| {
        NativeWasmValidationError(format!("interface must end with _v<N>: {}", iface))
    })?;

    let version_part = &iface[v_pos + 2..];
    if version_part.is_empty() || !version_part.chars().all(|c| c.is_ascii_digit()) {
        return Err(NativeWasmValidationError(format!(
            "interface version must be digits: {}",
            iface
        )));
    }

    let namespace_part = &iface[..v_pos];
    if namespace_part.is_empty() {
        return Err(NativeWasmValidationError(format!(
            "interface must have namespace before _v: {}",
            iface
        )));
    }

    if !namespace_part
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_')
    {
        return Err(NativeWasmValidationError(format!(
            "interface namespace must be [a-z0-9._]: {}",
            iface
        )));
    }

    Ok(())
}

#[cfg(all(test, feature = "std"))]
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
    fn invalid_interface_format_rejected() {
        let cap = RequiredCapability {
            export_name: "RAMEN_CAP_TEST".to_string(),
            interface: "invalid_format".to_string(), // missing _v<N>
            rights: 1,
            purpose: "test".to_string(),
        };
        assert!(validate_capability(&cap).is_err());
    }

    #[test]
    fn rights_exceeds_max_rejected() {
        let cap = RequiredCapability {
            export_name: "RAMEN_CAP_TEST".to_string(),
            interface: "harness.echo_v0".to_string(),
            rights: 0x10000, // exceeds MAX_RIGHTS_MASK (0xFFFF)
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
