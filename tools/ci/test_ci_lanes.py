"""Gate-first tests for complete CI coverage and fail-closed lane aggregation.

All commands are mocked. These tests never start a Foundry resource.
"""
import contextlib
import hashlib
import importlib.util
import inspect
import io
import json
import re
import sys
import itertools
from pathlib import Path
import tempfile
import shlex
import tomllib
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]


def load(name):
    path = ROOT / "tools/ci" / (name + ".py")
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    import sys
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


POLICY = load("change_policy")
EXTENDED_PATHS = (
    "tools/ci/foundry_review_boundaries.sh",
    "tools/ci/foundry_agent_task_suite.sh",
    "tools/ci/foundry_desktop_host_launch_ui1_0.sh",
    "tools/ci/foundry_desktop_editor_host_ui1_1a.sh",
    "tools/ci/foundry_editor_save_schema_ui1_1b.sh",
    "tools/ci/foundry_editor_native_preview_codec_ui1_1c.sh",
    "tools/ci/foundry_editor_native_save_codec_ui1_1c.sh",
    "tools/ci/foundry_desktop_editor_store_ui1_1b.sh",
    "tools/foundry/desktop_editor_native_read_gate.py",
    "tools/foundry/desktop_editor_native_preview_read_gate.py",
    "tools/ci/foundry_desktop_editor_task_ui1_1c.sh",
    "tools/ci/foundry_editor_adapter_migration_ui1_1d.sh",
    "tools/ci/foundry_boot_frame_pool_run0_0.sh",
    "tools/ci/foundry_compat_cleanup_s2.sh",
    "tools/ci/foundry_s7_all_security.sh",
    "tools/ci/foundry_boundary_s9_0_cleanup.sh",
    "tools/ci/foundry_trace_isolation_s9_0_per_domain.sh",
    "tools/ci/foundry_v012_phase5_trace_client.sh",
    "tools/ci/foundry_native_runner_s10_0.sh",
    "tools/ci/foundry_native_runner_s10_1.sh",
    "tools/ci/foundry_semantic_state_s10_2.sh",
    "tools/ci/foundry_projection_storage_s10_3.sh",
    "tools/ci/foundry_execution_fabric_s10_4.sh",
    "tools/ci/foundry_host_target_s10_5.sh",
    "tools/ci/foundry_broker_kernel_bridge_s10_5_1.sh",
    "tools/ci/foundry_qemu_ipc_bridge_s10_5_2.sh",
    "tools/ci/foundry_s11_driver_factory_s11_0.sh",
    "tools/ci/foundry_s11_replay.sh",
    "tools/ci/foundry_s11_reference_vault_s11_3.sh",
    "tools/ci/foundry_s11_runtime_net_s11_8.sh",
    "tools/ci/foundry_s12_golden_machine_s12_0.sh",
    "tools/ci/foundry_s12_gop_probe_s12_1.sh",
    "tools/ci/foundry_hil_appliance_s12_4.sh",
    "tools/ci/foundry_org_governance_g0.sh",
    "tools/ci/foundry_s13_persistent_storage_s13_0.sh",
    "tools/ci/foundry_s13_virtio_blk_oracle_s13_2.sh",
    "tools/ci/foundry_s13_replay.sh",
    "tools/ci/foundry_s13_block_sector_oracle_s13_4.sh",
    "tools/ci/foundry_s13_runtime_block_s13_6.sh",
)


class LaneTests(unittest.TestCase):
    def setUp(self):
        self.lanes = load("ci_lanes")

    def test_exact_extended_coverage_order_and_overrides(self):
        stages = self.lanes.extended_plan()
        self.assertEqual(tuple(s.argv[-1] for s in stages), EXTENDED_PATHS)
        self.assertEqual(len({s.name for s in stages}), 39)
        by_path = {s.argv[-1]: s for s in stages}
        self.assertEqual(dict(by_path["tools/ci/foundry_native_runner_s10_1.sh"].env), {"SKIP_E2E_ASSERTIONS": "1"})
        self.assertEqual(dict(by_path["tools/ci/foundry_s11_reference_vault_s11_3.sh"].env), {"REQUIRE_LIVE_ORACLE_TRACE": "1"})

    def test_isolated_lanes_partition_every_authoritative_stage(self):
        quality = self.lanes.lane_plan("quality")
        host = self.lanes.lane_plan("host")
        qemu = self.lanes.lane_plan("qemu")
        agent = self.lanes.lane_plan("agent")
        self.assertEqual(quality[0].name, "codegen")
        self.assertLess([s.name for s in quality].index("codegen"), [s.name for s in quality].index("fmt-check"))
        self.assertEqual([s.name for s in host[:5]], ["proof-image", "proof-prerequisites", "compiler-cache-prerequisites", "install-toolchain", "codegen"])
        self.assertEqual(host[2].argv, ("python3", "tools/ci/build_cache.py", "--check-inputs"))
        self.assertEqual(host[3].argv, ("python3", "tools/ci/ci_lanes.py", "install-toolchain"))
        self.assertEqual(qemu[0].name, "codegen")
        paths = [s.argv[-1] for s in host + agent + qemu if s in self.lanes.extended_plan()]
        self.assertCountEqual(paths, EXTENDED_PATHS)
        self.assertEqual(len(paths), len(set(paths)))
        self.assertIn("foundry-umbrella", [s.name for s in qemu])
        for i in (23, 24, 25, 29, 31, 34, 38):
            self.assertIn(EXTENDED_PATHS[i], [s.argv[-1] for s in qemu])
        self.assertEqual([s.name for s in quality if s.name.startswith("lint-strict-")],
                         [f"lint-strict-{i}" for i in range(1, 7)])
        self.assertIn("lint-baseline", [s.name for s in quality])
        self.assertIn("ci-optimization", [s.name for s in quality])
        self.assertIn("--locked", next(s.argv for s in quality if s.name == "workspace-tests"))

    def test_full_preflight_and_wrappers_use_same_inventory(self):
        full = self.lanes.preflight_plan()
        self.assertEqual(full[0].name, "proof-prerequisites")
        self.assertEqual(full[1].name, "compiler-cache-prerequisites")
        self.assertEqual(full[1].argv, ("python3", "tools/ci/build_cache.py", "--check-inputs"))
        self.assertEqual(len(full), 55)
        self.assertEqual(full[2:2 + len(self.lanes.quality_plan())], self.lanes.quality_plan())
        self.assertEqual(full[-39:], self.lanes.extended_plan())
        for script, command in (("foundry_preflight.sh", "all"), ("foundry_ci_extended.sh", "extended")):
            source = (ROOT / "tools/ci" / script).read_text()
            self.assertIn(f"ci_lanes.py {command}", source)
            self.assertNotIn("cargo test", source)

    def test_stage_runner_serial_environment_and_exit_propagation(self):
        stages = self.lanes.extended_plan()[19:21]
        calls = []

        def execute(argv, **kwargs):
            from types import SimpleNamespace
            calls.append((argv, kwargs))
            return SimpleNamespace(returncode=9)

        with patch.object(self.lanes.subprocess, "run", side_effect=execute):
            self.assertEqual(self.lanes.run_stages(stages, {"UNCHANGED": "yes"}), 9)
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0][0][-len(stages[0].argv):], list(stages[0].argv))
        self.assertIn("stage_timer.py", calls[0][0][1])
        self.assertEqual(calls[0][1]["env"]["SKIP_E2E_ASSERTIONS"], "1")
        self.assertEqual(calls[0][1]["env"]["UNCHANGED"], "yes")
        self.assertEqual(calls[0][1]["env"]["RUSTUP_TOOLCHAIN"], "nightly-2026-02-08")

    def test_lock_and_source_guard_denies_mutation_even_after_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.lock").write_bytes(b"original")
            checks = []

            def fingerprint(_):
                checks.append(True)
                return "source"

            def run(*_):
                (root / "Cargo.lock").write_bytes(b"changed")
                return 7

            with patch.object(self.lanes, "source_fingerprint", side_effect=fingerprint), patch.object(self.lanes, "run_stages", side_effect=run):
                with self.assertRaisesRegex(RuntimeError, "Cargo.lock changed"):
                    self.lanes.run_checked((), root)
            self.assertEqual(len(checks), 2)
            (root / "Cargo.lock").unlink()
            with self.assertRaises((OSError, RuntimeError)):
                self.lanes.run_checked((), root)

    def test_manifest_only_toolchain_commands(self):
        manifest = self.lanes.read_toolchain(ROOT)
        commands = self.lanes.toolchain_commands(manifest)
        self.assertIn("nightly-2026-02-08", commands[0])
        self.assertNotIn("nightly", [arg for cmd in commands for arg in cmd])
        self.assertEqual(commands[1][-len(manifest["components"]):], manifest["components"])
        self.assertEqual(commands[2][-len(manifest["targets"]):], manifest["targets"])
        self.assertIn("--toolchain", commands[1])
        self.assertIn("--toolchain", commands[2])

    def test_source_changes_and_stage_exceptions_still_check_final_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.lock").write_bytes(b"stable")
            with patch.object(self.lanes, "source_fingerprint", side_effect=("old", "new")), patch.object(self.lanes, "run_stages", return_value=0):
                with self.assertRaisesRegex(RuntimeError, "source changed"):
                    self.lanes.run_checked((), root)
            with patch.object(self.lanes, "source_fingerprint", side_effect=("stable", "stable")) as fingerprint, patch.object(self.lanes, "run_stages", side_effect=OSError("spawn denied")):
                with self.assertRaisesRegex(OSError, "spawn denied"):
                    self.lanes.run_checked((), root)
                self.assertEqual(fingerprint.call_count, 2)

    def test_all_sixteen_strict_package_checks_and_umbrella_environment_retained(self):
        import re
        packages = []
        for i in range(1, 7):
            body = (ROOT / f"tools/ci/foundry_lint_strict_tranche{i}.sh").read_text()
            self.assertIn('--all-targets --no-deps -- -D warnings', body)
            packages += re.findall(r"^check_crate ([a-z_0-9]+)$", body, re.M)
        self.assertCountEqual(packages, [
            "artifact_store_schema", "store_cli", "domain_manager", "portals",
            "artifact_store_core", "idl_codegen", "capsule_relay", "runtime_supervisor",
            "kernel_api", "store_service", "kernel", "ramen_sdk", "native_runner",
            "semantic_state", "execution_fabric", "driver_foundry",
        ])
        self.assertEqual(len(packages), 16)
        self.assertEqual(dict(self.lanes.UMBRELLA.env), {
            "S7_MIN_PROTOCOL_EVENTS": "6", "S7_MIN_PROTOCOL_PAIRS": "3",
            "S7_MIN_SCENARIO_EVENTS": "4", "S7_MIN_OBSERVED_CAPS": "1",
            "S7_MIN_CAP_GRANTED": "1", "S7_MIN_CAP_USED": "1",
            "S7_MIN_EXPORT_WIDTH": "1280", "S7_MIN_EXPORT_HEIGHT": "720",
            "S7_EVIDENCE_POLICY_PATH": "evidence_policy.toml",
        })


class AggregateWorkflowTests(unittest.TestCase):
    def test_fail_closed_result_matrix(self):
        allowed = getattr(POLICY, "foundry_allowed", None)
        self.assertTrue(callable(allowed), "missing fail-closed Foundry aggregate")
        states = ("success", "failure", "cancelled", "skipped", "", "unknown")
        for results in itertools.product(states, repeat=4):
            self.assertEqual(allowed("success", "true", *results), results == ("success",) * 4)
            self.assertEqual(allowed("success", "false", *results), results == ("skipped",) * 4)
        for change in states[1:]:
            self.assertFalse(allowed(change, "true", "success", "success", "success", "success"))
            self.assertFalse(allowed(change, "false", "skipped", "skipped", "skipped", "skipped"))
        for invalid in ("", "TRUE", "unknown"):
            self.assertFalse(allowed("success", invalid, "success", "success", "success", "success"))
        self.assertFalse(POLICY.merge_allowed("success", "true", "skipped", "success"))

    def test_workflow_parallel_lanes_required_aggregate_and_safe_caches(self):
        source = (ROOT / ".github/workflows/ci.yml").read_text()
        import re
        jobs = dict(re.findall(r"^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)", source, re.M | re.S))
        self.assertTrue({"quality", "host-docker", "agent-task", "qemu", "foundry", "merge-gate"} <= jobs.keys())
        for job, lane in (("quality", "quality"), ("host-docker", "host"), ("agent-task", "agent"), ("qemu", "qemu")):
            self.assertIn("needs: changes", jobs[job])
            self.assertIn("needs.changes.outputs.os_code == 'true'", jobs[job])
            self.assertIn(f"ci_lanes.py lane {lane}", jobs[job])
            if job in ("quality", "qemu"):
                self.assertIn("ci_lanes.py install-toolchain", jobs[job])
            self.assertIn("actions/upload-artifact@v4", jobs[job])
            self.assertIn("if: always()", jobs[job])
            self.assertIn("out/foundry/timings/", jobs[job])
            self.assertNotRegex(jobs[job], r"path:.*target")
        self.assertIn("needs: [changes, quality, host-docker, agent-task, qemu]", jobs["foundry"])
        self.assertIn("if: always()", jobs["foundry"])
        self.assertIn("change_policy.py foundry", jobs["foundry"])
        self.assertIn("needs: [changes, foundry, org-governance]", jobs["merge-gate"])
        self.assertIn("github.event.pull_request.number || github.ref", source)
        self.assertIn("cancel-in-progress: ${{ github.event_name == 'pull_request' }}", source)
        self.assertNotIn("rustup default nightly", source)
        self.assertIn("hashFiles('rust-toolchain.toml')", source)
        self.assertNotIn("continue-on-error", source)
        self.assertNotIn("paths-ignore", source)
        compiler_cache = jobs["host-docker"].split("Cache isolated compiler targets", 1)[1].split("- name:", 1)[0]
        self.assertIn("path: out/ci-optimization/compiler-cache", compiler_cache)
        self.assertNotIn("restore-keys", compiler_cache)
        self.assertNotIn("out/desktop", compiler_cache)
        self.assertIn('RAMEN_FOUNDRY_BUILD_CACHE: "1"', jobs["host-docker"])
        self.assertIn('just ', jobs["quality"])
        self.assertIn('SKIP_E2E_ASSERTIONS: "1"', jobs["host-docker"])


# Independent projections freeze behavior, not the implementation's Stage objects.
EXPECTED_EXTENDED_NAMES = (
    "review-boundaries", "agent-task", "desktop-launch", "desktop-editor-host",
    "editor-save-schema", "editor-preview-codec", "editor-native-save-codec",
    "desktop-editor-store", "editor-native-read", "editor-native-preview",
    "desktop-editor-task", "editor-adapter-migration", "boot-frame-pool", "compat-cleanup", "s7-security",
    "boundary-cleanup", "trace-isolation", "trace-client", "native-runner",
    "native-runner-ci", "semantic-state", "projection-storage", "execution-fabric",
    "host-target", "broker-kernel", "qemu-ipc", "driver-factory", "net-replay",
    "net-vault", "runtime-net", "golden-machine", "gop-probe", "hil-appliance",
    "org-governance", "persistent-storage", "blk-oracle", "block-replay",
    "sector-oracle", "runtime-block",
)
QEMU_EXTENDED_INDICES = (23, 24, 25, 28, 29, 30, 31, 32, 34, 35, 36, 37, 38)
UMBRELLA_ENVIRONMENT = (
    ("S7_MIN_PROTOCOL_EVENTS", "6"), ("S7_MIN_PROTOCOL_PAIRS", "3"),
    ("S7_MIN_SCENARIO_EVENTS", "4"), ("S7_MIN_OBSERVED_CAPS", "1"),
    ("S7_MIN_CAP_GRANTED", "1"), ("S7_MIN_CAP_USED", "1"),
    ("S7_MIN_EXPORT_WIDTH", "1280"), ("S7_MIN_EXPORT_HEIGHT", "720"),
    ("S7_EVIDENCE_POLICY_PATH", "evidence_policy.toml"),
)


def stage_projection(stages):
    return tuple((s.name, tuple(s.argv), tuple(s.env)) for s in stages)


def expected_extended_projection():
    result = []
    for i, (name, path) in enumerate(zip(EXPECTED_EXTENDED_NAMES, EXTENDED_PATHS)):
        argv = ("python3", path) if i in (8, 9) else (path,) if 14 <= i <= 19 else ("bash", path)
        env = (("SKIP_E2E_ASSERTIONS", "1"),) if i == 19 else (("REQUIRE_LIVE_ORACLE_TRACE", "1"),) if i == 28 else ()
        result.append((name, argv, env))
    return tuple(result)


def expected_quality_projection():
    return (
        ("codegen", ("bash", "tools/ci/run_codegen.sh"), ()),
        ("fmt-check", ("cargo", "fmt", "--all", "--check"), ()),
        ("ci-optimization", ("bash", "tools/ci/foundry_ci_optimization.sh"), ()),
        ("idl-lint", ("bash", "tools/ci/foundry_idl_lint.sh"), ()),
        ("build-targets", ("just", "build-targets"), ()),
        ("lint-baseline", ("bash", "tools/ci/foundry_lint_baseline.sh"), ()),
        *((f"lint-strict-{i}", ("bash", f"tools/ci/foundry_lint_strict_tranche{i}.sh"), ()) for i in range(1, 7)),
        ("workspace-tests", ("cargo", "test", "--locked", "--workspace", "--exclude", "kernel_uefi", "--exclude", "kernel_aarch64"), ()),
    )


def workflow_jobs():
    source = (ROOT / ".github/workflows/ci.yml").read_text()
    return dict(re.findall(r"^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)", source, re.M | re.S))


class FourLaneTests(unittest.TestCase):
    def setUp(self):
        self.lanes = load("ci_lanes")

    def test_four_lane_exact_partition(self):
        extended = self.lanes.extended_plan()
        self.assertEqual(stage_projection(extended), expected_extended_projection())
        self.assertEqual(tuple(s.name for s in extended), EXPECTED_EXTENDED_NAMES)
        bodies = {
            "agent": self.lanes.lane_plan("agent")[5:],
            "host": self.lanes.lane_plan("host")[5:],
            "qemu": self.lanes.lane_plan("qemu")[2:],
        }
        self.assertEqual(tuple(s.name for s in bodies["agent"]), ("agent-task",))
        self.assertEqual(tuple(s.name for s in bodies["qemu"]),
                         tuple(EXPECTED_EXTENDED_NAMES[i] for i in QEMU_EXTENDED_INDICES))
        self.assertEqual(tuple(s.name for s in bodies["host"]), tuple(
            name for i, name in enumerate(EXPECTED_EXTENDED_NAMES)
            if i != 1 and i not in QEMU_EXTENDED_INDICES))
        expected = expected_extended_projection()
        self.assertEqual(stage_projection(bodies["agent"]), (expected[1],))
        self.assertEqual(stage_projection(bodies["host"]), tuple(
            row for i, row in enumerate(expected) if i != 1 and i not in QEMU_EXTENDED_INDICES))
        self.assertEqual(stage_projection(bodies["qemu"]), tuple(
            expected[i] for i in QEMU_EXTENDED_INDICES))
        self.assertEqual(stage_projection(self.lanes.lane_plan("quality")), expected_quality_projection())
        self.assertEqual(stage_projection(self.lanes.lane_plan("qemu")[:2]), (
            ("codegen", ("bash", "tools/ci/run_codegen.sh"), ()),
            ("foundry-umbrella", ("tools/ci/foundry_all_s0_s1_s2_s3_s4_s5_s6.sh",), UMBRELLA_ENVIRONMENT),
        ))
        seen = [s.name for body in bodies.values() for s in body]
        self.assertEqual(len(seen), 39)
        self.assertEqual(len(set(seen)), 39)
        self.assertCountEqual(seen, EXPECTED_EXTENDED_NAMES)
        self.assertIn("desktop-editor-task", seen)
        for lane, body in bodies.items():
            self.assertTrue(all(s.lane == lane for s in body))

    def test_adapter_required_feature_targets_explicit_host_only(self):
        # This assertion must fail against the old stage inventory before adoption.
        stages = self.lanes.extended_plan()
        matching = [s for s in stages if s.name == "editor-adapter-migration"]
        self.assertEqual(len(matching), 1, "missing explicit required-feature adapter stage")
        stage = matching[0]
        self.assertEqual((stage.lane, stage.argv, stage.env), (
            "host", ("bash", "tools/ci/foundry_editor_adapter_migration_ui1_1d.sh"), ()))
        self.assertEqual([s.name for s in stages][10:13], (
            ["desktop-editor-task", "editor-adapter-migration", "boot-frame-pool"]))
        for lane in ("quality", "agent", "qemu"):
            self.assertNotIn(stage.name, [s.name for s in self.lanes.lane_plan(lane)])
        self.assertEqual([s.name for s in self.lanes.lane_plan("host")].count(stage.name), 1)
        source = (ROOT / stage.argv[-1]).read_text()
        commands = [shlex.split(line.split(" >", 1)[0]) for line in source.splitlines()
                    if line.startswith("cargo ")]
        self.assertEqual(commands, [
            ["cargo", "test", "--locked", "-p", "desktop_service", "--no-default-features",
             "--features", "desktop_v0_dev", "--test", "editor_adapter_migration", "--",
             "--test-threads=1", "--nocapture"],
            ["cargo", "test", "--locked", "-p", "artifact_editor", "--no-default-features",
             "--features", "host_editor_adapter_assertions_v0_dev", "--test",
             "editor_adapter_migration", "--", "--test-threads=1", "--nocapture"],
        ])
        self.assertIn("set -euo pipefail", source)
        self.assertIn('trap show_logs EXIT', source)
        self.assertNotIn("--skip", source)
        for crate, required in (
                ("services/desktop", ["desktop_v0_dev"]),
                ("apps/artifact_editor", ["host_editor_adapter_assertions_v0_dev"])):
            manifest = tomllib.loads((ROOT / crate / "Cargo.toml").read_text())
            self.assertEqual(manifest["features"]["default"], [])
            targets = [t for t in manifest["test"] if t["name"] == "editor_adapter_migration"]
            self.assertEqual(targets, [{"name": "editor_adapter_migration",
                "path": "tests/editor_adapter_migration.rs", "required-features": required}])
        desktop = tomllib.loads((ROOT / "services/desktop/Cargo.toml").read_text())["features"]
        app = tomllib.loads((ROOT / "apps/artifact_editor/Cargo.toml").read_text())["features"]
        self.assertEqual(desktop["editor_adapter_assertions_v0_dev"], ["editor_native_save_v0_dev"])
        self.assertEqual(app["host_editor_adapter_assertions_v0_dev"], [
            "host_native_save_v0_dev", "desktop_service/editor_adapter_assertions_v0_dev"])

    def test_canonical_local_exact_inventory(self):
        quality = expected_quality_projection()
        self.assertEqual(stage_projection(self.lanes.quality_plan()), quality)
        full = (
            ("proof-prerequisites", ("bash", "tools/ci/foundry_agent_task_suite.sh", "--check"), ()),
            ("compiler-cache-prerequisites", ("python3", "tools/ci/build_cache.py", "--check-inputs"), ()),
        ) + quality + (("foundry-umbrella", ("tools/ci/foundry_all_s0_s1_s2_s3_s4_s5_s6.sh",), UMBRELLA_ENVIRONMENT),) + expected_extended_projection()
        self.assertEqual(len(full), 55)
        self.assertEqual(stage_projection(self.lanes.preflight_plan()), full)
        for script, mode in (("foundry_preflight.sh", "all"), ("foundry_ci_extended.sh", "extended")):
            source = (ROOT / "tools/ci" / script).read_text()
            self.assertEqual(len(re.findall(r"^python3 tools/ci/ci_lanes\.py " + mode + r"\s*$", source, re.M)), 1)
            self.assertNotIn("ci_lanes.py lane", source)
        just = (ROOT / "justfile").read_text()
        self.assertRegex(just, r"(?m)^ci-lane lane:\n\s+python3 tools/ci/ci_lanes\.py lane \{\{quote\(lane\)\}\}")
        with patch.object(sys, "argv", ["ci_lanes.py", "lane", "agent"]), patch.object(self.lanes, "run_checked", return_value=0) as run:
            self.assertEqual(self.lanes.main(), 0)
            run.assert_called_once_with(self.lanes.lane_plan("agent"))
        with patch.object(sys, "argv", ["ci_lanes.py", "list", "agent"]), patch.object(self.lanes, "run_checked", side_effect=AssertionError("list must not run")), contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(self.lanes.main(), 0)
        listed = json.loads(output.getvalue())
        self.assertEqual([r["name"] for r in listed], [s.name for s in self.lanes.lane_plan("agent")])

    def test_four_result_fail_closed_aggregate(self):
        signature = inspect.signature(POLICY.foundry_allowed)
        self.assertEqual(tuple(signature.parameters), ("changes", "os_code", "quality", "host", "agent", "qemu"))
        self.assertTrue(all(p.default is inspect.Parameter.empty for p in signature.parameters.values()))
        with self.assertRaises(TypeError):
            POLICY.foundry_allowed("success", "true", "success", "success", "success")
        # Distinct tokens detect swapped CLI forwarding; symmetric success matrices cannot.
        sentinels = ("change-token", "class-token", "quality-token", "host-token", "agent-token", "qemu-token")
        with patch.object(sys, "argv", ["change_policy.py", "foundry", *sentinels]), patch.object(POLICY, "foundry_allowed", return_value=True) as allowed, contextlib.redirect_stdout(io.StringIO()) as output:
            POLICY.main()
        allowed.assert_called_once_with(*sentinels)
        self.assertEqual(output.getvalue().strip(), "foundry: PASS")
        with patch.object(sys, "argv", ["change_policy.py", "foundry", "success", "true", "success", "success", "success"]), patch.object(POLICY, "foundry_allowed") as allowed, contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit) as missing:
                POLICY.main()
            self.assertEqual(missing.exception.code, 2)
            allowed.assert_not_called()
        with patch.object(sys, "argv", ["change_policy.py", "foundry", "success", "true", "success", "success", "success", "success", "extra"]), patch.object(POLICY, "foundry_allowed") as allowed, contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit) as extra:
                POLICY.main()
            self.assertEqual(extra.exception.code, 2)
            allowed.assert_not_called()
        for results in (("success",) * 4, ("skipped",) * 4, ("success", "success", "cancelled", "success")):
            for os_code in ("true", "false"):
                expected = results == (("success",) * 4 if os_code == "true" else ("skipped",) * 4)
                with self.subTest(results=results, os_code=os_code), patch.object(sys, "argv", ["change_policy.py", "foundry", "success", os_code, *results]), contextlib.redirect_stdout(io.StringIO()):
                    if expected:
                        POLICY.main()
                    else:
                        with self.assertRaises(SystemExit) as rejected:
                            POLICY.main()
                        self.assertEqual(rejected.exception.code, "foundry: FAIL")
        with patch.object(sys, "argv", ["change_policy.py", "foundry", "failure", "false", "skipped", "skipped", "skipped", "skipped"]), contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaises(SystemExit) as failed_classification:
                POLICY.main()
            self.assertEqual(failed_classification.exception.code, "foundry: FAIL")
        jobs = workflow_jobs()
        aggregate = jobs["foundry"]
        self.assertIn("needs: [changes, quality, host-docker, agent-task, qemu]", aggregate)
        for name, value in (("CHANGES", "needs.changes.result"), ("OS_CODE", "needs.changes.outputs.os_code"), ("QUALITY", "needs.quality.result"), ("HOST", "needs.host-docker.result"), ("AGENT", "needs.agent-task.result"), ("QEMU", "needs.qemu.result")):
            self.assertIn(name + ": ${{ " + value + " }}", aggregate)
        self.assertIn("AGENT: ${{ needs.agent-task.result }}", aggregate)
        self.assertIn('change_policy.py foundry "$CHANGES" "$OS_CODE" "$QUALITY" "$HOST" "$AGENT" "$QEMU"', aggregate)
        self.assertIn("if: always()", aggregate)
        self.assertIn("needs: [changes, foundry, org-governance]", jobs["merge-gate"])

    def test_isolated_bootstrap_and_permissions(self):
        agent = self.lanes.lane_plan("agent")
        bootstrap = (
            ("proof-image", ("docker", "pull", "python@sha256:139020233cc412efe4c8135b0efe1c7569dc8b28ddd88bddb109b764f8977e30"), ()),
            ("proof-prerequisites", ("bash", "tools/ci/foundry_agent_task_suite.sh", "--check"), ()),
            ("compiler-cache-prerequisites", ("python3", "tools/ci/build_cache.py", "--check-inputs"), ()),
            ("install-toolchain", ("python3", "tools/ci/ci_lanes.py", "install-toolchain"), ()),
            ("codegen", ("bash", "tools/ci/run_codegen.sh"), ()),
        )
        self.assertEqual(stage_projection(agent[:5]), bootstrap)
        self.assertEqual(stage_projection(self.lanes.lane_plan("host")[:5]), bootstrap)
        self.assertEqual(len(agent), 6)
        from types import SimpleNamespace
        for fail_at in range(5):
            calls = []
            def failed_prerequisite(argv, **kwargs):
                calls.append(argv)
                return SimpleNamespace(returncode=13 if len(calls) - 1 == fail_at else 0)
            with self.subTest(failed_bootstrap=fail_at), patch.object(self.lanes.subprocess, "run", side_effect=failed_prerequisite):
                self.assertEqual(self.lanes.run_stages(agent, {}), 13)
            self.assertEqual(len(calls), fail_at + 1)
            self.assertNotIn("agent-task", [argv[3] for argv in calls])

        body = workflow_jobs()["agent-task"]
        self.assertIn("needs: changes", body)
        self.assertIn("if: needs.changes.outputs.os_code == 'true'", body)
        self.assertIn("runs-on: ubuntu-latest", body)
        self.assertEqual(body.count("uses: actions/checkout@v4"), 1)
        self.assertEqual(body.count("run: python3 tools/ci/ci_lanes.py lane agent"), 1)
        for setting in ('RAMEN_CI_STRICT: "1"', 'RUST_TEST_THREADS: "1"', 'LINT_ALLOW_WARNINGS: "0"', 'SKIP_E2E_ASSERTIONS: "1"', 'RAMEN_FOUNDRY_BUILD_CACHE: "1"'):
            self.assertIn(setting, body)
        self.assertNotIn("run: python3 tools/ci/ci_lanes.py install-toolchain", body)
        for forbidden in ("needs: quality", "needs: host-docker", "git worktree", " &", "continue-on-error"):
            self.assertNotIn(forbidden, body)
        # The existing cache prerequisite owns conditional CARGO_HOME/umask admission;
        # this lane split does not invent unconditional filesystem setup or relax it.

    def test_source_lock_finally_and_exit(self):
        stages = self.lanes.lane_plan("agent")
        for event in ("success", "failure", "spawn", "source", "source-failure", "lock", "lock-failure", "missing-lock", "symlink-lock"):
            with self.subTest(event=event), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                lock = root / "Cargo.lock"
                lock.write_bytes(b"original")
                def execute(*_):
                    if event == "spawn":
                        raise OSError("spawn denied")
                    if event in ("lock", "lock-failure"):
                        lock.write_bytes(b"changed")
                    if event == "missing-lock":
                        lock.unlink()
                    if event == "symlink-lock":
                        lock.unlink()
                        (root / "alias").write_bytes(b"original")
                        lock.symlink_to("alias")
                    return 7 if event in ("failure", "source-failure", "lock-failure") else 0
                values = ("before", "after") if event in ("source", "source-failure") else ("stable", "stable")
                with patch.object(self.lanes, "source_fingerprint", side_effect=values) as fingerprint, patch.object(self.lanes, "run_stages", side_effect=execute) as run:
                    if event in ("success", "failure"):
                        self.assertEqual(self.lanes.run_checked(stages, root), 0 if event == "success" else 7)
                    elif event == "spawn":
                        with self.assertRaisesRegex(OSError, "spawn denied"):
                            self.lanes.run_checked(stages, root)
                    else:
                        with self.assertRaisesRegex(RuntimeError, "source changed" if event in ("source", "source-failure") else "Cargo.lock changed"):
                            self.lanes.run_checked(stages, root)
                    self.assertEqual(fingerprint.call_count, 2)
                    run.assert_called_once_with(stages)

    def test_cache_and_exclusion_preservation(self):
        jobs = workflow_jobs()
        host = jobs["host-docker"]
        compiler = host.split("Cache isolated compiler targets", 1)[1].split("- name:", 1)[0]
        self.assertIn("path: out/ci-optimization/compiler-cache", compiler)
        self.assertNotIn("restore-keys", compiler)
        self.assertIn("hashFiles('Cargo.lock', 'rust-toolchain.toml', '**/*.rs', '**/Cargo.toml', 'idl/**', 'tools/**', 'docs/**', 'justfile', '.cargo/**', '.github/**', 'services/**/fixtures/**')", compiler)
        for lane in ("quality", "host-docker", "agent-task", "qemu"):
            body = jobs[lane]
            self.assertNotIn("actions/download-artifact", body)
            self.assertNotRegex(body, r"(?m)^\s+(?:path|run):.*out/desktop")
            self.assertNotIn("cached_acceptance: true", body)
        agent = jobs["agent-task"]
        self.assertNotIn("out/ci-optimization/compiler-cache", agent)
        self.assertNotIn("Cache isolated compiler targets", agent)
        self.assertEqual(agent.count("uses: actions/cache@v4"), 1)
        self.assertRegex(agent, r"(?m)^          path: \|\n            ~/\.cargo/registry\n            ~/\.cargo/git\n            ~/\.rustup/toolchains\n          key: ")
        self.assertIn("~/.cargo/registry", agent)
        self.assertIn("~/.cargo/git", agent)
        self.assertIn("~/.rustup/toolchains", agent)
        self.assertIn("hashFiles('rust-toolchain.toml')", agent)
        self.assertIn("hashFiles('Cargo.lock', '**/Cargo.toml')", agent)
        host_paths = {s.argv[-1] for s in self.lanes.lane_plan("host")[5:]}
        self.assertTrue({EXTENDED_PATHS[8], EXTENDED_PATHS[9], EXTENDED_PATHS[10]} <= host_paths)

    def test_timing_artifact_failure_retention(self):
        agent = self.lanes.lane_plan("agent")
        from types import SimpleNamespace
        for code in (0, 11):
            calls = []
            def execute(argv, **kwargs):
                calls.append((argv, kwargs))
                return SimpleNamespace(returncode=code)
            env = {"UNCHANGED": "yes", "RAMEN_FOUNDRY_BUILD_CACHE": "1", "RUST_TEST_THREADS": "1"}
            with patch.object(self.lanes.subprocess, "run", side_effect=execute):
                self.assertEqual(self.lanes.run_stages(agent, env), code)
            self.assertEqual(len(calls), len(agent) if code == 0 else 1)
            for i, (argv, kwargs) in enumerate(calls):
                self.assertEqual(argv, ["python3", str(ROOT / "tools/ci/stage_timer.py"), "--label", agent[i].name, "--", *agent[i].argv])
                self.assertEqual(kwargs["cwd"], ROOT)
                self.assertFalse(kwargs["check"])
                self.assertEqual(kwargs["env"]["UNCHANGED"], "yes")
                self.assertEqual(kwargs["env"]["RAMEN_FOUNDRY_BUILD_CACHE"], "1")
                self.assertEqual(kwargs["env"]["RUST_TEST_THREADS"], "1")
                self.assertEqual(kwargs["env"]["RUSTUP_TOOLCHAIN"], "nightly-2026-02-08")
        names = []
        for job in ("quality", "host-docker", "agent-task", "qemu"):
            body = workflow_jobs()[job]
            upload = body.split("uses: actions/upload-artifact@v4", 1)
            self.assertEqual(len(upload), 2)
            self.assertRegex(upload[0], r"if: always\(\)\s*$")
            self.assertIn("path: out/foundry/timings/", upload[1])
            self.assertIn("if-no-files-found: error", upload[1])
            name = re.search(r"(?m)^\s+name: (foundry-[^\n]+)$", upload[1]).group(1)
            self.assertIn("${{ github.run_id }}", name)
            self.assertIn("${{ github.run_attempt }}", name)
            names.append(name)
        self.assertEqual(len(set(names)), 4)
        # Upload during cancellation is best effort; missing artifacts never turn
        # an actual cancelled lane into accepted Foundry execution.
        self.assertFalse(POLICY.foundry_allowed("success", "true", "success", "success", "cancelled", "success"))

    def test_suite_consumer_behavior_unchanged(self):
        source = (ROOT / "tools/ci/foundry_agent_task_suite.sh").read_text()
        self.assertEqual(hashlib.sha256(source.encode()).hexdigest(), "9c7ced0895a5df2c24bd71d7d302d7e51ed90b9dda75cde09ab0dcd6b90be851")
        match = re.search(r"(?ms)^gates=\(\n(.*?)^\)", source)
        self.assertIsNotNone(match)
        self.assertEqual(match.group(1).split(), [
            "contract_a0", "protocol_a1_0", "proof_rt", "linux_control", "adapter", "lt",
            "ls_transactions", "subscriptions", "authority", "evaluator_controls",
            "reconciliation", "requestable_authority",
        ])
        self.assertIn('if [[ "$(uname -s)" != Linux ]]', source)
        self.assertIn("import jsonschema", source)
        self.assertIn("Sandbox(readonly={}, writable={})", source)
        self.assertLess(source.index("Sandbox(readonly={}, writable={})"), source.index('if [[ "${1:-}" == --check ]]'))
        self.assertEqual(source.count('bash "tools/ci/foundry_agent_task_${gate}.sh"'), 1)
        jobs = workflow_jobs()
        for job, lane in (("quality", "quality"), ("host-docker", "host"), ("agent-task", "agent"), ("qemu", "qemu")):
            body = jobs[job]
            commands = re.findall(r"(?m)^\s+run: python3 tools/ci/ci_lanes\.py (.+)$", body)
            self.assertEqual(commands.count("lane " + lane), 1)
            self.assertNotIn("all", commands)
            self.assertNotIn("extended", commands)
            self.assertNotRegex(body, r"(?m)^\s+run:.*(?:just preflight|just ci-extended|foundry_ci_extended\.sh|foundry_preflight\.sh|foundry_agent_task_suite\.sh)")


if __name__ == "__main__":
    unittest.main(verbosity=2)
