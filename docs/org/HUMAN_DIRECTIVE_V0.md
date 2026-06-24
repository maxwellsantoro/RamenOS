# HumanDirectiveV0

**Last Updated:** 2026-06-23
**Status:** G0 scaffold

A `HumanDirectiveV0` is the typed, hashed, authority-scoped artifact that
carries a human founder's vision injection or escalation into RamenOrg. It is
the replacement for "the founder pastes an idea into chat and an agent acts on
ambient context."

## Why it exists

RamenOrg is built to run with bounded agent context (see G0.5–G0.7 intake and
context-grant trials). If a founder's idea enters as free chat, it leaks
unbounded context and silently becomes authority. `HumanDirectiveV0` makes the
injection a first-class, citable object instead: it is hashed into the intake
manifest, it is referenced by the board packet that turns it into work, and its
claim boundary states what it does **not** authorize.

## Shape

- `directive_id`: stable id, e.g. `HD-2026-06-23-proceed-with-all`.
- `repo_sha`: the repo state the directive was issued against.
- `from_role`: always `Founder/Vision Channel`.
- `authority`: one of `vision_input`, `escalation`, `priority_change`.
- `directive`: the actual injected idea or instruction.
- `proposal_target`: `board` — a directive never directly authorizes work; the
  board turns it into a `WorkOrderV0` through the normal packet path.
- `constraints`: boundaries the board must preserve when acting on it.
- `claim_boundary`: what the directive does **not** grant.

## Claim boundary (default)

A human directive grants **vision input only**. It does not, by itself:

- merge or release anything,
- grant an agent authority above its current ladder rung,
- bypass a required review, vote, or evidence check,
- actuate hardware, or
- make a public support claim.

A directive is privileged as vision; it is **not** automatically merged. The
board may reject, defer, or reshape it, and must record that decision.

## Validation

`tools/org/validate_human_directive.py` checks schema shape, that the directive
references real repo state, and that the claim boundary preserves the default
denials. See also `docs/org/CLAIM_SAFETY.md`.
