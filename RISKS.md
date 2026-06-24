# RISKS

**Last Updated:** 2026-06-24
**Status:** Active

## R1: Compatibility gravity (Linux becomes the "real OS")
Mitigation:
- Native shell + portals are first-class.
- Store UX always labels native level and offers a graduation path.
- Ports require Foundry gates and dossier artifacts.

## R2: Performance bottleneck in capability broker
Mitigation:
- Broker decides grants; kernel validates handles on fast path.
- Never require synchronous broker calls in hot-path data movement.

## R3: GPU latency makes the desktop unusable
Mitigation:
- Display Export Harness must meet latency budget.
- Roadmap: Quarantine → Native scanout → copy engines → scheduling.

## R4: Tooling sprawl (Foundry + Store too big too early)
Mitigation:
- Vertical slices only.
- Every subsystem must ship with a consumer + a gate.

## R5: Hardware support long tail
Mitigation:
- Foundry pipeline from day 1 (trace/replay/minimize).
- “Golden machine” Tier-1 contract (IOMMU-class) before broad targets.

## R6: Trace corpus privacy + size explosion
Mitigation:
- Default to protocol traces (typed harness transcripts) as the spec.
- Treat bus-level evidence as an opt-in, tiered artifact.
- Redact/sanitize traces before upload; support local-only retention.
- Enforce size caps and retention policies per artifact type.

## R7: Static resource limits in #![no_std] kernel
Severity: Medium (Denial of Service)
Confidence: High
Evidence:
- `CAP_TABLE_SIZE = 64` in `kernel/src/cap_table.rs` limits IPC capability handles
- `MAX_REGIONS = 16` in `kernel/src/shmem.rs` limits shared memory regions
- `MAX_FRAMES = 131072` (512 MiB) in `kernel/src/mm/bump.rs` limits frame allocator
- All are hardcoded static arrays due to #![no_std] constraints
Mitigation:
- Documented as known constraint of static allocation model until dynamic memory available
- S8 landed `FrameAllocator` plus reusable `BitmapAllocator` paths, but the
  kernel still relies on bounded static backing structures during bring-up.
- Current limits sufficient for early bring-up and controlled workloads
- Gates include resource exhaustion assertions (`table_exhaustion_returns_no_memory`)

## R8: Hardware evidence overclaim
Severity: High (incorrect readiness or security claim)
Confidence: High
Mitigation:
- Keep QEMU, replay, live HIL, appliance, and metal evidence levels distinct.
- Require target provenance markers for graduation mode.
- Treat appliance observations as controller evidence, not target truth.
- Keep physical gates opt-in and fail closed on stale logs.

## Security vulnerability findings (V-XX)

### SMP Transition Hardening (2026-02-18)

The following SMP transition debt has been addressed as part of the security hardening effort:

**Fixed Issues:**
- **Trace Ring Publish Ordering**: Fixed publish-before-write pattern in both legacy `emit()` and `DomainTraceRing::emit()`. Event data is now written before incrementing the write index, preventing readers from seeing uninitialized data.
- **Legacy Trace Ring SMP Guards**: Added `LEGACY_SMP_ENABLED` flag and SMP guard checks to legacy trace ring functions (`emit()`, `read()`, `claim()`). These panic if used after SMP is enabled, forcing migration to the SMP-safe per-domain ring buffers.
- **SHMEM Handle Kind Validation**: The `validate_cap()` function already correctly checks `shm_cap.kind == HandleKind::Shmem` to prevent confused deputy attacks where IPC handles are used as SHMEM handles.

**Already Protected (Previous Hardening):**
- **FRAME_ALLOCATOR**: Already wrapped with `spin::Mutex` (security fix NEW-004) for thread-safe physical frame allocation.
- **ADDRESS_SPACE_TABLE**: Already wrapped with `spin::Mutex` (security fix NEW-004) for thread-safe address space management.
- **Capability Table**: Already has SMP guards with `smp_enabled()` function that panics if used after SMP is enabled.

**SMP-Ready Components:**
- **Per-Domain Trace Ring Buffers**: Use atomic operations and are safe for SMP use.
- **SHMEM Module**: Uses the protected `FRAME_ALLOCATOR` and is SMP-safe for physical memory operations. The `ShmemRegionTable` itself assumes single control thread but this is documented.

**Remaining SMP Debt:**
- **ShmemRegionTable**: Not yet wrapped in synchronization primitive for multi-threaded access. Documented as single-control-thread assumption.
- **Per-Domain MMU Operations**: Architecture-specific MMU implementations must ensure proper memory barriers for SMP.

### Mitigated findings (12/15 total)
- **V-01 (ContentId validation)**: Mitigated via SC-01 with gate [`tools/ci/foundry_hardening_wave_a_batch1.sh`](tools/ci/foundry_hardening_wave_a_batch1.sh). ContentId validation enforces strict SHA-256 format in artifact_store_core.
- **V-02 (wire safety)**: Mitigated via SC-02 with gate [`tools/ci/foundry_hardening_wave_a_batch2.sh`](tools/ci/foundry_hardening_wave_a_batch2.sh). Wire format hardened with explicit little-endian encoding.
- **V-03 (posix_runner default-off)**: Mitigated via SC-03 with gate [`tools/ci/foundry_hardening_wave_a_batch2.sh`](tools/ci/foundry_hardening_wave_a_batch2.sh). posix_runner now defaults to disabled (opt-in via explicit flag).
- **V-04 (unforgeable capability tokens)**: Mitigated via SC-04 with gate [`tools/ci/foundry_hardening_wave_b_batch2.sh`](tools/ci/foundry_hardening_wave_b_batch2.sh). Unforgeable capability token design and validation implemented.
- **V-05 (kernel capability table)**: Mitigated via SC-05 with gate [`tools/ci/foundry_hardening_wave_b_batch2.sh`](tools/ci/foundry_hardening_wave_b_batch2.sh). Kernel-side capability table and validation logic implemented.
- **V-06 (capability table validation)**: Mitigated via SC-05 with gate [`tools/ci/foundry_hardening_wave_b_batch2.sh`](tools/ci/foundry_hardening_wave_b_batch2.sh). Kernel capability table validation implemented.
- **V-07 (trace ring ordering)**: Mitigated via SC-06 with gate [`tools/ci/foundry_hardening_wave_b_batch1.sh`](tools/ci/foundry_hardening_wave_b_batch1.sh). Trace ring monotonic ordering enforced with gate assertions.
- **V-08 (init parser checked arithmetic)**: Mitigated via SC-07 with gate [`tools/ci/foundry_hardening_wave_b_batch1.sh`](tools/ci/foundry_hardening_wave_b_batch1.sh). Init parser arithmetic overflow checks added.
- **V-09 (log path confinement)**: Mitigated via SC-08 with gate [`tools/ci/foundry_hardening_wave_a_batch1.sh`](tools/ci/foundry_hardening_wave_a_batch1.sh). Log path confinement enforced in runtime_supervisor.
- **V-11 (store boundary split)**: Mitigated via SC-09 with gate [`tools/ci/foundry_hardening_wave_c.sh`](tools/ci/foundry_hardening_wave_c.sh). Store/service boundary enforcement and capability split implemented via artifact_store_schema crate.
- **V-14 (codegen fail-closed)**: Mitigated via SC-02 with gate [`tools/ci/foundry_hardening_wave_a_batch2.sh`](tools/ci/foundry_hardening_wave_a_batch2.sh). Codegen now fails closed on schema errors.
- **V-15 (pin nightly)**: Mitigated via SC-10 with gate [`tools/ci/foundry_hardening_wave_c.sh`](tools/ci/foundry_hardening_wave_c.sh). rust-toolchain.toml pinned to specific nightly version.
- **V-16 (capability handle aliasing, SC-13)**: Mitigated via handle kind disambiguation. Added `HandleKind` enum (Invalid/Ipc/Shmem) to prevent cross-table handle confusion. Both `StaticCapTable` and `ShmemRegionTable` validate handle kind, preventing confused deputy attacks. Verified via gate tests in `foundry_shmem_control_s8_phase2.sh`.

### Remaining lower-priority architectural risks (1/15)
- **V-10 (supervisor TCB breadth)**: Architectural pending kernel pivot. Supervisor TCB reduction requires further policy migration kernel-side; POSIX and compat runners remain host-side by design for S2/S5.

### Mitigated in S9.1 (previously listed as remaining)
- **V-12 (trace isolation)**: Mitigated via per-domain trace buffers (S9.0 Phase 1), domain-scoped writers (S9.1 Phase 2), trace capability access control (S9.1 Phase 3), kernel trace service (V-012 Phase 4), and user-space trace client (V-012 Phase 5).
- **V-13 (portal TOCTOU)**: Lower priority; mitigated in part by unforgeable capability tokens (SC-04). Full portal broker hardening remains on the backlog.
