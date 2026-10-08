# Foundry development and CI execution profile v0

**Date:** 2026-10-08
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

The `native_runner` development package profile strips debug metadata from its
executables, including the private validator worker. LT hashes the complete
selected worker inside its existing 2500 ms wall budget; omitted debug sections
reduce those pinned bytes without changing runtime code optimization, features,
Wasmtime configuration or the 1500 ms guest / 1000 ms host-call limits. This
applies to package outputs, not only the worker, and reduces rich host debugging
metadata. Set this package override to `strip = "none"` when that metadata is
needed; acceptance still hashes and supervises the actual selected executable.

A paired Linux experiment on the same source and fixture observed worker size
falling from 182,820,408 to 42,777,632 bytes. One invalid and one valid request per
build, with no retries or warmup, took 384/369 ms before and 292/288 ms after.
This small sample does not isolate hash cost, explain every hosted Docker attach
timeout or establish stable latency or whole-CI speedup. Existing functional and
timeout/containment assertions remain required on the selected build.

`tools/ci/ci_lanes.py` owns one canonical inventory. `just preflight` executes the
complete 55-stage serial sequence: proof and compiler-cache input prerequisites,
generation before formatting, tooling regressions, IDL lint, target builds,
baseline and six strict lint tranches, workspace tests, the umbrella and all 39
extended stages. The supplemental `editor-adapter-migration` host stage is inserted
immediately after `desktop-editor-task`; removing that one stage reproduces every
original 54-stage name, argument, order and environment override. It executes two
explicit, unfiltered required-feature Cargo test targets with default features
disabled, checks all four adapter assertion names and both actual MAX rejection
markers, and rejects incomplete fixtures. The default-feature workspace test stage
does not establish execution of these supplemental targets. The original 38
extended stages, including the earlier original 36, remain required.
`just ci-lane quality`, `just ci-lane host`, `just ci-lane agent` and
`just ci-lane qemu` select the four CI lanes. Quality retains its 13 stages.
The extended runtime bodies partition exactly into 25 host, one agent and 13 QEMU
stages. QEMU also runs generation and the umbrella; host and agent each run the
same five preparation stages before their runtime bodies. These isolated
preparations do not add stages to local `preflight` or `extended`. Historical
54-stage evidence describes its original source snapshot and does not validate
this 55-stage successor or an adapter migration.
No CI job additionally executes the full or extended sequence. Gates within a
lane remain serialized. CI selects `RUST_TEST_THREADS=1`, matching the accepted
integration profile, and executes the four lanes on separate runners with
separate output and compiler paths.
Local users must also use separate checkouts/output resources when running lanes
simultaneously. A tracked source fingerprint and Cargo.lock hash are checked
before and after execution, including failure. Direct Cargo commands use
`--locked`; legacy internal commands additionally rely on the unchanged-lock check.

CI installs the dated toolchain, components and targets from
`rust-toolchain.toml`, caches dependency downloads and toolchains, and verifies
the pinned Docker image, SW0 prerequisites and compiler-cache inputs before host
and agent toolchain installation or compilation. The agent runner has the same
strict environment and five-stage bootstrap as host, but restores only Cargo
downloads and the pinned toolchain; it has no compiler-target or evidence cache.
Host and agent depend only on successful change classification, so neither waits
for quality or the other lane. QEMU's
compatibility package cache uses its exact digest and the gate still checks the
bytes. Superseded PR runs may be cancelled within their event/PR group. The stable
required `foundry` job runs after every lane and fails closed on failure,
cancellation, unknown result or an unexpected skip. A successful docs-only
classification permits exactly four skipped lanes; OS-code classification requires
all four lane results to be successful. The policy and CLI require every result
in the order `changes, os_code, quality, host, agent, qemu`, with no optional
agent default. Missing results or failed classification deny acceptance.
`merge-gate` also requires
successful classification and separate org governance; author/reviewer authority
is unchanged.

Every stage passes through `stage_timer.py`: its record contains actual argv,
child PID, monotonic elapsed time and exit status. Unique atomic records survive
command failures, and each lane uploads its own uniquely named timing directory
with `if: always()` and missing files treated as an error. Upload during
cancellation is best effort; a cancelled lane still fails required acceptance.
Environment values
are not copied. An unproved reap is recorded as cleanup uncertainty and fails;
post-kill waiting is bounded. These records are instrumentation, never substitute PASS receipts.

## Source admission and change classification

The inexpensive path covers ordinary Markdown prose and JSON/YAML governance
packets under `docs/org/`. Before that exemption, executable contracts under
`docs/contracts/`, fixtures in any source directory, unknown structured inputs
outside `docs/org/`, and the reviewed execution-consumed Markdown inventory
require Foundry. Classification uses changed paths, so deleting an input has the
same requirement as modifying it. `CONSTITUTION.md` has a contract source pin;
`NEXT_TASKS.md` and `EVIDENCE_LEVELS.md` carry HIL appliance gate assertions, and
the virtio reference-vault READMEs are gate prerequisites. These documents require
Foundry. `CURRENT_STATUS.md` remains prose and is checked by governance. Policy
regressions audit literal OS gate document references, root document names,
source registries and contract source pins for new Markdown inputs.

NativeRead and NativePreview retain their historical source inventories and add
the unchanged NativeSave task collector's conservative dependency closure. That
collector follows transitive local path dependencies, workspace manifests, IDLs,
Rust sources under `src/tests/examples` and literal includes, including
`desktop_editor_core` and its fixtures. The adapter follows the same bounded dependency roots, excluding metadata-only
workspace members, and checks explicit Cargo `lib/bin/test/bench/example` paths; a target outside the collector's source set
is rejected. Supporting another source layout requires a reviewed collector
change, not merely adding an unexamined path to the registry. It also covers the native application and Save prerequisites; this is
a conservative source identity, not Cargo's exact feature-unit graph. The held,
byte-pinned collector and admission adapter validate the closure before and after
capture. A newly extracted crate, new include or missing dependency fails closed
until its registry is independently reviewed. The strengthened manifest feeds
the existing compiler-cache key; changed dependency bytes change its digest.

`test_editor_source_inventory.py` exercises the actual two source-freezing
functions in disposable copies. It verifies core and included-fixture mutation,
an omitted future transitive crate, missing includes, explicit Cargo target
layouts, and registration in the existing quality tooling gate. These are source-admission tests, not runtime
Read/Preview outcomes. Gate assertions, runtime test counts, feature exclusion
probes and deadlines are unchanged.

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
admission and hits. Only CI's host lane restores these compilation directories,
with an exact outer key and no fallback; no lane restores acceptance evidence. Compiler caching is a trusted host tooling
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
canonical lane set or its serial preflight equivalent on a fixed candidate. Mac host checks cannot certify the
Linux/Docker/QEMU profile. The four-lane configuration has executed successfully on GitHub: the reviewed
pre-merge source and post-merge run
[37730644587](https://github.com/maxwellsantoro/RamenOS/actions/runs/37730644587)
passed all eight jobs. In that post-merge run, job durations were host 820 s,
quality 309 s, agent 337 s and QEMU 292 s; creation to final completion was 845 s.
These observations identify the host lane as this run's critical path, but there
is no controlled before/after hosted speedup measurement. The largest host
stages in its uploaded timing records were:

| Host stage | Elapsed seconds |
| --- | ---: |
| `desktop-editor-task` | 143.025 |
| `review-boundaries` | 106.148 |
| `editor-native-preview` | 82.683 |
| `compat-cleanup` | 72.827 |
| `editor-native-read` | 67.226 |
| `desktop-editor-store` | 66.827 |

The retained 64 stage records are actual command durations, including bootstrap
work repeated by the lanes; they are not 64 distinct canonical preflight stages.
Profile compilation versus runtime inside these host stages before selecting the
next bounded optimization. Preserve feature configurations and fresh runtime
evidence before consolidating compilation or assertions.

The current development loop already reuses compiler state across edits; the
acceptance caches deliberately bind a frozen source identity. Keep those loops
separate. Re-run affected checks after a change or failure, then reserve one
integrated candidate run with joined producers and fixed source bytes. Additional
unmodified repetitions need a concrete unresolved concern. Cached builds,
planning documents and timing estimates supply no metal, release, target editor,
or process-isolation evidence. The scripted host editor proof and its remaining
reliability repair work are described in the authoritative status/task pair.
