# Updated-review foundation acceptance preparation

**Date:** 2026-10-14
**Status:** Implemented candidate; final assembled qualification pending

The user selected the updated-review P1/P2/P3 foundation follow-ups before further
editor-process implementation. These review packet labels are distinct from the
process P1/P2/P3 sequence in [Next Tasks](../../NEXT_TASKS.md). The in-process
human editor remains the consumer to preserve while the foundations are qualified.
PR #40 and PR #41 evidence qualifies preceding source only; changed inputs require
fresh assembled assertions and matching hosted qualification.

## Implemented boundaries

| Boundary | Candidate behavior | Required behavior, denial and recovery |
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
implementation. Independent candidate review found and repaired noncanonical close IDs and default Cargo executable ambiguity. Actual AArch64 target execution also exposed a table-sized boot-stack temporary: its compiled frame exceeded the 64 KiB stack. Bounded in-place fixture reset avoids that temporary while retaining the existing fixture ownership limits.

Earlier fixture/compiler failures remain separate preparation;
they do not qualify an assertion or establish target behavior.

## Qualification

Assembled local checks run in an isolated clone on macOS arm64 with the pinned
nightly-2026-02-08 toolchain, two compiler jobs and one test thread. Kernel332,
API including31 generated codec inventories and four compile-fail type controls,
codegen/content stability and strict Clippy for kernel/API/Store pass. The extended
boundary gate passes every nonempty selected family, including15 strict Store
process tests. Both S8 architectures pass all prior17 MMU assertions and the new
wire/name, fixed-retention and original-outcome probes. Affected editor consumers
and matching hosted qualification are still pending; this record is not final
acceptance until those complete.

| Check | Current acceptance state |
|-------|--------------------------|
| Host API/kernel/Store and codegen | Assembled suites and strict kernel/API/Store Clippy pass locally |
| Extended `just foundry-review-boundaries` | PASS with nonempty inventories and owning Store suite |
| Both-architecture S8 target probes | PASS on x86_64/AArch64 after boot-fixture reset repair |
| Affected original editor and recording consumers | Fresh assembled preservation pending |
| Required documentation/governance checks | Pending on assembled candidate |
| Matching hosted qualification | Pending; preceding PR results do not qualify this source |

The extended boundary gate keeps previous assertions and adds nonempty discovery
for generated codecs, wire padding/validity, UTF-8, fixed capacity, retained
outcomes and Store drains/epochs. S8 retains previous markers and requires new
wire/name, trace-capacity and Shmem-outcome markers on both architectures. Source
registry successors preserve every previous path and assertion pin while adding
the two IDLs/generated outputs, wire tests, offline operator binary and ordinary
executable-selection consumer. The independent codegen catalogue remains aligned
with the generation script. Compiler cache reuse does not reuse acceptance results.

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

After foundation qualification, resume the existing full P0 assertions and real
editor-child sequence with retained Save ownership. RUN0 firmware exit/retained
ranges, target Store grants and persistence, native input/storage, default POSIX
containment, physical durability and hardware graduation remain separate. No
provider/model trial or physical actuation is required for this host qualification.
