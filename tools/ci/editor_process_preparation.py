#!/usr/bin/env python3
"""Source-bound genuine setup and missing-process-API preparation, never runtime acceptance."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import stat
import sys
import tempfile

import native_save_task_sources as sources
import selected_test_runner as runner

ROOT = Path(__file__).resolve().parents[2]
TYPES = {
    "ChildBudget", "OwnedEditorChild", "OwnedEditorChildExitProof", "ProcessRetireReason",
    "ProcessLaunchRecovery", "ProcessWaitOutcome", "ProcessEditorEnrollment",
}
METHODS = {
    "prepare_process_editor", "registered_editor_process_executable", "spawn_process_editor",
    "pump_process_editor", "join_process_editor", "observe_process_child",
    "observe_process_editor", "take_process_save_intent", "request_process_editor_frame",
    "retire_process_editor",
}
ALLOWED_FILES = {
    "apps/artifact_editor/tests/process_task.rs",
    "apps/artifact_editor/tests/process_support/child_owner.rs",
}
SETUP_CASES = {
    "cleanup_owners::tests::process_preparation_cleanup_retry_preserves_deadline",
    "cleanup_owners::tests::process_preparation_consuming_unwind_retains_parent_root",
    "cleanup_owners::tests::process_preparation_joined_cleanup_releases_parent_root",
    "process_preparation_constructor_panic_joins_before_root_cleanup",
    "process_preparation_genuine_read_and_owned_reopen",
    "process_preparation_save_pause_unwind_retains_original_writer",
    "failure_control::process_preparation_init_validation_failure_retains_issuer",
    "failure_control::process_preparation_init_create_root_failure_retains_core",
    "failure_control::process_preparation_reopen_failure_panic_preserves_owner_then_recovers",
    "failure_control::process_preparation_reopen_exhaustion_retry_retains_original_fence",
    "failure_control::process_preparation_pre_reopen_probe_panic_retains_fence_then_recovers",
    "failure_control::process_preparation_consuming_reopen_unwind_quarantines_without_retry",
    "reopen_drop_control::process_preparation_failed_reopen_drop_retains_root",
}
POLICY_CASES = {case for case in SETUP_CASES if case.startswith("cleanup_owners::")} | {
    "failure_control::process_preparation_consuming_reopen_unwind_quarantines_without_retry",
}
AUTHORED_CASES = {
    "process_p0_child_budget_has_closed_bounds",
    "process_p0_genuine_enrollment_fences_legacy_alias_and_repeated_read",
    "process_p0_genuine_child_edits_raster_and_single_use_ctrl_s_capture",
    "process_p0_consuming_join_returns_live_owner_without_waiting",
}
EXTRAS = {
    "tools/ci/editor_process_preparation.py",
    "tools/ci/test_editor_process_preparation.py",
    "tools/ci/selected_test_runner.py",
    "docs/contracts/editor-process-v0.json",
    "docs/DESKTOP_EDITOR_PROCESS_API_V0.md",
    "docs/plans/editor-process-v0.md",
}


class PreparationError(RuntimeError):
    pass


def require(value, reason):
    if not value:
        raise PreparationError(reason)


def primary_locations(message, repo):
    spans = message.get("spans")
    require(isinstance(spans, list), "missing compiler spans")
    primary = [span for span in spans if isinstance(span, dict) and span.get("is_primary") is True]
    require(bool(primary), "missing primary compiler location")
    result = []
    for span in primary:
        name = span.get("file_name")
        require(isinstance(name, str), "missing compiler source name")
        path = Path(name)
        if path.is_absolute():
            require(path.is_relative_to(repo), "compiler source outside repository")
            name = path.relative_to(repo).as_posix()
        require(name in ALLOWED_FILES, "unexpected compiler source: " + name)
        result.append(name)
    return sorted(set(result))


def missing_symbols(code, message):
    if code == "E0432":
        require(message.startswith(("unresolved import ", "unresolved imports ")), "unexpected import diagnostic")
        imports = re.findall(r"`([^`]+)`", message)
        require(bool(imports) and all(name in {"desktop_service::editor_dev::" + symbol for symbol in TYPES}
                                     for name in imports), "unexpected missing import provider or symbol")
        symbols = [name.rsplit("::", 1)[-1] for name in imports]
        return set(symbols), set()
    if code == "E0433":
        match = re.fullmatch(r"failed to resolve: could not find `([^`]+)` in `desktop`", message)
        require(match is not None, "unexpected resolution diagnostic")
        symbol = match.group(1)
    elif code == "E0425":
        match = re.fullmatch(r"cannot find type `([^`]+)` in module `desktop`", message)
        require(match is not None, "unexpected unresolved-name diagnostic")
        symbol = match.group(1)
    elif code == "E0599":
        match = re.fullmatch(
            r"no method named `([^`]+)` found for struct `desktop_service::editor_dev::HostDesktop` in the current scope",
            message)
        require(match is not None and match.group(1) in METHODS, "unexpected missing method")
        return set(), {match.group(1)}
    else:
        raise PreparationError("unrelated compiler error: " + str(code))
    require(symbol in TYPES, "unexpected missing type")
    return {symbol}, set()


def classify_missing_api(stdout, exit_code, repo):
    """Admit only the current explicit frozen-API RED, retaining inference gaps."""
    require(exit_code == 101, "missing-API compile must actually exit 101")
    repo = Path(repo).absolute()
    try:
        rows = runner.json_rows(stdout)
    except runner.RunnerError as error:
        raise PreparationError(str(error)) from error
    finished = False
    missing_types, missing_methods = set(), set()
    errors, cascades, failure_notes = [], [], []
    for row in rows:
        require(not finished, "Cargo output after build-finished")
        reason = row.get("reason")
        require(reason in {"compiler-artifact", "compiler-message", "build-script-executed", "build-finished"}, "unexpected Cargo row")
        if reason == "build-finished":
            require(row.get("success") is False, "compiler success is not missing-API RED")
            finished = True
            continue
        if reason != "compiler-message":
            continue
        message = row.get("message")
        require(isinstance(message, dict), "missing compiler diagnostic")
        if message.get("level") not in {"error", "failure-note"}:
            continue
        if message.get("level") == "failure-note":
            note = message.get("message")
            require(message.get("code") is None and message.get("spans") == []
                    and isinstance(note, str) and len(note) <= 1024,
                    "unexpected compiler failure note metadata")
            require(note not in failure_notes, "duplicate compiler failure note")
            failure_notes.append(note)
            continue
        code = message.get("code")
        require(isinstance(code, dict) and isinstance(code.get("code"), str), "unclassified compiler error")
        code = code["code"]
        text = message.get("message")
        require(isinstance(text, str) and len(text) <= 16384, "invalid compiler error text")
        locations = primary_locations(message, repo)
        record = {"code": code, "message": text, "primary_sources": locations}
        if code == "E0282":
            record["status"] = "retained-unproved"
            cascades.append(record)
        else:
            types, methods = missing_symbols(code, text)
            missing_types.update(types)
            missing_methods.update(methods)
            errors.append(record)
    require(finished, "missing Cargo build-finished")
    require(missing_types == TYPES, "missing required frozen API type diagnostics")
    require(missing_methods == METHODS, "missing required frozen API method diagnostics")
    codes = {row["code"] for row in [*errors, *cascades]}
    note_kinds = set()
    for note in failure_notes:
        summary = re.fullmatch(r"Some errors have detailed explanations: (E\d{4}(?:, E\d{4})*)\.", note)
        explain = re.fullmatch(r"For more information about an error, try `rustc --explain (E\d{4})`\.", note)
        if summary is not None:
            mentioned = summary.group(1).split(", ")
            require(mentioned == sorted(codes), "compiler summary differs from actual diagnostic codes")
            kind = "summary"
        elif explain is not None:
            require(explain.group(1) in codes, "compiler explain note names an unobserved error")
            kind = "explain"
        else:
            raise PreparationError("unexpected compiler failure note")
        require(kind not in note_kinds, "duplicate compiler failure note kind")
        note_kinds.add(kind)
    return {"status": "EXPECTED_MISSING_API_RED", "actual_exit_code": exit_code,
            "missing_types": sorted(missing_types), "missing_methods": sorted(missing_methods),
            "missing_api_error_count": len(errors), "missing_api_errors": errors,
            "inference_cascade_count": len(cascades), "inference_cascades": cascades,
            "compiler_failure_notes": failure_notes,
            "runtime_executed": False, "all30_complete": False}


def snapshot(repo):
    fd = os.open(repo, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        names = sorted(set(sources.source_names(fd)) | EXTRAS)
        rows = []
        total = 0
        for name in names:
            raw = sources.read(fd, name)
            total += len(raw)
            require(total <= 64 * 1024 * 1024, "preparation source closure exceeds 64MiB")
            rows.append({"path": name, "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()})
        require(sorted(set(sources.source_names(fd)) | EXTRAS) == names, "source closure changed during capture")
        for row in rows:
            require(hashlib.sha256(sources.read(fd, row["path"])).hexdigest() == row["sha256"],
                    "source changed during capture: " + row["path"])
        return rows
    finally:
        os.close(fd)


def write_json(path, value):
    with path.open("x") as output:
        json.dump(value, output, sort_keys=True, indent=2)
        output.write("\n")


def validate_setup(summary):
    require(summary.get("status") == "PASS" and summary.get("package") == "artifact_editor",
            "genuine setup selection failed")
    targets = summary.get("targets")
    require(isinstance(targets, list) and len(targets) == 1, "setup must select exactly one harness")
    target = targets[0]
    require(target.get("name") == "process_setup_control" and target.get("kind") == "test",
            "unexpected setup harness")
    require(target.get("selected") == sorted(SETUP_CASES) and target.get("passed") == sorted(SETUP_CASES),
            "setup must actually pass the exact thirteen preparation cases")
    commands = summary.get("commands")
    require(isinstance(commands, list) and bool(commands), "missing setup command observations")
    for command in commands:
        require(command.get("exit_code") == 0 and command.get("reaped") is True
                and command.get("group_stop_before_reap") is True and "error" not in command,
                "uncertain or failed setup process")
    return sorted(SETUP_CASES)


def authored_cases(repo):
    raw = (repo / "apps/artifact_editor/tests/process_task.rs").read_text()
    names = set(re.findall(r"#\[test\]\s*fn\s+(\w+)\s*\(", raw))
    require(names == AUTHORED_CASES, "current authored process preparation inventory changed")
    return sorted(names)


def fresh_output(requested):
    if requested is not None:
        output = requested.absolute()
        output.mkdir(mode=0o700, parents=True, exist_ok=False)
    else:
        parent = ROOT / "out/editor-process-preparation"
        parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        require(not parent.is_symlink() and parent.is_dir(), "invalid canonical preparation output")
        output = Path(tempfile.mkdtemp(prefix="run.", dir=parent))
    info = output.lstat()
    require(stat.S_ISDIR(info.st_mode) and stat.S_IMODE(info.st_mode) == 0o700,
            "fresh preparation output must have mode0700")
    return output


def execute(args):
    output = fresh_output(args.output)
    evidence = output / "recording"
    evidence.mkdir(mode=0o700)
    nonce = secrets.token_hex(16)
    result = {"schema_version": 1, "status": "FAIL", "scope": "genuine-setup-and-missing-api-preparation",
              "runtime": False, "all30_complete": False, "setup_passed_count": 0,
              "authored_not_executed_count": len(AUTHORED_CASES),
              "setup_scope": {"genuine_in_process_controls": len(SETUP_CASES - POLICY_CASES),
                              "parent_ownership_policy_controls": len(POLICY_CASES)},
              "intentional_quarantines": {"initialization": 2, "reopen_drop": 1,
                                          "test_consumer_unwind": 1},
              "initialization_close_proved": False,
              "commands": [], "run_nonce": nonce,
              "recording": str(evidence), "source_registry": "dynamic-preparation-closure-not-runtime-admission"}
    previous_cwd = Path.cwd()
    # Recorders reserve directories exclusively in one shared evidence root.
    # Serialize this harness explicitly rather than relying on the caller's env.
    env_names = {"RAMEN_NATIVE_TASK_EVIDENCE": str(evidence), "RAMEN_NATIVE_TASK_RUN_NONCE": nonce,
                 "RUST_TEST_THREADS": "1"}
    previous_env = {key: os.environ.get(key) for key in env_names}
    try:
        os.chdir(ROOT)
        before = snapshot(ROOT)
        write_json(output / "source-before.json", before)
        result["source_manifest"] = "source-before.json"
        result["source_manifest_sha256"] = hashlib.sha256((output / "source-before.json").read_bytes()).hexdigest()
        result["authored_not_executed"] = authored_cases(ROOT)
        os.environ.update(env_names)
        setup = runner.run_owned([
            sys.executable, str(ROOT / "tools/ci/selected_test_runner.py"),
            "--package", "artifact_editor", "--test", "process_setup_control",
            "--features", "host_editor_process_setup_v0_dev", "--filter", "process_preparation_",
            "--output", str(output / "setup"),
        ], output, "setup-selection", work_seconds=600)
        result["commands"].append(setup)
        require(setup["exit_code"] == 0 and setup["reaped"] is True
                and setup["group_stop_before_reap"] is True, "setup selection did not exit and reap successfully")
        result["setup_passed"] = validate_setup(json.loads((output / "setup/summary.json").read_text()))
        result["setup_passed_count"] = len(result["setup_passed"])
        clippy = runner.run_owned([
            "cargo", "clippy", "--locked", "-p", "artifact_editor", "--test", "process_setup_control",
            "--features", "host_editor_process_setup_v0_dev", "--", "-D", "warnings",
        ], output, "setup-clippy", work_seconds=600)
        result["commands"].append(clippy)
        require(clippy["exit_code"] == 0 and clippy["reaped"] is True
                and clippy["group_stop_before_reap"] is True, "setup Clippy failed or cleanup uncertain")
        compile_result = runner.run_owned([
            "cargo", "test", "--locked", "-p", "artifact_editor", "--test", "process_task",
            "--features", "host_editor_process_v0_dev", "--no-run", "--message-format=json",
        ], output, "missing-api", work_seconds=600)
        result["commands"].append(compile_result)
        require(compile_result["reaped"] is True and compile_result["group_stop_before_reap"] is True,
                "missing-API compiler exit/reap uncertain")
        after = snapshot(ROOT)
        write_json(output / "source-after.json", after)
        require(before == after, "source inputs changed during preparation; results unqualified")
        result["missing_api"] = classify_missing_api((output / "missing-api.stdout").read_text(),
                                                      compile_result["exit_code"], ROOT)
        result["status"] = "PASS_PREPARATION_ONLY"
        print(f"EDITOR_PROCESS_PREPARATION: PASS_PREPARATION_ONLY setup={len(SETUP_CASES)} "
              f"authored_not_executed={len(AUTHORED_CASES)} "
              f"all30_complete=false runtime=false output={output}")
    except BaseException as error:
        result["error"] = str(error)
        raise
    finally:
        for key, value in previous_env.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value
        os.chdir(previous_cwd)
        write_json(output / "result.json", result)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, help="fresh owned destination; defaults beneath out/editor-process-preparation")
    args = parser.parse_args()
    try:
        execute(args)
    except (PreparationError, runner.RunnerError, OSError, ValueError, InterruptedError) as error:
        print(f"EDITOR_PROCESS_PREPARATION: FAIL {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
