# G0.7 Bounded Context Grant V0

**Date:** 2026-06-23
**Status:** Implemented
**Authority:** A0/A1 only

## Purpose

Make the portable intake explicitly sufficient for a patch plan without granting
arbitrary repository access. Context is selected by path, bound by SHA-256, and
classified as read-only or patch-authorized from the existing work-order scope.

## Definition of Done

1. Add `ContextGrantV0` documentation and schema.
2. Add `context_grant_refs` to `CurrentTaskV0` and the active task source.
3. Generate and validate a hash-bound context grant for the narrow S12.4.1 files.
4. Bind the grant and every granted file in `IntakeManifestV0`.
5. Add Granted Context and Not Granted / Out of Scope sections to the board brief.
6. Reject unhashbound, missing, changed, out-of-scope patch, and incomplete required context.
7. Run a fresh-agent intake trial at `PASS/PATCH-PLAN`; perform no implementation.
8. Preserve A0/A1 and all four authority denials.

## Result

G0.7 generated and validated a bounded context grant with eight hash-bound input
files plus one authorized new output path. The first trial correctly asked for
expansion when the new output path was not modeled. After adding
`authorized_new_paths`, the fresh-agent trial passed as `PASS/PATCH-PLAN` with
no hidden chat, no external file reads, no expansion request, and no
implementation.

## Gate

```text
just foundry-org-governance-g0
```
