#!/usr/bin/env bash
# Portable offline accounting contract; no provider calls or funded-run authority.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
EVIDENCE="$PWD/out/agent-task-provider-accounting"
mkdir -p "$EVIDENCE"
rm -f "$EVIDENCE/report.json"
python3 tools/agent_task/test_provider_accounting.py
python3 tools/agent_task/provider_accounting_gate.py --evidence "$EVIDENCE"
echo 'FOUNDRY_AGENT_TASK_PROVIDER_ACCOUNTING: PASS scope=offline-synthetic-usage-estimates provider_calls=false'
