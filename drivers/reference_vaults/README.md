# Reference Vaults

Reference Vaults pin driver context for humans and coding agents: device
specification, Oracle traces, capture provenance, harness contract, and known
limits. They reduce guesswork; they do not make a prompt deterministic or prove
that a trace covers every device state.

| Vault | Current evidence |
|-------|------------------|
| [virtio-net](virtio-net/README.md) | Linux init/packet Oracle, replay, embedded-vector network harness |
| [virtio-blk](virtio-blk/README.md) | Linux init/sector Oracle, replay, embedded-vector block harness |
| [Template](template/README.md) | Required context for a new vault |

Obtain and inspect the relevant vault before writing hardware interactions.
The canonical native IDL lives under [idl/](../../idl/); `harness.toml` is a
context copy whose alignment is checked by gates. Specification summaries must
retain their original source/version, and traces their actual origin/hash.
Fixture, live Linux Oracle, native QEMU device I/O, and metal qualification are
different scopes under [Evidence Levels](../../EVIDENCE_LEVELS.md).

A new vault should include `datasheets/`, `traces/`, `harness.toml`, `notes.md`,
and a README explaining capture/replay commands and missing evidence. Do not
replace unknown registers or error behavior with guesses from pre-training.
