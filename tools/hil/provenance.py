"""Bind HIL observations to a prepared build and a caller-selected boot nonce.

Build manifests and controller bundles are trusted lab records, not remote
attestation. The embedded build ID is distinct from the final EFI file digest.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "init"))
from build_init_image import PROFILES as INIT_PROFILES, build as build_init_image

MARKER_KEYS = ("git_sha", "init_profile", "machine_id", "storage_manifest_sha256",
               "kernel_build_id", "init_img_sha256", "boot_epoch_nonce")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    h = hashlib.sha256()
    with open(path, "rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def write_json(path, data):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    tmp.replace(path)


def parse_serial(path):
    markers = {}
    banners = 0
    for line in Path(path).read_text(encoding="utf-8", errors="strict").splitlines():
        line = line.strip()
        if line.startswith("RAMEN OS"):
            banners += 1
        if line.startswith("hil_evidence:"):
            match = re.fullmatch(r"hil_evidence: ([a-z0-9_]+)=([^\s]+)", line)
            require(banners == 1, "provenance outside target boot record")
            require(match is not None, "malformed provenance marker")
            key, value = match.groups()
            require(key in MARKER_KEYS and key not in markers, "unknown or duplicate provenance marker")
            markers[key] = value
    require(banners == 1, "expected exactly one target boot record")
    require(set(markers) == set(MARKER_KEYS), "incomplete provenance markers")
    for key in ("kernel_build_id", "storage_manifest_sha256", "init_img_sha256"):
        require(re.fullmatch(r"[0-9a-f]{64}", markers[key]), f"invalid {key}")
    require(re.fullmatch(r"[0-9a-f]{40}", markers["git_sha"]), "invalid git_sha")
    for key in ("init_profile", "machine_id"):
        require(re.fullmatch(r"[A-Za-z0-9._-]+", markers[key]) and markers[key] != "unknown", f"invalid {key}")
    require(re.fullmatch(r"[0-9a-fA-F]{1,16}", markers["boot_epoch_nonce"]), "invalid boot nonce")
    return markers


def validate_gate_marker(path, marker):
    lines = [line.strip() for line in Path(path).read_text().splitlines()]
    banners = [i for i, line in enumerate(lines) if line.startswith("RAMEN OS")]
    successes = [i for i, line in enumerate(lines) if line == marker]
    provenance = [i for i, line in enumerate(lines) if line.startswith("hil_evidence:")]
    require(len(banners) == 1 and len(successes) == 1 and bool(provenance), "missing or ambiguous boot result")
    require(banners[0] < min(provenance) and max(provenance) < successes[0], "success outside current proven boot")
    prefix = marker.removesuffix(" ok").split("=", 1)[0]
    for line in lines[banners[0]:]:
        require(not line.startswith(prefix + " failed"), "boot contains a conflicting failure")
        if "=" in marker and line.startswith(prefix + "="):
            require(line == marker, "boot contains a conflicting result")


def load_build(path):
    require(bool(path), "RAMEN_HIL_EXPECTED_BUILD is required")
    path = Path(path)
    expected = json.loads(path.read_text())
    require(expected.get("schema_version") == 1, "invalid build manifest schema")
    for artifact, hash_key in [("kernel_efi", "kernel_efi_sha256"), ("init_img", "init_img_sha256")]:
        target = path.parent / expected[artifact]
        require(digest(target) == expected[hash_key], f"prepared {artifact} hash mismatch")
    return expected


def validate_serial(log, expected_path, nonce, require_nonce=False):
    markers = parse_serial(log)
    expected = load_build(expected_path)
    for key in MARKER_KEYS[:-1]:
        require(markers[key] == expected.get(key), f"target {key} does not match prepared build")
    if nonce or require_nonce:
        require(bool(re.fullmatch(r"[0-9a-fA-F]{1,16}", nonce or "")), "expected boot nonce required")
        require(int(nonce, 16) != 0 and int(markers["boot_epoch_nonce"], 16) == int(nonce, 16), "stale or zero boot nonce")
    return markers


def validate_controller(path, serial, run_id, appliance_id, target_id, markers):
    require(bool(path) and bool(run_id) and bool(appliance_id) and bool(target_id), "controller run identity required")
    path = Path(path)
    payload = json.loads(path.read_text())
    checks = {"schema_version": 1, "evidence_kind": "hil_appliance_run_v0",
              "evidence_level": "PASS/HIL-APPLIANCE", "serial_input_kind": "live_device",
              "result": "pass", "run_id": run_id, "appliance_id": appliance_id, "target_id": target_id,
              "serial_log_sha256": digest(serial), "target_hil_evidence_markers": markers}
    for key, value in checks.items():
        require(payload.get(key) == value, f"controller {key} mismatch")
    for key in ("serial_log", "controller_log"):
        require(bool(payload.get(key)), f"missing controller {key}")
        require(digest(payload[key]) == payload.get(key + "_sha256"), f"controller {key} digest mismatch")
    start, end = payload.get("started_at_unix_ms"), payload.get("ended_at_unix_ms")
    require(type(start) is int and type(end) is int and 0 < start <= end, "invalid controller capture interval")
    return payload


def emit(out, gate, level, serial, marker, efi, init, claim_path):
    env = os.environ
    appliance = env.get("RAMEN_HIL_APPLIANCE") == "1"
    graduation = env.get("RAMEN_HIL_GRADUATION") == "1"
    require(level != "PASS/METAL" or graduation, "metal requires graduation mode")
    require(level in {"PASS/METAL", "PASS/HIL-LIVE", "PASS/HIL-LOG", "PASS/QEMU"}, "invalid evidence level")
    canonical_claim = {"PASS/QEMU": "qemu-or-scaffold", "PASS/HIL-LOG": "development-log-replay",
                       "PASS/HIL-LIVE": "appliance-live" if appliance else "operator-live",
                       "PASS/METAL": "appliance-mediated" if appliance else "operator-golden-machine"}[level]
    require(claim_path == canonical_claim, "claim path does not match evidence level")
    if level in {"PASS/METAL", "PASS/HIL-LIVE"}:
        require(not env.get("RAMEN_HIL_SERIAL_LOG"), "live evidence cannot use a replay log")
        require(bool(env.get("RAMEN_HIL_SERIAL_DEV")) and Path(env["RAMEN_HIL_SERIAL_DEV"]).is_char_device(), "live serial device required")
    expected, markers, controller = {}, {}, {}
    controller_path = env.get("RAMEN_HIL_CONTROLLER_EVIDENCE", "")
    if level in {"PASS/METAL", "PASS/HIL-LIVE"}:
        validate_gate_marker(serial, marker)
        markers = validate_serial(serial, env.get("RAMEN_HIL_EXPECTED_BUILD", ""),
                                  env.get("RAMEN_HIL_EXPECTED_NONCE", ""), graduation)
        expected = load_build(env["RAMEN_HIL_EXPECTED_BUILD"])
        require(digest(efi) == expected["kernel_efi_sha256"] and digest(init) == expected["init_img_sha256"], "gate artifacts differ from prepared build")
        if appliance:
            controller = validate_controller(controller_path, serial, env.get("RAMEN_HIL_RUN_ID", ""),
                                             env.get("RAMEN_HIL_APPLIANCE_ID", ""), expected["machine_id"], markers)
    payload = {
        "schema_version": 1, "gate_id": gate, "evidence_level": level, "claim_path": claim_path,
        "timestamp_utc": datetime.now(timezone.utc).isoformat(),
        "git_sha": expected.get("git_sha", env.get("RAMEN_GIT_SHA", "unknown")),
        "machine_id": expected.get("machine_id", env.get("RAMEN_MACHINE_ID", "unknown")),
        "storage_manifest_sha256": expected.get("storage_manifest_sha256", env.get("RAMEN_STORAGE_MANIFEST_SHA256", "unknown")),
        "kernel_build_id": expected.get("kernel_build_id", "unknown"),
        "kernel_efi_sha256": digest(efi) if efi else "unknown",
        "init_img_sha256": digest(init) if init else "unknown",
        "serial_log": str(Path(serial).resolve()), "serial_log_sha256": digest(serial),
        "marker": marker, "graduation_mode": graduation, "target_hil_evidence_markers": markers,
        "appliance": {"enabled": appliance, "appliance_id": controller.get("appliance_id", ""),
                      "target_id": controller.get("target_id", ""), "controller_evidence": controller_path if controller else "",
                      "controller_log": controller.get("controller_log", ""),
                      "controller_log_sha256": controller.get("controller_log_sha256", ""),
                      "power_events": controller.get("power_events", [])},
    }
    write_json(out, payload)


def main():
    parser = argparse.ArgumentParser(__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    verify = sub.add_parser("verify")
    verify.add_argument("serial")
    build = sub.add_parser("build")
    for arg in ["out", "efi", "init", "profile"]: build.add_argument(arg)
    relocate = sub.add_parser("relocate")
    for arg in ["source", "out", "efi", "init"]: relocate.add_argument(arg)
    prepared = sub.add_parser("prepared")
    for arg in ["manifest", "profile", "artifact"]: prepared.add_argument(arg)
    emission = sub.add_parser("emit")
    for arg in ["out", "gate", "level", "serial", "marker", "efi", "init", "claim_path"]: emission.add_argument(arg)
    args = parser.parse_args()
    env = os.environ
    if args.command == "verify":
        validate_serial(args.serial, env.get("RAMEN_HIL_EXPECTED_BUILD", ""), env.get("RAMEN_HIL_EXPECTED_NONCE", ""), env.get("RAMEN_HIL_GRADUATION") == "1")
    elif args.command == "prepared":
        expected = load_build(args.manifest)
        require(args.profile in INIT_PROFILES and expected.get("init_profile") == INIT_PROFILES[args.profile][0], "prepared image has wrong init profile")
        require(args.artifact in {"kernel_efi", "init_img"}, "invalid artifact selector")
        print((Path(args.manifest).resolve().parent / expected[args.artifact]).resolve())
    elif args.command == "build":
        data = {key.lower().removeprefix("ramen_"): env[key] for key in ["RAMEN_GIT_SHA", "RAMEN_MACHINE_ID", "RAMEN_STORAGE_MANIFEST_SHA256", "RAMEN_KERNEL_BUILD_ID", "RAMEN_INIT_IMG_SHA256"]}
        data.update(schema_version=1, init_profile=INIT_PROFILES[args.profile][0], kernel_efi_sha256=digest(args.efi),
                    kernel_efi=str(Path(args.efi).resolve()), init_img=str(Path(args.init).resolve()))
        write_json(args.out, data)
    elif args.command == "relocate":
        data = load_build(args.source)
        require(digest(args.efi) == data["kernel_efi_sha256"] and digest(args.init) == data["init_img_sha256"], "copied artifacts changed")
        data.update(kernel_efi=str(Path(args.efi).resolve()), init_img=str(Path(args.init).resolve()))
        write_json(args.out, data)
    else:
        emit(args.out, args.gate, args.level, args.serial, args.marker, args.efi, args.init, args.claim_path)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError) as error:
        raise SystemExit(f"HIL_PROVENANCE: FAIL {error}") from error
