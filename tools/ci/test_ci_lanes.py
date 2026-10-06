"""Gate-first tests for complete CI coverage and fail-closed lane aggregation.

All commands are mocked. These tests never start a Foundry resource.
"""
import importlib.util
import itertools
from pathlib import Path
import tempfile
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
    "tools/ci/foundry_desktop_editor_store_ui1_1b.sh",
    "tools/foundry/desktop_editor_native_read_gate.py",
    "tools/foundry/desktop_editor_native_preview_read_gate.py",
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
        self.assertEqual(len({s.name for s in stages}), 36)
        by_path = {s.argv[-1]: s for s in stages}
        self.assertEqual(dict(by_path[EXTENDED_PATHS[16]].env), {"SKIP_E2E_ASSERTIONS": "1"})
        self.assertEqual(dict(by_path[EXTENDED_PATHS[25]].env), {"REQUIRE_LIVE_ORACLE_TRACE": "1"})

    def test_isolated_lanes_partition_every_authoritative_stage(self):
        quality = self.lanes.lane_plan("quality")
        host = self.lanes.lane_plan("host")
        qemu = self.lanes.lane_plan("qemu")
        self.assertEqual(quality[0].name, "codegen")
        self.assertLess([s.name for s in quality].index("codegen"), [s.name for s in quality].index("fmt-check"))
        self.assertEqual([s.name for s in host[:5]], ["proof-image", "proof-prerequisites", "compiler-cache-prerequisites", "install-toolchain", "codegen"])
        self.assertEqual(host[2].argv, ("python3", "tools/ci/build_cache.py", "--check-inputs"))
        self.assertEqual(host[3].argv, ("python3", "tools/ci/ci_lanes.py", "install-toolchain"))
        self.assertEqual(qemu[0].name, "codegen")
        paths = [s.argv[-1] for s in host + qemu if s in self.lanes.extended_plan()]
        self.assertCountEqual(paths, EXTENDED_PATHS)
        self.assertEqual(len(paths), len(set(paths)))
        self.assertIn("foundry-umbrella", [s.name for s in qemu])
        for i in (20, 21, 22, 26, 28, 31, 35):
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
        self.assertEqual(len(full), 52)
        self.assertEqual(full[2:2 + len(self.lanes.quality_plan())], self.lanes.quality_plan())
        self.assertEqual(full[-36:], self.lanes.extended_plan())
        for script, command in (("foundry_preflight.sh", "all"), ("foundry_ci_extended.sh", "extended")):
            source = (ROOT / "tools/ci" / script).read_text()
            self.assertIn(f"ci_lanes.py {command}", source)
            self.assertNotIn("cargo test", source)

    def test_stage_runner_serial_environment_and_exit_propagation(self):
        stages = self.lanes.extended_plan()[16:18]
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
        for results in itertools.product(states, repeat=3):
            self.assertEqual(allowed("success", "true", *results), results == ("success",) * 3)
            self.assertEqual(allowed("success", "false", *results), results == ("skipped",) * 3)
        for change in states[1:]:
            self.assertFalse(allowed(change, "true", "success", "success", "success"))
        for invalid in ("", "TRUE", "unknown"):
            self.assertFalse(allowed("success", invalid, "success", "success", "success"))
        self.assertFalse(POLICY.merge_allowed("success", "true", "skipped", "success"))

    def test_workflow_parallel_lanes_required_aggregate_and_safe_caches(self):
        source = (ROOT / ".github/workflows/ci.yml").read_text()
        import re
        jobs = dict(re.findall(r"^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)", source, re.M | re.S))
        self.assertTrue({"quality", "host-docker", "qemu", "foundry", "merge-gate"} <= jobs.keys())
        for job, lane in (("quality", "quality"), ("host-docker", "host"), ("qemu", "qemu")):
            self.assertIn("needs: changes", jobs[job])
            self.assertIn("needs.changes.outputs.os_code == 'true'", jobs[job])
            self.assertIn(f"ci_lanes.py lane {lane}", jobs[job])
            if job != "host-docker":
                self.assertIn("ci_lanes.py install-toolchain", jobs[job])
            self.assertIn("actions/upload-artifact@v4", jobs[job])
            self.assertIn("if: always()", jobs[job])
            self.assertIn("out/foundry/timings/", jobs[job])
            self.assertNotRegex(jobs[job], r"path:.*target")
        self.assertIn("needs: [changes, quality, host-docker, qemu]", jobs["foundry"])
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


if __name__ == "__main__":
    unittest.main(verbosity=2)
