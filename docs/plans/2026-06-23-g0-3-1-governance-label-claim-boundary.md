# G0.3.1 Governance Label and Claim-Boundary Hygiene

**Last Updated:** 2026-06-23
**Status:** Scaffold

## Purpose

G0.3 made the active-task source schema-valid and added negative validator
fixtures. G0.3.1 is a small hygiene slice that keeps the generated governance
surface honest:

- Generated packet claim text and validator diagnostics identify the current
  G0.3.1 scaffold boundary instead of stale G0.1/G0.2 labels.
- `claim_boundary` must explicitly deny each authority expansion: no merge, no
  release, no HIL actuation, and no public support authority.
- `PASS/METAL` claims require HIL evidence refs.

Authority remains A0/A1 only.

## Deliverables

- Update `docs/org/current_task.yaml` labels and claim-boundary wording.
- Update renderer vote claim text.
- Update validator diagnostics and claim-boundary checks.
- Add negative validator cases for:
  - missing release/public-support denial
  - `PASS/METAL` without HIL evidence refs

## Non-Scope

- No G0.4 steward heartbeat implementation.
- No merge, release, HIL actuation, public support, credentials, or
  identity-level role separation authority.

## Gate

```bash
just foundry-org-governance-g0
```
