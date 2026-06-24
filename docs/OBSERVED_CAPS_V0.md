# Observed Capability Summary V0

**Last Updated:** 2026-02-18
**Status:** Active

## 0) Purpose
Observed caps are content-addressed JSON summaries that record **what capabilities were
actually used** during a run. They reference evidence (protocol traces) by content ID.

---

## 1) Artifact schema (v0)
```json
{
  "schema_version": 1,
  "program_id": "portal.file_picker.ro",
  "run_id": "sha256:<trace_id>",
  "launch_plan_id": "sha256:<plan_id>",
  "capabilities": [
    {
      "cap": "portal.file_picker.ro",
      "scope": { "artifact_ids": ["sha256:<selected_artifact>"] },
      "counts": { "granted": 1, "used": 1 },
      "evidence": ["sha256:<protocol_trace>"]
    }
  ],
  "evidence": ["sha256:<protocol_trace>"]
}
```

`launch_plan_id` is optional.

---

## 2) Notes (v0)
- `capabilities` must be non-empty.
- `used` must be ≤ `granted`.
- All `artifact_ids` and `evidence` entries must be content IDs.
- Evidence is **referential** (no duplication of trace blobs).
