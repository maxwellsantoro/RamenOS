---
name: foundry-gate
description: Run Foundry gates for a specific slice and report results
---

Run the requested Foundry gate (the invocation arguments, if supplied). Resolve
the current recipe from `just --list` and `justfile`, then inspect its script for inputs and evidence
scope. `CURRENT_STATUS.md` maps landed behavior to gates; `AGENTS.md` names required
integration checks. Avoid maintaining a second gate inventory here.

1. Check prerequisites and side effects before running an unfamiliar gate.
   A default gate request does not authorize physical actuation or paid work.
2. In a team, reserve fixed `out/` paths, sockets, build artifacts, and devices
   with the coordinator. Serialize conflicting gates unless their scripts support
   isolated outputs. Do not race another worker or repeat its unchanged result.
3. Run the scoped recipe and inspect assertions plus retained logs. An exit code
   alone is insufficient: distinguish PASS, failure, expected scaffold failure,
   and INCOMPLETE. Missing inputs or skipped assertions are not completed evidence.
4. Return the tested revision/diff, exact command and material environment,
   exit result, evidence paths, failing assertion, and untested scope. Preserve
   logs before another run reuses their path; never retain credentials.
5. Apply `EVIDENCE_LEVELS.md` and the contract's limits. Replay/inventory is not
   live HIL or metal proof. Do not launch unrelated suites; the integration owner
   runs combined checks after reviewing the assembled changes.

Use `docs/FOUNDRY_CI_OPTIMIZATION_V0.md` for the developer loop and compilation
reuse rules. Select focused iteration, packet acceptance and affected integration
checks separately; full preflight is not the default for each edit. Reuse a focused
report only when its relevant source, transitive dependencies, fixtures, features,
command and environment are unchanged. Compilation reuse still runs fresh assertions.
