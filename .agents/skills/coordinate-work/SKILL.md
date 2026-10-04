---
name: coordinate-work
description: Coordinate RamenOS work across simultaneous agents with bounded scopes, contract-first dependencies, independent review, and integrated gate evidence. Use when the user requests a team or parallel agent execution.
---

Read `AGENTS.md`, the status/task pair, and `docs/AGENTIC_WORKFLOW.md`. The workflow
owns dispatch fields, shared-resource rules, handoff evidence, and stop conditions;
do not duplicate the live queue in this skill.

Select ready packets that shorten the requested milestone's critical path. Freeze
shared contracts and write behavior/denial/failure assertions before dispatching
dependent implementations. Assign disjoint writable paths and one owner for
registries, generated outputs, gate resources, and status/history. Use worktrees
only when they improve isolation. Match active work to available implementation,
review, and integration capacity; reuse a freed worker for independent review.

Dispatch the workflow's bounded packet with source paths and a concrete completion
signal. Keep independent workers moving when an external dependency blocks one
packet. Replan contract or scope changes centrally rather than allowing competing
interfaces or overlapping edits.

Review handoffs, integrate accepted changes, run affected combined gates on the
assembled revision, and update authoritative status once. Report delivered
behavior, evidence, remaining prerequisites, and the next ready packets.
Coordination grants no additional approval, merge, release, hardware, spending,
or public-support authority; preserve RamenOrg packet and identity requirements.
