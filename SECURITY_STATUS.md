# Security Status

**Last Updated:** 2026-10-14
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
source and was not repeated for PR #38. The bootstrap/IPC follow-up has its own
source-bound local validation; hosted evidence must name its matching revision.
These repairs add no
production, target-isolation or release assurance.

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
| Store access | Credential/capability checks, domain ownership, descriptor-scoped host ingestion, durable publication recovery |
| Native execution | Typed manifests/broker grants, absolute host IPC deadlines, no uncertain automatic replay |
| POSIX compatibility | Explicit opt-in plus a host-portable rlimits-only default profile; seccomp/chroot/namespace helpers are tested but not default-wired |
| Kernel request admission | Live IPC kind/generation and creator-domain checks; Trace bearer operation rights. Oracle providers require explicit features and retain IPC validation. No general target-enforcement claim. |
| Wire formats | Versioned IDL and fail-closed length/encoding checks |
| Shared memory | Typed control plane, kernel validation, and domain accounting |
| Tracing | Per-domain buffers, scoped writers, and capability-checked reads |
| Evidence | Redaction/size policy and explicit QEMU/HIL/metal claim levels |

The detailed POSIX operating constraints remain in
[runtime_supervisor/POSIX_RUNNER_SECURITY.md](runtime_supervisor/POSIX_RUNNER_SECURITY.md).

## Residual Risk

- **V-10 supervisor TCB breadth:** policy and compatibility execution still leave
  substantial host-side trusted code. Reduction needs an explicit kernel-policy
  migration plan.
- **V-13 portal TOCTOU:** unforgeable handles reduce risk but do not replace a
  complete transaction and object-lifetime design.
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
