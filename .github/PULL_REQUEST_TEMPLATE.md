<!--
RamenOS favors small, evidence-bearing slices over large subsystem drops.
See CONTRIBUTING.md and AGENTS.md before opening.
-->

## Summary

<!-- What does this change do, and why? One or two sentences. -->

## Pillar / slice

<!-- OS Core | Driver Foundry | Store Platform | Docs/Org-only -->

## Evidence

<!-- Paste the gate command and its result. Do not overstate QEMU/replay as metal. -->

- Gate command(s):
- Result:
- Evidence level claimed (see EVIDENCE_LEVELS.md):

## Claim-boundary check

- [ ] Does not claim metal graduation, security readiness, or release readiness beyond its evidence.
- [ ] Any native-interface change went through `idl/` + `just codegen` (no hand-edited `*.generated.rs`).
- [ ] Preserves boundaries: kernel ≠ services ≠ store.

## Docs

- [ ] `CURRENT_STATUS.md` / `CHANGELOG.md` updated if a milestone landed.
- [ ] `DECISIONS.md` updated for any design choice or Constitution-affecting change.

## Separation of duties

PRs are opened by `ramen-implementer[bot]` (A2) and approved + merged by a
**different** identity (A3). Outside contributors: a maintainer will review and
merge your change.
