#!/usr/bin/env python3
"""Actual Store/supervisor cleanup regression with a private host QEMU stand-in.

Build binaries separately into a unique Cargo target, then pass their absolute
paths. This script executes the real S2 gate in a private copied fixture tree;
it does not launch a VM or use the repository's fixed compat output resources.
"""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import select
import signal
import socket
import subprocess
import sys
import tempfile
import time


ROOT = Path(__file__).resolve().parents[2]
MARKERS = ("COMPAT_S2: hello", "COMPAT_S2: read artifact ok", "COMPAT_S2: write blocked ok")


class OwnedProcess:
    """A held Linux pidfd, bound before any actuation; PID is observation only."""

    def __init__(self, pid, executable, token):
        self.pid = pid
        self.fd = os.pidfd_open(pid, 0)
        try:
            command = Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")
            environment = Path(f"/proc/{pid}/environ").read_bytes().split(b"\0")
            if (os.fsencode(executable) not in command
                    or f"S2_FIXTURE_TOKEN={token}".encode() not in environment
                    or not self.running()):
                raise RuntimeError("pidfd identity is not a live private fixture")
        except BaseException:
            os.close(self.fd)
            raise

    def running(self):
        poller = select.poll()
        poller.register(self.fd, select.POLLIN)
        return not poller.poll(0)

    def send(self, sig):
        try:
            signal.pidfd_send_signal(self.fd, sig)
        except ProcessLookupError:
            pass

    def cleanup(self):
        # All signals target the held generation. Never reap and then signal a
        # numeric PID: a completed process can only be reaped after actuation.
        try:
            if self.running():
                self.send(signal.SIGCONT)
                self.send(signal.SIGTERM)
                if not wait_until(lambda: not self.running(), 2):
                    self.send(signal.SIGKILL)
                    if not wait_until(lambda: not self.running(), 2):
                        raise RuntimeError("held fixture cleanup remains UNKNOWN")
            try:
                os.waitpid(self.pid, os.WNOHANG)
            except ChildProcessError:
                pass
        finally:
            os.close(self.fd)


def wait_until(predicate, timeout=5.0):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return True
        time.sleep(0.05)
    return bool(predicate())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("store", "cli", "supervisor"):
        parser.add_argument(f"--{name}-bin", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    binaries = {"store_service": args.store_bin, "store_cli": args.cli_bin,
                "runtime_supervisor": args.supervisor_bin}
    for path in binaries.values():
        if not path.is_absolute() or not path.is_file() or not os.access(path, os.X_OK):
            parser.error(f"not an absolute executable: {path}")
    gate_source = (ROOT / "tools/ci/foundry_compat_s2.sh").read_text()
    if 'wait_for_log "$log" 15' not in gate_source or any(
        f'grep -q "{marker}" "$log"' not in gate_source for marker in MARKERS
    ):
        parser.error("frozen 15-second marker contract changed")
    args.evidence.mkdir(parents=True, exist_ok=False)
    if (not sys.platform.startswith("linux") or not hasattr(os, "pidfd_open")
            or not hasattr(signal, "pidfd_send_signal")):
        (args.evidence / "result.json").write_text(json.dumps({
            "status": "INCOMPLETE", "passed": False, "vm_executed": False,
            "reason": "Linux pidfd ownership primitives required", "cases": []
        }, indent=2) + "\n")
        print("INCOMPLETE: Linux pidfd ownership primitives required")
        return 2
    try:
        probe_fd = os.pidfd_open(os.getpid(), 0)
        try:
            signal.pidfd_send_signal(probe_fd, 0)
        finally:
            os.close(probe_fd)
        libc = ctypes.CDLL(None, use_errno=True)
        if libc.prctl(36, 1, 0, 0, 0) != 0:
            raise OSError(ctypes.get_errno(), "fixture subreaper unavailable")
    except OSError as error:
        (args.evidence / "result.json").write_text(json.dumps({
            "status": "INCOMPLETE", "passed": False, "vm_executed": False,
            "reason": f"Linux ownership primitive unavailable: {error}", "cases": []
        }, indent=2) + "\n")
        print(f"INCOMPLETE: {error}")
        return 2
    report = {"fixture": "ordinary_host_process_not_vm", "vm_executed": False,
              "binary_sha256": {name: hashlib.sha256(path.read_bytes()).hexdigest()
                                for name, path in binaries.items()}, "cases": []}
    for case in ("normal", "timeout", "interruption", "build_failure", "unprovable_shutdown",
                 "missing_first", "missing_second"):
        case_report = run_case(case, binaries, args.evidence)
        report["cases"].append(case_report)
        print(f"{case}: {'PASS' if not case_report['failures'] else 'FAIL'}", flush=True)
    report["passed"] = all(not case["failures"] for case in report["cases"])
    (args.evidence / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return 0 if report["passed"] else 1


def run_case(case, binaries, evidence):
    failures = []
    state = None
    gate = None
    unrelated = None
    unrelated_survived = False
    store_pid = None
    child_remained = False
    owned = {}
    # Keep Unix socket paths short on macOS, regardless of evidence path length.
    with tempfile.TemporaryDirectory(prefix="s2-owned-", dir="/tmp") as directory:
        private = Path(directory)
        (private / "tools/ci").mkdir(parents=True)
        (private / "store").mkdir()
        shutil.copy2(ROOT / "tools/ci/foundry_compat_s2.sh", private / "tools/ci")
        helper = ROOT / "tools/ci/cargo_artifact.py"
        if helper.exists():
            shutil.copy2(helper, private / "tools/ci")
        shutil.copy2(ROOT / "store/catalog.json", private / "store")
        fixture_bin = private / "bin"
        fixture_bin.mkdir()
        token = os.urandom(16).hex()
        state_path = private / "child.json"
        control = private / "control.sock"
        fixtures = {}
        for name in ("kernel", "initrd", "artifact"):
            path = private / (name + ".fixture")
            path.write_bytes(f"synthetic nonbootable {name} {token}\n".encode())
            fixtures[name] = path
        # Cargo is a transport fixture: all runtime commands exec the actual
        # compiled binary. Build replies reference those same pinned binaries;
        # no compilation or simulated Store/supervisor handler occurs here.
        cargo_source = "#!" + sys.executable + "\n" + f"""
import json, os, sys
bins = { {name: str(path) for name, path in binaries.items()}!r}
args = sys.argv[1:]
if args[0] == 'build':
    if os.environ['S2_FIXTURE_CASE'] == 'build_failure':
        print('fixture compiler failure', file=sys.stderr)
        sys.exit(73)
    for name in ('store_service', 'runtime_supervisor'):
        print(json.dumps({{'reason':'compiler-artifact','target':{{'name':name,'kind':['bin']}},'profile':{{'test':False}},'executable':bins[name]}}))
    print(json.dumps({{'reason':'build-finished','success':True}}))
elif args[0] == 'run' and args[1] == '-p' and args[2] in bins:
    name = args[2]
    tail = args[3:]
    if tail and tail[0] == '--': tail = tail[1:]
    os.execv(bins[name], [bins[name]] + tail)
else:
    sys.exit('unexpected fixture Cargo command')
"""
        (fixture_bin / "cargo").write_text(cargo_source)
        qemu_source = "#!" + sys.executable + "\n" + """
import json, os, pathlib, socket, sys, time
args = sys.argv[1:]
serial = args[args.index('-serial') + 1]
assert serial.startswith('file:')
path = pathlib.Path(serial[5:])
path.parent.mkdir(parents=True, exist_ok=True)
server = socket.socket(socket.AF_UNIX)
server.bind(os.environ['S2_FIXTURE_CONTROL'])
server.listen(1)
server.settimeout(60)
state = {'pid':os.getpid(), 'parent_pid':os.getppid(), 'token':os.environ['S2_FIXTURE_TOKEN']}
state_path = pathlib.Path(os.environ['S2_FIXTURE_STATE'])
temporary = state_path.with_suffix('.tmp')
temporary.write_text(json.dumps(state))
os.replace(temporary, state_path)
# Wait for the controller to bind live pidfds before emitting any markers.
# The socket and nonce are private; no controller signals a file-record PID.
try:
    peer, _ = server.accept()
    with peer:
        peer.settimeout(60)
        request = peer.recv(128)
        if request != (state['token'] + ':start').encode():
            sys.exit('unbound start request')
        peer.sendall(json.dumps(state).encode())
        case = os.environ['S2_FIXTURE_CASE']
        markers = ['COMPAT_S2: hello', 'COMPAT_S2: read artifact ok', 'COMPAT_S2: write blocked ok']
        if case == 'missing_first': markers.pop(0)
        elif case == 'missing_second': markers.pop(1)
        elif case != 'normal': markers = ['fixture waiting without success markers']
        path.write_text('\\n'.join(markers) + '\\n')
        # Finite self-exit bounds even a failed controller without unsafe PID
        # recovery. Real supervisor normally kills/reaps this ordinary process.
        time.sleep(60)
finally:
    server.close()
"""
        (fixture_bin / "qemu-system-x86_64").write_text(qemu_source)
        for path in fixture_bin.iterdir():
            path.chmod(0o700)
        env = os.environ.copy()
        env.update(PATH=str(fixture_bin) + os.pathsep + env.get("PATH", ""),
                   S2_COMPAT_KERNEL=str(fixtures["kernel"]),
                   S2_COMPAT_INITRD=str(fixtures["initrd"]),
                   S2_COMPAT_ARTIFACT=str(fixtures["artifact"]),
                   S2_FIXTURE_CASE=case, S2_FIXTURE_TOKEN=token,
                   S2_FIXTURE_STATE=str(state_path), S2_FIXTURE_CONTROL=str(control))
        transcript = evidence / (case + ".gate.log")
        try:
            unrelated = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(120)"],
                                         start_new_session=True)
            with transcript.open("wb") as output:
                gate = subprocess.Popen(["bash", str(private / "tools/ci/foundry_compat_s2.sh")],
                                        cwd=private, env=env, stdout=output, stderr=subprocess.STDOUT,
                                        start_new_session=True)
                if case != "build_failure":
                    if not wait_until(lambda: state_path.exists() or gate.poll() is not None, 15):
                        failures.append("fixture child did not start")
                    if state_path.exists():
                        state = json.loads(state_path.read_text())
                        if state.get("token") != token or type(state.get("pid")) is not int:
                            raise AssertionError("unbound child fixture identity")
                        trace = transcript.read_text()
                        store_match = re.search(r"\+ S2_STORE_PID=(\d+)", trace)
                        if not store_match:
                            raise AssertionError("actual Store PID witness missing")
                        store_pid = int(store_match[1])
                        owned["store"] = OwnedProcess(store_pid, binaries["store_service"], token)
                        owned["supervisor"] = OwnedProcess(
                            state["parent_pid"], binaries["runtime_supervisor"], token)
                        owned["child"] = OwnedProcess(
                            state["pid"], fixture_bin / "qemu-system-x86_64", token)
                        with socket.socket(socket.AF_UNIX) as peer:
                            peer.settimeout(2)
                            peer.connect(str(control))
                            peer.sendall((token + ":start").encode())
                            acknowledgement = json.loads(peer.recv(4096))
                            if acknowledgement != state:
                                raise AssertionError("live nonce handshake mismatch")
                        if case == "interruption":
                            gate.send_signal(signal.SIGTERM)
                        elif case == "unprovable_shutdown":
                            owned["supervisor"].send(signal.SIGSTOP)
                    else:
                        failures.append("gate exited before child witness")
                try:
                    exit_code = gate.wait(timeout=25)
                except subprocess.TimeoutExpired:
                    failures.append("gate exceeded bounded fixture deadline")
                    gate.kill()
                    exit_code = gate.wait(timeout=5)
            text = transcript.read_text()
            # Old gates may launch services even when the build-failure fixture
            # requests an early stop. Adopt every witnessed child before cleanup.
            if state is None and state_path.exists():
                state = json.loads(state_path.read_text())
                if state.get("token") != token or type(state.get("pid")) is not int:
                    raise AssertionError("unbound late child fixture identity")
            unrelated_survived = unrelated.poll() is None
            if not unrelated_survived:
                failures.append("unrelated fixture was terminated")
            if case == "normal" and exit_code != 0:
                failures.append(f"successful marker gate returned {exit_code}")
            if case != "normal" and exit_code == 0:
                failures.append("original failure/interruption was masked")
            if case == "interruption" and exit_code not in (143, -signal.SIGTERM):
                failures.append(f"interruption status not preserved: {exit_code}")
            if state:
                pid = state["pid"]
                child_remained = owned["child"].running()
                if child_remained and case != "unprovable_shutdown":
                    failures.append("owned child remains after gate teardown")
                sup_match = re.search(r"\+ (?:S2_SUPERVISOR_PID|pid)=(\d+)", text)
                if not sup_match or int(sup_match[1]) != state["parent_pid"]:
                    failures.append("launcher PID is not actual owning supervisor")
                sup_log = private / "out/logs/supervisor_compat_s2.log"
                observed = sup_log.read_text() if sup_log.exists() else ""
                if f"compat_run_v0 spawned pid={pid}" not in observed:
                    failures.append("actual supervisor spawn witness missing")
                if case != "unprovable_shutdown" and "compat_run_v0 exited status=" not in observed:
                    failures.append("supervisor child kill/reap witness missing")
                if case != "unprovable_shutdown" and "supervisor_shutdown_status=130" not in text:
                    failures.append("graceful supervisor shutdown status missing")
                if case == "unprovable_shutdown" and (
                    "cleanup UNKNOWN label=supervisor" not in text
                    or "FOUNDRY_COMPAT_S2: ok" in text
                ):
                    failures.append("unprovable cleanup was not reported as UNKNOWN/failure")
                (evidence / (case + ".supervisor.log")).write_text(observed)
                (evidence / (case + ".child.json")).write_text(json.dumps(state) + "\n")
            store_match = re.search(r"\+ S2_STORE_PID=(\d+)", text)
            store_pid = int(store_match[1]) if store_match else None
            if store_match and ("store" not in owned or owned["store"].running()):
                failures.append("owned Store remains after gate teardown")
            if case == "build_failure" and (state_path.exists() or store_match):
                failures.append("build failure launched a service")
        finally:
            if gate is not None and gate.poll() is None:
                gate.kill()
                gate.wait(timeout=5)
            # A held pidfd can safely recover only its bound generation. No
            # nonce-file/transcript numeric fallback is permitted, even on RED.
            cleanup_errors = []
            for name in ("supervisor", "store", "child"):
                if name in owned:
                    try:
                        owned[name].cleanup()
                    except (OSError, RuntimeError) as error:
                        cleanup_errors.append(f"{name}: {error}")
            if unrelated is not None:
                unrelated.terminate()
                unrelated.wait(timeout=5)
            if cleanup_errors:
                raise RuntimeError("controller cleanup UNKNOWN: " + "; ".join(cleanup_errors))
        return {"name": case, "gate_exit": exit_code, "failures": failures,
                "child": state, "child_remained_after_gate": child_remained,
                "expected_cleanup_unknown": case == "unprovable_shutdown",
                "unrelated_survived": unrelated_survived}


if __name__ == "__main__":
    raise SystemExit(main())
