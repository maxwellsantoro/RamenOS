# Evidence Policy V0

**Last Updated:** 2026-10-04
**Status:** Active

Defines optional pre-ingestion marker replacement and byte limits. The canonical
implementation is [evidence_policy.rs](../artifact_store_schema/src/evidence_policy.rs).
It recognizes configured strings; it does not discover arbitrary secrets or
make evidence safe to publish.

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
- `max_bytes` (optional, positive) rejects input or replaced output over the limit;
  it never truncates a record into success.
- `kinds` (optional) scopes the entire policy, including the byte limit; an empty
  list applies to all kinds. Unselected kinds pass through unchanged.
- `redact_literals` replaces exact nonempty strings in UTF-8 content.
- `redact_hex_markers` replaces each configured spelling, its all-uppercase and
  all-lowercase forms, plus the exact `0x`-prefixed configured spelling. This is
  not general case-insensitive matching: arbitrary mixed-case forms can remain.
- `redact_base64_markers` replaces exact, case-sensitive strings; it does not
  decode or normalize alternative encodings.
- `replacement` sets the redaction marker

## UTF-8 and structured evidence

- `redact_literals` requires UTF-8 input (returns error for non-UTF-8)
- `redact_hex_markers` and `redact_base64_markers` only operate on UTF-8 content
- If input is non-UTF-8 and only hex/base64 markers are configured, input passes through unchanged
- If input is non-UTF-8 and literal markers are configured, returns error

Replacement is textual, not JSON/schema-aware. Consumers must validate the
resulting artifact and preserve its redaction/coverage limits before relying on
it for replay or claims. Omission of a marker is not evidence that a secret is absent.

## Integration

- `store_cli ingest --evidence-policy <path>`

The capsule relay does not currently expose this option. Producing a trace and
ingesting it with a configured policy are separate steps.

The configured integration applies the policy before hashing/writing. The content
ID identifies the resulting bytes; it does not certify complete redaction or
authorize uploading them.
