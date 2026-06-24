# BoardBriefV0

**Last Updated:** 2026-06-23
**Status:** G0.8.1 scaffold

`BoardBriefV0` is a generated Markdown brief derived from the validated
RamenOrg packet set. It exists for humans and agents that need a concise
handoff view before starting the next bounded task.

It is read-only. It grants no authority beyond the referenced work order and
does not replace packet validation.

## Output

```text
out/org/current_board_brief.md
```

## Required Sections

- Active Task
- Intake Binding
- Authority Boundary
- Required Gates
- Context Refs
- Granted Context
- Not Granted / Out of Scope
- Evidence Refs
- Allowed Next-Agent Actions

## Generation Rules

- The governance gate must validate packets before rendering the brief.
- The brief renderer must reject a non-passing validation report.
- The brief renderer must verify every checked artifact against the SHA-256
  ledger in the validation report before reading packet contents.
- The brief must cite the board packet, work order, handoff, and vote refs.
- The brief must cite validation status, validation report, current-task ref,
  and repo SHA.
- The renderer must emit `out/org/intake_manifest.json` for the verified bundle.
- The brief must list the hash-bound context grant and distinguish granted files
  from unavailable or out-of-scope repository state.
- The brief must preserve the claim boundary: no merge, no release, no HIL
  actuation, and no public support authority.
- G0.8.1 is A2-local only and grants no merge, release, self-approval, HIL
  actuation, or public support authority.
