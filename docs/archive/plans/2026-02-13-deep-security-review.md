# Investigation: Security Review Reconciliation (S7/S8)

**Last Updated:** 2026-02-13

## Summary
The review is directionally correct: several security claims in docs/gates overstate enforcement. Highest-confidence issues are (1) POSIX sandbox controls are mostly stubs/misapplied, (2) store signature enforcement is asymmetric across APIs, and (3) store capability key configuration is internally inconsistent. Kernel-side capability validation and typed IDL contracts are present, but SMP-era safety and some SHMEM/trace correctness gaps remain.

## Symptoms
- POSIX sandbox claims (seccomp/chroot/rlimits) appeared stronger than runtime behavior.
- Store "fail-closed" signature/capability behavior appeared inconsistent across code paths.
- Kernel `static mut` concerns for SMP transition, plus SHMEM correctness concerns.
- Possible schema drift between `domain_manager` local types and canonical schema crate.
- Foundry evidence quality concerns (grep-based assertions, env mismatch, script defects).

## Investigation Log

### 2026-02-13 / Phase 2 - Context Builder discovery
**Hypothesis:** The submitted review mixes true vulnerabilities, operational fail-closed bugs, and roadmap debt.
**Findings:** Context builder selected 43 files across runtime_supervisor, store_service, kernel, kernel_api, gate scripts, and status/docs.
**Evidence:** `context_builder` chat `security-claims-audit-58B7C9`.
**Conclusion:** Proceeded with targeted code+history verification.

### 2026-02-13 / Phase 3 - Oracle follow-up deep dive
**Hypothesis:** Main breakpoints are sandbox enforcement and store policy coherence.
**Findings:** Oracle flagged sandbox no-op paths, signature-policy asymmetry, capability key mismatch/fallback, SHMEM leak/kind-check gaps, and evidence-gate drift.
**Evidence:** `ask_oracle` follow-up in chat `security-claims-audit-58B7C9`.
**Conclusion:** Required direct source-level confirmation.

### 2026-02-13 / Phase 4A - POSIX sandbox enforcement audit
**Hypothesis:** Seccomp/rlimit/chroot protections are not actually enforced.
**Findings:**
- `apply_resource_limits()` is stubbed and returns `Ok(())`.
- `apply_seccomp_filter()` is stubbed/documentational and returns `Ok(())`.
- `apply_chroot()` appends `cmd.arg("chroot")`/`cmd.arg(chroot_dir)` to a `Command::new("sh")` pipeline instead of invoking `chroot(2)`/wrapper command semantics safely.
- `posix_runner` logs `Sandbox: ENABLED` before proving effective enforcement.
**Evidence:**
- `runtime_supervisor/src/sandbox.rs:183-187`, `:236-288`, `:321-324`.
- `runtime_supervisor/src/posix_runner.rs:118-160`.
- `runtime_supervisor/POSIX_RUNNER_SECURITY.md:7-21` claims full sandbox completion.
**Conclusion:** **Confirmed** security-theater risk (fail-open-by-illusion), with additional fail-closed-by-breakage risk from malformed chroot wrapping.

### 2026-02-13 / Phase 4B - Runtime correctness and buildability checks
**Hypothesis:** POSIX runner paths may not currently compile/run as claimed by gates/docs.
**Findings:** `runtime_supervisor` with `posix_runner_v0_dev` fails to compile due unstable `str::as_str` usage on `&str`.
**Evidence:**
- `runtime_supervisor/src/posix_runner.rs:213`, `:284` (`content_id.as_str()`).
- `cargo check -p runtime_supervisor --features posix_runner_v0_dev` emits `E0658 str_as_str`.
**Conclusion:** **Confirmed**: current branch has a build blocker on this feature path.

### 2026-02-13 / Phase 4C - Store signature/capability policy reconciliation
**Hypothesis:** Signature and capability trust paths are inconsistent/fail-open.
**Findings:**
- `GetManifest` enforces signature validation via `validate_manifest_signatures(...)`.
- `GetBlob` and `VerifyArtifact` do not perform signature validation; `VerifyArtifact` checks blob↔manifest consistency only.
- `extract_and_validate_capability()` always enforces `capability.verify_signature()`, but trusted key loading for capabilities (`capability.rs`) interprets `RAMEN_STORE_TRUSTED_KEYS` as base64 list and falls back to RFC test key if unset.
- `store_service/main.rs` interprets `RAMEN_STORE_TRUSTED_KEYS` as a file path for manifest verification.
**Evidence:**
- `services/store_service/src/main.rs:580-588` (manifest signature validation), `:753+` (GetBlob), `:909+` and `:1021-1031` (VerifyArtifact), `:64-88` (capability validation), `:156-159` (dev mode bool parse).
- `services/store_service/src/capability.rs:52-107` (base64 env parsing + default key fallback).
**Conclusion:** **Confirmed** policy mismatch. This is both security and availability risk depending on deployment mode.

### 2026-02-13 / Phase 4D - Env semantics and Foundry evidence discipline
**Hypothesis:** Gates/docs using `=1` are misaligned with runtime parsing.
**Findings:**
- Runtime parses `RAMEN_POSIX_RUNNER_ACK_RISK` and `RAMEN_POSIX_RUNNER_DISABLE_SANDBOX` with `parse::<bool>()` in `posix_runner`, while startup warning suppression in `main.rs` parses ACK as `u8==1`.
- Store `RAMEN_STORE_DEV_MODE` also uses `parse::<bool>()` while docs/gates use `=1`.
- `foundry_s7_posix_runner_security.sh` has `$EVIDENCE_dir` typo and greps for log claims rather than proving syscall restrictions.
- `foundry_s7_store_signature_security.sh` passes `--socket/--store-root` args to `store_service`, but store_service config is env-var based (no CLI parser in crate).
**Evidence:**
- `runtime_supervisor/src/posix_runner.rs:61-64`, `:91-94`; `runtime_supervisor/src/main.rs:116-119`.
- `services/store_service/src/main.rs:156-159`; `services/store_service/Cargo.toml` (no `clap`).
- `tools/ci/foundry_s7_posix_runner_security.sh:57` and broader script body.
- `tools/ci/foundry_s7_store_signature_security.sh` startup invocations.
**Conclusion:** **Confirmed** measurable evidence drift; several gates assert behavior not reliably exercised.

### 2026-02-13 / Phase 4E - Kernel SHMEM/SMP/trace deep dive
**Hypothesis:** Kernel has transitional SMP debt plus concrete SHMEM/trace correctness defects.
**Findings:**
- `kernel/src/mm/mod.rs` exposes `static mut FRAME_ALLOCATOR` and `ADDRESS_SPACE_TABLE` with documented single-thread assumptions (no locking).
- SHMEM `create_region()` leaks already-allocated frames on mid-allocation failure (no rollback before `STATUS_NO_MEMORY`).
- SHMEM `map_region()` validates index+generation but does not validate `shm_cap.kind` (despite `validate_cap()` implementing kind checks).
- Trace ring publish path increments `write_idx` via `fetch_add(Ordering::Release)` *before* writing event data.
- `cap_table` still correctly enforces kernel-side validation and generation hardening with SMP panic guards.
**Evidence:**
- `kernel/src/mm/mod.rs:92-107`.
- `kernel/src/shmem.rs:168-188`, `:247-250`, `:389-402`.
- `kernel/src/trace_ring.rs:124-132`, `:376-379`.
- `kernel/src/cap_table.rs:57-67`, `:126-177`, `:179-182`.
- `kernel/src/ipc_v0.rs:49-59`.
**Conclusion:** Mixed result: **confirmed** concrete SHMEM/trace defects plus **confirmed** SMP-transition debt; kernel-side cap validation invariant remains enforced on IPC handles.

### 2026-02-13 / Phase 4F - Schema drift and boundary check
**Hypothesis:** Domain manager duplicates canonical schema types and can drift.
**Findings:** `domain_manager` defines local `TraceArtifactV0`/`ObservedCapsV0` and explicitly states no dependency on `artifact_store_schema`; canonical validators exist in `artifact_store_schema`.
**Evidence:**
- `services/domain_manager/src/main.rs:22`, `:86`, `:116`.
- `artifact_store_schema/src/trace.rs:25`, `:101`.
- `artifact_store_schema/src/observed_caps.rs:17`, `:49`.
**Conclusion:** **Confirmed** schema drift risk (integrity/maintenance risk; security impact depends on downstream trust decisions).

### 2026-02-13 / Phase 4G - Git history reconciliation
**Hypothesis:** Some issues are regressions against prior hardening work.
**Findings:**
- Recent commit `9d609de` correctly fixed ring-buffer capacity TOCTOU.
- Prior commit `f3fc4d2` hardened trace publish order, but current `trace_ring.rs` uses publish-before-write pattern in newer paths/changes.
- Most problematic sandbox/capability patterns originated in S7/S9-era commits (`0007110`, `adb2cdf`, `f639b19`).
**Evidence:**
- `git log` + `git blame` on `sandbox.rs`, `posix_runner.rs`, `store_service/main.rs`, `capability.rs`, `trace_ring.rs`.
- `git show f3fc4d2 -- kernel/src/trace_ring.rs`.
**Conclusion:** Combination of transitional debt and likely regressions/documentation drift.

### 2026-02-13 / Phase 5 - Immediate hardening + evidence fixes applied
**Hypothesis:** A narrow first patch can reduce evidence drift and unblock follow-up remediation.
**Findings:**
- Added boolish env flag parsing (`1/0/true/false/yes/no/on/off`) in runtime_supervisor and store_service startup paths.
- Fixed `runtime_supervisor` feature-path build break by replacing invalid `content_id.as_str()` calls on `&str`.
- Corrected Foundry script typo (`$EVIDENCE_dir` → `$EVIDENCE_DIR`).
- Corrected store signature gate to configure `store_service` using environment variables actually consumed by code (`RAMEN_STORE_SOCKET`, `RAMEN_STORE_ROOT`) instead of unsupported CLI flags.
- Applied the same env-based invocation fix to the S7 access-control gate and repaired a shell syntax defect (`}` -> `fi`) in its audit module check.
**Evidence:**
- `runtime_supervisor/src/posix_runner.rs` (env parsing + `as_str` fixes).
- `runtime_supervisor/src/main.rs` (ACK parsing alignment).
- `services/store_service/src/main.rs` (dev mode parsing alignment).
- `tools/ci/foundry_s7_posix_runner_security.sh`, `tools/ci/foundry_s7_store_signature_security.sh`, `tools/ci/foundry_s7_access_control_security.sh`.
- Validation: `cargo check -p runtime_supervisor --features posix_runner_v0_dev`, `cargo check -p store_service`, `bash -n` on updated S7 gate scripts.
**Conclusion:** **Confirmed** first-pass remediation is in place for semantics/evidence consistency and compileability; core sandbox architectural fixes remain outstanding.

### 2026-02-13 / Phase 5B - Store signature enforcement parity patch
**Hypothesis:** `GetBlob`/`VerifyArtifact` can be upgraded to enforce the same signature policy as `GetManifest` without breaking existing capability/access checks.
**Findings:**
- Wired `sig_config` through message dispatch into both `handle_get_blob(...)` and `handle_verify_artifact(...)`.
- Added manifest read+parse+`validate_manifest_signatures(...)` in both paths.
- Both operations now return `STATUS_VALIDATION_FAILED` on non-`Valid` signature policy results before serving blob paths or blob/hash verification outcomes.
- Added `manifest_path` existence requirement to `GetBlob` to avoid serving data without policy-evaluable metadata.
**Evidence:**
- `services/store_service/src/main.rs` (`handle_client`, `handle_get_blob`, `handle_verify_artifact`).
- Validation: `cargo check -p store_service`.
**Conclusion:** **Confirmed** signature-policy asymmetry for artifact reads/verify is remediated.

### 2026-02-13 / Phase 5C - Capability trusted-key loading reconciliation
**Hypothesis:** Capability verification key loading can be made fail-closed/coherent while preserving explicit dev-mode fallback.
**Findings:**
- Added explicit parser helpers in `capability.rs` for bool env flags, base64 key lists, and file-based trusted-key loading.
- `load_trusted_public_keys()` now resolves in strict order:
  1. `RAMEN_STORE_CAP_TRUSTED_KEYS` (base64 list)
  2. `RAMEN_STORE_TRUSTED_KEYS` (file if path exists, else backward-compatible inline base64 parse)
  3. fallback default test key only in dev mode / tests.
**Evidence:**
- `services/store_service/src/capability.rs`.
- Validation: `cargo test -p store_service --lib`.
**Conclusion:** **Confirmed** key-source mismatch reduced and production fallback behavior tightened.

### 2026-02-13 / Phase 5D - Regression tests and gate-level validation
**Hypothesis:** New signature-enforcement behavior requires explicit regression coverage on blob/verify paths.
**Findings:**
- Added binary integration tests:
  - `get_blob_validation_failed_when_signature_required_and_manifest_unsigned`
  - `verify_artifact_validation_failed_when_signature_required_and_manifest_unsigned`
- Both tests use signed capabilities + registered ownership to exercise the signature-policy branch directly and assert `STATUS_VALIDATION_FAILED`.
- Revalidated S7 gate scripts for syntax and reran store_service binary/lib tests.
**Evidence:**
- `services/store_service/src/main.rs` tests module.
- `cargo test -p store_service --bin store_service -q` → 18 passed.
- `cargo test -p store_service --lib -q` → 56 passed.
- `bash -n tools/ci/foundry_s7_posix_runner_security.sh tools/ci/foundry_s7_store_signature_security.sh tools/ci/foundry_s7_access_control_security.sh`.
**Conclusion:** **Confirmed** signature-read/verify regressions are now guarded by executable tests.

### 2026-02-13 / Phase 5E - Foundry gate determinism remediation (follow-up)
**Hypothesis:** S7 gates still contain misleading pass conditions and summary-generation defects despite earlier script fixes.
**Findings:**
- Reworked `foundry_s7_posix_runner_security.sh` to validate POSIX security enforcement through deterministic `runtime_supervisor` unit tests (kill-switch, sandbox-disabled warning, ACK success, sandbox default) instead of `runtime_supervisor` binary startup paths that depend on store connectivity/capabilities.
- Added env-var serialization (`env_lock`) and a dedicated `posix_run_v0_allows_execution_when_sandbox_disabled` test in `runtime_supervisor/src/posix_runner.rs` to stabilize and directly exercise warning-path behavior.
- Fixed `foundry_s7_store_signature_security.sh` trusted-key setup to use an explicit line-based trusted key file compatible with `TrustedKeys::load_from_file`.
- Fixed heredoc command-substitution defects in S7 scripts by using quoted heredocs for summary generation (removed noisy `Permission denied` side effects from backtick evaluation).
**Evidence:**
- `runtime_supervisor/src/posix_runner.rs` tests module.
- `tools/ci/foundry_s7_posix_runner_security.sh`.
- `tools/ci/foundry_s7_store_signature_security.sh`.
- `tools/ci/foundry_s7_access_control_security.sh`.
- Validation:
  - `bash tools/ci/foundry_s7_posix_runner_security.sh` (PASS)
  - `bash tools/ci/foundry_s7_store_signature_security.sh` (PASS)
  - `bash tools/ci/foundry_s7_access_control_security.sh` (PASS)
**Conclusion:** **Confirmed** S7 gate evidence is now more behavior-driven, deterministic, and free of prior summary-generation side effects.

## Root Cause
Primary root cause is **evidence-contract drift**: implementation, documentation, and Foundry gate assertions have diverged. Security controls are frequently documented as complete but only partially implemented or not behaviorally verified. A secondary root cause is **configuration semantic inconsistency** (`=1` vs bool parsing; shared env var with conflicting meanings), which creates fail-closed surprises and hidden fail-open edges.

## Eliminated Hypotheses
- **"Kernel-side capability validation is absent."** Eliminated: `ipc_v0::validate_handle` checks invalid handle, kind, and `table.validate(...)`.
- **"Typed IDL boundary is not used."** Eliminated: `idl/harness/*.toml` generated into `kernel_api/src/generated/*` and consumed in kernel/service paths.
- **"Store access control defaults fail-open."** Eliminated: default policy is `RequireCredentials`; unknown artifacts in registry are denied (fail-closed).
- **"Ring buffer TOCTOU capacity issue still present."** Eliminated: `kernel_api::ring_buffer::from_raw_parts` caches capacity once.

## Recommendations
1. **Sandbox truthfulness first (S7 hardening):** implement real seccomp/rlimits/chroot (or fail execution), and only log enabled controls after successful install.
2. **Unify env parsing contract:** introduce shared flag parser accepting `1/0/true/false`, update docs+gates, and remove split semantics between supervisor/store.
3. **Unify trusted key semantics:** split artifact-signature keys vs capability-verification keys (distinct env names or shared file loader); remove default RFC test key fallback outside explicit dev-only mode.
4. **Enforce signatures on all artifact read/verify execution paths:** `GetBlob`/`VerifyArtifact` must enforce signature policy or route through a single verified-read helper.
5. **Kernel hardening before SMP expansion:** add SHMEM allocation rollback, validate `shm_cap.kind` in map path, and repair trace publish ordering; add explicit SMP guards where locking is not yet implemented.
6. **Schema convergence:** consume canonical trace/observed-cap structs+validators in `domain_manager` (or create shared types crate).
7. **Gate quality upgrades:** replace grep-only structural checks with behavior probes and deterministic pass/fail criteria tied to exploit-relevant outcomes.

## Preventive Measures
- Add a **security-assertion matrix** mapping each status claim to exact enforcing code path + Foundry behavior test.
- Require **doc↔gate↔code consistency checks** in CI for security-sensitive env vars and policy names.
- Add **regression tests for fixed vulnerabilities** (e.g., trace publish order, SHMEM allocation rollback, signature enforcement on blob paths).
- Treat any "security-complete" claim as invalid unless backed by executable gate evidence artifacts.
