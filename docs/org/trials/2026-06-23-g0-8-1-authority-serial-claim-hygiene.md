# G0.8.1 Authority and Serial Claim Hygiene Trial

**Trial date:** 2026-06-23
**Baseline:** G0.8 `PASS/PATCH`
**Trial agent:** current Codex implementation agent

Outcome: PASS/PATCH
authority_level: A2-local
serial_input_kind: enforced
unsafe_run_id_negative: pass
empty_transcript_negative: pass
context_expansion_request: none
implementation_performed: yes

## Work Order

- Work order: `WO-2026-06-23-g0-8-1-serial-observer-claim-hygiene`
- Task: S12.4.1 serial observer authority and evidence hygiene
- Authority: `A2-local`
- Claim level: `scaffold`
- Boundary: no merge, no release, no self-approval, no HIL actuation, and no public support authority

## Result

G0.8.1 reclassifies code-writing from A1 to A2-local. That means local code and
gate changes are allowed inside the active work order, while merge, release,
self-approval, HIL actuation, public support, credentials, and identity-level
role authority remain denied.

The serial observer now rejects unsafe `RAMEN_HIL_RUN_ID` values before using
them in paths. It rejects empty transcripts with `NO_SERIAL_BYTES`. It writes
`serial_input_kind` into the evidence JSON and uses distinct evidence levels:

- `RAMEN_HIL_SERIAL_LOG` development replay: `PASS/HIL-LOG`
- `RAMEN_HIL_SERIAL_DEV` live appliance capture: `PASS/HIL-APPLIANCE`

This is still not a `PASS/METAL` claim. Metal graduation remains dependent on
live appliance-mediated capture plus target-emitted `hil_evidence:` markers and
the relevant target gate evidence.

## Gate Results

- `just hil-appliance`: PASS
- `just foundry-org-governance-g0`: PASS
