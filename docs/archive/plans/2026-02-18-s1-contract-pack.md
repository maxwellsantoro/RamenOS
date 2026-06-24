# S1 Contract Pack (ADR-lite)

**Last Updated:** 2026-02-18
**Status:** Historical
**Scope:** Slice S1 planning only (host tooling first, no kernel integration).

## 0) Purpose
Define the minimal, explicit contracts for S1 so implementation can stay small and testable:
- Artifact addressing
- Artifact container layout
- Signing decision (for S1)
- Channel metadata
- Foundry gate expectations

This document is intentionally short and operational.

## 1) Artifact Addressing
Decision: Content IDs are SHA-256 of the raw artifact blob bytes.

Format:
- `sha256:<hex>` (lowercase hex, 64 chars)

Rationale:
- Single canonical source of truth (blob bytes only)
- Easy to compute/verify across host tools
- Stable across future container changes

## 2) Artifact Container (S1)
Decision: Artifact is a raw blob file plus a minimal JSON manifest sidecar.
No bundle/container format in S1.

Directory convention (host-only):
- `out/artifacts/<sha>.blob`
- `out/artifacts/<sha>.manifest.json`

Manifest schema (v0):
```
{
  "schema_version": 1,
  "content_id": "sha256:<hex>",
  "size_bytes": 12345,
  "kind": "component|trace|other",
  "channels": ["Experimental"],
  "signatures": []
}
```

Notes:
- `channels` is metadata only; no policy enforcement in S1.
- `kind` is an informational string; do not infer behavior from it yet.
- `signatures` is reserved for S2+; S1 does not enforce signatures.
- `schema_version` is validated by tools; S1 accepts version `1` only.

Atomic write rules (S1 host-only):
- Write blob to a temp path → fsync → rename into place
- Write manifest to a temp path → fsync → rename into place

## 3) Signing (S1 decision)
Decision: No signatures enforced in S1.

Rationale:
- Keep S1 scope small and avoid key management work during bring-up.
- Signatures can be introduced in S1.5/S2 with a stable artifact pipeline.

Forward-compatibility:
- Reserve a `signatures` field in the manifest now (empty array).

## 4) Channels (S1)
Decision: Channels are strings in manifest metadata only:
- `Experimental`
- `Candidate`
- `Stable`

No promotion rules in S1. Gates will treat channels as labels only.

## 5) Foundry Gate Expectations (S1)
Gate: a host-only gate that validates artifact hashing and a simulated
install/run/rollback round-trip.

Note: These are bring-up gates, not qualification gates. Foundry qualification
with cryptographic enforcement lands after S1.

Minimal gate steps:
1) Create a blob from a small input file (deterministic content).
2) Compute `sha256:<hex>` and write the manifest.
3) Verify `schema_version == 1`, `size_bytes` matches, and `content_id` == computed hash.
4) Simulate install/run/rollback:
   - install: copy blob + manifest to an "installed" directory
   - run: `store_cli emit-plan --program-id <id>` emits a launch plan referencing the `sha256:<hex>`
   - rollback: remove installed blob and confirm plan fails (or prints expected error)

Outputs are host-only JSON and log lines; no kernel dependencies in S1.

## 6) Minimal Implementation Order
1) `artifact_store_core` (host lib): `hash_blob(path) -> sha256:<hex>` and atomic write helpers
2) Extend `store_cli emit-plan --program-id <id>` to emit blob + manifest and use `artifact_ref` in launch plan.
3) Add `tools/ci/foundry_artifact_s1.sh` gate.
4) Update `runtime_supervisor` to validate `artifact_ref` exists in installed store.

## 6.5) S1.25 — Install Layout Contract (Host-Only)
Define the stable layout and responsibilities:

Roots:
- Build artifacts: `out/artifacts/`
- Installed artifacts: `out/installed/`

Layout (installed root):
- `installed/artifacts/<sha>.blob`
- `installed/artifacts/<sha>.manifest.json`

Write responsibilities:
- Store tooling writes build artifacts.
- Installer/gates copy into installed root.
- Supervisor reads installed root only.

## 7) Non-Goals for S1
- No kernel integration.
- No cryptographic signatures.
- No custom filesystem or bundle formats.
- No policy engine for channels.
