# Foundry development and CI execution profile v0

**Date:** 2026-10-05
**Scope:** Host tooling and CI orchestration; no OS authority or hardware graduation

This profile supports the ready editor and runtime packets in
[NEXT_TASKS.md](../NEXT_TASKS.md). It changes build preparation and scheduling,
while retaining the actual Foundry behavior, denial and failure assertions.
[Current Status](../CURRENT_STATUS.md) owns accepted validation results.

## Developer loop

Run `just codegen` after checkout and whenever IDL changes. Use a selected host
package and target while editing, for example:

```bash
just dev-check artifact_store_schema lib std
just dev-check store_service editor_native_read editor_native_read_v0_dev
just dev-check store_service editor_native_preview_read editor_native_preview_v0_dev
```

`dev-check` validates workspace membership, actual test targets, declared and
required features. It disables default features, uses the manifest-pinned
compiler, and runs a real build, strict Clippy and serialized tests. Host-ineligible
architecture packages are rejected. Parameters are shell-quoted before validation.
The persistent private target is keyed by package/test/features, actual compiler,
platform, lockfile, Cargo configuration and compiler environment. Its identity
intentionally persists across source edits; Cargo rechecks the live checkout.
Only one command owner can hold a target. An ordinary failed command with a known reap preserves its nonzero exit and can
reuse the warm target after a source fix. Interrupted or uncertain work quarantines
the entry. New command records retain actual exit,
reap, elapsed time and bounded output observations, and are labelled
`DEVELOPMENT_NOT_ACCEPTANCE`. They cannot replace an affected gate or preflight.

## Complete acceptance and lanes

`tools/ci/ci_lanes.py` owns one canonical inventory. `just preflight` executes the
complete serial sequence: proof and compiler-cache input prerequisites, generation before formatting, tooling
regressions, IDL lint, target builds, baseline and six strict lint tranches,
workspace tests, the umbrella and all 36 original extended stages.
`just ci-lane quality`, `just ci-lane host` and `just ci-lane qemu` select complete
partitions of that inventory. Gates within a lane remain serialized. CI selects `RUST_TEST_THREADS=1`,
matching the accepted integration profile. CI executes
those partitions on separate runners with separate output and compiler paths.
Local users must also use separate checkouts/output resources when running lanes
simultaneously. A tracked source fingerprint and Cargo.lock hash are checked
before and after execution, including failure. Direct Cargo commands use
`--locked`; legacy internal commands additionally rely on the unchanged-lock check.

CI installs the dated toolchain, components and targets from
`rust-toolchain.toml`, caches dependency downloads and toolchains, and verifies
the pinned Docker image, SW0 prerequisites and compiler-cache inputs before host
toolchain installation or compilation. QEMU's
compatibility package cache uses its exact digest and the gate still checks the
bytes. Superseded PR runs may be cancelled within their event/PR group. The stable
required `foundry` job runs after every lane and fails closed on failure,
cancellation, unknown result or an unexpected skip. A successful docs-only
classification permits exactly three skipped lanes. `merge-gate` also requires
successful classification and separate org governance; author/reviewer authority
is unchanged.

Every stage passes through `stage_timer.py`: its record contains actual argv,
child PID, monotonic elapsed time and exit status. Unique atomic records survive
command failures and CI uploads the lane's timing directory. Environment values
are not copied. An unproved reap is recorded as cleanup uncertainty and fails;
post-kill waiting is bounded. These records are instrumentation, never substitute PASS receipts.

## Opt-in acceptance compiler cache

NativeRead and NativePreview default to the existing fresh-target profile. Setting
`RAMEN_FOUNDRY_BUILD_CACHE=1` admits private compilation state from
`out/ci-optimization/compiler-cache`; CI's host lane explicitly selects this
profile. Every cache key binds the independently frozen source manifest, actual
Rust/Cargo versions, host platform, repository, lockfile, ancestor/Cargo-home
configuration, compiler environment, features, default-feature setting and phase.
The read-only `python3 tools/ci/build_cache.py --check-inputs` prerequisite checks
the actual lock and configuration paths before expensive suites. It starts no
compiler children and creates no cache entries. Configuration ancestors must pass
the same owner, permissions and nonsymlink checks even when configuration files
are absent. If a shared Cargo home is writable by another identity, use an owned,
private `CARGO_HOME` with explicit configuration and download paths; do not weaken
the permission checks. Select `umask 022` or a stricter mask before opt-in
acceptance so Cargo output directories cannot be writable by another identity.
The prerequisite and direct gate adapter reject a permissive mask before
compilation; the check immediately restores the caller's mask. Configuration and
lock inputs are checked again after execution.

Separate `exclusion-desktop`, `exclusion-store` and enabled targets prevent a
positive-feature library from overwriting an exclusion probe's library. Each
entry has an exclusive lock and checked owner, permissions and nonsymlink paths.
An in-flight marker is cleared only after normal completion. A failed or abandoned
entry denies reuse, including after abrupt coordinator termination. There are at
most 128 entries; capacity exhaustion fails instead of silently growing storage.
An operator may remove an inactive compiled-cache entry only after confirming
that its producers have stopped. Evidence directories and original failures must
be preserved. There is no automatic destructive reset or eviction.

Every admitted run still invokes Cargo and Clippy, executes its actual exclusion
probes, enumerates the exact binary tests and runs every behavior case. Source and
binary checks, new process birth identities, exits/reaps, nonces and fixture
cleanup remain fresh. Cached-profile runs also retain a separate, size/hash-checked copy of the actual
test executable in their private run directory, preserving historical bytes when
compiler targets are later rebuilt or removed. Results explicitly record cache
admission and hits. CI
restores compilation directories only, with an exact outer key and no fallback;
it never restores acceptance evidence. Compiler caching is a trusted host tooling
optimization, not a containment or hostile compiler/cache attestation boundary.

## Consolidation limits

The broker proxy roundtrip executes once; both its successful exit and snapshot
sentinel must come from that same captured execution. Codegen builds one actual
Cargo-selected executable, renders all 34 outputs and the aggregator each pass,
and writes only changed bytes. Missing or stale outputs are repaired, including
ignored modules. Rendering and failed-build checks remain mandatory.

The baseline warning metric and all 16 strict package checks remain separate.
A retained pinned-toolchain Cargo unit-graph comparison found that multi-package
batching changes dependency feature units in tranches 1, 2 and 6. That comparison
is not a Clippy diagnostic equivalence proof. Stage timings now expose their cost;
any future consolidation must preserve those configurations and checks rather
than infer equivalence from similar command flags.

Use focused checks during editing, then affected consumer gates and one complete
integrated preflight on a fixed candidate. Mac host checks cannot certify the
Linux/Docker/QEMU profile. Cached builds, planning documents and timing estimates
supply no metal, release, full UI1.1c or live Save evidence.
