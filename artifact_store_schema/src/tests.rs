//! Integration tests for schema-only boundary (SC-09).

#[cfg(feature = "std")]
use crate::{ContentId, Manifest};

#[cfg(all(test, feature = "std"))]
mod std_tests {
    use super::{ContentId, Manifest};

    const VALID_ID: &str =
        "sha256:0000000000000000000000000000000000000000000000000000000000000001";

    #[test]
    fn schema_types_are_serializable() {
        let manifest = Manifest {
            schema_version: 1,
            content_id: VALID_ID.into(),
            size_bytes: 1024,
            kind: "trace_artifact_v0".into(),
            channels: vec!["stable".into()],
            signatures: vec![],
        };

        let json = serde_json::to_string(&manifest).unwrap();
        let deserialized: Manifest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.schema_version, manifest.schema_version);
        assert_eq!(deserialized.content_id, manifest.content_id);
        assert_eq!(deserialized.size_bytes, manifest.size_bytes);
        assert_eq!(deserialized.kind, manifest.kind);
        assert_eq!(deserialized.channels, manifest.channels);
    }

    #[test]
    fn content_id_validation_works() {
        let valid = VALID_ID;
        let cid = ContentId::parse(valid).unwrap();
        assert_eq!(cid.as_str(), valid);
        assert_eq!(cid.hash_hex(), &valid[7..]);

        use core::str::FromStr;
        let cid2 = ContentId::from_str(valid).unwrap();
        assert_eq!(cid2.as_str(), valid);

        assert!(
            ContentId::parse("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20")
                .is_err()
        );
        assert!(ContentId::parse("sha256:abc").is_err());
        assert!(
            ContentId::parse(
                "sha256:000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F20"
            )
            .is_err()
        );
        assert!(
            ContentId::parse(
                "sha256:000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f2g"
            )
            .is_err()
        );
    }

    #[test]
    fn evidence_policy_validation_works() {
        use crate::evidence_policy::EvidencePolicyV0;

        let policy = EvidencePolicyV0 {
            schema_version: 1,
            max_bytes: Some(1024),
            kinds: vec!["trace_artifact_v0".into()],
            redact_literals: vec!["SECRET".into()],
            redact_hex_markers: vec!["deadbeef".into()],
            redact_base64_markers: vec!["SGVsbG8=".into()],
            replacement: "[REDACTED]".into(),
        };
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn schema_crate_exposes_only_types_and_validation() {
        let _cid = ContentId::parse(VALID_ID).unwrap();

        let _manifest = Manifest {
            schema_version: 1,
            content_id: VALID_ID.into(),
            size_bytes: 1024,
            kind: "test".into(),
            channels: vec![],
            signatures: vec![],
        };
    }
}
