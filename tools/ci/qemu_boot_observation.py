#!/usr/bin/env python3
"""Observe one fresh QEMU boot within the existing 30-second marker budget.

Observations describe serial text and actual process ownership; they establish no
hardware graduation, device-backed IO, or result reuse.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import stat
import subprocess
import sys
import time
from selected_test_runner import RunnerError, reap as held_reap, terminal_observation

MAX_FILE = 128 * 1024 * 1024
MAX_SERIAL = 8 * 1024 * 1024
PHASE_MARKERS = {
    "firmware-load-text": "BdsDxe: loading",
    "firmware-entry-text": "BdsDxe: starting",
    "kernel-entry-text": "RAMEN OS S0 boot",
    "init-profile-text": "hil_evidence: init_profile=",
}


class ObservationError(RuntimeError):
    pass


def require(condition, message):
    if not condition:
        raise ObservationError(message)


def identity(path):
    canonical = Path(path).resolve(strict=True)
    metadata = canonical.stat()
    require(stat.S_ISREG(metadata.st_mode) and 0 < metadata.st_size <= MAX_FILE, "firmware/image input must be bounded regular file")
    digest = hashlib.sha256()
    with canonical.open("rb") as source:
        opened = os.fstat(source.fileno())
        require((metadata.st_dev, metadata.st_ino) == (opened.st_dev, opened.st_ino), "input changed at open")
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    after = canonical.stat()
    require((metadata.st_dev, metadata.st_ino, metadata.st_size, metadata.st_mtime_ns) ==
            (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns), "input changed during observation")
    return {"path": str(canonical), "bytes": metadata.st_size, "sha256": digest.hexdigest()}


def prepare_vars(template, destination):
    template = Path(template)
    destination = Path(destination)
    try:
        before = identity(template)
        require(not destination.exists() and not destination.is_symlink(), "mutable vars destination already exists")
        with template.open("rb") as source, destination.open("xb") as output:
            while chunk := source.read(1024 * 1024):
                output.write(chunk)
            output.flush()
            os.fsync(output.fileno())
        require(identity(template) == before, "vars template changed while copying")
        copied = identity(destination)
        require((copied["bytes"], copied["sha256"]) == (before["bytes"], before["sha256"]), "vars copy differs from template")
        return copied
    except (OSError, ValueError) as error:
        raise ObservationError("fresh vars preparation failed") from error


def interrupted(signum, frame):
    raise InterruptedError(f"boot interrupted by signal {signum}")


def stop_and_reap(process, record):
    terminal = terminal_observation(process)
    record["terminal_before_cleanup"] = terminal is not None
    status = held_reap(process, record)
    record["group_stop_before_reap"] = True
    return status, terminal is None


def serial_bytes(path):
    if not path.exists():
        return b""
    metadata = path.lstat()
    require(stat.S_ISREG(metadata.st_mode) and metadata.st_size <= MAX_SERIAL, "serial output exhausted or nonregular")
    with path.open("rb") as source:
        output = source.read(MAX_SERIAL + 1)
    require(len(output) <= MAX_SERIAL, "serial output exhausted")
    return output


def observe_boot(command, run_dir, serial, markers, timeout, immutable_inputs, template, variables):
    run_dir, serial = Path(run_dir), Path(serial)
    require(run_dir.is_dir() and not run_dir.is_symlink(), "run directory must be fresh regular directory")
    require(not serial.exists() and not serial.is_symlink(), "serial log already exists")
    require(0 < timeout <= 30 and markers and len(markers) == len(set(markers)), "invalid original marker deadline or roster")
    inputs = dict(immutable_inputs)
    if template is not None:
        inputs["vars_template"] = Path(template)
    before = {name: identity(path) for name, path in inputs.items()}
    vars_before = None
    if variables is not None:
        variables = Path(variables)
        require(template is not None and variables.parent.resolve().is_relative_to(run_dir.resolve()), "vars must belong to this run")
        require(not variables.is_symlink(), "mutable vars cannot be a symlink")
        vars_before = identity(variables)
        require(vars_before["path"] != before["vars_template"]["path"] and
                (vars_before["bytes"], vars_before["sha256"]) ==
                (before["vars_template"]["bytes"], before["vars_template"]["sha256"]), "vars must be a fresh template copy")
    stdout = run_dir / "qemu.stdout"
    stderr = run_dir / "qemu.stderr"
    record_path = run_dir / "boot-observation.json"
    require(not record_path.exists(), "boot observation already exists")
    result = {"schema_version": 1, "scope": "fresh-qemu-serial-observation", "argv": command,
              "launches": 0, "pid": None, "exit_code": None, "reaped": False,
              "success": False, "failure": None, "deadline_seconds": timeout,
              "serial_log": str(serial), "inputs_before": before, "inputs_after": None,
              "vars_before": vars_before, "vars_after": None, "markers": {marker: None for marker in markers},
              "phases": {label: None for label in PHASE_MARKERS}, "stop_requested": False, "group_stop_before_reap": False, "terminal_before_cleanup": False}
    process = None
    previous = {sig: signal.signal(sig, interrupted) for sig in (signal.SIGINT, signal.SIGTERM)}
    began = time.monotonic_ns()
    result["start_monotonic_ns"] = began
    result["deadline_monotonic_ns"] = began + int(timeout * 1_000_000_000)
    try:
        with stdout.open("xb") as output, stderr.open("xb") as errors:
            process = subprocess.Popen(command, stdout=output, stderr=errors, start_new_session=True)
            result["pid"] = process.pid
            result["launches"] = 1
            result["spawn_observed_monotonic_ns"] = time.monotonic_ns()
            while True:
                now = time.monotonic_ns()
                if now >= result["deadline_monotonic_ns"]:
                    result["failure"] = "marker-timeout"
                    break
                text = serial_bytes(serial)
                now = time.monotonic_ns()
                if now >= result["deadline_monotonic_ns"]:
                    result["failure"] = "marker-timeout"
                    break
                for marker in markers:
                    if result["markers"][marker] is None and marker.encode() in text:
                        result["markers"][marker] = now
                for label, marker in PHASE_MARKERS.items():
                    if result["phases"][label] is None and marker.encode() in text:
                        result["phases"][label] = now
                natural = terminal_observation(process)
                if natural is not None and (natural.si_code != os.CLD_EXITED or natural.si_status != 0):
                    result["failure"] = "unexpected-process-exit"
                    break
                if all(observed is not None for observed in result["markers"].values()):
                    break
                if natural is not None:
                    result["failure"] = "process-exit-before-markers"
                    break
                require(stdout.stat().st_size + stderr.stat().st_size <= MAX_SERIAL, "QEMU diagnostic output exhausted")
                time.sleep(min(.02, (result["deadline_monotonic_ns"] - now) / 1_000_000_000))
            result["exit_code"], result["stop_requested"] = stop_and_reap(process, result)
            result["reaped"] = True
            if result["exit_code"] != 0 and (not result["stop_requested"] or result["exit_code"] not in (-signal.SIGTERM, -signal.SIGKILL)):
                result["failure"] = result["failure"] or "unexpected-process-exit"
        result["inputs_after"] = {name: identity(path) for name, path in inputs.items()}
        if result["inputs_after"] != before:
            result["failure"] = "immutable-input-changed"
        if variables is not None:
            result["vars_after"] = identity(variables)
        result["serial_bytes"] = len(serial_bytes(serial))
        result["serial_sha256"] = hashlib.sha256(serial_bytes(serial)).hexdigest()
        result["success"] = result["failure"] is None and result["reaped"] and all(
            observed is not None and observed < result["deadline_monotonic_ns"] for observed in result["markers"].values())
    except BaseException as error:
        result["failure"] = str(error)
        if process is not None:
            if process.returncode is not None:
                result["exit_code"] = process.returncode
                result["reaped"] = True
            else:
                try:
                    result["exit_code"], result["stop_requested"] = stop_and_reap(process, result)
                    result["reaped"] = True
                except (OSError, ObservationError, RunnerError) as cleanup:
                    result["cleanup_error"] = str(cleanup)
                    if process.returncode is not None:
                        result["exit_code"] = process.returncode
                        result["reaped"] = True
        raise
    finally:
        result["finished_monotonic_ns"] = time.monotonic_ns()
        record_path.write_text(json.dumps(result, sort_keys=True, indent=2) + "\n")
        for sig, handler in previous.items():
            signal.signal(sig, handler)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="action", required=True)
    copied = commands.add_parser("prepare-vars")
    copied.add_argument("--template", type=Path, required=True)
    copied.add_argument("--destination", type=Path, required=True)
    run = commands.add_parser("run")
    run.add_argument("--run-dir", type=Path, required=True)
    run.add_argument("--serial", type=Path, required=True)
    run.add_argument("--marker", action="append", required=True)
    run.add_argument("--timeout", type=float, default=30)
    run.add_argument("--code", type=Path, required=True)
    run.add_argument("--kernel", type=Path, required=True)
    run.add_argument("--init", type=Path, required=True)
    run.add_argument("--vars-template", type=Path)
    run.add_argument("--vars", type=Path)
    run.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    try:
        if args.action == "prepare-vars":
            prepare_vars(args.template, args.destination)
            print(args.destination)
            return 0
        command = args.command[1:] if args.command[:1] == ["--"] else args.command
        require(bool(command), "missing actual QEMU command")
        result = observe_boot(command, args.run_dir, args.serial, args.marker, args.timeout,
                              {"code": args.code, "kernel": args.kernel, "init": args.init}, args.vars_template, args.vars)
        print(f"QEMU_BOOT_OBSERVATION: {'PASS' if result['success'] else 'FAIL'} observation={args.run_dir / 'boot-observation.json'}")
        return 0 if result["success"] else 1
    except (ObservationError, RunnerError, OSError, ValueError, InterruptedError) as error:
        print(f"QEMU_BOOT_OBSERVATION: FAIL {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
