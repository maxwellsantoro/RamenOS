# POSIX Runner Security Guide

**Last Updated:** 2026-10-03
**Status:** Compatibility-only host development scaffold

The POSIX runner executes shell scripts with the invoking user's host access.
It is disabled unless compiled with `posix_runner_v0_dev`, and execution requires
`RAMEN_POSIX_RUNNER_ACK_RISK=1`. The acknowledgment permits execution and does
not establish script safety.

## Default runtime profile

The profile is **host-portable-rlimits-only**:

```text
seccomp=false namespaces=false chroot=false rlimits=true
```

On Linux, the runner attempts limits on open files, process count, file size,
address space, and CPU time. Limit failures are logged and do not abort execution.
The non-Linux path does not apply those Linux sandbox controls. It does not
require `RAMEN_POSIX_RUNNER_DISABLE_SANDBOX=1` merely to run on macOS.
Windows support is not established by these Unix host paths.

Store-integrated execution verifies the artifact before running it. This checks
content identity and the applicable Store policy, while scripts can still read
host files, use the network, spawn processes, or attack the host.

The [sandbox helpers](src/sandbox.rs) implement seccomp filters, mount/UTS/IPC/
network namespaces, and chroot for explicitly configured paths and tests. PID
namespace isolation is not supplied by the current spawn pattern. These helpers
are not wired into the default runner. `RAMEN_POSIX_RUNNER_DISABLE_SANDBOX=1`
explicitly bypasses its configured resource-limit path.

## Running a development plan

Build the named crate with its opt-in feature:

```bash
cargo build -p runtime_supervisor --features posix_runner_v0_dev
```

Use a valid launch plan referencing an artifact ingested into a running Store
service. Match the service socket explicitly; see
[Development Reference](../docs/DEVELOPMENT_REFERENCE.md) for service setup.
From the repository root, with those files already prepared:

```bash
RAMEN_POSIX_RUNNER_ACK_RISK=1 \
  cargo run -p runtime_supervisor --features posix_runner_v0_dev -- \
  --plan out/posix/launch_plan.json \
  --store-socket "$PWD/out/store-demo/store.sock" \
  --posix-log-path out/posix/script.log
```

The plan must select the POSIX runner and an accepted shell artifact; a made-up
hash or abbreviated JSON plan will fail validation. The acknowledgment and
bypass are development settings, not release configuration.

## Foundry scope

```bash
just foundry-s7-posix-runner-security
just foundry-posix-runner-s9-2-store-integration
```

The gates check acknowledgment/default-off behavior, warnings, profile honesty,
artifact validation, and selected helper assertions. Helper enforcement tests
may depend on Linux features or privileges. A helper PASS does not add that
control to the default runtime profile.

For failures, inspect the script and runtime logs. Resource-limit failures are
best effort; filesystem, network, and subprocess failures cannot be assumed to
come from seccomp or namespaces in this default profile. No invocation-overhead
benchmark or production-isolation guarantee is established here.

The [current risk reference](../docs/plans/posix_runner_remaining_risks.md)
records kernel exploits, compromised-parent, side-channel, and repeated-invocation
risks. [Security Status](../SECURITY_STATUS.md) and
[Current Status](../CURRENT_STATUS.md) govern broader claims.
