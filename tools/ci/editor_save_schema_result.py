#!/usr/bin/env python3
"""Fail-closed consumer for nine frozen pure editor-save schema assertions."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import selectors
import signal
import subprocess
import tempfile
import time

GATE = "foundry-editor-save-schema-ui1-1b"
LIMIT = 1024 * 1024
REQUIRED = (
    "editor_save_identity_binding_and_bounds_are_explicit",
    "editor_text_header_has_exact_le_layout_and_ascii_consumption",
    "editor_receipt_exact_176_bytes_and_every_bound_identity",
    "editor_permit_payload_cannot_wrap_successor_or_change_binding",
    "editor_journal_canonical_digest_and_json_denials",
    "editor_journal_retains_sixteen_allocation_records_and_rejects_seventeenth",
    "editor_journal_active_reservation_counts_submitted_and_retired_permits",
    "editor_journal_selection_and_receipt_are_one_replayed_transition",
    "editor_journal_counter_identity_and_state_shape_fail_closed",
)
FROZEN = {
    "artifact_store_schema/src/editor_save.rs": "2691ded3b4c7be79f7e25c380c63367e1a5d9151811c931e49c1203745564f85",
    "artifact_store_schema/src/lib.rs": "6357098f56c5cf649f5d385c660cb798930f0a2750997830e4b742264d14b89a",
    "artifact_store_schema/tests/editor_save.rs": "c3d7d70264eba5aea1510ade9452505738efd4a4cb3c58353d91b8f4df162c63",
    "docs/DESKTOP_EDITOR_STORE_API_V0.md": "27d88ca3e1ee3487ff1354af0088e80727aae3df1ca9e74f65b3931667a99c00",
}
CARGO_TEST = ["cargo", "test", "-p", "artifact_store_schema", "--no-default-features",
              "--features", "std", "--test", "editor_save"]
COMMANDS = {
    "consumer-self-test.log": ["python3", "tools/ci/editor_save_schema_result.py", "--self-test"],
    "no-std.log": ["cargo", "check", "-p", "artifact_store_schema", "--no-default-features"],
    "std-clippy.log": ["cargo", "clippy", "-p", "artifact_store_schema", "--no-default-features",
                       "--features", "std", "--test", "editor_save", "--", "-D", "warnings"],
    "build.jsonl": CARGO_TEST + ["--no-run", "--message-format=json"],
    "cases.log": CARGO_TEST + ["--", "--list"],
    "tests.log": CARGO_TEST + ["--", "--test-threads=1"],
}
FILES = set(COMMANDS) | {name + ".execution.json" for name in COMMANDS} | {"build.log"}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def sha_file(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def text(path):
    require(path.is_file() and not path.is_symlink(), "missing/nonregular evidence: " + str(path))
    require(0 < path.stat().st_size <= LIMIT, "empty/oversized evidence: " + str(path))
    data = path.read_bytes()
    require(data.endswith(b"\n") and b"\x00" not in data, "truncated/NUL evidence: " + str(path))
    return data.decode("utf-8")


def check_sources():
    for name, expected in FROZEN.items():
        require(sha_file(name) == expected, "frozen source mismatch: " + name)


def check_list(path):
    log = text(path)
    listed = re.findall(r"^(.+): (test|benchmark)$", log, re.MULTILINE)
    require(len(listed) == 9 and {n for n, _ in listed} == set(REQUIRED)
            and all(kind == "test" for _, kind in listed), "wrong/duplicate/extra assertion inventory")
    require(re.findall(r"^(\d+) tests, (\d+) benchmarks$", log, re.MULTILINE) == [("9", "0")],
            "missing/ambiguous list summary")


def check_tests(path):
    log = text(path)
    outcomes = re.findall(r"^test (.+) \.\.\. (\S+)$", log, re.MULTILINE)
    require(len(outcomes) == 9 and {n for n, _ in outcomes} == set(REQUIRED)
            and all(state == "ok" for _, state in outcomes), "absent/extra/skipped/failed assertions")
    require(re.findall(r"^running (\d+) tests$", log, re.MULTILINE) == ["9"], "wrong run count")
    summaries = re.findall(r"^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out; finished in [0-9.]+s$", log, re.MULTILINE)
    require(summaries == [("9", "0", "0", "0", "0")], "missing/truncated/ambiguous test summary")
    return [name for name, _ in outcomes]


def run_log(path, stderr_path, command):
    require(command and path.parent.is_dir(), "missing command/evidence directory")
    require(not path.exists() and not Path(str(path) + ".execution.json").exists(), "refuse log overwrite")
    if stderr_path:
        require(stderr_path.parent == path.parent and not stderr_path.exists(), "invalid separate stderr path")
    current = sum(p.stat().st_size for p in path.parent.iterdir() if p.is_file())
    budget = LIMIT - current - 65536
    require(budget > 0, "aggregate evidence exhausted")
    start = time.monotonic()
    child = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
    selector = selectors.DefaultSelector()
    selector.register(child.stdout, selectors.EVENT_READ, "stdout")
    selector.register(child.stderr, selectors.EVENT_READ, "stderr")
    count = 0
    fault = None
    with path.open("xb") as output:
        separate = stderr_path.open("xb") if stderr_path else None
        try:
            while selector.get_map():
                if time.monotonic() - start >= 600:
                    fault = "Timeout"
                    break
                for key, _ in selector.select(.1):
                    data = os.read(key.fd, 4096)
                    if not data:
                        selector.unregister(key.fileobj)
                        continue
                    count += len(data)
                    if count > budget:
                        fault = "EvidenceExhausted"
                        break
                    target = separate if separate and key.data == "stderr" else output
                    target.write(data)
                if fault:
                    break
            if fault:
                os.killpg(child.pid, signal.SIGKILL)
            child.wait(timeout=5)
        finally:
            selector.close()
            if child.poll() is None:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait(timeout=5)
            if separate:
                separate.close()
    artifacts = {p.name: {"bytes": p.stat().st_size, "sha256": sha_file(p)}
                 for p in [path] + ([stderr_path] if stderr_path else [])}
    record = dict(schema_version=1, argv=command, exit_code=child.returncode, fault=fault,
                  actual_pid=child.pid, reaped=child.returncode is not None,
                  elapsed_seconds=time.monotonic() - start, artifacts=artifacts)
    Path(str(path) + ".execution.json").write_text(json.dumps(record, sort_keys=True) + "\n")
    require(not fault and child.returncode == 0, "actual command failed: " + str(command))


def check_commands(evidence):
    records = {}
    for name, command in COMMANDS.items():
        record = json.loads(text(evidence / (name + ".execution.json")))
        require(isinstance(record, dict) and set(record) == {"schema_version", "argv", "exit_code", "fault",
                "actual_pid", "reaped", "elapsed_seconds", "artifacts"}, "unexpected command record schema")
        require(type(record.get("schema_version")) is int and record["schema_version"] == 1
                and record.get("argv") == command and type(record.get("exit_code")) is int
                and record["exit_code"] == 0 and record.get("fault") is None
                and record.get("reaped") is True and type(record.get("actual_pid")) is int
                and record["actual_pid"] > 0 and type(record.get("elapsed_seconds")) in (int, float)
                and 0 <= record["elapsed_seconds"] <= 605,
                "wrong/failed command witness: " + name)
        expected = {name, "build.log"} if name == "build.jsonl" else {name}
        require(set(record.get("artifacts", {})) == expected, "wrong command artifact inventory")
        for item, binding in record["artifacts"].items():
            require(binding == {"bytes": (evidence / item).stat().st_size,
                                "sha256": sha_file(evidence / item)}, "command/log binding mismatch")
        records[name] = record
    return records


def result(evidence):
    check_sources()
    present = {p.name for p in evidence.iterdir()}
    require(present == FILES, "missing/extra evidence files")
    require(sum(p.stat().st_size for p in evidence.iterdir()) <= LIMIT - 32768, "aggregate logs exceed bound")
    logs = {name: text(evidence / name) for name in COMMANDS}
    text(evidence / "build.log")
    records = check_commands(evidence)
    check_list(evidence / "cases.log")
    executed = check_tests(evidence / "tests.log")
    require(logs["consumer-self-test.log"] == '{"negative_cases": 9, "self_test": "PASS"}\n', "consumer self-test absent/failed")
    for name in ("no-std.log", "std-clippy.log"):
        require(re.search(r"^\s*Finished `dev` profile .* target\(s\) in .+$", logs[name], re.MULTILINE), "compile completion absent: " + name)
    rows = [json.loads(line) for line in logs["build.jsonl"].splitlines()]
    require(rows and all(isinstance(row, dict) for row in rows), "malformed build records")
    finished = [row for row in rows if row.get("reason") == "build-finished"]
    require(len(finished) == 1 and finished[0].get("success") is True
            and rows[-1].get("reason") == "build-finished", "build failed/truncated")
    binaries = [row for row in rows if row.get("reason") == "compiler-artifact"
                and row.get("target", {}).get("name") == "editor_save"
                and row.get("target", {}).get("kind") == ["test"] and row.get("profile", {}).get("test") is True
                and row.get("target", {}).get("src_path") == str(Path("artifact_store_schema/tests/editor_save.rs").resolve())
                and "std" in row.get("features", []) and row.get("executable")]
    require(len(binaries) == 1, "missing/ambiguous std acceptance executable")
    binary = Path(binaries[0]["executable"])
    require(binary.is_file() and binary.stat().st_size > 0, "acceptance executable missing/empty")
    sources = set(FROZEN) | {"Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "artifact_store_schema/Cargo.toml",
                            "tools/ci/editor_save_schema_result.py", "tools/ci/foundry_editor_save_schema_ui1_1b.sh"}
    sources.update(str(p) for p in Path("artifact_store_schema/src").rglob("*.rs"))
    return dict(schema_version=1, gate=GATE, outcome="PASS", scope="pure-editor-save-payload-schema",
                required_cases=list(REQUIRED), executed_cases=executed, filtered_tests=0,
                frozen_source_sha256=FROZEN, source_sha256={p: sha_file(p) for p in sorted(sources)},
                test_executable_sha256=sha_file(binary),
                command_evidence=records, artifact_sha256={p.name: sha_file(p) for p in sorted(evidence.iterdir())},
                no_std_warning_count=len(re.findall(r"^warning:", logs["no-std.log"], re.MULTILINE)),
                claims={name: False for name in ("store_io", "capability_enforcement", "store_transaction_runtime",
                                                "store_b_gate_accepted", "durable_receipt", "device_backed_io",
                                                "target_kernel", "hostile_containment")})


def self_test():
    listing = "".join(name + ": test\n" for name in REQUIRED) + "\n9 tests, 0 benchmarks\n"
    testing = "running 9 tests\n" + "".join("test " + name + " ... ok\n" for name in REQUIRED)
    testing += "\ntest result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n"
    with tempfile.TemporaryDirectory(prefix="ramen-editor-schema-consumer-") as tmp:
        path = Path(tmp) / "fixture.log"
        path.write_text(listing)
        check_list(path)
        path.write_text(testing)
        check_tests(path)
        negative = [(None, check_tests), ("", check_tests), (testing.rsplit("\n", 2)[0] + "\n", check_tests),
                    (listing + "foreign: test\n", check_list),
                    (listing.replace(REQUIRED[0], REQUIRED[1]), check_list),
                    (testing.replace(" ... ok", " ... ignored", 1), check_tests),
                    (testing.replace(" ... ok", " ... FAILED", 1), check_tests),
                    (testing.replace("9 passed; 0 failed", "8 passed; 1 failed"), check_tests),
                    (testing + "test foreign ... ok\n", check_tests)]
        for data, consumer in negative:
            path.unlink(missing_ok=True)
            if data is not None:
                path.write_text(data)
            rejected = False
            try:
                consumer(path)
            except (ValueError, OSError):
                rejected = True
            require(rejected, "consumer accepted negative fixture")
    print(json.dumps({"negative_cases": len(negative), "self_test": "PASS"}, sort_keys=True))


def main():
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--check-sources", action="store_true")
    group.add_argument("--check-list", type=Path)
    group.add_argument("--self-test", action="store_true")
    group.add_argument("--evidence", type=Path)
    group.add_argument("--run-log", type=Path)
    parser.add_argument("--stderr-log", type=Path)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.check_sources:
        check_sources()
    elif args.check_list:
        check_list(args.check_list)
    elif args.self_test:
        self_test()
    elif args.run_log:
        command = args.command[1:] if args.command[:1] == ["--"] else args.command
        run_log(args.run_log, args.stderr_log, command)
    else:
        output = args.evidence / "result.json"
        require(not output.exists(), "refuse result overwrite")
        data = (json.dumps(result(args.evidence), indent=2, sort_keys=True) + "\n").encode()
        require(len(data) + sum(p.stat().st_size for p in args.evidence.iterdir()) <= LIMIT, "result exceeds aggregate1MiB")
        with output.open("xb") as stream:
            stream.write(data)


if __name__ == "__main__":
    main()
