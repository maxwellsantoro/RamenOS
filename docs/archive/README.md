# Documentation Archive

**Last Updated:** 2026-06-24
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

## Archive Policy

Archive a document when all of the following are true:

1. Its implementation or investigation is complete or superseded.
2. It is not a living contract or operational guide.
3. No Foundry gate or generated governance artifact requires its current path.
4. Inbound links can be updated without obscuring current authority.

Git history is not a substitute for clear navigation, and the archive is not a
second backlog.
