# RamenOrg Heartbeats

**Last Updated:** 2026-06-23
**Status:** G0 scaffold

Heartbeats are recurring or manually invoked loops that emit artifacts. They are
not chats and do not own truth.

| Heartbeat | Reads | Emits |
|-----------|-------|-------|
| Steward | `CURRENT_STATUS.md`, `NEXT_TASKS.md`, `ROADMAP.md`, `AGENTS.md`, recent commits | `out/org/daily_status.md`, `out/org/status_drift.json` |
| Slice | Active task, plans, evidence policy, risks | `WorkOrderV0` for the next executable task |
| Foundry | Work order, gates, evidence level policy | Gate log summary and evidence references |
| Security | Authority surfaces, tool scopes, repo diffs, dependencies | Security review packet and block conditions |
| HIL | Appliance manifest, HIL work order, serial/power/reset policy | Appliance evidence wrapper and safety summary |
| Research | Research questions, prior art, product risks, design claims | Prior-art packet, claim matrix, paper/update plan |
| Board | Work orders, reviews, evidence packets, votes | Board packet and decision summary |
| Release | Status docs, changelog, evidence refs, public claims | Release claim packet and changelog/status sync |
| Community | Issues, docs, evidence refs, release claims | Draft responses and support triage summary |

## Heartbeat Rules

- Each heartbeat must declare its input files and output artifacts.
- A heartbeat may propose work but does not automatically authorize it.
- A heartbeat that observes doc drift must report it before downstream planning.
- Hardware heartbeats require HIL authority and must not replace target-emitted
  evidence with controller observations.
- Research heartbeats must keep an implementation landing path attached to every
  doctrine-level question.
