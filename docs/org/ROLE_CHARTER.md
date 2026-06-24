# RamenOrg Role Charter

**Last Updated:** 2026-06-23
**Status:** G0 scaffold

Roles are capability scopes, not personalities. A role may be filled by an
agent, a human, or a mixed review loop, but the authority and veto domain stay
the same.

| Role | Primary authority | Veto domain |
|------|-------------------|-------------|
| Project Chair | Slice priority, roadmap coherence, public narrative drafts | Strategically incoherent work |
| Kernel Architect | Constitution, TCB boundaries, IDL shape, kernel/service/store separation | Architecture violations |
| Foundry Evidence Officer | Gate truth, evidence levels, claim safety | Unsupported completion or graduation claims |
| Security Officer | Ambient authority, prompt injection, supply chain, privilege boundaries | Unsafe authority or release surfaces |
| HIL Lab Officer | Golden machine, HIL appliance, serial/power/reset evidence | Unsafe or unsupported hardware actuation |
| Driver Dossier Curator | Reference Vaults, Oracle traces, datasheets, replay artifacts | Driver work without dossier evidence |
| Research Office | Doctrine-level unknowns, prior art, claim boundaries, paper pipeline | Shallow implementation where research is required |
| Release Manager | Status/changelog sync, release notes, public claim levels | Releases whose claims exceed evidence |
| Community Lead | Issues, questions, onboarding drafts, support response drafts | Unsupported public support claims |
| Cost/Compute Steward | Token, CI, hardware, cloud, and lab spend | Runaway or unbudgeted loops |

## Separation Rule

No role should author, verify, approve, merge, and announce the same change.
Small documentation corrections can be fast-tracked, but completion, security,
release, hardware, and research claims require independent evidence review.

## Role Outputs

Roles emit artifacts, not only prose:

- Project Chair: roadmap deltas, board packets, priority decisions.
- Kernel Architect: architecture review notes, boundary objections.
- Foundry Evidence Officer: gate summaries, evidence-level validation.
- Security Officer: authority reviews, threat notes, block conditions.
- HIL Lab Officer: appliance run packets, safety checks, provenance summaries.
- Driver Dossier Curator: vault readiness notes, Oracle trace requirements.
- Research Office: research questions, prior-art packets, claim matrices.
- Release Manager: changelog/status diffs, release claim packets.
- Community Lead: response drafts with evidence refs.
- Cost/Compute Steward: budget observations and stop conditions.
