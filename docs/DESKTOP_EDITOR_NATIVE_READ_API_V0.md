# Native editor Read prerequisite v0

**Status:** Reviewed contract frozen for gate-first assertion authoring; handlers and runtime acceptance pending.

The [machine-readable contract](contracts/editor-native-read-v0.json) defines a
separate, default-off host prerequisite for UI1.1c. An editor peer approved through
the existing UI1.1a flow can request protocol-368 `ReadSelected` through a real
Store owner. Reading or copying returned bytes must check the original Desktop
authority, its lifetime, and the actual Store grant. This prerequisite does not
deliver keyboard Save, Store permission preview, reopening, or the full integrated
editor task. Existing volatile editor and host Store evidence remains separate.

## Authority and ownership

`HostDesktop::new_native_read` creates the actual private native authority and its
opaque registry witness. `approved_native_read` checks the existing approved
Artifact peer against the actual Desktop State and registers one bounded binding.
Callers cannot construct a witness, binding, original call, guard, or join proof
from diagnostic records or numeric identifiers.

`StoreFixture::create_native_read` consumes the concrete witness and one or two
approved bindings. The fresh Store profile must match each binding's owner,
object, generation, selected revision, and hash. The constructor checks live
authority before provisioning, releases enforcing locks for IO, and checks all
bindings again before exposing any native grant. It holds one Gate guard for the
final checks and exposure. `ApprovedReadGuard::view_binding` checks another
binding through that already-held guard; it must not acquire the mutex again.

A Desktop Read call issued before Store construction supplies no Store effect.
Registration and execution require the matching actual Core, grant, peer, native
identity, and current Store epoch. There is no readiness marker or separate
activation token. Native grants carry only Read authority. The legacy dispatch,
lease, draft, generic grant issuance, recovery, and reopen paths deny native
owners; nonnative UI1.1b behavior remains available through its existing feature.

The native lifetime gate and producer roster live in Desktop. Store's optional
native feature depends on Desktop's optional native module. Desktop may depend
on the artifact schema, but has no normal dependency on Store or Store IO.
Authority objects do not retain a back-reference to Desktop State.

## Calls, copies, and retirement

`start_native_read` captures the actual entry Instant before validation and derives
one checked deadline 1000 ms later. Only canonical protocol-368 request 1 is
supported. Requests 3, 5, and 7 are Unsupported. `NativeReadEntry::wait` returns
the original opaque call once. Store registration may remap its outer endpoint
handle; it preserves the typed payload and original entry/deadline.

Store execution is single-use. Repeated execution is NotReady and has no new
effect. Closing a query before its original deadline is NotReady; expiry closes
that query without retiring a still-live instance. A copy checks live Desktop
authority, the original query deadline, the actual Store epoch and grant, and the
selected-data descriptor before touching the caller's output. Invalid descriptors
leave output unchanged. A successful copy exposes the canonical 64-byte text
header and bounded ASCII body. Previously copied bytes remain observations.

Desktop lifecycle changes hold State, then Gate, and retire matching authority
before releasing State. Store copies acquire Object, Gate, then Data; an outer
Store Registry lock may precede Object where needed. No Gate holder acquires
Desktop State. No enforcing lock is held across IO, a pause wait, or an actual
thread join. Revocation, instance expiry, service faults, and query deadlines keep
their distinct meanings.

## Bounded producers and failure ownership

The shared roster holds at most 64 actual producer rows, including an IO subset
bounded by two. This Read prerequisite starts no IO producer. Desktop entry and
Store execution reserve rows before spawning and install the actual JoinHandles
before subsequent fallible work. Timeout does not free an unjoined row.

Opaque producer IDs remain accessible after wait or Timeout. A finished join
takes the actual handle under the roster lock, releases the lock, performs the
join, and retires that exact row once. An unfinished, concurrent, or repeated
join is NotReady. Numeric diagnostic IDs cannot manufacture join proofs. Drop
does not force a join of a hung thread.

The fixture permits two selected objects, sixteen historical bindings, and 64
origins. Identity counters use checked arithmetic, never reuse identities, and
remain exhausted after arithmetic overflow. The planned counter assertion tests
the exact Binding, Origin, and Producer transitions and a reachable set of 32
paused entries holding 64 actual handles. It does not claim saturation of every
other quota.

Construction starts no producers. A setup error or caught unwind retains the
consumed native witness and any actual partial Core in an opaque failure owner.
The constructor does not delete or reopen a partial root. Root-path observations
are diagnostics, not cleanup authority. Abort and out-of-memory termination are
outside the unwind guarantee.

## Gate-first acceptance

The frozen inventory has nine cases:

- `native_read_positive`
- `native_read_identity_denials`
- `native_read_control_denials`
- `native_read_constructor_recheck`
- `native_read_legacy_descriptor_denials`
- `native_read_once_deadline`
- `native_read_retirement`
- `native_read_owned_handles`
- `native_read_counters_domains`

Both packages use the default-off `editor_native_read_v0_dev` feature. The initial
assertions must fail against missing handlers, be independently reviewed, and
retain their actual RED result before implementation. The focused consumer is:

```sh
cargo test -p store_service --no-default-features \
  --features editor_native_read_v0_dev --test editor_native_read -- --test-threads=1
```

The coordinator owns features, exports, the Foundry gate, and integrated checks.
Acceptance requires the nine actual assertions, strict linting, default API
exclusion, and affected UI1.1a/UI1.1b consumer gates. Host evidence establishes no
editor process identity, device execution, target runtime, persistence across
power loss, or process containment.
