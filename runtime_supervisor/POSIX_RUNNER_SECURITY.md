# POSIX Runner Security Guide

**⚠️ SECURITY NOTICE:** The POSIX runner executes shell scripts and carries inherent security risks. Read this document carefully before use.

## Current Status

**S7 Security Hardening COMPLETE:** The POSIX runner now runs inside a sandbox with actual enforcement:

### Seccomp Syscall Filtering (Linux only)
- ✅ **Actual enforcement** using `seccompiler` crate with `prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER)`
- ✅ **Allowlist approach**: Only explicitly whitelisted syscalls are permitted
- ✅ **Blocked syscalls**:
  - Process creation: `execve`, `execveat`, `fork`, `vfork`, `clone`, `clone3`
  - Network: `socket`, `socketpair`, `bind`, `listen`, `accept`, `connect`, `sendto`, `recvfrom`
  - Privilege escalation: `setuid`, `setgid`, `seteuid`, `setegid`, `setresuid`, `setresgid`, `setfsuid`, `setfsgid`
  - File creation: `creat`, `mkdir`, `mknod` (blocked via seccomp + namespace)
  - Advanced IPC: `shmget`, `shmat`, `shmdt`, `semget`, `semop`, `msgget`, `msgrcv`, `msgsnd`
  - Other dangerous: `ptrace`, `kexec_load`, `init_module`, `finit_module`, `userfaultfd`
- ✅ **Allowed syscalls** (whitelist):
  - Basic I/O: `read`, `write`, `pread64`, `pwrite64`, `poll`, `select`, `ppoll`, `pselect6`
  - Memory: `brk`, `mmap`, `mprotect`, `munmap`, `mremap`
  - Process: `exit`, `exit_group`, `rt_sigreturn`, `rt_sigprocmask`
  - Time: `clock_gettime`, `gettimeofday`
  - Info: `uname`, `getuid`, `getgid`, `geteuid`, `getegid`, `getpid`, `getppid`
  - FS (read-only): `fstat`, `lstat`, `stat`, `fstatat`, `access`, `faccessat`, `readlink`
  - IPC: `pipe`, `pipe2` (for shell redirection)
  - File descriptors: `dup`, `dup2`, `close`
  - Misc: `ioctl` (limited for terminal handling), `getrlimit`, `getrusage`

### Namespace Isolation (Linux only)
- ✅ **Actual enforcement** using `libc::unshare()` in child process before exec
- ✅ **Isolated namespaces**:
  - `CLONE_NEWNS`: Mount namespace - isolates filesystem mounts
  - `CLONE_NEWUTS`: UTS namespace - isolates hostname and domain name
  - `CLONE_NEWIPC`: IPC namespace - isolates System V IPC and POSIX message queues
  - `CLONE_NEWNET`: Network namespace - isolates network interfaces
- ⚠️ **Note**: PID namespace (`CLONE_NEWPID`) is not used due to `Command::spawn()` pattern limitations

### Chroot Filesystem Restriction (Linux only)
- ✅ **Actual enforcement** using `libc::chroot()` in child process before exec
- ✅ **Followed by `chdir("/")`** to set working directory inside chroot
- ✅ **Creates minimal filesystem structure** in chroot directory
- ⚠️ **Limitations**:
  - Requires `CAP_SYS_CHROOT` capability or root privileges
  - Does not provide complete isolation - only affects path resolution
  - Process can escape chroot if it has root privileges and can create device nodes
  - Must be combined with namespaces, seccomp, and rlimits for proper isolation

### Resource Limits (Linux only)
- ✅ **Actual enforcement** using `libc::setrlimit()` in child process before exec
- ✅ **Conservative limits**:
  - `RLIMIT_NOFILE`: 64 (max open file descriptors)
  - `RLIMIT_NPROC`: 1 (prevents fork bombs and child processes)
  - `RLIMIT_FSIZE`: 1,048,576 bytes (1MB max file writes)
  - `RLIMIT_AS`: 268,435,456 bytes (256MB max virtual memory)
  - `RLIMIT_CPU`: 30 seconds (max CPU time)
- ⚠️ **Note**: Failures to set limits are logged but don't fail execution (defense-in-depth)

**Risk Level:** MEDIUM (reduced from HIGH with actual enforcement)

## What This Means

### Protected Against
- ✅ Arbitrary command execution
- ✅ Process creation (fork bombs)
- ✅ Network access
- ✅ Filesystem access outside sandbox
- ✅ Resource exhaustion attacks

### NOT Protected Against
- ⚠️ Kernel exploits
- ⚠️ Compromised parent process (runtime_supervisor)
- ⚠️ Side-channel attacks
- ⚠️ Hardware vulnerabilities (Spectre, etc.)
- ⚠️ Repeated invocation DoS

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
Use the sandboxed POSIX runner for development only.

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
4. **Sandbox enabled by default**: Sandbox is enabled unless explicitly disabled
5. **Seccomp filter enforcement**: Integration tests verify dangerous syscalls are blocked
6. **Resource limit enforcement**: Integration tests verify rlimits are applied
7. **Chroot confinement**: Integration tests verify filesystem access is restricted

### Integration Tests

The following integration tests verify actual security enforcement:
- `seccomp_filter_blocks_execve_syscall`: Verifies `execve` is blocked
- `seccomp_filter_blocks_socket_syscall`: Verifies `socket` is blocked
- `rlimit_enforces_process_limit`: Verifies `RLIMIT_NPROC` prevents fork
- `chroot_confines_filesystem_access`: Verifies files outside chroot are inaccessible
- `sandbox_full_enforcement`: Verifies all controls work together
- `sandbox_blocks_dangerous_operations`: Verifies dangerous operations are blocked

## Platform Support

- ✅ **Linux**: Full sandbox support with actual enforcement
  - Seccomp BPF filtering via `prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER)`
  - Namespace isolation via `libc::unshare()`
  - Chroot filesystem restriction via `libc::chroot()`
  - Resource limits via `libc::setrlimit()`
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
