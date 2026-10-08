# Desktop session v1: UI1.0 host launch contract

**Status:** UI1.0 implemented and independently reviewed; host gate passed
**Scope:** default-off Unix host development, real non-rendering witness process

This narrows the [accepted UI0 design](plans/desktop-v0.md) to a permission preview,
single-use confirmation, a real host child and its lifetime. Editor, input driver,
compositor, Store IO and target runtime are separate consumers. The selected fixture
hash/revision is plan metadata only; no artifact access is granted by this packet.

The canonical layouts are [desktop_session_v1.toml](../idl/portals/desktop_session_v1.toml).
Protocol `336` (`0x150`) selects version 1; incompatible versions need a distinct
protocol ID. The existing Envelope has no version field. Message IDs 1/2 prepare,
3/4 cancel, 5/6 confirm, 7/8 status, 9/10 close, 11/12 revoke and 13/14 restart are
request/reply pairs; 15/16 are reserved; 17/18 are child observation; 19 is the
service-to-child bootstrap. Generated layouts alone enforce no authority.

All messages fit 64 bytes. Explicit per-field little-endian codecs initialize all
bytes, require exact lengths and reject nonzero reserved fields/payload tails.
The inherited-pipe host frame uses the existing length-prefixed 88-byte envelope
representation, with a zero outer four-byte pad. Raw packed-handle reserved bits
and kind/generation are checked before normalization by `Handle::unpack`.
Unknown protocol/operation fails closed. Bulk preview bytes use a bounded read-only
host object descriptor; this fixture object is not a target shared-memory mapping.

## Authority and state

The trusted fixture broker registers two sessions at most, one active preview and
one live child each. Preview TTL is 1–30000 ms, instance TTL 1–600000 ms and the
independent child deadline 1–5000 ms. Each child retains at most 64 exchanges;
16 total instance slots reserve terminal-evidence capacity, including active children;
no evidence is evicted. Admission fails Exhausted before
exceeding these bounds; exhausted identity counters never wrap into validity.
It creates opaque peer contexts bound to its owner/endpoint
registry. Roles and peer/PID identity are not parsed from application payloads.
The parent creates the child context from its actual `Child` and owned pipes.
The Rust host fixture, its owner and process environment remain trusted; this is
not general process containment, an OS-authenticated IPC service or target kernel
capability enforcement.

| Endpoint class | Bit | Operation |
|---|---:|---|
| Trusted chrome | 1 | Prepare preview |
| Trusted chrome | 2 | Inject a fixture confirmation event and confirm |
| Trusted chrome | 4 | Cancel preview |
| Trusted chrome | 8 | Observe owned instance status |
| Trusted chrome | 16 | Close owned instance |
| Trusted chrome | 32 | Revoke owned instance |
| Trusted chrome | 64 | Prepare a restart preview |
| Child observation | 1 | Observe that child instance only |

Endpoint class, owner, session/instance and generation checks precede numeric-right
checks. Equal bit values across classes do not grant the other class's operations.
The exact approved child grant is `APP_OBSERVE_SELF=1`; unknown/requested broader
rights are denied. No file, Store, surface, focus, keyboard or confirmation right
is issued to the witness. Unrelated sessions cannot read a preview or instance.
Denied replies contain no foreign hashes, IDs, handles, PIDs or state.

Status values are Ok 0, Denied 1, Invalid 2, Unsupported 3, Stale 4, Exhausted 5,
NotReady 6, Conflict 7, Timeout 8, Disconnected 9, Unknown 10, Internal 11. Unused
statuses are reserved for their defined future meanings. Lifecycle values are
Idle 0, Preview 1, Running 2, Exited 3, Faulted 4, Revoked 5 and Expired 6.

Prepare binds the pinned executable hash, selected fixture hash/revision, policy
revision, requested exact rights, session generation and expiry in an authoritative
internal plan. A read-only preview describes that plan; writable caller bytes never
become the grant source. Each plan and confirmation event is single-use.

`inject_chrome_confirmation` is an explicit default-off trusted fixture route that
models one Enter press. It requires an opaque registered chrome context and CONFIRM
endpoint, and binds session/generation/plan/preview revision/expiry. Possession of
the endpoint without that event cannot launch. Confirm revalidates all frozen
identities and consumes the event and plan. Cancel, stale policy/fixture revision,
expiry and closure invalidate the event. Child or foreign-session injection fails.
A real keyboard/focus route and rendered trusted chrome remain later UI1.1/IN0 work.

The shared preview is fixed at 136 bytes: seven u64 fields (session ID/generation,
plan ID, preview revision, selected revision, policy revision, expiry), two bytes32
hashes (application and selected fixture), then schema_version/granted_rights/
total_len/reserved u32 fields. Version is 1, length is 136 and reserved is zero.
The reader checks the descriptor owner, lifetime and these fields before access.

## Execution, lifetime and observations

The host fixture pins the actual witness binary SHA-256 and checks the bytes it
executes. A private executable snapshot is preferred; any trusted host-path race
left by a platform must be reported explicitly. A real distinct PID and completed
typed observation exchange demonstrate execution; a marker or invented PID cannot
satisfy launch acceptance. `WitnessMode` selects Observe, Fault or Stall paths in
the same pinned fixture binary. A Flood mode emits 65 self-observation requests:
at most 64 exchanges may be accepted/retained; hitting the bound stops and reaps
that child with an explicit `exchange_limit_hit` evidence flag. These are deterministic fault inputs, not model
or target application runs.

The inherited pipe carries generated bootstrap/observation messages. The parent
binds dispatch to the actual owned child and endpoint. A bounded background
supervisor enforces an absolute real deadline even without a subsequent API call;
trusted fixture time is clamped monotonically under the enforcing registry lock
for plan/grant expiry and TTL origins; backward input cannot revive expired state. Pipe IO
or a child stall must not hold a global lock or block another session. Revocation,
expiry, close and Drop stop, kill if needed and reap the child, retire its endpoint
generation and preserve terminal evidence. No automatic restart is allowed.
Restart prepares a fresh preview and needs a fresh event/confirmation/instance.

`Lang` for the child is one self-observation operation. `ObsContract` includes its
own bootstrap, executable hash, instance state and observed replies. Retired
handles deny future access; already observed bytes and trusted host logs cannot
be erased. Host-client/deputy and transitive process authority remain broader and
unproved. Finite denials do not establish noninterference or runtime isolation.

## Frozen host fixture API and assertions

`desktop_service` exposes development APIs only with `desktop_v0_dev`. `DevConfig`
binds witness path/hash, preview/instance TTLs, child deadline and witness mode.
`DevDesktop::new` and session registration are fallible. `dispatch` accepts opaque
peer context, generated Envelope and controlled time. `maintenance` drives fake
time lifecycle work; it does not replace the independent real watchdog.
Read-only preview/child evidence accessors expose versioned preview metadata and
actual PID/hash/bootstrap/typed exchanges/reaping. Fixture policy/selected-identity
updates and restricted endpoint issuance are trusted broker instrumentation.
`is_endpoint_active` records registry retirement; `dispatch_recorded_child` is a
trusted fixture probe with the captured opaque identity of an actual prior child,
not a live post-kill request or a caller-declared role. It uses the same enforcing
dispatch path to check denial of a previously granted handle after retirement.
Neither probe is exposed on the child/native interface.

Assertions must first fail against absent handlers, then prove:

- A canceled preview creates no child/grants; fresh event/approval launches one
  pinned child with exactly the previewed observation right and a real exchange.
- Missing event, replayed confirmation, stale/expired plan and changed policy or
  fixture identity launch no child; a fresh authorized plan still works.
- Child attempts to prepare/confirm/restart or observe a foreign instance fail;
  wrong owner, endpoint class, kind, generation and rights fail at the service.
- Invalid protocol/operation/length/reserved/tail/raw-handle bits and outer pad
  cause no state change; authorized witnesses prevent reject-everything success.
- Revocation/expiry/close reap the actual child and deny retired handles. A fault
  has no silent restart; fresh preview/confirmation produces a fresh instance.
- A child that never yields or emits a reply is reaped by the absolute watchdog
  without dispatch/maintenance. Another session stays operable during the stall.
- A replaced/mismatched executable is rejected before spawn. Default builds
  expose no host development runtime or witness binary.

`just foundry-desktop-host-launch-ui1-0` passes 17 exact named cases on macOS and
Linux. It verifies default feature exclusion and retains actual PID, executable
hash, generated bootstrap, bounded canonical wire exchanges and independent
watchdog/Drop disappearance records. Source, test/witness binary and log/artifact
digests bind the report; missing, unexpected or ignored cases cannot pass.
Earlier failed runs are preserved separately. The 250 ms unrelated-session fixture
assertion is an observed check on the tested host, not a scheduler latency guarantee.

The coordinator registered the exact named-case Foundry gate before implementation,
owns codegen/workspace/gate resources, and runs integrated checks after independent
review. Proposed commands in UI0 do not become implemented evidence through this
contract. No hardware, model calls, spending, merge or release authority follows.
