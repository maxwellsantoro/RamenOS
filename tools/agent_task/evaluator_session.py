#!/usr/bin/env python3
"""External bounded scripted transport. No model, retries, repair helper or oracle."""

from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import selectors
import signal
import subprocess
import threading
import time
import tempfile
import uuid
from lifecycle_ledger import Ledger, LedgerError, encoded as ledger_encoded


class SessionStopped(RuntimeError):
    def __init__(self, reason):
        super().__init__(reason)
        self.evidence = None
        self.context = None


@dataclass(frozen=True)
class Limits:
    wall_ms: int = 30000
    reply_ms: int = 10000
    request_bytes: int = 131072
    response_bytes: int = 131072
    context_bytes: int = 1048576
    max_requests: int = 64
    stderr_bytes: int = 4096
    cleanup_ms: int = 10000

    def __post_init__(self):
        for field, value in vars(self).items():
            if type(value) is not int or value <= 0:
                raise ValueError("invalid evaluator limit: " + field)
        if not (
            self.wall_ms <= 120000
            and self.reply_ms <= self.wall_ms
            and self.request_bytes <= 131072
            and self.response_bytes <= 131072
            and self.context_bytes <= 4194304
            and self.max_requests <= 128
            and self.stderr_bytes <= 16384
            and self.cleanup_ms <= 30000
        ):
            raise ValueError("evaluator limit exceeds development ceiling")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def strict_reply(data):
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError("duplicate field")
            result[key] = value
        return result

    def constant(_):
        raise ValueError("nonfinite JSON")

    return json.loads(
        data.decode("utf-8"), object_pairs_hook=pairs, parse_constant=constant
    )


def proc_table():
    result = {}
    for entry in Path("/proc").glob("[0-9]*"):
        try:
            fields = (entry / "stat").read_text().rsplit(")", 1)[1].split()
            result[int(entry.name)] = (
                int(fields[1]),
                int(fields[2]),
                fields[0],
                fields[19],
            )
        except (OSError, ValueError, IndexError):
            pass
    return result


class Session:
    def __init__(
        self,
        command,
        limits=Limits(),
        *,
        visible,
        docker_scope=False,
        lifecycle_parent=None,
    ):
        self.limits = limits
        self.started = time.monotonic()
        self.deadline = self.started + limits.wall_ms / 1000
        self.requests = 0
        self.finished = False
        self.reason = None
        self.visible = []
        self.ledger = []
        self.context_bytes = 0
        self.rejected = None
        self.stderr = bytearray()
        self.private_replies = []
        self.partial_reply = None
        self.owned = {}
        self.cleanup = {}
        self.closed = False
        self.ended = None
        self.lock = threading.RLock()
        self.done = threading.Event()
        self.scope = uuid.uuid4().hex if docker_scope else None
        self.lifecycle = None
        self.process = None
        self.watchdog = None
        self.cleanup_deadline = None
        try:
            for role, raw in visible:
                self.offer(role, raw)
            env = {
                "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
                "HOME": os.environ.get("HOME", "/tmp"),
                "LANG": "C.UTF-8",
            }
            if self.scope:
                env["RAMEN_TASK_EVALUATOR_SCOPE"] = self.scope
                parent = (
                    Path(lifecycle_parent)
                    if lifecycle_parent is not None
                    else Path(tempfile.mkdtemp(prefix="ramenos-lifecycle-"))
                )
                parent.mkdir(mode=0o700, parents=True, exist_ok=True)
                self.lifecycle = Ledger.create(parent / self.scope, self.scope)
                env["RAMEN_TASK_EVALUATOR_LEDGER"] = str(self.lifecycle.root.resolve())
            if "RAMEN_TASK_LINUX_IMAGE" in os.environ:
                env["RAMEN_TASK_LINUX_IMAGE"] = os.environ["RAMEN_TASK_LINUX_IMAGE"]
            self.process = subprocess.Popen(
                command,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                bufsize=0,
                start_new_session=True,
                close_fds=True,
                env=env,
            )
            for stream in (
                self.process.stdin,
                self.process.stdout,
                self.process.stderr,
            ):
                os.set_blocking(stream.fileno(), False)
            self.capture_processes()
            self.watchdog = threading.Thread(
                target=self.watch, name="agent-task-session-deadline", daemon=False
            )
            self.watchdog.start()
            self.bootstrap = self.receive("bootstrap")
        except BaseException as error:
            self.close()
            if isinstance(error, SessionStopped):
                error.evidence = self.evidence()
                error.context = self.context()
            raise

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()

    def context(self):
        return b"".join(self.visible)

    def offer(self, role, raw):
        if not isinstance(raw, bytes):
            raise ValueError("visible bytes required")
        raw.decode("utf-8", errors="strict")
        wire = role.encode() + b"\n" + raw + b"\n"
        if self.context_bytes + len(wire) > self.limits.context_bytes:
            self.rejected = dict(role=role, bytes=len(wire), sha256=digest(wire))
            self.stop("context_limit")
        self.visible.append(wire)
        self.ledger.append(dict(role=role, bytes=len(wire), sha256=digest(wire)))
        self.context_bytes += len(wire)

    def capture_processes(self):
        with self.lock:
            self._capture_processes()

    def _capture_processes(self):
        if self.process is None:
            return
        table = proc_table()
        known = {
            pid
            for pid, previous in self.owned.items()
            if pid in table and table[pid][3] == previous[3]
        }
        if self.process.poll() is None:
            known.add(self.process.pid)
        change = True
        while change:
            change = False
            for pid, item in table.items():
                if pid not in known and item[0] in known:
                    known.add(pid)
                    change = True
        for pid in known:
            if pid in table:
                self.owned[pid] = table[pid]

    def terminate(self):
        with self.lock:
            if self.lifecycle is not None:
                try:
                    self.lifecycle.fence()
                except (LedgerError, OSError):
                    self.lifecycle_failure = "fence_failed"
            if self.process is None:
                return
            if self.cleanup_deadline is None:
                self.cleanup_deadline = time.monotonic() + self.limits.cleanup_ms / 1000
            self.capture_processes()
            current = proc_table()
            groups = set()
            if self.process.poll() is None:
                try:
                    pgid = os.getpgid(self.process.pid)
                    if pgid == self.process.pid:
                        groups.add(pgid)
                except ProcessLookupError:
                    pass
            for pid, previous in self.owned.items():
                if pid in current and current[pid][3] == previous[3]:
                    groups.add(current[pid][1])
            groups.discard(os.getpgrp())
            for pgid in groups:
                try:
                    os.killpg(pgid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                except PermissionError:
                    self.reason = "process_cleanup_failed"
            try:
                self.process.wait(
                    timeout=min(2, max(0.001, self.cleanup_deadline - time.monotonic()))
                )
            except subprocess.TimeoutExpired:
                self.reason = "process_cleanup_failed"
            current = proc_table()
            running = [
                pid
                for pid, previous in self.owned.items()
                if pid in current
                and current[pid][3] == previous[3]
                and current[pid][2] != "Z"
            ]
            self.cleanup = dict(
                leader_reaped=self.process.poll() is not None,
                observed_descendants_not_running=not running,
                observed_processes=len(self.owned),
                running=running,
                scope="owned observed host processes; unobserved escape not certified",
            )
            if running:
                self.reason = "process_cleanup_failed"

    def watch(self):
        if not self.done.wait(max(0, self.deadline - time.monotonic())):
            with self.lock:
                self.reason = self.reason or "session_timeout"
            self.terminate()

    def stop(self, reason):
        with self.lock:
            self.reason = self.reason or reason
        self.terminate()
        raise SessionStopped(self.reason)

    def remaining(self, deadline):
        if self.reason:
            raise SessionStopped(self.reason)
        now = time.monotonic()
        if now >= self.deadline:
            self.stop("session_timeout")
        if now >= deadline:
            self.stop("reply_timeout")
        return min(self.deadline, deadline) - now

    def drain_stderr(self):
        try:
            chunk = os.read(self.process.stderr.fileno(), 4096)
        except BlockingIOError:
            return None
        if not chunk:
            return False
        self.stderr.extend(chunk)
        if len(self.stderr) > self.limits.stderr_bytes:
            self.stop("stderr_limit")
        return True

    def receive(self, role, *, request=None):
        deadline = min(self.deadline, time.monotonic() + self.limits.reply_ms / 1000)
        data = bytearray()
        offset = 0
        with selectors.DefaultSelector() as selected:
            selected.register(self.process.stdout, selectors.EVENT_READ, "out")
            selected.register(self.process.stderr, selectors.EVENT_READ, "err")
            if request is not None:
                selected.register(self.process.stdin, selectors.EVENT_WRITE, "in")
            while True:
                self.capture_processes()
                remaining = self.remaining(deadline)
                for key, _ in selected.select(min(0.02, remaining)):
                    if key.data == "err":
                        if not self.drain_stderr():
                            selected.unregister(key.fileobj)
                    elif key.data == "in":
                        try:
                            offset += os.write(key.fileobj.fileno(), request[offset:])
                        except BlockingIOError:
                            continue
                        except BrokenPipeError:
                            self.stop("request_eof")
                        if offset == len(request):
                            selected.unregister(key.fileobj)
                    else:
                        try:
                            chunk = os.read(
                                key.fileobj.fileno(),
                                min(4096, self.limits.response_bytes + 2 - len(data)),
                            )
                        except BlockingIOError:
                            continue
                        if not chunk:
                            self.stop("response_eof")
                        data.extend(chunk)
                        self.partial_reply = dict(
                            role=role, bytes=len(data), sha256=digest(data)
                        )
                        if len(data) > self.limits.response_bytes + 1:
                            self.stop("response_limit")
                        if b"\n" in data:
                            if (
                                not data.endswith(b"\n")
                                or b"\n" in data[:-1]
                                or request is not None
                                and offset != len(request)
                            ):
                                self.stop("unsolicited_output")
                            self.private_replies.append(
                                dict(role=role, bytes=len(data), sha256=digest(data))
                            )
                            try:
                                reply = strict_reply(data)
                                if not isinstance(reply, dict):
                                    raise ValueError("object required")
                            except (ValueError, UnicodeDecodeError, RecursionError):
                                self.stop("response_invalid")
                            self.offer(role, bytes(data))
                            self.partial_reply = None
                            return reply

    def call(self, request):
        self.remaining(self.deadline)
        if self.closed:
            raise SessionStopped("session_closed")
        if self.finished:
            self.stop("request_after_final")
        if self.requests >= self.limits.max_requests:
            self.stop("request_limit")
        # A previous reply cannot be queued and mistaken for the next response.
        try:
            extra = os.read(self.process.stdout.fileno(), 4096)
        except BlockingIOError:
            extra = b""
        if extra:
            self.partial_reply = dict(
                role="unsolicited", bytes=len(extra), sha256=digest(extra)
            )
            self.stop("unsolicited_output")
        payload = (
            json.dumps(
                request,
                ensure_ascii=False,
                sort_keys=True,
                separators=(",", ":"),
                allow_nan=False,
            ).encode()
            + b"\n"
        )
        if len(payload) > self.limits.request_bytes + 1:
            self.stop("request_limit_bytes")
        self.offer("request", payload)
        self.requests += 1
        return self.receive("response", request=payload)

    def finish(self, text):
        self.remaining(self.deadline)
        if self.closed or self.finished or not isinstance(text, str):
            self.stop("final_invalid")
        data = text.encode()
        if not data or len(data) > self.limits.response_bytes:
            self.stop("final_limit")
        self.offer("final", data)
        self.finished = True

    def reconcile_containers(self):
        if not self.scope:
            return dict(required=False)
        if self.lifecycle is None:
            return dict(
                required=True,
                scope=self.scope,
                certified=False,
                observed_empty=False,
                failure="lifecycle_unavailable",
            )
        deadline = self.cleanup_deadline or (
            time.monotonic() + self.limits.cleanup_ms / 1000
        )

        def engine(args):
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("cleanup deadline")
            result = subprocess.run(
                ["docker", *args], capture_output=True, timeout=remaining
            )
            if result.returncode or len(result.stdout) + len(result.stderr) > 262144:
                raise RuntimeError("cleanup engine failure")
            return result.stdout

        try:
            selector = "label=org.ramenos.evaluator-session=" + self.scope
            ids = (
                engine(["ps", "-aq", "--no-trunc", "--filter", selector])
                .decode()
                .split()
            )
            state = self.lifecycle.snapshot()
            if state["state"] != "fenced":
                raise LedgerError("scope not fenced")
            known = {
                r["container_id"]
                for r in state["entries"].values()
                if r["container_id"] is not None
            }
            untracked = set(ids) - known
            if ids:
                engine(["rm", "--force", *ids])
            empty = not engine(["ps", "-aq", "--filter", selector]).strip()
            for identity, receipt in state["entries"].items():
                if receipt["phase"] == "acknowledged":
                    # Inspect the immutable acknowledgement ID as well as the
                    # label inventory; a changed label cannot hide a live object.
                    remaining = deadline - time.monotonic()
                    if remaining <= 0:
                        raise TimeoutError("cleanup deadline")
                    result = subprocess.run(
                        [
                            "docker",
                            "inspect",
                            "--type",
                            "container",
                            receipt["container_id"],
                        ],
                        capture_output=True,
                        timeout=remaining,
                    )
                    if result.returncode != 1 or b"No such" not in result.stderr:
                        raise LedgerError("acknowledged object still exists")
                    self.lifecycle.removed(identity)
            pending = [
                i
                for i, r in self.lifecycle.snapshot()["entries"].items()
                if r["phase"] == "intent"
            ]
            certified = (
                empty
                and not pending
                and not untracked
                and not getattr(self, "lifecycle_failure", None)
            )
            proof = (
                self.lifecycle.seal_reconciliation(observed_empty=empty)
                if certified
                else None
            )
            return dict(
                required=True,
                scope=self.scope,
                removed_count=len(ids),
                observed_empty=empty,
                certified=certified,
                possible_inflight_create=bool(pending),
                pending_intents=pending,
                untracked_containers=sorted(untracked),
                reconciliation=proof,
            )
        except (
            OSError,
            RuntimeError,
            subprocess.TimeoutExpired,
            TimeoutError,
            LedgerError,
        ):
            return dict(
                required=True,
                scope=self.scope,
                certified=False,
                observed_empty=False,
                failure="cleanup_uncertain",
            )

    def close(self):
        if self.closed:
            return
        self.done.set()
        if self.cleanup_deadline is None:
            self.cleanup_deadline = time.monotonic() + self.limits.cleanup_ms / 1000
        forced = self.reason is not None
        if self.process is not None:
            if not forced:
                self.capture_processes()
                self.process.stdin.close()
                try:
                    self.process.wait(
                        timeout=min(2, max(0.001, self.deadline - time.monotonic()))
                    )
                    if self.process.returncode != 0:
                        self.reason = "backend_exit"
                except subprocess.TimeoutExpired:
                    self.reason = "cleanup_forced"
                    forced = True
            self.terminate()
            if not self.reason:
                try:
                    extra = os.read(
                        self.process.stdout.fileno(), self.limits.response_bytes + 2
                    )
                    if extra:
                        self.partial_reply = dict(
                            role="unsolicited", bytes=len(extra), sha256=digest(extra)
                        )
                        self.reason = "unsolicited_output"
                    while self.drain_stderr() is True:
                        pass
                    if self.stderr:
                        self.reason = "backend_stderr"
                except BlockingIOError:
                    pass
                except SessionStopped:
                    pass
            for stream in (
                self.process.stdin,
                self.process.stdout,
                self.process.stderr,
            ):
                if not stream.closed:
                    stream.close()
        self.container_cleanup = self.reconcile_containers()
        if self.container_cleanup.get("required") and not self.container_cleanup.get(
            "certified"
        ):
            self.reason = self.reason or "cleanup_uncertain"
        self.closed = True
        self.ended = time.monotonic()
        self.done.set()
        if (
            self.watchdog is not None
            and self.watchdog is not threading.current_thread()
        ):
            self.watchdog.join(
                timeout=min(3, max(0.001, self.cleanup_deadline - time.monotonic()))
            )

    def evidence(self):
        try:
            lifecycle = (
                self.lifecycle.snapshot() if self.lifecycle is not None else None
            )
        except (LedgerError, OSError):
            lifecycle = None
        return dict(
            schema_version=1,
            outcome=self.reason or ("completed" if self.closed else "running"),
            limits=vars(self.limits),
            requests=self.requests,
            context_bytes=self.context_bytes,
            context_sha256=digest(self.context()),
            visible_ledger=list(self.ledger),
            rejected_visible=self.rejected,
            received_private=list(self.private_replies),
            partial_reply=self.partial_reply,
            stderr_bytes=len(self.stderr),
            stderr_sha256=digest(self.stderr),
            elapsed_ms=round(((self.ended or time.monotonic()) - self.started) * 1000),
            cleanup_elapsed_ms=round(
                (self.ended - (self.cleanup_deadline - self.limits.cleanup_ms / 1000))
                * 1000
            )
            if self.ended and self.cleanup_deadline
            else None,
            process_cleanup=dict(self.cleanup),
            container_cleanup=getattr(self, "container_cleanup", None),
            lifecycle=lifecycle,
            lifecycle_sha256=digest(ledger_encoded(lifecycle))
            if lifecycle is not None
            else None,
            model_trial=False,
            token_accounting=False,
        )
