#!/usr/bin/env python3
"""Private host lifecycle receipts. No model API or native interface authority."""

from contextlib import contextmanager
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import time
import uuid


class LedgerError(RuntimeError):
    pass


def require(condition):
    if not condition:
        raise LedgerError("lifecycle evidence unavailable or inconsistent")


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def sha(data):
    return hashlib.sha256(data).hexdigest()


def load(path, limit=262144):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result)
            result[key] = value
        return result

    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    with os.fdopen(fd, "rb") as stream:
        meta = os.fstat(stream.fileno())
        require(
            stat.S_ISREG(meta.st_mode)
            and meta.st_uid == os.geteuid()
            and meta.st_mode & 0o077 == 0
        )
        data = stream.read(limit + 1)
    require(0 < len(data) <= limit)

    def constant(_):
        raise LedgerError("nonfinite lifecycle evidence")

    return json.loads(
        data.decode("utf-8"), object_pairs_hook=pairs, parse_constant=constant
    )


def atomic(path, value):
    pending = path.with_name(".next-" + uuid.uuid4().hex)
    try:
        fd = os.open(
            pending, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC, 0o600
        )
        with os.fdopen(fd, "wb") as stream:
            stream.write(encoded(value))
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(pending, path)
        fd = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)
    finally:
        pending.unlink(missing_ok=True)


class Ledger:
    @classmethod
    def create(cls, root, scope):
        root = Path(root)
        root.mkdir(mode=0o700, parents=True, exist_ok=False)
        atomic(
            root / "state.json",
            dict(schema_version=1, scope=scope, state="open", entries={}),
        )
        return cls(root, scope)

    def __init__(self, root, scope):
        self.root = Path(root)
        self.scope = scope
        require(
            isinstance(scope, str) and re.fullmatch(r"[0-9a-f]{32}", scope) is not None
        )
        meta = self.root.lstat()
        require(
            stat.S_ISDIR(meta.st_mode)
            and meta.st_uid == os.geteuid()
            and meta.st_mode & 0o077 == 0
        )

    @contextmanager
    def locked(self):
        try:
            fd = os.open(
                self.root / "lock",
                os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC,
                0o600,
            )
            with os.fdopen(fd, "r+b"):
                meta = os.fstat(fd)
                require(
                    stat.S_ISREG(meta.st_mode)
                    and meta.st_uid == os.geteuid()
                    and meta.st_mode & 0o077 == 0
                )
                deadline = time.monotonic() + 0.5
                while True:
                    try:
                        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                        break
                    except BlockingIOError:
                        if time.monotonic() >= deadline:
                            raise LedgerError("lifecycle lock deadline")
                        time.sleep(0.005)
                state = load(self.root / "state.json")
                self.validate(state)
                yield state
        except (OSError, ValueError, TypeError, KeyError) as error:
            raise LedgerError("lifecycle ledger rejected") from error

    def validate(self, state):
        require(
            isinstance(state, dict)
            and set(state) == {"schema_version", "scope", "state", "entries"}
        )
        require(
            type(state["schema_version"]) is int
            and state["schema_version"] == 1
            and state["scope"] == self.scope
            and state["state"] in ("open", "fenced")
        )
        require(isinstance(state["entries"], dict) and len(state["entries"]) <= 128)
        for identity, item in state["entries"].items():
            require(re.fullmatch(r"[0-9a-f]{32}", identity) is not None)
            require(
                isinstance(item, dict)
                and set(item)
                == {"name", "image_id", "phase", "container_id", "owner_pid"}
            )
            require(
                item["name"] == "ramenos-sw0-" + identity
                and re.fullmatch(r"sha256:[0-9a-f]{64}", item["image_id"]) is not None
            )
            require(
                type(item["owner_pid"]) is int
                and item["owner_pid"] > 0
                and item["phase"] in ("intent", "acknowledged", "removed")
            )
            require(
                item["container_id"] is None
                if item["phase"] == "intent"
                else isinstance(item["container_id"], str)
                and re.fullmatch(r"[0-9a-f]{64}", item["container_id"]) is not None
            )
        require(
            len(
                {
                    i["container_id"]
                    for i in state["entries"].values()
                    if i["container_id"] is not None
                }
            )
            == sum(i["container_id"] is not None for i in state["entries"].values())
        )

    def snapshot(self):
        with self.locked() as state:
            return state

    def begin(self, identity, image_id):
        with self.locked() as state:
            require(
                state["state"] == "open"
                and identity not in state["entries"]
                and len(state["entries"]) < 128
            )
            state["entries"][identity] = dict(
                name="ramenos-sw0-" + identity,
                image_id=image_id,
                phase="intent",
                container_id=None,
                owner_pid=os.getpid(),
            )
            self.validate(state)
            atomic(self.root / "state.json", state)

    def acknowledge(self, identity, container_id):
        with self.locked() as state:
            item = state["entries"][identity]
            require(
                item["phase"] == "intent"
                and re.fullmatch(r"[0-9a-f]{64}", container_id) is not None
            )
            item.update(phase="acknowledged", container_id=container_id)
            self.validate(state)
            atomic(self.root / "state.json", state)

    def removed(self, identity):
        with self.locked() as state:
            require(state["entries"][identity]["phase"] == "acknowledged")
            state["entries"][identity]["phase"] = "removed"
            atomic(self.root / "state.json", state)

    def fence(self):
        with self.locked() as state:
            state["state"] = "fenced"
            atomic(self.root / "state.json", state)

    def seal_reconciliation(self, *, observed_empty):
        state = self.snapshot()
        require(
            observed_empty is True
            and state["state"] == "fenced"
            and all(i["phase"] == "removed" for i in state["entries"].values())
        )
        return dict(
            schema_version=1,
            scope=self.scope,
            ledger_sha256=sha(encoded(state)),
            certified=True,
            observed_empty=True,
        )


def reconcile_journal(store, ledger, proof):
    """Explicit trusted recovery of cleanup checkpoints; task effects stay intact."""
    require(isinstance(proof, dict))
    require(type(proof.get("schema_version")) is int)
    require(proof.get("certified") is True and proof.get("observed_empty") is True)
    require(proof == ledger.seal_reconciliation(observed_empty=True))
    state = ledger.snapshot()
    store = Path(store)
    require(store.is_dir() and not store.is_symlink())
    fd = os.open(
        store / "writer.lock",
        os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC,
        0o600,
    )
    with os.fdopen(fd, "r+b"):
        require(stat.S_ISREG(os.fstat(fd).st_mode))
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise LedgerError("journal has an active writer") from error
        journal = load(store / "journal.json", 2097152)
        require(
            isinstance(journal, dict)
            and isinstance(journal.get("runs"), list)
            and len(journal["runs"]) <= 128
        )
        original = sha(encoded(journal))
        repaired = 0
        for run in journal["runs"]:
            require(isinstance(run, dict))
            if (
                run.get("removed") is True
                and not run.get("invocation_pending")
                and not run.get("reconciliation_required")
            ):
                continue
            identity = run.get("lifecycle_invocation")
            require(identity in state["entries"])
            receipt = state["entries"][identity]
            require(receipt["phase"] == "removed")
            prior = sha(encoded(run))
            role = run.get("role")
            require(role in ("ls_shell", "lt_validator"))
            run.clear()
            run.update(
                role=role,
                lifecycle_invocation=identity,
                created=True,
                removed=True,
                invocation_pending=False,
                reconciled=True,
                name=receipt["name"],
                image_id=receipt["image_id"],
                container_id=receipt["container_id"],
                reconciliation_scope=ledger.scope,
                reconciliation_ledger_sha256=proof["ledger_sha256"],
                prior_evidence_sha256=prior,
            )
            repaired += 1
        if repaired:
            atomic(store / "journal.json", journal)
        return dict(
            repaired=repaired,
            prior_journal_sha256=original,
            reconciled_journal_sha256=sha(encoded(journal)),
            task_effects_modified=False,
        )
