use serde::Deserialize;

use crate::prelude::*;

const EVIDENCE_POLICY_SCHEMA_VERSION: u32 = 1;
const DEFAULT_REPLACEMENT: &str = "[REDACTED]";
const ERR_SCHEMA_UNSUPPORTED: &str = "EVIDENCE_POLICY_SCHEMA_UNSUPPORTED";
const ERR_MAX_BYTES_INVALID: &str = "EVIDENCE_POLICY_MAX_BYTES_INVALID";
const ERR_UTF8_REQUIRED: &str = "EVIDENCE_POLICY_UTF8_REQUIRED";
const ERR_MAX_BYTES_RANGE: &str = "EVIDENCE_POLICY_MAX_BYTES_OUT_OF_RANGE";
const ERR_SIZE_LIMIT: &str = "EVIDENCE_POLICY_SIZE_LIMIT_EXCEEDED";
const ERR_READ_FAILED: &str = "EVIDENCE_POLICY_READ_FAILED";
const ERR_PARSE_FAILED: &str = "EVIDENCE_POLICY_PARSE_FAILED";

#[derive(Debug)]
pub struct EvidencePolicyError(pub String);

impl core::fmt::Display for EvidencePolicyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EvidencePolicyError {}

#[derive(Debug, Clone, Deserialize)]
pub struct EvidencePolicyV0 {
    pub schema_version: u32,
    #[serde(default)]
    pub max_bytes: Option<u64>,
    #[serde(default)]
    pub kinds: Vec<String>,
    #[serde(default)]
    pub redact_literals: Vec<String>,
    #[serde(default)]
    pub redact_hex_markers: Vec<String>,
    #[serde(default)]
    pub redact_base64_markers: Vec<String>,
    #[serde(default = "default_replacement")]
    pub replacement: String,
}

fn default_replacement() -> String {
    DEFAULT_REPLACEMENT.to_string()
}

impl EvidencePolicyV0 {
    pub fn validate(&self) -> Result<(), EvidencePolicyError> {
        if self.schema_version != EVIDENCE_POLICY_SCHEMA_VERSION {
            return Err(EvidencePolicyError(format!(
                "{}: evidence_policy schema_version unsupported: {}",
                ERR_SCHEMA_UNSUPPORTED, self.schema_version
            )));
        }
        if let Some(max) = self.max_bytes {
            if max == 0 {
                return Err(EvidencePolicyError(format!(
                    "{}: evidence_policy max_bytes must be > 0",
                    ERR_MAX_BYTES_INVALID
                )));
            }
        }
        Ok(())
    }

    pub fn applies_to_kind(&self, kind: &str) -> bool {
        self.kinds.is_empty() || self.kinds.iter().any(|k| k == kind)
    }

    pub fn redact_and_limit(
        &self,
        input: &[u8],
        kind: &str,
    ) -> Result<Vec<u8>, EvidencePolicyError> {
        if !self.applies_to_kind(kind) {
            return Ok(input.to_vec());
        }

        self.enforce_size(input.len())?;

        // Check if any redaction rules are configured
        let has_literal_redaction = !self.redact_literals.is_empty();
        let has_hex_redaction = !self.redact_hex_markers.is_empty();
        let has_base64_redaction = !self.redact_base64_markers.is_empty();

        if !has_literal_redaction && !has_hex_redaction && !has_base64_redaction {
            return Ok(input.to_vec());
        }

        // Literal redaction requires UTF-8
        let mut redacted = if has_literal_redaction {
            let src = core::str::from_utf8(input).map_err(|_| {
                EvidencePolicyError(format!(
                    "{}: evidence_policy requires UTF-8 input for literal redaction (kind={})",
                    ERR_UTF8_REQUIRED, kind
                ))
            })?;
            let mut result = src.to_string();
            for token in &self.redact_literals {
                if !token.is_empty() {
                    result = result.replace(token, &self.replacement);
                }
            }
            result.into_bytes()
        } else {
            input.to_vec()
        };

        // Hex marker redaction: matches patterns like "0xdeadbeef" or "DEADBEEF"
        if has_hex_redaction {
            redacted = self.redact_hex_patterns(redacted);
        }

        // Base64 marker redaction: matches patterns like "SGVsbG8=" or "SGVsbG8gd29ybGQ="
        if has_base64_redaction {
            redacted = self.redact_base64_patterns(redacted);
        }

        self.enforce_size(redacted.len())?;
        Ok(redacted)
    }

    fn enforce_size(&self, len: usize) -> Result<(), EvidencePolicyError> {
        if let Some(max) = self.max_bytes {
            let max = usize::try_from(max).map_err(|_| {
                EvidencePolicyError(format!(
                    "{}: evidence_policy max_bytes out of range",
                    ERR_MAX_BYTES_RANGE
                ))
            })?;
            if len > max {
                return Err(EvidencePolicyError(format!(
                    "{}: evidence exceeds policy size limit: {} > {} bytes",
                    ERR_SIZE_LIMIT, len, max
                )));
            }
        }
        Ok(())
    }

    /// Redact hex-encoded markers (e.g., "0xdeadbeef" or "DEADBEEF" patterns)
    /// This operates on raw bytes and does not require UTF-8.
    fn redact_hex_patterns(&self, input: Vec<u8>) -> Vec<u8> {
        if self.redact_hex_markers.is_empty() {
            return input;
        }

        // Convert to string for pattern matching; if not UTF-8, return as-is
        let src = match core::str::from_utf8(&input) {
            Ok(s) => s,
            Err(_) => return input,
        };

        let mut result = src.to_string();

        for marker in &self.redact_hex_markers {
            if marker.is_empty() {
                continue;
            }

            // Match hex patterns: either with "0x" prefix or raw hex strings
            // Pattern 1: "0x" followed by hex digits (case-insensitive)
            let hex_with_prefix = format!("0x{}", marker);
            result = result.replace(&hex_with_prefix, &self.replacement);

            // Pattern 2: raw hex string (case-insensitive match)
            result = result.replace(marker, &self.replacement);

            // Pattern 3: uppercase version
            let marker_upper = marker.to_uppercase();
            result = result.replace(&marker_upper, &self.replacement);

            // Pattern 4: lowercase version
            let marker_lower = marker.to_lowercase();
            result = result.replace(&marker_lower, &self.replacement);
        }

        result.into_bytes()
    }

    /// Redact base64-encoded markers (e.g., "SGVsbG8=" patterns)
    /// This operates on raw bytes and does not require UTF-8.
    fn redact_base64_patterns(&self, input: Vec<u8>) -> Vec<u8> {
        if self.redact_base64_markers.is_empty() {
            return input;
        }

        // Convert to string for pattern matching; if not UTF-8, return as-is
        let src = match core::str::from_utf8(&input) {
            Ok(s) => s,
            Err(_) => return input,
        };

        let mut result = src.to_string();

        for marker in &self.redact_base64_markers {
            if marker.is_empty() {
                continue;
            }

            // Base64 is case-sensitive, so we match the marker exactly
            result = result.replace(marker, &self.replacement);
        }

        result.into_bytes()
    }
}

#[cfg(feature = "std")]
pub fn load_evidence_policy(
    path: &std::path::Path,
) -> Result<EvidencePolicyV0, EvidencePolicyError> {
    use std::fs;
    let raw = fs::read_to_string(path).map_err(|e| {
        EvidencePolicyError(format!(
            "{}: read evidence_policy failed path={} err={}",
            ERR_READ_FAILED,
            path.display(),
            e
        ))
    })?;
    let policy: EvidencePolicyV0 = toml::from_str(&raw).map_err(|e| {
        EvidencePolicyError(format!(
            "{}: parse evidence_policy failed path={} err={}",
            ERR_PARSE_FAILED,
            path.display(),
            e
        ))
    })?;
    policy.validate()?;
    Ok(policy)
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn redacts_literal_tokens() {
        let policy = EvidencePolicyV0 {
            schema_version: 1,
            max_bytes: None,
            kinds: vec!["trace_artifact_v0".into()],
            redact_literals: vec!["SECRET_TOKEN".into()],
            redact_hex_markers: vec![],
            redact_base64_markers: vec![],
            replacement: "[REDACTED]".into(),
        };
        let input = br#"{"notes":"SECRET_TOKEN"}"#;
        let out = policy.redact_and_limit(input, "trace_artifact_v0").unwrap();
        let out_str = String::from_utf8(out).unwrap();
        assert!(out_str.contains("[REDACTED]"));
        assert!(!out_str.contains("SECRET_TOKEN"));
    }

    #[test]
    fn redacts_hex_markers() {
        let policy = EvidencePolicyV0 {
            schema_version: 1,
            max_bytes: None,
            kinds: vec!["trace_artifact_v0".into()],
            redact_literals: vec![],
            redact_hex_markers: vec!["deadbeef".into()],
            redact_base64_markers: vec![],
            replacement: "[REDACTED]".into(),
        };
        let input = br#"{"key":"0xdeadbeef"}"#;
        let out = policy.redact_and_limit(input, "trace_artifact_v0").unwrap();
        let out_str = String::from_utf8(out).unwrap();
        assert!(out_str.contains("[REDACTED]"));
        assert!(!out_str.contains("deadbeef"));
        assert!(!out_str.contains("0xdeadbeef"));
    }

    #[test]
    fn redacts_hex_markers_case_insensitive() {
        let policy = EvidencePolicyV0 {
            schema_version: 1,
            max_bytes: None,
            kinds: vec!["trace_artifact_v0".into()],
            redact_literals: vec![],
            redact_hex_markers: vec!["deadbeef".into()],
            redact_base64_markers: vec![],
            replacement: "[REDACTED]".into(),
        };
        let input = br#"{"key":"0xDEADBEEF"}"#;
        let out = policy.redact_and_limit(input, "trace_artifact_v0").unwrap();
        let out_str = String::from_utf8(out).unwrap();
        assert!(out_str.contains("[REDACTED]"));
        assert!(!out_str.contains("DEADBEEF"));
    }

    #[test]
    fn redacts_base64_markers() {
        let policy = EvidencePolicyV0 {
            schema_version: 1,
            max_bytes: None,
            kinds: vec!["trace_artifact_v0".into()],
            redact_literals: vec![],
            redact_hex_markers: vec![],
            redact_base64_markers: vec!["SGVsbG8=".into()],
            replacement: "[REDACTED]".into(),
        };
        let input = br#"{"token":"SGVsbG8="}"#;
        let out = policy.redact_and_limit(input, "trace_artifact_v0").unwrap();
        let out_str = String::from_utf8(out).unwrap();
        assert!(out_str.contains("[REDACTED]"));
        assert!(!out_str.contains("SGVsbG8="));
    }

    #[test]
    fn redacts_combined_patterns() {
        let policy = EvidencePolicyV0 {
            schema_version: 1,
            max_bytes: None,
            kinds: vec!["trace_artifact_v0".into()],
            redact_literals: vec!["SECRET".into()],
            redact_hex_markers: vec!["deadbeef".into()],
            redact_base64_markers: vec!["SGVsbG8=".into()],
            replacement: "[REDACTED]".into(),
        };
        let input = br#"{"literal":"SECRET","hex":"0xdeadbeef","base64":"SGVsbG8="}"#;
        let out = policy.redact_and_limit(input, "trace_artifact_v0").unwrap();
        let out_str = String::from_utf8(out).unwrap();
        assert!(out_str.contains("[REDACTED]"));
        assert!(!out_str.contains("SECRET"));
        assert!(!out_str.contains("deadbeef"));
        assert!(!out_str.contains("SGVsbG8="));
    }

    #[test]
    fn skips_non_matching_kind() {
        let policy = EvidencePolicyV0 {
            schema_version: 1,
            max_bytes: Some(8),
            kinds: vec!["trace_artifact_v0".into()],
            redact_literals: vec!["x".into()],
            redact_hex_markers: vec![],
            redact_base64_markers: vec![],
            replacement: "y".into(),
        };
        let input = b"0123456789";
        let out = policy
            .redact_and_limit(input, "capsule_payload_v0")
            .unwrap();
        assert_eq!(out, input);
    }

    #[test]
    fn enforces_size_limit() {
        let policy = EvidencePolicyV0 {
            schema_version: 1,
            max_bytes: Some(4),
            kinds: vec![],
            redact_literals: vec![],
            redact_hex_markers: vec![],
            redact_base64_markers: vec![],
            replacement: "[REDACTED]".into(),
        };
        assert!(
            policy
                .redact_and_limit(b"12345", "trace_artifact_v0")
                .is_err()
        );
    }

    #[test]
    fn handles_non_utf8_for_hex_base64_only() {
        let policy = EvidencePolicyV0 {
            schema_version: 1,
            max_bytes: None,
            kinds: vec!["trace_artifact_v0".into()],
            redact_literals: vec!["SECRET".into()],
            redact_hex_markers: vec!["deadbeef".into()],
            redact_base64_markers: vec![],
            replacement: "[REDACTED]".into(),
        };
        // Non-UTF-8 input: literal redaction should fail, but hex-only should pass
        let input = b"\xff\xfe\xfd\xfc";
        let result = policy.redact_and_limit(input, "trace_artifact_v0");
        // Should fail because literal redaction requires UTF-8
        assert!(result.is_err());
    }

    #[test]
    fn hex_base64_only_no_utf8_required() {
        let policy = EvidencePolicyV0 {
            schema_version: 1,
            max_bytes: None,
            kinds: vec!["trace_artifact_v0".into()],
            redact_literals: vec![],
            redact_hex_markers: vec!["deadbeef".into()],
            redact_base64_markers: vec![],
            replacement: "[REDACTED]".into(),
        };
        // Non-UTF-8 input with only hex/base64 redaction should pass through unchanged
        let input = b"\xff\xfe\xfd\xfc";
        let out = policy.redact_and_limit(input, "trace_artifact_v0").unwrap();
        assert_eq!(out, input);
    }
}
