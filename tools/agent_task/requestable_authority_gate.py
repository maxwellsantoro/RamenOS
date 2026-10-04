#!/usr/bin/env python3
"""All-arm issued-right projections, single-right effects and lifecycle points."""

import argparse
import base64
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

from jsonschema import Draft202012Validator
from evaluator_session import Session
from evaluator_controls_gate import private_write, binary_sha
from lifecycle_ledger import encoded
from ls_transactions import LinuxShellTask
from requestable_authority import consume, matrix_summary, CATALOG_HASH, RIGHT_TUPLES
from authority_manifest import UNIVERSE, UNIVERSE_HASH

LS_LIFETIME_PROBE = """
import sys
fd = None
direct = []
def extra(phase):
    global fd
    if phase == 'before_expiry':
        fd = os.open('/inputs/config.json', os.O_RDONLY)
        return
    mounted = Path('/inputs/config.json').read_bytes()
    os.lseek(fd, 0, os.SEEK_SET)
    child = subprocess.run([sys.executable, '-c',
        'import os,sys;sys.stdout.buffer.write(os.read('+str(fd)+',65536))'],
        pass_fds=(fd,), capture_output=True, timeout=3)
    assert child.returncode == 0 and not child.stderr and child.stdout == mounted
    direct.append(dict(phase=phase, mounted_read_sha256=sha(mounted),
                       descendant_read_sha256=sha(child.stdout)))
"""


def digest(data):
    return hashlib.sha256(data).hexdigest()


def grade(store, arm, fixture, kind):
    path = store / ("task.json" if arm == "RT" else "journal.json")
    journal = json.loads(path.read_bytes())
    pointer = (
        journal["state"]["accepted"]
        if arm == "RT"
        else {k: journal[k] for k in ("revision", "content_id")}
    )
    expected = 1 if kind == "witness" else 0
    assert pointer["revision"] == expected and len(journal["receipts"]) == expected
    file = fixture / ("schema.json" if expected else "config.json")
    assert pointer["content_id"] == "sha256:" + digest(file.read_bytes())
    matches = list((store / "cas").rglob(pointer["content_id"][7:] + ".blob"))
    assert len(matches) == 1 and matches[0].read_bytes() == file.read_bytes()
    if arm != "RT":
        assert all(
            row.get("removed") is True and row.get("created") is True
            for row in journal["runs"]
        ), journal["runs"]
    return journal


def schemas(binary):
    return {
        v: json.loads(
            subprocess.check_output(
                [str(binary), "--describe" if v == 1 else "--describe-v2"], timeout=10
            )
        )
        for v in (1, 2)
    }


def validators(descriptions):
    return {
        v: (
            {t["name"]: Draft202012Validator(t["input_schema"]) for t in d["tools"]},
            Draft202012Validator(d["response_schema"]),
        )
        for v, d in descriptions.items()
    }


def check_trace(result, checks, fixture):
    assert len(result["trace"]) <= 64
    for n, row in enumerate(result["trace"], 1):
        request, reply = row["request"], row["response"]
        assert request["request_id"] == str(n) and reply["request_id"] == str(n)
        v = request["schema_version"]
        assert reply["schema_version"] == v
        checks[v][0][request["call"]["operation"]].validate(request)
        checks[v][1].validate(reply)
        if reply["status"] == "ok" and request["call"]["operation"] == "get_task_state":
            state = reply["result"]["state"]
            assert state["policy_id"] == "sha256:" + digest(
                (fixture / "policy.json").read_bytes()
            )
            assert state["schema_id"] == "sha256:" + digest(
                (fixture / "schema.json").read_bytes()
            )
            assert state["validator_id"] == "sha256:" + digest(
                (fixture / "validator.wasm").read_bytes()
            )
        if reply["status"] == "ok" and request["call"]["operation"] == "read_input":
            name = {
                "resource:0000000000000001": "config.json",
                "resource:0000000000000064": "schema.json",
                "resource:0000000000000065": "notes.txt",
            }[request["call"]["resource"]]
            data = base64.b64decode(reply["result"]["bytes_base64"], validate=True)
            assert data == (fixture / name).read_bytes()
            assert reply["result"]["content_id"] == "sha256:" + digest(data)
        if (
            reply["status"] == "ok"
            and request["call"]["operation"] == "stage_candidate"
        ):
            data = base64.b64decode(request["call"]["bytes_base64"], validate=True)
            assert reply["result"]["content_id"] == "sha256:" + digest(data)
    assert (
        result["whole_authority_relation"] == "unknown"
        and result["continuous_envelope_certified"] is False
    )


def run_case(args, arm, fixture, root, kind, masks, policy, descriptions):
    store = root / "store"
    candidate = root / "candidate"
    if arm == "LS":
        client = Path(__file__).with_name("authority_client.py").read_text()
        consumer = Path(__file__).with_name("requestable_authority.py").read_text()
        program = (
            client
            + "\n"
            + consumer
            + "\n"
            + LS_LIFETIME_PROBE
            + (
                "bootstrap=json.loads(Path('/task/bootstrap.json').read_bytes())\n"
                f"result=consume(shell_call,bootstrap,{kind!r},{masks!r},{policy},extra=extra)\n"
                "result['direct_lifetime_observations']=direct\n"
                "Path('/candidate/gate-result.json').write_text(json.dumps(result))\nprint('done')\n"
            )
        )
        with LinuxShellTask(fixture, store, candidate, args.worker) as session:
            run = session.run(["python3", "-c", program], wall_ms=35000)
            assert (
                run.stdout == b"done\n" and not run.stderr and run.evidence["removed"]
            )
            data = (candidate / "gate-result.json").read_bytes()
            assert len(data) <= 2097152
            result = json.loads(data)
            runtime = session.evidence()
            local_writes = []
            for row in result["trace"]:
                request, reply = row["request"], row["response"]
                if (
                    request["call"]["operation"] == "stage_candidate"
                    and reply["status"] != "ok"
                ):
                    data = (
                        candidate / ("staged-" + request["request_id"])
                    ).read_bytes()
                    assert data == base64.b64decode(
                        request["call"]["bytes_base64"], validate=True
                    )
                    local_writes.append(
                        dict(
                            case=row["case"],
                            content_sha256=digest(data),
                            bytes=len(data),
                        )
                    )
            result["local_file_writes_before_broker_denial"] = local_writes
            if kind == "lifetime":
                assert [r["phase"] for r in result["direct_lifetime_observations"]] == [
                    "after_expiry",
                    "after_revocation",
                ]
                expected = "sha256:" + digest((fixture / "config.json").read_bytes())
                assert all(
                    r["mounted_read_sha256"] == r["descendant_read_sha256"] == expected
                    for r in result["direct_lifetime_observations"]
                )
        check_trace(result, validators(descriptions), fixture)
    else:
        binary = args.rt if arm == "RT" else args.lt
        with Session(
            [
                str(binary),
                "--fixture",
                str(fixture),
                "--store",
                str(store),
                "--worker",
                str(args.worker),
            ],
            visible=[
                ("task", b"Scripted finite declared-interface authority probes"),
                ("tools", encoded(descriptions)),
            ],
            docker_scope=True,
            lifecycle_parent=args.evidence / "lifecycles",
        ) as session:
            checks = validators(descriptions)

            def call(request):
                checks[request["schema_version"]][0][
                    request["call"]["operation"]
                ].validate(request)
                reply = session.call(request)
                checks[request["schema_version"]][1].validate(reply)
                return reply

            result = consume(call, session.bootstrap, kind, masks, policy)
            session.finish(
                "Finite probes complete; ambient and continuous authority remain unknown"
            )
        runtime = session.evidence()
        assert (
            runtime["outcome"] == "completed"
            and runtime["container_cleanup"]["certified"]
        ), runtime
        check_trace(result, checks, fixture)
    journal = grade(store, arm, fixture, kind)
    return result, runtime, journal


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("rt", "lt", "worker", "fixture", "evidence"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    descriptions = schemas(args.rt)
    assert descriptions == schemas(args.lt)
    assert {t for tuples in RIGHT_TUPLES.values() for t in tuples} <= set(UNIVERSE)
    reports = {}
    artifacts = {}
    with tempfile.TemporaryDirectory(prefix="ramenos-requestable-") as temporary:
        temp = Path(temporary)
        narrow = temp / "narrow-fixture"
        shutil.copytree(args.fixture, narrow)
        policy_json = json.loads((narrow / "policy.json").read_bytes())
        policy_json["allowed_rights"] = 17
        (narrow / "policy.json").write_bytes(encoded(policy_json))
        private_write(args.evidence / "attenuated-policy.json", encoded(policy_json))
        for arm in ("RT", "LT", "LS"):
            rows = {31: [], 17: []}
            cases = []
            specs = [
                ("matrix", list(range(start, min(start + 4, 32))), 31)
                for start in range(1, 32, 4)
            ]
            specs += [
                ("matrix", list(range(1, 32)), 17),
                ("witness", [], 31),
                ("lifetime", [], 31),
            ]
            for n, (kind, masks, policy) in enumerate(specs):
                root = temp / (arm + "-" + str(n))
                root.mkdir()
                fixture = args.fixture if policy == 31 else narrow
                result, runtime, journal = run_case(
                    args, arm, fixture, root, kind, masks, policy, descriptions
                )
                rows[policy].extend(result["rows"])
                name = arm.lower() + "-" + str(n)
                data = encoded(dict(result=result, runtime=runtime, journal=journal))
                private_write(args.evidence / (name + ".json"), data)
                artifacts[name + ".json"] = digest(data)
                cases.append(
                    dict(
                        name=name,
                        kind=kind,
                        policy=policy,
                        requests=len(result["trace"]),
                    )
                )
            reports[arm] = dict(
                full=matrix_summary(rows[31], 31),
                attenuated=matrix_summary(rows[17], 17),
                cases=cases,
            )
        assert reports["RT"]["full"] == reports["LT"]["full"] == reports["LS"]["full"]
        assert (
            reports["RT"]["attenuated"]
            == reports["LT"]["attenuated"]
            == reports["LS"]["attenuated"]
        )
    files = set(
        subprocess.check_output(["git", "ls-files", "-co", "--exclude-standard", "-z"])
        .decode()
        .split("\0")
    ) - {""}
    files.add("Cargo.lock")
    source = {
        p: digest(Path(p).read_bytes()) for p in sorted(files) if Path(p).is_file()
    }
    private_write(args.evidence / "source-files.json", encoded(source))
    report = dict(
        schema_version=1,
        source_commit=subprocess.check_output(
            ["git", "rev-parse", "HEAD"], text=True
        ).strip(),
        source_files_sha256=digest(encoded(source)),
        catalog_sha256=CATALOG_HASH,
        authority_universe_sha256=UNIVERSE_HASH,
        catalog=RIGHT_TUPLES,
        arms=reports,
        artifacts_sha256=artifacts,
        binaries_sha256=dict(
            RT=binary_sha(args.rt),
            LT=binary_sha(args.lt),
            worker=binary_sha(args.worker),
        ),
        fixture_sha256={
            p.name: digest(p.read_bytes())
            for p in args.fixture.iterdir()
            if p.is_file()
        },
        attenuated_policy_sha256=digest(encoded(policy_json)),
        claim="finite-issued-rights-and-lifecycle-points",
        declared_interface_projection_relation="equal",
        whole_authority_relation="unknown",
        continuous_envelope_certified=False,
        full_a2_conformance=False,
        model_trial=False,
        target_kernel_enforcement=False,
    )
    private_write(args.evidence / "report.json", encoded(report))
    print(
        "Requestable authority: PASS 31 subsets x 2 policies x 3 arms; single-right effects and expiry/renewal/revocation points; whole authority unknown"
    )


if __name__ == "__main__":
    main()
