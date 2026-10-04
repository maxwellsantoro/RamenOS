#!/usr/bin/env python3
"""Actual all-arm fixed-universe observations, never a model evaluator."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import select
import subprocess
import tempfile
import time
from jsonschema import Draft202012Validator
from authority_client import exercise, sha
from authority_manifest import (
    UNIVERSE,
    UNIVERSE_HASH,
    build_manifest,
    compare,
    validate,
)
from ls_transactions import LinuxShellTask


def read(process):
    deadline = time.monotonic() + 10
    line = bytearray()
    while not line.endswith(b"\n"):
        remaining = deadline - time.monotonic()
        assert (
            remaining > 0 and select.select([process.stdout], [], [], remaining)[0]
        ), "typed response deadline"
        chunk = os.read(process.stdout.fileno(), min(4096, 131074 - len(line)))
        assert chunk, "typed response EOF"
        line.extend(chunk)
        assert len(line) <= 131073 and b"\n" not in line[:-1], "typed response frame"
    assert line.endswith(b"\n") and len(line) <= 131073
    return json.loads(line)


def grade(root, accepted, arm, fixture):
    # Independent evaluator inspection; no accepted-output helper is exposed
    # to any consumer. Hash the private sealed bytes and check the repair.
    if arm == "RT":
        journal = json.loads((root / "task.json").read_bytes())
        assert journal["state"]["accepted"] == {"revision": 1, "content_id": accepted}
        matches = list((root / "cas").rglob(accepted[7:] + ".blob"))
        assert len(matches) == 1, matches
        data = matches[0].read_bytes()
    else:
        journal = json.loads((root / "journal.json").read_bytes())
        assert journal["revision"] == 1 and journal["content_id"] == accepted
        data = (root / "cas" / (accepted[7:] + ".blob")).read_bytes()
    assert sha(data) == accepted
    expected = json.loads((fixture / "config.json").read_bytes())
    expected["enabled"] = json.loads((fixture / "schema.json").read_bytes())["enabled"]
    assert json.loads(data) == expected
    return dict(
        accepted=accepted,
        accepted_revision=1,
        independent_bytes_checked=True,
        journal_pointer_checked=True,
    )


def typed(binary, arm, fixture, worker, root):
    schemas = {
        version: json.loads(
            subprocess.check_output(
                [str(binary), "--describe" if version == 1 else "--describe-v2"],
                timeout=10,
            )
        )
        for version in (1, 2)
    }
    requests = {
        version: {
            t["name"]: Draft202012Validator(t["input_schema"]) for t in s["tools"]
        }
        for version, s in schemas.items()
    }
    replies = {
        version: Draft202012Validator(s["response_schema"])
        for version, s in schemas.items()
    }
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
        Draft202012Validator(schemas[1]["bootstrap_schema"]).validate(bootstrap)
        proc = Path("/proc") / str(process.pid)
        status = dict(
            line.split(":", 1)
            for line in (proc / "status").read_text().splitlines()
            if ":" in line
        )
        runtime = dict(
            kind="trusted-host-client",
            uid=int(status["Uid"].split()[0]),
            gid=int(status["Gid"].split()[0]),
            credentials={
                name: status[name].strip()
                for name in ("Uid", "Gid", "CapEff", "NoNewPrivs", "Seccomp")
            },
            client_pid=process.pid,
            namespaces={
                name: os.readlink(proc / "ns" / name) for name in ("mnt", "pid", "net")
            },
            open_descriptor_count=len(list((proc / "fd").iterdir())),
            model_interface="bounded JSON v1/v2; no host path or process operation",
            enforcement="RT host service"
            if arm == "RT"
            else "independent LT host broker and contained validator",
            host_client_isolation=False,
        )

        def call(request):
            version = request["schema_version"]
            requests[version][request["call"]["operation"]].validate(request)
            process.stdin.write(json.dumps(request).encode() + b"\n")
            process.stdin.flush()
            result = read(process)
            replies[version].validate(result)
            return result

        result = exercise(call, bootstrap)
        process.stdin.close()
        assert (
            process.wait(timeout=10) == 0
            and not process.stdout.read()
            and not process.stderr.read()
        )
        result["runtime"] = runtime
        result["grading"] = grade(root, result["accepted"], arm, fixture)
        if arm == "LT":
            journal = json.loads((root / "journal.json").read_bytes())
            result["validator_runs"] = journal["runs"]
        return result
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=10)
        for stream in (process.stdin, process.stdout, process.stderr):
            if not stream.closed:
                stream.close()


def shell(fixture, worker, root):
    private = root / "canary"
    canary = os.urandom(32)
    private.write_bytes(canary)
    client = Path(__file__).with_name("authority_client.py").read_text()
    with LinuxShellTask(fixture, root / "store", root / "candidate", worker) as session:
        script = (
            client + "\nmain_ls(" + repr(str(private)) + "," + str(os.getpid()) + ")\n"
        )
        run = session.run(["python3", "-c", script], wall_ms=20000)
        assert (
            run.evidence["removed"]
            and run.evidence["exit_code"] == 0
            and not run.stderr
        )
        result = json.loads(run.stdout)
        for name, value in result["runtime"]["namespaces"].items():
            assert value != os.readlink("/proc/self/ns/" + name)
        continuation = result.pop("continuation")
        # A fresh contained command reconnects with the original subscription.
        # Per-command socket close is not whole-session disconnect for LS.
        code = (
            client
            + "\nr=shell_call("
            + repr(
                dict(
                    schema_version=2,
                    request_id="1000",
                    call=dict(operation="poll_task", **continuation),
                )
            )
            + "); assert r['status']=='ok' and r['result']['event_types']==[]; print('ok')\n"
        )
        second = session.run(["python3", "-c", code], wall_ms=5000)
        assert (
            second.stdout == b"ok\n"
            and not second.stderr
            and second.evidence["removed"]
        )
        result["observations"].append(
            dict(
                case="ls_across_commands",
                phase="renewed",
                elapsed_ms=(time.monotonic_ns() - result["started_ns"]) // 1_000_000,
                tuple_id="broker.raw",
                outcome="allowed",
                status="ok",
                forbidden=False,
                channel="probe",
            )
        )
        result["grading"] = grade(root / "store", result["accepted"], "LS", fixture)
        result["linux_configuration"] = session.evidence()
        endpoint = session.endpoint / "socket"
    assert not endpoint.exists() and private.read_bytes() == canary
    result["runtime"].update(
        host_client_isolation=True,
        model_interface="contained shell commands, conventional taskctl v1 and raw broker v1/v2",
        enforcement="Linux namespaces/mounts/credentials/seccomp/cgroups plus independent transaction broker",
    )
    return result


def main():
    parser = argparse.ArgumentParser()
    for name in ("rt", "lt", "fixture", "worker", "evidence"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    assert platform.system() == "Linux"
    args.evidence.mkdir(parents=True, exist_ok=True)
    (args.evidence / "report.json").unlink(missing_ok=True)
    for version in ("--describe", "--describe-v2"):
        assert subprocess.check_output(
            [str(args.rt.resolve()), version]
        ) == subprocess.check_output([str(args.lt.resolve()), version])
    results = {}
    with tempfile.TemporaryDirectory(prefix="ramenos-authority-") as temp:
        for arm in ("RT", "LT", "LS"):
            root = Path(temp) / arm
            root.mkdir()
            results[arm] = (
                shell(args.fixture.resolve(), args.worker.resolve(), root)
                if arm == "LS"
                else typed(
                    (args.rt if arm == "RT" else args.lt).resolve(),
                    arm,
                    args.fixture.resolve(),
                    args.worker.resolve(),
                    root / "store",
                )
            )
    # Compare common semantic traces and common probe results, never substitute
    # desired policy for an observed response. LS extras remain separate records.
    common = results["RT"]["trace"]
    common_observations = [
        {k: v for k, v in o.items() if k != "elapsed_ms"}
        for o in results["RT"]["observations"]
    ]
    for arm in ("LT", "LS"):
        for left, right in zip(common, results[arm]["trace"], strict=True):
            assert left == right, (arm, "common semantic results differ", left, right)
        observed = [
            {k: v for k, v in o.items() if k != "elapsed_ms"}
            for o in results[arm]["observations"]
            if not o["case"].startswith("ls_")
        ]
        assert observed == common_observations, (arm, "common probe outcomes differ")
    manifests = {}
    for arm, result in results.items():
        (args.evidence / (arm.lower() + "-observations.json")).write_text(
            json.dumps(result, indent=2) + "\n"
        )
        containers = (
            result.get("validator_runs", [])
            if arm == "LT"
            else result.get("linux_configuration", {}).get("journal_shell_history", [])
            + result.get("linux_configuration", {}).get("validator_runs", [])
        )
        assert all(r.get("removed") is True for r in containers)
        provenance = dict(
            runtime=result["runtime"],
            configuration_artifact=arm.lower() + "-observations.json",
            configuration_sha256=sha(
                (args.evidence / (arm.lower() + "-observations.json")).read_bytes()
            ),
            source="trusted scripted consumer; independent private sealed-output grading",
            containers_removed=len(containers),
        )
        manifest = build_manifest(arm, result["observations"], provenance)
        validate(manifest)
        assert (
            manifest["forbidden_probes"]["attempts"] >= 15
            and manifest["forbidden_probes"]["successful"] == 0
        )
        assert manifest["unknown"] and set(manifest["exercised"]) <= set(
            manifest["maximum_observed_available"]
        )
        manifests[arm] = manifest
        (args.evidence / (arm.lower() + "-manifest.json")).write_text(
            json.dumps(manifest, indent=2) + "\n"
        )
    assert (
        "process.delegate" in manifests["LS"]["probe_exercised"]
        and "process.delegate" not in manifests["LS"]["exercised"]
    )
    source = {
        p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
        for p in subprocess.check_output(
            ["git", "ls-files", "-co", "--exclude-standard", "-z"]
        )
        .decode()
        .split("\0")
        if p and Path(p).is_file()
    }
    source["Cargo.lock"] = hashlib.sha256(Path("Cargo.lock").read_bytes()).hexdigest()
    (args.evidence / "source-files.json").write_text(
        json.dumps(source, indent=2) + "\n"
    )
    (args.evidence / "universe.json").write_text(
        json.dumps(
            dict(schema_version=1, sha256=UNIVERSE_HASH, tuples=UNIVERSE), indent=2
        )
        + "\n"
    )
    report = dict(
        schema_version=1,
        claim="fixed-universe-authority-and-negative-cases",
        environment="linux",
        source_commit=subprocess.check_output(
            ["git", "rev-parse", "HEAD"], text=True
        ).strip(),
        source_files_sha256=hashlib.sha256(
            json.dumps(source, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest(),
        universe_sha256=UNIVERSE_HASH,
        fixture_sha256={
            p.name: sha(p.read_bytes())
            for p in sorted(args.fixture.iterdir())
            if p.is_file()
        },
        worker_sha256=sha(args.worker.read_bytes()),
        binaries_sha256={
            arm: sha(p.read_bytes()) for arm, p in (("RT", args.rt), ("LT", args.lt))
        },
        common_cases_per_arm=len(common_observations),
        forbidden_probes={arm: m["forbidden_probes"] for arm, m in manifests.items()},
        comparisons={
            left + "_vs_" + right: compare(manifests[left], manifests[right])
            for left, right in (("LT", "LS"), ("RT", "LT"), ("RT", "LS"))
        },
        unknowns_preserved=True,
        full_authority_conformance=False,
        narrower_authority_claim=False,
        model_comparison=False,
        target_kernel_enforcement=False,
    )
    (args.evidence / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(
        "Authority gate passed",
        len(common_observations),
        "common cases per arm; forbidden probes",
        report["forbidden_probes"],
        "; inclusion unknown",
    )


if __name__ == "__main__":
    main()
