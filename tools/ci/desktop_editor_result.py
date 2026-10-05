#!/usr/bin/env python3
"""Fail closed on the UI1.1a inventory and retained actual host observations."""

import argparse
import hashlib
import json
from pathlib import Path
import platform
import re
import struct
import subprocess
import tomllib

REQUIRED = tuple("ui1_1_" + name for name in (
    "keyboard_edits_ascii_and_renders_owned_frame",
    "ascii_bounds_preserve_prior_draft",
    "focus_routes_reserved_keys_to_trusted_chrome",
    "focus_epoch_reset_overflow_and_detach",
    "surface_freeze_alias_quota_and_chrome_clip",
    "wire_schema_descriptor_and_identity_fail_closed",
    "editor_grants_are_exact_and_preview_single_use",
    "volatile_save_and_reopen_are_explicitly_labeled",
    "revoke_paused_key_frame_and_save",
    "fault_restart_discards_unsaved_draft_and_old_grants",
    "focus_compositor_restart_retires_old_epochs",
    "stalled_session_leaves_other_session_usable",
    "clock_capacity_and_counter_limits_fail_closed",
))
IDL = ("idl/harness/input_v1.toml", "idl/services/desktop_focus_v1.toml",
       "idl/services/desktop_surface_v1.toml",
       "idl/portals/desktop_editor_session_v1.toml",
       "idl/portals/desktop_artifact_v1.toml")
FONT = "50ab6522c495f06a69590ccb6958458be15e02a2ca4c6b1024b052e826d21c64"
REGISTRIES = (2, 1, 1, 1, 1, 1, 5, 2, 4, 1, 2, 2, 11)


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate JSON field")
        result[key] = value
    return result


def read_json(path):
    return json.loads(path.read_text(), object_pairs_hook=unique_object,
                      parse_constant=lambda _: (_ for _ in ()).throw(ValueError("nonfinite JSON")))


def integer(value, maximum=(1 << 64) - 1):
    return type(value) is int and 0 <= value <= maximum


def check_list(path):
    names = re.findall(r"^(.+): (?:test|benchmark)$", path.read_text(), re.MULTILINE)
    require(len(names) == 13 and set(names) == set(REQUIRED), "missing/extra/duplicate editor cases")


def layouts():
    result = {}
    for path in IDL:
        data = tomllib.loads(Path(path).read_text())
        for name, message in data["message"].items():
            fields = {}
            offset = 0
            for field in message["fields"]:
                key, kind = field.split(":")
                size = {"u32": 4, "u64": 8, "bytes32": 32}[kind]
                fields[key] = (offset, size)
                offset += size
            result[data["protocol"], message["msg_type"]] = (name, offset, fields)
    require(len(result) == 43, "wire inventory changed without gate review")
    return result


def frame(value, schema):
    require(isinstance(value, str) and re.fullmatch(r"[0-9a-f]{176}", value), "invalid frame hex")
    raw = bytes.fromhex(value)
    protocol, kind, handle, length = struct.unpack_from("<IIQI", raw)
    layout = schema.get((protocol, kind))
    canonical = bool(layout and length == layout[1] and not any(raw[20 + length:]))
    canonical = canonical and (handle == 0 or (
        handle >> 56 in (1, 2, 3) and not (handle >> 48 & 255)
        and handle >> 32 & 0xffff and handle & 0xffffffff))
    if layout:
        for name, (offset, size) in layout[2].items():
            if name == "reserved" and any(raw[20 + offset:20 + offset + size]):
                canonical = False
    return raw, layout, bool(canonical)


def label_present(pixels, label):
    font = Path("services/desktop/tests/fixtures/ascii8x16_v0.bin").read_bytes()
    rows = []
    for y in range(16):
        rows.append(b"".join(b"\0\0\0\xff" if font[(c - 32) * 16 + y] & (128 >> x)
                             else bytes((224, 224, 224, 255)) for c in label for x in range(8)))
    width = len(rows[0])
    return any(all(pixels[(y + dy) * 2560 + x * 4:(y + dy) * 2560 + x * 4 + width] == row
                   for dy, row in enumerate(rows))
               for y in (0, 8, 16, 24, 32, 528, 536, 544, 552)
               for x in range(0, 641 - len(label) * 8, 8))


def semantic_observations(case_number, index, data, schema):
    values = data["observations"]
    require(len({v["name"] for v in values}) == len(values), "duplicate semantic observation")
    observations = {v["name"]: v["value"] for v in values}

    def matches(name, fields):
        value = observations.get(name)
        require(isinstance(value, dict), f"missing semantic witness {name}")
        require(all(k in value and type(value[k]) is type(want) and value[k] == want
                    for k, want in fields.items()), f"failed semantic witness {name}")
        return value

    def positive(value):
        return integer(value) and value > 0

    def barriers(prefix, permit):
        entered = matches(prefix + "_entered", {"entered": True, "settled": False, "permit_issued": permit})
        settled = matches(prefix + "_settled", {"entered": True, "settled": True, "permit_issued": permit})
        require(positive(entered["request_id"]) and entered["request_id"] == settled["request_id"]
                and entered["operation_id"] == settled["operation_id"], "unbound barrier ordering")
        return entered

    def receipt(value, operation_id):
        require(isinstance(value, str) and re.fullmatch(r"[0-9a-f]{352}", value), "invalid receipt bytes")
        raw = bytes.fromhex(value)
        require(struct.unpack_from("<IIII", raw) == (1, 176, 0, 0), "wrong committed receipt schema")
        identity = struct.unpack_from("<12Q", raw, 16)
        require(all(positive(v) for v in identity) and identity[7] == operation_id
                and identity[8:10] == (1, 2), "unbound committed receipt")
        require(raw[112:144] == hashlib.sha256(b"note=old\n").digest()
                and raw[144:176] == hashlib.sha256(b"note=new\n").digest(), "wrong receipt content hashes")

    if case_number == 0:
        if index == 0:
            matches("edit", {"before_hash": sha(b"note=old\n"), "after_hash": sha(b"note=new\n"),
                             "cursor": 8, "selection": [0, 8]})
            require([f["crop_sha256"] for f in data["frames"]] == [
                "69679689bb8a4cc1ce90a4f761525f8820f73056401d116736443a6585fee3c1",
                "cfa160298051a6ece5616dc73eb963da3ae6fe55324c59b6370bc6fe1281cf7f"], "missing immutable raster goldens")
        else:
            value = matches("scroll", {"byte_len": 4096})
            require(positive(value["first_visible_line"]), "no actual scrolling witness")
    elif case_number == 1:
        matches("ascii_bounds", {"accepted_edge": 4096, "rejected_edge": 4097,
                                 "prior_preserved": True, "lf_tab_shift_witness": True})
    elif case_number == 2:
        matches("trusted_route", {"generated_input_focus_chrome": True, "privileged_denials": 10,
                                   "foreign_valid_a_endpoint_denied": True, "foreign_input_denied": True,
                                   "both_consumers_usable": True})
    elif case_number == 3:
        value = matches("reset", {"old_held_release_suppressed": True, "fresh_attach_usable": True})
        require(positive(value["old_focus_epoch"]) and value["new_focus_epoch"] > value["old_focus_epoch"]
                and positive(value["overflow_sequence"]) and value["successor"] == value["overflow_sequence"] + 1,
                "invalid focus/reset transition")
    elif case_number == 4:
        value = matches("surface", {"retained_alias_denied": True, "chrome_pixels_unchanged": True})
        require(positive(value["frozen_sequence"]) and positive(value["old_mapping"])
                and value["fresh_mapping"] > value["old_mapping"], "invalid surface generation witness")
    elif case_number == 5:
        matches("wire", {"protocols": [802, 832, 833, 352, 368], "raw_negative_fields": 6,
                         "wrong_direction_denied": True, "schema_hash_range_generation_checked": True,
                         "selected_unchanged": True})
        probes = observations.get("raw_codec_probes")
        expected = {(p, f) for p in (802, 832, 833, 352, 368)
                    for f in ("protocol", "operation", "length", "tail", "handle", "pad")}
        require(isinstance(probes, list) and len(probes) == 30
                and {(p["protocol"], p["field"]) for p in probes} == expected, "incomplete raw codec probes")
        for probe in probes:
            _, _, canonical = frame(probe["request_hex"], schema)
            require(not canonical and probe["status"] == (3 if probe["field"] in ("protocol", "operation") else 2),
                    "accepted malformed raw wire")
    elif case_number == 6:
        if index == 0:
            value = matches("grants", {"profile": 63, "masks": [1, 1, 15, 7], "replay_denied": True})
            require(isinstance(value["grants_hex"], str) and re.fullmatch(r"[0-9a-f]{928}", value["grants_hex"]),
                    "incorrect grants byte length")
        else:
            matches("stale_preview", {"cause": ("policy", "application", "artifact", "expiry")[index - 1],
                                      "no_instance_before_fresh_approval": True})
    elif case_number == 7:
        if index == 0:
            value = matches("save", {"revision": 2, "volatile_label": True, "reopen_verified": True})
        else:
            value = matches("unknown_reconciliation", {"dispatches": 1, "permits": 1, "transitions": 1})
            matches("unknown_ui", {"actual_ctrl_s": True, "unknown_state": True, "repeated_save_no_replay": True,
                                   "newer_draft_after_receipt": "unsaved", "confirmed_revision": 2})
            barrier = barriers("after_permit", True)
            require(barrier["operation_id"] == value["operation_id"], "unbound original save barrier")
            require((data["mutation_dispatch_count"], data["permit_count"], data["transition_count"]) == (1, 1, 1),
                    "save replay or duplicate transition")
            dispatched = [struct.unpack_from("<II", bytes.fromhex(e["request_hex"])) for e in data["exchanges"]]
            require(dispatched.count((368, 3)) == dispatched.count((368, 5)) == 1
                    and sum(n for _, n in data["ledger"]["operations_per_object"]) == 1,
                    "replayed mutation request or operation allocation")
        receipt(value["receipt_hex"], value["operation_id"])
    elif case_number == 8:
        if index < 2:
            matches("retirement", {"effect": ("key", "frame")[index], "no_late_effect": True,
                                   "other_object_read_ok": True})
            matches("late_frame_probe", {"a_app_frame_absent": True, "repeated_compose_not_ready": True,
                                         "b_real_frame_ok": True})
            require(barriers("before_effect", False)["operation_id"] == 0, "unexpected non-save operation ID")
            require(sum(f["frame_kind"] == "app" for f in data["frames"]) == 2, "missing prior A and working B frame")
        else:
            after = index == 3
            value = matches("save_retirement", {"after_permit": after, "no_replay": True, "transitions": int(after)})
            require(barriers("save", after)["operation_id"] == value["original_operation_id"], "unbound retired save")
            require(data["mutation_dispatch_count"] == 1 and data["permit_count"] == int(after)
                    and data["transition_count"] == int(after), "retired save effect mismatch")
            if after:
                matches("unknown_recovery_pixels", {"save_unknown": True, "draft_lost": True,
                                                    "blank_app_crop": True, "no_guessed_outcome": True})
                require(any(f["frame_kind"] == "recovery" for f in data["frames"]), "missing Unknown recovery frame")
    elif case_number == 9:
        value = matches("fault_recovery", {"draft_lost": True, "explicit_ctrl_r_approval": True, "confirmed_revision": 1})
        require(positive(value["old_instance"]) and value["new_instance"] > value["old_instance"]
                and any(f["frame_kind"] == "recovery" for f in data["frames"]), "missing explicit recovery output")
    elif case_number == 10:
        value = matches("service_recovery", {"service": ("focus", "compositor")[index], "old_contexts_stale": True})
        require(positive(value["old_epoch"]) and value["new_epoch"] > value["old_epoch"]
                and positive(value["old_focus"]) and value["new_focus"] > value["old_focus"], "service epoch did not advance")
    elif case_number == 11:
        value = matches("deadline", {"operation": ("read", "save")[index], "response_status": (8, 10)[index],
                                    "b_keys_frame_read_ok": True, "no_maintenance_poll": True})
        require(integer(value["elapsed_ms"], 2000) and value["elapsed_ms"] >= 900
                and integer(value["b_elapsed_ms"], 999), "unbounded or premature response deadline")
        stalled = barriers("stall", index == 1)
        if index == 1:
            retry = matches("prepermit_retry", {
                "timeout_status": 8, "timeout_reply_redacted": True,
                "draft_unsaved_after_timeout": True, "old_source_stale": True,
                "fresh_explicit_ctrl_s": True, "old_worker_still_paused": True,
                "retry_base_revision": 2, "retry_revision": 3,
                "no_late_selection": True, "b_working": True,
                "allocate_requests": 4, "allocated_operations": 3, "original_commit_requests": 3,
            })
            timed_out, retried = retry["timed_out_operation_id"], retry["retry_operation_id"]
            known_operations = {stalled["operation_id"], timed_out, retried}
            require(all(positive(operation) for operation in known_operations) and len(known_operations) == 3,
                    "timeout reused the original operation")
            entered = barriers("prepermit", False)
            require(entered["operation_id"] == timed_out, "unbound pre-permit deadline")
            for phase, expected in (("before", (1, 1, 1)), ("timeout", (2, 1, 1)),
                                    ("retry", (3, 2, 2)), ("after_release", (3, 2, 2))):
                require(all(integer(retry[phase + "_" + effect], 32)
                            and retry[phase + "_" + effect] == count
                            for effect, count in zip(("dispatches", "permits", "transitions"), expected)),
                        "pre-permit timeout admitted late or replayed a save")
            require((data["mutation_dispatch_count"], data["permit_count"], data["transition_count"]) == (3, 2, 2),
                    "retry witness disagrees with runtime effects")
            dispatched = [struct.unpack_from("<II", bytes.fromhex(e["request_hex"])) for e in data["exchanges"]]
            allocated = [bytes.fromhex(e["reply_hex"]) for e in data["exchanges"]
                         if struct.unpack_from("<II", bytes.fromhex(e["request_hex"])) == (368, 3)]
            successful = [raw for raw in allocated if struct.unpack_from("<II", raw) == (368, 4)
                          and struct.unpack_from("<I", raw, 36)[0] == 0]
            require(dispatched.count((368, 3)) == 4 and len(successful) == dispatched.count((368, 5)) == 3
                    and {struct.unpack_from("<Q", raw, 28)[0] for raw in successful} == known_operations
                    and sum(n for _, n in data["ledger"]["operations_per_object"]) == 3,
                    "implicit mutation replay or allocation after timeout")
            value = retry["retry_receipt_hex"]
            require(isinstance(value, str) and re.fullmatch(r"[0-9a-f]{352}", value), "invalid retry receipt")
            raw = bytes.fromhex(value)
            require(struct.unpack_from("<IIII", raw) == (1, 176, 0, 0), "noncommitted retry receipt")
            identity = struct.unpack_from("<12Q", raw, 16)
            require(all(positive(v) for v in identity) and identity[7] == retried and identity[8:10] == (2, 3),
                    "retry receipt has wrong operation or revision binding")
            require(raw[112:144] == hashlib.sha256(b"note=new\n").digest()
                    and isinstance(retry["retry_content_hash"], str)
                    and re.fullmatch(r"[0-9a-f]{64}", retry["retry_content_hash"])
                    and raw[144:176].hex() == retry["retry_content_hash"], "retry receipt has wrong source/base hash")
    else:
        if index == 0:
            matches("monotonic_clock", {"b_advances_to": 102, "backward_a": 1, "a_revived": False})
        elif index == 1:
            matches("operation_capacity", {"retained": 16, "seventeenth_denied": True, "revision": 17})
            require(data["permit_count"] == data["transition_count"] == 16
                    and any(count == 16 for _, count in data["ledger"]["operations_per_object"]), "operation capacity not reached")
        elif index == 2:
            matches("instance_capacity", {"retained": 16, "seventeenth_denied": True})
            require(data["ledger"]["instances"] == 16, "instance capacity not reached")
        elif index < 10:
            counter = index - 3
            matches("counter_exhaustion", {"counter_index": counter, "wrapped": False, "valid_witness_before_seed": True})
            value = matches("counter_domain_retirement", {
                "counter_index": counter,
                "domain": ("identity", "focus", "queue", "mapping", "revision", "service", "time")[counter],
                "first_failure_status": 5, "retained_endpoint_status": 0 if counter == 0 else 4,
                "positive_read": counter != 6, "receipt_preserved": counter in (4, 5),
                "writer_aliases_retired": counter == 3,
            })
            prior = int(counter in (4, 5))
            for effect in ("permits", "transitions"):
                require(all(integer(value[phase + "_" + effect], 32)
                            and value[phase + "_" + effect] == prior
                            for phase in ("before", "failed", "after")), "counter exhaustion changed an admitted effect")
            require(value["before_dispatches"] == prior
                    and value["failed_dispatches"] == value["after_dispatches"] == prior + int(counter == 4)
                    and all(integer(value[phase + "_dispatches"], 32) for phase in ("before", "failed", "after")),
                    "counter exhaustion replayed a mutation")
            require(data["mutation_dispatch_count"] == value["after_dispatches"]
                    and data["permit_count"] == value["after_permits"]
                    and data["transition_count"] == value["after_transitions"], "counter witness disagrees with runtime")
            require(integer(value["queued_before"], 128) and integer(value["queued_after"], 128)
                    and value["queued_after"] == data["ledger"]["queued_keys"], "unbound queue retirement")
            if counter in (1, 2):
                require(value["queued_before"] > 0 and value["queued_after"] == 0,
                        "counter exhaustion left prior input deliverable")
        else:
            matches("trace_exhaustion", {"retained_before_overflow": 256, "terminal_status": 5,
                                         "post_overflow_evidence_failed": True, "not_silently_truncated": True})


def trace_terminal(folder, schema):
    path = folder / "trace-terminal-10.json"
    terminal = read_json(path)
    prior = folder / "registry-10.json"
    require(terminal["schema_version"] == 1 and terminal["case"] == REQUIRED[12]
            and terminal["registry_index"] == 10 and terminal["prior_record_sha256"] == sha(prior.read_bytes())
            and terminal["prior_exchange_count"] == len(read_json(prior)["exchanges"]) == 256
            and terminal["post_overflow_evidence_status"] == 5 and terminal["intentional_expected_failure"] is True,
            "unbound intentional trace-exhaustion witness")
    request, qlayout, qcanonical = frame(terminal["terminal_request_hex"], schema)
    reply, rlayout, rcanonical = frame(terminal["terminal_reply_hex"], schema)
    require(qcanonical and rcanonical and qlayout and qlayout[0] == "get_status"
            and rlayout and rlayout[0] == "get_status_reply" and struct.unpack_from("<II", request) == (352, 7)
            and struct.unpack_from("<II", reply) == (352, 8) and request[20:28] == reply[20:28]
            and struct.unpack_from("<Q", request, 20)[0] > 0 and struct.unpack_from("<I", reply, 64)[0] == 5
            and not any(reply[28:64]) and not any(reply[68:]), "invalid redacted terminal GetStatus pair")


def validate_records(evidence):
    schema = layouts()
    root = evidence / "cases"
    require(root.is_dir() and {p.name for p in root.iterdir()} == set(REQUIRED), "incomplete case evidence")
    expected_hashes = {
        "api_sha256": sha(Path("docs/DESKTOP_EDITOR_HOST_API_V0.md").read_bytes()),
        "test_sha256": sha(Path("services/desktop/tests/editor_host.rs").read_bytes()),
        "support_sha256": sha(Path("services/desktop/tests/support/editor_host_assertions.rs").read_bytes()),
    }
    counts = {}
    for case_number, case in enumerate(REQUIRED):
        folder = root / case
        files = sorted(folder.glob("registry-*.json"))
        require(0 < len(files) <= 64, "missing/unbounded registry witnesses")
        indices = set()
        observations = []
        for path in files:
            data = read_json(path)
            index = data["registry_index"]
            require(integer(index, 63) and index not in indices, "duplicate/invalid registry index")
            indices.add(index)
            require(path.name == f"registry-{index}.json", "registry filename mismatch")
            require(data["schema_version"] == 1 and type(data["schema_version"]) is int,
                    "unknown evidence schema")
            require(data["case"] == case and data["backend"] == "volatile"
                    and data["execution"] == "in_process" and data["font_sha256"] == FONT,
                    "incorrect evidence provenance")
            require(all(data[k] == v for k, v in expected_hashes.items()), "stale assertion/API source")
            exchanges = data["exchanges"]
            require(isinstance(exchanges, list) and 0 < len(exchanges) <= 256, "missing/unbounded exchanges")
            for exchange in exchanges:
                request, _, canonical = frame(exchange["request_hex"], schema)
                require(type(exchange["request_canonical"]) is bool
                        and exchange["request_canonical"] == canonical, "incorrect canonical request classification")
                raw, layout, valid = frame(exchange["reply_hex"], schema)
                require(valid and layout and "status" in layout[2], "noncanonical service reply")
                offset, _ = layout[2]["status"]
                status = struct.unpack_from("<I", raw, 20 + offset)[0]
                require(status <= 11, "unknown service status")
                if status in (1, 2, 3, 4, 5, 6, 8, 9, 11):
                    require(not any(byte for i, byte in enumerate(raw[20:84])
                                    if not (i < 8 or offset <= i < offset + 4)), "unredacted failure")
                if status in (7, 10):
                    qidentity = struct.unpack_from("<II", request)
                    ridentity = struct.unpack_from("<II", raw)
                    require(request[20:28] == raw[20:28], "uncorrelated observable outcome")
                    extra = set()
                    if status == 7 and canonical and qidentity == (368, 5) and ridentity == (368, 6):
                        extra.update(range(16, 24)) # Own current revision only.
                    if status == 10:
                        require(canonical and (qidentity, ridentity) in (((368, 5), (368, 6)), ((368, 7), (368, 8))),
                                "Unknown outside original artifact observation")
                        op_offset = 52 if qidentity == (368, 5) else 44
                        require(struct.unpack_from("<Q", raw, 28)[0] > 0 and raw[28:36] == request[op_offset:op_offset + 8],
                                "Unknown discloses a foreign or unallocated operation")
                        extra.update(range(8, 16))
                    require(not any(byte for i, byte in enumerate(raw[20:84])
                                    if not (i < 8 or offset <= i < offset + 4 or i in extra)),
                            "observable outcome contains unauthorized data")
            ledger = data["ledger"]
            for key, maximum in (("sessions", 2), ("selected_objects", 2), ("instances", 16),
                                 ("endpoints", 64), ("shared_objects", 32), ("queued_keys", 128)):
                require(integer(ledger[key], maximum), "registry quota exceeded")
            operations = ledger["operations_per_object"]
            require(isinstance(operations, list) and len(operations) <= 2, "unbounded operation ledger")
            require(all(isinstance(row, list) and len(row) == 2 and integer(row[0])
                        and integer(row[1], 16) for row in operations), "invalid operation ledger")
            require(len({row[0] for row in operations}) == len(operations), "duplicate selected object")
            for key in ("mutation_dispatch_count", "permit_count", "transition_count"):
                require(integer(data[key], 32), "unbounded save counter")
            require(data["transition_count"] <= data["permit_count"] <= data["mutation_dispatch_count"],
                    "transition without admitted mutation")
            frames = data["frames"]
            require(isinstance(frames, list) and len(frames) <= 16, "unbounded frame evidence")
            for item in frames:
                name = item["file"]
                require(isinstance(name, str) and re.fullmatch(fr"registry-{index}-frame-[0-9]+\.bgra", name),
                        "unsafe frame reference")
                pixels = (folder / name).read_bytes()
                require(len(pixels) == 640 * 568 * 4 and sha(pixels) == item["sha256"], "frame bytes/hash mismatch")
                require(sha(pixels[48 * 640 * 4:528 * 640 * 4]) == item["crop_sha256"], "crop hash mismatch")
                require(all(integer(item[k]) and item[k] > 0 for k in ("session_id", "instance_id")), "unbound frame identity")
                if item["frame_kind"] == "recovery":
                    require(integer(item["focus_epoch"]) and integer(item["sequence"])
                            and item["focus_epoch"] == item["sequence"] == 0
                            and pixels[48 * 2560:528 * 2560] == bytes([255]) * (480 * 2560)
                            and label_present(pixels, b"DRAFT LOST")
                            and label_present(pixels, b"VOLATILE / IN-PROCESS"), "invalid compositor recovery output")
                    if case_number == 8 and index == 3:
                        require(label_present(pixels, b"SAVE UNKNOWN"), "missing pending-save recovery label")
                else:
                    require(item["frame_kind"] == "app" and all(integer(item[k]) and item[k] > 0
                            for k in ("focus_epoch", "sequence")), "invalid app frame identity")
            values = data["observations"]
            require(isinstance(values, list) and 0 < len(values) <= 256, "missing/unbounded semantic observations")
            require(all(isinstance(v, dict) and isinstance(v.get("name"), str)
                        and re.fullmatch(r"[a-zA-Z0-9_]+", v["name"]) and "value" in v for v in values),
                    "invalid semantic observation")
            observations.extend(v["name"] for v in values)
            semantic_observations(case_number, index, data, schema)
        require(indices == set(range(REGISTRIES[case_number])), "incomplete semantic registry inventory")
        terminal_files = list(folder.glob("trace-terminal-*.json"))
        require([p.name for p in terminal_files] == (["trace-terminal-10.json"] if case_number == 12 else []),
                "unexpected trace-exhaustion exception")
        if case_number == 12:
            trace_terminal(folder, schema)
        counts[case] = dict(registries=len(indices), observations=observations)
    return counts


def result(evidence):
    check_list(evidence / "cases.log")
    text = (evidence / "tests.log").read_text()
    outcomes = re.findall(r"^test (\S+) \.\.\. (\w+)$", text, re.MULTILINE)
    require(len(outcomes) == 13 and {name for name, _ in outcomes} == set(REQUIRED)
            and all(status == "ok" for _, status in outcomes), "incomplete/failed editor assertions")
    summaries = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out", text)
    require(summaries == [("13", "0", "0", "0", "0")], "invalid test summary")
    records = validate_records(evidence)
    executable = []
    finished = False
    for line in (evidence / "build.jsonl").read_text().splitlines():
        data = json.loads(line, object_pairs_hook=unique_object)
        require(not finished, "Cargo record after completion")
        if data.get("reason") == "build-finished":
            require(data.get("success") is True, "failed acceptance build")
            finished = True
        if data.get("reason") == "compiler-artifact" and data.get("target", {}).get("name") == "editor_host":
            require(data.get("target", {}).get("kind") == ["test"]
                    and Path(data["target"]["src_path"]).resolve() == Path("services/desktop/tests/editor_host.rs").resolve()
                    and data.get("profile", {}).get("test") is True and data.get("executable"), "not the built acceptance test")
            executable.append(data["executable"])
    require(finished and len(executable) == 1 and Path(executable[0]).is_absolute(), "missing/ambiguous acceptance executable")
    sources = set(IDL + ("Cargo.toml", "Cargo.lock", "justfile", "docs/DESKTOP_EDITOR_HOST_API_V0.md",
                        "docs/DESKTOP_EDITOR_WIRE_V1.md", "docs/plans/desktop-editor-v0.md",
                        "tools/ci/desktop_editor_result.py", "tools/ci/foundry_desktop_editor_host_ui1_1a.sh"))
    sources.update(str(p) for p in Path("services/desktop").rglob("*") if p.is_file())
    sources.update(str(p) for p in Path("kernel_api/src").rglob("*.rs"))
    return dict(schema_version=1, gate="foundry-desktop-editor-host-ui1-1a", outcome="PASS",
                scope="volatile-in-process-editor", host=platform.platform(),
                source_commit=subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
                dirty_diff_sha256=sha(subprocess.check_output(["git", "diff", "HEAD", "--binary"])),
                source_sha256={p: sha(Path(p).read_bytes()) for p in sorted(sources)},
                test_executable_sha256=sha(Path(executable[0]).read_bytes()),
                artifact_sha256={str(p.relative_to(evidence)): sha(p.read_bytes()) for p in sorted(evidence.rglob("*"))
                                 if p.is_file() and p.name != "result.json"},
                required_cases=list(REQUIRED), executed_cases=[name for name, _ in outcomes],
                case_evidence=records, default_runtime_exposed=False,
                claims=dict(target_runtime=False, keyboard_device=False, store_io=False,
                            process_containment=False, physical_hardware=False, model_comparison=False))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--check-list", type=Path)
    group.add_argument("--evidence", type=Path)
    args = parser.parse_args()
    if args.check_list:
        check_list(args.check_list)
    else:
        output = result(args.evidence)
        (args.evidence / "result.json").write_text(json.dumps(output, indent=2) + "\n")


if __name__ == "__main__":
    main()
