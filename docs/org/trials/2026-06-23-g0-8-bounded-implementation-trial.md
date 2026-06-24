# G0.8 Bounded Implementation Trial

**Trial date:** 2026-06-23
**Baseline:** G0.7 `PASS/PATCH-PLAN`
**Trial agent:** current Codex implementation agent

Outcome: PASS/PATCH
fork_context: false
context_sufficiency: sufficient
context_expansion_request: none
hidden_chat_context_needed: no
external_files_read: none_beyond_grant_for_implementation_surface
implementation_performed: yes

## Work Order

- Work order: `WO-2026-06-23-g0-8-s12-4-1-serial-observer`
- Task: S12.4.1 HIL appliance serial observer scaffold
- Authority: `A2-local` (reclassified by G0.8.1)
- Claim level: `scaffold`
- Boundary: no merge, no release, no self-approval, no HIL actuation, and no public support authority

## Implementation Surface

The S12.4.1 implementation patch touched:

- `tools/hil/appliance_capture_serial.sh`
- `tools/ci/foundry_hil_appliance_s12_4.sh`

Project-control files were updated separately to record the G0.8 work order,
trial evidence, and gate checks. Those updates are not part of the bounded
S12.4.1 implementation-surface claim. No hardware actuation script was added or
run.

## Result

The serial observer scaffold now:

- allocates a HIL appliance run id;
- captures from `RAMEN_HIL_SERIAL_DEV` or copies `RAMEN_HIL_SERIAL_LOG` for
  development replay only;
- rejects `RAMEN_HIL_GRADUATION=1` when paired with `RAMEN_HIL_SERIAL_LOG`;
- archives `*.serial.log`, `*.controller.log`, and `*.json` under
  `out/evidence/`;
- scans `RAMEN OS`, `golden_machine:*`, `persistent_storage:*`, and
  `hil_evidence:*` markers;
- emits wrapper evidence with `evidence_kind: hil_appliance_run_v0`; G0.8.1
  later split development replay to `PASS/HIL-LOG` and live capture to
  `PASS/HIL-APPLIANCE`.

## Gate Results

- `just hil-appliance`: PASS
- `just foundry-org-governance-g0`: PASS

## Findings

G0.8 proves the bounded context flow can carry a small implementation patch
when the missing output path from G0.7 has become a concrete file. G0.8.1
corrects the authority label to A2-local. After the file exists, it must move
from `authorized_new_paths` into hash-bound granted context so the intake
manifest stays fresh and reproducible.

This is not a metal graduation claim. Live target proof still requires
appliance-mediated capture with target-emitted `hil_evidence:` markers, and
stale log replay remains development-only.
