# Current Status

**Last Updated:** 2026-10-06
**Status:** Active and authoritative for landed state
**Current Slice:** UI1.1 host editor; S12.4 HIL appliance physical loop pending
**Software Lane:** SW0 foundations through A2.9 implemented; full A2 and comparison pending

RamenOS is public pre-alpha, building toward the everyday OS for humans and AI
agents described in [VISION.md](VISION.md). This file records implemented behavior
and evidence boundaries. [NEXT_TASKS.md](NEXT_TASKS.md) owns the next work;
[CHANGELOG.md](CHANGELOG.md) holds detailed milestone history and
[DECISIONS.md](DECISIONS.md) holds rationale.

## Foundry development and CI execution

The independently reviewed [execution profile](docs/FOUNDRY_CI_OPTIMIZATION_V0.md)
adds `just dev-check` with one exclusive persistent compiler target, one-build
content-stable generation, and a complete canonical Foundry inventory. CI splits
quality, host/Docker, agent-task and QEMU work across isolated runners; the stable required
Foundry aggregate fails closed on failed, cancelled or unexpectedly skipped lanes.
The accepted dependency resolution is now tracked in `Cargo.lock`; toolchain and
download caches follow the pinned manifest. NativeRead and NativePreview keep
fresh targets by default and support explicit, source-bound compilation reuse
with separate exclusion/enabled targets, fresh assertions and retained binaries.
Early read-only input and umask checks deny unsafe cache setup before compilation.
Every stage retains actual monotonic timing and exit/reap observations.

The CI optimization integration passed all 52 stages on its assembled
candidate with `RAMEN_CI_STRICT=1`, `RUST_TEST_THREADS=1`,
`RAMEN_FOUNDRY_BUILD_CACHE=1`, a private Cargo home and `umask 022`.
The run took 844.028 seconds and includes the original 36 extended stages,
Docker controls, host consumers, QEMU and storage assertions. Initial permission
and fixture-access failures remain retained; permission repair affected only
owned temporary compilation outputs and preserved executable bytes. This is
host/QEMU evidence, not a kernel fix, physical qualification or release proof.

A measured schema developer check ran 94 library tests on both passes; its
command-time sum fell from 70.871 seconds cold to 1.238 seconds warm. Independently
reviewed focused Linux cache runs reran all nine NativeRead and five NativePreview
cases: command sums fell from 41.590 to 5.697 seconds and 56.784 to 6.412 seconds.
Those focused measurements precede the final umask-only helper amendment and
exclude setup and binary retention. Final integration validates the amended
helper. They do not establish a whole-CI speedup. All sixteen strict package
checks remain because multi-package Cargo unit graphs changed feature units.
The hosted workflow is reviewed configuration; a GitHub-hosted run is not claimed.

## Execution state

The S12.4 physical lane awaits test-hardware setup: first live serial capture,
then Intel AMT 11 power/reset, S12 on SATA, and S13 NVMe boot/update/rollback.
No live capture or actuation is scheduled. SW0 continues independently of lab
access and NVMe graduation. The dependency-driven queue in `NEXT_TASKS.md` also
allows S14/S15 contracts, host/replay work, and QEMU implementation to proceed
without the model comparison or physical qualification. Driver work retains its
own Reference Vault, Oracle and gate requirements; physical integration requires
the prepared observation/actuation loop. The [desktop v0 design](docs/plans/desktop-v0.md)
is accepted after independent review. UI1.0 now implements host permission preview
and launch lifetime with a real
non-rendering Rust witness and generated protocol-336 messages. The default-off
Unix fixture passed its 17-case Foundry gate on macOS and Linux, retaining actual
process identity, typed exchanges and cleanup evidence. UI1.1a now adds a
keyboard-driven volatile editor with typed focus and offscreen surfaces.
UI1.1b now adds a default-off real host Store transaction and joined-owner
recovery. The bounded native Read prerequisite connects approved editor authority
to that Store with live copy checks. The default-off, trusted in-process keyboard/editor/Store task is accepted as
UI1.1c. The actual editor process and RUN0 post-firmware memory ownership remain
next dependencies. RUN0.0's
pure map/retention admission prerequisite now passes 17 reviewed assertions,
with sticky insertion-overflow rejection and conservative bounded selection.
Actual firmware exit, retained-object collection and allocator installation are
not connected yet. USB
xHCI/HID, the target runtime and target desktop remain future work.

The [UI1.1 editor proposal](docs/plans/desktop-editor-v0.md) and
[external boot-profile Oracle proposal](docs/plans/boot-profile-oracle-v0.md)
are independently reviewed preparation packets. UI1.1 separates the volatile
in-process editor, real Store transaction, integrated task and actual editor
process. The Oracle proposal keeps external inspection distinct from guest
access and requires a reviewed relocated-entry resolution method before capture.
The editor's [wire allocation](docs/DESKTOP_EDITOR_WIRE_V1.md) now has five
canonical IDLs and generated Rust modules: input 802, focus 832, surface 833,
editor session 352 and artifact 368. Independent review checked all 43 messages;
IDL lint, `kernel_api` checks and the existing 17-case launch consumer pass.
The [shared host API](docs/DESKTOP_EDITOR_HOST_API_V0.md) is independently
reviewed and frozen, with a pinned font and independent old/new raster
expectations. Producer held state survives focus changes; confirmation requires
a release and fresh press. Exact rights, checked leases, observable pause
barriers and live save-admission state now support 13 executable assertions.
The default-off in-process Rust editor passes on macOS and the assembled Linux
checkout: real logical key input changes bounded ASCII drafts and offscreen
pixels; protected chrome displays `VOLATILE / IN-PROCESS`. Preview confirmation,
focus changes, surface alias retirement, denial, fault recovery and scoped
counter exhaustion are exercised through issued contexts. One irreversible
volatile save permit survives after-admission uncertainty without replay;
definitively fenced pre-permit timeout permits a fresh explicit `Ctrl+S`.
The gate retains 34 registry witnesses, canonical exchanges, actual composed
frames and original receipts, and checks that the development API is absent by
default. This establishes no Store IO, editor PID, device, target runtime or
process containment. Strict Linux preflight passes on the assembled UI1.1a
revision, including this gate in the extended Foundry suite.
The [Store transaction prerequisite](docs/plans/editor-store-transaction-v0.md)
specifies a default-off Store owner, private per-object CAS, one atomic
selection/receipt journal and joined-writer recovery. The
[exact Store API](docs/DESKTOP_EDITOR_STORE_API_V0.md) is independently reviewed
and frozen: pure records/codecs, opaque fixture authority, bounded admission and
actual join/fence witnesses prepare seven gate-first assertions. Pure payloads
and their canonical journal validator now pass the nine-case
`just foundry-editor-save-schema-ui1-1b` gate on Linux, including strict Clippy
and a no-default-feature build. All 115 schema tests pass on macOS. These records
provide no IO or commit authority. The seven service assertions are independently
reviewed and their original RED compile check failed at the missing Store module
as expected.
The default-off Store owner now passes
`just foundry-desktop-editor-store-ui1-1b` on macOS and assembled Linux. Its seven
behavior cases exercise private per-object CAS, atomic selection/receipt journals,
bounded admission, pre-permit closure, irreversible permits, same-base conflict,
original-receipt reconciliation and supported joined-writer reopen. Sixteen
evidence assertions check 48 fixture legs, 56 producer epochs, 112 snapshots,
actual typed calls and lease bytes, joins/fences, malformed input, descriptor
aliases and explicit owned-root cleanup. Each accepted run also performs 131
native Rust decodes of captured text, receipts and eligible journals; all exit
successfully and are reaped. Strict Clippy and default API exclusion pass. Strict
Linux preflight passes with this gate registered in the extended suite, including
the existing launch, volatile editor, save-schema and host/QEMU consumers.

This Store prerequisite proves a trusted in-process host CAS fixture using supported
fault hooks and a retained reopening owner. At that prerequisite milestone, connecting
the editor's live save authority to Store and delivering the integrated UI1.1c task
remained pending. A separate
[native Read API](docs/DESKTOP_EDITOR_NATIVE_READ_API_V0.md) and its
[bounded contract](docs/contracts/editor-native-read-v0.json) now connect an actual
approved editor peer to a real host Store Read. Nine reviewed gate-first assertions
pass on macOS and assembled Linux, with original deadlines, current instance and
Store grant checks, canonical selected-data copies and actual owned joins. Strict
Clippy for both services and native API exclusion without the opt-in feature pass.
The source-bound gate retains its actual test binary, process birth identity,
original logs, exit and reap evidence. It does not collect optional fixture exports
or establish a universal transcript validator. Strict integrated Linux preflight
passes for the same source candidate with
`RUST_TEST_THREADS=1`, including the affected host/QEMU consumers. This establishes
the serialized Rust test profile; parallel child-spawn reliability remains
unproved. Save admission and full UI1.1c remain separate.
The [native Store preview Read contract](docs/contracts/editor-native-preview-read-v0.json)
and [pure shared-data codec contract](docs/contracts/editor-native-preview-codec-v0.json)
are independently reviewed and frozen preparation. They specify actual input
tickets, current Store selection, one activation row, protected Read-only chrome,
and five precursor cases. The pure codec now implements schema2's 464-byte
grants and the 248-byte NoSave/Unavailable observation, with seven independently
reviewed assertions passing on macOS and Linux after a retained missing-module
RED. The Mac schema suite passes all 122 tests; the affected save-schema and native
Read gates remain green. Strict Linux preflight passes with
`RUST_TEST_THREADS=1`. Mac and Linux used separately retained dependency
resolutions. An initial preflight sandbox cleanup failure is retained; a fresh
focused sandbox run and the subsequent complete preflight passed without a source
change. These bytes grant no authority, Store IO, current-time or publication
proof. The contract JSON retains its creation-stage preparation snapshot; this
file owns current implementation status. The native preview observation amendment
is independently reviewed and frozen: genuine inactive rows have privileged
denial probes, protected pixels have literal font expectations, and an optional
248-byte record is tied to actual positive-actor frame publication. Pre-instance
preview remains 464-byte metadata. Own query identities are historical observations;
actual retained owners and joined producers must prove the two separate 64 limits.
The default-off NativePreview Read precursor now implements actual input tickets,
Store-current pinning, fresh approval, same-row activation/delivery and protected
Read-only composition. Five independently reviewed source-bound cases pass on
macOS and Linux through `just foundry-editor-native-preview-read-ui1-1c`, with
strict Clippy and exclusion from the older NativeRead feature. They check real
Store copies, inactive/foreign denials, original clocks, paused frame publication,
retained setup/error owners and separate query/producer capacity with actual joins.
The two platforms used the identical 278-source manifest and explicitly captured
Cargo.lock. Affected Mac NativeRead, volatile editor and Store gates pass.
Strict integrated Linux preflight passes with `RAMEN_CI_STRICT=1` and `RUST_TEST_THREADS=1`.

The initial API-absence RED and subsequent 0/5 and 4/1 behavioral failures remain
preserved. Reviewed handler corrections fixed canonical native error replies,
Cancel wire validation, foreign lease denial ordering and original pin/Produce
lifetimes. The 4/1 run also exposed an assertion comparing across rightful delivery;
its reviewed repair now asserts the exact live-pin 1-to-0 transition with all other
counts unchanged before checking foreign pairing leaves that delivered state intact.
The gate retains actual executable, process birth/reap and bounded command logs.
Wire, pixel, lease and service-thread-join checks are assertions in that pinned
binary, with no independent exported transcript or per-thread native identity claim.
Original Produce validation-wait forwarding is source-reviewed; no injected wait
scenario was run. Contract creation-stage snapshots remain provenance, while this
file owns implementation status.
The [pure native Save data contract](docs/contracts/editor-native-save-codec-v0.json)
is implemented in a separate schema3 Save grant codec (464 bytes) and schema2
original Commit outcome codec (248 bytes). Eight independently authored and
reviewed assertion families pass on macOS and Linux through
`just foundry-editor-native-save-codec-ui1-1c`, with strict std Clippy and no_std
checks. Actual missing-module RED preceded implementation; the earlier mixed
compiler failure and reviewed fixture-loop corrections remain retained. All six
Active handle pairs are checked for duplicates. Exact original Binding, epoch,
receipt outcome and 176-byte digest joins are data correlation only.

The new gate freezes all 38 source inputs, builds in a fresh private target, and
retains the Cargo-selected binary, exact unfiltered list/outcomes, owned child
exit/reap, and bounded logs. Affected pure Save (nine), Preview (seven), NativeRead
(nine) and NativePreview (five) gates pass on macOS and in strict Linux preflight.
At the pure-codec prerequisite milestone, the canonical inventory had 37 extended
and 53 preflight stages; the original stages and CI overrides remained. Contract creation-stage JSON flags
and source pins remain historical provenance. No live Save API, mutation permit,
Store IO or protected current-draft Saved claim follows from this pure codec.
At that milestone, the live bridge and original eighteen-case integrated task
remained pending.
The independently reviewed [live Save/task API contract](docs/contracts/editor-native-save-task-v0.json)
now closes the cross-service preparation: fresh Save approval, private keyboard
draft/render ownership, original-only allocation lookup, immutable permit and
protected current-draft Saved joins, a shared 64 producer budget including the IO limit of 2,
and actual Core-bound joined-owner reopen. A separate default-off
`apps/artifact_editor` consumer is specified with asynchronous once-only Save
completion; edits and rendering may continue while an original remains pending.
Own unresolved originals retain protected banners independently of the current
draft or a fresh unrelated-object launch.

The contract reserves additive Artifact 368 messages 9/10 and a Save-only fixture
profile with three objects/two sessions, 131072 exchanges, 128 total composed
frames and 16 pending frames. Existing modes retain their limits and wire bytes;
the Store shared-data cap of 32 remains separate from 48 durable operation slots.
Recording ceilings are finite bounds, not observed fixture counts. That API
preparation milestone alone implemented no new IDL/generated types, executable
eighteen-case assertions, RED, Save handlers or task gate. The subsequent packet
derived and independently reviewed the exact assertion/evidence inventory before
actual RED, codegen and handlers. At that API-preparation milestone, the required S11/S12/S13 and organization
governance checks passed on its final Linux documentation candidate. Existing
runtime code and its 53-stage acceptance scope then remained unchanged.

The subsequent implementation candidate acquired independently reviewed executable
assertions for all original eighteen cases and a source-derived recording
inventory covering 93 scenarios. Actual repeated compilation established the
missing native APIs as the initial RED after narrow, independently reviewed
fixture type corrections. The original mixed failure and clean repeat are
retained separately. Additive Artifact 368 messages 9/10 are generated, and IDL
lint plus both bare-metal `kernel_api` target checks pass. Concrete Desktop, Store and `apps/artifact_editor` adapters have passed independent
source and integrated host-task review; focused library compilation alone does
not establish that behavior.

The first nineteen fresh source-admitted macOS runs have executed all eighteen
cases without filtering. The nineteenth runtime passes all eighteen with actual process
birth and reaped exit, taking 194.38 seconds. This includes the original keyboard,
protected-frame, malformed-input, authority, restart, isolation, Store conflict,
lost-reply/readback, application recovery and counter/capacity assertions. The
reviewed StatusVersion fence cleanup and failed OriginalRecord entry-owner
retention now pass in the full run. The Writer fixture requires the old closed
intent's exact Stale denial and a genuine fresh CtrlS/Allocate Exhausted control;
it passes unchanged IO/files, one durable operation and counts1/0/0.

In the fourteenth run, the three strict-check logs complete successfully, then
the recording reader rejects its unsupported fixed-zero ownership-ingestion
expectation.
Actual final Store sidecars carry Unix ingestion timestamps. Retained-corpus
diagnostics also find that historical closures need the authentic prior Core
proof, and that two genuine Saved frames omit already-observed draft/render joins
from their records. The reader now validates complete bounded reopen lineage,
full immutable operations and genuine original tickets/joins, with separate
finite allocation-only retirement and source-owned deadline controls. Current
closures still require their actual witnesses. The recorder retains the two
Saved joins from the existing publication assertions; generic Present remains
Unsaved. An isolated compilation and the complete original surface case pass
with both existing legs, unchanged frame/file/observation counts and actual
process births/reaps. This selected case is diagnostic evidence only.

Root has adopted the independently reviewed reader, recorder and pin changes.
The fifteenth fresh combined runtime passes all eighteen cases, and all three
strict checks exit successfully and reap. The failure-reporting repair retains
all eleven genuinely returned commands in a separate incomplete file, with no
pending return. The fourteenth run's missing later process rows are not
reconstructed. Recording validation now reaches the malformed-source controls
and rejects an incorrect Commit-ticket allocation expectation: actual pre-seal
Commit tickets have no allocation or binding, while their matching genuine
Allocate tickets retain the full allocation. A bounded same-intent ticket/reply
and producer crossjoin is independently reviewed and integrated. Private reader
diagnostics then found two further decoding assumptions: the Desktop flag is
true for the existing NativeRead control, and Artifact 368 receipt length is at
payload offset 36. Their reviewed corrections preserve all authority checks.
The capture quota test now exposes the genuine execution and records the required
duplicate-poll NotReady control. Its complete original capacity case passes
privately with all 32 scenarios, seven capture controls, unchanged selection/IO
and actual cleanup; this is diagnostic evidence only. Root has adopted the
reviewed assertion and source-pin changes. The sixteenth fresh runtime passes
all eighteen cases in 155.75 seconds; strict3 succeeds and all eleven returned
commands are retained and reaped. Recording validation finds one undersized
Document observation bound: six existing getter calls exceed the recorded limit
of five. A reviewed private metadata correction preserves all six witnesses.
Independent reviews accepted the reader corrections for internally joined IO,
coarse Desktop producer kinds and the separate current versus original actors
after editor recovery. The private diagnostic now validates all 93 scenarios,
727 ordered controls and genuine verifier callbacks on the unchanged sixteenth
run corpus. Root has adopted the reader, the one observation-bound correction
and their exact pins. The seventeenth fresh task gate passes on a new 203-source snapshot: all eighteen
cases in 220.14 seconds, strict3, complete 93-scenario recording validation and
final source checks. Every returned child is retained and reaped. Every failed
predecessor remains retained. All five affected Mac consumer gates have passed and are independently accepted
within their existing scopes. The first combined Linux 54-stage attempt stops
at verifier formatting: three prerequisite/codegen timings pass, then fmt-check
and the outer timer fail. All 745 source rows match before and after; these four
stage timings plus the outer timing are partial failure evidence, not a complete
55-timing inventory. Earlier startup denials and mode repairs to coordinator-owned
temporary ancestors are retained without source/kernel byte changes or weaker
admission policy. The independently reviewed verifier formatting and exact two-pin
outer update are adopted, and the full Cargo formatting check passes. Fresh
Gate18 passes all eighteen in 159.49 seconds, strict3, complete 93-scenario
recording validation and final source checks; all eleven returned commands are
retained/reaped. Formal Gate18 result evidence has passed independent review. The second combined
Linux attempt retains eight canonical stage timings plus the failed outer timer:
formatting passes, then baseline lint rejects only the verifier hex-parity
predicate (`manual_is_multiple_of`). All 745 source rows match before and after;
this remains partial failure evidence, not complete54 acceptance. The reviewed
single-predicate method correction and exact two outer pins are adopted without
warning suppression. Full local Cargo formatting and baseline lint checks pass.
Neither changed file belongs to any of the five accepted affected Mac consumer
closures; those recorded source rows were current at Gate19, before the later
Cargo profile change. Fresh Gate19 passes all
eighteen in 194.38 seconds, three strict checks, complete 93-scenario recording validation and final source checks,
with all eleven returned commands retained/reaped. Formal Gate19 result evidence has passed independent review.

The third combined Linux attempt retains 27 canonical stage timings and a
failed outer timing of 778.888 seconds. Its native task passes all eighteen in
99.91 seconds and completes three strict checks, with all eleven returned
commands retained/reaped. Verifier-binary retention then rejects the actual Cargo
example's two hardlinked executable aliases, before recording-reader validation,
verifier callbacks or final task acceptance. All 745 source rows match before
and after; this remains partial Linux failure evidence.

The independently reviewed verifier-only repair admits only the closed
canonical/hashed Cargo alias pair, retaining bounded regular-file ownership,
held no-follow identities, full hashing and a private single-link executable
copy. It checks the final deadline after the last source hash. Failure-path
file-descriptor closure and parent-path replacement controls are retained. Eight
focused component-control families pass on macOS and Linux, and a separate
control retains the actual earlier Linux Cargo pair. These are component results,
not task or full-preflight acceptance.

The twentieth fresh formal macOS run passes seventeen cases and fails the
unchanged stalled-session 1000 ms assertion, with native test time 226.02 seconds
and actual exit/reap evidence. Strict checks, the recording reader and verifier
callbacks do not run after this failure. The failed packet remains retained.
An independently reviewed, instrumented private A/B pair uses the original
assertion and copied retained-file load from nine earlier cases. Support
intervals measure 451.574/468.450 ms in A and 132.091/98.957 ms in B; raw plus crop
hashing measures 267.937/279.907 ms and 17.082/17.518 ms, while scans remain near
50 ms. Both sides pass their one focused case. Compilation takes 17.877/19.148
seconds and whole focused tests take 3.63/3.57 seconds. This is one diagnostic
pair, not the sole cause of the formal failure or a whole-CI/hosted speedup.

Root adopts exactly the reviewed 42-byte Cargo.toml addition
`[profile.dev.package.sha2]` / `opt-level = 3`. Both actual diagnostic builds have
86 compiler units; only sha2 changes opt-level 0 to 3, with debug assertions,
overflow checks and debuginfo 2 unchanged. Native/service crates retain opt-level
0. At sha2 profile adoption, diagnostic support and recorder instrumentation were
not adopted; the separate frame-batch design remained held. Because the root manifest is source-bound, all
five affected Mac consumer gates run fresh: Desktop launch, volatile editor,
Store editor, NativeRead and NativePreviewRead return zero with actual process
birth/reap and source pre/post checks. Their independent UI0 evidence review has
accepted all five source closures at that profile-adoption candidate. These results do not replace
the formal task or Linux validation.

Fresh formal Task21 passes all eighteen without filtering in 86.40 seconds
against packet `yaKqSww8`, source manifest
`084efcb9e50929addd39fc639fca6a939908fd3de55d0e34050ff5d5d2689e83`.
Three strict checks complete successfully, and all eleven actual command returns
are retained/reaped. The reader validates all 93 scenarios and records 727 ordered
controls and seven verifier callbacks in this run; these are observed counts,
not fixed future inventory assumptions. Actual case artifacts contain 3,799 files,
3,953 entries, 547,274,819 bytes and 28,374,339 JSON bytes. Final source checks pass.
The retained result is SHA-256
`10d73330268571f939a29405ec80031c6f35c4a716e8d2d7049da8dece777e87`;
independent formal Task21 evidence review accepts this trusted in-process macOS
result (report SHA-256
`9b51672d897df75fe06e5fb3f75f02a3c92ac0e732ba61b1dada324f664eb6ea`).
This fresh host result does not establish whole-CI acceleration from comparison
with uncontrolled earlier runs. At that Task21 milestone, complete fresh Linux54
validation and milestone acceptance remained pending.

A separate fresh focused Linux task on packet `ihrAae0O` passes all eighteen
without filtering in 66.33 seconds, three strict checks, all 93 recording
scenarios and final source checks. It records 727 ordered controls and seven
verifier callbacks; all eleven actual commands retain birth, exit and reap
evidence. Independent review accepts the focused task and its completed packet
transfer. All 745 source identities, bytes, types and modes match before and
after. This run also records the authentic two-link Cargo verifier alias pair
and its exact private single-link retained copy. This is focused trusted-host
Linux evidence, not complete canonical preflight or milestone acceptance.

The fourth canonical Linux54 attempt then passes its first 26 stages and fails
stage 27, the integrated task's recording reader. The failed outer timing is
745.488301585 seconds. Its native tests pass all eighteen without filtering in
65.67 seconds, and three strict checks pass. The preserved incomplete command
record retains all eleven actual command births, exits and reaps, plus seven
completed verifier callbacks in incomplete scope; no final Task result exists.
All 745 source bytes, modes, aliases and Git-selected paths match before and
after. Independent reviews accept the partial failure metadata and complete
failed-packet preservation, not Task recording or full54 acceptance. Each
retained stage and outer command reports reaped; the aggregate cleanup flag
remains uncertain because the complete 54-stage record is unavailable.

Source audit and independent diagnosis locate the reader rejection in
`app_unknown/0` and `app_unknown/1`: actual joined Dispatcher 12 is retained in
the genuine fence but absent from the caller's join observations. The reader's
internally joined IO route correctly rejects this non-IO row. This does not
establish a runtime producer fault. At that fourth-run failure, a fixture-only
collection repair remained pending review and fresh evidence. The earlier failed
histories remain retained; these partial results establish no whole-CI speedup
or containment.

The independently reviewed fixture collection repair and CI4lane configuration
are now adopted source. The repair retains genuine current-Core opaque tokens
before the deliberately incomplete quiesce, records actual once-only returned
proofs before final ownership transfer and checks recorded service-fence coverage;
services, Reader/schema, all eighteen assertions and 1000 ms windows are unchanged.
The focused maintained CI optimization gate passes 61 Python and seven Rust
controls in 42.080959416984115 seconds with actual exit/reap evidence. Its scope
is tooling controls, not hosted performance or complete integration. CI4 preserves
all 54 local and 38 extended stages and requires quality/host/agent/QEMU results;
the 18 mocked RED/GREEN controls and isolated lane checks remain separate evidence.

Formal Mac Task22 then passes seventeen cases and fails the unchanged B-session
1000 ms assertion, with native test time 196.92 seconds; it is not a Task PASS.
Its source-bound failure remains retained. Focused Linux v7 passes all eighteen
in 62.55 seconds before strict Clippy rejects one nested fixture conditional.
The reviewed short-circuit AND-equivalent repair preserves its body and assertion
order. Focused Linux v8 subsequently passes the complete task and its independent
review, on the earlier recorder preimage; it does not validate the later pair writer.

A separate measured diagnostic did not reproduce the Mac22 deadline failure:
its two intervals were 237.556/216.218 ms, with six recorder scans taking
130.610/111.500 ms. This supports a bounded scan-reduction choice, not a sole-cause
finding or CI speed claim. The frame plus optional chrome writer now shares one
exclusive reservation and full scan, checks combined limits and both targets
before first output growth, and verifies each complete single-link file separately.
A second write failure preserves the first completed RawRef, actual partial bytes
and abandoned marker. Legacy singleton writes are unchanged. The accepted marker
footprint can conservatively deny within four bytes of the aggregate ceiling;
no cap is relaxed. Eight focused families run actual baseline RED (one PASS,
seven expected failures), then all eight pass with the reviewed implementation;
independent GREEN review is
`013a0d58c979ebe4cdee5fb833c07ed75d233c351fd2d1c22d7cfe8a832dbe58`.
These component controls do not replace the original task.

Mac Task23 on source
`7221fafc9d83ec55984da8d59bc96ce3e74cf7fa758c9eab58dfe6fe89e2a930`
reports all eighteen unfiltered outcomes PASS in 92.99 seconds, strict3 and
complete 93-scenario recording validation. Its retained result is
`ee3ec6f939ffba3cdf893bb8eec2cbb86d89ab262a255371f6fb729440243fca`.
Focused Linux v9 on that same Task source independently passes the complete task:
18 unfiltered cases in 60.769 seconds, strict3, all 93 scenarios, 727 ordered
controls and seven verifier callbacks observed in that run. Its actual result is
`3e30617e88b1c5fcc15e9d02b7e7ec83b168905c831511539b4682bb37c6e34d`,
accepted within focused trusted-host scope by review
`a9a668cbb879e9a9f5b57302f4d6a428a80004713eb54e2efa5723d5376e0d7e`.
Those counts are observations, not future acceptance constants. Neither focused
run establishes full54 acceptance or whole-CI acceleration.

The subsequent fifth canonical attempt stops at fmt-check after 3.264 seconds:
the earlier AND-equivalent fixture correction needs only rustfmt line wrapping.
It reaches four canonical timings: prerequisites, cache prerequisites and codegen
pass, then fmt-check fails; the failed outer is the fifth timing record, not a
complete55 inventory. The Task stage does not execute. Whole745 source pre/post
matches; incomplete-stage collection conservatively sets downstream cleanup
uncertainty true. This is a formatting failure, not a native-runtime failure.
Its partial evidence and review `d86568dfa0c211fbd085213c8f98c9ed2044970961cd9c02832b52578e1579d7` remain retained. A format-only
successor changes the source-bound fixture/inventory/outer pins; Mac23 and v9
are historical outcomes and cannot substitute for fresh final-source execution.

Final-source Mac Task `24`, packet `pQFwONsI`,
source `8f8fb22911c2696f323f36935a3920e1f1f6a0f4dc52be659b1a1c43954a6a4f` and result `be9d403723a32acc1b5aa3a2f8b38b6bd640e068b810500bc018e4a7663f35e4`
pass all eighteen unfiltered outcomes in 91.26 seconds,
strict3, all eleven actual command returns/reaps, complete 93-scenario reader
validation and final source checks. Its observed 727 ordered
controls and 7 verifier callbacks remain run-specific.
Independent review `76386e9f955592a9b2f8973c257d82aa4c00c831f836aa0550b7d413eaa6d89d` binds that exact current source;
no Mac22 success or reuse of historical Mac23 is implied. Fresh focused Linux
successor result `107c90a4fd7e5fed08f2120e613290ac05566d0033a009749a1ac2e0f549ee84` and independent review
`ff14ecb95a2e420d730dffb7c6a4f8704b814b3d48c7cdc521c7faf2c943b55d` bind that same final Task source before
canonical validation; they alone do not establish complete54 acceptance.

UI1.1c acceptance within its default-off trusted in-process host scope requires
the separate supplemental result `1a3c3af1f6ac534bd51990054755593bc3a9995b05484fd29d0a68ba9ef206f2` and independent
review `3c2c6740c3fdfd5d0f56b7079b4dbe49403c158d6c7a525e9a263611806666ac`, plus the fresh affected-consumer review below.
The sixth canonical Linux run `native-save-final-linux54-6f6e8667343a8e996c7fedc708a4378c` retains raw wrapper result
`3fe759b5e4a2ca76983f02b16f08f5675279edb80775ae4799f64e03e0787e2b`:
all 54 stages pass in
827.595754605 seconds, with all 54 canonical stage timing records
retaining actual argv, PID, exit and wait/reap outcomes without cleanup uncertainty.
The separate 55th outer timing record and actual owned outer birth, exit and reap
observation are retained. Task command and verifier-callback birth/reap identities
are retained separately. No native birth identity is claimed for each canonical stage timer.
The raw wrapper remains `INCOMPLETE_OR_FAIL_PRESERVED` with actual exit 1: its
shared-stdout parser at line 478 mistakes nested CI unit-test lane labels for
canonical stage labels. This is an acceptance-only classification refusal, not
a sixth canonical execution failure. Independent terminal-metadata review
`f68882546cfcbeef041954eef11aaaceebc8846f4f59d3cba2bf03c7eec68679` accepts all54 plus outer55 execution facts and unchanged
745-source closure; it does not substitute for the separate supplemental acceptance.
The original raw result, log and exit remain unchanged. Its integrated task packet `aMpP1aQU`, result
`6dcfa3fc1ea09a727191cad54a267506e6bcec67086e7a5f5887a9862a45db8d` and task review `38ea2526b181baae9f04149e810d3f6962da60f7aed7763d97cad3ccd3ab8369` bind the exact final Task source
`8f8fb22911c2696f323f36935a3920e1f1f6a0f4dc52be659b1a1c43954a6a4f` and retain
all eighteen unfiltered outcomes in 61.63 seconds, three
strict checks, all eleven command returns/reaps and complete 93-scenario reader
validation. The reader observes 727 ordered controls and
7 verifier callbacks in that run; these are actual
counts, not future inventory assumptions. The 745-source snapshot
`e3f16a3ce97cd87c90d1eb7474629b3546142039bf11c76e42f95db4c7ad0c36` matches before and after, and actual dependency
resolution is bound by `846c4662dd8e7bd59cc606f3b2e7887d0ff6f30e1d80fe682c198e6828f04ff4`. All five partial canonical Linux failures and
all preceding task failures remain retained. This acceptance supplies no missing
historical process rows and establishes no whole-CI or hosted speedup.

The full54 runtime snapshot precedes these four maintained milestone document
updates. The compiled Task203 closure remains byte-identical; separate planning
checks cover the documentation delta.

Owned backtraces retain failing callers without separate diagnostic reruns.

The reviewed host path also contains a bounded recording reader, pure manifest
verifier and owned verifier callback. Source reviews and focused component
controls validate their stated file/protocol/child-ownership scope. Source
admission covers the exact 203-input compiled and trusted closure.
`just foundry-desktop-editor-task-ui1-1c` is included in the accepted inventories
of 38 extended and 54 full stages; the prior 53-stage acceptance remains historical. Fresh canonical Linux
consumer results bind Desktop launch `4e3996984e2963fd7d85ef53bc72c889e6468b455cd9f8b39466974115af6825`, volatile editor
`7e5b0c4539bef513a423132c44e9b664f1af079aa9c011e4a6c8879d4f0dfd0a`, legacy Store `ea33d2a083b34274c3186d165cd2bc361a12ad58ca1ec4089ae3203f8540ba89`,
NativeRead `78f2374eb36e7e39617a1b60841c314c94f6a23205ad54d5182ce14cc908e057` and NativePreview
`0ebd85edd71246d816a86c9979682dba87e519b42ba6b1680369ca0cf112eb91`, with complete compact current-source
metadata/log review of trusted fresh gate outcomes
`bff3a0be5f64cf1735dae772cd24178f537d06feb8a245af5580dcee4d75a636`. Omitted launch children and legacy cases
are not independently revalidated raw trees by that compact review. No historical result is relabeled current;
the canonical Linux run preserves every previous stage and reruns all five consumers. Component checks
do not replace task execution, recordings, affected consumers or full preflight.
UI1.1d's independently reviewed [process contract](docs/contracts/editor-process-v0.json),
[API inventory](docs/DESKTOP_EDITOR_PROCESS_API_V0.md) and
[implementation plan](docs/plans/editor-process-v0.md) freeze the next Linux-only,
default-off boundary. They require actual child-local editing/raster, a sealed
executable launched through its held FD, kernel-authenticated messages, immutable
publication pairs and parent-owned original Save recovery. Thirty finite process assertion families are specified. Three reviewed private
smoke assertions reached actual missing-API compiler RED on Linux: only the 22
expected missing process exports/methods remain, with no assertion binary executed.
The failed predecessor probes remain INCOMPLETE; full process coverage and genuine
uncertain-owner cleanup are required before runtime execution.

The pure prerequisite is implemented in [desktop_editor_core](services/desktop_editor_core/src/lib.rs):
bounded ASCII editing/navigation, explicit legacy/native initializer/reset policies,
exact existing raster behavior and an explicit little-endian 88-byte process codec.
Protocol384/version1 is registered in [IDL](idl/portals/desktop_editor_process_v1.toml)
and generated through `just codegen`. Actual missing-module and missing-export RED
preceded implementation. `just foundry-editor-core-ui1-1d-prerequisite` passes on
macOS and Linux with all 26 unfiltered assertions: one generated-layout test,
sixteen model tests and nine codec tests. Formatting, IDL lint, kernel_api build
and all-targets core Clippy with warnings denied pass. Complete strict Linux
preflight passes all 54 canonical stages on the integrated candidate, including
workspace discovery, existing host consumers, Docker controls and QEMU. Required
planning gates also pass. Failed evidence-wrapper, lint and initial preflight
attempts remain retained. The initial preflight caught the missing new output in
the code-generation stability test; the reviewed correction adds it to the
independent expected roster and preserves all seven behavior tests. Fresh affected
CI validation passes 61 Python controls and seven Rust tests. Existing complete
workspace-test discovery includes the new prerequisite tests;
the canonical54/extended38 stage inventories are unchanged. The native task source
registry grows203 to207 for the new IDL/generated/layout/workspace inputs, preserving
all original assertions and existing dependency versions.

At that pure prerequisite milestone, existing Desktop adapters, process handlers,
authenticated transport, sealed publication and parent-owned Save recovery remained
dependent work. Pure data tests supply no input, process, FD, Focus or Save authority
and do not accept UI1.1d runtime.

### UI1.1d shared adapter migration — accepted host preservation

**Accepted adapter preservation.** Final macOS adapter4 and the unchanged
pure-core26/quality checks have independent actual-outcome reviews. CI62 Python
and seven Rust controls pass, including the missing-stage RED/GREEN control.
Fresh strict Linux55 passes on runtime source snapshot
`86c3ac3e20743cafe47604e69276396d3c8ada0c99517d3ff7d8ffe1202bc694`,
with original13/18, strict3, complete Reader93, all actual exits/reaps and stable
source/S2 inputs. Raw result
`f9342ae11a4d1187ec733fb4ad6647cd5ad21204732212b89c739aa4291fa760`
and independent review
`d266a33e9baf5078e704f5c40b7d9e2f6c4a99c89893ee1638b99d15151ed0e8`
bind this acceptance. Existing agent-task evidence remains limited to offline
synthetic provider accounting; Linux provider controls/reconciliation remain
INCOMPLETE. Required planning checks on the four-document successor will be recorded
separately from this runtime snapshot.

For this milestone:

Both existing `EditorClient` and `NativeTextEditor` adapters delegate pure
editing, navigation, scrolling and rasterization to the shared editor core.
Checked snapshots transfer all seven fields: body, cursor, selection, first
visible line, selection anchor, preferred column and text generation. Preserve
legacy/native initialization, reset, overflow and Save policies. Private native
construction and rasterization propagate errors; native editing computes in
detached scratch and checks actual owner admission and view-generation increment
before committing. The optional acyclic core dependency is enabled through
default-off `desktop_v0_dev`; the core holds no Desktop, Store, Focus or Save
authority. A local SaveRequested hint cannot create Save authority.

Four unchanged supplemental gate-first assertions cover both real adapters'
selected snapshot continuation, including nonzero scroll, and actual native
text/view MAX rejection with hidden owner state unchanged. They use genuine
Read/input fixtures and separately enabled private owner controls. The explicit
`just foundry-editor-adapter-migration-ui1-1d` host stage runs both required-feature
targets without filtering; ordinary default-feature workspace tests cannot stand
in for them. The deliberate insert after `desktop-editor-task` gives55 full/39
extended stages while preserving every original54/38 name, relative order, argv
and environment. The actual Task source registry successor adds ten legitimate
adapter/core/test/font paths,207→217, with no removals or historical-count
allowlist. Source membership does not establish execution. Original13/18 and
Reader93 requirements remain unchanged.

The accepted pure-core54 milestone remains valid for its source snapshot.
Its failures and all migration/CI predecessor failures remain retained. The
Linux55 run freshly executes the required feature targets and affected consumers;
Task217 source manifest
`42537d43fd8ce8b3afa2a0b55d30313c8d0428d3c135f25fc99cd513d0221817`
is exact and unchanged by these four documentation updates. The separate required
planning checks do not rerun or relabel the accepted runtime snapshot.

The next process packet completes all thirty executable assertion families and
ownership-safe runtime staging before dependent carrier/child/Save handlers.
Private cleanup-v5 is independently source-reviewed preparation only. Genuine
constructor/setup, Read/reopen, consuming-join bounds/non-unwinding owner return,
Save-pause panic and uncertain parent-death controls remain required. Retain actual
owners and charges when quiescence is unproved, preserve original cleanup
deadlines, and never infer child reap from watchdog parent death. Actual child
execution, authenticated channels, sealed publications and retained original Save
owners after child death remain pending. Target/QEMU ports, HIL and hosted CI
performance measurement remain separate; adapter preservation establishes no
process runtime or containment acceptance.

CI4 configuration remains complete; hosted scheduling/cache measurement is separate. The reviewed
[RUN0 entry preparation](docs/plans/run0-relocated-entry-preparation-v0.md) records
source-derived retention gaps and a single prior-run diagnostic candidate method.
Its collector/profile/schema are not frozen and capture remains INCOMPLETE.
Neither contract preparation establishes target execution or containment.

The eight planned preview identities retain their Save, recovery and IO2
dependencies. Those preview and pure prerequisites alone did not establish the
original eighteen-case integrated task; its host acceptance is described above.
The five reused volatile UI records remain separately stamped. Actual editor PID,
cold-start anti-rollback, device flush, power-loss durability, target execution and
runtime containment remain unproved. The Mac fixture requires its verified
writable Data-volume temporary-directory layout. The API contract and pure gate
alone supply no transaction authority. The Oracle packet adds no boot capture or
guest-runtime evidence.

Strict Linux preflight passes at `a05b0c6` with the reviewed compatibility cleanup correction,
including the 17-case boot admission and 17-case existing desktop launch gates.
The compatibility gate launches built Store/supervisor executables, handles
SIGTERM through the supervisor's child kill/reap path, and bounds teardown of
its own jobs. Forced or unproved shutdown fails with `UNKNOWN`. The real Store
and compatibility VM gates pass with all three serial markers and no remaining
owned QEMU process. `just foundry-compat-cleanup-s2` adds seven Linux private
process regressions, including interruption and missing-marker denials, with
unrelated-process survival checked through held pidfds. These regressions use
an ordinary process stand-in; they do not supply VM or general containment proof.
Test-only coordination of freshly written validator scripts also removes a
reproduced parallel-spawn `ETXTBSY` race without changing production supervision
or its timeout, result and descendant-cleanup assertions.

## Implemented foundations and their boundaries

| Area | Landed behavior | Evidence and limits |
|------|-----------------|---------------------|
| Kernel / S0–S8 | x86_64 and aarch64 boot, typed IPC, capabilities, shared-memory mappings, tracing and SPSC ring foundations | Selected target/QEMU paths; fixed-size tables. Capability-table use after SMP transition is deliberately blocked; general SMP/IRQ support remains incomplete |
| Boot admission / RUN0.0 prerequisite | Allocation-free full-map validation, seven retention reasons, sticky map-overflow denial and bounded deterministic pool selection | `just foundry-boot-frame-pool-run0-0` · [Contract](docs/BOOT_FRAME_OWNERSHIP_V0.md); 17 pure cases, kernel consumer tests/builds and existing S8 integration pass. No actual firmware-exit/collector/allocator or target-runtime proof |
| Typed interfaces | IDL/codegen, protocol/message IDs, bounded wire contracts | Native contracts are defined in `idl/`; generated syntax alone grants no authority |
| Desktop / UI1.0 | Host permission preview, single-use synthetic confirmation, exact self-observation grants, real pinned child, expiry/revocation/fault/restart and independent watchdog | `just foundry-desktop-host-launch-ui1-0` · [Contract](docs/DESKTOP_SESSION_V1.md); default-off trusted Unix fixture, 17 cases and retained process/wire evidence. No editor, compositor, Store, target or process-containment proof |
| Editor / UI1.1a | Logical keyboard editing, focus and preview approval, offscreen composition, volatile save/receipt and explicit recovery | `just foundry-desktop-editor-host-ui1-1a` · [Contract](docs/DESKTOP_EDITOR_HOST_API_V0.md); default-off trusted in-process fixture, 13 cases and source-bound wire/pixel/receipt evidence on macOS/Linux. Real Store IO, actual editor process, device input and target execution remain separate |
| Editor Store / UI1.1b | Private host CAS, atomic selection/receipt journal, irreversible admission and original-operation recovery through joined-owner reopen | `just foundry-desktop-editor-store-ui1-1b` · [API](docs/DESKTOP_EDITOR_STORE_API_V0.md) · [Recording contract](docs/contracts/editor-store-recording-v0.json); seven behavior and sixteen evidence cases pass on macOS/Linux, with captured bytes decoded by the native Rust codec. Integrated editor authority, actual editor PID, device/target durability and containment remain separate |
| Native Save data / UI1.1c prerequisite | Versioned Save grants and original Commit outcomes, canonical references and complete supplied Binding/receipt correlation | `just foundry-editor-native-save-codec-ui1-1c` · [Contract](docs/contracts/editor-native-save-codec-v0.json); eight pure host cases on macOS/Linux, strict std Clippy and no_std checks and 53-stage strict Linux preflight. Live mutation, Store IO, current-draft Saved and full task remain separate |
| Native Read / UI1.1c prerequisite | Approved editor peer, fresh Store Read, original request deadline, live copy authority and shared owned-producer roster | `just foundry-desktop-editor-native-read-ui1-1c-prerequisite` · [API](docs/DESKTOP_EDITOR_NATIVE_READ_API_V0.md); nine host cases on macOS/Linux, strict Clippy and feature exclusion. Integrated Save/task, optional-export transcript validation, editor PID and device/target execution remain separate |
| Native preview Read / UI1.1c precursor | Actual Store-current pin, one-use input approval, same-row activation/delivery, protected Read-only frames, live Store reads and retained-owner accounting | `just foundry-editor-native-preview-read-ui1-1c` · [Contract](docs/contracts/editor-native-preview-read-v0.json); five default-off trusted host cases on macOS/Linux, identical source/lock inputs, strict Clippy and feature exclusion. Finite source-bound log assertions; no universal exported transcript, integrated Save/full task, editor PID, target/device or containment proof |
| Native Save/task / UI1.1c | Focused keyboard editing, protected frames, actual Store Save/reopen and original-only failure recovery | `just foundry-desktop-editor-task-ui1-1c` · [Contract](docs/contracts/editor-native-save-task-v0.json); eighteen original host assertions and complete 93-scenario recordings pass on macOS/Linux with strict checks, exact affected consumers and full54 Linux preflight. Trusted in-process, default-off scope; editor child, target persistence, device and containment remain separate |
| Native runner / S10 | Host Wasmtime execution, manifests, granted-handle injection and guest deadlines | Host runtime; no complete target userspace loader or Wasmtime environment |
| Semantic State / S10 | Host snapshots, subscriptions/reactor, capability-filtered views; selected QEMU snapshot/IPC paths | Multi-source aggregation and target reactor remain incomplete; default boot/time metadata includes fixtures |
| Store / S1–S10 | Host CAS, signatures, durable ownership, path/tag queries, read-only projections and typed CoW commits | Full user launch/porting flow and target persistence remain incomplete |
| Execution fabric / S10 | Placement, lease, duplicate-observer, launch-plan and trace contracts | Synthetic nodes/load and simulated routing; no distributed transport |
| Driver Foundry / S11 | virtio-net Reference Vault, Linux Oracle capture, replay and typed harness transfers | Embedded Oracle packet vectors in QEMU; device-backed native send/receive unproven |
| Golden machine / S12 | Machine contract, GOP probe, HIL boot/IOMMU gate scaffolds and appliance tooling | First live appliance capture, AMT actuation and physical graduation pending |
| Storage / S13 | Block IDL, virtio-blk Oracle capture/replay and typed harness transfers; NVMe/atomic-update probes | Embedded sector vectors and QEMU scaffolds; native device read/write/flush and physical two-boot rollback remain unproven |
| Compatibility / S2–S9 | Separate Linux capsule VM, host POSIX and GPU quarantine paths with gates | Boundaries differ per runner; default POSIX profile is rlimits-only, not general containment |
| RamenOrg / G0 | Governance schemas, packets, renderers, validators, bounded trials and drift gate | A2-local only; no autonomous merge/release/hardware/public-support authority |

[PLATFORM_OVERVIEW.md](PLATFORM_OVERVIEW.md) explains responsibilities;
[SLICES.md](SLICES.md) defines slice scope. [SECURITY_STATUS.md](SECURITY_STATUS.md)
and [RISKS.md](RISKS.md) record residual risks.

## SW0: runnable evidence, not a completed experiment

The useful task repairs one configuration, runs a pinned WASM validator, commits
an immutable artifact, and denies named unauthorized operations. RT means RamenOS
typed, LT Linux typed, and LS Linux scoped shell. These gates use scripted
consumers and trusted host fixtures, not model trials or target-native task clients.

| Step | Implemented scope | Gate / contract |
|------|-------------------|-----------------|
| A0 | Pure transaction model and deterministic synthetic fixtures; no IO enforcement | `just foundry-agent-task-contract-a0` · [Contract](docs/AGENT_TASK_CONTRACT_V0.md) |
| A1.0 | Generated protocol-14 layouts and allocation-free request preflight | `just foundry-agent-task-protocol-a1-0` · [Protocol](docs/AGENT_TASK_PROTOCOL_V1.md) |
| A1.1 | Useful RT host service task, immutable staging, supervised validator, durable receipts, denials and replay | `just foundry-agent-task-proof-rt` · [Service proof](docs/AGENT_TASK_SERVICE_PROOF_V1.md) |
| A2.1 | Linux scripted repair, inspected Docker containment, pinned worker and named OS probes | `just foundry-agent-task-linux-control` · [Linux control](docs/AGENT_TASK_LINUX_CONTROL_V1.md) |
| A2.2 | Shared strict JSON codec/descriptions and opt-in RT IPC adapter | `just foundry-agent-task-adapter` · [Adapter](docs/AGENT_TASK_ADAPTER_V1.md) |
| A2.3 | Independent LT broker, grants, sealed validation, durable transactions and named RT/LT cases | `just foundry-agent-task-lt` · [LT backend](docs/AGENT_TASK_LT_BACKEND_V1.md) |
| A2.4 | Contained LS commands/launcher, shared Linux transactions, peer checks and original-receipt recovery | `just foundry-agent-task-ls-transactions` · [LS transactions](docs/AGENT_TASK_LS_TRANSACTIONS_V1.md) |
| A2.5 | Shared v2 pull/poll/cancel subscriptions and typed lifecycle comparison | `just foundry-agent-task-subscriptions` · [Subscriptions](docs/AGENT_TASK_SUBSCRIPTIONS_V2.md) |
| A2.6 | Finite canonical inventory, 33 common cases and separate LS OS probes; unknown authority retained | `just foundry-agent-task-authority` · [Authority](docs/AGENT_TASK_AUTHORITY_V1.md) |
| A2.7 | Synthetic bank/release contract, bounded sessions, 45 development attempts and retained failures | `just foundry-agent-task-evaluator-controls` · [Evaluator controls](docs/AGENT_TASK_EVALUATOR_CONTROLS_V1.md) |
| A2.8 | Named acknowledged-ID cleanup, unresolved-create quarantine and explicit interrupted-commit receipt recovery | `just foundry-agent-task-reconciliation` · [Reconciliation](docs/AGENT_TASK_RECONCILIATION_V1.md) |
| A2.9 | 31 issued-right subsets under two policies, single-right effects and named lifetime witnesses | `just foundry-agent-task-requestable-authority` · [Requestable authority](docs/AGENT_TASK_REQUESTABLE_AUTHORITY_V1.md) |

A2.6's frozen suite has zero successful forbidden probes, but does not prove whole
`E_max`, continuous `E(t)`, or narrower authority. A2.9 establishes equality only
for the declared-interface issued-right projection. LS mounted files, retained
observations/descriptors and raw broker access remain broader observations;
typed host clients and transitive deputy authority remain incompletely bounded.
A finite host-file witness now records nine named observations across RT/LT/LS
before expiry, after expiry and after revocation. The trusted Python evaluator/host
consumer in RT/LT reads an unrelated owner-only canary; the contained LS Python
consumer is denied access to that exact unmounted path. Actual PID/UID/GID and
namespace identities identify these actors, separately from adapters or model
interfaces. Read/grant/revocation witnesses bind the same resource, capability
and generation; malformed attribution is rejected. Both authority gates passed
in an isolated Linux/Docker checkout, with 33 common cases per arm and
31 right subsets under two policies per arm. This does not complete whole-authority
inclusion, continuous lifetime coverage or full A2.
Unacknowledged Docker create intents cannot certify cleanup from an empty inventory.

Portable SW-E accounting now freezes bank/release/context/provider/rate identities
and a finite three-arm schedule. Strict reports retain failed, unknown,
over-budget and pending attempts; integer uncached-token estimates cannot certify
the declared ceiling with missing usage. `just foundry-agent-task-provider-accounting`
checks ten unit assertions and a deterministic synthetic consumer. It makes no
provider calls and supplies no billing, attestation or funding authority. Combined
`foundry-agent-task-evaluator-controls` and `foundry-agent-task-reconciliation`
passed in an isolated Linux/Docker checkout with the pinned toolchain/image:
45 retained scripted repair attempts and six interrupted-commit cases, plus
late-create and abrupt-publication recovery assertions. Reports are retained
under `out/roadmap-linux/`; these checks do not complete full A2.

Full A2, real hidden-bank/study releases, actual provider supervision/usage capture, model
comparison, production registration and target task enforcement remain pending.
The [Agent Task Proof plan](docs/plans/2026-09-16-agent-task-proof.md) defines their
acceptance and the three separate contrasts. No comparative agent advantage is claimed.

## Recent boundary fixes

The StoreClient transport follow-up applies socket timeouts on initial and
replacement connections and one absolute deadline across response-frame reads.
Retained clients reconnect when peer closure is observable before dispatch;
transport failures discard the stream for the next explicit operation. Uncertain
requests, including ingestion, are never silently replayed. Real-server idle
expiry and fake-server lost/malformed/truncated reply and withheld/trickled
response regressions cover both ordinary reads and descriptor ingestion. This
is host transport evidence; connect and individual writes are not covered by
the response-frame deadline.

The 2026-10-04 follow-up defines `validation_current` as observation freshness in
RT/LT, independently of commit eligibility. RT direct/poll regressions cover failed
outcomes, truncation, expiry and revocation; portable LT predicate checks pass.
Expanded paired Linux cases run in the required Linux/Docker gates; local macOS
checks establish RT execution and portable LT predicates.
Ordinary Store preparation now has configurable byte/concurrency/deadline bounds,
runs outside the global registry/projection locks, and cancels on disconnect.
A two-client stalled-worker test checks unrelated reads and cleanup; publication
revalidates authority and preserves owner-bound intents. Durable publication IO
still uses the locks; total CAS quota and crash-orphan staging cleanup remain work.
The README now leads with the runnable RT host proof and a retained fixture result.
These changes add host evidence, not a model comparison or target/hardware claim.

The 2026-10-03 review changes are implemented and recorded in `CHANGELOG.md`:

- Store reads bind the requested content ID to authenticated metadata and blob bytes;
  native execution hashes the byte snapshot it consumes.
- Host ingestion uses caller-opened regular-file descriptors; pathname-only requests
  fail closed. Native ingestion retains its IDL shared-memory source contract.
- Owner/manifest publication intents precede CAS visibility; restart/retry recovers
  partial publication and unrelated orphans remain denied.
- Native Unix/chardev IPC shares the invocation's absolute deadline through connect
  and partial transfers; uncertain dispatch is not automatically replayed.
- LT duplicate staging preserves capability/validation and counts unique candidates,
  matching RT; portable and executable regressions cover capacity.
- CI and preflight use the same complete implemented SW0 sequence. This does not
  turn that sequence into full A2 conformance or a completed model study.

Earlier memory, tracing, WASM, projection and durability fixes remain in the
changelog and their contract/gate documents. They establish host/QEMU behavior,
not complete SMP, client isolation, physical durability or production assurance.

## Physical inventory and graduation boundary

The x86_64 COM1 console uses 115200 8N1, matching the HIL appliance contract
and capture tools. The S12 GOP gate checks the emulated UART's programmed
parameters in a QEMU trace; first live Pi/ThinkCentre validation remains pending.

The pinned reference is the Lenovo ThinkCentre M900 SFF, machine type 10FH,
Core i7-6700, 8 GiB RAM, with a 240 GB SanDisk SATA SSD. The Raspberry Pi 4
(4 GiB), FTDI USB-to-RS-232 adapter and null-modem chain are physically installed.
Firmware/AMT preflight and the first live serial capture remain pending; a compatible
M.2 2280 PCIe NVMe drive is still required for S13 graduation.

`PASS/QEMU` is not metal evidence. `PASS/HIL-LOG`, `PASS/HIL-LIVE`,
`PASS/HIL-APPLIANCE`, and `PASS/METAL` have separate provenance requirements in
[EVIDENCE_LEVELS.md](EVIDENCE_LEVELS.md). Standalone `operator-golden-machine`
and `appliance-mediated` metal claims must be stamped separately.

S13.8 currently probes A/B metadata. Graduation still requires an implemented
publication/readback/selection verifier, a new-slot boot and a separate rollback
boot with fresh nonces and matching artifact identities. Firmware NVMe detection
and vector-backed block transfers establish neither native NVMe I/O nor that protocol.

## Documentation maintenance

The 2026-10-04 roadmap review replaces global sequencing barriers with bounded
parallel work and explicit integration checkpoints. The coordinator owns the
shared contract and status files; sub-agents own disjoint changes and return
gate evidence for review. Project skills share one source across agent clients.
The governance drift gate now checks that agent instructions route to maintained
planning owners, with negative cases for missing links and duplicated queues.
This adds workflow/documentation validation, not OS, model, or hardware evidence.

The 2026-10-03 documentation review consolidates status here, execution criteria
in `NEXT_TASKS.md`, and direction in `ROADMAP.md`. Current references and agent
skills were reconciled with source/gates; historical security plans are archived
behind maintained references at their existing paths. This is documentation work
and adds no runtime, model, hardware, security or release-readiness evidence.

## Validation entry points

```bash
just s11
just s12
just s13
just hil-appliance
just foundry-org-governance-g0
```

SW0 gates above expose their individual fixture scopes. Full `just preflight`
requires Linux, Python `jsonschema`, Docker/seccomp, and the installed pinned image.
Physical gates are opt-in and require documented preparation/provenance.

Strict Linux preflight passed for the committed UI1.0/accounting/finite-authority
batch `a44993e` with the pinned compatibility kernel and Docker image. Its isolated
checkout matched that revision's tracked sources except two trailing spaces in
the desktop contract; source/artifact digests and the difference are retained in
the coordinator checkpoint. This run predates the subsequent RUN0.0 pure changes;
their focused gates, consumer checks and target builds are recorded separately.
