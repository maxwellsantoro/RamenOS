"""Source-only candidate: Root-owned invocation of the pinned pure Rust verifier.

Never import/execute from an artifact-selected path. No Store/process authority
comes from a diagnostic. A successful memo entry belongs to this invocation's
same source/binary/profile and complete immutable input bytes only.
"""
import hashlib
import json
import os
import select
import subprocess
import time


class OwnedManifestVerifier:
    MAX_UNIQUE = 1024  # Source-derived 299 object inventories * <=3 signed CAS bodies.
    INPUT_CAP = 16384
    OUTPUT_CAP = 1024
    STDERR_CAP = 128

    def __init__(self, primitives, binary, binary_info, cwd, env, deadline):
        self.h, self.binary, self.binary_info = primitives, binary, binary_info
        self.cwd, self.env, self.deadline = cwd, dict(env), deadline
        self.observations = []
        self.memo = {}

    def __call__(self, mode, body, manifest):
        h = self.h
        h.require(mode in ('native-task', 'native-read') and type(body) is bytes
                  and len(body) <= 4096 and type(manifest) is bytes
                  and 0 < len(manifest) <= 2048, 'root verifier closed input')
        key = (mode, body, manifest)
        if key in self.memo:
            h.unchanged_binary(self.binary, self.binary_info)
            h.require(time.monotonic() < self.deadline, 'memo original gate deadline')
            return self.memo[key]
        h.require(len(self.observations) < self.MAX_UNIQUE, 'complete verifier unique input bound')
        raw = json.dumps({'body_hex': body.hex(), 'manifest_hex': manifest.hex()},
                         separators=(',', ':')).encode('ascii')
        h.require(0 < len(raw) <= self.INPUT_CAP, 'verifier input pre-growth bound')
        h.unchanged_binary(self.binary, self.binary_info)
        deadline = min(self.deadline, time.monotonic() + 10)
        h.require(deadline - time.monotonic() >= 3, 'verifier work/reap budget')
        argv = [str(self.binary), mode]
        process = None
        held = False
        status = birth = method = None
        stdout, stderr = bytearray(), bytearray()
        offset = 0
        started = time.monotonic_ns()
        complete = False

        def reap():
            nonlocal held, status
            if held:
                observed = process.poll()
                if observed is not None:
                    held = False  # Clear signal authority before diagnostics.
                    status = observed

        try:
            process = subprocess.Popen(argv, cwd=self.cwd, env=self.env,
                                       stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                       stderr=subprocess.PIPE, close_fds=True, bufsize=0)
            held = True
            # The child waits for stdin EOF; no input is sent before native birth.
            birth, method = h.native_birth(process)
            read_streams = {process.stdout.fileno(): (stdout, self.OUTPUT_CAP),
                            process.stderr.fileno(): (stderr, self.STDERR_CAP)}
            for fd in read_streams:
                os.set_blocking(fd, False)
            input_fd = process.stdin.fileno()
            os.set_blocking(input_fd, False)
            while held or read_streams:
                h.require(time.monotonic() < deadline - 2, 'verifier original work deadline')
                reads, writes, _ = select.select(list(read_streams),
                                                 [input_fd] if input_fd is not None else [], [], .02)
                for fd in writes:
                    n = os.write(fd, raw[offset:offset + 4096])
                    h.require(n > 0 and offset + n <= len(raw), 'actual bounded verifier stdin')
                    offset += n
                    if offset == len(raw):
                        process.stdin.close()  # Actual EOF is required by the verifier.
                        input_fd = None
                for fd in reads:
                    output, cap = read_streams[fd]
                    block = os.read(fd, min(4096, cap + 1 - len(output)))
                    if not block:
                        del read_streams[fd]
                        continue
                    h.require(len(output) + len(block) <= cap, 'verifier output pre-growth bound')
                    output.extend(block)
                reap()
            h.require(status is not None and not held and offset == len(raw), 'verifier actual EOF/exit/reap')
            h.require(time.monotonic() < deadline, 'verifier final original deadline')
            h.unchanged_binary(self.binary, self.binary_info)
            h.require(status == 0 and not stderr and 0 < len(stdout) <= self.OUTPUT_CAP
                      and stdout.endswith(b'\n') and stdout.count(b'\n') == 1,
                      'verifier accepted closed single-line protocol only')
            complete = True
        finally:
            try:
                reap()
                if held:
                    process.kill()  # Only this still-held Popen child; no artifact PID.
                    while held and time.monotonic() < deadline:
                        reap()
                        if held:
                            time.sleep(min(.01, max(0, deadline - time.monotonic())))
                    h.require(not held, 'owned verifier reap uncertain')
            finally:
                if process is not None:
                    self.observations.append({
                        'ordinal': len(self.observations), 'argv': argv,
                        'input_byte_len': len(raw), 'input_sha256': hashlib.sha256(raw).hexdigest(),
                        'body_byte_len': len(body), 'body_sha256': hashlib.sha256(body).hexdigest(),
                        'manifest_byte_len': len(manifest), 'manifest_sha256': hashlib.sha256(manifest).hexdigest(),
                        'stdout_hex': bytes(stdout).hex(), 'stderr_hex': bytes(stderr).hex(),
                        'protocol_accepted': complete,
                        'actual_process': {'native_pid': process.pid, 'start_identity': birth,
                            'start_identity_method': method, 'spawn_observed': True,
                            'start_monotonic_ns': started, 'exit_observed_monotonic_ns': time.monotonic_ns(),
                            'exit_code_or_null': status if status is not None and status >= 0 else None,
                            'term_signal_or_null': -status if status is not None and status < 0 else None,
                            'wait_reaped': not held, 'wait_method': 'owned_subprocess_poll_wait', 'argv': argv}})
                    for stream in (process.stdin, process.stdout, process.stderr):
                        if stream is not None:
                            stream.close()
        h.require(time.monotonic() < deadline, 'verifier final publication deadline')
        result = bytes(stdout)
        self.memo[key] = result
        try:
            h.require(time.monotonic() < deadline, 'verifier successful return deadline')
        except BaseException:
            self.memo.pop(key, None)  # Rejected final admission never becomes a reusable result.
            raise
        return result
