# S12.4 / S13.9: HIL Appliance Controller

**Last Updated:** 2026-06-24
**Status:** Active; S12.4.1 serial observer first, then S12.4.2 power/reset
**Gate:** `tools/ci/foundry_hil_appliance_s12_4.sh`
**Related:** `hardware/hil_appliance_v0.toml`, `hardware/golden_machine_v0.toml`, `EVIDENCE_LEVELS.md`, `docs/HIL_APPLIANCE_EVIDENCE_V0.md`, `docs/plans/2026-06-21-s12-golden-machine-design.md`, `docs/plans/2026-06-21-s13-persistent-storage-design.md`

---

## Executive summary

RamenOS needs a stable physical test appliance between agents and the sacrificial golden machine.

The HIL appliance is an always-on Raspberry Pi-class controller that provides serial observation, power/reset control, evidence capture, and later KVM-grade video/HID/virtual-media control. It exists to remove human reboot/cable/manual-log work from bare-metal development so agents can iterate against physical hardware safely and repeatably.

This is **not** part of the RamenOS target TCB. It is lab infrastructure. Its job is to make target evidence reproducible, timestamped, and machine-readable.

---

## 0. Doctrine

**The golden machine is allowed to hang, corrupt its boot state, panic, or become temporarily unbootable. The HIL appliance must remain stable.**

Therefore:

- the Raspberry Pi/controller is the observer and actuator;
- the golden machine is the target under test;
- the build host may be separate from both, usually the Ryzen/Linux workstation;
- the controller records evidence, not truth by assertion;
- metal graduation requires fresh live capture, not stale copied serial logs.

Target shape:

```text
build host / agent runner
  -> produces RamenOS artifacts
  -> sends artifacts / test intent to HIL appliance

HIL appliance
  -> controls power/reset/boot media
  -> captures serial/video/input traces
  -> emits evidence bundle

Golden machine
  -> boots RamenOS artifact
  -> emits serial provenance markers
  -> is disposable / wipeable / recoverable
```

---

## 0.1 Naming

Canonical names:

| Context | Name |
|---------|------|
| Human-facing long name | HIL Appliance Controller |
| Human-facing short name | HIL appliance |
| File/env/code prefix | `hil_appliance` / `RAMEN_HIL_APPLIANCE` |
| Slice label | S12.4 / S13.9 HIL Appliance Controller |

Avoid "Pi oracle" or similar names. The appliance is not an oracle. It is an observer/actuator whose evidence must be cross-checked against target-emitted `hil_evidence:` markers.

---

## 1. Hardware contract

Source of truth: `hardware/hil_appliance_v0.toml`.

Minimum v0 appliance:

- Raspberry Pi 3/4/5 class controller, Ethernet preferred.
- Linux userspace with SSH access from the build host.
- USB-to-RS232 adapter for target PC COM/DB9 serial capture.
- Optional motherboard COM-header-to-DB9 bracket on target side.
- 2-channel relay or opto-isolated switch for `PWR_SW` and `RESET_SW` front-panel pins.
- Stable power supply and persistent storage for logs.

Electrical safety rule:

- Pi GPIO UART is **3.3V TTL only**.
- PC COM/DB9 RS-232 is **not** TTL and must not be wired directly to Pi GPIO.
- Default serial path is target COM/DB9/header -> RS-232 adapter/cable -> USB serial adapter -> Pi USB.
- Raw GPIO UART is allowed only for a documented 3.3V TTL target debug header.

Planned KVM expansion:

- UVC HDMI capture dongle for video evidence.
- USB HID gadget for keyboard/mouse injection.
- USB mass-storage gadget or controlled boot-media mux for artifact booting.
- Optional smart plug / USB relay / network PDU for hard power recovery.

---

## 2. Interfaces

### 2.1 Controller environment

Initial scripts should consume these env vars:

```bash
RAMEN_HIL_APPLIANCE=1
RAMEN_HIL_APPLIANCE_ID=pi-hil-01
RAMEN_HIL_TARGET_ID=intel-nuc-12-reference
RAMEN_HIL_SERIAL_DEV=/dev/ttyUSB0
RAMEN_HIL_POWER_RELAY=pwr_sw
RAMEN_HIL_RESET_RELAY=reset_sw
RAMEN_HIL_EVIDENCE_DIR=out/evidence
```

### 2.2 Controller commands

The first implementation should expose shell-level commands before any daemon/API:

```bash
tools/hil/appliance_capture_serial.sh
tools/hil/appliance_press_power.sh
tools/hil/appliance_press_reset.sh
tools/hil/appliance_power_cycle.sh
tools/hil/appliance_run_gate.sh s12-hil
tools/hil/appliance_run_gate.sh s13-hil
```

A later daemon/API may wrap these commands, but the shell contract should remain testable without a service.

### 2.3 Evidence bundle

Every appliance-mediated run writes an evidence bundle:

```text
out/evidence/hil_appliance_<run_id>.json
out/evidence/hil_appliance_<run_id>.serial.log
out/evidence/hil_appliance_<run_id>.controller.log
```

Canonical schema: `docs/HIL_APPLIANCE_EVIDENCE_V0.md`.

The appliance evidence object is a wrapper around a physical run. It should reference per-gate evidence files rather than replacing them:

```text
hil_appliance_<run_id>.json
  -> references s13_7_nvme_boot_evidence.json
  -> references s13_8_atomic_update_evidence.json
  -> references serial/controller/video artifacts by hash
```

Minimum JSON fields:

```json
{
  "schema_version": 1,
  "evidence_kind": "hil_appliance_run_v0",
  "evidence_level": "PASS/HIL-APPLIANCE",
  "run_id": "hil_appliance_20260622T131700Z_pi-hil-01_s13-hil",
  "appliance_id": "pi-hil-01",
  "target_id": "intel-nuc-12-reference",
  "git_sha": "...",
  "gate": "s13-hil",
  "started_at_unix_ms": 0,
  "ended_at_unix_ms": 0,
  "serial_device": "/dev/ttyUSB0",
  "serial_input_kind": "live_device",
  "serial_log_sha256": "...",
  "controller_log_sha256": "...",
  "power_events": [],
  "artifact_hashes": {},
  "serial_markers_observed": [],
  "target_hil_evidence_markers": {},
  "gate_evidence": [
    "out/evidence/s13_7_nvme_boot_evidence.json",
    "out/evidence/s13_8_atomic_update_evidence.json"
  ],
  "result": "pass"
}
```

Graduation mode must also capture the `hil_evidence:` markers required by `EVIDENCE_LEVELS.md`.

---

## 3. Phases

### S12.4.0 — Appliance manifest + inventory gate — SCAFFOLD LANDED

Deliverables:

- `hardware/hil_appliance_v0.toml`.
- `docs/HIL_APPLIANCE_EVIDENCE_V0.md`.
- `tools/ci/foundry_hil_appliance_s12_4.sh` inventory gate.
- Negative assertions for unsafe serial wiring language in docs/scripts.

Gate behavior:

- Default CI validates docs + manifest only.
- Physical controller inventory runs only with `RAMEN_HIL_APPLIANCE=1`.
- Strict CI may require a connected controller.

### S12.4.1 — Serial observer — ACTIVE

Deliverables:

- `tools/hil/appliance_capture_serial.sh`.
- Run-id allocation.
- Timestamped serial transcript.
- Marker scanner for `RAMEN OS`, `golden_machine:*`, `persistent_storage:*`, and `hil_evidence:*`.

Definition of done:

- Pi captures a live boot serial transcript from the golden target.
- Transcript is archived under `out/evidence/`.
- Stale `RAMEN_HIL_SERIAL_LOG` replay remains development-only and cannot satisfy appliance graduation.

### S12.4.2 — Power/reset actuator

Deliverables:

- `tools/hil/appliance_press_power.sh`.
- `tools/hil/appliance_press_reset.sh`.
- `tools/hil/appliance_power_cycle.sh`.
- Relay-backed power-button press.
- Relay-backed reset-button press.
- Hard timeout and recovery path.
- Controller log records all actuator events.

Definition of done:

- Appliance can reboot the target without human intervention.
- Appliance can recover from a hung boot by reset/power cycle.
- Evidence bundle records every actuation with timestamps.

### S13.9.0 — Appliance-mediated S13 HIL

Deliverables:

- `RAMEN_HIL_APPLIANCE=1 RAMEN_HIL_GRADUATION=1 just s13-hil` path.
- Appliance captures live serial for S13.7/S13.8.
- Evidence bundle references S13 gate evidence JSON.

Definition of done:

- S13.7 NVMe boot marker is captured live through the appliance.
- S13.8 atomic-update marker and active-slot marker are captured live through the appliance.
- Evidence includes target ID, appliance ID, git SHA, artifact hashes, serial transcript, and power-cycle transcript.

### S14-pre — KVM-grade control

Deliverables:

- HDMI/UVC capture.
- USB HID keyboard/mouse injection.
- Optional screenshot/video artifact capture.

Definition of done:

- Agent can observe firmware/boot UI without a human monitor.
- Agent can send keyboard input for boot menu / firmware interaction when required.

### S14-pre+ — Virtual boot media

Deliverables:

- USB mass-storage gadget or controlled boot-media path.
- Artifact handoff from build host to appliance.
- Reboot into supplied EFI/init image without manual copying.

Definition of done:

- Agent can build an artifact, hand it to the appliance, reboot the target, and collect evidence without manual media handling.

### Later — Hardware fuzzing control plane

Potential expansions:

- USB fuzzing fixture.
- PCIe device inventory and controlled passthrough experiments.
- Power-failure injection during storage/update tests.
- Long-run soak and randomized reboot campaigns.

These remain out of scope until the appliance v0 loop is stable.

---

## 4. CI policy

| Mode | Env | Behavior |
|------|-----|----------|
| Default CI | none | Validate docs/manifests only; no hardware required |
| Appliance inventory | `RAMEN_HIL_APPLIANCE=1` | Validate controller tools, serial device, relay config, dry-run evidence JSON |
| HIL live | `RAMEN_HIL_GOLDEN_MACHINE=1 RAMEN_HIL_APPLIANCE=1` | Run live capture + power/reset control |
| Graduation | `RAMEN_HIL_GRADUATION=1 RAMEN_HIL_APPLIANCE=1` | Disallow stale logs; require live serial + evidence JSON |
| Strict | `RAMEN_CI_STRICT=1` | Hardware skips become failures |

---

## 5. Foundry gate

Gate: `tools/ci/foundry_hil_appliance_s12_4.sh`.

Default assertions:

- design doc exists and references `hardware/hil_appliance_v0.toml`;
- manifest parses as TOML;
- wrapper evidence schema exists and defines `hil_appliance_run_v0`;
- `NEXT_TASKS.md` names the appliance slice before S13 metal graduation;
- docs contain the RS-232 vs TTL warning;
- no default path requires physical hardware.

Appliance-enabled assertions:

- `RAMEN_HIL_SERIAL_DEV` exists;
- serial device can be opened/configured at 115200 8N1;
- relay config is present;
- controller can write an evidence JSON dry-run;
- optional: target boot banner captured in a live run.

Graduation assertions:

- no `RAMEN_HIL_SERIAL_LOG` fallback;
- live serial transcript exists;
- `hil_evidence:` markers present;
- evidence bundle includes `appliance_id`, `target_id`, `git_sha`, artifact hashes, and power events.

---

## 6. Scope guard

In scope now:

- Pi appliance plan and manifest.
- Serial observation.
- Power/reset relay control.
- Evidence packaging.
- Integration with S12/S13 HIL gate discipline.

Out of scope until after v0:

- Replacing Pi-KVM wholesale.
- PCIe protocol analyzers.
- Native USB/xHCI target stack.
- General hardware fuzzing campaigns.
- Treating controller observations as trusted kernel facts.

---

## 7. Why this belongs before deeper bare metal

Manual reboot/capture workflows do not scale to agentic OS development. The appliance turns bare-metal iteration into a repeatable loop:

```text
build -> stage -> reboot -> observe -> classify -> archive -> retry
```

That loop is required before RamenOS can safely pursue native USB/HID, storage rollback failure injection, hardware fuzzing, or autonomous driver work on real machines.
