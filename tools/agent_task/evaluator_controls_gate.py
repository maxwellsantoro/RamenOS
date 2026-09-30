#!/usr/bin/env python3
"""Scripted A2.7 matrix. Synthetic partitions never stand in for a hidden study."""

import argparse
import base64
import hashlib
import itertools
import json
import os
from pathlib import Path
import shlex
import stat
import subprocess
import sys
import tempfile

from jsonschema import Draft202012Validator
from evaluator_session import Limits, Session, SessionStopped
from fixture_bank import (
    create,
    encoded,
    commitments,
    manifest_hash,
    select_fixture,
    read,
    sha,
)

LS_TOOLS = dict(
    schema_version=1,
    interface="scoped_shell",
    input_schema=dict(
        type="object",
        additionalProperties=False,
        required=["schema_version", "request_id", "command"],
        properties=dict(
            schema_version=dict(type="integer", const=1),
            request_id=dict(type="string", pattern="^[1-9][0-9]*$"),
            command=dict(type="string", minLength=1, maxLength=8192),
        ),
    ),
    description="A command string runs /bin/sh -c in one contained process. Fixed /inputs is read-only; /candidate is writable. /task/taskctl exposes grant, read, stage, validate, commit, receipt, state, revoke. Use --request-id ID before the verb. grant takes --policy-cap, --task-id, --resource, --rights read,stage,validate,commit,observe, --lifetime-ms. Other verbs take --task-cap. read takes --resource; stage --file; validate --candidate-cap --validator-id; commit --candidate-cap --expected-revision --expected-content-id; receipt --commit-request-id; revoke --policy-cap. No automatic retry or repair. A lost commit reply may require explicit receipt lookup.",
    command_wall_ms=10000,
    command_frame_bytes=8192,
    combined_output_bytes=16384,
)


def private_write(path, data):
    path.write_bytes(data)
    os.chmod(path, 0o600)


def binary_sha(path):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    with os.fdopen(fd, "rb") as stream:
        assert stat.S_ISREG(os.fstat(stream.fileno()).st_mode)
        total = 0
        hasher = hashlib.sha256()
        while chunk := stream.read(1048576):
            total += len(chunk)
            assert total <= 1073741824, "binary hash bound"
            hasher.update(chunk)
    assert total > 0
    return hasher.hexdigest()


class TaskTimeout(RuntimeError):
    pass


def retain(session, evidence, name, store):
    private_write(evidence / (name + "-context.bin"), session.context())
    private_write(
        evidence / (name + "-session.json"), encoded(session.evidence()) + b"\n"
    )
    for filename in ("journal.json", "task.json"):
        path = store / filename
        if path.exists():
            private_write(evidence / (name + "-" + filename), read(path, 2097152))


def schedule(instances):
    ids = [i["instance_id"] for i in instances]
    assert len(ids) == len(set(ids)), "repeats require a cluster-aware study contract"
    ordered = sorted(
        ids, key=lambda identity: sha(("arm-order-v1/" + identity).encode())
    )
    permutations = list(itertools.permutations(("RT", "LT", "LS")))
    return [
        dict(instance_id=identity, order=list(permutations[n % 6]))
        for n, identity in enumerate(ordered)
    ]


def shell_command(request):
    call = dict(request["call"])
    verb = dict(
        request_grant="grant",
        read_input="read",
        stage_candidate="stage",
        validate_candidate="validate",
        commit_candidate="commit",
        get_receipt="receipt",
        get_task_state="state",
        revoke_grant="revoke",
    )[call.pop("operation")]
    prefix = ""
    if "bytes_base64" in call:
        payload = call.pop("bytes_base64")
        path = "/candidate/repair-" + request["request_id"]
        code = (
            "import base64;from pathlib import Path;Path("
            + repr(path)
            + ").write_bytes(base64.b64decode("
            + repr(payload)
            + ",validate=True))"
        )
        prefix = shlex.join(["python3", "-c", code]) + " && "
        call["file"] = path
    args = ["/task/taskctl", "--request-id", request["request_id"], verb]
    for key, value in call.items():
        args += [
            "--" + key.replace("_", "-"),
            ",".join(value) if isinstance(value, list) else str(value),
        ]
    return prefix + shlex.join(args)


def repair(call, bootstrap):
    counter = 0

    def invoke(operation, **fields):
        nonlocal counter
        counter += 1
        response = call(
            dict(
                schema_version=1,
                request_id=str(counter),
                call=dict(operation=operation, **fields),
            )
        )
        if response["status"] == "timeout":
            assert response["result"] is None
            raise TaskTimeout(operation)
        assert (
            response["schema_version"] == 1
            and response["request_id"] == str(counter)
            and response["status"] == "ok"
        ), response
        assert response["result"]["operation"] == operation
        return response["result"]

    cap = invoke(
        "request_grant",
        policy_cap=bootstrap["policy_cap"],
        task_id=bootstrap["task_id"],
        resource=bootstrap["resources"][0]["resource"],
        rights=["read", "stage", "validate", "commit", "observe"],
        lifetime_ms=60000,
    )["task_cap"]
    state = invoke("get_task_state", task_cap=cap)["state"]
    assert state["revision"] == "0"
    # This is the scripted consumer, not an adapter/tool operation. The target
    # document is an authorized input; the private oracle is never read here.
    inputs = [
        invoke("read_input", task_cap=cap, resource=r["resource"])
        for r in bootstrap["resources"]
    ]
    target = json.loads(base64.b64decode(inputs[1]["bytes_base64"], validate=True))
    payload = encoded(target)
    staged = invoke(
        "stage_candidate", task_cap=cap, bytes_base64=base64.b64encode(payload).decode()
    )
    valid = invoke(
        "validate_candidate",
        task_cap=cap,
        candidate_cap=staged["candidate_cap"],
        validator_id=state["validator_id"],
    )
    assert valid["outcome"] == "valid" and not valid["truncated"]
    receipt = invoke(
        "commit_candidate",
        task_cap=cap,
        candidate_cap=staged["candidate_cap"],
        expected_revision=state["revision"],
        expected_content_id=state["content_id"],
    )
    commit_request = str(counter)
    assert receipt["revision"] == "1" and receipt["content_id"] == "sha256:" + sha(
        payload
    )
    recovered = invoke("get_receipt", task_cap=cap, commit_request_id=commit_request)
    assert (
        recovered["revision"] == receipt["revision"]
        and recovered["content_id"] == receipt["content_id"]
    )
    final = invoke(
        "read_input", task_cap=cap, resource=bootstrap["resources"][0]["resource"]
    )
    assert base64.b64decode(final["bytes_base64"], validate=True) == payload
    return receipt["content_id"]


def grade(store, oracle, arm, accepted):
    truth = json.loads(read(oracle))
    assert accepted == "sha256:" + truth["expected_sha256"]
    journal = json.loads(
        read(store / ("task.json" if arm == "RT" else "journal.json"), 2097152)
    )
    pointer = (
        journal["state"]["accepted"]
        if arm == "RT"
        else dict(revision=journal["revision"], content_id=journal["content_id"])
    )
    assert pointer == dict(revision=1, content_id=accepted)
    matches = list((store / "cas").rglob(accepted[7:] + ".blob"))
    assert len(matches) == 1
    data = read(matches[0], 65536)
    assert sha(data) == accepted[7:] and json.loads(data) == truth["expected"]
    if arm != "RT":
        assert journal["runs"] and all(
            r.get("removed") is True
            and not r.get("invocation_pending")
            and not r.get("reconciliation_required")
            for r in journal["runs"]
        )
    return dict(
        accepted_revision=1,
        accepted_bytes_sha256=sha(data),
        private_oracle_checked=True,
        journal_pointer_checked=True,
    )


def run_arm(arm, fixture, item, root, binaries, worker, contracts, evidence):
    store = root / "store"
    assert not root.exists(), "fresh storage per arm"
    root.mkdir()
    descriptor = encoded(LS_TOOLS) if arm == "LS" else contracts[1]
    visible = [("task", item["task_text"].encode()), ("tools-v1", descriptor)]
    if arm != "LS":
        visible.append(("tools-v2", contracts[2]))
    command = (
        [
            str(binaries[arm]),
            "--fixture",
            str(fixture),
            "--store",
            str(store),
            "--worker",
            str(worker),
        ]
        if arm != "LS"
        else [
            sys.executable,
            str(Path(__file__).with_name("ls_launcher.py").resolve()),
            "--fixture",
            str(fixture),
            "--store",
            str(store),
            "--candidate",
            str(root / "candidate"),
            "--worker",
            str(worker),
            "--wall-ms",
            "10000",
        ]
    )
    limit = Limits(request_bytes=8192 if arm == "LS" else 131072)
    name = item["instance_id"] + "-" + arm.lower()
    try:
        session = Session(command, limit, visible=visible, docker_scope=True)
    except SessionStopped as error:
        private_write(
            evidence / (name + "-session.json"), encoded(error.evidence) + b"\n"
        )
        private_write(evidence / (name + "-context.bin"), error.context)
        raise
    schema = json.loads(contracts[1])
    request_schemas = {
        t["name"]: Draft202012Validator(t["input_schema"]) for t in schema["tools"]
    }
    response_schema = Draft202012Validator(schema["response_schema"])
    try:
        bootstrap = session.bootstrap["task"] if arm == "LS" else session.bootstrap
        Draft202012Validator(schema["bootstrap_schema"]).validate(bootstrap)

        def call(request):
            request_schemas[request["call"]["operation"]].validate(request)
            if arm == "LS":
                outer = dict(
                    schema_version=1,
                    request_id=request["request_id"],
                    command=shell_command(request),
                )
                Draft202012Validator(LS_TOOLS["input_schema"]).validate(outer)
                reply = session.call(outer)
                assert (
                    reply["status"] == "ok"
                    and reply["exit_code"] in (0, 1)
                    and not reply["stderr_base64"]
                )
                exit_code = reply["exit_code"]
                reply = json.loads(
                    base64.b64decode(reply["stdout_base64"], validate=True)
                )
                assert exit_code == (0 if reply["status"] == "ok" else 1)
            else:
                reply = session.call(request)
            response_schema.validate(reply)
            return reply

        try:
            accepted = repair(call, bootstrap)
            task_outcome = "completed"
            session.finish("Committed " + accepted)
        except TaskTimeout as error:
            assert str(error) == "validate_candidate"
            accepted = None
            task_outcome = "task_timeout"
            session.finish("Validator timed out; no successful publication claimed")
    finally:
        session.close()
        retain(session, evidence, name, store)
    result = session.evidence()
    assert (
        result["outcome"] == "completed"
        and result["container_cleanup"]["certified"]
        and result["process_cleanup"]["observed_descendants_not_running"]
    ), result
    assert (
        result["context_bytes"]
        == sum(r["bytes"] for r in result["visible_ledger"])
        == len(session.context())
    )
    # Only selected task/authorized inputs and this arm's interactions enter
    # context. No private grading result or other arm output is appended.
    assert (
        b"oracle.json" not in session.context()
        and b"private_oracle_checked" not in session.context()
    )
    if accepted is not None:
        checked = grade(store, fixture.parent / "oracle.json", arm, accepted)
    else:
        journal = json.loads(
            read(store / ("task.json" if arm == "RT" else "journal.json"), 2097152)
        )
        truth = json.loads(read(fixture.parent / "oracle.json"))
        pointer = (
            journal["state"]["accepted"]
            if arm == "RT"
            else dict(revision=journal["revision"], content_id=journal["content_id"])
        )
        assert pointer == dict(
            revision=0, content_id="sha256:" + truth["initial_sha256"]
        )
        if arm != "RT":
            assert all(
                r.get("removed") is True and not r.get("reconciliation_required")
                for r in journal["runs"]
            )
        checked = dict(
            accepted_revision=0, no_publication=True, journal_pointer_checked=True
        )
    return dict(
        arm=arm,
        outcome=task_outcome,
        requests=result["requests"],
        visible_bytes=result["context_bytes"],
        context_sha256=result["context_sha256"],
        grading=checked,
        cleanup_certified=True,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("rt", "lt", "worker", "validator", "evidence"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    contracts = {}
    for version in (1, 2):
        flag = "--describe" if version == 1 else "--describe-v2"
        left = subprocess.check_output([str(args.rt), flag], timeout=10)
        right = subprocess.check_output([str(args.lt), flag], timeout=10)
        assert left == right and len(left) <= 131072
        contracts[version] = left
    with tempfile.TemporaryDirectory(prefix="ramenos-evaluator-controls-") as tmp:
        root = Path(tmp)
        manifest = create(
            root / "bank",
            read(args.validator),
            seed=hashlib.sha256(b"synthetic-evaluator-control-bank-v1").digest(),
            synthetic=True,
        )
        dev = [i for i in manifest["instances"] if i["partition"] == "development"]
        plan = schedule(dev)
        assert len({tuple(p["order"]) for p in plan}) == 6
        counts = [
            sum(tuple(p["order"]) == order for p in plan)
            for order in itertools.permutations(("RT", "LT", "LS"))
        ]
        assert max(counts) - min(counts) <= 1
        outputs = []
        for block in plan:
            fixture, item = select_fixture(
                root / "bank", block["instance_id"], "development"
            )
            rows = [
                run_arm(
                    arm,
                    fixture,
                    item,
                    root / (item["instance_id"] + "-" + arm),
                    dict(RT=args.rt, LT=args.lt),
                    args.worker,
                    contracts,
                    args.evidence,
                )
                for arm in block["order"]
            ]
            completed = [r for r in rows if r["outcome"] == "completed"]
            assert len({r["grading"]["accepted_bytes_sha256"] for r in completed}) <= 1
            outputs.append(
                dict(
                    instance_id=item["instance_id"],
                    error_class=item["error_class"],
                    condition=item["condition"],
                    order=block["order"],
                    arms=rows,
                )
            )
        # Force an actual contained command past the outer watchdog. The scoped
        # label reconciles observed containers, but an interrupted create RPC
        # remains uncertified and blocks a comparison instead of becoming PASS.
        fixture, item = select_fixture(
            root / "bank", dev[0]["instance_id"], "development"
        )
        forced_root = root / "forced"
        command = [
            sys.executable,
            str(Path(__file__).with_name("ls_launcher.py").resolve()),
            "--fixture",
            str(fixture),
            "--store",
            str(forced_root / "store"),
            "--candidate",
            str(forced_root / "candidate"),
            "--worker",
            str(args.worker),
            "--wall-ms",
            "10000",
        ]
        forced = Session(
            command,
            Limits(wall_ms=3000, reply_ms=3000, request_bytes=8192),
            visible=[
                ("task", item["task_text"].encode()),
                ("tools", encoded(LS_TOOLS)),
            ],
            docker_scope=True,
        )
        try:
            forced.call(dict(schema_version=1, request_id="1", command="sleep 30"))
            raise AssertionError("watchdog did not stop command")
        except SessionStopped:
            pass
        finally:
            forced.close()
        failure = forced.evidence()
        assert (
            failure["outcome"] == "session_timeout"
            and failure["container_cleanup"]["observed_empty"]
            and not failure["container_cleanup"]["certified"]
            and failure["process_cleanup"]["observed_descendants_not_running"]
        ), failure
        retain(forced, args.evidence, "retained-timeout", forced_root / "store")
        private_write(args.evidence / "retained-timeout.json", encoded(failure) + b"\n")
    files = (
        subprocess.check_output(["git", "ls-files", "-co", "--exclude-standard", "-z"])
        .decode()
        .split("\0")
    )
    source = {
        p: sha(Path(p).read_bytes())
        for p in sorted(set(files) - {""})
        if Path(p).is_file()
    }
    source["Cargo.lock"] = sha(Path("Cargo.lock").read_bytes())
    (args.evidence / "source-files.json").write_bytes(encoded(source) + b"\n")
    rows = [r for case in outputs for r in case["arms"]]
    assert len(rows) == 45 and all(
        any(r["arm"] == arm and r["outcome"] == "completed" for r in rows)
        for arm in ("RT", "LT", "LS")
    )
    report = dict(
        schema_version=1,
        claim="synthetic-bank-and-scripted-session-controls",
        source_commit=subprocess.check_output(
            ["git", "rev-parse", "HEAD"], text=True
        ).strip(),
        source_files_sha256=sha(encoded(source)),
        bank=commitments(manifest),
        bank_sha256=manifest_hash(manifest),
        binaries_sha256=dict(
            RT=binary_sha(args.rt),
            LT=binary_sha(args.lt),
            worker=binary_sha(args.worker),
        ),
        validator_sha256=sha(read(args.validator)),
        tool_contracts_sha256={str(v): sha(data) for v, data in contracts.items()},
        cases=outputs,
        attempted_arm_runs=45,
        completed_arm_runs=sum(r["outcome"] == "completed" for r in rows),
        retained_task_timeouts=sum(r["outcome"] == "task_timeout" for r in rows),
        retained_timeout=failure,
        model_trial=False,
        hidden_bank_claim=False,
        full_a2_conformance=False,
        authority_inclusion="unknown",
        token_accounting=False,
        forced_cleanup_certification=False,
    )
    (args.evidence / "report.json").write_bytes(encoded(report) + b"\n")
    print(
        f"Evaluator controls: PASS {report['completed_arm_runs']}/45 repairs; {report['retained_task_timeouts']} validator timeouts retained; six arm orders; forced timeout quarantined"
    )


if __name__ == "__main__":
    main()
