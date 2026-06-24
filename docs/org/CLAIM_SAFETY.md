# RamenOrg Claim Safety

**Last Updated:** 2026-06-23
**Status:** G0 scaffold

Claim safety prevents the organization from saying more than the evidence
supports.

## General Rule

Every claim must have:

- A subject: what changed or what is true.
- A scope: where it is true.
- An evidence level: what kind of proof supports it.
- Evidence refs: logs, gates, traces, docs, measurements, or decisions.
- A claim boundary: what is not being claimed.

## Unsafe Patterns

| Unsafe wording | Safer wording |
|----------------|---------------|
| "S13 is complete" | "S13 QEMU loop is mature; S13.7/S13.8 metal graduation pending PASS/METAL" |
| "Appliance proved target truth" | "Appliance captured target-emitted evidence markers" |
| "Research proves the design" | "Research defines claim boundaries and an evaluation plan" |
| "AI board approved it" | "Role votes approved the proposal with evidence refs" |
| "Offer boundaries prevent leakage" | "Offer boundaries reduce request freedom; leakage requires an ObsContract and measured channels" |
| "A board packet authorizes work" | "A board packet summarizes state; authority comes from bounded work orders and explicit decisions" |
| "Schema/tool refs prove implementation success" | "Schema/tool refs are design evidence; gate, claim, HIL, and release evidence are tracked separately" |

## Research Claims

Research outputs must state whether they are:

- `outline`: problem and scope only.
- `prior-art`: literature and comparison map.
- `doctrine`: model and design claim.
- `prototype-plan`: implementation and gate plan.
- `measured`: evaluation data supports a bounded claim.
- `productized`: implemented behavior with Foundry gate coverage.

Only `measured` or `productized` research may support strong product claims.
