# Security Policy

**Status:** Pre-alpha. RamenOS is **not** production software and makes **no**
security-readiness claim. Do not rely on it to protect untrusted workloads or
hardware.

## Reporting a vulnerability

**Do not open a public issue for a security problem.** Report privately via
GitHub's **Report a vulnerability** flow on the **Security** tab (private
vulnerability reporting / security advisories). If that channel is unavailable
to you, contact the maintainer through the profile linked in the README.

Please include, as far as you can:

- The affected component or gate.
- The evidence level the issue undermines (see [EVIDENCE_LEVELS.md](EVIDENCE_LEVELS.md)).
- A reproducer or gate command.
- The claim boundary it violates (e.g. a gate asserting more than its evidence).

## Supported versions

Only the `main` branch tip is in scope. There are **no tagged releases** and no
stable API/ABI. Pre-alpha means interfaces and behavior may change without notice.

## Current posture

RamenOS is pre-alpha. Foundational controls have landed — fail-closed defaults
across the Store, runner, wire-format, capability, and trace-isolation paths —
but **architectural risk remains** and no formal verification, independent audit,
or stable release threat model exists.

- Current controls and residual risk: [SECURITY_STATUS.md](SECURITY_STATUS.md)
- Active risk register: [RISKS.md](RISKS.md)
- Claim/evidence vocabulary: [EVIDENCE_LEVELS.md](EVIDENCE_LEVELS.md)

Treat any `PASS/QEMU` or replay result as **non-metal** evidence. The default
POSIX runner profile is host-portable rlimits-only; seccomp, namespaces, and
chroot are tested helpers, not current default containment.
