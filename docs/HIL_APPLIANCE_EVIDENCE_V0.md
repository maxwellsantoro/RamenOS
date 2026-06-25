# HIL Appliance Evidence V0

**Last Updated:** 2026-06-24
**Status:** Scaffold schema for S12.4 / S13.9
**Gate:** `tools/ci/foundry_hil_appliance_s12_4.sh`

The HIL appliance evidence object is a **wrapper** around a physical run. It records what the Raspberry Pi-class controller observed and actuated. It does not replace target-emitted `hil_evidence:` markers or per-gate evidence JSON.

## Naming

Canonical names:

| Context | Name |
|---------|------|
| Human-facing long name | HIL Appliance Controller |
| Human-facing short name | HIL appliance |
| File/env/code prefix | `hil_appliance` / `RAMEN_HIL_APPLIANCE` |
| Slice label | S12.4 / S13.9 HIL Appliance Controller |

Avoid inventing parallel names such as "lab KVM" or "Pi oracle" in scripts or gates. The appliance is not an oracle; it is an observer/actuator.

## Relationship to per-gate evidence

Appliance evidence is a supervisor envelope:

```text
hil_appliance_<run_id>.json
  -> references s13_7_nvme_boot_evidence.json
  -> references s13_8_atomic_update_evidence.json
  -> references serial/controller/video artifacts by hash
```

Per-gate evidence remains claim-bearing. Appliance evidence records the run context and correlates logs, power events, and controller observations. See `EVIDENCE_LEVELS.md` for the canonical claim-safety language.

Per-gate HIL evidence also carries `claim_path`. Appliance wrapper evidence uses
`PASS/HIL-APPLIANCE`; per-gate S13 metal evidence uses `PASS/METAL` with
`claim_path: appliance-mediated` when the appliance is present, or
`claim_path: operator-golden-machine` for standalone live golden-machine
graduation.

## Required JSON shape

```json
{
  "schema_version": 1,
  "evidence_kind": "hil_appliance_run_v0",
  "evidence_level": "PASS/HIL-APPLIANCE",
  "claim_path": "appliance-mediated",
  "run_id": "hil_appliance_20260622T131700Z_pi-hil-01_s13-hil",
  "appliance_id": "pi-hil-01",
  "target_id": "intel-nuc-12-reference",
  "git_sha": "unknown",
  "gate": "s13-hil",
  "started_at_unix_ms": 0,
  "ended_at_unix_ms": 0,
  "serial_device": "/dev/ttyUSB0",
  "serial_input_kind": "live_device",
  "serial_log": "out/evidence/hil_appliance_<run_id>.serial.log",
  "serial_log_sha256": "unknown",
  "controller_log": "out/evidence/hil_appliance_<run_id>.controller.log",
  "controller_log_sha256": "unknown",
  "power_events": [
    {
      "kind": "press_power",
      "started_at_unix_ms": 0,
      "duration_ms": 500,
      "result": "ok"
    }
  ],
  "artifact_hashes": {
    "kernel_efi_sha256": "unknown",
    "init_img_sha256": "unknown"
  },
  "serial_markers_observed": [
    "RAMEN OS"
  ],
  "target_hil_evidence_markers": {
    "git_sha": "unknown",
    "init_profile": "unknown",
    "machine_id": "unknown",
    "storage_manifest_sha256": "unknown",
    "kernel_efi_sha256": "unknown",
    "init_img_sha256": "unknown",
    "boot_epoch_nonce": "unknown"
  },
  "gate_evidence": [
    "out/evidence/s13_7_nvme_boot_evidence.json",
    "out/evidence/s13_8_atomic_update_evidence.json"
  ],
  "result": "pass"
}
```

## Validation doctrine

A valid appliance bundle must satisfy all of these:

1. It identifies the appliance and target.
2. It records timestamps for the run and any power/reset events.
3. It records serial and controller artifacts by path and SHA-256 when present.
4. It references, rather than replaces, per-gate evidence files.
5. In graduation mode, it includes target-emitted `hil_evidence:` markers parsed from the live serial log.
6. It never upgrades a stale operator-provided log into `PASS/METAL`.
7. It distinguishes `serial_input_kind: development_log` from
   `serial_input_kind: live_device`; development-log replay uses `PASS/HIL-LOG`
   and live appliance serial uses `PASS/HIL-APPLIANCE`.

The S12.4 scaffold gate validates that this schema exists, is referenced by `hardware/hil_appliance_v0.toml`, and remains a wrapper over per-gate evidence.

## Claim safety

Safe claim:

> The HIL appliance captured a live target run and correlated controller-side events with target-emitted evidence markers.

Unsafe claim:

> The HIL appliance proves the target state independently.

The target proves target claims by emitting provenance markers and passing the relevant Foundry gate. The appliance proves the lab loop was live, observable, and reproducible.
