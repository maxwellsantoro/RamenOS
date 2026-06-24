# G0.5 Agent Intake Bundle and Freshness Binding

**Date:** 2026-06-23
**Status:** Implemented
**Authority:** A0/A1 only

## Purpose

G0.5 makes the G0.4 board brief portable without allowing a stale validation
result to attest to changed packet state. The packet validation report becomes
a SHA-256 ledger, and the generated intake manifest binds the brief, packet set,
current task source, and validation report into one self-checking bundle.

## Definition of Done

1. Packet validation reports SHA-256 for every checked packet and the current-task source.
2. The board brief renderer rejects missing, changed, or unvalidated intake artifacts.
3. The brief cites validation status, validation report, current task, packet refs, and repo SHA.
4. `out/org/intake_manifest.json` records paths and hashes for the complete intake bundle.
5. The governance gate validates manifest structure, hashes, packet relationships, and brief citations.
6. A negative test proves a stale pass report cannot render a changed packet set.
7. Authority remains A0/A1 only.

## Gate

```text
just foundry-org-governance-g0
```

The gate remains a project-control check. It grants no merge, release, HIL
actuation, public support, credential, or identity-level role authority.
