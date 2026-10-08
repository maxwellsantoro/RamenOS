# Documentation Archive

**Last Updated:** 2026-10-08
**Status:** Historical and non-authoritative

This directory preserves completed or superseded plans, designs, and
investigations for traceability. Archived material may describe old paths,
commands, risks, or sequencing and must not be used as current project truth.

## Source of Truth

- Landed state: [CURRENT_STATUS.md](../../CURRENT_STATUS.md)
- Execution order: [NEXT_TASKS.md](../../NEXT_TASKS.md)
- Direction: [ROADMAP.md](../../ROADMAP.md)
- Decisions: [DECISIONS.md](../../DECISIONS.md)
- Chronology: [CHANGELOG.md](../../CHANGELOG.md)

## Layout

- [`plans/`](plans/): completed slice plans, superseded designs, and historical
  investigations.

Files keep descriptive, date-prefixed names where practical. Links into the
archive are welcome when historical rationale matters, but new implementation
work should cite a maintained contract, decision, or active plan as well.

The [pre-review status snapshot](plans/2026-10-06-status-integration-snapshot.md)
retains detailed host-editor integration chronology and original evidence identities
removed from routine intake on October 8. Its pending statements are historical.

## Archive Policy

Archive a document when all of the following are true:

1. Its implementation or investigation is complete or superseded.
2. It is not a living contract or operational guide.
3. No Foundry gate or generated governance artifact requires its current path.
4. Inbound links can be updated without obscuring current authority.

Git history is not a substitute for clear navigation, and the archive is not a
second backlog.

Do not load archived implementation recipes into routine agent intake. The
coordinator links a specific historical section only when it explains a current
decision or regression. Copy actionable remaining work into the maintained queue
with current prerequisites and acceptance; old TODOs do not authorize execution.

When a gate or runtime warning requires an old path, keep a concise current
reference there and archive the superseded analysis here. Preserve original
dates and results; an archive banner does not renew a claim or authorize work.

Some G0 plans and trial reports remain outside this directory because gates bind
their exact paths. Their historical banners give them the same non-authoritative
status. A maintained architecture reference stays active while its contract is
used, even when its implementation milestones are complete; it should point to
current owners instead of repeating the backlog.
