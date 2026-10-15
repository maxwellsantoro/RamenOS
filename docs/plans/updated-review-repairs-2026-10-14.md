# Accepted updated-review foundations and qualification

**Date:** 2026-10-14
**Last Updated:** 2026-10-15
**Status:** Six bounded foundation findings accepted through PR #42

The user selected the updated-review P1/P2/P3 foundation follow-ups before further
editor-process implementation. These review packet labels are distinct from the
process P1/P2/P3 sequence in [Next Tasks](../../NEXT_TASKS.md). The in-process
human editor remains the consumer preserved by foundation qualification.
PR #40 and PR #41 evidence qualifies preceding source only; changed inputs require
fresh assembled assertions and matching hosted qualification.

## Implemented boundaries

| Boundary | Accepted behavior | Behavior, denial and recovery assertions |
|----------|--------------------|----------------------------------------|
| Wire and names | Generated explicit field codecs initialize implicit padding and unused bytes. Decode requires an approved codec; bounded names preserve UTF-8 boundaries and expose checked decoding. | Field/padding assertions, unsupported raw-copy/bool/enum/reference compile failures, valid name truncation, malformed stored-name denial and next valid registration. |
| Trace capacity | Protocol 534/v4 reports fixed 64-event/1536-byte retention, with a separate 24..1536-byte limit divisible by 24. Delivery/Commit stays protocol 532/v3. V1 requested-size metadata remains deprecated compatibility behavior. | Exact 64-event bytes, 65th-event overflow, stale read/Commit denial without consumption, metadata resync and next successful Commit; committed retry preserves its original cursor. Short delivery does not shrink retention; unrelated domains progress. |
| Shmem original outcomes | Protocol 533/v3 binds a nonzero owner operation key to exact offer fields. Queries and acceptance retries return the original outcome without remapping. 64 retained receipts survive unmap, close and slot reuse until both participants acknowledge terminal results. | Lost Offer/Accept replies, one reference after repeated acceptance, role/header/IPC denials with zero metadata, changed-payload key denial, joint acknowledgement and next successful offer. Noncanonical close denial preserves pending consent; canonical close and successor mapping recover. |
| Host Store operation lifetime | Per-ID admitted leases drain before revoke acknowledgement; every concurrent retry joins the drain. Waiting releases the authority mutex. Reads recheck health before success; publication excludes authority poisoning through durable commit. | Same-ID/concurrent-retry ordering, unrelated management during an actual FIFO-held GetBlob read, uncertain-health denial after admission, panic accounting cleanup, fresh successful publication and bounded development accounting. |
| Host Store capacity/key retirement | Explicit offline revoke-all epochs retain at most 1024 distinct IDs per epoch and at most 64 retired keys. Fresh exclusive keys and an exact external checkpoint pin separate epochs; artifacts remain while grants reset. | Held-owner denial, stale/source/history/signature/key-reuse/floor rejection, actual operator CLI rotation and strict restart, old operation denial, fresh grant/publication/read, bounded-history and uncertain post-rename handling. Ordinary Cargo launch still selects the service. |

The authoritative new interfaces are
[trace capacity v4](../../idl/harness/trace_capacity_v4.toml) and
[Shmem outcome v3](../../idl/harness/shmem_outcome_v3.toml). The
[Store operator contract](../STORE_AUTHORITY_EPOCH_V1.md) owns transition encoding,
exclusive key provisioning, external floor configuration and interrupted cutover.
The JSON proof reader checks shape and signature lengths; migration verifies both
signatures against the source and fresh keys before persistence.

Gate-first evidence distinguishes missing-API compiler RED from actual behavior
RED. The native main-route consumers failed on missing typed replies before route
implementation. Independent review found and repaired noncanonical close IDs and
default Cargo executable ambiguity. Actual AArch64 target execution also exposed
a table-sized boot-stack temporary: its compiled frame exceeded the 64 KiB stack.
Bounded in-place fixture reset avoids that temporary while retaining the existing
fixture ownership limits. Those initial failures remain evidence; they were not
reclassified as runtime acceptance.

Earlier fixture/compiler failures remain separate preparation;
they do not qualify an assertion or establish target behavior.

## Qualification

PR #42 qualifies reviewed runtime head
`49ea1d403b4bc59fdcd13122c8726b2434230c93`. Merged main
`b41c43addf6fe3c5f5782b48d2a2a332b0f192f4` has the identical tree
`b9367fad0b2d0e7e8fa3f8b918ca779934867964`. PR #40/41 results remain historical
predecessor evidence; they were not reused as acceptance of this changed source.

Assembled local checks ran in an isolated clone on macOS arm64 with pinned
nightly-2026-02-08, two compiler jobs and one test thread. 332 kernel tests, 31 generated
API codec inventories, three wire assertions, four compile-fail type controls,
codegen/content stability and strict kernel/API/Store Clippy pass. The extended
`just foundry-review-boundaries` passes its existing controls and nonempty new
families, including 26 native and 13 Store boundary assertions. The actual strict
Store suite passes all 15 cases. Both-architecture
`just foundry-shmem-dataplane-s8-phase4-integration` passes all prior 17 MMU
assertions plus wire/name, fixed-retention and original-outcome probes.

Affected consumer preservation passes freshly: volatile13, Store7, Preview codec7,
Save codec8, NativeRead9, NativePreview5, adapter4, original task18, actual Reader93
scenarios and reliability13. Local S11/S12/S13 and org-governance also pass. These
counts name distinct maintained inventories; they do not combine into a universal
OS-security or target-readiness score.

| Check | Accepted evidence |
|-------|--------------------------|
| Host API/kernel/Store and codegen | Assembled suites, codec/type controls, content stability and strict kernel/API/Store Clippy PASS locally |
| Extended `just foundry-review-boundaries` | PASS with all previous controls, nonempty inventories and owning Store suite |
| Both-architecture S8 target probes | PASS on x86_64/AArch64 after boot-fixture reset repair |
| Affected editor and recording consumers | All named original/affected inventories PASS, including actual task18/Reader93 and reliability13 |
| Required documentation/governance checks | S11/S12/S13 and org-governance PASS locally |
| Matching hosted qualification | Push, PR and post-merge runs each PASS all nine jobs and 68 canonical records on their first attempt |

The [matching push run](https://github.com/maxwellsantoro/RamenOS/actions/runs/38674300567),
[PR run](https://github.com/maxwellsantoro/RamenOS/actions/runs/38674334354) and
[post-merge main run](https://github.com/maxwellsantoro/RamenOS/actions/runs/38675734768)
each passed all nine jobs and all 68 exact canonical lane/stage records on the
first attempt. Hosted artifact collection verified the full expected record
roster; a green summary or partial artifact set was not substituted for it.
Retained command logs, negative controls and target serial preserve the source,
environment, initial failures and repaired outcomes. Independent technical reviews
preceded acceptance; qualified host/QEMU results retain their stated boundaries.

The extended boundary gate keeps previous assertions and adds nonempty discovery
for generated codecs, wire padding/validity, UTF-8, fixed capacity, retained
outcomes and Store drains/epochs. S8 retains previous markers and requires new
wire/name, trace-capacity and Shmem-outcome markers on both architectures. Source
registry successors preserve every previous path and assertion pin while adding
the two IDLs/generated outputs, wire tests, offline operator binary and ordinary
executable-selection consumer. The independent codegen catalogue remains aligned
with the generation script. Compiler cache reuse does not reuse acceptance results.

## Separate tooling follow-up

An additional local tooling rerun after PR #42 exposed an empty PID-marker race
in the interruption fixture, then an absent serial file in a timeout fixture
that assumed Python would start and write within 50 ms. Both are retained actual
failures. The test-only repairs publish the complete PID atomically and establish
actual child serial output before returning from the spawn wrapper. The observer's
50 ms budget still starts before spawn and is not reset or extended; timeout,
reap and real-output retention assertions remain. Production timer/observer and
runtime source are unchanged. Independent narrow reviews found no issues.

The complete local tooling suite then passed all 149 Python and seven Rust
controls. The two fixture files have their own PR #43 at
`df64395bea8972166761a5493ad3e9249bec5a72`, with full hosted CI separate from the
accepted runtime tree above.

Both the [push run](https://github.com/maxwellsantoro/RamenOS/actions/runs/38676395342)
and [PR run](https://github.com/maxwellsantoro/RamenOS/actions/runs/38676398578)
passed all nine jobs and 68 verified records on their first attempt. PR #43 merged
as `3a876fd2e6789798d48c3dd1f160f4c37aa6fad6`, identical to the reviewed fixture
tree `ea34c973a01388d8409de30190722cdd5652f156`.

**Retained post-merge failures**: the matching
[post-merge run](https://github.com/maxwellsantoro/RamenOS/actions/runs/38677419361)
first encountered an LT validator Docker attach timeout (2421 ms). The container
was removed with 30 ms cleanup; the failed stage records confirmed child reap and
no cleanup uncertainty. The editor lane also failed in the final reliability
gate with only a generic owned-command diagnostic; its cause is unclassified.
Both failures and all first-attempt records are retained. The first unchanged
retry hit the same validator attach timeout (2421 ms), with removal confirmed
and 49 ms cleanup. The complete editor retry passed all its assertions, including
the final reliability stage. The third unchanged agent attempt also failed at
the same request with a 2409 ms attach timeout and confirmed removal (35 ms
cleanup). No production deadline, containment setting or assertion was relaxed.

PR #44's initial diagnostic head `0023d1a` exposed the concrete budget use:
Docker creation took 2314 ms, inspection completed at 2325 ms, and only about
97 ms remained of the 2422 ms sandbox budget. Input was written and closed;
310 stdout bytes arrived, but the complete EOF/CLI-exit contract had not finished.
Those bytes remain unusable as a validation result. The push and PR runs failed
this same required assertion, retaining the new progress evidence. The internal
cause of Docker's slow create remains unclassified.

PR #44's successor head `6086da350e1fbe1b35b67b6b7ae2f2cfb6b53c0a`, tree
`0c711523e22e7ec679f4e106c6b8a6a3b7627fee`, adds an explicit one-time TEST
recovery scenario. After a confirmed timeout it asserts removal, no uncertain
cleanup or poisoned task, unusable timeout validation, unchanged candidate and
accepted bytes/revision, and denied commit without a receipt. Exactly one fresh
validation request must then return the required `invalid` result under the
unchanged budget. Both attempts are printed and retained, and all subsequent
transaction/retry/restart/revocation assertions remain. This changes the fixture's
availability expectation; it does not fix cold Docker latency.

The launcher gains no automatic retry, sleep, Docker probe or execution/cleanup
budget extension. Five failure-evidence controls pass locally, and separate
read-only reviews found no concrete regression. A forced-branch diagnostic passes
the selected full transaction test, using the current Python source and an older
compatible worker binary; it does not qualify the current compiled worker.
Isolated Linux probes on Docker 29.8.2 did not reproduce the hosted Docker 28.0.4
cold-create delay and supply diagnostic evidence only.

The revised [push run](https://github.com/maxwellsantoro/RamenOS/actions/runs/38680308019)
passes all nine jobs and 68 verified canonical records on its first attempt.
The [PR run](https://github.com/maxwellsantoro/RamenOS/actions/runs/38680311935)
passed quality, host and agent on its first attempt. The agent lane exercised the
actual timeout recovery: create 2312 ms, timeout 2421 ms, removal confirmed, then
a fresh invalid validation in 135 ms. The PR's first QEMU attempt reached kernel
readiness but timed out in the unchanged snapshot bridge; its first editor attempt
reported only an opaque owned-command failure in the task gate. Both remain
retained and unclassified. Both lanes passed the matching push run and their
one unchanged PR rerun. The final PR conclusion is nine
successful jobs with 68 verified canonical records (quality/host/agent attempt 1,
QEMU/editor attempt 2). PR #44 merged through the separate A3 identity as
`3c665db5154e2001f2403cdd32b60b3002310371`, with the same tree as reviewed head
`6086da3`.

The [PR #44 post-merge run](https://github.com/maxwellsantoro/RamenOS/actions/runs/38681806877)
exposed a fixture assertion gap: a completed, exit-zero sandbox run can still be
rejected by the broker when total operation time including cleanup exceeds
2500 ms. That record has no sandbox `failure_reason`; the recovery test raised
`KeyError` before testing recovery. This failed run remains retained and is not
called accepted post-merge qualification.

PR #45, head `dc933e016b430b6e0f10c57327b9f5ecfef65d52`, tree
`b9f918b5ac49c3822d9f8557b20efac9f183b28d`, changes only that test. Its alternate
route requires exit zero and a persisted timeout validation with aggregate wall
time strictly greater than the unchanged budget. All cleanup, denied-commit,
unchanged-state and one-fresh-request assertions remain. The recovery diagnostic
also retains the rejected aggregate wall time. Both routes pass forced-branch
controls against the real Python backend and subsequent Docker worker calls;
those older-worker controls are diagnostics, not current-head qualification.
Separate narrow review found no concrete regression.

The [PR #45 first PR attempt](https://github.com/maxwellsantoro/RamenOS/actions/runs/38682355383)
hit a Docker create timeout before acknowledgement (2404 ms). Although removal
inspection reported absence, creation remained unconfirmed; the broker correctly
returned `io`, poisoned the task and required reconciliation. The test rejected
that outcome rather than treating uncertainty as recoverable. This failure is
retained; no retry through the poisoned task or production budget change was
introduced.

The [PR #45 push run](https://github.com/maxwellsantoro/RamenOS/actions/runs/38682329153)
passes all nine jobs and 68 verified canonical records on its first attempt.
Quality, host, QEMU and editor also passed the PR's first attempt. The one
unchanged agent-only rerun in a fresh CI fixture passed; the final PR conclusion
is nine successful jobs with 68 verified records (agent attempt 2, other four
lanes attempt 1). PR #45 merged through the separate A3 identity as
`ce61077956b74971119467d23ae7845581236354`, with the identical reviewed tree.
The matching [post-merge run](https://github.com/maxwellsantoro/RamenOS/actions/runs/38684043666)
passes all nine jobs and 68 verified canonical records on its first attempt.
This qualifies the assembled runtime tree `b9f918b5ac49c3822d9f8557b20efac9f183b28d`
on merged `ce61077`, including the corrected recovery fixture. Earlier failed
attempts remain retained. No uncertain creation was accepted as recovery, no
partial worker result was accepted after timeout, and no production deadline,
authority check or containment setting was relaxed. The final Markdown update
also changes `NEXT_TASKS.md`, an explicitly executable Foundry input. Its path
classification therefore requires fresh full five-lane Foundry qualification,
even though runtime source is unchanged; no lane waiver is requested.

## Limits and next dependency

Caller identity still comes from trusted kernel fixtures; no authenticated
process/syscall carrier, protocol-specific IPC grants or production trace-client
carrier is established. MMU mutation requires exclusive hierarchy access; target
markers do not establish SMP synchronization or active userspace translation.
Shmem joint acknowledgement can impose receipt-capacity backpressure, and trace
overflow requires explicit resync.

Store authority remains an ordinary host implementation. GetBlob returns paths;
already delivered paths/bytes and same-UID filesystem access cannot be retracted.
Per-ID drains do not promise bounded disk latency, management progress during
stuck authority fsync/publication, or unrelated data progress through other locks.
Offline rotation revokes all grants, requires exclusive root/epoch keys and an
operator-maintained pin outside Store, and supplies no hardware monotonicity.
An actor replacing both state and trusted configuration defeats that anchor.

With foundation qualification complete, resume the existing full P0 assertions
and real editor-child sequence with retained Save ownership. RUN0 firmware exit/retained
ranges, target Store grants and persistence, native input/storage, default POSIX
containment, physical durability and hardware graduation remain separate. No
provider/model trial or physical actuation is required for this host qualification.
