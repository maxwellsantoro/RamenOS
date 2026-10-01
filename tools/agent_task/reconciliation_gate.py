#!/usr/bin/env python3
"""Named forced lifecycle and interrupted commit recovery through real adapters."""

import argparse
import base64
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import uuid

from evaluator_session import Session, Limits, SessionStopped
from evaluator_controls_gate import (
    repair,
    shell_command,
    LS_TOOLS,
    binary_sha,
    private_write,
)
from lifecycle_ledger import reconcile_journal, encoded, sha


class CommitReady(Exception):
    pass


def send_abandoned(session, request):
    data = encoded(request) + b"\n"
    session.offer("request", data)
    session.requests += 1
    offset = 0
    deadline = time.monotonic() + 1
    while offset < len(data):
        assert time.monotonic() < deadline
        try:
            offset += os.write(session.process.stdin.fileno(), data[offset:])
        except BlockingIOError:
            time.sleep(0.001)


def abort(session):
    try:
        session.stop("injected_interruption")
    except SessionStopped:
        pass
    session.close()


def read_journal(root, arm):
    return json.loads(
        (root / ("task.json" if arm == "RT" else "journal.json")).read_bytes()
    )


def pointer(journal, arm):
    return (
        journal["state"]["accepted"]
        if arm == "RT"
        else dict(revision=journal["revision"], content_id=journal["content_id"])
    )


def launch(arm, fixture, store, candidate, args):
    if arm == "LS":
        command = [
            sys.executable,
            str(Path(__file__).with_name("ls_launcher.py").resolve()),
            "--fixture",
            str(fixture),
            "--store",
            str(store),
            "--candidate",
            str(candidate),
            "--worker",
            str(args.worker),
            "--wall-ms",
            "10000",
        ]
        tools = encoded(LS_TOOLS)
    else:
        binary = args.rt if arm == "RT" else args.lt
        command = [
            str(binary),
            "--fixture",
            str(fixture),
            "--store",
            str(store),
            "--worker",
            str(args.worker),
        ]
        tools = subprocess.check_output([str(binary), "--describe"], timeout=10)
    s = Session(
        command,
        Limits(request_bytes=8192 if arm == "LS" else 131072),
        visible=[
            (
                "task",
                b"Scripted development repair and explicit interrupted-commit receipt lookup",
            ),
            ("tools", tools),
        ],
        docker_scope=True,
        lifecycle_parent=args.evidence / "lifecycles",
    )
    return s, s.bootstrap["task"] if arm == "LS" else s.bootstrap


def request_call(s, arm, request):
    if arm != "LS":
        return s.call(request)
    outer = dict(
        schema_version=1,
        request_id=request["request_id"],
        command=shell_command(request),
    )
    r = s.call(outer)
    assert (
        r["status"] == "ok" and not r["stderr_base64"] and r["exit_code"] in (0, 1)
    ), r
    response = json.loads(base64.b64decode(r["stdout_base64"], validate=True))
    assert r["exit_code"] == (0 if response["status"] == "ok" else 1)
    return response


def save_session(args, name, s, store, arm):
    private_write(args.evidence / (name + "-session.json"), encoded(s.evidence()))
    private_write(args.evidence / (name + "-context.bin"), s.context())
    private_write(
        args.evidence / (name + "-journal.json"), encoded(read_journal(store, arm))
    )


def commit_case(args, arm, phase, root):
    store = root / "store"
    candidate = root / "candidate"
    session, bootstrap = launch(arm, args.fixture, store, candidate, args)
    pending = None

    def call(request):
        nonlocal pending
        if request["call"]["operation"] == "commit_candidate":
            pending = request
            raise CommitReady()
        return request_call(session, arm, request)

    try:
        try:
            repair(call, bootstrap)
        except CommitReady:
            pass
        assert pending is not None
        oldcap = pending["call"]["task_cap"]
        pending["request_id"] = "55"
        if phase == "after_publication":
            outgoing = (
                pending
                if arm != "LS"
                else dict(
                    schema_version=1,
                    request_id="55",
                    command=shell_command(pending) + "; sleep 30",
                )
            )
            send_abandoned(session, outgoing)
            deadline = time.monotonic() + 5
            while pointer(read_journal(store, arm), arm)["revision"] != 1:
                assert time.monotonic() < deadline, (
                    "commit not published within checkpoint bound"
                )
                time.sleep(0.005)
        abort(session)
    finally:
        session.close()
        save_session(args, arm.lower() + "-" + phase, session, store, arm)
    cleanup = session.evidence()["container_cleanup"]
    assert cleanup["certified"] and not cleanup["possible_inflight_create"], cleanup
    before = read_journal(store, arm)
    effects = {k: v for k, v in before.items() if k != "runs"}
    repaired = dict(repaired=0, task_effects_modified=False)
    if arm != "RT":
        repaired = reconcile_journal(
            store, session.lifecycle, cleanup["reconciliation"]
        )
        assert {
            k: v for k, v in read_journal(store, arm).items() if k != "runs"
        } == effects
        if arm == "LS" and phase == "after_publication":
            assert repaired["repaired"] >= 1, (
                "sleeping shell checkpoint was not exercised"
            )
    restarted, b = launch(arm, args.fixture, store, candidate, args)
    try:

        def invoke(n, operation, **fields):
            response = request_call(
                restarted,
                arm,
                dict(
                    schema_version=1,
                    request_id=str(n),
                    call=dict(operation=operation, **fields),
                ),
            )
            assert response["request_id"] == str(n) and response["schema_version"] == 1
            return response

        denied = invoke(70, "get_receipt", task_cap=oldcap, commit_request_id="55")
        assert denied["status"] == "denied" and denied["result"] is None
        grant = invoke(
            71,
            "request_grant",
            policy_cap=b["policy_cap"],
            task_id=b["task_id"],
            resource=b["resources"][0]["resource"],
            rights=["read", "observe", "commit"],
            lifetime_ms=60000,
        )
        assert grant["status"] == "ok"
        cap = grant["result"]["task_cap"]
        state = invoke(72, "get_task_state", task_cap=cap)["result"]["state"]
        receipts = [
            invoke(n, "get_receipt", task_cap=cap, commit_request_id="55")
            for n in (73, 74)
        ]
        expected = 1 if phase == "after_publication" else 0
        assert state["revision"] == str(expected)
        if expected:
            assert all(
                r["status"] == "ok"
                and r["result"]["revision"] == "1"
                and r["result"]["content_id"] == state["content_id"]
                for r in receipts
            )
            matches = list((store / "cas").rglob(state["content_id"][7:] + ".blob"))
            assert (
                len(matches) == 1
                and sha(matches[0].read_bytes()) == state["content_id"][7:]
            )
            assert (
                matches[0].read_bytes() == (args.fixture / "schema.json").read_bytes()
            )
        else:
            assert all(
                r["status"] == "not_found" and r["result"] is None for r in receipts
            )
            assert state["content_id"] == "sha256:" + sha(
                (args.fixture / "config.json").read_bytes()
            )
        restarted.finish("Explicit receipt lookup only; commit was not retried")
    finally:
        restarted.close()
        save_session(
            args, arm.lower() + "-" + phase + "-recovered", restarted, store, arm
        )
    assert restarted.evidence()["outcome"] == "completed"
    final = read_journal(store, arm)
    assert (
        pointer(final, arm)["revision"] == expected
        and len(final["receipts"]) == expected
    )
    return dict(
        arm=arm,
        phase=phase,
        accepted_revision=expected,
        receipt_count=expected,
        old_grant_denied=True,
        commit_retried=False,
        cleanup_certified=True,
        journal_reconciliation=repaired,
    )


def late_create_case(args):
    code = "import time;print('{}',flush=True);time.sleep(30)"
    session = Session(
        [sys.executable, "-u", "-c", code],
        visible=[("task", b"Private late-create probe")],
        docker_scope=True,
        lifecycle_parent=args.evidence / "lifecycles",
    )
    image = json.loads(
        subprocess.check_output(
            [
                "docker",
                "image",
                "inspect",
                "python@sha256:139020233cc412efe4c8135b0efe1c7569dc8b28ddd88bddb109b764f8977e30",
            ],
            timeout=10,
        )
    )[0]["Id"]
    identity = uuid.uuid4().hex
    session.lifecycle.begin(identity, image)
    abort(session)
    initial = session.evidence()["container_cleanup"]
    assert (
        initial["observed_empty"]
        and not initial["certified"]
        and initial["possible_inflight_create"]
    )
    # A prepared request reaches the actual daemon after the first empty
    # inventory. This simulates a paused/in-flight caller, not a model operation.
    result = (
        subprocess.check_output(
            [
                "docker",
                "create",
                "--name",
                "ramenos-sw0-" + identity,
                "--label",
                "org.ramenos.evaluator-session=" + session.scope,
                image,
                "true",
            ],
            timeout=10,
        )
        .decode()
        .strip()
    )
    try:
        session.lifecycle.acknowledge(identity, result)
        session.cleanup_deadline = (
            time.monotonic() + 10
        )  # explicit new reconciliation window
        reconciled = session.reconcile_containers()
        assert reconciled["certified"] and reconciled["observed_empty"]
        stale = subprocess.run(
            ["docker", "start", result], capture_output=True, timeout=5
        )
        assert stale.returncode != 0 and b"No such" in stale.stderr
        private_write(
            args.evidence / "late-create.json",
            encoded(
                dict(
                    initial=initial,
                    reconciled=reconciled,
                    lifecycle=session.lifecycle.snapshot(),
                )
            ),
        )
        return dict(
            empty_inventory_did_not_certify=True,
            late_actual_create_acknowledged=True,
            explicit_reconciliation_certified=True,
            stale_start_failed=True,
        )
    finally:
        subprocess.run(
            ["docker", "rm", "--force", result], capture_output=True, timeout=5
        )


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ("rt", "lt", "worker", "fixture", "evidence"):
        p.add_argument("--" + name, type=Path, required=True)
    args = p.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ramenos-reconciliation-") as temp:
        cases = []
        for arm in ("RT", "LT", "LS"):
            for phase in ("before_dispatch", "after_publication"):
                root = Path(temp) / (arm + "-" + phase)
                root.mkdir()
                cases.append(commit_case(args, arm, phase, root))
        late = late_create_case(args)
    files = (
        subprocess.check_output(["git", "ls-files", "-co", "--exclude-standard", "-z"])
        .decode()
        .split("\0")
    )
    source = {
        name: sha(Path(name).read_bytes())
        for name in sorted(set(files) - {""})
        if Path(name).is_file()
    }
    source["Cargo.lock"] = sha(Path("Cargo.lock").read_bytes())
    private_write(args.evidence / "source-files.json", encoded(source))
    report = dict(
        schema_version=1,
        source_commit=subprocess.check_output(
            ["git", "rev-parse", "HEAD"], text=True
        ).strip(),
        source_files_sha256=sha(encoded(source)),
        claim="acknowledged-lifecycle-and-explicit-receipt-recovery",
        cases=cases,
        late_create=late,
        binaries_sha256=dict(
            RT=binary_sha(args.rt),
            LT=binary_sha(args.lt),
            worker=binary_sha(args.worker),
        ),
        unacknowledged_create_certified=False,
        model_trial=False,
        full_a2_conformance=False,
        target_kernel_enforcement=False,
    )
    private_write(args.evidence / "report.json", encoded(report))
    print(
        "Reconciliation: PASS six interrupted commit cases; pending/late create stays unknown until explicit acknowledgement/removal"
    )


if __name__ == "__main__":
    main()
