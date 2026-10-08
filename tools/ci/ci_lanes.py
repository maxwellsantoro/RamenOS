#!/usr/bin/env python3
"""Canonical complete Foundry inventory; no cached PASS or parallel gate children.

CI lanes execute on separate runners. Local ``all`` preserves complete preflight
coverage; ``extended`` is the original extended sequence. Each stage delegates
timing and exit propagation to stage_timer.py. Legacy gate internals do not all
pass --locked yet: a tracked lock and immutable tracked inputs are checked before
and in finally after every invocation. Direct Cargo commands are locked.
"""
import argparse
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]
PROOF_IMAGE = "python@sha256:139020233cc412efe4c8135b0efe1c7569dc8b28ddd88bddb109b764f8977e30"


@dataclass(frozen=True)
class Stage:
    name: str
    lane: str
    argv: tuple[str, ...]
    env: tuple[tuple[str, str], ...] = ()


def shell(name, path, lane="host", env=(), direct=False):
    return Stage(name, lane, (path,) if direct else ("bash", path), env)


CODEGEN = shell("codegen", "tools/ci/run_codegen.sh", "quality")
PREREQUISITES = Stage("proof-prerequisites", "host", ("bash", "tools/ci/foundry_agent_task_suite.sh", "--check"))
CACHE_PREREQUISITES = Stage("compiler-cache-prerequisites", "host", ("python3", "tools/ci/build_cache.py", "--check-inputs"))
IMAGE = Stage("proof-image", "host", ("docker", "pull", PROOF_IMAGE))
TOOLCHAIN = Stage("install-toolchain", "host", ("python3", "tools/ci/ci_lanes.py", "install-toolchain"))
UMBRELLA = shell("foundry-umbrella", "tools/ci/foundry_all_s0_s1_s2_s3_s4_s5_s6.sh", "qemu", (
    ("S7_MIN_PROTOCOL_EVENTS", "6"), ("S7_MIN_PROTOCOL_PAIRS", "3"),
    ("S7_MIN_SCENARIO_EVENTS", "4"), ("S7_MIN_OBSERVED_CAPS", "1"),
    ("S7_MIN_CAP_GRANTED", "1"), ("S7_MIN_CAP_USED", "1"),
    ("S7_MIN_EXPORT_WIDTH", "1280"), ("S7_MIN_EXPORT_HEIGHT", "720"),
    ("S7_EVIDENCE_POLICY_PATH", "evidence_policy.toml"),
), direct=True)
EXTENDED = (
    shell("review-boundaries", "tools/ci/foundry_review_boundaries.sh"),
    shell("agent-task", "tools/ci/foundry_agent_task_suite.sh", "agent"),
    shell("desktop-launch", "tools/ci/foundry_desktop_host_launch_ui1_0.sh"),
    shell("desktop-editor-host", "tools/ci/foundry_desktop_editor_host_ui1_1a.sh"),
    shell("editor-save-schema", "tools/ci/foundry_editor_save_schema_ui1_1b.sh"),
    shell("editor-preview-codec", "tools/ci/foundry_editor_native_preview_codec_ui1_1c.sh"),
    shell("editor-native-save-codec", "tools/ci/foundry_editor_native_save_codec_ui1_1c.sh"),
    shell("desktop-editor-store", "tools/ci/foundry_desktop_editor_store_ui1_1b.sh"),
    Stage("editor-native-read", "host", ("python3", "tools/foundry/desktop_editor_native_read_gate.py")),
    Stage("editor-native-preview", "host", ("python3", "tools/foundry/desktop_editor_native_preview_read_gate.py")),
    shell("desktop-editor-task", "tools/ci/foundry_desktop_editor_task_ui1_1c.sh"),
    shell("editor-adapter-migration", "tools/ci/foundry_editor_adapter_migration_ui1_1d.sh"),
    shell("boot-frame-pool", "tools/ci/foundry_boot_frame_pool_run0_0.sh"),
    shell("compat-cleanup", "tools/ci/foundry_compat_cleanup_s2.sh"),
    shell("s7-security", "tools/ci/foundry_s7_all_security.sh", direct=True),
    shell("boundary-cleanup", "tools/ci/foundry_boundary_s9_0_cleanup.sh", direct=True),
    shell("trace-isolation", "tools/ci/foundry_trace_isolation_s9_0_per_domain.sh", direct=True),
    shell("trace-client", "tools/ci/foundry_v012_phase5_trace_client.sh", direct=True),
    shell("native-runner", "tools/ci/foundry_native_runner_s10_0.sh", direct=True),
    shell("native-runner-ci", "tools/ci/foundry_native_runner_s10_1.sh", env=(("SKIP_E2E_ASSERTIONS", "1"),), direct=True),
    shell("semantic-state", "tools/ci/foundry_semantic_state_s10_2.sh"),
    shell("projection-storage", "tools/ci/foundry_projection_storage_s10_3.sh"),
    shell("execution-fabric", "tools/ci/foundry_execution_fabric_s10_4.sh"),
    shell("host-target", "tools/ci/foundry_host_target_s10_5.sh", "qemu"),
    shell("broker-kernel", "tools/ci/foundry_broker_kernel_bridge_s10_5_1.sh", "qemu"),
    shell("qemu-ipc", "tools/ci/foundry_qemu_ipc_bridge_s10_5_2.sh", "qemu"),
    shell("driver-factory", "tools/ci/foundry_s11_driver_factory_s11_0.sh"),
    shell("net-replay", "tools/ci/foundry_s11_replay.sh"),
    shell("net-vault", "tools/ci/foundry_s11_reference_vault_s11_3.sh", "qemu", (("REQUIRE_LIVE_ORACLE_TRACE", "1"),)),
    shell("runtime-net", "tools/ci/foundry_s11_runtime_net_s11_8.sh", "qemu"),
    shell("golden-machine", "tools/ci/foundry_s12_golden_machine_s12_0.sh", "qemu"),
    shell("gop-probe", "tools/ci/foundry_s12_gop_probe_s12_1.sh", "qemu"),
    shell("hil-appliance", "tools/ci/foundry_hil_appliance_s12_4.sh", "qemu"),
    shell("org-governance", "tools/ci/foundry_org_governance_g0.sh"),
    shell("persistent-storage", "tools/ci/foundry_s13_persistent_storage_s13_0.sh", "qemu"),
    shell("blk-oracle", "tools/ci/foundry_s13_virtio_blk_oracle_s13_2.sh", "qemu"),
    shell("block-replay", "tools/ci/foundry_s13_replay.sh", "qemu"),
    shell("sector-oracle", "tools/ci/foundry_s13_block_sector_oracle_s13_4.sh", "qemu"),
    shell("runtime-block", "tools/ci/foundry_s13_runtime_block_s13_6.sh", "qemu"),
)


def extended_plan():
    return EXTENDED


def quality_plan():
    return (
        CODEGEN,
        Stage("fmt-check", "quality", ("cargo", "fmt", "--all", "--check")),
        shell("ci-optimization", "tools/ci/foundry_ci_optimization.sh", "quality"),
        shell("idl-lint", "tools/ci/foundry_idl_lint.sh", "quality"),
        Stage("build-targets", "quality", ("just", "build-targets")),
        shell("lint-baseline", "tools/ci/foundry_lint_baseline.sh", "quality"),
        *(shell(f"lint-strict-{i}", f"tools/ci/foundry_lint_strict_tranche{i}.sh", "quality") for i in range(1, 7)),
        Stage("workspace-tests", "quality", ("cargo", "test", "--locked", "--workspace", "--exclude", "kernel_uefi", "--exclude", "kernel_aarch64")),
    )


def lane_plan(lane):
    if lane == "quality":
        return quality_plan()
    if lane in ("host", "agent"):
        return (IMAGE, PREREQUISITES, CACHE_PREREQUISITES, TOOLCHAIN, CODEGEN) + tuple(s for s in EXTENDED if s.lane == lane)
    if lane == "qemu":
        return (CODEGEN, UMBRELLA) + tuple(s for s in EXTENDED if s.lane == "qemu")
    raise ValueError("unknown CI lane")


def preflight_plan():
    # Prerequisites remain first locally; each isolated CI lane has its own setup.
    return (PREREQUISITES, CACHE_PREREQUISITES) + quality_plan() + (UMBRELLA,) + EXTENDED


def run_stages(stages, environment=None):
    inherited = dict(os.environ if environment is None else environment)
    inherited["RUSTUP_TOOLCHAIN"] = read_toolchain(ROOT)["channel"]
    for stage in stages:
        env = inherited | dict(stage.env)
        command = ["python3", str(ROOT / "tools/ci/stage_timer.py"), "--label", stage.name, "--", *stage.argv]
        print(f"FOUNDRY_LANES: stage={stage.name} lane={stage.lane}", flush=True)
        result = subprocess.run(command, cwd=ROOT, env=env, check=False)
        if result.returncode:
            return result.returncode
    return 0


def file_hash(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def source_fingerprint(root):
    # Include content even for already-dirty inputs, not just status filenames.
    # Ignored generated outputs are derived per runner; no cache supplies them.
    raw = subprocess.check_output(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], cwd=root)
    digest = hashlib.sha256()
    for name in sorted(set(raw.split(b"\0"))):
        if not name:
            continue
        path = root / os.fsdecode(name)
        digest.update(name + b"\0")
        if path.is_symlink():
            digest.update(b"symlink\0" + os.fsencode(os.readlink(path)))
        elif path.is_file():
            digest.update(bytes.fromhex(file_hash(path)))
        else:
            digest.update(b"missing\0")
    return digest.hexdigest()


def run_checked(stages, root=ROOT):
    lock = root / "Cargo.lock"
    if not lock.is_file() or lock.is_symlink():
        raise RuntimeError("tracked Cargo.lock prerequisite missing or unsafe")
    # Root owns adding the lock to Git. Require it when running a real checkout.
    if root == ROOT:
        subprocess.run(["git", "ls-files", "--error-unmatch", "Cargo.lock"], cwd=root, check=True, stdout=subprocess.DEVNULL)
    original_lock = file_hash(lock)
    original_source = source_fingerprint(root)
    try:
        return run_stages(stages)
    finally:
        final_source = source_fingerprint(root)
        if not lock.is_file() or lock.is_symlink() or file_hash(lock) != original_lock:
            raise RuntimeError("Cargo.lock changed during Foundry execution")
        if final_source != original_source:
            raise RuntimeError("tracked source changed during Foundry execution")


def read_toolchain(root):
    value = tomllib.loads((root / "rust-toolchain.toml").read_text())["toolchain"]
    channel = value["channel"]
    if not isinstance(channel, str) or not re.fullmatch(r"nightly-\d{4}-\d{2}-\d{2}", channel):
        raise ValueError("manifest must pin a dated nightly")
    for key in ("components", "targets"):
        if not isinstance(value[key], list) or not value[key] or any(not isinstance(v, str) or not re.fullmatch(r"[a-zA-Z0-9_-]+", v) for v in value[key]):
            raise ValueError("invalid manifest toolchain members")
    return value


def toolchain_commands(manifest):
    channel = manifest["channel"]
    return [
        ["rustup", "toolchain", "install", channel, "--profile", "minimal"],
        ["rustup", "component", "add", "--toolchain", channel, *manifest["components"]],
        ["rustup", "target", "add", "--toolchain", channel, *manifest["targets"]],
    ]


def main():
    parser = argparse.ArgumentParser(__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("all", "extended", "install-toolchain", "toolchain"):
        sub.add_parser(name)
    sub.add_parser("lane").add_argument("lane", choices=("quality", "host", "agent", "qemu"))
    listing = sub.add_parser("list")
    listing.add_argument("mode", choices=("all", "extended", "quality", "host", "agent", "qemu"))
    args = parser.parse_args()
    if args.command in ("toolchain", "install-toolchain"):
        manifest = read_toolchain(ROOT)
        if args.command == "toolchain":
            print(manifest["channel"])
            return 0
        for command in toolchain_commands(manifest):
            subprocess.run(command, cwd=ROOT, check=True)
        return 0
    mode = args.mode if args.command == "list" else args.lane if args.command == "lane" else args.command
    stages = preflight_plan() if mode == "all" else extended_plan() if mode == "extended" else lane_plan(mode)
    if args.command == "list":
        print(json.dumps([{"name": s.name, "lane": s.lane, "argv": s.argv, "env": dict(s.env)} for s in stages], indent=2))
        return 0
    return run_checked(stages)


if __name__ == "__main__":
    raise SystemExit(main())
