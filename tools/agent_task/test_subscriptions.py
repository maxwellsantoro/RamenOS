#!/usr/bin/env python3
"""Shared v2 model lifecycle cases on genuine RT IPC and independent LT."""

import argparse
import base64
import hashlib
import json
from pathlib import Path
import select
import subprocess
import tempfile
import time
from jsonschema import Draft202012Validator


def read(process):
    assert select.select([process.stdout], [], [], 10)[0], "response deadline"
    line = process.stdout.readline(131074)
    assert line.endswith(b"\n") and len(line) <= 131073
    return json.loads(line)


def exercise(binary, fixture, worker, root, schema):
    transcripts = []
    response = Draft202012Validator(schema["response_schema"])
    requests = {
        t["name"]: Draft202012Validator(t["input_schema"]) for t in schema["tools"]
    }
    stale = None
    for restart in range(2):
        process = subprocess.Popen(
            [
                str(binary),
                "--fixture",
                str(fixture),
                "--store",
                str(root),
                "--worker",
                str(worker),
            ],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            bufsize=0,
        )
        try:
            bootstrap = read(process)
            Draft202012Validator(schema["bootstrap_schema"]).validate(bootstrap)
            counter = 0

            def call(op, expected="ok", **fields):
                nonlocal counter
                counter += 1
                request = dict(
                    schema_version=2,
                    request_id=str(counter),
                    call=dict(operation=op, **fields),
                )
                requests[op].validate(request)
                process.stdin.write(json.dumps(request).encode() + b"\n")
                process.stdin.flush()
                reply = read(process)
                response.validate(reply)
                assert reply["request_id"] == str(counter)
                assert reply["status"] in (
                    expected if isinstance(expected, tuple) else (expected,)
                ), (request, reply)
                if reply["status"] in ("denied", "expired"):
                    assert reply["result"] is None
                transcripts.append(
                    dict(
                        request=request,
                        response=reply,
                        terminal_expiry=isinstance(expected, tuple),
                    )
                )
                return reply["result"]

            def grant(rights, life=60000):
                return call(
                    "request_grant",
                    policy_cap=bootstrap["policy_cap"],
                    task_id=bootstrap["task_id"],
                    resource=bootstrap["resources"][0]["resource"],
                    rights=rights,
                    lifetime_ms=life,
                )["task_cap"]

            cap = grant(["read", "stage", "validate", "commit", "observe"])
            if restart:
                call("poll_task", "not_found", task_cap=cap, subscription_cap=stale)
                break
            read_only = grant(["read"])
            call(
                "subscribe_task",
                "denied",
                task_cap=read_only,
                event_types=["output_changed"],
            )
            observer = grant(["observe"])
            sub = call(
                "subscribe_task",
                task_cap=observer,
                event_types=["output_changed", "validation_changed"],
            )["subscription_cap"]
            filtered = call(
                "subscribe_task", task_cap=observer, event_types=["output_changed"]
            )["subscription_cap"]
            assert call("poll_task", task_cap=observer, subscription_cap=sub) == dict(
                operation="poll_task", event_types=[], state=None
            )
            call("poll_task", "denied", task_cap=cap, subscription_cap=sub)
            call("unsubscribe_task", "denied", task_cap=cap, subscription_cap=sub)
            state = call("get_task_state", task_cap=cap)["state"]
            failed = call(
                "stage_candidate", task_cap=cap,
                bytes_base64=base64.b64encode((fixture / "config.json").read_bytes()).decode(),
            )["candidate_cap"]
            assert call("validate_candidate", task_cap=cap, candidate_cap=failed,
                        validator_id=state["validator_id"])["outcome"] == "invalid"
            direct = call("get_task_state", task_cap=observer)["state"]
            event = call("poll_task", task_cap=observer, subscription_cap=sub)
            assert event["event_types"] == ["validation_changed"]
            assert direct == event["state"] or all(
                direct[k] == event["state"][k] for k in direct if k != "now_ms"
            )
            assert direct["validation"]["outcome"] == "invalid"
            assert direct["validation_current"] is True
            call("commit_candidate", "validation_failed", task_cap=cap,
                 candidate_cap=failed, expected_revision=state["revision"],
                 expected_content_id=state["content_id"])
            candidate = call(
                "stage_candidate",
                task_cap=cap,
                bytes_base64=base64.b64encode(
                    (fixture / "schema.json").read_bytes()
                ).decode(),
            )["candidate_cap"]
            assert (
                call("poll_task", task_cap=observer, subscription_cap=sub)[
                    "event_types"
                ]
                == []
            )
            for _ in range(2):
                assert (
                    call(
                        "validate_candidate",
                        task_cap=cap,
                        candidate_cap=candidate,
                        validator_id=state["validator_id"],
                    )["outcome"]
                    == "valid"
                )
            receipt = call(
                "commit_candidate",
                task_cap=cap,
                candidate_cap=candidate,
                expected_revision=state["revision"],
                expected_content_id=state["content_id"],
            )
            event = call("poll_task", task_cap=observer, subscription_cap=sub)
            assert event["event_types"] == ["output_changed", "validation_changed"]
            assert (
                event["state"]["revision"] == "1"
                and event["state"]["content_id"] == receipt["content_id"]
            )
            assert event["state"]["rights"] == ["observe"]
            assert event["state"]["validation"]["outcome"] == "valid"
            event = call("poll_task", task_cap=observer, subscription_cap=filtered)
            assert (
                event["event_types"] == ["output_changed"]
                and event["state"]["revision"] == "1"
            )
            assert (
                call("poll_task", task_cap=observer, subscription_cap=sub)["state"]
                is None
            )
            assert call(
                "unsubscribe_task", task_cap=observer, subscription_cap=filtered
            )["cancelled"]
            call(
                "unsubscribe_task",
                "not_found",
                task_cap=observer,
                subscription_cap=filtered,
            )
            # Fill the bounded registry and prove explicit cancellation frees a slot.
            caps = [sub]
            for _ in range(15):
                caps.append(
                    call(
                        "subscribe_task",
                        task_cap=observer,
                        event_types=["output_changed"],
                    )["subscription_cap"]
                )
            call(
                "subscribe_task",
                "capacity",
                task_cap=observer,
                event_types=["output_changed"],
            )
            call("unsubscribe_task", task_cap=observer, subscription_cap=caps.pop())
            replacement = call(
                "subscribe_task", task_cap=observer, event_types=["validation_changed"]
            )["subscription_cap"]
            call("unsubscribe_task", task_cap=observer, subscription_cap=replacement)
            short = grant(["observe"], 200)
            expiring = call(
                "subscribe_task", task_cap=short, event_types=["output_changed"]
            )["subscription_cap"]
            time.sleep(0.25)
            call(
                "poll_task",
                ("expired", "denied"),
                task_cap=short,
                subscription_cap=expiring,
            )
            # Queue a real validation event, then revoke before draining it.
            call(
                "validate_candidate",
                task_cap=cap,
                candidate_cap=candidate,
                validator_id=state["validator_id"],
            )
            call("revoke_grant", policy_cap=bootstrap["policy_cap"], task_cap=cap)
            call("poll_task", "denied", task_cap=observer, subscription_cap=sub)
            fresh = grant(["observe"])
            call("poll_task", "not_found", task_cap=fresh, subscription_cap=sub)
            stale = call(
                "subscribe_task", task_cap=fresh, event_types=["output_changed"]
            )["subscription_cap"]
            # Unsupported v1 subscriptions, duplicate types, unknown identity,
            # and oversized masks fail before backend dispatch.
            for mutate in range(4):
                request = dict(
                    schema_version=2,
                    request_id="999",
                    call=dict(
                        operation="subscribe_task",
                        task_cap=fresh,
                        event_types=["output_changed"],
                    ),
                )
                if mutate == 0:
                    request["schema_version"] = 1
                elif mutate == 1:
                    request["call"]["event_types"] *= 2
                elif mutate == 2:
                    request["call"]["domain_id"] = 7
                else:
                    request["call"]["event_types"] = ["unknown"]
                process.stdin.write(json.dumps(request).encode() + b"\n")
                process.stdin.flush()
                bad = read(process)
                assert bad["status"] == "invalid" and bad["result"] is None
        finally:
            process.stdin.close()
            assert process.wait(timeout=10) == 0, process.stderr.read()
            assert not process.stdout.read(), "unsolicited model event"
            process.stdout.close()
            process.stderr.close()
    return transcripts


def exercise_observation(binary, fixture, worker, root, schema, outcome):
    """Controlled trusted-worker observations, plus delivery/expiry/revocation.

    These fixtures test observation semantics; they do not certify real timeout
    or host-failure supervision, which has separate backend gates.
    """
    process = subprocess.Popen(
        [str(binary), "--fixture", str(fixture), "--store", str(root), "--worker", str(worker)],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, bufsize=0,
    )
    transcript = []
    response = Draft202012Validator(schema["response_schema"])
    requests = {t["name"]: Draft202012Validator(t["input_schema"]) for t in schema["tools"]}
    try:
        bootstrap = read(process)
        Draft202012Validator(schema["bootstrap_schema"]).validate(bootstrap)
        def call(operation, expected="ok", **fields):
            request = dict(schema_version=2, request_id=str(len(transcript) + 1),
                           call=dict(operation=operation, **fields))
            requests[operation].validate(request)
            process.stdin.write(json.dumps(request).encode() + b"\n")
            process.stdin.flush()
            reply = read(process)
            response.validate(reply)
            assert reply["status"] == expected, (request, reply)
            transcript.append(dict(request=request, response=reply))
            return reply["result"]
        cap = call("request_grant", policy_cap=bootstrap["policy_cap"],
                   task_id=bootstrap["task_id"], resource=bootstrap["resources"][0]["resource"],
                   lifetime_ms=60000, rights=["read", "stage", "validate", "commit", "observe"])["task_cap"]
        sub = call("subscribe_task", task_cap=cap, event_types=["validation_changed"])["subscription_cap"]
        initial = call("get_task_state", task_cap=cap)["state"]
        assert not initial["validation_current"]
        candidate = call("stage_candidate", task_cap=cap,
                         bytes_base64=base64.b64encode((fixture / "config.json").read_bytes()).decode())["candidate_cap"]
        validation = dict(task_cap=cap, candidate_cap=candidate, validator_id=initial["validator_id"])
        assert call("validate_candidate", **validation)["outcome"] == outcome
        direct = call("get_task_state", task_cap=cap)["state"]
        event = call("poll_task", task_cap=cap, subscription_cap=sub)
        assert event["event_types"] == ["validation_changed"]
        assert direct["validation_current"] and event["state"]["validation_current"]
        assert normalize(direct) == normalize(event["state"])
        call("commit_candidate", "validation_failed", task_cap=cap, candidate_cap=candidate,
             expected_revision=initial["revision"], expected_content_id=initial["content_id"])
        call("validate_candidate", **validation)  # Leave a pending event across evidence expiry.
        time.sleep(1.65)  # Development fixture TTL is 1500 ms; grant remains live.
        stale = call("get_task_state", task_cap=cap)["state"]
        event = call("poll_task", task_cap=cap, subscription_cap=sub)
        assert not stale["validation_current"] and not event["state"]["validation_current"]
        assert normalize(stale) == normalize(event["state"])
        call("validate_candidate", **validation)  # Revoke before this queued event is consumed.
        call("revoke_grant", policy_cap=bootstrap["policy_cap"], task_cap=cap)
        call("get_task_state", "denied", task_cap=cap)
        call("poll_task", "denied", task_cap=cap, subscription_cap=sub)
        assert transcript[-1]["response"]["result"] is None
        assert transcript[-2]["response"]["result"] is None
        return transcript
    finally:
        process.stdin.close()
        try:
            assert process.wait(timeout=10) == 0
        finally:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)
            process.stdout.close()
            process.stderr.close()


def normalize(value, aliases=None):
    if aliases is None:
        aliases = {}
    if isinstance(value, dict):
        return {
            k: normalize(v, aliases)
            for k, v in value.items()
            if k
            not in {
                "expires_at_ms",
                "now_ms",
                "valid_until_ms",
                "wall_elapsed_ms",
                "guest_elapsed_ms",
            }
        }
    if isinstance(value, list):
        return [normalize(v, aliases) for v in value]
    if isinstance(value, str) and value.startswith("cap:"):
        return aliases.setdefault(value, "cap#" + str(len(aliases)))
    return value


def main():
    parser = argparse.ArgumentParser()
    for name in ("rt", "lt", "fixture", "worker", "evidence"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    contracts = [
        subprocess.check_output([str(b.resolve()), "--describe-v2"], timeout=10).rstrip(
            b"\n"
        )
        for b in (args.rt, args.lt)
    ]
    assert contracts[0] == contracts[1]
    schema = json.loads(contracts[0])
    assert schema["schema_version"] == 2 and len(schema["tools"]) == 11
    assert subprocess.check_output([str(args.rt.resolve()), "--describe"]).rstrip(
        b"\n"
    ) == subprocess.check_output([str(args.lt.resolve()), "--describe"]).rstrip(b"\n")
    observations = {}
    controlled_workers = {}
    with tempfile.TemporaryDirectory(prefix="ramenos-subscriptions-") as temp:
        transcripts = [
            exercise(
                b.resolve(),
                args.fixture.resolve(),
                args.worker.resolve(),
                Path(temp) / name,
                schema,
            )
            for name, b in (("rt", args.rt), ("lt", args.lt))
        ]
        for name, source_outcome, truncated, observed in (
            ("invalid", "invalid", False, "invalid"),
            ("timeout", "timeout", False, "timeout"),
            ("host_failure", "host_failure", False, "host_failure"),
            ("truncation", "valid", True, "host_failure"),
        ):
            result = dict(schema_version=1, outcome=source_outcome, diagnostics=[],
                          truncated=truncated, guest_elapsed_ms=0)
            worker = Path(temp) / (name + "-worker")
            body = "#!/bin/sh\ncat >/dev/null\nprintf '%s' '" + json.dumps(result) + "'\n"
            worker.write_text(body)
            worker.chmod(0o700)
            controlled_workers[name] = dict(source=body, sha256=hashlib.sha256(worker.read_bytes()).hexdigest())
            rows = [exercise_observation(binary.resolve(), args.fixture.resolve(), worker,
                        Path(temp) / (arm + "-" + name), schema, observed)
                    for arm, binary in (("RT", args.rt), ("LT", args.lt))]
            assert normalize(rows[0]) == normalize(rows[1]), (name, rows)
            observations[name] = dict(RT=rows[0], LT=rows[1])
    a, b = [normalize(t) for t in transcripts]
    # Native registry reclamation may turn an expired cap into denied. Both
    # are terminal and fully redacted; this is the only status allowance.
    for left, right in zip(a, b, strict=True):
        if (
            left["terminal_expiry"]
            and right["terminal_expiry"]
            and {left["response"]["status"], right["response"]["status"]}
            <= {
                "denied",
                "expired",
            }
        ):
            left["response"]["status"] = right["response"]["status"] = (
                "terminal_authority"
            )
        assert left == right, (left, right)
    args.evidence.mkdir(parents=True, exist_ok=True)
    for name, transcript in zip(("rt", "lt"), transcripts):
        (args.evidence / (name + "-transcript.json")).write_text(
            json.dumps(transcript, indent=2) + "\n"
        )
    (args.evidence / "failed-observations.json").write_text(json.dumps(observations, indent=2) + "\n")
    paths = (
        subprocess.check_output(["git", "ls-files", "-co", "--exclude-standard", "-z"])
        .decode()
        .split("\0")
    )
    legacy = subprocess.check_output([str(args.rt.resolve()), "--describe"]).rstrip(
        b"\n"
    )
    assert (
        hashlib.sha256(legacy).hexdigest()
        == "24af8e0fe29809ee8a5b849f4fda1af1b3d5e2430437e96d12a140bd04498290"
    ), "v1 contract changed"
    source = {
        p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
        for p in sorted(set(paths))
        if p and Path(p).is_file()
    }
    source["Cargo.lock"] = hashlib.sha256(Path("Cargo.lock").read_bytes()).hexdigest()
    (args.evidence / "source-files.json").write_text(
        json.dumps(source, indent=2) + "\n"
    )
    report = dict(
        schema_version=1,
        environment="linux",
        claim="scripted-shared-subscription-lifecycle",
        source_commit=subprocess.check_output(
            ["git", "rev-parse", "HEAD"], text=True
        ).strip(),
        source_files_sha256=hashlib.sha256(
            json.dumps(source, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest(),
        tool_contract_sha256=hashlib.sha256(contracts[0]).hexdigest(),
        legacy_contract_sha256=hashlib.sha256(legacy).hexdigest(),
        binaries_sha256={
            name: hashlib.sha256(path.read_bytes()).hexdigest()
            for name, path in (
                ("rt", args.rt),
                ("lt", args.lt),
                ("worker", args.worker),
            )
        },
        fixture_sha256={
            path.name: hashlib.sha256(path.read_bytes()).hexdigest()
            for path in sorted(args.fixture.iterdir())
            if path.is_file()
        },
        operations_per_arm=len(transcripts[0]),
        coalescing=True,
        failed_observation_cases=list(observations),
        failed_observation_requests_per_case=len(next(iter(observations.values()))["RT"]),
        controlled_workers=controlled_workers,
        failed_observations_sha256=hashlib.sha256((args.evidence / "failed-observations.json").read_bytes()).hexdigest(),
        cancellation=True,
        capacity=16,
        revoke_before_poll=True,
        expiry_redacted=True,
        disconnect_restart=True,
        unsolicited_events=0,
        full_authority_conformance=False,
        model_comparison=False,
        target_kernel_enforcement=False,
    )
    (args.evidence / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(
        "Shared subscription lifecycle passed",
        report["operations_per_arm"],
        "operations per arm",
    )


if __name__ == "__main__":
    main()
