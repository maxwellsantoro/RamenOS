#!/usr/bin/env python3
"""Opt-in LS shell helper. All file IO here stays inside the agent container."""

import argparse
import base64
import json
import os
from pathlib import Path
import socket
import stat
import sys
import time

FRAME = 131072
LIMIT = 65536


def read_file(path):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    with os.fdopen(fd, "rb") as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise ValueError("regular file required")
        data = stream.read(LIMIT + 1)
        if not data or len(data) > LIMIT:
            raise ValueError("candidate byte bound")
        return data


def write_file(path, data):
    # Avoid FIFOs/symlinks and truncation before checking the opened inode.
    fd = os.open(
        path,
        os.O_WRONLY | os.O_CREAT | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC,
        0o600,
    )
    with os.fdopen(fd, "wb") as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise ValueError("regular file required")
        stream.truncate(0)
        stream.write(data)


def parse():
    parser = argparse.ArgumentParser(
        description="Scoped task transactions; no automatic retry or repair"
    )
    parser.add_argument("--request-id", required=True)
    verbs = parser.add_subparsers(dest="verb", required=True)
    for verb in [
        "grant",
        "read",
        "stage",
        "validate",
        "commit",
        "receipt",
        "state",
        "revoke",
    ]:
        p = verbs.add_parser(verb)
        if verb in ["grant", "revoke"]:
            p.add_argument("--policy-cap", required=True)
        if verb != "grant":
            p.add_argument("--task-cap", required=True)
        if verb == "grant":
            p.add_argument("--task-id", default="17")
            p.add_argument("--resource", default="resource:0000000000000001")
            p.add_argument("--rights", required=True)
            p.add_argument("--lifetime-ms", type=int, default=60000)
        if verb == "read":
            p.add_argument("--resource", required=True)
            p.add_argument("--output", type=Path)
        if verb == "stage":
            p.add_argument("--file", type=Path, required=True)
        if verb in ["validate", "commit"]:
            p.add_argument("--candidate-cap", required=True)
        if verb == "validate":
            p.add_argument("--validator-id", required=True)
        if verb == "commit":
            p.add_argument("--expected-revision", required=True)
            p.add_argument("--expected-content-id", required=True)
        if verb == "receipt":
            p.add_argument("--commit-request-id", required=True)
    return parser.parse_args()


def main():
    args = parse()
    op = {
        "grant": "request_grant",
        "read": "read_input",
        "stage": "stage_candidate",
        "validate": "validate_candidate",
        "commit": "commit_candidate",
        "receipt": "get_receipt",
        "state": "get_task_state",
        "revoke": "revoke_grant",
    }[args.verb]
    call = {"operation": op}
    for key in [
        "task_cap",
        "policy_cap",
        "task_id",
        "resource",
        "lifetime_ms",
        "candidate_cap",
        "validator_id",
        "expected_revision",
        "expected_content_id",
        "commit_request_id",
    ]:
        if hasattr(args, key):
            call[key] = getattr(args, key)
    if args.verb == "grant":
        call["rights"] = args.rights.split(",")
    if args.verb == "stage":
        call["bytes_base64"] = base64.b64encode(read_file(args.file)).decode()
    request = {"schema_version": 1, "request_id": args.request_id, "call": call}
    payload = json.dumps(request, separators=(",", ":")).encode()
    if len(payload) > FRAME:
        raise ValueError("request bound")
    with socket.socket(socket.AF_UNIX) as connection:
        # One fixed endpoint and one request. Retries belong to the caller.
        deadline = time.monotonic() + 25
        connection.settimeout(25)
        connection.connect("/task/socket")
        connection.sendall(payload + b"\n")
        data = bytearray()
        while not data.endswith(b"\n"):
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("broker deadline")
            connection.settimeout(remaining)
            chunk = connection.recv(min(4096, FRAME + 2 - len(data)))
            if not chunk:
                raise ValueError("lost reply")
            data.extend(chunk)
            if len(data) > FRAME + 1:
                raise ValueError("reply bound")
    response = json.loads(data)
    if (
        response.get("schema_version") != 1
        or response.get("request_id") != args.request_id
    ):
        raise ValueError("reply identity")
    result = response.get("result")
    if response.get("status") == "ok":
        if not isinstance(result, dict) or result.get("operation") != op:
            raise ValueError("reply operation")
        if args.verb == "read" and args.output is not None:
            data = base64.b64decode(result["bytes_base64"], validate=True)
            if not data or len(data) > LIMIT:
                raise ValueError("read byte bound")
            write_file(args.output, data)
            # Output is an agent-side path; it never becomes broker authority.
            result = dict(result)
            del result["bytes_base64"]
            result["written_bytes"] = len(data)
            response["result"] = result
    print(json.dumps(response, separators=(",", ":")))
    return 0 if response.get("status") == "ok" else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception:
        print(
            "task command failed; no automatic retry (a commit reply may be lost)",
            file=sys.stderr,
        )
        sys.exit(2)
