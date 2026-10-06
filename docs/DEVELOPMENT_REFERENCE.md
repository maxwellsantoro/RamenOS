# Development Reference

**Last Updated:** 2026-10-05
**Status:** Host tooling and operator reference

Start with [Getting Started](GETTING_STARTED.md) for setup and focused gates.
Use [Current Status](../CURRENT_STATUS.md) and [Next Tasks](../NEXT_TASKS.md)
for landed state and execution order. Store commands below run on the host.

These tools support the [Vision](../VISION.md) of an everyday OS for humans and
AI agents. Host commands exercise components of that product; they do not imply
a complete target desktop or hardware-qualified runtime.

## Fast iteration and complete checks

Use `just dev-check PACKAGE TEST FEATURES` for an exclusive warm host build,
strict Clippy and selected serialized tests. Features are a comma-separated list;
`TEST` defaults to `lib`, and default features are disabled. Run `just codegen`
after checkout and IDL edits. Development records do not replace Foundry gates.
Run affected consumers and `just preflight` on the fixed integration candidate.

CI separates quality, host/Docker and QEMU jobs on isolated runners, then requires
all applicable lanes through the stable `foundry` check. Stage timings are retained
under `out/foundry/timings/`. The host lane explicitly enables an exact-source,
phase-separated compiler cache while rerunning every acceptance check. Local gates
keep fresh targets unless `RAMEN_FOUNDRY_BUILD_CACHE=1` is set. See the
[execution profile](FOUNDRY_CI_OPTIMIZATION_V0.md) for keys, cleanup and claim limits.

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
- `RAMEN_STORE_INGEST_MAX_BYTES`: per-artifact ceiling, default 1 GiB, range
  1 byte–16 GiB; independent of SW0's configuration-file ceiling.
- `RAMEN_STORE_MAX_INGESTIONS`: concurrent preparation/publication reservations,
  default 4, range 1–16. Their byte ceilings bound live staging capacity.
- `RAMEN_STORE_INGEST_TIMEOUT_MS`: preparation deadline, default 30000 ms,
  range 1–3600000 ms; source metadata/read, output copy/hash/sync run in a
  supervised child outside the registry/projection locks.
- `RAMEN_STORE_MAX_CONNECTIONS`: active connection ceiling, default 32,
  range 1–1024; excess connections are closed.
- `RAMEN_STORE_REQUEST_TIMEOUT_MS`: absolute frame/descriptor handshake and idle
  connection budget, default 5000 ms, range 1–60000 ms.

The host `StoreClient` configures its timeout on both initial and replacement
connections (default 30 seconds). It bounds the complete response frame with one
absolute deadline, including partial header/body reads, and applies a timeout to
each blocking write. This is not an invocation-wide connect/write/read deadline.
Before dispatch, an already observable peer closure causes reconnection. A closure
that races dispatch or any transport/framing failure returns an error and discards
the stream; the next explicit operation reconnects. No request is automatically
replayed after dispatch. An ingestion error can leave publication uncertain, so
callers must reconcile artifact/ownership state before deciding to ingest again.

Invalid settings fail startup. Host ingestion validates write authority before
receiving the descriptor, stages privately, then revalidates authority and current
ownership before publication. Disconnect during preparation cancels the worker;
no candidate is published. Timeout requests termination of the acknowledged
worker process group; if kernel-stalled work cannot yet be reaped, the request
returns and its reservation remains occupied until termination is confirmed.
This avoids unbounded replacement workers. These controls bound source work and
live staging, not total retained CAS size or every local filesystem operation.
Publication still performs durable Store IO under the locks; a filesystem stall
there remains a separate availability limit. A lost reply after publication does
not establish that publication failed.

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
