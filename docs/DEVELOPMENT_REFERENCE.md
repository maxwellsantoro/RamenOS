# Development Reference

**Last Updated:** 2026-09-16
**Status:** Host tooling and operator reference

Start with [Getting Started](GETTING_STARTED.md) for setup and focused gates.
Use [Current Status](../CURRENT_STATUS.md) and [Next Tasks](../NEXT_TASKS.md)
for landed state and execution order. Store commands below run on the host.

## Hardware and evidence

Default CI checks host behavior, QEMU, inventory, and replay. Physical gates are
opt-in and require prepared images, fresh nonces, live capture, and matching
provenance. Follow [Evidence Levels](../EVIDENCE_LEVELS.md) and the
[HIL appliance plan](plans/2026-06-22-hil-appliance-controller.md) before running
physical gates; enabling a flag alone is not sufficient for graduation.

The appliance is lab infrastructure, outside the target TCB. Development-log
replay, live serial observation, appliance capture, and metal graduation have
separate evidence requirements.

## Store CLI Examples

Emit a launch plan from the catalog:

```bash
cargo run -p store_cli -- emit-plan \
  --catalog store/catalog.json \
  --program-id ramen.demo.hello \
  --out out/store/launch_plan.json
```

Ingest a file into a local installed store:

```bash
cargo run -p store_cli -- ingest \
  --src /path/to/file \
  --installed-root out/installed
```

Validate an execution launch plan:

```bash
cargo run -p store_cli -- validate-execution-launch-plan \
  --src out/store/launch_plan.json
```

## Operational Knobs

Store service:

- `RAMEN_STORE_TRUSTED_KEYS`: trusted Ed25519 key file, required outside dev.
- `RAMEN_STORE_DEV_MODE`: explicit local-dev opt-in for unsigned artifacts.
- `RAMEN_STORE_ACCESS_POLICY`: `AllowAll`, `RequireCredentials`,
  `RequireKnownService`, or `Whitelist`; default is fail-closed.
- `RAMEN_STORE_SOCKET`, `RAMEN_STORE_ROOT`, `RAMEN_STORE_AUDIT_LOG`: local paths.

POSIX runner:

- `RAMEN_POSIX_RUNNER_ACK_RISK=1`: required kill-switch acknowledgment.
- `RAMEN_POSIX_RUNNER_DISABLE_SANDBOX=1`: dangerous local-dev bypass.

HIL:

- `RAMEN_HIL_APPLIANCE=1`: enable physical appliance inventory/control paths.
- `RAMEN_HIL_GRADUATION=1`: require live graduation discipline.
- `RAMEN_HIL_SERIAL_DEV` / `RAMEN_HIL_SERIAL_LOG`: live serial device or
  development log input, depending on the gate.

Development modes are explicit, noisy, and should never be treated as release
configuration.

## Repository Map

- **Target OS:** [kernel/](../kernel/), [kernel_uefi/](../kernel_uefi/),
  [kernel_aarch64/](../kernel_aarch64/), [kernel_api/](../kernel_api/).
- **Typed interfaces:** [idl/](../idl/), [idl_codegen/](../idl_codegen/),
  [schemas/](../schemas/).
- **Services and runtime:** [services/](../services/),
  [runtime_supervisor/](../runtime_supervisor/), [sdk/](../sdk/).
- **Driver Foundry:** [driver_foundry/](../driver_foundry/),
  [drivers/reference_vaults/](../drivers/reference_vaults/), [hardware/](../hardware/).
- **Store platform:** [store/](../store/), [store_cli/](../store_cli/),
  [artifact_store_core/](../artifact_store_core/),
  [artifact_store_schema/](../artifact_store_schema/).
- **Gates and docs:** [tools/ci/](../tools/ci/), [tools/hil/](../tools/hil/),
  [docs/](../docs/).
