# Current Status

**Last Updated:** 2026-10-03
**Status:** Active and authoritative for landed state
**Current Slice:** S12.4 HIL appliance v0 physical loop
**Software Lane:** SW0 foundations through A2.9 implemented; full A2 and comparison pending

RamenOS is public pre-alpha, building toward the everyday OS for humans and AI
agents described in [VISION.md](VISION.md). This file records implemented behavior
and evidence boundaries. [NEXT_TASKS.md](NEXT_TASKS.md) owns the next work;
[CHANGELOG.md](CHANGELOG.md) holds detailed milestone history and
[DECISIONS.md](DECISIONS.md) holds rationale.

## Execution state

The S12.4 physical lane awaits test-hardware setup: first live serial capture,
then Intel AMT 11 power/reset, S12 on SATA, and S13 NVMe boot/update/rollback.
No live capture or actuation is scheduled. SW0 continues independently of lab
access and NVMe graduation. S14's prerequisites from both lanes remain in
`NEXT_TASKS.md`; USB xHCI/HID and the S15 desktop are future work.

## Implemented foundations and their boundaries

| Area | Landed behavior | Evidence and limits |
|------|-----------------|---------------------|
| Kernel / S0–S8 | x86_64 and aarch64 boot, typed IPC, capabilities, shared-memory mappings, tracing and SPSC ring foundations | Selected target/QEMU paths; fixed-size tables. Capability-table use after SMP transition is deliberately blocked; general SMP/IRQ support remains incomplete |
| Typed interfaces | IDL/codegen, protocol/message IDs, bounded wire contracts | Native contracts are defined in `idl/`; generated syntax alone grants no authority |
| Native runner / S10 | Host Wasmtime execution, manifests, granted-handle injection and guest deadlines | Host runtime; no complete target userspace loader or Wasmtime environment |
| Semantic State / S10 | Host snapshots, subscriptions/reactor, capability-filtered views; selected QEMU snapshot/IPC paths | Multi-source aggregation and target reactor remain incomplete; default boot/time metadata includes fixtures |
| Store / S1–S10 | Host CAS, signatures, durable ownership, path/tag queries, read-only projections and typed CoW commits | Full user launch/porting flow and target persistence remain incomplete |
| Execution fabric / S10 | Placement, lease, duplicate-observer, launch-plan and trace contracts | Synthetic nodes/load and simulated routing; no distributed transport |
| Driver Foundry / S11 | virtio-net Reference Vault, Linux Oracle capture, replay and typed harness transfers | Embedded Oracle packet vectors in QEMU; device-backed native send/receive unproven |
| Golden machine / S12 | Machine contract, GOP probe, HIL boot/IOMMU gate scaffolds and appliance tooling | First live appliance capture, AMT actuation and physical graduation pending |
| Storage / S13 | Block IDL, virtio-blk Oracle capture/replay and typed harness transfers; NVMe/atomic-update probes | Embedded sector vectors and QEMU scaffolds; native device read/write/flush and physical two-boot rollback remain unproven |
| Compatibility / S2–S9 | Separate Linux capsule VM, host POSIX and GPU quarantine paths with gates | Boundaries differ per runner; default POSIX profile is rlimits-only, not general containment |
| RamenOrg / G0 | Governance schemas, packets, renderers, validators, bounded trials and drift gate | A2-local only; no autonomous merge/release/hardware/public-support authority |

[PLATFORM_OVERVIEW.md](PLATFORM_OVERVIEW.md) explains responsibilities;
[SLICES.md](SLICES.md) defines slice scope. [SECURITY_STATUS.md](SECURITY_STATUS.md)
and [RISKS.md](RISKS.md) record residual risks.

## SW0: runnable evidence, not a completed experiment

The useful task repairs one configuration, runs a pinned WASM validator, commits
an immutable artifact, and denies named unauthorized operations. RT means RamenOS
typed, LT Linux typed, and LS Linux scoped shell. These gates use scripted
consumers and trusted host fixtures, not model trials or target-native task clients.

| Step | Implemented scope | Gate / contract |
|------|-------------------|-----------------|
| A0 | Pure transaction model and deterministic synthetic fixtures; no IO enforcement | `just foundry-agent-task-contract-a0` · [Contract](docs/AGENT_TASK_CONTRACT_V0.md) |
| A1.0 | Generated protocol-14 layouts and allocation-free request preflight | `just foundry-agent-task-protocol-a1-0` · [Protocol](docs/AGENT_TASK_PROTOCOL_V1.md) |
| A1.1 | Useful RT host service task, immutable staging, supervised validator, durable receipts, denials and replay | `just foundry-agent-task-proof-rt` · [Service proof](docs/AGENT_TASK_SERVICE_PROOF_V1.md) |
| A2.1 | Linux scripted repair, inspected Docker containment, pinned worker and named OS probes | `just foundry-agent-task-linux-control` · [Linux control](docs/AGENT_TASK_LINUX_CONTROL_V1.md) |
| A2.2 | Shared strict JSON codec/descriptions and opt-in RT IPC adapter | `just foundry-agent-task-adapter` · [Adapter](docs/AGENT_TASK_ADAPTER_V1.md) |
| A2.3 | Independent LT broker, grants, sealed validation, durable transactions and named RT/LT cases | `just foundry-agent-task-lt` · [LT backend](docs/AGENT_TASK_LT_BACKEND_V1.md) |
| A2.4 | Contained LS commands/launcher, shared Linux transactions, peer checks and original-receipt recovery | `just foundry-agent-task-ls-transactions` · [LS transactions](docs/AGENT_TASK_LS_TRANSACTIONS_V1.md) |
| A2.5 | Shared v2 pull/poll/cancel subscriptions and 50-operation typed lifecycle comparison | `just foundry-agent-task-subscriptions` · [Subscriptions](docs/AGENT_TASK_SUBSCRIPTIONS_V2.md) |
| A2.6 | Finite canonical inventory, 33 common cases and separate LS OS probes; unknown authority retained | `just foundry-agent-task-authority` · [Authority](docs/AGENT_TASK_AUTHORITY_V1.md) |
| A2.7 | Synthetic bank/release contract, bounded sessions, 45 development attempts and retained failures | `just foundry-agent-task-evaluator-controls` · [Evaluator controls](docs/AGENT_TASK_EVALUATOR_CONTROLS_V1.md) |
| A2.8 | Named acknowledged-ID cleanup, unresolved-create quarantine and explicit interrupted-commit receipt recovery | `just foundry-agent-task-reconciliation` · [Reconciliation](docs/AGENT_TASK_RECONCILIATION_V1.md) |
| A2.9 | 31 issued-right subsets under two policies, single-right effects and named lifetime witnesses | `just foundry-agent-task-requestable-authority` · [Requestable authority](docs/AGENT_TASK_REQUESTABLE_AUTHORITY_V1.md) |

A2.6's frozen suite has zero successful forbidden probes, but does not prove whole
`E_max`, continuous `E(t)`, or narrower authority. A2.9 establishes equality only
for the declared-interface issued-right projection. LS mounted files, retained
observations/descriptors and raw broker access remain broader observations;
typed host clients and transitive deputy authority remain incompletely bounded.
Unacknowledged Docker create intents cannot certify cleanup from an empty inventory.

Full A2, real hidden-bank/study releases, provider/token accounting, model
comparison, production registration and target task enforcement remain pending.
The [Agent Task Proof plan](docs/plans/2026-09-16-agent-task-proof.md) defines their
acceptance and the three separate contrasts. No comparative agent advantage is claimed.

## Recent boundary fixes

The 2026-10-03 review changes are implemented and recorded in `CHANGELOG.md`:

- Store reads bind the requested content ID to authenticated metadata and blob bytes;
  native execution hashes the byte snapshot it consumes.
- Host ingestion uses caller-opened regular-file descriptors; pathname-only requests
  fail closed. Native ingestion retains its IDL shared-memory source contract.
- Owner/manifest publication intents precede CAS visibility; restart/retry recovers
  partial publication and unrelated orphans remain denied.
- Native Unix/chardev IPC shares the invocation's absolute deadline through connect
  and partial transfers; uncertain dispatch is not automatically replayed.
- LT duplicate staging preserves capability/validation and counts unique candidates,
  matching RT; portable and executable regressions cover capacity.
- CI and preflight use the same complete implemented SW0 sequence. This does not
  turn that sequence into full A2 conformance or a completed model study.

Earlier memory, tracing, WASM, projection and durability fixes remain in the
changelog and their contract/gate documents. They establish host/QEMU behavior,
not complete SMP, client isolation, physical durability or production assurance.

## Physical inventory and graduation boundary

The pinned reference is the Lenovo ThinkCentre M900 SFF, machine type 10FH,
Core i7-6700, 8 GiB RAM, with a 240 GB SanDisk SATA SSD. The Raspberry Pi 4
(4 GiB), FTDI USB-to-RS-232 adapter and null-modem chain are physically installed.
Firmware/AMT preflight and the first live serial capture remain pending; a compatible
M.2 2280 PCIe NVMe drive is still required for S13 graduation.

`PASS/QEMU` is not metal evidence. `PASS/HIL-LOG`, `PASS/HIL-LIVE`,
`PASS/HIL-APPLIANCE`, and `PASS/METAL` have separate provenance requirements in
[EVIDENCE_LEVELS.md](EVIDENCE_LEVELS.md). Standalone `operator-golden-machine`
and `appliance-mediated` metal claims must be stamped separately.

S13.8 currently probes A/B metadata. Graduation still requires an implemented
publication/readback/selection verifier, a new-slot boot and a separate rollback
boot with fresh nonces and matching artifact identities. Firmware NVMe detection
and vector-backed block transfers establish neither native NVMe I/O nor that protocol.

## Documentation maintenance

The 2026-10-03 documentation review consolidates status here, execution criteria
in `NEXT_TASKS.md`, and direction in `ROADMAP.md`. Current references and agent
skills were reconciled with source/gates; historical security plans are archived
behind maintained references at their existing paths. This is documentation work
and adds no runtime, model, hardware, security or release-readiness evidence.

## Validation entry points

```bash
just s11
just s12
just s13
just hil-appliance
just foundry-org-governance-g0
```

SW0 gates above expose their individual fixture scopes. Full `just preflight`
requires Linux, Python `jsonschema`, Docker/seccomp, and the installed pinned image.
Physical gates are opt-in and require documented preparation/provenance.
