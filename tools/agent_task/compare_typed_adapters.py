#!/usr/bin/env python3
"""Named development-fixture point cases. Does not establish all-arm conformance."""

import argparse
import base64
import hashlib
import json
from pathlib import Path
import select
import subprocess
import tempfile
from jsonschema import Draft202012Validator

parser = argparse.ArgumentParser()
for name in ["rt", "lt", "fixture", "worker", "evidence"]:
    parser.add_argument("--" + name, type=Path, required=True)
args = parser.parse_args()
contracts = [
    subprocess.check_output([str(b.resolve()), "--describe"], timeout=10).rstrip(b"\n")
    for b in [args.rt, args.lt]
]
assert contracts[0] == contracts[1], "tool descriptions differ"
schema = json.loads(contracts[0])
responses = Draft202012Validator(schema["response_schema"])
bootstraps = Draft202012Validator(schema["bootstrap_schema"])
requests = {t["name"]: Draft202012Validator(t["input_schema"]) for t in schema["tools"]}


def read(p):
    assert select.select([p.stdout], [], [], 10)[0], "response timeout"
    line = p.stdout.readline(131074)
    assert line.endswith(b"\n") and len(line) <= 131073
    return json.loads(line)


def exercise(binary, store):
    p = subprocess.Popen(
        [
            str(binary.resolve()),
            "--fixture",
            str(args.fixture.resolve()),
            "--store",
            str(store),
            "--worker",
            str(args.worker.resolve()),
        ],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        bufsize=0,
    )
    transcript = []
    try:
        b = read(p)
        bootstraps.validate(b)
        transcript.append({"bootstrap": b})

        def call(n, operation, expected="ok", **kw):
            request = dict(
                schema_version=1,
                request_id=str(n),
                call=dict(operation=operation, **kw),
            )
            requests[operation].validate(request)
            p.stdin.write(json.dumps(request).encode() + b"\n")
            p.stdin.flush()
            r = read(p)
            responses.validate(r)
            assert r["status"] == expected, (operation, r)
            transcript.append({"request": request, "response": r})
            return r["result"]

        grant = dict(
            policy_cap=b["policy_cap"],
            task_id=b["task_id"],
            resource=b["resources"][0]["resource"],
            lifetime_ms=60000,
        )
        cap = call(
            1,
            "request_grant",
            rights=["read", "stage", "validate", "commit", "observe"],
            **grant,
        )["task_cap"]
        state = call(2, "get_task_state", task_cap=cap)["state"]
        inputs = [
            call(3 + i, "read_input", task_cap=cap, resource=r["resource"])
            for i, r in enumerate(b["resources"])
        ]
        for r in inputs:
            assert (
                "sha256:"
                + hashlib.sha256(
                    base64.b64decode(r["bytes_base64"], validate=True)
                ).hexdigest()
                == r["content_id"]
            )
        config = json.loads(base64.b64decode(inputs[0]["bytes_base64"]))
        target = json.loads(base64.b64decode(inputs[1]["bytes_base64"]))
        config["enabled"] = target["enabled"]
        candidate = json.dumps(config, sort_keys=True, separators=(",", ":")).encode()
        staged = call(
            6,
            "stage_candidate",
            task_cap=cap,
            bytes_base64=base64.b64encode(candidate).decode(),
        )
        commit = dict(
            task_cap=cap,
            candidate_cap=staged["candidate_cap"],
            expected_revision=state["revision"],
            expected_content_id=state["content_id"],
        )
        call(7, "commit_candidate", "validation_failed", **commit)
        call(
            8,
            "validate_candidate",
            "denied",
            task_cap=cap,
            candidate_cap=staged["candidate_cap"],
            validator_id="sha256:" + "0" * 64,
        )
        validated = call(
            9,
            "validate_candidate",
            task_cap=cap,
            candidate_cap=staged["candidate_cap"],
            validator_id=state["validator_id"],
        )
        assert validated["outcome"] == "valid"
        receipt = call(10, "commit_candidate", **commit)
        assert receipt["revision"] == "1"
        assert call(10, "commit_candidate", **commit) == receipt
        call(
            10,
            "commit_candidate",
            "request_reuse",
            **dict(commit, expected_revision="1"),
        )
        call(11, "get_receipt", task_cap=cap, commit_request_id="10")
        accepted = call(19, "get_task_state", task_cap=cap)["state"]
        assert (
            accepted["revision"] == "1"
            and accepted["content_id"] == receipt["content_id"]
            and accepted["validation"]["outcome"] == "valid"
        )
        readcap = call(12, "request_grant", rights=["read"], **grant)["task_cap"]
        call(13, "stage_candidate", "denied", task_cap=readcap, bytes_base64="e30=")
        call(14, "get_receipt", "denied", task_cap=readcap, commit_request_id="10")
        call(
            15,
            "read_input",
            "denied",
            task_cap=cap,
            resource="resource:00000000000003e7",
        )
        call(
            16,
            "read_input",
            "denied",
            task_cap=b["policy_cap"],
            resource=b["resources"][0]["resource"],
        )
        call(17, "revoke_grant", policy_cap=b["policy_cap"], task_cap=cap)
        call(18, "get_task_state", "denied", task_cap=cap)
        p.stdin.close()
        assert p.wait(timeout=10) == 0
        assert not p.stderr.read(16385)
        return transcript
    finally:
        if p.poll() is None:
            p.kill()
            p.wait(timeout=10)
        for pipe in [p.stdin, p.stdout, p.stderr]:
            if not pipe.closed:
                pipe.close()


# Only declared backend-generated handles and clock/duration offsets may differ.
# IDs, resource names, rights, status/outcome, payloads, hashes, revisions and
# diagnostics are compared without normalization.
TIME_FIELDS = {
    "expires_at_ms",
    "now_ms",
    "valid_until_ms",
    "wall_elapsed_ms",
    "guest_elapsed_ms",
}


def normalized(transcript):
    caps = {}

    def visit(v, key=None):
        if isinstance(v, dict):
            return {k: visit(x, k) for k, x in v.items() if k not in TIME_FIELDS}
        if isinstance(v, list):
            return [visit(x) for x in v]
        if isinstance(v, str) and v.startswith("cap:"):
            if v not in caps:
                caps[v] = "opaque:" + str(len(caps))
            return caps[v]
        return v

    return visit(transcript)


with tempfile.TemporaryDirectory(prefix="ramenos-typed-conformance-") as temporary:
    rt = exercise(args.rt, Path(temporary) / "rt")
    lt = exercise(args.lt, Path(temporary) / "lt")
left = normalized(rt)
right = normalized(lt)
for i, (a, b) in enumerate(zip(left, right)):
    assert a == b, ("named point-case response differs", i, a, b)
assert len(left) == len(right)
args.evidence.mkdir(parents=True, exist_ok=True)
transcripts = {"RT": rt, "LT": lt}
(args.evidence / "point-cases.json").write_text(
    json.dumps(transcripts, indent=2) + "\n"
)
report = json.loads((args.evidence / "report.json").read_bytes())
report.update(
    {
        "named_point_cases_checked": True,
        "point_case_requests_per_arm": len(rt) - 1,
        "tool_contracts_identical": True,
        "point_cases_sha256": hashlib.sha256(
            (args.evidence / "point-cases.json").read_bytes()
        ).hexdigest(),
        "normalized_fields": sorted(TIME_FIELDS)
        + ["opaque capability identity (consistent first-use aliases)"],
        "rt_binary_sha256": hashlib.sha256(args.rt.read_bytes()).hexdigest(),
        "lt_binary_sha256": hashlib.sha256(args.lt.read_bytes()).hexdigest(),
    }
)
(args.evidence / "report.json").write_text(json.dumps(report, indent=2) + "\n")
print(
    f"RT/LT: identical descriptions and {len(rt) - 1} named point-case responses after declared normalization"
)
