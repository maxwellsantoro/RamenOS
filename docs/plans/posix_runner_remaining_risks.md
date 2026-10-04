# POSIX Runner Remaining Risks

**Last Updated:** 2026-10-03
**Status:** Current operating reference; compatibility-only development scaffold

The default profile in [posix_runner.rs](../../runtime_supervisor/src/posix_runner.rs)
is `host-portable-rlimits-only`: `seccomp=false namespaces=false chroot=false
rlimits=true`. Linux attempts the configured resource limits; failures are logged
and execution continues. Non-Linux execution does not apply that Linux sandbox
path. Neither path supplies default filesystem or network containment.

Execution requires the off-by-default `posix_runner_v0_dev` feature and
`RAMEN_POSIX_RUNNER_ACK_RISK=1`. Store-integrated calls authenticate the artifact
before running it as a shell script. Artifact verification identifies accepted
content; it does not make the script's behavior safe.

## Current risk boundary

- Shell code can access the invoking user's filesystem, network, and processes.
- Resource limits reduce selected exhaustion paths when successfully applied;
  they do not bound repeated invocations or all effects of descendants.
- A script may attempt **kernel exploits** or exploit host software.
- A **compromised parent** can undermine launch configuration and checks.
- Resource timing and other **side channel** observations remain possible.
- `RAMEN_POSIX_RUNNER_DISABLE_SANDBOX=1` explicitly bypasses even the configured
  resource-limit path.

Seccomp, namespace, and chroot helpers have focused tests in
[sandbox.rs](../../runtime_supervisor/src/sandbox.rs). Those tests do not establish
containment for the default runner. General guest isolation, hardware security,
and production execution are outside these gates' claims.

## Validation and operation

Follow the [POSIX Runner Security Guide](../../runtime_supervisor/POSIX_RUNNER_SECURITY.md)
for feature flags, valid launch plans, and platform limits. Relevant gates are
`just foundry-s7-posix-runner-security` and
`just foundry-posix-runner-s9-2-store-integration`. A PASS covers their named
assertions, including the default-profile declaration and artifact checks.

The [original risk analysis](../archive/plans/2026-02-09-posix-runner-risk-analysis.md)
is preserved for history. Its stronger isolation descriptions and timing estimates
are superseded by this reference and [Security Status](../../SECURITY_STATUS.md).
