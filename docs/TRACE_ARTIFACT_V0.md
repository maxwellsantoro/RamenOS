# Trace Artifact V0 Schema

**Last Updated:** 2026-10-04
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

[trace.rs](../artifact_store_schema/src/trace.rs) owns the current wrapper and
validation rules. A valid shape is not proof that a backend produced the events.

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
    "trace_id": "capture-identifier",  // optional producer metadata
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
- `bytes_hex` is hex encoding of the raw request/response payload bytes, with a
  maximum decoded size of 64 KiB per event. Emit lowercase for consistency;
  the schema validator also accepts uppercase.
- `op` is optional if the harness schema is unknown; include when available.
- The schema requires a nonempty harness name and event list. It checks monotonic
  sequence numbers and hex bounds, not response authenticity or request pairing.

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

## 4) Identity and normalization (v0)

The Store content ID hashes the final stored JSON bytes. Optional `trace_id` is
producer metadata; the schema does not bind it to that content ID. Do not embed
the blob's own hash inside its hashed content. References to the stored artifact
carry the content ID outside it.

Timestamp fields are optional. A deterministic fixture may omit them; actual
Oracle provenance must retain the timestamps and origin required by its gate.
Serialization differences change the blob hash. Scenario payload references are
generic JSON at this layer and need validation by the consuming contract.

---

## 5) Transcript checks and replay boundary (v0)

[replay_protocol_trace.py](../tools/trace/replay_protocol_trace.py) statically
checks request/reply pairing, contiguous sequence numbers, named operation widths
and selected ping/echo relationships. It hashes canonical pairs and can compare
two supplied transcripts. Despite its name, it does not launch a backend or send
recorded requests. Schema validation and these checks have different coverage.

Actual backend replay requires a pinned environment, dispatch of the recorded
requests, captured fresh responses and a declared comparison rule. The driver
Oracle/replay lane uses its own trace schema and gates; neither transcript shape
nor digest equality establishes native device I/O or physical qualification.

---

## 6) Redaction + size policy (v0)
- `bytes_hex` may be redacted (replace with empty string + note).
- Traces default to local-only; upload is opt-in.
- Configured [evidence-policy](EVIDENCE_POLICY_V0.md) size caps reject oversized
  input/output; they do not truncate it into a successful artifact.
- Redaction changes bytes and may prevent byte-faithful replay. Record that limit
  and validate the resulting JSON and hex shape after text replacement.
