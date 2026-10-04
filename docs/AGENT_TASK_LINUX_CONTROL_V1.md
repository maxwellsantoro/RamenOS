# SW0 A2.1 — Linux scoped-shell foundation

**Status:** implemented development-fixture gate, not full A2 conformance.
**Command:** `just foundry-agent-task-linux-control` on Linux with Docker.
**Claim:** one scripted repair, pinned WASM validation and the named Linux probes.
The LT backend, shared protocol, and durable Linux transactions are outside
this A2.1 gate and landed in later steps listed in [Current Status](../CURRENT_STATUS.md).
The real hidden fixture bank and complete authority conformance remain pending.
No comparative model, narrower-authority, target-kernel or physical claim follows.

## Actual Linux substrate

Bubblewrap 0.9.0 on `bigman` failed to create an isolated network namespace under
its existing Ubuntu policy. Keep that policy intact and use the installed Docker
engine. The trusted launcher controls Docker; the consumer never receives engine
access, host credentials, host environment, mount selection or a daemon socket.
This host tooling is separate from native IDL interfaces and production routing.

The gate requires a locally installed image and resolves it to an immutable image
ID before any commands. It never pulls or falls back to an unsandboxed process.
The default image is pinned by digest:

```sh
docker pull python@sha256:139020233cc412efe4c8135b0efe1c7569dc8b28ddd88bddb109b764f8977e30
just foundry-agent-task-linux-control
```

CI installs that same digest in a separate setup step and runs the required gate.
A trusted operator may set `RAMEN_TASK_LINUX_IMAGE` to another installed image;
its actual immutable ID is recorded, and every assertion must still pass. Freeze
the image/runtime before future comparisons. Missing Linux, Docker, image, or
containment fails the gate; there is no skipped-test PASS.

Each invocation creates a new container as UID/GID 65534, with all capabilities
dropped, no-new-privileges, a read-only root, private PID/mount/network namespaces,
no external networking, Docker's default seccomp profile, 32 PIDs, 2 GiB memory
and swap ceiling, one CPU, and a 64-descriptor limit. Private scratch is a 16 MiB
`/tmp` tmpfs with nosuid/nodev/noexec. The launcher checks actual daemon config and
exact bind mounts; the consumer probe checks UID, capability state, seccomp,
no-new-privileges, descriptors and namespace identities. Host process, private
workspace, socket and network probes execute inside the container.

These are finite configuration/probe assertions, not a proof against every
kernel, daemon, syscall or helper behavior. The daemon and launcher are trusted
and have broader host authority than the consumer. The image includes executable
helpers and metadata beyond the RT fixture's operation inventory.

## Useful task and sealing

The development fixture bytes live in `tools/agent_task/fixtures/` and are shared
with the independently runnable A1.1 RT gate. A trusted, feature-gated Rust example
compiles its WAT validator and exports the inputs; it is never an agent tool.
There is one development instance, not a pilot or hidden final bank.

The scripted shell reads a read-only `/inputs` directory and writes a repaired
configuration in `/candidate`. The script itself chooses the repair; the launcher
provides no solving helper. The evaluator independently checks the configuration,
preserved label and untouched private workspace. The equality-schema dialect is
the same deliberately small dialect as RT.

The host seals candidate bytes from one regular, non-symlink inode with bounded,
nonblocking reads. Symlink and FIFO staging fail. A private read-only CAS subset
contains those exact bytes, schema, validator and the invalid initial config.
The validator invocation mounts only that subset and the exact RT worker binary.
It uses the same 1500 ms guest / 2500 ms whole-invocation / 4096 diagnostic limits.
The worker verifies CAS hashes and executes the pinned WASM. Subsequent client
edits and a forged result file cannot change the sealed candidate or trusted
validation observation. The worker cannot publish output. This milestone writes
an evaluator-accepted artifact, not an LS durable commit receipt or grant service.

## Deadlines and cleanup

Ordinary commands have a 10-second invocation budget; negative cases reduce it.
The deadline begins before container creation and includes inspection, start,
stdin/stdout/stderr and execution. Input is capped at 8192 bytes and aggregate
output at 16384 bytes. Oversized output fails rather than returning a partial
successful observation. Trusted engine diagnostics stay outside agent responses.

Every exit forcibly removes the named container and checks that it no longer
exists. This ends its private PID namespace and descendants, including background
processes. CLI transports close and its process group is killed when necessary.
Cleanup has its own bounded engine waits and is measured separately; engine
failure invalidates the gate. A timed-out create RPC without confirmed creation
cannot certify cleanup and fails as `creation_not_confirmed`. There is no hard
real-time guarantee for the host scheduler or Docker daemon.

## Authority and evidence

`out/agent-task-linux-control/report.json` records fixture/worker/image pins,
source fingerprints, inspected configs, actual namespace/descriptor inventory,
per-invocation elapsed/cleanup measurements and probe results. `accepted.json`
contains the sealed result. `RAMEN_TASK_LINUX_EVIDENCE_DIR` selects another output.
Reports belong to the evaluator, outside future model context. Private canary
bytes are not written into the report.

The mapping separates desired policy from available authority. LS can read and
enumerate the entire inputs directory, including policy and validator bytes;
write arbitrary candidate files; execute image helpers and interpreted candidate
code; create private processes, IPC and scratch; observe its container metadata;
and pass its descriptors to its own descendants. Candidate storage persists
between commands until task cleanup. Open descriptors survive permission changes:
the gate changes schema mode after a consumer opens it, verifies new opens are
denied, and observes the old descriptor still reading until container termination.
A mode change is therefore not equivalent to RT grant revocation.

The forced probes cover private read/write, workspace B, traversal, host process
metadata, input/root writes, daemon socket, host loopback/external networking,
unsafe staging, invalid validation, mutable/forged results, output/deadline bounds,
PID ceilings and background cleanup. Zero successful forbidden probes applies
only to that suite. Unprobed effects and all-arm/time-indexed authority mapping
remain `unknown`; this report makes no set-inclusion claim. Protocol descriptions,
serializer, pagination/events, LS transactions and LT/RT point comparisons are
covered by later contracts, not this A2.1 result.

Use the [current queue](../NEXT_TASKS.md) for remaining full-A2 authority,
lifecycle, hidden-bank and evaluator work. The proposed full conformance command
in the [study plan](plans/2026-09-16-agent-task-proof.md) is not a runnable recipe
or a conclusion supplied by this foundation gate.
