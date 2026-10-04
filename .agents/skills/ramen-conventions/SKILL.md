---
name: ramen-conventions
description: RamenOS architecture invariants and coding patterns. Apply when writing or reviewing kernel, service, or store code.
user-invocable: false
---

Read `AGENTS.md` and `CONSTITUTION.md` for the stable contract; use
`CURRENT_STATUS.md` and `NEXT_TASKS.md` for landed scope and next work.

- Native interfaces are typed Harnesses/Portals defined in IDL. POSIX remains
  compatibility-only; no native ioctl or untyped command escape hatch.
- Brokers decide grants; the kernel validates capabilities on native fast paths.
  Define request authority separately from observable authority at service boundaries.
- Control uses bounded typed messages; bulk data uses shared memory. Check object
  lifetime, consumer ranges, and actual copy costs rather than inferring them.
- No heap allocation in `kernel/`. Architecture code belongs in `kernel/src/arch/`.
  `kernel_api` has no external dependencies; `kernel` permits the recorded `spin`
  exception. New dependencies need an explicit decision.
- Services consume `kernel_api` contracts, never kernel internals. Artifact types
  belong in `artifact_store_schema`; Store I/O ownership stays in the owning layer.
- New contracts go in `idl/harness/`, `idl/portals/`, or `idl/services/` and are
  registered in `tools/ci/run_codegen.sh`. Never hand-edit generated content.
- Write behavior/denial assertions before implementation; deliver a bounded slice
  and real consumer. Keep domain accounting, traces, and grants scoped.
- Obtain the Reference Vault and Oracle traces before hardware code. Derive port
  policy from observed-capability evidence and test it against scenarios.
- Core human interactions must remain usable without a model. Performance,
  containment, metal and readiness claims require matching evidence.
- Run formatting, affected checks, and required integration gates. Record design
  decisions in `DECISIONS.md`; update status/changelog for landed milestones.

Keep guidance here focused on applying the contract; do not duplicate the current
queue or weaken invariants with an informal future exception.
