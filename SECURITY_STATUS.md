# Security Status

**Last Updated:** 2026-10-08
**Status:** Pre-alpha; foundational remediation landed, architectural risk remains

## Host editor transition reliability

October 8 external review of merged `d3a95551`, followed by independent static
inspection, identifies pending-reply reclamation, abandoned surface acquisitions,
render-time focus authority and definitive pre-admission Save state gaps. The
review reports runtime reproductions; the current optimization packet does not
rerun or repair them. JournalSync staging/reopen additionally needs a focused
reproducer. Existing finite green gates do not cover these transitions.
[Next Tasks](NEXT_TASKS.md#ready-work-front) owns the repair and regression sequence;
[Current Status](CURRENT_STATUS.md) bounds the default-off trusted host scope.

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
| Kernel fast paths | Capability kind, generation, and rights validation |
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
