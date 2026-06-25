# POSIX Runner Security Guide

**SECURITY NOTICE:** The POSIX runner executes shell scripts and carries inherent
security risks. Read this document carefully before use.

## Current Status

The default POSIX runner profile is **host-portable-rlimits-only**:

```text
seccomp=false namespaces=false chroot=false rlimits=true
```

The runtime still requires `RAMEN_POSIX_RUNNER_ACK_RISK=1`, logs every launch,
and applies resource limits on Linux. The seccomp, namespace, and chroot helpers
exist in `runtime_supervisor/src/sandbox.rs` and have focused tests, but they
are not wired into the default runner path because they are not portable on
unprivileged CI hosts.

### Default Runtime Controls

- **Kill-switch:** execution is blocked unless
  `RAMEN_POSIX_RUNNER_ACK_RISK=1` is set.
- **Artifact path:** store-integrated calls verify the artifact through
  `store_service` before execution.
- **Resource limits:** Linux `setrlimit()` hooks are configured for open files,
  process count, file size, address space, and CPU time. Limit failures are
  logged but do not fail execution.
- **Not default-wired:** seccomp filtering, namespace isolation, and chroot.

### Helper Controls

The following helpers are implemented and tested in isolation:

- **Seccomp syscall filtering:** allowlist-based BPF filter via `seccompiler`.
- **Namespace isolation:** `libc::unshare()` for mount, UTS, IPC, and network
  namespaces. PID namespaces are not used by the current `Command::spawn()`
  pattern.
- **Chroot filesystem restriction:** `libc::chroot()` followed by `chdir("/")`.

These helpers are not a claim about the default POSIX runner profile until a
future gate wires them through `posix_run_v0_sandboxed`.

**Risk Level:** HIGH. This remains a compatibility-only development scaffold
that can execute arbitrary shell scripts once the explicit risk gate is set.

## What This Means

### Protected Against

- Accidental execution without the explicit risk acknowledgment.
- Some resource exhaustion paths on Linux when resource limits apply.
- Unsigned or invalid artifacts on store-integrated verified paths.

### NOT Protected Against

- Arbitrary shell behavior after `RAMEN_POSIX_RUNNER_ACK_RISK=1`.
- Network access, filesystem traversal, or child process creation in the
  default profile.
- Kernel exploits, compromised parent process, side channels, or hardware
  vulnerabilities.
- Repeated invocation DoS.

See `docs/plans/posix_runner_remaining_risks.md` for complete risk analysis.

## Usage

### Enable the POSIX Runner

The POSIX runner is disabled by default. Enable it with a feature flag:

```bash
cargo build --features posix_runner_v0_dev
```

### Suppress Startup Warning

To acknowledge the security risk and suppress the warning:

```bash
export RAMEN_POSIX_RUNNER_ACK_RISK=1
```

### Create a Launch Plan

```json
{
  "program_id": "dev.example.myscript",
  "runner": "posix_runner_v0",
  "artifact_ref": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
}
```

### Run the Script

```bash
cargo run --bin runtime_supervisor -- \
  --plan launch_plan.json \
  --posix-log-path out/script.log
```

## Security Checklist

Before using the POSIX runner, ensure:

- [ ] You have read and understood `docs/plans/posix_runner_remaining_risks.md`
- [ ] The script artifact is from a trusted source
- [ ] The script has been audited for security issues
- [ ] You are running in a development/test environment (NOT production)
- [ ] You have acknowledged the risk with `RAMEN_POSIX_RUNNER_ACK_RISK=1`
- [ ] You have monitoring in place to detect sandbox violations
- [ ] You have a plan to migrate to the native runner (S10+)

## Migration Path

### Short-Term (Current)
Use the POSIX runner for development only. Treat the default profile as
rlimits-only, not as a seccomp/chroot container.

### Medium-Term (S9.3)
Integrate with store service for artifact validation:
- Verify content hashes before execution
- Check manifest signatures
- Validate artifact permissions

### Long-Term (S10+)
Migrate to native personality runner:
- No shell execution
- Capability-based access control
- Kernel-mediated IO
- Type-safe interfaces

## Foundry Testing

Run the Foundry gate to verify sandboxing features:

```bash
just foundry-s7-posix-runner-security
```

Expected output:
```
=== FOUNDRY_S7_POSIX_RUNNER_SECURITY: PASS ===
```

### Test Coverage

The Foundry gate now includes:
1. **Kill-switch enforcement**: POSIX runner refuses execution without `RAMEN_POSIX_RUNNER_ACK_RISK=1`
2. **Sandbox-disabled warning**: Explicit warning when `RAMEN_POSIX_RUNNER_DISABLE_SANDBOX=1` is set
3. **ACK allows execution**: Execution succeeds with proper acknowledgment
4. **Default profile honesty**: the runtime log and unit contract report
   `seccomp=false namespaces=false chroot=false rlimits=true`
5. **Seccomp helper enforcement**: helper tests verify dangerous syscalls are blocked
6. **Resource limit helper enforcement**: helper tests verify rlimits are applied
7. **Chroot helper confinement**: helper tests verify filesystem access is restricted

### Integration Tests

The following tests verify helper behavior, not the default runner profile:
- `seccomp_filter_blocks_execve_syscall`: Verifies `execve` is blocked
- `seccomp_filter_blocks_socket_syscall`: Verifies `socket` is blocked
- `rlimit_enforces_process_limit`: Verifies `RLIMIT_NPROC` prevents fork
- `chroot_confines_filesystem_access`: Verifies files outside chroot are inaccessible
- `sandbox_full_enforcement`: Verifies all controls work together when explicitly configured
- `sandbox_blocks_dangerous_operations`: Verifies dangerous operations are blocked when explicitly configured

## Platform Support

- **Linux**: default runner profile applies the configured resource-limit path.
  Seccomp, namespace, and chroot helpers are available only when explicitly
  wired by tests or future runner profiles.
- ⚠️ **macOS**: Partial support
  - No seccomp support (not available on macOS)
  - Sandbox returns error on non-Linux platforms
  - Use `RAMEN_POSIX_RUNNER_DISABLE_SANDBOX=1` to run without sandbox (dangerous, dev only)
- ⚠️ **Windows**: Not supported
  - No sandbox support (no namespaces, seccomp, or chroot)
  - Sandbox returns error on non-Linux platforms
  - Use `RAMEN_POSIX_RUNNER_DISABLE_SANDBOX=1` to run without sandbox (dangerous, dev only)

## Performance Impact

Approximate overhead per invocation:
- Sandbox setup: 10-50ms
- Seccomp filtering: 1-5% CPU
- Total: 15-55ms + 1-5% CPU

Acceptable for development, not for high-frequency production use.

## Troubleshooting

### "Sandboxing is only supported on Linux"
You are trying to use the POSIX runner on macOS or Windows. The sandbox requires Linux kernel features (namespaces, seccomp).

### "Failed to apply sandbox"
The sandbox setup failed. Check:
- You are running Linux 3.5+
- You have the required capabilities (CAP_SYS_ADMIN)
- The temporary directory is writable

### "Script execution failed"
The script may have tried to:
- Execute a command (blocked by seccomp)
- Create a child process (blocked by seccomp)
- Access the network (blocked by namespace)
- Write a large file (blocked by rlimit)

Check the script log for details.

## Further Reading

- **Implementation:** `runtime_supervisor/src/sandbox.rs`
- **Remaining Risks:** `docs/plans/posix_runner_remaining_risks.md`
- **Current Status:** `CURRENT_STATUS.md`
- **Remediation Plan:** `docs/plans/security_remediation_v006_v007_v012.md`
- **Constitution:** `CONSTITUTION.md`

## Support

For questions or issues:
1. Check the documentation above
2. Review the security risks in `docs/plans/posix_runner_remaining_risks.md`
3. Run the Foundry gate to verify your setup
4. File an issue if you find a security vulnerability

---

**Remember:** The POSIX runner is a development scaffold. For production, use the native runner (S10+).
