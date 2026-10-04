---
name: ramen-conventions
description: RamenOS architecture invariants and coding patterns. Apply when writing or reviewing kernel, service, or store code.
user-invocable: false
---

Use `AGENTS.md` and `CONSTITUTION.md` for invariants; load the relevant maintained
contract through `docs/INDEX.md`. This skill applies those rules at code boundaries.

Before changing an operation, trace its caller, grant broker, kernel validation,
data objects, and consumer. Check the boundaries the change actually affects:

- **Authority:** distinguish request rights (`Lang`) from returned/retained
  observations (`ObsContract`), including deputy effects, revocation, and work
  that outlives the initiating request. Test denials at the enforcing component.
- **Wire/data:** validate message version, size, reserved fields, object handles,
  offset/length arithmetic, and lifetime at consumption. IDL-generated `repr(C)`
  layouts do not establish portable serialization; follow `idl/tools/README.md`.
- **Ownership:** services use `kernel_api`, not kernel internals. Shared artifact
  types belong in `artifact_store_schema`; IO and publication stay with the Store
  owner. Keep grants, traces, budgets, and accounting tied to their domain.
- **Recovery:** test affected consumers and an unrelated consumer sharing the
  resource when a service stalls, dies, or restarts. Preserve uncertain outcomes;
  do not replay a mutation without its contract's reconciliation rule.
- **Evidence:** distinguish host fixtures, replay, QEMU, and live devices. For
  hardware, begin with the Reference Vault and Oracle trace; for application
  ports, derive the manifest from observed-capability evidence and scenarios.

Report which behavior and boundary the checks establish, with remaining limits.
For concurrent work, honor the dispatch scope and shared-resource ownership in
`docs/AGENTIC_WORKFLOW.md`; send cross-owner changes to the coordinator.
