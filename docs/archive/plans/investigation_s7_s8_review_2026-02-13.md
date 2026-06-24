# Investigation: S7 Hardening / S8 Transition Security Review Validation

**Last Updated:** 2026-02-13

## Summary
The review is directionally correct on core architecture risks, but several findings are even worse in practice: the POSIX sandbox path is largely non-functional (seccomp + rlimits are no-ops, chroot wrapping is malformed), S7 security gates are currently failing, and evidence/status docs overstate enforcement.

**Tooling note:** The requested `context_builder` workflow was attempted, but no callable `context_builder` endpoint was available in this session (CLI command unavailable; MCP server handshake timed out). Investigation proceeded with direct source + gate evidence collection.

## Symptoms
- Claimed S7 hardening (sandbox + fail-closed policies) appears inconsistent with executable behavior.
- Foundry S7 security gates are expected to provide measurable evidence, but current runs fail.
- Kernel still relies on `static mut` globals across allocator/trace/init paths with no synchronization primitives.

## Investigation Log

### 2026-02-13T02:11:18Z / Phase 1 - Initial assessment
**Hypothesis:** The provided review is mostly accurate but may contain stale line references.
**Findings:** Review themes map to active code areas (`runtime_supervisor`, `store_service`, `kernel/mm`, `domain_manager`). Some referenced line numbers are stale but components exist.
**Evidence:** `CONSTITUTION.md:7-11`, `kernel/src/ipc_v0.rs:42-60`, `kernel_api/src/lib.rs:157-171`
**Conclusion:** Confirmed relevance; proceed with source-level validation.

### 2026-02-13T02:14:00Z / Phase 2 - POSIX sandbox deep dive
**Hypothesis:** Sandbox advertises stronger controls than implemented.
**Findings:**
- `apply_seccomp_filter` is a documented stub returning `Ok(())`.
- `apply_resource_limits` is also a stub returning `Ok(())`.
- `apply_chroot` appends args to `sh` command instead of wrapping process execution; this does not apply chroot and can break execution.
**Evidence:** `runtime_supervisor/src/sandbox.rs:181-187`, `runtime_supervisor/src/sandbox.rs:241-288`, `runtime_supervisor/src/sandbox.rs:314-318`, `runtime_supervisor/src/posix_runner.rs:135-160`
**Conclusion:** Confirmed critical gap. “Sandbox enabled” logs do not imply seccomp/rlimits/chroot enforcement.

### 2026-02-13T02:18:30Z / Phase 2b - Runtime policy/env parsing checks
**Hypothesis:** Runtime kill-switch/dev-mode knobs may not parse documented values.
**Findings:**
- `RAMEN_POSIX_RUNNER_ACK_RISK` and `RAMEN_POSIX_RUNNER_DISABLE_SANDBOX` are parsed as `bool`, but docs/gates use `=1`.
- `RAMEN_STORE_DEV_MODE` parsed as `bool` too; `=1` is treated as false.
- `runtime_supervisor/main.rs` uses `u8==1` parsing for ack, creating cross-file inconsistency.
**Evidence:** `runtime_supervisor/src/posix_runner.rs:61-64`, `runtime_supervisor/src/posix_runner.rs:91-94`, `services/store_service/src/main.rs:156-159`, `runtime_supervisor/src/main.rs:115-120`
**Conclusion:** Confirmed; documented env usage and implementation are inconsistent.

### 2026-02-13T02:22:10Z / Phase 3 - Foundry gate evidence validation
**Hypothesis:** S7 gates pass and provide enforceable proof.
**Findings:**
- Combined gate fails end-to-end.
- POSIX gate fails at build due unstable `str::as_str` use on `&str`.
- Store/access gates fail because `RAMEN_STORE_DEV_MODE=1` does not activate dev mode.
- POSIX gate script has a variable typo (`EVIDENCE_dir`) in redirection path.
**Evidence:**
- Run: `bash tools/ci/foundry_s7_all_security.sh` (failed)
- Log: `out/evidence/s7_posix_runner/build.log:16-34`
- Log: `out/evidence/s7_store_signature/test2_dev_mode.log:5-9`
- Log: `out/evidence/s7_access_control/test1_default_policy.log:5-9`
- Script typo: `tools/ci/foundry_s7_posix_runner_security.sh:60`
**Conclusion:** Confirmed major evidence-discipline gap: gates currently do not substantiate S7 claims.

### 2026-02-13T02:25:40Z / Phase 4 - Kernel synchronization / SMP readiness
**Hypothesis:** Kernel uses single-thread assumptions that block SMP hardening.
**Findings:**
- `FRAME_ALLOCATOR` and `ADDRESS_SPACE_TABLE` remain `static mut`.
- Shared-memory paths mutate allocator via unsafe globals.
- `cap_table` has SMP panic guard, but no equivalent global synchronization layer for mm/trace/init globals.
- Global `#![allow(static_mut_refs)]` remains active.
**Evidence:** `kernel/src/mm/mod.rs:89-101`, `kernel/src/shmem.rs:172-175`, `kernel/src/lib.rs:2`, `docs/LINT_DEBT.md:11-15`
**Conclusion:** Confirmed high-priority prerequisite for SMP and isolation integrity.

### 2026-02-13T02:29:20Z / Phase 4b - Positive control checks (sound implementations)
**Hypothesis:** Some review positives should be retained.
**Findings:**
- Ring buffer caches shared capacity at construction (TOCTOU mitigation).
- Capability generation counter wraps while skipping 0.
- Kernel-side capability validation exists on IPC/shmem control path.
**Evidence:** `kernel_api/src/ring_buffer.rs:160-163`, `kernel/src/cap_table.rs:178-181`, `kernel/src/ipc_v0.rs:42-60`
**Conclusion:** Confirmed; these are solid foundations.

### 2026-02-13T02:32:00Z / Phase 4c - Domain manager schema drift check
**Hypothesis:** Local type redefinitions may drift from artifact schema contracts.
**Findings:**
- `domain_manager` defines local trace/observed-caps structs instead of using `artifact_store_schema`.
- Similar shapes exist in schema crate and are reused by other services.
- Domain manager path does not invoke schema validators before writing artifacts.
**Evidence:** `services/domain_manager/src/main.rs:22-124`, `artifact_store_schema/src/trace.rs:25-99`, `artifact_store_schema/src/observed_caps.rs:17-47`, `services/domain_manager/src/main.rs:1-2`
**Conclusion:** Confirmed medium risk (contract drift / weaker validation discipline).

## Root Cause
The primary root cause is **verification drift** between security intent, implementation, and gates:
1. Security controls are documented as enforced, but key enforcement functions are stubs (`seccomp`, `rlimits`) or malformed (`chroot` wrapping).
2. Gate scripts rely heavily on grep/log assertions and currently fail under real execution paths.
3. Environment policy parsing is inconsistent (`bool` vs `u8`) and conflicts with documented `=1` usage.
4. Kernel concurrency hardening is acknowledged as debt but not yet converted into synchronization primitives required for SMP-safe operation.

## Eliminated Hypotheses
- **“Kernel-side capability checks are missing entirely.”** Eliminated: checks are present in IPC/shmem control dispatch (`kernel/src/ipc_v0.rs:42-60`).
- **“Store service is fail-open by default.”** Eliminated in production path: without trusted keys and without dev mode, startup aborts (`services/store_service/src/main.rs:235-248`).
- **“Ring buffer still has live capacity TOCTOU.”** Eliminated: capacity is cached once at construction (`kernel_api/src/ring_buffer.rs:160-163`).

## Recommendations
1. **Fail closed on sandbox feature gaps immediately**
   - If seccomp/rlimits/chroot cannot be applied, return error and refuse execution.
   - Remove/replace misleading “Sandbox: ENABLED” semantics with per-control outcome logging.
2. **Unify env var parsing contract**
   - Adopt one parser helper accepting `1/0`, `true/false`, `yes/no` consistently across `runtime_supervisor` and `store_service`.
3. **Repair S7 Foundry gates before claiming S7 complete**
   - Fix compile blocker (`content_id.as_str()` calls on `&str`).
   - Fix script variable typo (`EVIDENCE_dir`).
   - Convert grep-only checks into behavior tests with explicit pass/fail criteria and captured artifacts.
4. **Prioritize kernel sync primitives for S8/SMP path**
   - Introduce `Spinlock<T>`/`RawSpinlock` wrappers for mm/trace/init globals.
   - Remove `#![allow(static_mut_refs)]` incrementally with targeted gate coverage.
5. **Schema contract consolidation**
   - Prefer `artifact_store_schema` shared types + validators in `domain_manager` (or generate a dedicated shared crate) to prevent drift.

## Preventive Measures
- Add a **Security Control Truth Table** gate that asserts each claimed control has executable enforcement (not just symbol presence).
- Require every security phase claim in `CURRENT_STATUS.md`/`SECURITY_STATUS.md` to link to at least one reproducible passing gate log.
- Add CI job matrix for `runtime_supervisor` with `--features posix_runner_v0_dev` to prevent feature-rot.
- Add “doc/code parity” checks for environment variable semantics and security defaults.
