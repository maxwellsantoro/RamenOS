#!/usr/bin/env bash
# Foundry gate for G0 RamenOrg / research-backed project-control scaffold.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

GATE_ID="FOUNDRY_ORG_GOVERNANCE_G0"

fail() {
  echo "$GATE_ID: FAIL code=$1 detail=$2" >&2
  exit 1
}

require_file() {
  local path="$1"
  local code="$2"
  test -f "$path" || fail "$code" "missing required file: $path"
}

echo "=== RamenOrg Governance Gate (G0) ==="

require_file "docs/plans/2026-06-23-research-backed-ramenorg.md" "PLAN_MISSING"
require_file "docs/plans/2026-06-23-g0-1-board-packet-validators.md" "G0_1_PLAN_MISSING"
require_file "docs/plans/2026-06-23-g0-2-active-task-cross-packet.md" "G0_2_PLAN_MISSING"
require_file "docs/plans/2026-06-23-g0-3-current-task-negative-fixtures.md" "G0_3_PLAN_MISSING"
require_file "docs/plans/2026-06-23-g0-3-1-governance-label-claim-boundary.md" "G0_3_1_PLAN_MISSING"
require_file "docs/plans/2026-06-23-g0-4-read-only-steward-heartbeat.md" "G0_4_PLAN_MISSING"
require_file "docs/plans/2026-06-23-g0-5-agent-intake-freshness-binding.md" "G0_5_PLAN_MISSING"
require_file "docs/plans/2026-06-23-g0-6-intake-only-agent-trial.md" "G0_6_PLAN_MISSING"
require_file "docs/plans/2026-06-23-g0-7-bounded-context-grant.md" "G0_7_PLAN_MISSING"
require_file "docs/plans/2026-06-23-g0-8-bounded-implementation-trial.md" "G0_8_PLAN_MISSING"
require_file "docs/plans/2026-06-23-g0-8-1-implementation-authority-serial-claim-hygiene.md" "G0_8_1_PLAN_MISSING"
require_file "docs/org/current_task.yaml" "CURRENT_TASK_MISSING"
require_file "docs/org/CURRENT_TASK_V0.md" "CURRENT_TASK_DOC_MISSING"
require_file "docs/org/ORG_CONSTITUTION.md" "ORG_CONSTITUTION_MISSING"
require_file "docs/org/WORK_ORDER_V0.md" "WORK_ORDER_MISSING"
require_file "docs/org/HANDOFF_PACKET_V0.md" "HANDOFF_MISSING"
require_file "docs/org/BOARD_VOTE_V0.md" "BOARD_VOTE_MISSING"
require_file "docs/org/BOARD_PACKET_V0.md" "BOARD_PACKET_MISSING"
require_file "docs/org/BOARD_BRIEF_V0.md" "BOARD_BRIEF_DOC_MISSING"
require_file "docs/org/INTAKE_BUNDLE_V0.md" "INTAKE_BUNDLE_DOC_MISSING"
require_file "docs/org/CONTEXT_GRANT_V0.md" "CONTEXT_GRANT_DOC_MISSING"
require_file "docs/org/trials/2026-06-23-g0-6-intake-only-agent-trial.md" "G0_6_TRIAL_MISSING"
require_file "docs/org/trials/2026-06-23-g0-7-bounded-context-patch-plan.md" "G0_7_TRIAL_MISSING"
require_file "docs/org/trials/2026-06-23-g0-8-bounded-implementation-trial.md" "G0_8_TRIAL_MISSING"
require_file "docs/org/trials/2026-06-23-g0-8-1-authority-serial-claim-hygiene.md" "G0_8_1_TRIAL_MISSING"
require_file "docs/research/RESEARCH_PROGRAM.md" "RESEARCH_PROGRAM_MISSING"
require_file "docs/research/questions/RQ-0001-offer-boundaries.md" "RQ0001_MISSING"
require_file "docs/research/questions/RQ-0002-ai-org-kernel.md" "RQ0002_MISSING"
require_file "schemas/org/work_order_v0.schema.json" "WORK_ORDER_SCHEMA_MISSING"
require_file "schemas/org/handoff_packet_v0.schema.json" "HANDOFF_SCHEMA_MISSING"
require_file "schemas/org/board_vote_v0.schema.json" "BOARD_VOTE_SCHEMA_MISSING"
require_file "schemas/org/board_packet_v0.schema.json" "BOARD_PACKET_SCHEMA_MISSING"
require_file "schemas/org/current_task_v0.schema.json" "CURRENT_TASK_SCHEMA_MISSING"
require_file "schemas/org/intake_manifest_v0.schema.json" "INTAKE_MANIFEST_SCHEMA_MISSING"
require_file "schemas/org/context_grant_v0.schema.json" "CONTEXT_GRANT_SCHEMA_MISSING"
require_file "tools/org/render_board_packet.py" "BOARD_PACKET_RENDERER_MISSING"
require_file "tools/org/render_board_brief.py" "BOARD_BRIEF_RENDERER_MISSING"
require_file "tools/org/render_context_grant.py" "CONTEXT_GRANT_RENDERER_MISSING"
require_file "tools/org/validate_context_grant.py" "CONTEXT_GRANT_VALIDATOR_MISSING"
require_file "tools/org/validate_intake_manifest.py" "INTAKE_MANIFEST_VALIDATOR_MISSING"
require_file "tools/org/validate_packets.py" "PACKET_VALIDATOR_MISSING"
require_file "tools/org/test_validate_packets.py" "PACKET_VALIDATOR_TEST_MISSING"
require_file "tools/org/test_intake_bundle.py" "INTAKE_BUNDLE_TEST_MISSING"
require_file "tools/org/test_context_grant.py" "CONTEXT_GRANT_TEST_MISSING"
require_file "tools/org/status_drift.py" "STATUS_DRIFT_TOOL_MISSING"
require_file "docs/plans/2026-06-23-g0-9-first-a2-to-a3-loop.md" "G0_9_PLAN_MISSING"
require_file "docs/org/trials/2026-06-23-g0-9-first-a2-to-a3-loop.md" "G0_9_TRIAL_MISSING"
require_file "docs/org/HUMAN_DIRECTIVE_V0.md" "HUMAN_DIRECTIVE_DOC_MISSING"
require_file "docs/org/MERGE_GATE_V0.md" "MERGE_GATE_DOC_MISSING"
require_file "docs/org/RAMEN_IMPLEMENTER_BOT.md" "IMPLEMENTER_BOT_DOC_MISSING"
require_file "docs/research/SLICE_NAMESPACING.md" "SLICE_NAMESPACING_DOC_MISSING"
require_file "docs/research/slices/R-OFFERS-1-airlock-leakage-meter.md" "R_OFFERS_1_DOC_MISSING"
require_file "schemas/org/human_directive_v0.schema.json" "HUMAN_DIRECTIVE_SCHEMA_MISSING"
require_file "schemas/org/merge_request_v0.schema.json" "MERGE_REQUEST_SCHEMA_MISSING"
require_file "tools/org/validate_human_directive.py" "HUMAN_DIRECTIVE_VALIDATOR_MISSING"
require_file "tools/org/validate_merge.py" "MERGE_VALIDATOR_MISSING"
require_file "tools/org/render_g0_9.py" "G0_9_RENDERER_MISSING"
require_file "tools/org/test_validate_merge.py" "MERGE_VALIDATOR_TEST_MISSING"
require_file "tools/org/test_validate_human_directive.py" "HUMAN_DIRECTIVE_TEST_MISSING"

grep -q 'No ambient project authority' docs/org/ORG_CONSTITUTION.md \
  || fail "ORG_AMBIENT_AUTHORITY_GUARD_MISSING" "org constitution must reject ambient project authority"
grep -q 'No same-agent write, approve, merge, and announce path' docs/org/ORG_CONSTITUTION.md \
  || fail "ORG_SEPARATION_GUARD_MISSING" "org constitution must separate write/approve/merge/announce"
grep -q 'RamenOS is a research-backed OS' docs/research/INDEX.md \
  || fail "RESEARCH_BACKED_GUARD_MISSING" "research index must use research-backed OS framing"
grep -q 'Lang' docs/research/questions/RQ-0001-offer-boundaries.md \
  || fail "RQ0001_LANG_MISSING" "offer research question must track request authority"
grep -q 'ObsContract' docs/research/questions/RQ-0001-offer-boundaries.md \
  || fail "RQ0001_OBSCONTRACT_MISSING" "offer research question must track observable authority"
grep -q 'Handoff packets' docs/research/questions/RQ-0002-ai-org-kernel.md \
  || fail "RQ0002_HANDOFF_MISSING" "org kernel research question must track handoffs"

echo "$GATE_ID: INFO step=status_drift"
python3 tools/org/status_drift.py --root "$ROOT_DIR" --out out/org/status_drift.json

echo "$GATE_ID: INFO step=validate_current_task"
python3 tools/org/validate_packets.py --root "$ROOT_DIR" \
  --schema-dir schemas/org \
  --current-task docs/org/current_task.yaml \
  --current-task-schema schemas/org/current_task_v0.schema.json \
  --skip-packets \
  --out out/org/current_task_validation.json

echo "$GATE_ID: INFO step=render_context_grant"
python3 tools/org/render_context_grant.py --root "$ROOT_DIR" \
  --current-task docs/org/current_task.yaml \
  --out out/org/context_grant.json

echo "$GATE_ID: INFO step=validate_context_grant"
python3 tools/org/validate_context_grant.py --root "$ROOT_DIR" \
  --current-task docs/org/current_task.yaml \
  --schema schemas/org/context_grant_v0.schema.json \
  --grant out/org/context_grant.json

echo "$GATE_ID: INFO step=render_board_packet"
python3 tools/org/render_board_packet.py --root "$ROOT_DIR" \
  --current-task docs/org/current_task.yaml \
  --out-dir out/org/examples \
  --board-out out/org/current_board_packet.json

echo "$GATE_ID: INFO step=validate_packets"
python3 tools/org/validate_packets.py --root "$ROOT_DIR" \
  --schema-dir schemas/org \
  --packet-dir out/org/examples \
  --current-task docs/org/current_task.yaml \
  --current-task-schema schemas/org/current_task_v0.schema.json \
  --out out/org/packet_validation.json

echo "$GATE_ID: INFO step=render_board_brief"
python3 tools/org/render_board_brief.py --root "$ROOT_DIR" \
  --packet-dir out/org/examples \
  --validation-report out/org/packet_validation.json \
  --context-grant out/org/context_grant.json \
  --out out/org/current_board_brief.md \
  --manifest-out out/org/intake_manifest.json

require_file "out/org/current_board_brief.md" "BOARD_BRIEF_MISSING"
require_file "out/org/intake_manifest.json" "INTAKE_MANIFEST_MISSING"
grep -q '^## Active Task$' out/org/current_board_brief.md \
  || fail "BOARD_BRIEF_ACTIVE_TASK_MISSING" "board brief must include Active Task section"
grep -q '^## Authority Boundary$' out/org/current_board_brief.md \
  || fail "BOARD_BRIEF_AUTHORITY_BOUNDARY_MISSING" "board brief must include Authority Boundary section"
grep -q '^## Intake Binding$' out/org/current_board_brief.md \
  || fail "BOARD_BRIEF_INTAKE_BINDING_MISSING" "board brief must include Intake Binding section"
grep -q '^## Required Gates$' out/org/current_board_brief.md \
  || fail "BOARD_BRIEF_REQUIRED_GATES_MISSING" "board brief must include Required Gates section"
grep -q '^## Context Refs$' out/org/current_board_brief.md \
  || fail "BOARD_BRIEF_CONTEXT_REFS_MISSING" "board brief must include Context Refs section"
grep -q '^## Granted Context$' out/org/current_board_brief.md \
  || fail "BOARD_BRIEF_GRANTED_CONTEXT_MISSING" "board brief must include Granted Context section"
grep -q '^## Not Granted / Out of Scope$' out/org/current_board_brief.md \
  || fail "BOARD_BRIEF_NOT_GRANTED_MISSING" "board brief must include Not Granted / Out of Scope section"
grep -q '^## Evidence Refs$' out/org/current_board_brief.md \
  || fail "BOARD_BRIEF_EVIDENCE_REFS_MISSING" "board brief must include Evidence Refs section"
grep -q '^## Allowed Next-Agent Actions$' out/org/current_board_brief.md \
  || fail "BOARD_BRIEF_ALLOWED_ACTIONS_MISSING" "board brief must include Allowed Next-Agent Actions section"
grep -q 'no merge, no release, no HIL actuation, and no public support authority' out/org/current_board_brief.md \
  || grep -q 'no merge, no release, no self-approval, no HIL actuation, and no public support authority' out/org/current_board_brief.md \
  || fail "BOARD_BRIEF_AUTHORITY_DENIALS_MISSING" "board brief must preserve authority denials"

echo "$GATE_ID: INFO step=validate_intake_manifest"
python3 tools/org/validate_intake_manifest.py --root "$ROOT_DIR" \
  --schema schemas/org/intake_manifest_v0.schema.json \
  --manifest out/org/intake_manifest.json

echo "$GATE_ID: INFO step=negative_intake_freshness"
python3 tools/org/test_intake_bundle.py --root "$ROOT_DIR" \
  --packet-dir out/org/examples \
  --work-dir out/org/intake-negative

echo "$GATE_ID: INFO step=negative_context_grant_validation"
python3 tools/org/test_context_grant.py --root "$ROOT_DIR" \
  --grant out/org/context_grant.json \
  --work-dir out/org/context-grant-negative

echo "$GATE_ID: INFO step=negative_packet_validation"
python3 tools/org/test_validate_packets.py --root "$ROOT_DIR" \
  --packet-dir out/org/examples \
  --work-dir out/org/negative

echo "$GATE_ID: INFO step=validate_g0_6_intake_trial"
G0_6_TRIAL="docs/org/trials/2026-06-23-g0-6-intake-only-agent-trial.md"
grep -q '^Outcome: PASS/PLAN$' "$G0_6_TRIAL" \
  || fail "G0_6_OUTCOME_MISSING" "G0.6 trial must record PASS/PLAN outcome"
grep -q '^fork_context: false$' "$G0_6_TRIAL" \
  || fail "G0_6_ISOLATION_MISSING" "G0.6 trial must record isolated agent context"
grep -q '^hidden_chat_context_needed: no$' "$G0_6_TRIAL" \
  || fail "G0_6_HIDDEN_CONTEXT_RESULT_MISSING" "G0.6 trial must record hidden chat result"
grep -q '^external_files_read: none$' "$G0_6_TRIAL" \
  || fail "G0_6_EXTERNAL_READ_RESULT_MISSING" "G0.6 trial must record external file reads"
grep -q '^Patch readiness: blocked_by_absent_referenced_context$' "$G0_6_TRIAL" \
  || fail "G0_6_PATCH_FINDING_MISSING" "G0.6 trial must preserve the patch-context finding"
grep -q 'WO-2026-06-23-s12-4-1-serial-observer' "$G0_6_TRIAL" \
  || fail "G0_6_WORK_ORDER_MISSING" "G0.6 trial must recover the work-order id"
grep -q 'Authority level: `A1`' "$G0_6_TRIAL" \
  || fail "G0_6_AUTHORITY_LEVEL_MISSING" "G0.6 trial must recover A1 authority"
grep -q 'no merge, no release, no HIL actuation, and no public support authority' "$G0_6_TRIAL" \
  || fail "G0_6_AUTHORITY_BOUNDARY_MISSING" "G0.6 trial must preserve authority denials"
grep -q 'just hil-appliance' "$G0_6_TRIAL" \
  || fail "G0_6_HIL_GATE_MISSING" "G0.6 trial must recover the HIL appliance gate"
grep -q 'just foundry-org-governance-g0' "$G0_6_TRIAL" \
  || fail "G0_6_GOVERNANCE_GATE_MISSING" "G0.6 trial must recover the governance gate"
G0_6_SUPPLIED_COUNT="$(awk '
  /^## Supplied Artifacts$/ { supplied = 1; next }
  supplied && /^## / { supplied = 0 }
  supplied && /^- `out\/org/ { count += 1 }
  END { print count + 0 }
' "$G0_6_TRIAL")"
test "$G0_6_SUPPLIED_COUNT" -eq 6 \
  || fail "G0_6_SUPPLIED_COUNT_INVALID" "G0.6 trial must record exactly six supplied artifacts"
for ref in \
  out/org/current_board_brief.md \
  out/org/intake_manifest.json \
  out/org/examples/board_packet_s12_4_1_serial_observer.json \
  out/org/examples/work_order_s12_4_1_serial_observer.json \
  out/org/examples/handoff_planner_to_implementer_s12_4_1_serial_observer.json \
  out/org/examples/board_vote_foundry_evidence_s12_4_1_serial_observer.json; do
  grep -q "$ref" "$G0_6_TRIAL" \
    || fail "G0_6_SUPPLIED_REF_MISSING" "G0.6 trial must cite supplied artifact: $ref"
done

echo "$GATE_ID: INFO step=validate_g0_7_patch_plan_trial"
G0_7_TRIAL="docs/org/trials/2026-06-23-g0-7-bounded-context-patch-plan.md"
require_file "$G0_7_TRIAL" "G0_7_TRIAL_MISSING"
grep -q '^Outcome: PASS/PATCH-PLAN$' "$G0_7_TRIAL" \
  || fail "G0_7_OUTCOME_MISSING" "G0.7 trial must record PASS/PATCH-PLAN outcome"
grep -q '^fork_context: false$' "$G0_7_TRIAL" \
  || fail "G0_7_ISOLATION_MISSING" "G0.7 trial must record isolated agent context"
grep -q '^hidden_chat_context_needed: no$' "$G0_7_TRIAL" \
  || fail "G0_7_HIDDEN_CONTEXT_RESULT_MISSING" "G0.7 trial must record hidden chat result"
grep -q '^external_files_read: none$' "$G0_7_TRIAL" \
  || fail "G0_7_EXTERNAL_READ_RESULT_MISSING" "G0.7 trial must record external file reads"
grep -q '^implementation_performed: no$' "$G0_7_TRIAL" \
  || fail "G0_7_IMPLEMENTATION_BOUNDARY_MISSING" "G0.7 trial must remain plan-only"
grep -q '^context_sufficiency: sufficient$' "$G0_7_TRIAL" \
  || fail "G0_7_CONTEXT_SUFFICIENCY_MISSING" "G0.7 trial must prove context sufficiency"
grep -q '^context_expansion_request: none$' "$G0_7_TRIAL" \
  || fail "G0_7_EXPANSION_PROTOCOL_MISSING" "G0.7 trial must record no expansion request"
grep -q 'tools/hil/appliance_capture_serial.sh' "$G0_7_TRIAL" \
  || fail "G0_7_AUTHORIZED_NEW_PATH_MISSING" "G0.7 trial must cite the authorized new path"
G0_7_SUPPLIED_COUNT="$(awk '
  /^## Supplied Artifacts$/ { supplied = 1; next }
  supplied && /^## / { supplied = 0 }
  supplied && /^- `/ { count += 1 }
  END { print count + 0 }
' "$G0_7_TRIAL")"
test "$G0_7_SUPPLIED_COUNT" -eq 15 \
  || fail "G0_7_SUPPLIED_COUNT_INVALID" "G0.7 trial must record exactly fifteen supplied artifacts"

echo "$GATE_ID: INFO step=validate_g0_8_implementation_trial"
G0_8_TRIAL="docs/org/trials/2026-06-23-g0-8-bounded-implementation-trial.md"
grep -q '^Outcome: PASS/PATCH$' "$G0_8_TRIAL" \
  || fail "G0_8_OUTCOME_MISSING" "G0.8 trial must record PASS/PATCH outcome"
grep -q '^implementation_performed: yes$' "$G0_8_TRIAL" \
  || fail "G0_8_IMPLEMENTATION_RESULT_MISSING" "G0.8 trial must record implementation"
grep -q '^context_sufficiency: sufficient$' "$G0_8_TRIAL" \
  || fail "G0_8_CONTEXT_SUFFICIENCY_MISSING" "G0.8 trial must record sufficient context"
grep -q '^context_expansion_request: none$' "$G0_8_TRIAL" \
  || fail "G0_8_EXPANSION_PROTOCOL_MISSING" "G0.8 trial must record no expansion request"
grep -q '^external_files_read: none_beyond_grant_for_implementation_surface$' "$G0_8_TRIAL" \
  || fail "G0_8_EXTERNAL_READ_BOUNDARY_MISSING" "G0.8 trial must record implementation-surface read boundary"
grep -q 'tools/hil/appliance_capture_serial.sh' "$G0_8_TRIAL" \
  || fail "G0_8_SERIAL_OBSERVER_MISSING" "G0.8 trial must cite the serial observer script"
grep -q 'tools/ci/foundry_hil_appliance_s12_4.sh' "$G0_8_TRIAL" \
  || fail "G0_8_HIL_GATE_MISSING" "G0.8 trial must cite the HIL appliance gate"
grep -q 'no merge, no release, no HIL actuation, and no public support authority' "$G0_8_TRIAL" \
  || grep -q 'no merge, no release, no self-approval, no HIL actuation, and no public support authority' "$G0_8_TRIAL" \
  || fail "G0_8_AUTHORITY_BOUNDARY_MISSING" "G0.8 trial must preserve authority denials"
grep -q '`just hil-appliance`: PASS' "$G0_8_TRIAL" \
  || fail "G0_8_HIL_APPLIANCE_RESULT_MISSING" "G0.8 trial must record hil-appliance gate result"
grep -q '`just foundry-org-governance-g0`: PASS' "$G0_8_TRIAL" \
  || fail "G0_8_GOVERNANCE_RESULT_MISSING" "G0.8 trial must record governance gate result"
grep -q 'This is not a metal graduation claim' "$G0_8_TRIAL" \
  || fail "G0_8_METAL_CLAIM_GUARD_MISSING" "G0.8 trial must reject metal graduation claim"

echo "$GATE_ID: INFO step=validate_g0_8_1_hygiene_trial"
G0_8_1_TRIAL="docs/org/trials/2026-06-23-g0-8-1-authority-serial-claim-hygiene.md"
grep -q '^Outcome: PASS/PATCH$' "$G0_8_1_TRIAL" \
  || fail "G0_8_1_OUTCOME_MISSING" "G0.8.1 trial must record PASS/PATCH outcome"
grep -q '^authority_level: A2-local$' "$G0_8_1_TRIAL" \
  || fail "G0_8_1_AUTHORITY_MISSING" "G0.8.1 trial must record A2-local authority"
grep -q '^serial_input_kind: enforced$' "$G0_8_1_TRIAL" \
  || fail "G0_8_1_INPUT_KIND_MISSING" "G0.8.1 trial must record serial_input_kind enforcement"
grep -q '^unsafe_run_id_negative: pass$' "$G0_8_1_TRIAL" \
  || fail "G0_8_1_RUN_ID_NEGATIVE_MISSING" "G0.8.1 trial must record unsafe run id negative"
grep -q '^empty_transcript_negative: pass$' "$G0_8_1_TRIAL" \
  || fail "G0_8_1_EMPTY_NEGATIVE_MISSING" "G0.8.1 trial must record empty transcript negative"
grep -q 'PASS/HIL-LOG' "$G0_8_1_TRIAL" \
  || fail "G0_8_1_REPLAY_LEVEL_MISSING" "G0.8.1 trial must distinguish replay evidence level"
grep -q 'PASS/HIL-APPLIANCE' "$G0_8_1_TRIAL" \
  || fail "G0_8_1_LIVE_LEVEL_MISSING" "G0.8.1 trial must preserve live evidence level"
grep -q 'no merge, no release, no self-approval, no HIL actuation, and no public support authority' "$G0_8_1_TRIAL" \
  || fail "G0_8_1_AUTHORITY_BOUNDARY_MISSING" "G0.8.1 trial must preserve authority denials"

echo "$GATE_ID: INFO step=render_g0_9"
python3 tools/org/render_g0_9.py --root "$ROOT_DIR"

echo "$GATE_ID: INFO step=validate_g0_9_packets"
python3 tools/org/validate_packets.py --root "$ROOT_DIR" \
  --schema-dir schemas/org \
  --packet-dir out/org/examples-g0-9 \
  --out out/org/g0_9_packet_validation.json

echo "$GATE_ID: INFO step=validate_human_directive"
python3 tools/org/validate_human_directive.py --root "$ROOT_DIR" \
  --directive out/org/human_directive_proceed_with_all.json \
  --out out/org/human_directive_validation.json

echo "$GATE_ID: INFO step=negative_human_directive_validation"
python3 tools/org/test_validate_human_directive.py --root "$ROOT_DIR" \
  --work-dir out/org/directive-negative

echo "$GATE_ID: INFO step=validate_merge_gate"
python3 tools/org/validate_merge.py --root "$ROOT_DIR" \
  --merge-request out/org/examples-g0-9/merge_request_g0_9_slice_namespacing.json \
  --out out/org/merge_validation.json

echo "$GATE_ID: INFO step=negative_merge_validation"
python3 tools/org/test_validate_merge.py --root "$ROOT_DIR" \
  --work-dir out/org/merge-negative

echo "$GATE_ID: INFO step=validate_g0_9_loop_trial"
G0_9_TRIAL="docs/org/trials/2026-06-23-g0-9-first-a2-to-a3-loop.md"
grep -q '^Outcome: PASS/LOOP-LOCAL$' "$G0_9_TRIAL" \
  || fail "G0_9_OUTCOME_MISSING" "G0.9 trial must record PASS/LOOP-LOCAL outcome"
grep -q '^authority_level: A2-local$' "$G0_9_TRIAL" \
  || fail "G0_9_AUTHORITY_MISSING" "G0.9 trial must record A2-local authority"
grep -q '^separation_of_duties: enforced$' "$G0_9_TRIAL" \
  || fail "G0_9_SEPARATION_MISSING" "G0.9 trial must record enforced separation of duties"
grep -q '^research_blocks_implementation: enforced$' "$G0_9_TRIAL" \
  || fail "G0_9_RESEARCH_BLOCK_MISSING" "G0.9 trial must record enforced research-blocks-implementation"
grep -q '^loop_closure: local$' "$G0_9_TRIAL" \
  || fail "G0_9_LOOP_CLOSURE_MISSING" "G0.9 trial must record local loop closure"
grep -q '^remote_merge_precondition: pending$' "$G0_9_TRIAL" \
  || fail "G0_9_REMOTE_PRECONDITION_MISSING" "G0.9 trial must record pending remote merge precondition"
grep -q 'MR-2026-06-23-g0-9-slice-namespacing' "$G0_9_TRIAL" \
  || fail "G0_9_MERGE_REQUEST_MISSING" "G0.9 trial must cite the merge request"
grep -q 'no merge, no release, no self-approval, no HIL actuation, and no public support authority' "$G0_9_TRIAL" \
  || fail "G0_9_AUTHORITY_BOUNDARY_MISSING" "G0.9 trial must preserve authority denials"
grep -q 'PASS/LOOP-LOCAL' "$G0_9_TRIAL" \
  || fail "G0_9_LOOP_LOCAL_MISSING" "G0.9 trial must distinguish LOOP-LOCAL evidence level"
grep -q 'PASS/MERGE` claim' "$G0_9_TRIAL" \
  || fail "G0_9_MERGE_CLAIM_GUARD_MISSING" "G0.9 trial must reject a PASS/MERGE overclaim"

grep -q 'requires_rq' docs/research/slices/R-OFFERS-1-airlock-leakage-meter.md \
  || fail "R_OFFERS_1_REQUIRES_RQ_MISSING" "R-OFFERS-1 must bind requires_rq"
grep -q 'R-OFFERS-1' docs/research/SLICE_NAMESPACING.md \
  || fail "SLICE_NAMESPACING_R_OFFERS_1_MISSING" "namespacing doc must allocate R-OFFERS-1"

echo "$GATE_ID: METRIC status_drift=pass"
echo "$GATE_ID: METRIC current_task_validation=pass"
echo "$GATE_ID: METRIC context_grant=pass"
echo "$GATE_ID: METRIC context_grant_negative=pass"
echo "$GATE_ID: METRIC packet_validation=pass"
echo "$GATE_ID: METRIC board_brief=pass"
echo "$GATE_ID: METRIC intake_manifest=pass"
echo "$GATE_ID: METRIC intake_freshness_negative=pass"
echo "$GATE_ID: METRIC negative_packet_validation=pass"
echo "$GATE_ID: METRIC intake_only_agent_trial=pass_plan"
echo "$GATE_ID: METRIC bounded_context_trial=pass_patch_plan"
echo "$GATE_ID: METRIC bounded_implementation_trial=pass_patch"
echo "$GATE_ID: METRIC authority_serial_claim_hygiene=pass_patch"
echo "$GATE_ID: METRIC g0_9_packets=pass"
echo "$GATE_ID: METRIC human_directive=pass"
echo "$GATE_ID: METRIC human_directive_negative=pass"
echo "$GATE_ID: METRIC merge_gate=pass"
echo "$GATE_ID: METRIC merge_gate_negative=pass"
echo "$GATE_ID: METRIC first_a2_to_a3_loop=pass_loop_local"
echo "$GATE_ID: PASS scaffold"
echo "$GATE_ID: ok"
