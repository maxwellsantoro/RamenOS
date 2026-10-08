#!/usr/bin/env python3
"""Fail closed on missing/skipped UI1.0 cases and retain the tested source identity."""

import argparse
import hashlib
import json
from pathlib import Path
import platform
import re
import subprocess
import struct

CASES = (
    "approve_runs_pinned_child_with_exact_observation_grant",
    "cancel_and_single_use_confirmation_do_not_spawn_twice",
    "confirmation_requires_one_trusted_generation_bound_event",
    "actual_child_cannot_launch_restart_or_observe_foreign_instance",
    "peer_owner_endpoint_right_kind_and_session_generation_are_enforced",
    "preview_expiry_policy_and_selected_identity_changes_fail_closed",
    "unknown_rights_and_executable_identity_mismatch_never_launch",
    "replaced_executable_is_rejected_before_private_snapshot_launch",
    "config_time_and_shared_capacity_exhaustion_fail_closed",
    "wire_lengths_protocol_operation_reserved_and_tail_are_rejected",
    "raw_frame_rejects_reserved_handle_bits_outer_pad_and_payload_tail",
    "revoke_close_and_expiry_reap_child_and_retire_old_grants",
    "fault_is_visible_and_restart_requires_fresh_confirmation",
    "absolute_watchdog_reaps_stalled_child_without_dispatch_or_maintenance",
    "stalled_or_retired_session_does_not_block_unrelated_session",
    "drop_reaps_owned_child",
    "history_and_child_exchange_bounds_are_enforced",
)
REQUIRED = tuple("ui1_0_" + name for name in CASES)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def check_list(path):
    listed = re.findall(r"^(.+): (?:test|benchmark)$", Path(path).read_text(), re.MULTILINE)
    if len(listed) != len(REQUIRED) or set(listed) != set(REQUIRED):
        raise ValueError("missing/duplicate/unexpected launch assertions")



def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def frame(value):
    require(isinstance(value, str) and re.fullmatch(r"[0-9a-f]{176}", value), "invalid frame hex")
    raw = bytes.fromhex(value)
    protocol, kind, handle, length = struct.unpack_from("<IIQI", raw)
    lengths = {1: 64, 2: 48, 5: 40, 6: 48, 13: 40, 14: 48, 17: 40, 18: 64}
    require(protocol == 336 and kind in lengths and length == lengths[kind], "invalid typed frame")
    require(not any(raw[20 + length:]), "nonzero frame tail/pad")
    require(handle == 0 or (handle >> 56 in (1, 2, 3) and not (handle >> 48 & 255)
                           and handle & 0xffffffff), "invalid packed handle")
    payload = raw[20:20 + length]
    reserved_offsets = {1: 60, 2: 44, 6: 44, 14: 44}
    require(kind not in reserved_offsets or not any(payload[reserved_offsets[kind]:]), "nonzero reserved")
    return kind, handle, payload


def validate_children(root, witness_hash):
    require(root.is_dir(), "missing actual child evidence")
    grouped = {name: [] for name in REQUIRED}
    for directory in sorted(root.iterdir()):
        require(directory.is_dir() and not directory.is_symlink(), "unexpected evidence entry")
        match = re.fullmatch(r"(ui1_0_\w+)-(\d+)", directory.name)
        require(match and match[1] in grouped, "unknown child case")
        name, pid = match[1], int(match[2])
        require(pid > 0, "invalid PID")
        require({p.name for p in directory.iterdir()} <=
                {"spawn.json", "child.json", "independent-disappearance.json"}, "unexpected child artifacts")
        spawn = json.loads((directory / "spawn.json").read_text())
        child = json.loads((directory / "child.json").read_text())
        for record in (spawn, child):
            for key, ceiling in (("schema_version", 0xffffffff), ("granted_rights", 0xffffffff),
                                 ("instance_id", 0xffffffffffffffff),
                                 ("instance_generation", 0xffffffffffffffff),
                                 ("observation_handle", 0xffffffffffffffff)):
                require(type(record.get(key)) is int and 0 < record[key] <= ceiling,
                        "invalid bounded integer evidence")
            require(record.get("schema_version") == 1 and record.get("scope") == "default-off-host-fixture",
                    "invalid child schema/scope")
            require(record.get("granted_rights") == 1 and record.get("instance_id", 0) > 0
                    and record.get("instance_generation", 0) > 0, "invalid exact child grant")
        require(spawn.get("source") == "confirm-reply" and child.get("source") == "actual-owned-child-pipe",
                "invalid evidence actor")
        for key, record, ceiling, minimum in (
                ("child_pid", spawn, 0xffffffff, 1), ("actual_pid", child, 0xffffffff, 1),
                ("session_id", child, 0xffffffffffffffff, 1),
                ("session_generation", child, 0xffffffffffffffff, 1),
                ("bootstrap_reserved", child, 0xffffffff, 0),
                ("observation_count", child, 64, 0)):
            require(type(record.get(key)) is int and minimum <= record[key] <= ceiling,
                    "invalid bounded process/bootstrap/count evidence")
        require(spawn.get("child_pid") == child.get("actual_pid") == pid, "PID mismatch")
        require(spawn.get("pinned_executable_sha256") == child.get("executable_sha256") == witness_hash,
                "executed hash mismatch")
        for key in ("instance_id", "instance_generation", "observation_handle", "granted_rights"):
            require(spawn.get(key) == child.get(key), "bootstrap/approval mismatch")
        handle = child["observation_handle"]
        require(handle >> 56 == 1 and not (handle >> 48 & 255) and handle & 0xffffffff,
                "invalid observation handle")
        require(child.get("bootstrap_reserved") == 0 and child.get("session_id", 0) > 0
                and child.get("session_generation", 0) > 0, "invalid bootstrap")
        exchanges = child.get("exchanges")
        require(isinstance(exchanges, list) and len(exchanges) <= 64, "unbounded exchanges")
        observed, denials = 0, set()
        for exchange in exchanges:
            require(set(exchange) == {"request_hex", "reply_hex"}, "invalid exchange schema")
            kind, request_handle, request = frame(exchange["request_hex"])
            reply_kind, _, reply = frame(exchange["reply_hex"])
            require(kind in (1, 5, 13, 17) and reply_kind == kind + 1, "incorrect reply operation")
            require(request[:8] == reply[:8] and request_handle == handle, "request identity mismatch")
            offset = {1: 36, 5: 36, 13: 36, 17: 60}[kind]
            status = int.from_bytes(reply[offset:offset + 4], "little")
            require(status in (0, 1), "unexpected child outcome")
            if status == 1:
                denials.add(kind)
                require(not any(reply[8:offset]) and not any(reply[offset + 4:]), "denial leaks metadata")
            else:
                require(kind == 17, "child acquired privileged operation")
                values = struct.unpack_from("<5Q", request)
                require(values[1:] == (child["session_id"], child["session_generation"],
                                      child["instance_id"], child["instance_generation"]), "foreign observe allowed")
                require(int.from_bytes(reply[8:16], "little") == child["instance_id"]
                        and int.from_bytes(reply[16:24], "little") == child["instance_generation"]
                        and reply[24:56].hex() == witness_hash
                        and int.from_bytes(reply[56:60], "little") == 2, "incorrect own observation")
                observed += 1
        require(child.get("observation_count") == observed, "observation count mismatch")
        require(type(child.get("reaped")) is bool and type(child.get("exchange_limit_hit")) is bool,
                "invalid terminal flags")
        disappearance_path = directory / "independent-disappearance.json"
        disappearance = json.loads(disappearance_path.read_text()) if disappearance_path.exists() else None
        if disappearance:
            for key, ceiling in (("schema_version", 0xffffffff), ("child_pid", 0xffffffff),
                                 ("instance_id", 0xffffffffffffffff)):
                require(type(disappearance.get(key)) is int and 0 < disappearance[key] <= ceiling,
                        "invalid disappearance integer")
            expected = {"ui1_0_drop_reaps_owned_child": "drop",
                        "ui1_0_absolute_watchdog_reaps_stalled_child_without_dispatch_or_maintenance": "absolute-watchdog"}
            require(disappearance.get("schema_version") == 1
                    and disappearance.get("scope") == "default-off-host-fixture"
                    and disappearance.get("cause") == expected.get(name)
                    and disappearance.get("child_pid") == pid
                    and disappearance.get("instance_id") == child["instance_id"]
                    and disappearance.get("kill_zero_process_absent") is True, "invalid OS disappearance")
        grouped[name].append(dict(pid=pid, instance_id=child["instance_id"], observed=observed,
                                  denied_operations=sorted(denials), reaped=child["reaped"],
                                  exchange_limit_hit=child["exchange_limit_hit"],
                                  independent_disappearance=bool(disappearance)))
    require(any(row["observed"] == 1 for row in grouped[REQUIRED[0]]), "no real launch observation")
    require(any(row["observed"] == 1 and set(row["denied_operations"]) == {1, 5, 13, 17}
                for row in grouped[REQUIRED[3]]), "missing actual child denial transcripts")
    for name in (REQUIRED[13], REQUIRED[15]):
        require(grouped[name] and all(row["independent_disappearance"] for row in grouped[name]),
                "missing independent cleanup proof")
    require(all(row["reaped"] for row in grouped[REQUIRED[13]]), "watchdog did not reap")
    require(not any(row["reaped"] for row in grouped[REQUIRED[15]]), "Drop snapshot must precede cleanup")
    require(sum(row["reaped"] and not row["exchange_limit_hit"] for row in grouped[REQUIRED[16]]) == 16,
            "missing bounded terminal history")
    require(any(row["reaped"] and row["exchange_limit_hit"] and row["observed"] == 64
                for row in grouped[REQUIRED[16]]), "missing exchange-bound retirement")
    require(sum(row["reaped"] for row in grouped[REQUIRED[11]]) >= 3, "missing close/revoke/expiry reaping")
    require(any(row["reaped"] for row in grouped[REQUIRED[12]]), "missing fault reaping")
    return grouped


def result(evidence):
    check_list(evidence / "cases.log")
    outcomes = re.findall(r"^test (.+) \.\.\. (\w+)$",
                          (evidence / "tests.log").read_text(), re.MULTILINE)
    if (len(outcomes) != len(REQUIRED) or {name for name, _ in outcomes} != set(REQUIRED)
            or any(state != "ok" for _, state in outcomes)):
        raise ValueError("missing/skipped/failed launch assertions")
    summaries = re.findall(r"^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;", (evidence / "tests.log").read_text(), re.MULTILINE)
    if summaries != [(str(len(REQUIRED)), "0", "0", "0", "0")]:
        raise ValueError("unexpected test summary")
    sources = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "kernel_api/Cargo.toml", "kernel_api/src/lib.rs",
               "kernel_api/src/generated/desktop_session_v1.generated.rs",
               "idl/portals/desktop_session_v1.toml", "docs/DESKTOP_SESSION_V1.md",
               "tools/ci/desktop_launch_result.py",
               "tools/ci/foundry_desktop_host_launch_ui1_0.sh"]
    sources += [str(p) for p in sorted(Path("services/desktop").rglob("*"))
                if p.is_file() and p.suffix in (".rs", ".toml")]
    sources += [str(p) for p in sorted(Path("kernel_api/src").rglob("*.rs"))]
    sources += ["tools/ci/run_codegen.sh", "tools/ci/foundry_ci_extended.sh", "justfile"]
    sources = sorted(set(sources))
    target = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"], text=True))["target_directory"]
    witness = Path(target) / "debug/desktop-launch-witness"
    executables = [row["executable"] for line in (evidence / "build.jsonl").read_text().splitlines()
                   if (row := json.loads(line)).get("reason") == "compiler-artifact"
                   and row.get("target", {}).get("name") == "launch_lifetime"
                   and row.get("executable")]
    if len(executables) != 1:
        raise ValueError("missing/ambiguous built acceptance executable")
    children = validate_children(evidence / "children", sha(witness.read_bytes()))
    artifacts = {str(p.relative_to(evidence)): sha(p.read_bytes())
                 for p in sorted(evidence.rglob("*")) if p.is_file() and p.name != "result.json"}
    return dict(schema_version=1, gate="foundry-desktop-host-launch-ui1-0",
                outcome="PASS", scope="typed-host-launch-lifetime", host=platform.platform(),
                source_commit=subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
                dirty_diff_sha256=sha(subprocess.check_output(["git", "diff", "HEAD", "--binary"])),
                source_sha256={p: sha(Path(p).read_bytes()) for p in sources},
                witness_sha256=sha(witness.read_bytes()),
                test_executable_sha256=sha(Path(executables[0]).read_bytes()),
                artifact_sha256=artifacts, child_evidence=children, required_cases=list(REQUIRED),
                executed_cases=[name for name, _ in outcomes],
                default_runtime_exposed=False, default_witness_selectable=False,
                claims=dict(target_runtime=False, keyboard_device=False, compositor=False,
                            store_io=False, process_containment=False, physical_hardware=False,
                            model_comparison=False))


def main():
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--check-list", type=Path)
    group.add_argument("--evidence", type=Path)
    args = parser.parse_args()
    if args.check_list:
        check_list(args.check_list)
    else:
        report = result(args.evidence)
        (args.evidence / "result.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
