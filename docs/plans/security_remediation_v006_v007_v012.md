# Security Remediation Reference: V-006, V-007, V-012

**Last Updated:** 2026-10-03
**Status:** Landed foundation milestones; residual architectural risk remains

The original implementation program is preserved in the
[historical plan](../archive/plans/2026-02-09-security-remediation-v006-v007-v012.md).
This stable path remains available to runtime guidance and Foundry references.
Use [Security Status](../../SECURITY_STATUS.md) for the current posture and
[Next Tasks](../../NEXT_TASKS.md) for authorized next work.

| Boundary | Landed control | Current limit |
|----------|----------------|---------------|
| POSIX execution | Default-off feature, risk acknowledgment, artifact verification, declared rlimits-only profile | Seccomp, namespaces, and chroot helpers are not default containment; limit failures are best effort |
| Store access | Typed Store IPC, credential/capability policy, verified IDs and descriptor-scoped ingestion | Host service remains trusted; development modes deliberately relax signature/access policy |
| Tracing | Per-domain buffers/writers and capability-checked trace service/client | Named isolation and ordering assertions do not qualify the entire OS for SMP or untrusted workloads |

Relevant maintained references:

- [POSIX operating constraints](../../runtime_supervisor/POSIX_RUNNER_SECURITY.md)
- [Store service IPC](v007_phase2_store_service_ipc_design.md)
- [Trace artifact contract](../TRACE_ARTIFACT_V0.md)
- [Security risk register](../../RISKS.md)
- [Remediation chronology](../../CHANGELOG.md)

Run the focused Store, POSIX, and trace recipes listed by `just --list` and the
[justfile](../../justfile). Full `just preflight` also covers these foundations,
subject to its Linux evaluator prerequisites. Keep helper tests, runtime controls,
and target enforcement distinct when describing a result.
