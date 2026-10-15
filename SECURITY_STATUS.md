# Security Status

**Last Updated:** 2026-10-15
**Status:** Pre-alpha; foundational remediation landed, architectural risk remains

## Host editor transition reliability

The October 8 pending-reply, abandoned-surface, render-time focus and definitive
pre-admission Save findings, together with the JournalSync staging reproducer,
are repaired and independently reviewed in the merged reliability batch.
`just foundry-editor-reliability-ui1-1` requires thirteen added assertions;
the original eighteen-case task and complete recording reader remain separate.
[Current Status](CURRENT_STATUS.md) records the exact accepted source and evidence.
The trusted, default-off host scope still excludes an actual editor child,
containment, device-backed target persistence and physical durability.

The October 14 foundation review successor rejects overlapping broker grant
generations and retains failed revocations for retry, checks actual physical
backing for Oracle-vector transfers, preserves complete trace events with real
stream metadata, handles empty ring operations, and denies malformed explicit
runner configuration while separating node placement from domain identity.
PR #38 and its post-merge main run qualify those named host/QEMU paths through
five hosted lanes. The strict Linux56 wrapper remains bound to the preceding
source and was not repeated for PR #38. The bootstrap/IPC follow-up merged through PR #39 and passed its matching
post-merge run on `23f37445`. PR #40's comprehensive repairs merged as `b09e919`;
its [post-merge run](https://github.com/maxwellsantoro/RamenOS/actions/runs/38662942304)
passed all nine jobs and 68 verified stage/preparation records on its first
attempt. Reviewed-head push qualification retained two LT deadline failures before
an unchanged-source successful retry. The source-bound local and hosted scope is
in [the repair plan](docs/plans/comprehensive-review-repairs-2026-10-14.md#qualification).
These repairs add no production, target-isolation or release assurance.

## Accepted updated-review foundations

PR #42 implements and qualifies explicit field codecs and checked UTF-8 names,
truthful fixed trace capacity v4, retained Shmem outcomes v3 and per-ID Store
operation drains. Queries reconcile original results without restoring mapping
authority; 64 outcome slots require joint terminal acknowledgement. Actual
FIFO-held host reads exercise unrelated authority progress and admission-to-reply
health checks; publication excludes authority poisoning through durable commit.
Offline revoke-all rotation permits a fresh 1024-ID epoch with exclusive keys,
retirement history of at most 64 keys and an exact external checkpoint pin. The
[operator contract](docs/STORE_AUTHORITY_EPOCH_V1.md) states the provisioning and
rollback-anchor assumptions.

The assembled boundary gate, host kernel/API/Store/codegen checks, strict Clippy,
both-architecture S8 assertions and all affected editor consumers pass locally,
including original task18/Reader93 and reliability13. S11/S12/S13 and governance
pass. PR #42's push, PR and post-merge runs each pass all nine jobs and 68 canonical
records on the first attempt, qualifying the six bounded findings on the reviewed
runtime tree. PR #40/41 evidence qualifies preceding source only. PR #43 carries
test-only timing repairs; PR #44/#45 add bounded timeout diagnostics and test recovery.
Production budgets, containment and public error redaction remain unchanged. The
[acceptance record](docs/plans/updated-review-repairs-2026-10-14.md) records exact
source, retained failures and the remaining boundaries before process work.

## Summary

Safety for humans and AI agents is a product goal in [VISION.md](VISION.md).
Explicit authority, modular boundaries, and recoverable failures are design
requirements; this document describes the current evidence and residual risks
rather than treating the everyday-OS destination as a security assurance.

The tracked S7 and S9 remediation milestones are complete. RamenOS now has
fail-closed authorization and wire checks on named Store, runner, capability,
and trace paths, with deterministic Foundry coverage. POSIX resource limits
remain best effort and are not an isolation boundary.

This is not a production-security claim. The system remains pre-alpha, physical
graduation is incomplete, and several controls are scaffolds or bounded host-side
implementations.

## Landed Controls

| Area | Current control |
|------|-----------------|
| Artifact identity | Requested ID bound to authenticated manifest/blob; consumed WASM snapshot hashed |
| Store access | Credential/capability checks, exact durable live grants and permanent revocation tombstones, signed descriptor ingestion, domain ownership and publication recovery |
| Native execution | Typed manifests/broker grants, absolute host IPC deadlines, no uncertain automatic replay |
| POSIX compatibility | Explicit opt-in plus a host-portable rlimits-only default profile; seccomp/chroot/namespace helpers are tested but not default-wired |
| Kernel request admission | Independent kernel-private caller context, live IPC kind/generation and matching Trace/Shmem namespaces. Oracle providers require explicit features and retain IPC validation. No general target-enforcement claim. |
| Wire formats | Versioned IDL and fail-closed length/encoding checks |
| Shared memory | Versioned offer/recipient acceptance, self-only legacy mapping, domain accounting and atomic collision-safe MMU table updates under exclusive hierarchy access |
| Tracing | Per-domain buffers, scoped writers and caller-owned Shmem delivery with explicit cursor Commit; completed receipt reconciliation |
| Evidence | Redaction/size policy and explicit QEMU/HIL/metal claim levels |

The detailed POSIX operating constraints remain in
[runtime_supervisor/POSIX_RUNNER_SECURITY.md](runtime_supervisor/POSIX_RUNNER_SECURITY.md).

## Residual Risk

- **V-10 supervisor TCB breadth:** policy and compatibility execution still leave
  substantial host-side trusted code. Reduction needs an explicit kernel-policy
  migration plan.
- **V-13 portal TOCTOU:** unforgeable handles reduce risk but do not replace a
  complete transaction and object-lifetime design.
- **Execution identity:** caller context is currently supplied by trusted boot/test
  fixtures. There is no authenticated untrusted syscall/process carrier or general
  protocol-specific IPC authority. Trace client defaults to MockTransport.
- **Page-table concurrency:** mapping requires continuous exclusive hierarchy
  access. This does not provide SMP synchronization or active userspace execution.
- **Host Store authority:** the durable registry controls subsequent service
  operations; GetBlob paths, previously delivered bytes and same-UID filesystem
  access cannot be retracted. Target Domain Manager policy/backend and native
  Store transport remain unfinished. Strict startup requires explicit issuer keys.
  Offline epoch retirement depends on exclusive root/epoch keys and an exact pin
  outside the Store root; it provides no hardware monotonicity. Replacing both
  files and trusted operator configuration defeats that rollback anchor. Stuck
  authority fsync/publication can still block management; per-ID drains do not
  guarantee unrelated data progress through other shared locks.
- **Retained observations and capacity:** fixed trace retention is 64 events,
  independent of delivery limit; overflow requires explicit cursor resync. Shmem
  outcome retention is 64 receipts and joint acknowledgement can impose capacity
  backpressure. Neither query nor a completed receipt restores revoked resources.
- **Static kernel limits:** fixed-size capability, shared-memory, and allocator
  structures can still produce controlled denial of service.
- **Bootstrap memory:** a held firmware allocation bounds the active-firmware
  allocator; the complete RUN0 firmware-exit, retained-range and physical-access
  boundary remains unimplemented. AArch64's fixed pool is a QEMU fixture.
- **Observable authority:** separating `Lang` and `ObsContract` states review
  obligations; it does not prove covert-channel or hidden-affordance noninterference.
- **Hardware trust:** S12/S13 do not yet have full `PASS/METAL` evidence.
- **Compatibility isolation:** the default POSIX runner profile is rlimits-only;
  seccomp, namespaces, and chroot are helper controls, not current default
  containment.
- **Security assurance:** no formal verification, independent audit, or stable
  release threat model has been completed.

See [RISKS.md](RISKS.md) for the active risk register.

## Validation

Relevant gates include:

```bash
just preflight
just s11
just s12
just s13
```

Focused historical gates cover content-ID validation, wire safety, runner
default-off behavior, capability tables, trace ordering/isolation, Store access,
signature policy, and native-runner integration. Gate success proves only the
scope asserted by that gate.

## Historical Record

The detailed remediation sequence is retained in:

- [Security remediation program](docs/archive/plans/2026-02-09-security-remediation-v006-v007-v012.md)
- [Store service IPC design](docs/archive/plans/2026-02-09-store-service-ipc-design.md)
- [S7 implementation record](docs/archive/plans/2026-02-10-s7-security-hardening-phase2.md)
- [S7 gate record](docs/archive/plans/2026-02-18-s7-security-hardening-phase3.md)
- [S9.3 migration record](docs/archive/plans/2026-02-10-s9-3-migration-guide.md)
- [Changelog](CHANGELOG.md)

Those records explain how the current controls arrived; they do not override
this status, [CURRENT_STATUS.md](CURRENT_STATUS.md), or current code and gates.
