# Trace Artifact V0 Schema

**Last Updated:** 2026-02-18
**Status:** Active

## 0) Purpose
Trace artifacts are content-addressed JSON documents stored in the Artifact Store.
They capture either:
- **protocol_trace**: typed harness transcripts (spec-by-example), or
- **scenario_trace**: user intent + portal interactions (app scenarios).

Protocol traces are the default spec input for Driver Capsules and Foundry replay.
Hardware-driver Oracle traces use the sibling `driver_protocol_trace_v0` schema
(`artifact_store_schema::driver_protocol_trace`) because PCI/MMIO/IRQ events are
not request/response harness transcripts.

---

## 1) Artifact wrapper (v0)
```json
{
  "schema_version": 1,
  "trace_type": "protocol_trace",
  "protocol_trace": { ... }
}
```

`trace_type`:
- `"protocol_trace"` or `"scenario_trace"`

Exactly one of `protocol_trace` or `scenario_trace` must be present.

---

## 2) protocol_trace (required for Driver Capsule v0)
```json
{
  "metadata": {
    "trace_id": "sha256:<hex>",        // optional; should match artifact content_id
    "timestamp_start": "RFC3339",       // optional
    "timestamp_end": "RFC3339",         // optional
    "capsule_id": "string",             // optional
    "capsule_image": "sha256:<hex>",    // optional
    "harness_name": "ping_harness",
    "harness_version": 0,
    "policy_bundle_id": "sha256:<hex>"  // optional
  },
  "events": [
    {
      "seq": 1,
      "dir": "request",
      "op": "ping",
      "bytes_hex": "70696e67",
      "result": "ok",
      "notes": "optional"
    },
    {
      "seq": 2,
      "dir": "response",
      "op": "pong",
      "bytes_hex": "706f6e67",
      "result": "ok"
    }
  ]
}
```

**Field notes**
- `seq` is strictly monotonic per trace.
- `bytes_hex` is lowercase hex encoding of the raw request/response payload bytes.
- `op` is optional if the harness schema is unknown; include when available.

---

## 3) scenario_trace (optional in S3)
Scenario traces capture user intent + portal interactions. v0 uses events as **indexes**
into evidence artifacts (protocol traces, observed caps) rather than duplicating data.
The wrapper stays the same:
```json
{
  "metadata": { "scenario_id": "string", "timestamp_start": "RFC3339", "timestamp_end": "RFC3339" },
  "events": [
    { "seq": 1, "name": "protocol_trace_ref", "payload": { "content_id": "sha256:<hex>" } },
    { "seq": 2, "name": "observed_caps_ref", "payload": { "content_id": "sha256:<hex>" } },
    { "seq": 3, "name": "selection", "payload": { "artifact_id": "sha256:<hex>" } }
  ]
}
```

---

## 4) Normalization (v0)
For deduplication and stable IDs:
- Compute the artifact `content_id` from the full JSON blob.
- If `trace_id` is present, it **must** equal the artifact `content_id`.
- Timestamp fields are optional; omit them for deterministic traces.

---

## 5) Replay contract (v0)
Foundry can replay a protocol_trace by:
1) launching the same backend (capsule image hash),
2) sending the recorded request sequence,
3) asserting response equivalence (byte match by default).

---

## 6) Redaction + size policy (v0)
- `bytes_hex` may be redacted (replace with empty string + note).
- Traces default to local-only; upload is opt-in.
- Size caps apply at ingestion (truncate with marker + note).
