#!/usr/bin/env python3
"""Lint RamenOS IDL files for wire-contract integrity."""

from __future__ import annotations

import pathlib
import re
import sys


ROOT = pathlib.Path(__file__).resolve().parents[2]
IDL_DIR = ROOT / "idl"
GENERATED_DIR = ROOT / "kernel_api" / "src" / "generated"
DYNAMIC_TYPES = {"string", "bytes"}


def iter_idls() -> list[pathlib.Path]:
    return sorted(IDL_DIR.glob("**/*.toml"))


def parse_idl(path: pathlib.Path) -> tuple[int | None, dict[str, tuple[int | None, list[str]]]]:
    protocol: int | None = None
    messages: dict[str, tuple[int | None, list[str]]] = {}
    current_message: str | None = None
    collecting_fields = False
    field_chunks: list[str] = []

    for raw_line in path.read_text().splitlines():
        line = raw_line.split("#", 1)[0].strip()
        if not line:
            continue

        protocol_match = re.fullmatch(r"protocol\s*=\s*(\d+)", line)
        if protocol_match:
            protocol = int(protocol_match.group(1))
            continue

        message_match = re.fullmatch(r"\[message\.([A-Za-z0-9_-]+)\]", line)
        if message_match:
            current_message = message_match.group(1)
            messages.setdefault(current_message, (None, []))
            continue

        msg_type_match = re.fullmatch(r"msg_type\s*=\s*(\d+)", line)
        if msg_type_match and current_message is not None:
            _, fields = messages[current_message]
            messages[current_message] = (int(msg_type_match.group(1)), fields)
            continue

        if collecting_fields:
            field_chunks.append(line)
            if "]" in line:
                if current_message is not None:
                    msg_type, _ = messages[current_message]
                    messages[current_message] = (
                        msg_type,
                        re.findall(r'"([^"]+)"', " ".join(field_chunks)),
                    )
                collecting_fields = False
                field_chunks = []
            continue

        if line.startswith("fields"):
            collecting_fields = True
            field_chunks = [line]
            if "]" in line:
                if current_message is not None:
                    msg_type, _ = messages[current_message]
                    messages[current_message] = (
                        msg_type,
                        re.findall(r'"([^"]+)"', " ".join(field_chunks)),
                    )
                collecting_fields = False
                field_chunks = []

    return protocol, messages


def field_type(field: str) -> str:
    if ":" not in field:
        raise ValueError(f"field must be name:type: {field}")
    return field.split(":", 1)[1].strip()


def lint_idls() -> list[str]:
    errors: list[str] = []
    protocols: dict[int, pathlib.Path] = {}

    for path in iter_idls():
        rel = path.relative_to(ROOT)
        try:
            protocol, messages = parse_idl(path)
        except Exception as exc:
            errors.append(f"{rel}: IDL parse failed: {exc}")
            continue

        if protocol is None:
            errors.append(f"{rel}: missing required protocol")
        elif protocol == 0:
            errors.append(f"{rel}: protocol must be a non-zero integer")
        elif protocol in protocols:
            errors.append(
                f"{rel}: protocol {protocol} duplicates {protocols[protocol].relative_to(ROOT)}"
            )
        else:
            protocols[protocol] = path

        seen_msg_types: dict[int, str] = {}
        for msg_name, (msg_type, fields) in messages.items():
            if msg_type is None:
                errors.append(f"{rel}:{msg_name}: missing required msg_type")
            elif msg_type == 0:
                errors.append(f"{rel}:{msg_name}: msg_type must be non-zero")
            elif msg_type in seen_msg_types:
                errors.append(
                    f"{rel}:{msg_name}: msg_type {msg_type} duplicates {seen_msg_types[msg_type]}"
                )
            else:
                seen_msg_types[msg_type] = msg_name
            for field in fields:
                try:
                    ty = field_type(field)
                except ValueError as exc:
                    errors.append(f"{rel}:{msg_name}: {exc}")
                    continue
                if ty in DYNAMIC_TYPES:
                    errors.append(
                        f"{rel}:{msg_name}: dynamic type {ty!r} is not wire-safe"
                    )

    return errors


def lint_generated_wire_refs() -> list[str]:
    errors: list[str] = []
    if not GENERATED_DIR.exists():
        return errors

    forbidden = [
        (re.compile(r"&'static\s+str"), "&'static str"),
        (re.compile(r"&'static\s+\[u8\]"), "&'static [u8]"),
        (re.compile(r"\*\s*(const|mut)\s+"), "raw pointer"),
    ]

    for path in sorted(GENERATED_DIR.glob("*.rs")):
        text = path.read_text()
        rel = path.relative_to(ROOT)
        for pattern, label in forbidden:
            if pattern.search(text):
                errors.append(f"{rel}: generated kernel IPC contains {label}")

    return errors


def main() -> int:
    errors = lint_idls() + lint_generated_wire_refs()
    if errors:
        for err in errors:
            print(f"IDL_LINT: {err}", file=sys.stderr)
        return 1
    print("IDL_LINT: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
