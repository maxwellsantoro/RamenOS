# Claim Artifact V0 Schema

**Last Updated:** 2026-02-18
**Status:** Active

## 0) Purpose
Claims are content-addressed JSON documents that record queue item ownership.
They enable an auditable, offline-first claim/lock workflow.

Rule: "Latest valid claim wins" — the most recent claim for a queue item takes precedence.

---

## 1) Artifact schema (v0)
```json
{
  "schema_version": 1,
  "queue_item_id": "sha256:...",
  "claimant_id": "user@example.com",
  "timestamp": "2026-02-05T12:00:00Z",
  "lease_duration_secs": 604800,
  "notes": "optional notes"
}
```

---

## 2) Fields

### queue_item_id (required)
Content ID of the queue item being claimed.
Must be `sha256:` prefixed.

### claimant_id (required)
Identifier of the claimant (email, username, etc.).

### timestamp (required)
Claim timestamp in RFC 3339 format.
Example: `2026-02-05T12:00:00Z`

### lease_duration_secs (optional)
Lease duration in seconds. If specified, the claim expires after this duration.
Common values:
- 604800 (1 week)
- 2592000 (30 days)

### notes (optional)
Optional notes about the claim.

---

## 3) Validation rules
- `schema_version` must be 1
- `queue_item_id` must be `sha256:` prefixed
- `claimant_id` must be non-empty
- `timestamp` must parse as RFC 3339

---

## 4) Claim resolution
The "latest valid claim wins" rule means:
1. Claims are sorted by timestamp
2. The most recent valid claim takes precedence
3. Expired claims (past lease duration) are ignored

---

## 5) CLI commands
```bash
# Create a claim
store_cli claim --item sha256:abc123 --claimant "user@example.com" --lease-secs 604800 --out claim.json

# Validate a claim
store_cli validate-claim --src claim.json

# Resolve claim winner from a chain (latest valid claim wins)
store_cli resolve-claim --src claim_old.json --src claim_new.json --now 2026-02-05T13:00:00Z
```
