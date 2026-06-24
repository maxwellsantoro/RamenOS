# Evidence Policy V0

**Last Updated:** 2026-02-18
**Status:** Active

Defines pre-ingestion evidence handling rules (redaction + size limits).

## Schema

```toml
schema_version = 1
max_bytes = 262144
kinds = ["trace_artifact_v0", "observed_caps_v0", "crash_context_v0"]
redact_literals = ["SECRET_TOKEN", "api_key=", "authorization: bearer "]
redact_hex_markers = ["deadbeef", "c0ffee"]
redact_base64_markers = ["SGVsbG8=", "d29ybGQ="]
replacement = "[REDACTED]"
```

## Rules

- `schema_version` must be `1`
- `max_bytes` (optional) rejects oversized evidence
- `kinds` (optional) scopes policy to selected artifact kinds; empty means all kinds
- `redact_literals` (optional) replaces literal strings in UTF-8 evidence content
- `redact_hex_markers` (optional) replaces hex-encoded patterns (case-insensitive)
  - Matches both `0xdeadbeef` and raw `deadbeef` patterns
  - Case-insensitive matching (matches `deadbeef`, `DEADBEEF`, `DeadBeef`, etc.)
- `redact_base64_markers` (optional) replaces base64-encoded patterns (case-sensitive)
  - Matches exact base64 strings (e.g., `SGVsbG8=`)
- `replacement` sets the redaction marker

## Multi-Encoding Redaction Details

### Hex Marker Redaction
Hex marker redaction operates on UTF-8 evidence content. It matches patterns in multiple forms:
- With `0x` prefix: `0xdeadbeef` → `[REDACTED]`
- Raw hex string: `deadbeef` → `[REDACTED]`
- Case-insensitive: `DEADBEEF`, `DeadBeef`, `deadbeef` all match

This is useful for redacting:
- Memory addresses
- Hash digests
- Binary identifiers

### Base64 Marker Redaction
Base64 marker redaction operates on UTF-8 evidence content. It matches exact base64 strings:
- Exact match: `SGVsbG8=` → `[REDACTED]`
- Case-sensitive: `sgvsbG8=` does NOT match `SGVsbG8=`

This is useful for redacting:
- Encoded tokens
- Certificate fingerprints
- Encrypted payloads

### UTF-8 Requirements
- `redact_literals` requires UTF-8 input (returns error for non-UTF-8)
- `redact_hex_markers` and `redact_base64_markers` only operate on UTF-8 content
- If input is non-UTF-8 and only hex/base64 markers are configured, input passes through unchanged
- If input is non-UTF-8 and literal markers are configured, returns error

## Integration

- `store_cli ingest --evidence-policy <path>`
- `capsule_relay --evidence-policy <path>`

Applied before content hashing/writing so stored artifacts are policy-compliant by construction.
