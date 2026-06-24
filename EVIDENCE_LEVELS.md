# Evidence Levels

**Last Updated:** 2026-06-22
**Status:** Authoritative for HIL gate reporting

Foundry gates may print `PASS`, but **PASS is not one thing**. Use these levels in gates, docs, and evidence JSON.

## Levels

| Level | Meaning | Typical source |
|-------|---------|----------------|
| `PASS/QEMU` | Inventory + build + QEMU negative smoke only | Default `foundry_s13_nvme_boot_s13_7.sh` / `foundry_s13_atomic_update_s13_8.sh` without `RAMEN_HIL_GOLDEN_MACHINE=1` |
| `PASS/HIL-LOG` | Operator-provided serial log replay (`RAMEN_HIL_SERIAL_LOG`) | Development convenience; **not** metal graduation |
| `PASS/HIL-LIVE` | Serial captured from `RAMEN_HIL_SERIAL_DEV` during this gate run | Lab evidence; still weaker than graduation |
| `PASS/HIL-APPLIANCE` | Live serial captured by the appliance plus controller power/reset transcript | Appliance-mediated lab evidence; bridge toward autonomous CI |
| `PASS/METAL` | `RAMEN_HIL_GRADUATION=1` + live serial + `hil_evidence:` provenance markers + evidence JSON bundle | Tier-1 / golden-machine graduation |

## Graduation mode

Set for serious metal runs:

```bash
export RAMEN_HIL_GOLDEN_MACHINE=1
export RAMEN_HIL_GRADUATION=1
export RAMEN_HIL_SERIAL_DEV=/dev/ttyUSB0
# optional: export RAMEN_HIL_APPLIANCE=1
# optional: export RAMEN_HIL_APPLIANCE_ID=pi-hil-01
# optional: export RAMEN_HIL_MACHINE_ID=amd-ryzen-lab   # default: intel-nuc-12-reference
```

`RAMEN_HIL_GRADUATION=1` **disallows** `RAMEN_HIL_SERIAL_LOG` (stale/copied logs).

When `RAMEN_HIL_APPLIANCE=1`, the evidence bundle must also include controller identity and power/reset events from `hardware/hil_appliance_v0.toml`.

## Serial provenance markers

Metal graduation logs must include:

```
hil_evidence: git_sha=...
hil_evidence: init_profile=...
hil_evidence: machine_id=...
hil_evidence: storage_manifest_sha256=...
hil_evidence: kernel_efi_sha256=...
hil_evidence: init_img_sha256=...
hil_evidence: boot_epoch_nonce=...
```

Appliance-mediated runs should additionally record controller-side metadata in JSON rather than serial:

```
appliance_id
controller_log_sha256
serial_log_sha256
power_events
video_artifact_sha256   # optional, once KVM capture lands
```

## Evidence bundles

HIL gates write JSON under `out/evidence/`:

- `s13_7_nvme_boot_evidence.json`
- `s13_8_atomic_update_evidence.json`
- `hil_appliance_<run_id>.json` — wrapper schema: `docs/HIL_APPLIANCE_EVIDENCE_V0.md`

The appliance wrapper may correlate per-gate evidence files via `gate_evidence`, but it must not replace those gate-owned evidence files.

## Claim safety (S13)

| Safe claim | Unsafe claim |
|------------|--------------|
| QEMU Driver Factory loop mature | Native NVMe driver works |
| UEFI boot from NVMe ESP path observed (S13.7 HIL) | RamenOS has native block storage on metal |
| A/B UEFI metadata recognized (S13.8 scaffold) | Atomic update/rollback proved on metal |
| Appliance captured target-emitted evidence | Appliance observations are target truth |
| S13.7/S13.8 **gate scaffolds** complete | S13 slice **complete** |

S13 is **complete** only after `PASS/METAL` on Tier-1 class hardware for both S13.7 and S13.8 with the full two-boot atomic-update protocol (future hardening). Appliance evidence improves reproducibility but does not replace target-emitted provenance markers.
