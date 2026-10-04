#!/usr/bin/env python3
"""Opt-in trusted launcher: one contained shell command per bounded JSON line."""

import argparse
import base64
import sys
from pathlib import Path
from linux_sandbox import SandboxFailure
from ls_transactions import LinuxShellTask
from lt_backend import TaskError, canonical, decimal, decode

FRAME = 8192


def send(record):
    sys.stdout.buffer.write(canonical(record) + b"\n")
    sys.stdout.buffer.flush()


def main():
    parser = argparse.ArgumentParser(
        description="Development LS launcher; no automatic repair or retry"
    )
    for name in ["fixture", "store", "candidate", "worker"]:
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--wall-ms", type=int, default=10000)
    args = parser.parse_args()
    if not 1 <= args.wall_ms <= 35000:
        raise TaskError("trusted command budget")
    with LinuxShellTask(
        args.fixture, args.store, args.candidate, args.worker
    ) as session:
        send(
            {
                "schema_version": 1,
                "interface": "scoped_shell",
                "task": session.task.bootstrap,
                "paths": {
                    "inputs": "/inputs",
                    "candidate": "/candidate",
                    "commands": "/task/taskctl",
                },
                "command_frame_bytes": FRAME,
                "command_wall_ms": args.wall_ms,
                "combined_output_bytes": 16384,
                "max_commands": 128,
            }
        )
        count = 0
        while True:
            line = sys.stdin.buffer.readline(FRAME + 2)
            if not line:
                break
            if len(line) > FRAME + 1 or not line.endswith(b"\n"):
                raise TaskError("frame bound")
            rid = None
            try:
                request = decode(line)
                if (
                    not isinstance(request, dict)
                    or set(request) != {"schema_version", "request_id", "command"}
                    or type(request["schema_version"]) is not int
                    or request["schema_version"] != 1
                ):
                    raise TaskError("invalid")
                decimal(request["request_id"], True)
                rid = request["request_id"]
                if (
                    not isinstance(request["command"], str)
                    or not request["command"]
                    or "\0" in request["command"]
                ):
                    raise TaskError("invalid")
            except (ValueError, TypeError, TaskError, RecursionError):
                send(
                    {
                        "schema_version": 1,
                        "request_id": rid,
                        "status": "invalid",
                        "exit_code": None,
                        "stdout_base64": "",
                        "stderr_base64": "",
                    }
                )
                continue
            if count >= 128:
                raise TaskError("session command limit")
            count += 1
            try:
                result = session.run(
                    ["/bin/sh", "-c", request["command"]], wall_ms=args.wall_ms
                )
                reply = {
                    "schema_version": 1,
                    "request_id": rid,
                    "status": "ok",
                    "exit_code": result.evidence["exit_code"],
                    "stdout_base64": base64.b64encode(result.stdout).decode(),
                    "stderr_base64": base64.b64encode(result.stderr).decode(),
                }
            except (SandboxFailure, TaskError) as error:
                status = (
                    "timeout" if getattr(error, "reason", None) == "timeout" else "io"
                )
                reply = {
                    "schema_version": 1,
                    "request_id": rid,
                    "status": status,
                    "exit_code": None,
                    "stdout_base64": "",
                    "stderr_base64": "",
                }
            send(reply)
            if session.transport_poison or session.task.poisoned:
                raise TaskError("session poisoned")


if __name__ == "__main__":
    try:
        main()
    except Exception:
        print(
            "LS launcher stopped; explicit receipt recovery may be required",
            file=sys.stderr,
        )
        sys.exit(1)
