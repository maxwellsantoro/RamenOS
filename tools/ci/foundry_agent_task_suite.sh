#!/usr/bin/env bash
# Shared SW0 sequence for full local preflight and CI. No platform skips.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
if [[ "$(uname -s)" != Linux ]]; then
  echo 'FOUNDRY_AGENT_TASK_SUITE: INCOMPLETE reason=Linux-required' >&2
  exit 1
fi
if ! python3 - <<'PY'
import sys
sys.path.insert(0, 'tools/agent_task')
import jsonschema
from linux_sandbox import Sandbox
Sandbox(readonly={}, writable={})
PY
then
  echo 'FOUNDRY_AGENT_TASK_SUITE: INCOMPLETE reason=requires-jsonschema-Docker-seccomp-and-installed-pinned-image' >&2
  exit 1
fi
if [[ "${1:-}" == --check ]]; then
  echo 'FOUNDRY_AGENT_TASK_SUITE: READY environment=linux'
  exit 0
fi
gates=(
  contract_a0 protocol_a1_0 proof_rt linux_control adapter lt
  ls_transactions subscriptions authority evaluator_controls reconciliation requestable_authority
)
for gate in "${gates[@]}"; do
  bash "tools/ci/foundry_agent_task_${gate}.sh"
done
echo 'FOUNDRY_AGENT_TASK_SUITE: PASS environment=linux claim=scripted-host-proof'
