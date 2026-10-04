"""Trusted LS launcher and one task-scoped broker connection per shell helper.

The Linux transaction engine is shared with LT to keep the interface contrast
explicit. No shell-selected file path or caller context reaches the host broker.
"""

import copy
import hashlib
import os
from pathlib import Path
import socket
import struct
import tempfile
import threading
import time
import uuid

from linux_sandbox import Sandbox
from lt_backend import FIELDS, LinuxTask, TaskError, canonical, decode

FRAME = 131072
MAX_CALLS = 2048
MAX_SHELLS = 128


class LinuxShellTask:
    def __init__(self, fixture, root, candidate, worker):
        self.task = None
        self.temporary = None
        self.listener = None
        self.thread = None
        self.guard = threading.RLock()
        self.stopping = threading.Event()
        self.audit = []
        self.runs = []
        self.transport_poison = False
        try:
            self.task = LinuxTask(fixture, root, worker, 7)
            self.candidate = Path(candidate)
            self.candidate.mkdir(mode=0o777, parents=True, exist_ok=True)
            if self.candidate.is_symlink() or not self.candidate.is_dir():
                raise TaskError("unsafe candidate directory")
            os.chmod(self.candidate, 0o777)
            self.temporary = tempfile.TemporaryDirectory(prefix="ramenos-ls-session-")
            private = Path(self.temporary.name)
            self.endpoint = private / "task"
            self.endpoint.mkdir(mode=0o755)
            inputs = private / "inputs"
            inputs.mkdir(mode=0o755)
            for name, data in self.task.inputs.items():
                (inputs / name).write_bytes(data)
                os.chmod(inputs / name, 0o444)
            helper = Path(__file__).with_name("ls_taskctl.py").read_bytes()
            (self.endpoint / "taskctl").write_bytes(helper)
            os.chmod(self.endpoint / "taskctl", 0o555)
            (self.endpoint / "bootstrap.json").write_bytes(
                canonical(self.task.bootstrap) + b"\n"
            )
            os.chmod(self.endpoint / "bootstrap.json", 0o444)
            self.helper_id = "sha256:" + hashlib.sha256(helper).hexdigest()
            self.listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            self.listener.bind(str(self.endpoint / "socket"))
            os.chmod(self.endpoint / "socket", 0o666)
            self.listener.listen(8)
            self.listener.settimeout(0.2)
            # The container sees only fixed task inputs, its candidate directory
            # and the broker endpoint/helper. Host journal/runtime stay private.
            self.sandbox = Sandbox(
                readonly={"/inputs": inputs, "/task": self.endpoint},
                writable={"/candidate": self.candidate},
            )
            self.thread = threading.Thread(
                target=self.serve, name="ls-task-broker", daemon=False
            )
            self.thread.start()
        except BaseException:
            self.close()
            raise

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()

    def close(self):
        self.stopping.set()
        if self.thread is not None:
            # A dispatched commit may finish after its caller disappears. Keep
            # the writer lock until the broker finishes; receipts permit recovery.
            self.thread.join()
            self.thread = None
        if self.listener is not None:
            self.listener.close()
            self.listener = None
        if self.temporary is not None:
            self.temporary.cleanup()
            self.temporary = None
        if self.task is not None:
            self.task.close()

    def record_shell(self, index, evidence):
        with self.guard:
            record = dict(evidence, role="ls_shell")
            self.task.state["runs"][index] = record
            if (
                record.get("removed") is not True
                or record.get("reconciliation_required") is True
            ):
                self.transport_poison = True
                self.task.poisoned = True
            try:
                self.task.save()
            except Exception:
                self.transport_poison = True
                self.task.poisoned = True
                raise
            self.runs.append(record)

    def run(self, command, *, input_bytes=b"", wall_ms=10000):
        if self.stopping.is_set() or self.transport_poison or self.task.poisoned:
            raise TaskError("LS session stopped")
        invocation = uuid.uuid4().hex
        with self.guard:
            if len(self.runs) >= MAX_SHELLS or len(self.task.state["runs"]) >= 128:
                raise TaskError("capacity")
            index = len(self.task.state["runs"])
            # A crash before confirmed removal cannot silently authorize a new
            # session. This is a cleanup checkpoint, not a model operation.
            self.task.state["runs"].append(
                {
                    "role": "ls_shell",
                    "created": False,
                    "removed": False,
                    "invocation_pending": True,
                    "lifecycle_invocation": invocation,
                }
            )
            try:
                self.task.save()
            except Exception:
                self.transport_poison = True
                self.task.poisoned = True
                raise
        try:
            result = self.sandbox.run(
                command,
                input_bytes=input_bytes,
                wall_ms=wall_ms,
                allow_nonzero=True,
                invocation_id=invocation,
            )
        except Exception as error:
            record = dict(getattr(error, "evidence", {}) or {}, role="ls_shell")
            if (
                record.get("removed") is not True
                or getattr(error, "reason", None) == "creation_not_confirmed"
            ):
                record["reconciliation_required"] = True
            self.record_shell(index, record)
            raise
        self.record_shell(index, result.evidence)
        return result

    def read_request(self, connection):
        deadline = time.monotonic() + 1
        data = bytearray()
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TaskError("invalid")
            connection.settimeout(remaining)
            chunk = connection.recv(min(4096, FRAME + 2 - len(data)))
            if not chunk:
                raise TaskError("invalid")
            data.extend(chunk)
            if len(data) > FRAME + 1:
                raise TaskError("invalid")
            if b"\n" in chunk:
                # A connection carries exactly one record, no pipelining.
                if not data.endswith(b"\n") or b"\n" in data[:-1]:
                    raise TaskError("invalid")
                return bytes(data[:-1])

    def serve(self):
        while not self.stopping.is_set():
            try:
                connection, _ = self.listener.accept()
            except socket.timeout:
                continue
            except OSError:
                if not self.stopping.is_set():
                    self.transport_poison = True
                return
            with connection:
                started = time.monotonic_ns()
                request = None
                payload = b""
                delivered = False
                peer_uid = None
                peer_gid = None
                try:
                    peer_pid, peer_uid, peer_gid = struct.unpack(
                        "3i",
                        connection.getsockopt(
                            socket.SOL_SOCKET, socket.SO_PEERCRED, 12
                        ),
                    )
                    if peer_uid != 65534 or peer_gid != 65534:
                        raise TaskError("denied")
                    payload = self.read_request(connection)
                    request = decode(payload)
                    if len(self.audit) >= MAX_CALLS:
                        raise TaskError("capacity")
                    with self.guard:
                        response = self.task.execute(request)
                    encoded = canonical(response)
                    if len(encoded) > FRAME:
                        raise TaskError("capacity")
                except (
                    TaskError,
                    ValueError,
                    TypeError,
                    OSError,
                    RecursionError,
                ) as error:
                    status = (
                        str(error)
                        if isinstance(error, TaskError)
                        and str(error) in ["denied", "capacity"]
                        else "invalid"
                    )
                    response = {
                        "schema_version": 1,
                        "request_id": None,
                        "status": status,
                        "result": None,
                    }
                    encoded = canonical(response)
                # No private exceptions, paths or diagnostic messages go back.
                try:
                    connection.settimeout(2)
                    connection.sendall(encoded + b"\n")
                    delivered = True
                except OSError:
                    pass
                operation = (
                    request.get("call", {}).get("operation")
                    if isinstance(request, dict)
                    and isinstance(request.get("call"), dict)
                    else None
                )
                if not isinstance(operation, str) or operation not in FIELDS:
                    operation = None
                if len(self.audit) < MAX_CALLS:
                    self.audit.append(
                        {
                            "request_sha256": hashlib.sha256(payload).hexdigest(),
                            "operation": operation,
                            "peer_uid": peer_uid,
                            "peer_gid": peer_gid,
                            "request_id": response["request_id"],
                            "status": response["status"],
                            "reply_write_succeeded": delivered,
                            "elapsed_ms": (time.monotonic_ns() - started) // 1_000_000,
                        }
                    )

    def evidence(self):
        # Trusted evaluator only. These records are not mounted or command verbs.
        with self.guard:
            return copy.deepcopy(self.snapshot())

    def snapshot(self):
        return {
            "pins": dict(self.task.pins),
            "worker": self.task.worker_id,
            "helper": self.helper_id,
            "revision": self.task.state["revision"],
            "content_id": self.task.state["content_id"],
            "broker_peer_uid": 65534,
            "broker_peer_gid": 65534,
            "broker_audit": list(self.audit),
            "shell_runs": list(self.runs),
            "journal_shell_history": [
                r for r in self.task.state["runs"] if r.get("role") == "ls_shell"
            ],
            "validator_runs": [
                r for r in self.task.state["runs"] if r.get("role") != "ls_shell"
            ],
            "available_authority": [
                "read/enumerate all task input files",
                "read/write/enumerate candidate directory",
                "execute image helpers and taskctl",
                "own processes/private scratch and metadata",
                "connect to scoped broker; raw bounded packets and own-descendant delegation",
                "bounded stdin/stdout/stderr",
            ],
            "broker_deputy_authority": "trusted host files, journal, pinned worker and Docker engine",
            "full_authority_mapping": False,
            "narrower_authority_claim": False,
        }
