#!/usr/bin/env python3
"""Opt-in LT trusted Linux broker. No RT service or A0 enforcement is imported.

The private launcher supplies filesystem/context/worker selection. The model
receives only bounded point-operation JSON. Linux protects the broker journal
from the nonroot validator; the broker authorizes transactions and observations.
"""

import argparse
import base64
import copy
import fcntl
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import secrets
import stat
import sys
import tempfile
import time
from linux_sandbox import Sandbox, SandboxFailure, bounded_regular_read

LIMIT = 65536
MAX_FRAME = 131072
RIGHTS = {"read": 1, "stage": 2, "validate": 4, "commit": 8, "observe": 16}
FIELDS = {
    "read_input": {"task_cap", "resource"},
    "request_grant": {"policy_cap", "task_id", "resource", "rights", "lifetime_ms"},
    "stage_candidate": {"task_cap", "bytes_base64"},
    "validate_candidate": {"task_cap", "candidate_cap", "validator_id"},
    "commit_candidate": {
        "task_cap",
        "candidate_cap",
        "expected_revision",
        "expected_content_id",
    },
    "get_receipt": {"task_cap", "commit_request_id"},
    "get_task_state": {"task_cap"},
    "revoke_grant": {"policy_cap", "task_cap"},
    "subscribe_task": {"task_cap", "event_types"},
    "poll_task": {"task_cap", "subscription_cap"},
    "unsubscribe_task": {"task_cap", "subscription_cap"},
}
BUDGET = {
    "guest_ms": 1500,
    "wall_ms": 2500,
    "host_call_ms": 1000,
    "max_diagnostics_bytes": 4096,
}


class TaskError(Exception):
    pass


def digest(data):
    return "sha256:" + hashlib.sha256(data).hexdigest()


def encoded(data):
    return base64.b64encode(data).decode()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def decimal(value, positive=False):
    if not isinstance(value, str) or not re.fullmatch(r"0|[1-9][0-9]{0,19}", value):
        raise TaskError("invalid")
    number = int(value)
    if number > 2**64 - 1 or (positive and number == 0):
        raise TaskError("invalid")
    return number


def token():
    while True:
        value = secrets.token_hex(8)
        if value != "0000000000000000":
            return "cap:" + value


def no_duplicates(pairs):
    result = {}
    for k, v in pairs:
        if k in result:
            raise TaskError("invalid")
        result[k] = v
    return result


def decode(data):
    return json.loads(
        data,
        object_pairs_hook=no_duplicates,
        parse_constant=lambda _: (_ for _ in ()).throw(TaskError("invalid")),
    )


def hash_regular(path, destination=None):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    digestor = hashlib.sha256()
    count = 0
    with os.fdopen(fd, "rb") as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise TaskError("io")
        while True:
            chunk = stream.read(1024 * 1024)
            if not chunk:
                break
            count += len(chunk)
            if count > 512 * 1024 * 1024:
                raise TaskError("io")
            digestor.update(chunk)
            if destination is not None:
                destination.write(chunk)
    if count == 0:
        raise TaskError("io")
    return "sha256:" + digestor.hexdigest()


def atomic(path, data):
    temp = path.with_name(".pending-" + secrets.token_hex(12))
    try:
        fd = os.open(temp, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC, 0o600)
        with os.fdopen(fd, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temp, path)
        fd = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)
    finally:
        temp.unlink(missing_ok=True)


class LinuxTask:
    def __init__(self, fixture, root, worker, domain):
        if platform.system() != "Linux":
            raise TaskError("Linux required")
        self.poisoned = False
        self.lock = None
        self.root = Path(root)
        self.worker = Path(worker).resolve()
        self.domain = domain
        self.grants = {}
        self.subscriptions = {}
        self.policy_cap = token()
        self.started = time.monotonic_ns()
        try:
            self.root.mkdir(mode=0o700, parents=True, exist_ok=True)
            if self.root.is_symlink() or not self.root.is_dir():
                raise TaskError("unsafe root")
            os.chmod(self.root, 0o700)
            fd = os.open(
                self.root / "writer.lock",
                os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC,
                0o600,
            )
            self.lock = os.fdopen(fd, "r+b")
            if not stat.S_ISREG(os.fstat(fd).st_mode):
                raise TaskError("unsafe lock")
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            self.cas = self.root / "cas"
            self.cas.mkdir(mode=0o700, exist_ok=True)
            if self.cas.is_symlink():
                raise TaskError("unsafe CAS")
            self.inputs = {
                name: bounded_regular_read(
                    Path(fixture) / name, LIMIT if name != "validator.wasm" else 1048576
                )
                for name in [
                    "config.json",
                    "schema.json",
                    "policy.json",
                    "notes.txt",
                    "validator.wasm",
                ]
            }
            self.pins = {
                name: digest(self.inputs[file])
                for name, file in [
                    ("initial", "config.json"),
                    ("schema", "schema.json"),
                    ("policy", "policy.json"),
                    ("notes", "notes.txt"),
                    ("validator", "validator.wasm"),
                ]
            }
            self.policy = decode(self.inputs["policy.json"])
            if set(self.policy) != {
                "schema_version",
                "task_id",
                "domain_id",
                "resource_id",
                "allowed_rights",
                "max_grant_ms",
            } or any(type(v) is not int for v in self.policy.values()):
                raise TaskError("invalid policy")
            if (
                (
                    self.policy["schema_version"],
                    self.policy["task_id"],
                    self.policy["domain_id"],
                    self.policy["resource_id"],
                )
                != (1, 17, 7, 1)
                or not 1 <= self.policy["allowed_rights"] <= 31
                or not 1 <= self.policy["max_grant_ms"] <= 300000
            ):
                raise TaskError("invalid policy")
            # Pin worker bytes too; replacement is checked before each validation.
            self.worker_id = hash_regular(self.worker)
            self.sealed_worker = self.root / "worker"
            pending = self.root / (".worker-" + secrets.token_hex(12))
            try:
                fd = os.open(
                    pending, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC, 0o500
                )
                with os.fdopen(fd, "wb") as output:
                    copied = hash_regular(self.worker, output)
                    output.flush()
                    os.fsync(output.fileno())
                if copied != self.worker_id:
                    raise TaskError("io")
                os.chmod(pending, 0o555)
                os.replace(pending, self.sealed_worker)
            finally:
                pending.unlink(missing_ok=True)
            self.sandbox = Sandbox(
                readonly={"/validator": self.sealed_worker}, writable={}
            )
            path = self.root / "journal.json"
            if path.exists() or path.is_symlink():
                self.state = decode(bounded_regular_read(path, 4_000_000))
                self.recover()
            else:
                self.state = {
                    "version": 1,
                    "pins": self.pins,
                    "worker": self.worker_id,
                    "generation": 1,
                    "revision": 0,
                    "content_id": self.pins["initial"],
                    "candidates": {},
                    "receipts": {},
                    "runs": [],
                    "last_validation": None,
                }
            # Every open advances the session epoch, including the initial open.
            self.state["generation"] += 1
            for data in self.inputs.values():
                self.put(data)
            self.save()
            self.bootstrap = {
                "schema_version": 1,
                "task_id": "17",
                "policy_cap": self.policy_cap,
                "resources": [
                    {
                        "resource": "resource:0000000000000001",
                        "logical_resource": "workspace:a/config",
                    },
                    {
                        "resource": "resource:0000000000000064",
                        "logical_resource": "task:schema",
                    },
                    {
                        "resource": "resource:0000000000000065",
                        "logical_resource": "task:notes",
                    },
                ],
            }
        except BaseException:
            self.close()
            raise

    def close(self):
        self.subscriptions.clear()
        if self.lock is not None:
            self.lock.close()
            self.lock = None

    def now(self):
        return (time.monotonic_ns() - self.started) // 1_000_000

    def blob_path(self, cid):
        if not isinstance(cid, str) or not re.fullmatch(r"sha256:[0-9a-f]{64}", cid):
            raise TaskError("io")
        return self.cas / (cid[7:] + ".blob")

    def load(self, cid, limit=LIMIT):
        data = bounded_regular_read(self.blob_path(cid), limit)
        if digest(data) != cid:
            raise TaskError("io")
        return data

    def put(self, data):
        cid = digest(data)
        path = self.blob_path(cid)
        if path.exists() or path.is_symlink():
            self.load(cid, max(LIMIT, len(data)))
        else:
            atomic(path, data)
        return cid

    def save(self):
        data = canonical(self.state)
        if len(data) > 4_000_000:
            raise TaskError("capacity")
        atomic(self.root / "journal.json", data)

    def check_validation(self, v, generation):
        if not isinstance(v, dict) or set(v) != {
            "candidate_id",
            "schema_id",
            "policy_id",
            "validator_id",
            "generation",
            "valid_until_ms",
            "wall_elapsed_ms",
            "guest_elapsed_ms",
            "diagnostics_bytes",
            "diagnostics_truncated",
            "outcome",
        }:
            raise TaskError("validation record")
        for field in ["candidate_id", "schema_id", "policy_id", "validator_id"]:
            self.blob_path(v[field])
        for field in [
            "generation",
            "valid_until_ms",
            "wall_elapsed_ms",
            "guest_elapsed_ms",
            "diagnostics_bytes",
        ]:
            decimal(v[field])
        if (
            not 1 <= int(v["generation"]) <= generation
            or type(v["diagnostics_truncated"]) is not bool
            or int(v["diagnostics_bytes"]) > BUDGET["max_diagnostics_bytes"]
            or v["outcome"] not in ["valid", "invalid", "timeout", "host_failure"]
        ):
            raise TaskError("validation bounds")

    def recover(self):
        s = self.state
        if (
            set(s)
            != {
                "version",
                "pins",
                "worker",
                "generation",
                "revision",
                "content_id",
                "candidates",
                "receipts",
                "runs",
                "last_validation",
            }
            or type(s["version"]) is not int
            or s["version"] != 1
            or s["pins"] != self.pins
            or s["worker"] != self.worker_id
        ):
            raise TaskError("journal pins")
        if (
            type(s["generation"]) is not int
            or not 1 <= s["generation"] < 2**64 - 1
            or type(s["revision"]) is not int
            or not 0 <= s["revision"] <= 64
        ):
            raise TaskError("journal revision")
        if len(s["candidates"]) > 64 or len(s["receipts"]) > 64 or len(s["runs"]) > 128:
            raise TaskError("journal bounds")
        for cap, c in s["candidates"].items():
            if (
                not re.fullmatch(r"cap:[0-9a-f]{16}", cap)
                or cap == "cap:0000000000000000"
                or set(c)
                != {
                    "content_id",
                    "validation",
                }
            ):
                raise TaskError("candidate record")
            self.load(c["content_id"])
            v = c["validation"]
            if v is not None:
                self.check_validation(v, s["generation"])
            if v is not None and (
                v["candidate_id"] != c["content_id"]
                or v["schema_id"] != self.pins["schema"]
                or v["policy_id"] != self.pins["policy"]
                or v["validator_id"] != self.pins["validator"]
            ):
                raise TaskError("validation pins")
        revision = 0
        content = self.pins["initial"]
        caps = set(s["candidates"])
        for request_id, r in sorted(
            s["receipts"].items(), key=lambda item: int(item[1]["reply"]["revision"])
        ):
            decimal(request_id, True)
            if set(r) != {
                "binding",
                "reply",
                "validation",
                "generation",
                "committed_at_ms",
            }:
                raise TaskError("receipt record")
            b = r["binding"]
            reply = r["reply"]
            if (
                set(b) != {"candidate_cap", "expected_revision", "expected_content_id"}
                or b["candidate_cap"] not in s["candidates"]
                or decimal(b["expected_revision"]) != revision
                or b["expected_content_id"] != content
            ):
                raise TaskError("receipt chain")
            c = s["candidates"][b["candidate_cap"]]
            revision += 1
            content = c["content_id"]
            if (
                set(reply) != {"operation", "receipt_cap", "revision", "content_id"}
                or reply["operation"] != "commit_candidate"
                or reply["revision"] != str(revision)
                or reply["content_id"] != content
                or reply["receipt_cap"] in caps
                or reply["receipt_cap"] == "cap:0000000000000000"
                or not re.fullmatch(r"cap:[0-9a-f]{16}", reply["receipt_cap"])
            ):
                raise TaskError("receipt reply")
            v = r["validation"]
            self.check_validation(v, s["generation"])
            if (
                type(r["generation"]) is not int
                or not 1 <= r["generation"] <= s["generation"]
                or type(r["committed_at_ms"]) is not int
                or r["committed_at_ms"] < 0
                or v is None
                or v["generation"] != str(r["generation"])
                or v["candidate_id"] != content
                or v["schema_id"] != self.pins["schema"]
                or v["policy_id"] != self.pins["policy"]
                or v["validator_id"] != self.pins["validator"]
                or v["outcome"] != "valid"
                or v["diagnostics_truncated"]
                or decimal(v["valid_until_ms"]) <= r["committed_at_ms"]
                or decimal(v["wall_elapsed_ms"]) > BUDGET["wall_ms"]
                or decimal(v["guest_elapsed_ms"]) > BUDGET["guest_ms"]
            ):
                raise TaskError("receipt validation")
            caps.add(reply["receipt_cap"])
        if (s["revision"], s["content_id"]) != (revision, content):
            raise TaskError("accepted chain")
        self.load(content)
        if s["last_validation"] is not None and s["last_validation"] not in [
            c["validation"] for c in s["candidates"].values()
        ]:
            raise TaskError("latest validation")
        if any(
            not isinstance(run, dict)
            or run.get("removed") is not True
            or run.get("reconciliation_required") is True
            for run in s["runs"]
        ):
            raise TaskError("container cleanup requires reconciliation")

    def mint(self):
        used = (
            set(self.grants)
            | set(self.subscriptions)
            | set(self.state["candidates"])
            | {r["reply"]["receipt_cap"] for r in self.state["receipts"].values()}
            | {self.policy_cap}
        )
        for _ in range(16):
            cap = token()
            if cap not in used and cap != "cap:0000000000000000":
                return cap
        raise TaskError("capacity")

    def authorize(self, cap, right):
        g = self.grants.get(cap)
        if (
            self.domain != 7
            or g is None
            or g["generation"] != self.state["generation"]
            or not g["bits"] & RIGHTS[right]
        ):
            raise TaskError("denied")
        if self.now() >= g["expires"]:
            raise TaskError("expired")
        return g

    def candidate(self, cap):
        c = self.state["candidates"].get(cap)
        if c is None:
            raise TaskError("denied")
        self.load(c["content_id"])
        return c

    def valid(self, v):
        return (
            v is not None
            and v["generation"] == str(self.state["generation"])
            and self.now() < int(v["valid_until_ms"])
            and v["outcome"] == "valid"
            and not v["diagnostics_truncated"]
            and int(v["wall_elapsed_ms"]) <= BUDGET["wall_ms"]
            and int(v["guest_elapsed_ms"]) <= BUDGET["guest_ms"]
        )

    def execute(self, request):
        rid = None
        version = 1
        try:
            if (
                not isinstance(request, dict)
                or set(request) != {"schema_version", "request_id", "call"}
                or type(request["schema_version"]) is not int
                or request["schema_version"] not in (1, 2)
            ):
                raise TaskError("invalid")
            decimal(request["request_id"], True)
            rid = request["request_id"]
            version = request["schema_version"]
            c = request["call"]
            if self.poisoned:
                raise TaskError("io")
            if (
                not isinstance(c, dict)
                or c.get("operation") not in FIELDS
                or set(c) != (FIELDS[c["operation"]] | {"operation"})
            ):
                raise TaskError("invalid")
            if (
                c["operation"] in ("subscribe_task", "poll_task", "unsubscribe_task")
                and version != 2
            ):
                raise TaskError("invalid")
            # Checks below independently enforce authorization even for direct
            # trusted-evaluator calls that bypass the Rust syntax boundary.
            result = self.dispatch(rid, c)
            return {
                "schema_version": version,
                "request_id": rid,
                "status": "ok",
                "result": result,
            }
        except (
            TaskError,
            OSError,
            SandboxFailure,
            ValueError,
            KeyError,
            TypeError,
        ) as error:
            status = str(error) if isinstance(error, TaskError) else "io"
            if status not in [
                "denied",
                "invalid",
                "conflict",
                "validation_failed",
                "expired",
                "timeout",
                "capacity",
                "io",
                "request_reuse",
                "not_found",
            ]:
                status = "io"
            if status == "io":
                self.poisoned = True
            return {
                "schema_version": version,
                "request_id": rid,
                "status": status,
                "result": None,
            }

    def dispatch(self, rid, c):
        op = c["operation"]
        now = self.now()
        self.subscriptions = {
            cap: s
            for cap, s in self.subscriptions.items()
            if s["grant"] in self.grants
            and self.grants[s["grant"]]["generation"] == self.state["generation"]
            and now < self.grants[s["grant"]]["expires"]
        }
        if op in ("subscribe_task", "poll_task", "unsubscribe_task"):
            for name in ("task_cap", "subscription_cap"):
                if name in c and (
                    not isinstance(c[name], str)
                    or not re.fullmatch(
                        r"cap:(?!0000000000000000$)[0-9a-f]{16}", c[name]
                    )
                ):
                    raise TaskError("invalid")
            self.authorize(c["task_cap"], "observe")
            if op == "subscribe_task":
                types = c["event_types"]
                if (
                    not isinstance(types, list)
                    or not 1 <= len(types) <= 2
                    or any(
                        type(e) is not str
                        or e not in ("output_changed", "validation_changed")
                        for e in types
                    )
                    or len(set(types)) != len(types)
                ):
                    raise TaskError("invalid")
                if len(self.subscriptions) >= 16:
                    raise TaskError("capacity")
                cap = self.mint()
                self.subscriptions[cap] = {
                    "grant": c["task_cap"],
                    "types": set(types),
                    "pending": set(),
                }
                return dict(
                    operation=op,
                    subscription_cap=cap,
                    revision=str(self.state["revision"]),
                )
            sub = self.subscriptions.get(c["subscription_cap"])
            if sub is None:
                raise TaskError("not_found")
            if sub["grant"] != c["task_cap"]:
                raise TaskError("denied")
            if op == "unsubscribe_task":
                del self.subscriptions[c["subscription_cap"]]
                return dict(operation=op, cancelled=True)
            types = [
                e
                for e in ("output_changed", "validation_changed")
                if e in sub["pending"]
            ]
            state = (
                self.dispatch(
                    rid, {"operation": "get_task_state", "task_cap": c["task_cap"]}
                )["state"]
                if types
                else None
            )
            sub["pending"].clear()
            return dict(operation=op, event_types=types, state=state)
        if op == "request_grant":
            rs = c["rights"]
            life = c["lifetime_ms"]
            if (
                not isinstance(rs, list)
                or not rs
                or len(rs) > 5
                or any(r not in RIGHTS for r in rs)
                or len(set(rs)) != len(rs)
                or type(life) is not int
                or not 1 <= life <= 300000
            ):
                raise TaskError("invalid")
            decimal(c["task_id"], True)
            bits = sum(RIGHTS[r] for r in rs)
            if (
                self.domain != 7
                or c["policy_cap"] != self.policy_cap
                or c["task_id"] != "17"
                or c["resource"] != "resource:0000000000000001"
                or bits & ~self.policy["allowed_rights"]
                or life > self.policy["max_grant_ms"]
            ):
                raise TaskError("denied")
            self.grants = {
                cap: g
                for cap, g in self.grants.items()
                if now < g["expires"] and g["generation"] == self.state["generation"]
            }
            if len(self.grants) >= 256:
                raise TaskError("capacity")
            cap = self.mint()
            g = {
                "generation": self.state["generation"],
                "expires": now + life,
                "bits": bits,
            }
            self.grants[cap] = g
            return dict(
                operation=op,
                task_cap=cap,
                generation=str(g["generation"]),
                expires_at_ms=str(g["expires"]),
                rights=[r for r in RIGHTS if bits & RIGHTS[r]],
            )
        if op == "revoke_grant":
            self.grants = {
                cap: g
                for cap, g in self.grants.items()
                if now < g["expires"] and g["generation"] == self.state["generation"]
            }
            if (
                self.domain != 7
                or c["policy_cap"] != self.policy_cap
                or c["task_cap"] not in self.grants
            ):
                raise TaskError("denied")
            if self.state["generation"] >= 2**64 - 1:
                raise TaskError("capacity")
            count = len(self.grants)
            self.state["generation"] += 1
            self.grants.clear()
            self.subscriptions.clear()
            self.save()
            return dict(
                operation=op,
                generation=str(self.state["generation"]),
                revoked_count=count,
            )
        right = {
            "read_input": "read",
            "stage_candidate": "stage",
            "validate_candidate": "validate",
            "commit_candidate": "commit",
            "get_receipt": "commit",
            "get_task_state": "observe",
        }[op]
        g = self.authorize(c["task_cap"], right)
        if op == "read_input":
            name = {
                "resource:0000000000000001": "config.json",
                "resource:0000000000000064": "schema.json",
                "resource:0000000000000065": "notes.txt",
            }.get(c["resource"])
            if name is None:
                raise TaskError("denied")
            # Resource A denotes the current accepted output, matching the
            # native contract. Immutable mounted fixture inputs remain separate
            # LS observations; they do not roll back this logical resource.
            data = (
                self.load(self.state["content_id"])
                if name == "config.json"
                else self.inputs[name]
            )
            return dict(
                operation=op, content_id=digest(data), bytes_base64=encoded(data)
            )
        if op == "stage_candidate":
            b64 = c["bytes_base64"]
            if not isinstance(b64, str) or len(b64) > 87384:
                raise TaskError("invalid")
            try:
                data = base64.b64decode(b64, validate=True)
            except ValueError:
                raise TaskError("invalid") from None
            if not data or len(data) > LIMIT or encoded(data) != b64:
                raise TaskError("invalid")
            if len(self.state["candidates"]) >= 64:
                raise TaskError("capacity")
            content = self.put(data)
            cap = self.mint()
            self.state["candidates"][cap] = {"content_id": content, "validation": None}
            self.save()
            return dict(operation=op, candidate_cap=cap, content_id=content)
        if op == "get_receipt":
            decimal(c["commit_request_id"], True)
            r = self.state["receipts"].get(c["commit_request_id"])
            if r is None:
                raise TaskError("not_found")
            return dict(
                operation=op,
                commit_request_id=c["commit_request_id"],
                revision=r["reply"]["revision"],
                content_id=r["reply"]["content_id"],
            )
        if op == "get_task_state":
            self.load(self.state["content_id"])
            v = self.state["last_validation"]
            state = dict(
                task_id="17",
                resource="resource:0000000000000001",
                revision=str(self.state["revision"]),
                content_id=self.state["content_id"],
                generation=str(self.state["generation"]),
                rights=[r for r in RIGHTS if g["bits"] & RIGHTS[r]],
                expires_at_ms=str(g["expires"]),
                now_ms=str(now),
                schema_id=self.pins["schema"],
                policy_id=self.pins["policy"],
                validator_id=self.pins["validator"],
                input_resources=[r["resource"] for r in self.bootstrap["resources"]],
                validation=v,
                validation_current=self.valid(v),
            )
            return dict(operation=op, state=state)
        candidate = self.candidate(c["candidate_cap"])
        if op == "commit_candidate":
            decimal(c["expected_revision"])
            if not re.fullmatch(r"sha256:[0-9a-f]{64}", c["expected_content_id"]):
                raise TaskError("invalid")
            binding = {
                k: c[k]
                for k in ["candidate_cap", "expected_revision", "expected_content_id"]
            }
            old = self.state["receipts"].get(rid)
            if old is not None:
                if old["binding"] != binding:
                    raise TaskError("request_reuse")
                return old["reply"]
            if (c["expected_revision"], c["expected_content_id"]) != (
                str(self.state["revision"]),
                self.state["content_id"],
            ):
                raise TaskError("conflict")
            if not self.valid(candidate["validation"]):
                raise TaskError("validation_failed")
            if len(self.state["receipts"]) >= 64:
                raise TaskError("capacity")
            revision = self.state["revision"] + 1
            cap = self.mint()
            reply = dict(
                operation=op,
                receipt_cap=cap,
                revision=str(revision),
                content_id=candidate["content_id"],
            )
            self.state["revision"] = revision
            self.state["content_id"] = candidate["content_id"]
            self.state["receipts"][rid] = {
                "binding": binding,
                "reply": reply,
                "validation": copy.deepcopy(candidate["validation"]),
                "generation": self.state["generation"],
                "committed_at_ms": self.now(),
            }
            self.save()
            self.signal("output_changed")
            return reply
        if op == "validate_candidate":
            if c["validator_id"] != self.pins["validator"]:
                raise TaskError("denied")
            if len(self.state["runs"]) >= 128:
                raise TaskError("capacity")
            candidate["validation"] = None
            self.state["last_validation"] = None
            self.save()
            start = time.monotonic_ns()
            if hash_regular(self.sealed_worker) != self.worker_id:
                raise TaskError("io")
            outcome = "host_failure"
            diagnostics = b""
            truncated = False
            guest = 0
            failure = None
            with tempfile.TemporaryDirectory(
                prefix="ramenos-lt-validator-"
            ) as temporary:
                store = Path(temporary)
                os.chmod(store, 0o755)
                for cid, limit in dict(
                    [
                        (candidate["content_id"], LIMIT),
                        (self.pins["schema"], LIMIT),
                        (self.pins["validator"], 1048576),
                    ]
                ).items():
                    (store / (cid[7:] + ".blob")).write_bytes(self.load(cid, limit))
                    os.chmod(store / (cid[7:] + ".blob"), 0o444)
                budget = copy.deepcopy(BUDGET)
                elapsed = (time.monotonic_ns() - start) // 1_000_000
                budget["wall_ms"] -= elapsed
                if budget["wall_ms"] <= 0:
                    raise TaskError("timeout")
                job = dict(
                    schema_version=1,
                    store_root="/store",
                    candidate_id=candidate["content_id"],
                    schema_id=self.pins["schema"],
                    validator_id=self.pins["validator"],
                    budget=budget,
                )
                try:
                    sandbox = Sandbox(
                        readonly={"/store": store, "/validator": self.sealed_worker},
                        writable={},
                    )
                    remaining = (
                        BUDGET["wall_ms"] - (time.monotonic_ns() - start) // 1_000_000
                    )
                    if remaining <= 0:
                        raise SandboxFailure("timeout")
                    job["budget"]["wall_ms"] = remaining
                    job["budget"]["guest_ms"] = min(BUDGET["guest_ms"], remaining)
                    job["budget"]["host_call_ms"] = min(
                        BUDGET["host_call_ms"], remaining
                    )
                    run = sandbox.run(
                        ["/validator"], input_bytes=canonical(job), wall_ms=remaining
                    )
                    self.state["runs"].append(run.evidence)
                    result = decode(run.stdout)
                    if (
                        set(result)
                        != {
                            "schema_version",
                            "outcome",
                            "diagnostics",
                            "truncated",
                            "guest_elapsed_ms",
                        }
                        or type(result["schema_version"]) is not int
                        or result["schema_version"] != 1
                        or result["outcome"]
                        not in ["valid", "invalid", "timeout", "host_failure"]
                        or type(result["truncated"]) is not bool
                        or type(result["guest_elapsed_ms"]) is not int
                        or result["guest_elapsed_ms"] < 0
                        or not isinstance(result["diagnostics"], list)
                        or any(
                            type(b) is not int or not 0 <= b <= 255
                            for b in result["diagnostics"]
                        )
                    ):
                        raise TaskError("validation_failed")
                    diagnostics = bytes(result["diagnostics"])
                    guest = result["guest_elapsed_ms"]
                    truncated = result["truncated"]
                    outcome = result["outcome"]
                    if (
                        len(diagnostics) > BUDGET["max_diagnostics_bytes"]
                        or guest > BUDGET["guest_ms"]
                    ):
                        raise TaskError("validation_failed")
                    if truncated and outcome == "valid":
                        outcome = "host_failure"
                except SandboxFailure as error:
                    if error.evidence:
                        self.state["runs"].append(error.evidence)
                        if (
                            error.evidence.get("removed") is not True
                            or error.reason == "creation_not_confirmed"
                        ):
                            error.evidence["reconciliation_required"] = True
                            self.poisoned = True
                            self.save()
                            raise TaskError("io") from None
                    outcome = "timeout" if error.reason == "timeout" else "host_failure"
                    failure = "timeout" if outcome == "timeout" else "io"
                except (ValueError, KeyError, TypeError, TaskError):
                    outcome = "host_failure"
                    failure = "validation_failed"
                    diagnostics = b""
            elapsed = (time.monotonic_ns() - start) // 1_000_000
            if elapsed > BUDGET["wall_ms"]:
                outcome = "timeout"
                failure = "timeout"
            # Preserve cleanup evidence even if the grant expired during execution.
            self.save()
            self.authorize(c["task_cap"], "validate")
            until = min(g["expires"], self.now() + BUDGET["guest_ms"])
            candidate["validation"] = dict(
                candidate_id=candidate["content_id"],
                schema_id=self.pins["schema"],
                policy_id=self.pins["policy"],
                validator_id=self.pins["validator"],
                generation=str(self.state["generation"]),
                valid_until_ms=str(until),
                wall_elapsed_ms=str(elapsed),
                guest_elapsed_ms=str(guest),
                diagnostics_bytes=str(len(diagnostics)),
                diagnostics_truncated=truncated,
                outcome=outcome,
            )
            self.state["last_validation"] = candidate["validation"]
            self.save()
            self.signal("validation_changed")
            if failure:
                raise TaskError(failure)
            return dict(
                operation=op,
                outcome=outcome,
                valid_until_ms=str(until),
                diagnostics_base64=encoded(diagnostics),
                truncated=truncated,
            )
        raise TaskError("invalid")

    def signal(self, event):
        for sub in self.subscriptions.values():
            if event in sub["types"]:
                sub["pending"].add(event)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--store", type=Path, required=True)
    parser.add_argument("--worker", type=Path, required=True)
    args = parser.parse_args()
    task = LinuxTask(args.fixture, args.store, args.worker, 7)
    try:
        sys.stdout.buffer.write(canonical(task.bootstrap) + b"\n")
        sys.stdout.buffer.flush()
        while True:
            line = sys.stdin.buffer.readline(MAX_FRAME + 2)
            if not line:
                break
            if not line.endswith(b"\n") or len(line) > MAX_FRAME + 1:
                raise TaskError("frame bound")
            reply = task.execute(decode(line))
            sys.stdout.buffer.write(canonical(reply) + b"\n")
            sys.stdout.buffer.flush()
    finally:
        task.close()


if __name__ == "__main__":
    try:
        main()
    except Exception:
        print(
            "LT broker stopped; explicit receipt lookup may be required",
            file=sys.stderr,
        )
        sys.exit(1)
