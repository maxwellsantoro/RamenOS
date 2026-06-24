# Security Status

**Last Updated:** 2026-06-24
**Status:** Pre-alpha; foundational remediation landed, architectural risk remains

## Summary

The tracked S7 and S9 remediation milestones are complete. RamenOS now has
fail-closed defaults across its early Store, runner, wire-format, capability,
and trace-isolation paths, with deterministic Foundry coverage.

This is not a production-security claim. The system remains pre-alpha, physical
graduation is incomplete, and several controls are scaffolds or bounded host-side
implementations.

## Landed Controls

| Area | Current control |
|------|-----------------|
| Artifact identity | Strict content IDs and signature-aware Store paths |
| Store access | Credential/capability checks and domain-scoped visibility |
| Native execution | Typed manifests and capability-broker grants |
| POSIX compatibility | Explicit opt-in plus Linux sandbox controls where supported |
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
- **Compatibility isolation:** seccomp, namespaces, and chroot are platform-
  dependent layers, not proof of containment against kernel compromise.
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

- [Security remediation program](docs/plans/security_remediation_v006_v007_v012.md)
- [Store service IPC design](docs/plans/v007_phase2_store_service_ipc_design.md)
- [S7 implementation record](docs/archive/plans/2026-02-10-s7-security-hardening-phase2.md)
- [S7 gate record](docs/archive/plans/2026-02-18-s7-security-hardening-phase3.md)
- [S9.3 migration record](docs/archive/plans/2026-02-10-s9-3-migration-guide.md)
- [Changelog](CHANGELOG.md)

Those records explain how the current controls arrived; they do not override
this status, [CURRENT_STATUS.md](CURRENT_STATUS.md), or current code and gates.
