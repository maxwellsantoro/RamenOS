//! Claim v0 — queue item claim/lock workflow.

use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[cfg(feature = "std")]
use time::Duration;
#[cfg(feature = "std")]
use time::OffsetDateTime;
#[cfg(feature = "std")]
use time::format_description::well_known::Rfc3339;

const CLAIM_SCHEMA_VERSION: u32 = 1;

#[derive(Debug)]
pub struct ClaimValidationError(pub String);

impl core::fmt::Display for ClaimValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ClaimValidationError {}

/// Claim artifact for queue item lock.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClaimV0 {
    pub schema_version: u32,

    /// Content ID of queue item being claimed.
    pub queue_item_id: String,

    /// Claimant identifier (e.g., email, username).
    pub claimant_id: String,

    /// Claim timestamp (RFC 3339 format).
    pub timestamp: String,

    /// Optional lease duration in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_duration_secs: Option<u64>,

    /// Optional notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Validate a claim artifact.
pub fn validate_claim(claim: &ClaimV0) -> Result<(), ClaimValidationError> {
    if claim.schema_version != CLAIM_SCHEMA_VERSION {
        return Err(ClaimValidationError(format!(
            "claim schema_version unsupported: {}",
            claim.schema_version
        )));
    }

    if !claim.queue_item_id.starts_with("sha256:") {
        return Err(ClaimValidationError(
            "queue_item_id must be sha256: content ID".into(),
        ));
    }

    if claim.claimant_id.trim().is_empty() {
        return Err(ClaimValidationError("claimant_id required".into()));
    }

    if claim.timestamp.trim().is_empty() {
        return Err(ClaimValidationError("timestamp required".into()));
    }

    #[cfg(feature = "std")]
    let _ = parse_timestamp(&claim.timestamp)?;

    Ok(())
}

#[cfg(feature = "std")]
fn parse_timestamp(ts: &str) -> Result<OffsetDateTime, ClaimValidationError> {
    OffsetDateTime::parse(ts, &Rfc3339).map_err(|_| {
        ClaimValidationError(
            "timestamp must be RFC 3339 format (e.g., 2026-02-05T12:00:00Z)".into(),
        )
    })
}

#[cfg(feature = "std")]
fn claim_expires_at(claim: &ClaimV0) -> Result<Option<OffsetDateTime>, ClaimValidationError> {
    let timestamp = parse_timestamp(&claim.timestamp)?;
    let lease_secs = match claim.lease_duration_secs {
        Some(v) => v,
        None => return Ok(None),
    };
    let lease_secs_i64 = i64::try_from(lease_secs)
        .map_err(|_| ClaimValidationError("lease_duration_secs out of range".into()))?;
    let lease = Duration::seconds(lease_secs_i64);
    let expires = timestamp
        .checked_add(lease)
        .ok_or_else(|| ClaimValidationError("claim expiry overflow".into()))?;
    Ok(Some(expires))
}

#[cfg(feature = "std")]
pub fn claim_is_expired(
    claim: &ClaimV0,
    now: OffsetDateTime,
) -> Result<bool, ClaimValidationError> {
    match claim_expires_at(claim)? {
        Some(expires) => Ok(now > expires),
        None => Ok(false),
    }
}

/// Resolve active claim by applying "latest valid claim wins".
///
/// Rules:
/// - claim must pass schema validation
/// - expired claims are ignored
/// - newest timestamp wins
/// - if timestamps are equal, claimant_id lexical order breaks ties deterministically
#[cfg(feature = "std")]
pub fn resolve_latest_valid_claim(
    claims: &[ClaimV0],
    now: OffsetDateTime,
) -> Result<Option<&ClaimV0>, ClaimValidationError> {
    let mut winner: Option<(&ClaimV0, OffsetDateTime)> = None;

    for claim in claims {
        validate_claim(claim)?;
        if claim_is_expired(claim, now)? {
            continue;
        }

        let ts = parse_timestamp(&claim.timestamp)?;
        match winner {
            None => winner = Some((claim, ts)),
            Some((current, current_ts)) => {
                if ts > current_ts || (ts == current_ts && claim.claimant_id > current.claimant_id)
                {
                    winner = Some((claim, ts));
                }
            }
        }
    }

    Ok(winner.map(|(claim, _)| claim))
}

/// Create a new claim for a queue item.
#[cfg(feature = "std")]
pub fn create_claim(
    queue_item_id: &str,
    claimant_id: &str,
    lease_duration_secs: Option<u64>,
) -> ClaimV0 {
    let timestamp = now_rfc3339();
    ClaimV0 {
        schema_version: 1,
        queue_item_id: queue_item_id.to_string(),
        claimant_id: claimant_id.to_string(),
        timestamp,
        lease_duration_secs,
        notes: None,
    }
}

/// Get current timestamp in RFC 3339 format.
#[cfg(feature = "std")]
fn now_rfc3339() -> String {
    #[cfg(feature = "std")]
    {
        OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn validate_ok() {
        let claim = ClaimV0 {
            schema_version: 1,
            queue_item_id: "sha256:abc123".into(),
            claimant_id: "user@example.com".into(),
            timestamp: "2026-02-05T12:00:00Z".into(),
            lease_duration_secs: Some(604800),
            notes: None,
        };
        validate_claim(&claim).unwrap();
    }

    #[test]
    fn validate_missing_claimant() {
        let claim = ClaimV0 {
            schema_version: 1,
            queue_item_id: "sha256:abc123".into(),
            claimant_id: "".into(),
            timestamp: "2026-02-05T12:00:00Z".into(),
            lease_duration_secs: None,
            notes: None,
        };
        assert!(validate_claim(&claim).is_err());
    }

    #[test]
    fn validate_bad_timestamp() {
        let claim = ClaimV0 {
            schema_version: 1,
            queue_item_id: "sha256:abc123".into(),
            claimant_id: "user@example.com".into(),
            timestamp: "not-a-timestamp".into(),
            lease_duration_secs: None,
            notes: None,
        };
        assert!(validate_claim(&claim).is_err());
    }

    #[test]
    fn create_claim_works() {
        let claim = create_claim("sha256:abc", "user@test.com", Some(3600));
        assert_eq!(claim.schema_version, 1);
        assert_eq!(claim.queue_item_id, "sha256:abc");
        assert_eq!(claim.claimant_id, "user@test.com");
        assert!(OffsetDateTime::parse(&claim.timestamp, &Rfc3339).is_ok());
    }

    #[test]
    fn latest_valid_claim_wins() {
        let claims = vec![
            ClaimV0 {
                schema_version: 1,
                queue_item_id: "sha256:abc123".into(),
                claimant_id: "alice@example.com".into(),
                timestamp: "2026-02-05T12:00:00Z".into(),
                lease_duration_secs: Some(86400),
                notes: None,
            },
            ClaimV0 {
                schema_version: 1,
                queue_item_id: "sha256:abc123".into(),
                claimant_id: "bob@example.com".into(),
                timestamp: "2026-02-05T13:00:00Z".into(),
                lease_duration_secs: Some(86400),
                notes: None,
            },
        ];

        let now = OffsetDateTime::parse("2026-02-05T14:00:00Z", &Rfc3339).unwrap();
        let winner = resolve_latest_valid_claim(&claims, now).unwrap().unwrap();
        assert_eq!(winner.claimant_id, "bob@example.com");
    }

    #[test]
    fn expired_claim_is_ignored() {
        let claims = vec![
            ClaimV0 {
                schema_version: 1,
                queue_item_id: "sha256:abc123".into(),
                claimant_id: "expired@example.com".into(),
                timestamp: "2026-02-05T12:00:00Z".into(),
                lease_duration_secs: Some(60),
                notes: None,
            },
            ClaimV0 {
                schema_version: 1,
                queue_item_id: "sha256:abc123".into(),
                claimant_id: "active@example.com".into(),
                timestamp: "2026-02-05T11:00:00Z".into(),
                lease_duration_secs: Some(86400),
                notes: None,
            },
        ];

        let now = OffsetDateTime::parse("2026-02-05T13:00:00Z", &Rfc3339).unwrap();
        let winner = resolve_latest_valid_claim(&claims, now).unwrap().unwrap();
        assert_eq!(winner.claimant_id, "active@example.com");
    }
}
