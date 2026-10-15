#!/usr/bin/env python3
"""Fail-closed controls for preparation-only missing-API diagnostic admission."""
import json
from pathlib import Path
import unittest

import editor_process_preparation as preparation


TYPES = {
    "ChildBudget", "OwnedEditorChild", "OwnedEditorChildExitProof", "ProcessRetireReason",
    "ProcessLaunchRecovery", "ProcessWaitOutcome", "ProcessEditorEnrollment",
}
METHODS = {
    "prepare_process_editor", "registered_editor_process_executable", "spawn_process_editor",
    "pump_process_editor", "join_process_editor", "observe_process_child",
    "observe_process_editor", "take_process_save_intent", "request_process_editor_frame",
    "retire_process_editor",
}
SOURCE = "apps/artifact_editor/tests/process_task.rs"


def diagnostic(code, message, source=SOURCE):
    return {"reason": "compiler-message", "message": {
        "level": "error", "code": {"code": code}, "message": message,
        "spans": [{"file_name": source, "is_primary": True}], "children": [],
    }}


def complete_errors():
    result = [diagnostic("E0432", "unresolved imports " + ", ".join(
        f"`desktop_service::editor_dev::{symbol}`" for symbol in sorted(TYPES)))]
    result.extend(diagnostic("E0599", f"no method named `{symbol}` found for struct `desktop_service::editor_dev::HostDesktop` in the current scope")
                  for symbol in sorted(METHODS))
    return result


def cargo_output(errors, success=False):
    return "\n".join(json.dumps(row) for row in [*errors, {"reason": "build-finished", "success": success}])


class MissingApiPreparationControls(unittest.TestCase):
    def classify(self, errors=None, exit_code=101, success=False):
        return preparation.classify_missing_api(
            cargo_output(complete_errors() if errors is None else errors, success), exit_code,
            Path("/fixture"))

    def test_known_missing_api_is_preparation_only(self):
        result = self.classify()
        self.assertEqual(set(result["missing_types"]), TYPES)
        self.assertEqual(set(result["missing_methods"]), METHODS)
        self.assertEqual(result["inference_cascade_count"], 0)
        self.assertFalse(result["runtime_executed"])
        self.assertFalse(result["all30_complete"])

    def test_unrelated_compiler_failure_is_rejected(self):
        with self.assertRaises(preparation.PreparationError):
            self.classify(complete_errors() + [diagnostic("E0308", "mismatched types")])

    def test_unexpected_source_is_rejected(self):
        errors = complete_errors()
        errors[0]["message"]["spans"][0]["file_name"] = "services/desktop/src/editor_dev/host.rs"
        with self.assertRaises(preparation.PreparationError):
            self.classify(errors)

    def test_unexpected_method_is_rejected(self):
        with self.assertRaises(preparation.PreparationError):
            self.classify(complete_errors() + [diagnostic("E0599", "no method named `fake_child` found for struct `HostDesktop`")])

    def test_unexpected_type_is_rejected(self):
        with self.assertRaises(preparation.PreparationError):
            self.classify(complete_errors() + [diagnostic("E0433", "failed to resolve: use of undeclared type `Unrelated`")])

    def test_success_is_not_missing_api_red(self):
        with self.assertRaises(preparation.PreparationError):
            self.classify(exit_code=0, success=True)

    def test_missing_required_method_is_rejected(self):
        errors = [row for row in complete_errors() if "`join_process_editor`" not in row["message"]["message"]]
        with self.assertRaises(preparation.PreparationError):
            self.classify(errors)

    def test_inference_cascade_is_separate_and_unproved(self):
        result = self.classify(complete_errors() + [diagnostic("E0282", "type annotations needed")])
        self.assertEqual(result["inference_cascade_count"], 1)
        self.assertEqual(result["inference_cascades"][0]["code"], "E0282")
        self.assertEqual(result["inference_cascades"][0]["status"], "retained-unproved")

    def test_cascade_without_missing_api_evidence_is_rejected(self):
        with self.assertRaises(preparation.PreparationError):
            self.classify([diagnostic("E0282", "type annotations needed")])

    def test_missing_primary_location_is_rejected(self):
        errors = complete_errors()
        errors[0]["message"]["spans"] = []
        with self.assertRaises(preparation.PreparationError):
            self.classify(errors)

    def test_malformed_or_unfinished_cargo_output_is_rejected(self):
        for output in ["", "not JSON", json.dumps(complete_errors()[0]),
                       cargo_output(complete_errors()) + "\n{}"]:
            with self.subTest(output=output), self.assertRaises(preparation.PreparationError):
                preparation.classify_missing_api(output, 101, Path("/fixture"))

    def test_build_success_with_failing_exit_is_rejected(self):
        with self.assertRaises(preparation.PreparationError):
            self.classify(success=True)

    def test_absolute_allowed_locations_are_admitted(self):
        errors = complete_errors()
        for error in errors:
            error["message"]["spans"][0]["file_name"] = "/fixture/" + SOURCE
        self.assertEqual(self.classify(errors)["missing_api_error_count"], len(errors))

    def test_allowed_codes_do_not_admit_unrelated_name_errors(self):
        for code, message in [("E0425", "cannot find value `ChildBudget` in this scope"),
                              ("E0432", "unresolved import `unrelated::Accidental`"),
                              ("E0599", "no associated item named `spawn_process_editor` found for struct `Other`")]:
            with self.subTest(code=code), self.assertRaises(preparation.PreparationError):
                self.classify(complete_errors() + [diagnostic(code, message)])

    def test_right_symbol_from_wrong_provider_is_rejected(self):
        for code, message in [
            ("E0432", "unresolved import `other::ChildBudget`"),
            ("E0433", "failed to resolve: could not find `ProcessLaunchRecovery` in `other`"),
            ("E0425", "cannot find type `ProcessEditorEnrollment` in module `other`"),
        ]:
            with self.subTest(code=code), self.assertRaises(preparation.PreparationError):
                self.classify(complete_errors() + [diagnostic(code, message)])

    def test_right_method_on_wrong_receiver_is_rejected(self):
        for receiver in ["Other", "other::HostDesktop", "ChildBudget"]:
            message = f"no method named `spawn_process_editor` found for struct `{receiver}` in the current scope"
            with self.subTest(receiver=receiver), self.assertRaises(preparation.PreparationError):
                self.classify(complete_errors() + [diagnostic("E0599", message)])

    def test_frozen_module_alias_diagnostics_are_admitted(self):
        errors = complete_errors() + [
            diagnostic("E0433", "failed to resolve: could not find `ProcessLaunchRecovery` in `desktop`"),
            diagnostic("E0425", "cannot find type `ProcessEditorEnrollment` in module `desktop`"),
        ]
        self.assertEqual(self.classify(errors)["missing_api_error_count"], len(errors))

    def test_standard_consistent_failure_notes_are_retained(self):
        errors = complete_errors() + [diagnostic("E0282", "type annotations needed")]
        errors += [failure_note("Some errors have detailed explanations: E0282, E0432, E0599."),
                   failure_note("For more information about an error, try `rustc --explain E0282`.")]
        result = self.classify(errors)
        self.assertEqual(len(result["compiler_failure_notes"]), 2)

    def test_unrelated_or_inconsistent_failure_notes_are_rejected(self):
        for text in ["could not execute compiler", "Some errors have detailed explanations: E0308, E0432, E0599.",
                     "For more information about an error, try `rustc --explain E0308`.",
                     "Some errors have detailed explanations: E0432."]:
            with self.subTest(note=text), self.assertRaises(preparation.PreparationError):
                self.classify(complete_errors() + [failure_note(text)])


class PreparationInventoryControls(unittest.TestCase):
    def summary(self):
        names = sorted(preparation.SETUP_CASES)
        return {"status": "PASS", "package": "artifact_editor", "targets": [{
            "name": "process_setup_control", "kind": "test", "selected": names, "passed": names,
        }], "commands": [{"exit_code": 0, "reaped": True, "group_stop_before_reap": True}]}

    def test_exact_executed_setup_inventory(self):
        self.assertEqual(len(preparation.validate_setup(self.summary())), 6)
        self.assertEqual(len(preparation.AUTHORED_CASES), 4)

    def test_missing_or_foreign_executed_case_is_rejected(self):
        for field in ("selected", "passed"):
            for alter in (lambda names: names[:-1], lambda names: names + ["foreign"]):
                summary = self.summary()
                summary["targets"][0][field] = alter(summary["targets"][0][field])
                with self.subTest(field=field), self.assertRaises(preparation.PreparationError):
                    preparation.validate_setup(summary)

    def test_uncertain_setup_exit_is_rejected(self):
        for field, value in (("reaped", False), ("group_stop_before_reap", False), ("exit_code", 101)):
            summary = self.summary()
            summary["commands"][0][field] = value
            with self.subTest(field=field), self.assertRaises(preparation.PreparationError):
                preparation.validate_setup(summary)

    def test_hosted_preparation_and_evidence_are_separate_from_canonical_inventory(self):
        workflow = (preparation.ROOT / ".github/workflows/ci.yml").read_text()
        self.assertIn("--directory out/editor-process-preparation/timings --label editor-process-preparation -- just foundry-editor-process-preparation-ui1-1d", workflow)
        self.assertIn("name: editor-process-preparation-${{ github.run_id }}-${{ github.run_attempt }}", workflow)
        self.assertIn("path: out/editor-process-preparation/", workflow)


def failure_note(message):
    return {"reason": "compiler-message", "message": {
        "level": "failure-note", "code": None, "message": message, "spans": [], "children": [],
    }}


if __name__ == "__main__":
    unittest.main()
