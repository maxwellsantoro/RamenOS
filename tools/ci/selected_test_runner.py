#!/usr/bin/env python3
"""Compile Cargo-selected harnesses once and verify actual selected outcomes.

No result reuse. Existing outer stage deadlines still apply. Each owned command
retains argv, PID, actual exit/reap and logs; package defaults remain Cargo's.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
from pathlib import Path
import signal
import stat
import subprocess
import sys
import tempfile
import time

MAX_LOG = 64 * 1024 * 1024
MAX_INPUT = 1024 * 1024 * 1024


class RunnerError(RuntimeError):
    pass


def require(condition, message):
    if not condition:
        raise RunnerError(message)


def json_rows(text):
    result = []
    for line in text.splitlines():
        require(bool(line.strip()), "empty structured result row")
        try:
            row = json.loads(line)
        except (ValueError, TypeError) as error:
            raise RunnerError("malformed structured result") from error
        require(isinstance(row, dict), "structured result must be an object")
        result.append(row)
    require(bool(result), "empty structured result")
    return result


def parse_artifacts(text, package_id, package_root, test_target=None, metadata_targets=None):
    result = []
    paths = set()
    finished = False
    for row in json_rows(text):
        require(not finished, "Cargo result after build-finished")
        reason = row.get("reason")
        require(reason in {"compiler-artifact", "compiler-message", "build-script-executed", "build-finished"}, "unknown Cargo result")
        if reason == "build-finished":
            require(row.get("success") is True, "Cargo build-finished must succeed")
            finished = True
            continue
        if reason != "compiler-artifact" or row.get("package_id") != package_id:
            continue
        if row.get("profile", {}).get("test") is not True or row.get("executable") is None:
            continue
        target = row.get("target", {})
        kinds = target.get("kind")
        require(isinstance(kinds, list) and kinds and all(isinstance(kind, str) for kind in kinds)
                and len(kinds) == len(set(kinds)), "invalid Cargo target kinds")
        explicit = test_target is not None and kinds == ["test"] and target.get("name") == test_target
        if test_target is not None and not explicit:
            continue
        require(isinstance(target.get("test"), bool), "missing Cargo default test selection")
        if metadata_targets is not None:
            matches = [candidate for candidate in metadata_targets
                       if candidate.get("name") == target.get("name") and candidate.get("kind") == kinds
                       and candidate.get("src_path") == target.get("src_path")]
            require(len(matches) == 1 and matches[0].get("test") == target["test"], "Cargo target metadata mismatch")
        # Cargo builds default non-test examples/bin helpers in the test
        # profile, but an explicit --test target can override test=false.
        if not target["test"] and not explicit:
            continue
        libraries = {"lib", "rlib", "cdylib", "dylib", "staticlib", "proc-macro"}
        if set(kinds) <= libraries:
            kind = "proc-macro" if kinds == ["proc-macro"] else "lib"
        else:
            require(len(kinds) == 1 and kinds[0] in {"bin", "test", "example"}, "unsupported Cargo test target")
            kind = kinds[0]
        source = Path(target.get("src_path", ""))
        require(source.is_absolute() and source.resolve().is_relative_to(package_root.resolve()), "foreign Cargo target source")
        executable = row["executable"]
        require(isinstance(executable, str) and Path(executable).is_absolute(), "nonabsolute Cargo executable")
        require(executable not in paths, "duplicate Cargo test executable")
        paths.add(executable)
        result.append({"name": target["name"], "kind": kind, "kinds": kinds, "src_path": str(source),
                       "executable": executable, "features": row.get("features", [])})
    require(finished and bool(result), "missing successful Cargo build or test executable")
    return result


def doctest_plan(metadata_targets):
    """Cargo's default doc-test scope is distinct from test/doc generation flags."""
    eligible = []
    libraries = {"lib", "rlib", "cdylib", "dylib", "staticlib", "proc-macro"}
    for target in metadata_targets:
        kinds = target.get("kind")
        require(isinstance(kinds, list) and kinds and all(isinstance(kind, str) for kind in kinds),
                "missing doctest target kind metadata")
        if not set(kinds) <= libraries:
            continue
        require(isinstance(target.get("doctest"), bool), "missing library doctest eligibility")
        if target["doctest"]:
            require(isinstance(target.get("name"), str) and bool(target["name"]), "missing doctest library name")
            eligible.append({"name": target["name"], "kinds": kinds})
    return {"run": bool(eligible), "eligible_libraries": eligible,
            "reason": "default_doctest_enabled_library" if eligible else "no_default_doctest_enabled_library"}


def case_name(value):
    require(isinstance(value, str) and 0 < len(value) <= 1024 and "\x00" not in value and "\n" not in value and not value.startswith("-"), "invalid test name")
    return value


def parse_discovery(text):
    rows = json_rows(text)
    require(rows[0].get("type") == "suite" and rows[0].get("event") == "discovery", "missing discovery start")
    require(rows[-1].get("type") == "suite" and rows[-1].get("event") == "completed", "missing discovery completion")
    cases = {}
    for row in rows[1:-1]:
        require(row.get("type") == "test" and row.get("event") == "discovered", "unexpected discovery event")
        name = case_name(row.get("name"))
        require(name not in cases and isinstance(row.get("ignore"), bool), "duplicate or invalid discovery case")
        cases[name] = row["ignore"]
    end = rows[-1]
    require(end.get("tests") == len(cases) and end.get("benchmarks") == 0 and end.get("total") == len(cases)
            and end.get("ignored") == sum(cases.values()), "discovery counts differ")
    return cases


def select_cases(targets, filters, required_filters):
    selected = {path: sorted(name for name in cases if not filters or any(part in name for part in filters))
                for path, cases in targets.items()}
    names = [name for cases in selected.values() for name in cases]
    require(bool(names), "zero selected tests")
    for part in [*filters, *required_filters]:
        require(bool(part) and any(part in name for name in names), f"empty intended test filter: {part}")
    for path, cases in selected.items():
        require(all(not targets[path][name] for name in cases), "selected ignored test")
    return selected


def validate_outcomes(text, expected, returncode, filtered_out=None):
    require(returncode == 0, f"actual test process failed: {returncode}")
    expected = set(expected)
    require(bool(expected), "zero-test outcome admission")
    rows = json_rows(text)
    require(rows[0].get("type") == "suite" and rows[0].get("event") == "started"
            and rows[0].get("test_count") == len(expected), "missing or mismatched suite start")
    started = set()
    completed = set()
    for row in rows[1:-1]:
        require(row.get("type") == "test", "unexpected non-test result before suite completion")
        name = case_name(row.get("name"))
        require(name in expected, "extra test outcome")
        require("skipping" not in str(row.get("stdout", "")).lower(), "skipping output cannot establish a test")
        if row.get("event") == "started":
            require(name not in started, "duplicate test start")
            started.add(name)
        else:
            require(row.get("event") == "ok" and name in started and name not in completed,
                    "failed, ignored, reordered or duplicate test outcome")
            completed.add(name)
    end = rows[-1]
    require(end.get("type") == "suite" and end.get("event") == "ok"
            and end.get("passed") == len(expected) and end.get("failed") == 0
            and end.get("ignored") == 0 and end.get("measured") == 0
            and isinstance(end.get("filtered_out"), int), "missing or incorrect suite completion")
    if filtered_out is not None:
        require(end["filtered_out"] == filtered_out, "unexpected filtered-out count")
    require(started == expected and completed == expected, "missing intended test outcomes")
    return sorted(completed)


def input_identity(path):
    path = Path(path)
    try:
        before = path.lstat()
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= MAX_INPUT, "input must be bounded regular file")
        digest = hashlib.sha256()
        with path.open("rb") as source:
            opened = os.fstat(source.fileno())
            require((before.st_dev, before.st_ino) == (opened.st_dev, opened.st_ino), "input identity changed at open")
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                digest.update(chunk)
        after = path.stat()
        require((before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns) ==
                (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns), "input changed while hashing")
        return {"path": str(path.resolve()), "sha256": digest.hexdigest(), "bytes": before.st_size}
    except OSError as error:
        raise RunnerError(f"required input unavailable: {path}") from error


def interrupted(signum, frame):
    raise InterruptedError(f"owned command interrupted by signal {signum}")


def terminal_observation(process):
    # WNOWAIT retains the actual child identity even after natural exit. Group
    # cleanup must precede the consuming parent wait, preventing PID reuse.
    require(hasattr(os, "WNOWAIT") and hasattr(os, "waitid"), "held child observation unavailable")
    observed = os.waitid(os.P_PID, process.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
    require(observed is None or observed.si_pid == process.pid, "unexpected held terminal child")
    return observed


def parse_groups(data, returncode):
    require(returncode == 0 and len(data) <= 65536, "process-group observation failed or exhausted")
    rows = data.splitlines()
    require(0 < len(rows) <= 8192, "incomplete process-group observation")
    result = []
    for row in rows:
        fields = row.split()
        require(len(fields) == 3 and fields[0].isdigit() and fields[1].isdigit()
                and 0 < len(fields[2]) <= 16 and re.fullmatch(rb"[A-Zt?][A-Za-z+<>?-]*", fields[2]), f"malformed process-group observation row: {row!r}")
        result.append({"pgid": int(fields[0]), "pid": int(fields[1]), "state": fields[2].decode("ascii")})
    return result


def group_snapshot():
    # macOS returns EPERM for a zombie-only group. Observe the kernel's
    # complete numeric group inventory while its actual parent PID is held.
    command = ["/bin/ps", "-axo", "pgid=,pid=,stat="]
    with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
        observation = subprocess.Popen(command, stdout=stdout, stderr=stderr)
        try:
            status = observation.wait(timeout=2)
        except BaseException:
            observation.terminate()
            try:
                observation.wait(timeout=1)
            except subprocess.TimeoutExpired:
                observation.kill()
                observation.wait(timeout=1)
            raise RunnerError("process-group observation did not return/reap")
        require(stderr.tell() == 0 and stdout.tell() <= 65536, "process-group observation incomplete")
        stdout.seek(0)
        return parse_groups(stdout.read(65537), status)


def reap(process, record=None):
    record = record if record is not None else {}
    cleanup_error = None
    for signum in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(process.pid, signum)
        except ProcessLookupError:
            pass
        except PermissionError:
            try:
                require(terminal_observation(process) is not None, "permission-denied active owned group")
                groups = group_snapshot()
                members = [row for row in groups if row["pgid"] == process.pid]
                held = [row for row in members if row["pid"] == process.pid and row["state"].startswith("Z")]
                others = [row for row in members if row not in held]
                record["group_observation"] = {"method": "/bin/ps -axo pgid=,pid=,stat=", "status": 0,
                                               "rows": len(groups), "held_terminal_parent": held,
                                               "held_pgid_other_members": len(others), "syscall_error": "EPERM"}
                require(not others, "permission-denied owned group still has other members")
                record["group_absent_before_reap"] = True
            except RunnerError as error:
                cleanup_error = error
                break
    try:
        status = process.wait(timeout=5)
    except subprocess.TimeoutExpired as error:
        raise RunnerError("owned command reap uncertain") from error
    if cleanup_error is not None:
        raise cleanup_error
    return status


def run_owned(argv, output, label, work_seconds=600):
    require(isinstance(work_seconds, (int, float)) and 0 < work_seconds <= 600, "invalid command work budget")
    stdout_path, stderr_path = output / f"{label}.stdout", output / f"{label}.stderr"
    record = {"argv": argv, "pid": None, "exit_code": None, "reaped": False,
              "stdout": stdout_path.name, "stderr": stderr_path.name, "group_stop_before_reap": False,
              "work_seconds": work_seconds}
    process = None
    previous = {sig: signal.signal(sig, interrupted) for sig in (signal.SIGINT, signal.SIGTERM)}
    began = time.monotonic_ns()
    deadline = began + int(work_seconds * 1_000_000_000)
    record["work_deadline_monotonic_ns"] = deadline
    try:
        with stdout_path.open("xb") as stdout, stderr_path.open("xb") as stderr:
            process = subprocess.Popen(argv, stdout=stdout, stderr=stderr, start_new_session=True)
            record["pid"] = process.pid
            while True:
                terminal = terminal_observation(process)
                require(time.monotonic_ns() < deadline, "owned command work deadline exceeded")
                if terminal is not None:
                    break
                require(stdout_path.stat().st_size + stderr_path.stat().st_size <= MAX_LOG, "owned command output exhausted")
                time.sleep(max(0, min(.05, (deadline - time.monotonic_ns()) / 1_000_000_000)))
            record["exit_code"] = reap(process, record)
            record["group_stop_before_reap"] = True
            record["reaped"] = True
            require(stdout_path.stat().st_size + stderr_path.stat().st_size <= MAX_LOG, "owned command output exhausted")
    except BaseException as error:
        record["error"] = str(error)
        if process is not None:
            if process.returncode is not None:
                # Never signal a PID whose consuming wait already completed.
                record["exit_code"] = process.returncode
                record["reaped"] = True
            else:
                try:
                    record["exit_code"] = reap(process, record)
                    record["group_stop_before_reap"] = True
                    record["reaped"] = True
                except (OSError, RunnerError) as cleanup:
                    record["cleanup_error"] = str(cleanup)
                    if process.returncode is not None:
                        record["exit_code"] = process.returncode
                        record["reaped"] = True
        raise
    finally:
        record["elapsed_ns"] = time.monotonic_ns() - began
        (output / f"{label}.command.json").write_text(json.dumps(record, sort_keys=True) + "\n")
        for sig, handler in previous.items():
            signal.signal(sig, handler)
    return record


def execute(args):
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    summary = {"schema_version": 1, "package": args.package, "status": "FAIL", "commands": [],
               "filters": args.filter, "required_filters": args.require_filter, "targets": [],
               "environment_sha256": hashlib.sha256(json.dumps(sorted(os.environ.items())).encode()).hexdigest(),
               "required_inputs": {}}
    try:
        summary["required_inputs"] = {str(path): input_identity(path) for path in args.required_input}
        metadata = run_owned(["cargo", "metadata", "--locked", "--no-deps", "--format-version=1"], output, "metadata")
        summary["commands"].append(metadata)
        require(metadata["exit_code"] == 0, "Cargo metadata failed")
        workspace = json.loads((output / "metadata.stdout").read_text())
        packages = [row for row in workspace["packages"] if row["name"] == args.package]
        require(len(packages) == 1, "package must resolve to one Cargo package")
        package = packages[0]
        summary["package_id"] = package["id"]
        base = ["cargo", "test", "--locked", "-p", args.package]
        if args.test:
            base.extend(["--test", args.test])
        compile_record = run_owned([*base, "--no-run", "--message-format=json"], output, "compile")
        summary["commands"].append(compile_record)
        require(compile_record["exit_code"] == 0, "Cargo test compilation failed")
        artifacts = parse_artifacts((output / "compile.stdout").read_text(), package["id"], Path(package["manifest_path"]).parent, args.test, package["targets"])
        discoveries = {}
        for index, artifact in enumerate(artifacts):
            identity = input_identity(Path(artifact["executable"]))
            artifact["binary_identity"] = identity
            listed = run_owned([artifact["executable"], "--list", "--format=json", "-Z", "unstable-options"], output, f"list-{index}")
            summary["commands"].append(listed)
            require(listed["exit_code"] == 0, "test discovery process failed")
            discoveries[artifact["executable"]] = parse_discovery((output / f"list-{index}.stdout").read_text())
        selected = select_cases(discoveries, args.filter, args.require_filter)
        for index, artifact in enumerate(artifacts):
            names = selected[artifact["executable"]]
            artifact["selected"] = names
            artifact["discovered"] = discoveries[artifact["executable"]]
            if names:
                command = [artifact["executable"], "--format=json", "-Z", "unstable-options", "--show-output", "--exact", *names]
                skipped = sorted(set(discoveries[artifact["executable"]]) - set(names))
                result = run_owned(command, output, f"run-{index}")
                summary["commands"].append(result)
                artifact["passed"] = validate_outcomes((output / f"run-{index}.stdout").read_text(), names, result["exit_code"], len(skipped))
            require(input_identity(Path(artifact["executable"])) == artifact["binary_identity"], "selected test executable changed")
            summary["targets"].append(artifact)
        summary["doctests"] = {"requested": args.include_doc_tests, "run": False, "reason": "not_requested"}
        if args.include_doc_tests:
            require(args.test is None, "explicit integration target has no doc-test scope")
            summary["doctests"].update(doctest_plan(package["targets"]))
            if summary["doctests"]["run"]:
                documented = run_owned(["cargo", "test", "--locked", "-p", args.package, "--doc"], output, "doc-tests")
                summary["commands"].append(documented)
                require(documented["exit_code"] == 0, "actual package doctests failed")
        for path, before in summary["required_inputs"].items():
            require(input_identity(Path(path)) == before, "required fixture input changed")
        summary["status"] = "PASS"
        print(f"SELECTED_TEST_RUNNER: PASS package={args.package} tests={sum(len(row['selected']) for row in artifacts)} output={output}")
    except BaseException as error:
        summary["error"] = str(error)
        raise
    finally:
        (output / "summary.json").write_text(json.dumps(summary, sort_keys=True, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", required=True)
    parser.add_argument("--test")
    parser.add_argument("--filter", action="append", default=[])
    parser.add_argument("--require-filter", action="append", default=[])
    parser.add_argument("--required-input", type=Path, action="append", default=[])
    parser.add_argument("--include-doc-tests", action="store_true")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        execute(args)
    except (RunnerError, OSError, ValueError, InterruptedError) as error:
        print(f"SELECTED_TEST_RUNNER: FAIL {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
