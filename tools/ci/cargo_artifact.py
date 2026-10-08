#!/usr/bin/env python3
"""Select one actual non-test binary from a completed Cargo JSON build stream.

Gate callers provide a literal target name. This validates trusted host build
records; it is not artifact attestation or permission to execute arbitrary input.
"""

import argparse
import json
import os
from pathlib import Path
import sys


def reject(message):
    raise ValueError(message)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            reject("duplicate JSON key")
        result[key] = value
    return result


def select_executable(json_path: Path, target_name: str) -> str:
    if not isinstance(target_name, str) or not target_name:
        reject("missing literal binary target name")
    executables = []
    finished = False
    with open(json_path, encoding="utf-8") as stream:
        for line in stream:
            record = json.loads(line, object_pairs_hook=unique_object, parse_constant=reject)
            if not isinstance(record, dict) or not isinstance(record.get("reason"), str):
                reject("malformed Cargo message")
            if finished:
                reject("message after build-finished")
            if record["reason"] == "build-finished":
                if record.get("success") is not True:
                    reject("unsuccessful build-finished")
                finished = True
            elif record["reason"] == "compiler-artifact":
                target = record.get("target")
                if not isinstance(target, dict):
                    reject("malformed compiler-artifact target")
                name, kinds = target.get("name"), target.get("kind")
                if not isinstance(name, str) or not isinstance(kinds, list):
                    reject("malformed compiler-artifact identity")
                if not kinds or any(not isinstance(kind, str) for kind in kinds):
                    reject("malformed compiler-artifact kinds")
                # A package may also emit a same-name library; it is not a bin.
                if name != target_name or "bin" not in kinds:
                    continue
                if kinds != ["bin"]:
                    reject("ambiguous selected target kind")
                profile = record.get("profile")
                if not isinstance(profile, dict) or profile.get("test") is not False:
                    reject("selected artifact is not a non-test binary")
                executable = record.get("executable")
                if not isinstance(executable, str) or not executable:
                    reject("missing selected executable")
                if any(ord(char) < 32 or ord(char) == 127 for char in executable):
                    reject("control character in executable path")
                path = Path(executable)
                if not path.is_absolute() or not path.is_file() or not os.access(path, os.X_OK):
                    reject("selected executable is not an absolute executable file")
                executables.append(executable)
    if not finished or len(executables) != 1:
        reject("expected one selected binary and a completed build")
    return executables[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("build_json", type=Path)
    parser.add_argument("target_name")
    args = parser.parse_args()
    try:
        executable = select_executable(args.build_json, args.target_name)
    except (OSError, ValueError) as error:
        print(f"Cargo build artifact rejected ({args.build_json}): {error}", file=sys.stderr)
        return 1
    print(executable)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
