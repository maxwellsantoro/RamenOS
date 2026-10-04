# SW0 A2.5 — shared typed subscription lifecycle

**Status:** opt-in host RT and Linux LT implementations, scripted development
fixture evidence. **Gate:** `just foundry-agent-task-subscriptions` (Linux).
Physical hardware, model trials, full authority conformance and target integration
remain pending.

The typed adapters expose the same eleven-operation version 2 tool contract via
`--describe-v2`. Version 2 includes the eight existing transactions and three
explicit subscription operations. `--describe` retains the exact version 1
artifact and its eight operations. Valid requests and replies carry their chosen
`schema_version`; the bootstrap retains version 1. A malformed request rejected
by the shared codec receives the existing neutral version 1 `invalid` response
with null request ID/result. Neither transport emits unsolicited JSON events.

```json
{"schema_version":2,"request_id":"8","call":{"operation":"subscribe_task","task_cap":"cap:0123456789abcdef","event_types":["output_changed","validation_changed"]}}
{"schema_version":2,"request_id":"9","call":{"operation":"poll_task","task_cap":"cap:0123456789abcdef","subscription_cap":"cap:fedcba9876543210"}}
{"schema_version":2,"request_id":"10","call":{"operation":"unsubscribe_task","task_cap":"cap:0123456789abcdef","subscription_cap":"cap:fedcba9876543210"}}
```

## Authority and lifecycle

| Operation | Required authority | Authorized result |
|-----------|--------------------|-------------------|
| `subscribe_task` | Current OBSERVE grant for this task | Opaque subscription capability and starting revision |
| `poll_task` | The subscription's original current OBSERVE grant, same domain and adapter connection/session | Ordered coalesced event types and a fresh state snapshot, or an empty list and null state |
| `unsubscribe_task` | The same original current OBSERVE grant and connection/session | `cancelled: true`; subsequent use returns `not_found` within authorized scope |

Event types must be a unique nonempty subset of `output_changed` and
`validation_changed`. Output events correspond to a new successful commit;
successful receipt retries do not create another effect/event. Validation events
correspond to recorded validator observations, including invalid results. Staging,
state reads and denied operations do not generate these events. Subscription
creation has no initial event; explicit state reads provide initial/resynchronized
state. There is no background tool invocation or repair/retry policy.

Each connection/session retains at most 16 pull subscriptions. Each subscription
retains at most two pending types, irrespective of how many changes occur before
polling. Types return in output-then-validation order. Polling supplies one fresh
snapshot reflecting current accepted revision, pins, the observing grant's rights,
and validation freshness. It does not retain historical snapshots or promise one
message per transition. Filtering event types does not narrow the snapshot fields
already authorized by OBSERVE.

Authorization is checked before existence lookup and state delivery. Another live
grant cannot take over a subscription. Wrong rights/domain/grant/connection return
redacted `denied`; expired authority returns redacted `expired` or `denied` after
registry reclamation. Generation revocation removes pending subscriptions and
queued observations. Revocation counts live grants, excluding expired entries.
Explicit cancellation frees a slot. EOF/disconnect discards this connection's
subscriptions; reopen/restart never restores them from the transaction journal.
Durable candidates and receipts retain their existing recovery semantics.

Polling drains pending types once snapshot mapping succeeds. Notifications have
at-most-once delivery: a lost poll reply can consume them. Read current state
explicitly to resynchronize. Notification delivery has no durable acknowledgment,
replay log, cursor, ordering across different subscriptions or automatic retry.
IO uncertainty still poisons the transaction backend; polling grants no authority
to bypass recovery checks.

## Implementations and scope

RT maps these operations onto additive protocol-14 IDL message pairs 21/22
(`subscribe_task_pull`), 23/24 (`poll_task`), and 25/26 (`unsubscribe_task`). They
use generated bindings, allocation-free request/reply preflight, typed control
messages and read-only shared-memory snapshots. Existing push subscribe 17/18
and event 19 retain their meaning. Pull subscriptions receive the same service
change signals, but never enter the push emitter. Polling checks connection,
domain and original grant under the service lock, then returns/drains bounded
pending state. The adapter consumes/releases every nonempty reply mapping.

LT independently implements this lifecycle in its private Linux broker; it does
not import RT enforcement or the A0 reference state machine. Subscriptions are
session-local in-memory objects, excluded from its durable CAS/receipt journal.
Both adapters share descriptions, codec, schemas and response serializer. The
codec is default/backend-free; both executable backends remain feature gated.

LS's conventional helper still exposes eight version 1 commands. A raw LS client
can send version 2 packets to the shared Linux broker; those subscriptions live
for the whole LS launcher session, across command socket disconnects. This
additional observable authority and lifecycle difference belong in the upcoming
all-arm inventory. This milestone establishes the RT/LT typed lifecycle only.

## Evidence and next work

The standalone gate tests generated wire bounds/redaction, native cross-connection
denial and disconnect removal, adapter capacity/cancellation/restart/mapping
cleanup, and an independent executable/schema consumer. The latter compares
50 operations per arm, including repeated validation coalescing, filtered output
events, observer-only snapshots, grant substitution, queued-event revocation,
expiry, cancellation, capacity reuse, malformed requests and orderly EOF/restart.
Version 1's description hash remains
`24af8e0fe29809ee8a5b849f4fda1af1b3d5e2430437e96d12a140bd04498290`.

Comparison aliases opaque handles consistently and excludes declared clock/
duration fields. Only the predeclared expiry case allows `expired` versus
`denied`; every other status, rights set, event type, byte/hash/revision and
validation outcome must agree. This finite check does not establish complete
protocol equivalence or canonical authority conformance.

`out/agent-task-subscriptions/` records private evaluator transcripts, fixture,
source/lockfile fingerprints, tool/binary/worker hashes and a report. Keep these
artifacts outside model context. Canonical available/exercised authority mapping,
all-arm negative fixtures, hidden-bank partitioning and evaluator session/cost
supervision are next. No paid/model or physical run is part of this gate.
