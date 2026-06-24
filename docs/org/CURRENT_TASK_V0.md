# CurrentTaskV0

**Last Updated:** 2026-06-23
**Status:** G0.8 scaffold

`CurrentTaskV0` is the machine-readable source for the active RamenOrg packet
set. The renderer reads `docs/org/current_task.yaml`; the governance gate
validates it against `schemas/org/current_task_v0.schema.json` before rendering.

It is not an authority grant. Authority still comes from bounded work orders and
explicit decisions.

## Validation Rules

- `schema_version` must be `1`.
- `authority_level` may be `A0`, `A1`, or bounded `A2-local` represented as
  `A2`; A2-local permits code and gate work only inside the active work order
  and does not permit merge, release, self-approval, HIL actuation, or public
  support authority.
- Context, claim, and evidence refs must exist.
- `context_grant_refs` must name existing files selected for the bounded grant.
- Gate refs must be `just <recipe>` or an existing executable-style repo path.
- The claim boundary must explicitly include all four denials: no merge, no
  release, no HIL actuation, and no public support authority.
- Any `PASS/METAL` claim level requires HIL evidence refs.
