#!/usr/bin/env python3
"""Exercise planning-owner routing through the public drift-check CLI."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from status_drift import REQUIRED_FILES


CHECKER = Path(__file__).with_name("status_drift.py")
ROUTING = """# Agent instructions
Read [landed evidence](CURRENT_STATUS.md), [ready work](NEXT_TASKS.md), and
[team workflow](docs/AGENTIC_WORKFLOW.md).
"""


class StatusDriftTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for relative in REQUIRED_FILES:
            self.write(relative, "")
        self.write("AGENTS.md", ROUTING)
        self.write("docs/AGENTIC_WORKFLOW.md", "# Team workflow\n")
        self.write("CURRENT_STATUS.md", "S12.4 HIL appliance physical work awaits setup.\n")
        self.write("NEXT_TASKS.md", """**Now:** Independent software work; HIL appliance awaits setup.
H0 serial observer
G0.8.1 implementation authority; RQ-0001; RQ-0002
""")
        self.write("ROADMAP.md", "G0 Org Kernel and Research Office\n")
        self.write("docs/org/current_task.yaml", """task_id: S12.4.1
next_gate: just hil-appliance
authority_level: A2
boundary: no merge, no release, no self-approval, no HIL actuation, and no public support authority
""")
        self.write("docs/research/slices/R-OFFERS-1-airlock-leakage-meter.md", "# R-OFFERS-1\n")
        self.write("docs/research/SLICE_NAMESPACING.md", "`S##` / `R-<PROGRAM>-<n>` / `G#`\n")

    def write(self, relative: str, text: str) -> None:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def check(self) -> tuple[int, dict]:
        result = subprocess.run(
            [sys.executable, str(CHECKER), "--root", str(self.root)],
            capture_output=True, text=True, check=False,
        )
        report = json.loads(result.stdout)
        return result.returncode, {
            "status": report["status"],
            "failures": [check for check in report["checks"] if not check["ok"]],
        }

    def test_routing_survives_independent_queue_changes(self) -> None:
        for task in ("authority controls", "input contract", "target loader"):
            with self.subTest(task=task):
                self.write("NEXT_TASKS.md", f"""**Now:** {task}
HIL appliance awaits setup; H0 serial observer
G0.8.1 implementation authority; RQ-0001; RQ-0002
""")
                code, report = self.check()
                self.assertEqual(code, 0, report)

    def test_missing_or_misdirected_owner_link_fails(self) -> None:
        for target in ("CURRENT_STATUS.md", "NEXT_TASKS.md", "docs/AGENTIC_WORKFLOW.md"):
            for replacement in ("", "archive/old-plan.md"):
                with self.subTest(target=target, replacement=replacement):
                    self.write("AGENTS.md", ROUTING.replace(f"({target})", f"({replacement})"))
                    code, report = self.check()
                    self.assertNotEqual(code, 0, report)

    def test_duplicated_agent_queue_fails_even_if_current(self) -> None:
        self.write("AGENTS.md", ROUTING + "\n- **Now:** S12.4 HIL appliance serial observer\n")
        code, report = self.check()
        self.assertNotEqual(code, 0, report)

    def test_missing_workflow_fails(self) -> None:
        (self.root / "docs/AGENTIC_WORKFLOW.md").unlink()
        code, report = self.check()
        self.assertNotEqual(code, 0, report)


if __name__ == "__main__":
    unittest.main()
