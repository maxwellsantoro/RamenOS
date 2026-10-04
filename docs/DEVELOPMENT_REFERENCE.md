# Development Reference

**Last Updated:** 2026-10-03
**Status:** Host tooling and operator reference

Start with [Getting Started](GETTING_STARTED.md) for setup and focused gates.
Use [Current Status](../CURRENT_STATUS.md) and [Next Tasks](../NEXT_TASKS.md)
for landed state and execution order. Store commands below run on the host.

These tools support the [Vision](../VISION.md) of an everyday OS for humans and
AI agents. Host commands exercise components of that product; they do not imply
a complete target desktop or hardware-qualified runtime.

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

For a self-contained demo, run `just foundry-store-s0`. For interactive use,
start a development service from the repository root in one terminal:

```bash
mkdir -p out/store-demo
RAMEN_STORE_DEV_MODE=1 RAMEN_STORE_ACCESS_POLICY=AllowAll \
  RAMEN_STORE_SOCKET="$PWD/out/store-demo/store.sock" \
  RAMEN_STORE_ROOT="$PWD/out/store-demo/artifacts" \
  cargo run -p store_service
```

Those settings permit unsigned artifacts and local access for this demo. In a
second terminal, emit and validate a plan:

```bash
cargo run -p store_cli -- emit-plan \
  --catalog store/catalog.json \
  --program-id ramen.demo.hello \
  --store-socket "$PWD/out/store-demo/store.sock" \
  --tmp-root out/store-demo/tmp \
  --out out/store-demo/launch_plan.json

cargo run -p store_cli -- validate-execution-launch-plan \
  --src out/store-demo/launch_plan.json
```

To ingest a prepared file, supply its actual path and the same `--store-socket`
to `cargo run -p store_cli -- ingest --src ...`. Ingestion consumes a scoped
source descriptor. The service owns artifact storage; the client's
`--installed-root` is not a substitute for connecting to it.

The service defaults to `out/store_service.sock`, while these client commands
default to `/tmp/store_service.sock`; using one explicit socket avoids that
mismatch. Stop the foreground service with Ctrl-C when finished.

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

- `RAMEN_HIL_APPLIANCE=1`: enable appliance inventory/live serial paths;
  power/reset actuation remains a separate milestone.
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
